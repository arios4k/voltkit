//! Read-only comparison of two local directory trees.
use std::collections::BTreeSet;
use std::fmt::Write;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

use crate::i18n::tr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Match,
    QuickMatch,
    Changed,
    OriginalOnly,
    BackupOnly,
    TypeMismatch,
    Skipped,
}

pub struct Entry {
    pub path: PathBuf,
    pub status: Status,
    pub detail: Option<String>,
    pub can_preview: bool,
}

#[derive(Default)]
pub struct Progress {
    pub cancelled: AtomicBool,
    pub checked: AtomicUsize,
}

impl Progress {
    fn check(&self) -> io::Result<()> {
        if self.cancelled.load(Relaxed) {
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        Ok(())
    }
}

pub fn compare(roots: [&Path; 2], verify: bool, progress: &Progress) -> io::Result<Vec<Entry>> {
    for root in roots {
        if !fs::symlink_metadata(root)?.is_dir() {
            return Err(io::Error::from(io::ErrorKind::NotADirectory));
        }
    }

    let mut pending = vec![(PathBuf::new(), [true, true])];
    let mut results = Vec::new();
    while let Some((path, present)) = pending.pop() {
        progress.check()?;
        let absolute = roots.map(|root| root.join(&path));
        let mut can_preview = false;
        let status = (|| {
            let [a, b] = [
                present[0]
                    .then(|| fs::symlink_metadata(&absolute[0]))
                    .transpose()?,
                present[1]
                    .then(|| fs::symlink_metadata(&absolute[1]))
                    .transpose()?,
            ];
            let directories = [a.as_ref(), b.as_ref()].map(|m| m.is_some_and(Metadata::is_dir));
            if let (Some(a), Some(b)) = (&a, &b)
                && a.file_type() != b.file_type()
            {
                return Ok(Some(Status::TypeMismatch));
            }
            if directories.contains(&true) {
                let names = [
                    children(&absolute[0], directories[0], progress)?,
                    children(&absolute[1], directories[1], progress)?,
                ];
                for name in names[0].union(&names[1]) {
                    progress.check()?;
                    pending.push((
                        path.join(name),
                        names.each_ref().map(|set| set.contains(name)),
                    ));
                }
            }
            let status = match (a, b) {
                (Some(a), Some(b)) if a.is_file() => {
                    can_preview = true;
                    if a.len() != b.len() {
                        Status::Changed
                    } else if verify {
                        if same_contents(&absolute, [&a, &b], progress)? {
                            Status::Match
                        } else {
                            Status::Changed
                        }
                    } else if a.modified()? == b.modified()? {
                        Status::QuickMatch
                    } else {
                        Status::Changed
                    }
                }
                (Some(a), Some(_)) if a.is_symlink() => {
                    if fs::read_link(&absolute[0])? == fs::read_link(&absolute[1])? {
                        Status::Match
                    } else {
                        Status::Changed
                    }
                }
                (Some(a), Some(_)) if a.is_dir() => return Ok(None),
                (Some(_), Some(_)) => Status::Skipped,
                (Some(_), None) => Status::OriginalOnly,
                (None, Some(_)) => Status::BackupOnly,
                (None, None) => unreachable!("a listed path belongs to at least one folder"),
            };
            Ok::<_, io::Error>(Some(status))
        })();
        progress.check()?;
        progress.checked.fetch_add(1, Relaxed);
        let (status, detail) = match status {
            Ok(Some(status)) => (status, None),
            Ok(None) => continue,
            Err(error) => (Status::Skipped, Some(error.to_string())),
        };
        results.push(Entry {
            path,
            status,
            detail,
            can_preview,
        });
    }
    results.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(results)
}

fn children(
    path: &Path,
    directory: bool,
    progress: &Progress,
) -> io::Result<BTreeSet<std::ffi::OsString>> {
    let mut names = BTreeSet::new();
    if directory {
        for entry in fs::read_dir(path)? {
            progress.check()?;
            names.insert(entry?.file_name());
        }
    }
    Ok(names)
}

fn open_regular(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    Ok(file)
}

fn fingerprint(m: &Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    (
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    )
}

fn same_contents(
    paths: &[PathBuf; 2],
    expected: [&Metadata; 2],
    progress: &Progress,
) -> io::Result<bool> {
    let mut a = open_regular(&paths[0])?;
    let mut b = open_regular(&paths[1])?;
    if fingerprint(&a.metadata()?) != fingerprint(expected[0])
        || fingerprint(&b.metadata()?) != fingerprint(expected[1])
    {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }
    let mut first = [0; 64 * 1024];
    let mut second = [0; 64 * 1024];
    let mut remaining = expected[0].len();
    while remaining > 0 {
        progress.check()?;
        let size = remaining.min(first.len() as u64) as usize;
        a.read_exact(&mut first[..size])?;
        b.read_exact(&mut second[..size])?;
        if first[..size] != second[..size] {
            return Ok(false);
        }
        remaining -= size as u64;
    }
    for (path, before) in paths.iter().zip(expected) {
        if fingerprint(&fs::symlink_metadata(path)?) != fingerprint(before) {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
    }
    Ok(true)
}

pub fn preview(path: &Path) -> io::Result<String> {
    const LIMIT: u64 = 128 * 1024;
    let mut bytes = Vec::new();
    open_regular(path)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT || bytes.contains(&0) {
        return Err(io::Error::from(io::ErrorKind::InvalidData));
    }
    String::from_utf8(bytes).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
}

impl Status {
    pub fn matches(self) -> bool {
        matches!(self, Self::Match | Self::QuickMatch)
    }
}

pub fn status_label(status: Status) -> &'static str {
    match status {
        Status::Match => tr("Matches"),
        Status::QuickMatch => tr("Same size and time"),
        Status::Changed => tr("Changed"),
        Status::OriginalOnly => tr("Original only"),
        Status::BackupOnly => tr("Backup only"),
        Status::TypeMismatch => tr("Different file types"),
        Status::Skipped => tr("Could not verify"),
    }
}

pub struct ResultSet {
    pub roots: [PathBuf; 2],
    pub verified: bool,
    pub entries: Vec<Entry>,
}

impl ResultSet {
    pub fn summary(&self) -> String {
        let mut differences = 0;
        let mut skipped = 0;
        for entry in &self.entries {
            match entry.status {
                Status::Match | Status::QuickMatch => {}
                Status::Skipped => skipped += 1,
                _ => differences += 1,
            }
        }
        let counts = trf!(
            "Differences: {} · Could not verify: {}",
            differences,
            skipped
        );
        if !self.verified {
            format!(
                "{}\n{}",
                counts,
                tr("Quick comparison checks file size and modification time only.")
            )
        } else if differences == 0 && skipped == 0 {
            tr("File contents and symbolic link targets match.").into()
        } else {
            counts
        }
    }

