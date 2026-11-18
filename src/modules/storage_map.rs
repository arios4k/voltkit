//! Read-only scan of allocated disk space. Child IDs always follow their parents.
use std::collections::HashSet;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

pub struct Node {
    pub path: PathBuf,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub directory: bool,
    pub bytes: u64,
    pub skipped: usize,
}

pub struct Scan {
    pub nodes: Vec<Node>,
    pub files: Vec<usize>,
}

#[derive(Default)]
pub struct Progress {
    pub cancelled: AtomicBool,
    pub scanned: AtomicUsize,
}

impl Progress {
    pub fn check(&self) -> io::Result<()> {
        if self.cancelled.load(Relaxed) {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        Ok(())
    }
}

pub fn scan(root: &Path, progress: &Progress) -> io::Result<Scan> {
    let root_meta = fs::symlink_metadata(root)?;
    if !root_meta.is_dir() {
        return Err(io::Error::from(io::ErrorKind::NotADirectory));
    }
    let root_entries = fs::read_dir(root)?;
    let mut nodes = vec![Node {
        path: root.to_owned(),
        parent: None,
        children: Vec::new(),
        directory: true,
        bytes: root_meta.blocks().saturating_mul(512),
        skipped: 0,
    }];
    let mut directories = HashSet::from([(root_meta.dev(), root_meta.ino())]);
    let mut hardlinks = HashSet::new();
    let mut pending = vec![0];
    let mut root_entries = Some(root_entries);
    while let Some(parent) = pending.pop() {
        progress.check()?;
        let entries = match root_entries.take() {
            Some(entries) => entries,
            None => match fs::read_dir(&nodes[parent].path) {
                Ok(entries) => entries,
                Err(_) => {
                    nodes[parent].skipped += 1;
                    continue;
                }
            },
        };
        for entry in entries {
            progress.check()?;
            let Ok(entry) = entry else {
                nodes[parent].skipped += 1;
                continue;
            };
            progress.scanned.fetch_add(1, Relaxed);
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                nodes[parent].skipped += 1;
                continue;
            };
            if !meta.is_file() && !meta.is_dir() {
                continue;
            }
            let identity = (meta.dev(), meta.ino());
            let id = nodes.len();
            if meta.is_dir() {
                if !directories.insert(identity) {
                    nodes[parent].skipped += 1;
                    continue;
                }
                pending.push(id);
            }
            let bytes = if meta.is_file() && meta.nlink() > 1 && !hardlinks.insert(identity) {
                0
            } else {
                meta.blocks().saturating_mul(512)
            };
            nodes.push(Node {
                path,
                parent: Some(parent),
                children: Vec::new(),
                directory: meta.is_dir(),
                bytes,
                skipped: 0,
            });
            nodes[parent].children.push(id);
        }
    }
    for id in (1..nodes.len()).rev() {
        let parent = nodes[id].parent.unwrap();
        nodes[parent].bytes = nodes[parent].bytes.saturating_add(nodes[id].bytes);
        nodes[parent].skipped += nodes[id].skipped;
    }
    for id in 0..nodes.len() {
        progress.check()?;
        let mut children = std::mem::take(&mut nodes[id].children);
        children.sort_unstable_by(|&a, &b| {
            nodes[b]
                .bytes
                .cmp(&nodes[a].bytes)
                .then_with(|| nodes[a].path.cmp(&nodes[b].path))
        });
        nodes[id].children = children;
    }
    let mut files: Vec<_> = (1..nodes.len())
        .filter(|&id| !nodes[id].directory)
        .collect();
    files.sort_unstable_by(|&a, &b| {
        nodes[b]
            .bytes
            .cmp(&nodes[a].bytes)
            .then_with(|| nodes[a].path.cmp(&nodes[b].path))
    });
    progress.check()?;
    Ok(Scan { nodes, files })
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Divide along the longest side, assigning each item area proportional to its size.
/// The caller supplies a bounded list of nonzero weights, largest first.
pub fn treemap(weights: &[u64], bounds: Rect) -> Vec<Rect> {
    fn divide(weights: &[u64], bounds: Rect, output: &mut Vec<Rect>) {
        if weights.len() <= 1 {
            if !weights.is_empty() {
                output.push(bounds);
            }
            return;
        }
        let total: u128 = weights.iter().map(|&weight| u128::from(weight)).sum();
        let mut left = u128::from(weights[0]);
        let mut split = 1;
        for &next in &weights[1..weights.len() - 1] {
            if left * 2 + u128::from(next) >= total {
                break;
            }
            left += u128::from(next);
            split += 1;
        }
        let ratio = left as f64 / total as f64;
        let (first, second) = if bounds.width >= bounds.height {
            let width = bounds.width * ratio;
            (
                Rect { width, ..bounds },
                Rect {
                    x: bounds.x + width,
                    width: bounds.width - width,
                    ..bounds
                },
            )
        } else {
            let height = bounds.height * ratio;
            (
                Rect { height, ..bounds },
                Rect {
                    y: bounds.y + height,
                    height: bounds.height - height,
                    ..bounds
                },
            )
        };
        divide(&weights[..split], first, output);
        divide(&weights[split..], second, output);
    }

    let mut output = Vec::with_capacity(weights.len());
    divide(weights, bounds, &mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_counts_allocated_blocks_once_and_does_not_follow_links() -> io::Result<()> {
        let root = std::env::temp_dir().join(format!("voltkit-storage-map-{}", std::process::id()));
        fs::create_dir(&root)?;
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        let nested = root.join("nested");
        fs::create_dir(&nested)?;
        let file = nested.join("sample");
        fs::write(&file, [1; 8192])?;
        fs::hard_link(&file, root.join("duplicate"))?;
        std::os::unix::fs::symlink(&root, nested.join("cycle"))?;
        let allocated = fs::metadata(&file)?.blocks() * 512;
        let sample = scan(&root, &Progress::default())?;
        let root_bytes = [root, nested]
            .iter()
            .map(|path| fs::metadata(path).unwrap().blocks() * 512)
            .sum::<u64>();
        assert_eq!(sample.nodes[0].bytes, root_bytes + allocated);
        assert_eq!(sample.nodes.len(), 4);
        Ok(())
    }

    #[test]
    fn treemap_preserves_area_and_fills_bounds_without_overlap() {
        let bounds = Rect {
            x: 10.0,
            y: 30.0,
            width: 640.0,
            height: 300.0,
        };
        let weights = [70, 20, 7, 3];
        let rects = treemap(&weights, bounds);
        for (index, (rect, weight)) in rects.iter().zip(weights).enumerate() {
            let area = bounds.width * bounds.height * weight as f64 / 100.0;
            assert!((rect.width * rect.height - area).abs() < 0.0001);
            let center = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            assert!(bounds.contains(center.0, center.1));
            assert_eq!(
                rects.iter().position(|r| r.contains(center.0, center.1)),
                Some(index)
            );
        }
        assert_eq!(rects.len(), weights.len());
    }
}
