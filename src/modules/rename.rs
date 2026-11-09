//! Batch rename: search and replace in file names, with regular expressions,
//! numbering and letter case, a preview that names every conflict, and undo.
//!
//! Nothing is renamed until [`apply`] is called. It renames in two steps (to
//! temporary names, then to the final ones), so swapping two names or renaming
//! a chain `a→b, b→c` works, and if anything fails everything done so far is
//! put back. Every applied batch is written to a journal so [`undo`] can reverse it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use regex::{NoExpand, Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use crate::i18n::tr;

/// Which part of a file name the search and replace works on.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// The name without its extension (`photo` in `photo.jpg`).
    #[default]
    Name,
    /// Only the extension (`jpg`).
    Extension,
    /// The whole name, extension included.
    Full,
}

/// A change of letter case, applied after the replacement.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Case {
    #[default]
    Keep,
    Upper,
    Lower,
    Title,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Options {
    /// What to look for. Empty matches the whole part of the name.
    pub search: String,
    /// What to put instead. `{n}` and `{n:3}` (padded to 3 digits) are the item's number.
    /// With regular expressions, `$1` and `${name}` are the groups of the match.
    pub replace: String,
    pub regex: bool,
    pub case_sensitive: bool,
    /// Replace every match (the default), or only the first.
    pub all_matches: bool,
    pub scope: Scope,
    pub case: Case,
    /// The first number, and how much each next item adds.
    pub start: i64,
    pub step: i64,
}

impl Default for Options {
    fn default() -> Self {
        Options { search: String::new(), replace: String::new(), regex: false, case_sensitive: true, all_matches: true, scope: Scope::Name, case: Case::Keep, start: 1, step: 1 }
    }
}

/// What will happen to one item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// The new name is valid and free.
    Ready,
    /// The name does not change.
    Unchanged,
    /// It cannot be renamed, and why.
    Problem(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub from: PathBuf,
    pub to: PathBuf,
    pub status: Status,
}

/// Splits a file name into its name and extension. A leading dot belongs to the name
/// (`.bashrc` has no extension), and so does a trailing one.
pub fn split_name(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 && i + 1 < name.len() => (&name[..i], &name[i + 1..]),
        _ => (name, ""),
    }
}

fn build_regex(o: &Options) -> Result<Regex> {
    let pattern = if o.search.is_empty() { "^.*$".to_owned() } else if o.regex { o.search.clone() } else { regex::escape(&o.search) };
    RegexBuilder::new(&pattern)
        .case_insensitive(!o.case_sensitive)
        .size_limit(1 << 20)
        .build()
        .map_err(|e| anyhow::anyhow!("{}", crate::i18n::fill(tr("The search is not a valid regular expression: {}"), &[&first_line(&e.to_string())])))
}

fn first_line(text: &str) -> String {
    text.lines().last().unwrap_or(text).trim().to_owned()
}

