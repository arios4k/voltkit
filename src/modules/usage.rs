//! What's Using This?: finds the processes that keep a file, folder or drive
//! busy, or use a network port, and closes them on request.
//!
//! Everything comes from /proc, so it only sees processes this user may
//! inspect; `voltkit using --machine` run as root (through pkexec) sees the rest.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use crate::i18n::tr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Path(PathBuf),
    Port(u16),
}

impl Target {
    /// A port number ("3000" or ":3000"), or a path to a file or folder.
    pub fn parse(text: &str) -> Result<Target> {
        let text = text.trim();
        let digits = text.strip_prefix(':').unwrap_or(text);
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            let port: u16 = digits.parse().ok().filter(|&p| p > 0).context(tr("ports go from 1 to 65535"))?;
            return Ok(Target::Port(port));
        }
        let path = Path::new(text);
        let path = path.canonicalize().with_context(|| trf!("{} doesn't exist", path.display()))?;
        Ok(Target::Path(path))
    }

    pub fn describe(&self) -> String {
        match self {
            Target::Path(p) => p.display().to_string(),
            Target::Port(p) => trf!("port {}", p),
        }
    }
}

/// A process that uses the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub pid: u32,
    /// Process start time (clock ticks after boot), so a reused PID is never
    /// mistaken for this process when closing it.
    pub start: u64,
    pub uid: u32,
    /// The process name, as the kernel reports it.
    pub command: String,
    /// The desktop app this process belongs to, when the desktop launched it.
    pub app_id: Option<String>,
    /// How it uses the target, in plain words.
    pub uses: Vec<String>,
}

impl Holder {
    /// Why Voltkit won't offer to close this process, if it won't.
    pub fn protected(&self) -> Option<&'static str> {
        protection(self.pid, &self.command)
    }
}

#[derive(Debug, Default)]
pub struct Scan {
    pub holders: Vec<Holder>,
    /// Processes of other users that couldn't be inspected without root.
    pub unreadable: usize,
}

pub fn scan(target: &Target) -> Scan {
    let sockets = match target {
        Target::Port(port) => sockets_on_port(*port),
        Target::Path(_) => HashMap::new(),
    };
    let me = std::process::id();
    let mut result = Scan::default();
    let Ok(dir) = std::fs::read_dir("/proc") else { return result };
    for entry in dir.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
        if pid == me {
            continue;
        }
        let proc_dir = entry.path();
        let Some((uid, start, command)) = process_info(&proc_dir) else { continue };
        // Kernel threads (and finished processes) have no command line and
        // hold nothing of the user's.
        if command_line(&proc_dir).is_empty() {
            continue;
        }
        let uses = match target {
            Target::Path(path) => path_uses(&proc_dir, path),
            Target::Port(_) => socket_uses(&proc_dir, &sockets),
        };
        match uses {
            Some(uses) if !uses.is_empty() => {
                let app_id = app_id(&proc_dir);
                result.holders.push(Holder { pid, start, uid, command, app_id, uses });
            }
            Some(_) => {}
            None => result.unreadable += 1,
        }
    }
    result.holders.sort_by(|a, b| a.protected().is_some().cmp(&b.protected().is_some()).then(a.pid.cmp(&b.pid)));
    result
}

/// (uid, start time, name) from /proc/PID/status and /proc/PID/stat.
fn process_info(proc_dir: &Path) -> Option<(u32, u64, String)> {
    let status = std::fs::read_to_string(proc_dir.join("status")).ok()?;
    let uid = status.lines().find_map(|l| l.strip_prefix("Uid:"))?.split_whitespace().next()?.parse().ok()?;
    let stat = std::fs::read_to_string(proc_dir.join("stat")).ok()?;
    let (start, command) = parse_stat(&stat)?;
    Some((uid, start, command))
}

/// The name (in parentheses, may contain spaces) and the start time (field 22).
fn parse_stat(stat: &str) -> Option<(u64, String)> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let command = stat[open + 1..close].to_owned();
    let start = stat[close + 1..].split_whitespace().nth(19)?.parse().ok()?;
    Some((start, command))
}

