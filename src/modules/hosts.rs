//! Reading and writing `/etc/hosts` without disturbing anything we don't edit:
//! comments, blank lines and untouched entries are written back exactly as read.

use std::io::Write;
use std::net::IpAddr;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub const PATH: &str = "/etc/hosts";
pub const BACKUP: &str = "/etc/hosts.voltkit-backup";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub enabled: bool,
    pub ip: String,
    pub names: Vec<String>,
    pub comment: String,
    /// The line as read, kept so unchanged entries are written back verbatim.
    raw: Option<String>,
}

impl Entry {
    pub fn new(ip: &str, names: Vec<String>, comment: &str) -> Self {
        Self { enabled: true, ip: ip.to_owned(), names, comment: comment.trim().to_owned(), raw: None }
    }

    /// Entries the system relies on (localhost, IPv6 basics, this computer's
    /// own name). Changing them can break networking or `sudo`, so the editor
    /// shows them but doesn't change them.
    pub fn is_system(&self) -> bool {
        self.ip == "127.0.1.1"
            || self.names.iter().any(|n| n == "localhost" || n.starts_with("localhost.") || n.starts_with("ip6-"))
    }

    fn render(&self) -> String {
        if let Some(raw) = &self.raw {
            return raw.clone();
        }
        let mut line = format!("{}\t{}", self.ip, self.names.join(" "));
        if !self.comment.is_empty() {
            line.push_str(&format!("\t# {}", self.comment));
        }
        if self.enabled { line } else { format!("# {line}") }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Line {
    Entry(Entry),
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostsFile {
    lines: Vec<Line>,
}

pub fn valid_ip(ip: &str) -> bool {
    ip.parse::<IpAddr>().is_ok()
}

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && !name.starts_with(['-', '.'])
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'))
}

/// Reads "IP name… # comment", or None if the text isn't a host entry.
fn parse_entry(text: &str) -> Option<(String, Vec<String>, String)> {
    let (data, comment) = text.split_once('#').unwrap_or((text, ""));
    let mut words = data.split_whitespace();
    let ip = words.next().filter(|ip| valid_ip(ip))?;
    let names: Vec<String> = words.map(str::to_owned).collect();
    if names.is_empty() || !names.iter().all(|n| valid_name(n)) {
        return None;
    }
    Some((ip.to_owned(), names, comment.trim().to_owned()))
}

impl HostsFile {
    pub fn parse(text: &str) -> Self {
        let lines = text
            .lines()
            .map(|raw| {
                let trimmed = raw.trim_start();
                // "# 1.2.3.4 name" is a turned-off entry; other comments stay comments.
                let (enabled, body) = match trimmed.strip_prefix('#') {
                    Some(rest) => (false, rest.trim_start_matches('#').trim()),
                    None => (true, trimmed),
                };
                match parse_entry(body) {
                    Some((ip, names, comment)) => {
                        Line::Entry(Entry { enabled, ip, names, comment, raw: Some(raw.to_owned()) })
                    }
                    None => Line::Other(raw.to_owned()),
                }
            })
            .collect();
        Self { lines }
    }

    /// Reads /etc/hosts, or the file in VOLTKIT_HOSTS_FILE (used for demo
    /// screenshots; saving always writes /etc/hosts).
    pub fn load() -> Result<Self> {
        let path = std::env::var("VOLTKIT_HOSTS_FILE").unwrap_or_else(|_| PATH.to_owned());
        Ok(Self::parse(&std::fs::read_to_string(&path).with_context(|| format!("cannot read {path}"))?))
    }

    pub fn render(&self) -> String {
        let mut out: String = self
            .lines
            .iter()
            .map(|l| match l {
                Line::Entry(e) => e.render(),
                Line::Other(raw) => raw.clone(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        out.push('\n');
        out
    }

    /// Entries with their positions, for `get`, `set` and `remove`.
    pub fn entries(&self) -> Vec<(usize, &Entry)> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match l {
                Line::Entry(e) => Some((i, e)),
                Line::Other(_) => None,
            })
            .collect()
    }

    pub fn get(&self, index: usize) -> Option<&Entry> {
        match self.lines.get(index) {
            Some(Line::Entry(e)) => Some(e),
            _ => None,
        }
    }

    /// Replaces an entry; it is written in normal form from now on.
    pub fn set(&mut self, index: usize, mut entry: Entry) {
        entry.raw = None;
        self.lines[index] = Line::Entry(entry);
    }

    pub fn set_enabled(&mut self, index: usize, enabled: bool) {
        if let Some(Line::Entry(e)) = self.lines.get_mut(index)
            && e.enabled != enabled
        {
            e.enabled = enabled;
            e.raw = None;
        }
    }

    pub fn remove(&mut self, index: usize) {
        self.lines.remove(index);
    }

    pub fn push(&mut self, mut entry: Entry) {
        entry.raw = None;
        self.lines.push(Line::Entry(entry));
    }
}

/// Writes the hosts file as root through polkit (one password prompt): the new
/// file is written beside the old one, the old one is backed up, then the new
/// one is moved into place, so a failure never leaves a half-written file.
pub fn save(file: &HostsFile) -> Result<()> {
    let script = format!(
        "set -e; umask 022; cat > {PATH}.voltkit-new; cp -p {PATH} {BACKUP}; mv {PATH}.voltkit-new {PATH}"
    );
    let mut child = Command::new("pkexec")
        .args(["sh", "-c", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("pkexec is needed to save /etc/hosts")?;
    child.stdin.take().unwrap().write_all(file.render().as_bytes())?;
    let out = child.wait_with_output()?;
    match out.status.code() {
        Some(0) => Ok(()),
        // pkexec: 126 = authorization refused or dialog dismissed, 127 = not authorized
        Some(126) | Some(127) => bail!("not saved: permission was not granted"),
        _ => bail!("could not save {PATH}: {}", String::from_utf8_lossy(&out.stderr).trim()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "127.0.0.1\tlocalhost
127.0.1.1   my-laptop

# The following lines are desirable for IPv6 capable hosts
::1     ip6-localhost ip6-loopback
192.168.1.20  nas nas.home   # storage box
# 10.0.0.5 staging.example.com
";

    #[test]
    fn untouched_file_round_trips_exactly() {
        assert_eq!(HostsFile::parse(SAMPLE).render(), SAMPLE);
    }

    #[test]
    fn reads_enabled_and_disabled_entries_but_not_prose() {
        let f = HostsFile::parse(SAMPLE);
        let entries = f.entries();
        assert_eq!(entries.len(), 5);
        let nas = entries.iter().find(|(_, e)| e.names[0] == "nas").unwrap().1;
        assert_eq!(nas.names, ["nas", "nas.home"]);
        assert_eq!(nas.comment, "storage box");
        let staging = entries.iter().find(|(_, e)| e.ip == "10.0.0.5").unwrap().1;
        assert!(!staging.enabled);
        // "# The following lines…" is prose, not an entry.
        assert!(!entries.iter().any(|(_, e)| e.names.iter().any(|n| n == "following")));
    }

    #[test]
    fn marks_system_entries() {
        let f = HostsFile::parse(SAMPLE);
        let system: Vec<_> = f.entries().into_iter().filter(|(_, e)| e.is_system()).map(|(_, e)| e.ip.clone()).collect();
        assert_eq!(system, ["127.0.0.1", "127.0.1.1", "::1"]);
    }

    #[test]
    fn edits_only_change_their_line() {
        let mut f = HostsFile::parse(SAMPLE);
        let (staging, _) = f.entries().into_iter().find(|(_, e)| e.ip == "10.0.0.5").unwrap();
        f.set_enabled(staging, true);
        let (nas, _) = f.entries().into_iter().find(|(_, e)| e.ip == "192.168.1.20").unwrap();
        f.remove(nas);
        f.push(Entry::new("127.0.0.1", vec!["myapp.test".into()], "local dev"));
        let out = f.render();
        assert!(out.contains("\n10.0.0.5\tstaging.example.com\n"));
        assert!(!out.contains("nas.home"));
        assert!(out.ends_with("127.0.0.1\tmyapp.test\t# local dev\n"));
        // Everything else is untouched.
        assert!(out.starts_with("127.0.0.1\tlocalhost\n127.0.1.1   my-laptop\n\n# The following lines"));
    }

    #[test]
    fn this_machines_hosts_file_round_trips_exactly() {
        if let Ok(text) = std::fs::read_to_string(PATH) {
            let text = if text.ends_with('\n') { text } else { text + "\n" };
            assert_eq!(HostsFile::parse(&text).render(), text);
        }
    }

    #[test]
    fn validates_addresses_and_names() {
        assert!(valid_ip("10.0.0.1") && valid_ip("::1") && valid_ip("fe80::1"));
        assert!(!valid_ip("10.0.0") && !valid_ip("example.com"));
        assert!(valid_name("dev.local") && valid_name("my_host-2"));
        assert!(!valid_name("") && !valid_name("-bad") && !valid_name("has space") && !valid_name("ünï"));
    }
}