/// Replaces `{n}` and `{n:W}` by the item's number.
fn expand_number(replace: &str, number: i64) -> String {
    let mut out = String::new();
    let mut rest = replace;
    while let Some(i) = rest.find("{n") {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(end) = tail.find('}') else { break };
        let spec = &tail[2..end];
        let width = match spec {
            "" => Some(0),
            s => s.strip_prefix(':').and_then(|w| w.parse::<usize>().ok()).filter(|w| *w <= 12),
        };
        match width {
            Some(w) => out.push_str(&format!("{number:0w$}")),
            None => out.push_str(&tail[..=end]),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

fn change_case(text: &str, case: Case) -> String {
    match case {
        Case::Keep => text.to_owned(),
        Case::Upper => text.to_uppercase(),
        Case::Lower => text.to_lowercase(),
        Case::Title => {
            let mut out = String::with_capacity(text.len());
            let mut start = true;
            for c in text.chars() {
                if start && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                    start = false;
                } else if c.is_alphabetic() {
                    out.extend(c.to_lowercase());
                } else {
                    out.push(c);
                    start = c.is_whitespace() || matches!(c, '_' | '-' | '.' | '(' | '[');
                }
            }
            out
        }
    }
}

fn transform(part: &str, re: &Regex, o: &Options, number: i64) -> String {
    let replace = expand_number(&o.replace, number);
    let out = match (o.regex && !o.search.is_empty(), o.all_matches) {
        (true, true) => re.replace_all(part, replace.as_str()),
        (true, false) => re.replace(part, replace.as_str()),
        (false, true) => re.replace_all(part, NoExpand(&replace)),
        (false, false) => re.replace(part, NoExpand(&replace)),
    };
    change_case(&out, o.case)
}

/// The new name of the item called `name`, which is number `index` in the batch
/// (counting from 0). Folders have no extension.
#[cfg(test)]
pub fn new_name(name: &str, is_dir: bool, index: usize, o: &Options) -> Result<String> {
    let re = build_regex(o)?;
    Ok(new_name_with(&re, name, is_dir, index, o))
}

fn new_name_with(re: &Regex, name: &str, is_dir: bool, index: usize, o: &Options) -> String {
    let number = o.start + o.step * index as i64;
    let (stem, ext) = if is_dir { (name, "") } else { split_name(name) };
    let join = |stem: &str, ext: &str| if ext.is_empty() { stem.to_owned() } else { format!("{stem}.{ext}") };
    match o.scope {
        Scope::Name => join(&transform(stem, re, o, number), ext),
        Scope::Extension if ext.is_empty() => name.to_owned(),
        Scope::Extension => join(stem, &transform(ext, re, o, number)),
        Scope::Full => transform(name, re, o, number),
    }
}

fn check_name(name: &str) -> Option<String> {
    if name.is_empty() {
        Some(tr("The new name is empty").to_owned())
    } else if name == "." || name == ".." {
        Some(tr("The new name is not allowed").to_owned())
    } else if name.contains('/') || name.contains('\0') {
        Some(tr("The new name has a slash in it").to_owned())
    } else if name.len() > 255 {
        Some(tr("The new name is too long").to_owned())
    } else {
        None
    }
}

/// What the batch would do, without touching anything. The order of `paths` sets the numbering.
pub fn plan(paths: &[PathBuf], o: &Options) -> Result<Vec<Item>> {
    let re = build_regex(o)?;
    let mut items: Vec<Item> = paths
        .iter()
        .enumerate()
        .map(|(i, from)| {
            let name = from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let new = new_name_with(&re, &name, from.is_dir(), i, o);
            let to = from.with_file_name(&new);
            let status = if new == name {
                Status::Unchanged
            } else if let Some(problem) = check_name(&new) {
                Status::Problem(problem)
            } else {
                Status::Ready
            };
            Item { from: from.clone(), to, status }
        })
        .collect();

    // Names that will be free because their owner is renamed away.
    let leaving: HashSet<PathBuf> = items.iter().filter(|i| i.status != Status::Unchanged).map(|i| i.from.clone()).collect();
    let mut claimed: HashMap<PathBuf, usize> = HashMap::new();
    for item in items.iter().filter(|i| i.status == Status::Ready) {
        *claimed.entry(item.to.clone()).or_default() += 1;
    }
    for item in &mut items {
        if item.status != Status::Ready {
            continue;
        }
        if claimed.get(&item.to).copied().unwrap_or(0) > 1 {
            item.status = Status::Problem(tr("Another item gets the same name").to_owned());
        } else if item.to.exists() && !leaving.contains(&item.to) {
            item.status = Status::Problem(tr("A file with that name already exists").to_owned());
        }
    }
    Ok(items)
}

/// One finished rename, as written to the undo journal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Done {
    pub from: PathBuf,
    pub to: PathBuf,
}

/// Renames every ready item. All or nothing: on a failure what was done is put back.
pub fn apply(items: &[Item]) -> Result<Vec<Done>> {
    if let Some(bad) = items.iter().find(|i| matches!(i.status, Status::Problem(_))) {
        bail!("{}: {}", bad.from.display(), match &bad.status { Status::Problem(p) => p.as_str(), _ => "" });
    }
    let jobs: Vec<Done> = items.iter().filter(|i| i.status == Status::Ready).map(|i| Done { from: i.from.clone(), to: i.to.clone() }).collect();
    two_step(&jobs)?;
    Ok(jobs)
}

fn temp_path(from: &Path, i: usize) -> PathBuf {
    from.with_file_name(format!(".voltkit-rename-{}-{i}.tmp", std::process::id()))
}

/// Moves every `from` to its `to` through a temporary name, undoing the lot if one fails.
fn two_step(jobs: &[Done]) -> Result<()> {
    let mut parked: Vec<(usize, PathBuf)> = Vec::new();
    let mut placed: Vec<usize> = Vec::new();
    let mut fail: Option<anyhow::Error> = None;
    for (i, job) in jobs.iter().enumerate() {
        let tmp = temp_path(&job.from, i);
        match std::fs::rename(&job.from, &tmp) {
            Ok(()) => parked.push((i, tmp)),
            Err(e) => {
                fail = Some(anyhow::Error::new(e).context(job.from.display().to_string()));
                break;
            }
        }
    }
    if fail.is_none() {
        for (i, tmp) in &parked {
            if jobs[*i].to.exists() {
                fail = Some(anyhow::anyhow!("{}: {}", jobs[*i].to.display(), tr("A file with that name already exists")));
                break;
            }
            match std::fs::rename(tmp, &jobs[*i].to) {
                Ok(()) => placed.push(*i),
                Err(e) => {
                    fail = Some(anyhow::Error::new(e).context(jobs[*i].to.display().to_string()));
                    break;
                }
            }
        }
    }
    match fail {
        None => Ok(()),
        Some(e) => {
            // Put everything back, in reverse.
            for i in placed.iter().rev() {
                let _ = std::fs::rename(&jobs[*i].to, temp_path(&jobs[*i].from, *i));
            }
            for (i, tmp) in parked.iter().rev() {
                let _ = std::fs::rename(tmp, &jobs[*i].from);
            }
            Err(e)
        }
    }
}

// ---------------------------------------------------------------- Undo journal

fn journal_path() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("voltkit/rename-undo.json"))
}