fn command_line(proc_dir: &Path) -> Vec<u8> {
    std::fs::read(proc_dir.join("cmdline")).unwrap_or_default()
}

/// How a process uses `target` or anything inside it. `None` when its open
/// files can't be read (another user's process).
fn path_uses(proc_dir: &Path, target: &Path) -> Option<Vec<String>> {
    let inside = |p: &Path| p.starts_with(target);
    let mut uses = Vec::new();
    let mut seen = HashSet::new();
    let fds = std::fs::read_dir(proc_dir.join("fd")).ok()?;
    for fd in fds.flatten() {
        if let Ok(link) = std::fs::read_link(fd.path()) {
            let link = strip_deleted(&link);
            if inside(&link) && seen.insert(link.clone()) {
                uses.push(trf!("has {} open", link.display()));
            }
        }
    }
    if let Ok(cwd) = std::fs::read_link(proc_dir.join("cwd"))
        && inside(&cwd)
    {
        uses.push(trf!("is working in {}", cwd.display()));
    }
    if let Ok(exe) = std::fs::read_link(proc_dir.join("exe")) {
        let exe = strip_deleted(&exe);
        if inside(&exe) {
            uses.push(trf!("is running from {}", exe.display()));
            seen.insert(exe);
        }
    }
    // Libraries and other files mapped into memory.
    if let Ok(maps) = std::fs::read_to_string(proc_dir.join("maps")) {
        for line in maps.lines() {
            let Some(path) = line.split_whitespace().nth(5).filter(|p| p.starts_with('/')) else { continue };
            let path = strip_deleted(Path::new(path));
            if inside(&path) && seen.insert(path.clone()) {
                uses.push(trf!("has {} loaded", path.display()));
            }
        }
    }
    Some(uses)
}

fn strip_deleted(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    PathBuf::from(text.strip_suffix(" (deleted)").unwrap_or(&text))
}

/// Socket inodes on `port`, with a description of each: listening TCP
/// sockets and bound UDP sockets.
fn sockets_on_port(port: u16) -> HashMap<u64, String> {
    let mut found = HashMap::new();
    for (file, proto) in [("tcp", "TCP"), ("tcp6", "TCP"), ("udp", "UDP"), ("udp6", "UDP")] {
        let Ok(text) = std::fs::read_to_string(format!("/proc/net/{file}")) else { continue };
        for (inode, description) in parse_sockets(&text, proto, port) {
            found.insert(inode, description);
        }
    }
    found
}

/// Parses a /proc/net/{tcp,udp}[6] table.
fn parse_sockets(text: &str, proto: &str, port: u16) -> Vec<(u64, String)> {
    const LISTEN: &str = "0A";
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let local_port = u16::from_str_radix(fields.get(1)?.rsplit(':').next()?, 16).ok()?;
            let state = *fields.get(3)?;
            let inode: u64 = fields.get(9)?.parse().ok()?;
            if local_port != port || inode == 0 {
                return None;
            }
            match proto {
                "TCP" if state == LISTEN => Some((inode, trf!("is listening on TCP port {}", port))),
                "TCP" => Some((inode, trf!("has a TCP connection on port {}", port))),
                _ => Some((inode, trf!("is using UDP port {}", port))),
            }
        })
        .collect()
}

fn socket_uses(proc_dir: &Path, sockets: &HashMap<u64, String>) -> Option<Vec<String>> {
    let fds = std::fs::read_dir(proc_dir.join("fd")).ok()?;
    let mut uses: Vec<String> = fds
        .flatten()
        .filter_map(|fd| std::fs::read_link(fd.path()).ok())
        .filter_map(|link| {
            let inode = link.to_str()?.strip_prefix("socket:[")?.strip_suffix(']')?.parse::<u64>().ok()?;
            sockets.get(&inode).cloned()
        })
        .collect();
    uses.sort();
    uses.dedup();
    Some(uses)
}