    pub fn entry_label(entry: &Entry) -> String {
        let path = if entry.path.as_os_str().is_empty() {
            tr("Top-level folder").to_string()
        } else {
            format!("{:?}", entry.path)
        };
        let label = format!("{} · {}", status_label(entry.status), path);
        match &entry.detail {
            Some(detail) => format!("{label} · {detail}"),
            None => label,
        }
    }

    pub fn report(&self, include_matches: bool) -> String {
        let mut report = format!("{}\n{}\n", tr("Folder Compare"), self.summary());
        let _ = writeln!(report, "{}: {:?}", tr("Original"), self.roots[0]);
        let _ = writeln!(
            report,
            "{}: {:?}\n",
            tr("Backup or comparison folder"),
            self.roots[1]
        );
        for entry in self
            .entries
            .iter()
            .filter(|entry| include_matches || !entry.status.matches())
        {
            let _ = writeln!(report, "{}", Self::entry_label(entry));
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::FileTimes;
    use std::os::unix::{fs::symlink, net::UnixListener};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn verification_detects_same_size_edits_and_distinguishes_skipped_entries() -> io::Result<()> {
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir = Cleanup(std::env::temp_dir().join(format!("voltkit-compare-{nonce}")));
        fs::create_dir(&test_dir.0)?;
        let roots = ["original", "backup"].map(|name| test_dir.0.join(name));
        let timestamp = FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(60));
        for (root, text) in roots.iter().zip([b"abcd", b"wxyz"]) {
            fs::create_dir(root)?;
            let edited = root.join("edited");
            fs::write(&edited, text)?;
            File::open(edited)?.set_times(timestamp)?;
            fs::write(root.join("same"), b"same")?;
            symlink("missing-target", root.join("link"))?;
        }
        fs::create_dir(roots[0].join("empty"))?;
        let _a = UnixListener::bind(roots[0].join("socket"))?;
        let _b = UnixListener::bind(roots[1].join("socket"))?;

        let paths = roots.each_ref().map(PathBuf::as_path);
        let quick = compare(paths, false, &Progress::default())?;
        let verified = compare(paths, true, &Progress::default())?;
        let has = |entries: &[Entry], path: &str, status| {
            entries
                .iter()
                .any(|entry| entry.path == Path::new(path) && entry.status == status)
        };
        assert!(has(&quick, "edited", Status::QuickMatch));
        assert!(has(&verified, "edited", Status::Changed));
        assert!(has(&verified, "same", Status::Match));
        assert!(has(&verified, "link", Status::Match));
        assert!(has(&verified, "empty", Status::OriginalOnly));
        assert!(has(&verified, "socket", Status::Skipped));

        let cancelled = Progress::default();
        cancelled.cancelled.store(true, Relaxed);
        assert!(
            matches!(compare(paths, true, &cancelled), Err(err) if err.kind() == io::ErrorKind::Interrupted)
        );
        Ok(())
    }
}