/// Remembers the batch as the one to undo. Only the last batch is kept.
pub fn remember(done: &[Done]) -> Result<()> {
    let Some(path) = journal_path() else { return Ok(()) };
    remember_at(&path, done)
}

fn remember_at(path: &Path, done: &[Done]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string(done)?).with_context(|| path.display().to_string())
}

/// The batch that `undo` would reverse.
pub fn last_batch() -> Vec<Done> {
    journal_path().map(|p| read_journal(&p)).unwrap_or_default()
}

fn read_journal(path: &Path) -> Vec<Done> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Reverses the last batch. Fails, changing nothing, if a renamed item has moved or its old name is taken.
pub fn undo() -> Result<usize> {
    let Some(path) = journal_path() else { return Ok(0) };
    undo_at(&path)
}

fn undo_at(path: &Path) -> Result<usize> {
    let batch = read_journal(path);
    if batch.is_empty() {
        bail!("{}", tr("There is nothing to undo"));
    }
    for d in &batch {
        if !d.to.exists() {
            bail!("{}: {}", d.to.display(), tr("it is no longer there, so the rename can't be undone"));
        }
    }
    let back: Vec<Done> = batch.iter().map(|d| Done { from: d.to.clone(), to: d.from.clone() }).collect();
    // The old names may be taken only by items that are part of this very batch.
    let leaving: HashSet<&Path> = back.iter().map(|d| d.from.as_path()).collect();
    for d in &back {
        if d.to.exists() && !leaving.contains(d.to.as_path()) {
            bail!("{}: {}", d.to.display(), tr("A file with that name already exists"));
        }
    }
    two_step(&back)?;
    let _ = std::fs::remove_file(path);
    Ok(back.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(search: &str, replace: &str) -> Options {
        Options { search: search.into(), replace: replace.into(), ..Options::default() }
    }

    fn name(n: &str, o: &Options) -> String {
        new_name(n, false, 0, o).unwrap()
    }

    fn tree(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("voltkit-rename-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(dir: &Path, names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(|n| {
            let p = dir.join(n);
            std::fs::write(&p, n).unwrap();
            p
        }).collect()
    }

    #[test]
    fn splits_names_and_extensions() {
        assert_eq!(split_name("photo.jpg"), ("photo", "jpg"));
        assert_eq!(split_name("archive.tar.gz"), ("archive.tar", "gz"));
        assert_eq!(split_name(".bashrc"), (".bashrc", ""));
        assert_eq!(split_name("noext"), ("noext", ""));
        assert_eq!(split_name("trailing."), ("trailing.", ""));
    }

    #[test]
    fn plain_search_and_replace_leaves_the_extension_alone() {
        assert_eq!(name("IMG_0001.jpg", &opts("IMG", "Holiday")), "Holiday_0001.jpg");
        assert_eq!(name("a.b.txt", &opts(".", "_")), "a_b.txt");
        let first = Options { all_matches: false, ..opts("a", "o") };
        assert_eq!(name("banana.txt", &first), "bonana.txt");
        assert_eq!(name("banana.txt", &opts("a", "o")), "bonono.txt");
    }

    #[test]
    fn the_search_can_be_case_insensitive_and_the_replacement_is_literal() {
        let o = Options { case_sensitive: false, ..opts("img", "$1") };
        assert_eq!(name("IMG_1.png", &o), "$1_1.png");
    }

    #[test]
    fn regular_expressions_use_groups() {
        let o = Options { regex: true, ..opts(r"(\d{4})-(\d{2})-(\d{2})", "$3.$2.$1") };
        assert_eq!(name("report 2026-11-07.pdf", &o), "report 07.11.2026.pdf");
        let bad = Options { regex: true, ..opts("(", "") };
        assert!(new_name("x", false, 0, &bad).is_err());
    }

    #[test]
    fn an_empty_search_replaces_the_whole_part_and_numbers_count_up() {
        let o = Options { start: 7, step: 2, ..opts("", "photo-{n:3}") };
        assert_eq!(new_name("a.jpg", false, 0, &o).unwrap(), "photo-007.jpg");
        assert_eq!(new_name("b.jpg", false, 2, &o).unwrap(), "photo-011.jpg");
        assert_eq!(expand_number("x{n}{n:2}{nope}{n:99}", 5), "x505{nope}{n:99}");
    }

    #[test]
    fn scope_chooses_what_is_changed() {
        let ext = Options { scope: Scope::Extension, ..opts("jpeg", "jpg") };
        assert_eq!(name("a.jpeg", &ext), "a.jpg");
        assert_eq!(name("jpeg", &ext), "jpeg");
        let full = Options { scope: Scope::Full, ..opts("a.jpeg", "b.png") };
        assert_eq!(name("a.jpeg", &full), "b.png");
        assert_eq!(new_name("v1.2", true, 0, &opts(".", "-")).unwrap(), "v1-2", "a folder has no extension");
    }

    #[test]
    fn letter_case_is_changed_last() {
        let up = Options { case: Case::Upper, ..opts("zzz", "") };
        assert_eq!(name("hello world.TXT", &up), "HELLO WORLD.TXT");
        assert_eq!(change_case("hello wORLD-of_things", Case::Title), "Hello World-Of_Things");
        assert_eq!(change_case("ÉCOLE", Case::Lower), "école");
    }

    #[test]
    fn invalid_names_are_problems_not_renames() {
        let dir = tree("invalid");
        let files = touch(&dir, &["a.txt"]);
        let slash = plan(&files, &Options { regex: true, ..opts("a", "x/y") }).unwrap();
        assert!(matches!(slash[0].status, Status::Problem(_)));
        let empty = plan(&files, &Options { scope: Scope::Full, ..opts("a.txt", "") }).unwrap();
        assert!(matches!(empty[0].status, Status::Problem(_)));
        let same = plan(&files, &opts("zzz", "y")).unwrap();
        assert_eq!(same[0].status, Status::Unchanged);
    }

    #[test]
    fn conflicts_are_found_before_anything_happens() {
        let dir = tree("conflict");
        let files = touch(&dir, &["a.txt", "b.txt", "keep.txt"]);
        // Two items get one name.
        let same = plan(&files[..2], &Options { scope: Scope::Full, regex: true, ..opts(r"^[ab]\.txt$", "x.txt") }).unwrap();
        assert!(same.iter().all(|i| matches!(i.status, Status::Problem(_))));
        // A name that is taken by a file that is not renamed.
        let taken = plan(&files[..1], &opts("a", "keep")).unwrap();
        assert!(matches!(taken[0].status, Status::Problem(_)));
        // A name that is taken by a file that is renamed away: fine.
        let chain = plan(&files[..2], &Options { regex: true, ..opts("^[ab]$", "b") }).unwrap();
        assert!(matches!(chain[0].status, Status::Problem(_)), "both would be named b");
        let shift = plan(&touch(&dir, &["c1.txt", "c2.txt"]), &Options { start: 2, ..opts("", "c{n}") }).unwrap();
        assert!(shift.iter().all(|i| i.status == Status::Ready || i.status == Status::Unchanged), "{shift:?}");
    }

    #[test]
    fn swapping_and_chains_work_and_undo_restores_everything() {
        let dir = tree("swap");
        let files = touch(&dir, &["a.txt", "b.txt"]);
        // Number them in reverse: a→b, b→a.
        let items = vec![
            Item { from: files[0].clone(), to: files[1].clone(), status: Status::Ready },
            Item { from: files[1].clone(), to: files[0].clone(), status: Status::Ready },
        ];
        let done = apply(&items).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("b.txt")).unwrap(), "a.txt");
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "b.txt");
        let journal = dir.join("journal.json");
        remember_at(&journal, &done).unwrap();
        assert_eq!(undo_at(&journal).unwrap(), 2);
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "a.txt");
        assert!(!journal.exists());
        assert!(undo_at(&journal).is_err(), "nothing left to undo");
    }

    #[test]
    fn a_failure_puts_everything_back() {
        let dir = tree("fail");
        let files = touch(&dir, &["one.txt", "two.txt"]);
        // The second target's folder does not exist, so the second step fails.
        let items = vec![
            Item { from: files[0].clone(), to: dir.join("uno.txt"), status: Status::Ready },
            Item { from: files[1].clone(), to: dir.join("missing/dos.txt"), status: Status::Ready },
        ];
        assert!(apply(&items).is_err());
        assert!(files[0].exists() && files[1].exists());
        assert!(!dir.join("uno.txt").exists());
    }

    #[test]
    fn apply_refuses_a_batch_with_problems() {
        let items = vec![Item { from: "/tmp/x".into(), to: "/tmp/y".into(), status: Status::Problem("no".into()) }];
        assert!(apply(&items).is_err());
    }

    #[test]
    fn undo_refuses_when_things_moved() {
        let dir = tree("moved");
        let journal = dir.join("j.json");
        remember_at(&journal, &[Done { from: dir.join("old.txt"), to: dir.join("gone.txt") }]).unwrap();
        assert!(undo_at(&journal).is_err());
        let files = touch(&dir, &["new.txt", "old.txt"]);
        remember_at(&journal, &[Done { from: files[1].clone(), to: files[0].clone() }]).unwrap();
        assert!(undo_at(&journal).is_err(), "the old name is taken by something else");
    }
}