/// The desktop app ID from the process's systemd scope, which the desktop
/// names after the app it launched: `app-gnome-org.gnome.Nautilus-1234.scope`,
/// `app-org.gnome.Terminal.slice`.
fn app_id(proc_dir: &Path) -> Option<String> {
    let cgroup = std::fs::read_to_string(proc_dir.join("cgroup")).ok()?;
    let path = cgroup.lines().find_map(|l| l.strip_prefix("0::"))?;
    path.rsplit('/').find_map(app_id_from_unit)
}

fn app_id_from_unit(unit: &str) -> Option<String> {
    let name = unit.strip_prefix("app-")?;
    let name = name.strip_suffix(".scope").or_else(|| name.strip_suffix(".service")).or_else(|| name.strip_suffix(".slice"))?;
    let name = name.replace("\\x2d", "-");
    let name = name.split('@').next()?;
    // A launcher prefix, then the application ID, then "-<number>".
    let name = name.rsplit_once('-').filter(|(_, n)| n.chars().all(|c| c.is_ascii_digit())).map_or(name, |(id, _)| id);
    let name = ["gnome-", "flatpak-", "kde-", "dbus-", "snap-"].iter().find_map(|p| name.strip_prefix(p)).unwrap_or(name);
    // Real app IDs are reverse-DNS names; anything else is a plain command.
    (name.contains('.') && !name.contains(' ')).then(|| name.to_owned())
}

/// Parts of the system Voltkit won't close, with what they are.
fn protection(pid: u32, command: &str) -> Option<&'static str> {
    if pid == 1 {
        return Some(tr("the system's service manager"));
    }
    const PARTS: &[(&str, &str)] = &[
        ("gnome-shell", N_!("the desktop itself")),
        ("plasmashell", N_!("the desktop itself")),
        ("kwin", N_!("the window manager")),
        ("Xwayland", N_!("the display server")),
        ("Xorg", N_!("the display server")),
        ("gnome-session", N_!("your login session")),
        ("systemd", N_!("a system service manager")),
        ("dbus-", N_!("the message bus apps use to talk to each other")),
        ("gdm", N_!("the login screen")),
        ("sshd", N_!("the remote login service")),
        ("login", N_!("your login session")),
        ("polkit", N_!("the system's permission service")),
        ("udisksd", N_!("the drive manager")),
        ("gvfs", N_!("the desktop's file system service")),
        ("xdg-", N_!("a desktop portal service")),
        ("pipewire", N_!("the sound and video service")),
        ("wireplumber", N_!("the sound and video service")),
        ("pulseaudio", N_!("the sound service")),
        ("NetworkManager", N_!("the network service")),
        ("voltkit", N_!("Voltkit itself")),
    ];
    PARTS.iter().find(|(prefix, _)| command.starts_with(prefix)).map(|(_, what)| tr(what))
}

/// Whether the process is still running (and is still the same process).
pub fn is_running(pid: u32, start: u64) -> bool {
    let proc_dir = PathBuf::from(format!("/proc/{pid}"));
    let Ok(stat) = std::fs::read_to_string(proc_dir.join("stat")) else { return false };
    let zombie = stat.rfind(')').and_then(|i| stat[i + 1..].split_whitespace().next()) == Some("Z");
    !zombie && parse_stat(&stat).is_some_and(|(s, _)| s == start)
}

/// Asks the process to quit (SIGTERM), or with `force` stops it at once
/// (SIGKILL). Other users' processes go through pkexec, which asks for the
/// administrator password.
pub fn close(holder: &Holder, force: bool) -> Result<()> {
    if holder.protected().is_some() {
        bail!("{}", trf!("{} is part of the system, so Voltkit won't close it", holder.command));
    }
    if !is_running(holder.pid, holder.start) {
        return Ok(());
    }
    let signal = if force { "-KILL" } else { "-TERM" };
    let pid = holder.pid.to_string();
    let mine = holder.uid == current_uid();
    let output = if mine {
        Command::new("kill").args([signal, &pid]).output()
    } else {
        Command::new("pkexec").args(["kill", signal, &pid]).output()
    }
    .context(tr("could not run kill"))?;
    match output.status.code() {
        Some(0) => Ok(()),
        Some(126) | Some(127) if !mine => bail!("{}", tr("not closed: permission was not granted")),
        _ if !is_running(holder.pid, holder.start) => Ok(()),
        _ => bail!("{}", trf!("could not close {}: {}", holder.command, String::from_utf8_lossy(&output.stderr).trim())),
    }
}

/// A process's name and, when the desktop launched it, its app ID.
pub fn process(pid: u32) -> Option<(String, Option<String>)> {
    let proc_dir = PathBuf::from(format!("/proc/{pid}"));
    let (_, _, command) = process_info(&proc_dir)?;
    Some((command, app_id(&proc_dir)))
}

pub fn current_uid() -> u32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("Uid:"))?.split_whitespace().next()?.parse().ok())
        .unwrap_or(u32::MAX)
}

/// User names from /etc/passwd.
pub fn user_name(uid: u32) -> String {
    std::fs::read_to_string("/etc/passwd")
        .ok()
        .and_then(|text| {
            text.lines().find_map(|l| {
                let mut f = l.split(':');
                let name = f.next()?;
                (f.nth(1)?.parse::<u32>().ok()? == uid).then(|| name.to_owned())
            })
        })
        .unwrap_or_else(|| uid.to_string())
}

// The --machine format, for scans run as root: one holder per line, fields
// separated by tabs, uses separated by the unit separator.
const USE_SEP: char = '\u{1f}';

pub fn to_machine(scan: &Scan) -> String {
    let mut out = format!("unreadable\t{}\n", scan.unreadable);
    for h in &scan.holders {
        let uses: Vec<String> = h.uses.iter().map(|u| u.replace(['\t', '\n', USE_SEP], " ")).collect();
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            h.pid,
            h.start,
            h.uid,
            h.command.replace(['\t', '\n'], " "),
            h.app_id.as_deref().unwrap_or(""),
            uses.join(&USE_SEP.to_string())
        ));
    }
    out
}

pub fn from_machine(text: &str) -> Scan {
    let mut scan = Scan::default();
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if let ["unreadable", n] = f.as_slice() {
            scan.unreadable = n.parse().unwrap_or(0);
            continue;
        }
        let [pid, start, uid, command, app, uses] = f.as_slice() else { continue };
        let (Ok(pid), Ok(start), Ok(uid)) = (pid.parse(), start.parse(), uid.parse()) else { continue };
        scan.holders.push(Holder {
            pid,
            start,
            uid,
            command: (*command).to_owned(),
            app_id: (!app.is_empty()).then(|| (*app).to_owned()),
            uses: uses.split(USE_SEP).filter(|u| !u.is_empty()).map(str::to_owned).collect(),
        });
    }
    scan
}

/// Scans as root through pkexec, to include every user's processes.
pub fn scan_as_root(target: &Target) -> Result<Scan> {
    let exe = std::env::current_exe()?;
    let arg = match target {
        Target::Path(p) => p.to_string_lossy().into_owned(),
        Target::Port(p) => p.to_string(),
    };
    let output = Command::new("pkexec")
        .arg(exe)
        .args(["using", "--machine", "--lang", crate::i18n::language(), "--"])
        .arg(arg)
        .output()
        .context(tr("pkexec is needed"))?;
    match output.status.code() {
        Some(0) => Ok(from_machine(&String::from_utf8_lossy(&output.stdout))),
        Some(126) | Some(127) => bail!("{}", tr("permission was not granted")),
        _ => bail!("{}", String::from_utf8_lossy(&output.stderr).trim()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_targets() {
        assert_eq!(Target::parse("3000").unwrap(), Target::Port(3000));
        assert_eq!(Target::parse(":8080").unwrap(), Target::Port(8080));
        assert!(Target::parse("0").is_err() && Target::parse("70000").is_err());
        assert_eq!(Target::parse("/tmp").unwrap(), Target::Path(PathBuf::from("/tmp").canonicalize().unwrap()));
        assert!(Target::parse("/no/such/place").is_err());
    }

    #[test]
    fn reads_stat_with_odd_names() {
        let stat = "1234 (Web Content) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 987654 20";
        assert_eq!(parse_stat(stat), Some((987654, "Web Content".to_owned())));
        let tricky = "99 (a) b) R 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 42 20";
        assert_eq!(parse_stat(tricky), Some((42, "a) b".to_owned())));
    }

    #[test]
    fn reads_app_ids_from_units() {
        assert_eq!(app_id_from_unit("app-gnome-org.gnome.Nautilus-1234.scope").as_deref(), Some("org.gnome.Nautilus"));
        assert_eq!(app_id_from_unit("app-flatpak-org.gimp.GIMP-5678.scope").as_deref(), Some("org.gimp.GIMP"));
        assert_eq!(app_id_from_unit("app-org.gnome.Terminal.slice").as_deref(), Some("org.gnome.Terminal"));
        assert_eq!(app_id_from_unit("app-gnome-io.github.arios4k.Voltkit.Daemon-2492.scope").as_deref(), Some("io.github.arios4k.Voltkit.Daemon"));
        assert_eq!(app_id_from_unit("app-gnome-voltkit-7300.scope"), None);
        assert_eq!(app_id_from_unit("vte-spawn-1.scope"), None);
    }

    #[test]
    fn parses_socket_tables() {
        let tcp = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000:0BB8 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 55501 1
   1: 0100007F:0BB8 0100007F:D431 01 00000000:00000000 00:00000000 00000000  1000        0 55502 1
   2: 00000000:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 55503 1";
        let found = parse_sockets(tcp, "TCP", 3000);
        assert_eq!(found, vec![
            (55501, "is listening on TCP port 3000".to_owned()),
            (55502, "has a TCP connection on port 3000".to_owned()),
        ]);
    }

    #[test]
    fn finds_its_own_open_file() {
        // Run the scan from a child process view: this test process holds the file.
        let dir = std::env::temp_dir().join(format!("voltkit-usage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("busy.txt");
        std::fs::write(&file, "x").unwrap();
        let _open = std::fs::File::open(&file).unwrap();
        let me = PathBuf::from(format!("/proc/{}", std::process::id()));
        let uses = path_uses(&me, &dir.canonicalize().unwrap()).unwrap();
        assert!(uses.iter().any(|u| u.starts_with("has ") && u.ends_with("busy.txt open")), "{uses:?}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn finds_a_listening_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let sockets = sockets_on_port(port);
        let me = PathBuf::from(format!("/proc/{}", std::process::id()));
        let uses = socket_uses(&me, &sockets).unwrap();
        assert_eq!(uses, vec![format!("is listening on TCP port {port}")]);
    }

    #[test]
    fn protects_the_system() {
        assert!(protection(1, "systemd").is_some());
        assert!(protection(500, "gnome-shell").is_some());
        assert!(protection(500, "Xwayland").is_some());
        assert!(protection(500, "nautilus").is_none());
        assert!(protection(500, "node").is_none());
    }

    #[test]
    fn closes_a_process_and_checks_it() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let proc_dir = PathBuf::from(format!("/proc/{}", child.id()));
        let (uid, start, command) = process_info(&proc_dir).unwrap();
        let holder = Holder { pid: child.id(), start, uid, command, app_id: None, uses: vec![] };
        assert!(is_running(holder.pid, holder.start));
        // A different start time is a different process: never touched.
        assert!(!is_running(holder.pid, holder.start + 1));
        close(&holder, false).unwrap();
        child.wait().unwrap();
        assert!(!is_running(holder.pid, holder.start));
    }

    #[test]
    fn machine_format_round_trips() {
        let scan = Scan {
            holders: vec![Holder {
                pid: 42,
                start: 7,
                uid: 0,
                command: "nginx".into(),
                app_id: None,
                uses: vec!["is listening on TCP port 80".into(), "has /var/log/x open".into()],
            }],
            unreadable: 3,
        };
        let back = from_machine(&to_machine(&scan));
        assert_eq!(back.holders, scan.holders);
        assert_eq!(back.unreadable, 3);
    }
}
