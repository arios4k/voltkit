//! Environment variables for the desktop session, kept in
//! `~/.config/environment.d/90-voltkit.conf`. systemd reads that folder at
//! login, so the variables reach every app started in the session.

use std::path::PathBuf;

use anyhow::{Context, Result};

const FILE_NAME: &str = "90-voltkit.conf";
const HEADER: &str = "# Environment variables set in Voltkit. Applied at login.\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Var {
    pub name: String,
    pub value: String,
}

pub fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("environment.d").join(FILE_NAME))
}

/// Names a shell and systemd both accept: a letter or underscore, then letters,
/// digits and underscores.
pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Variables that wire the running session together (bus addresses, display
/// sockets, session IDs). They're set per login, and overriding them breaks the
/// session, so the editor doesn't offer them.
pub fn is_session_plumbing(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY", "XAUTHORITY", "SESSION_MANAGER",
        "DESKTOP_STARTUP_ID", "XDG_ACTIVATION_TOKEN", "INVOCATION_ID", "JOURNAL_STREAM", "MANAGERPID",
        "SYSTEMD_EXEC_PID", "PWD", "OLDPWD", "SHLVL", "_", "SSH_AUTH_SOCK", "GPG_AGENT_INFO", "USER",
        "LOGNAME", "HOME", "SHELL",
    ];
    const PREFIXES: &[&str] = &["XDG_SESSION_", "XDG_RUNTIME_DIR", "XDG_SEAT", "XDG_VTNR", "GNOME_", "GDM", "GIO_LAUNCHED", "VOLTKIT_"];
    EXACT.contains(&name) || PREFIXES.iter().any(|p| name.starts_with(p))
}

pub fn parse(text: &str) -> Vec<Var> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .filter(|(name, _)| valid_name(name.trim()))
        .map(|(name, value)| Var { name: name.trim().to_owned(), value: value.to_owned() })
        .collect()
}

pub fn render(vars: &[Var]) -> String {
    let mut out = HEADER.to_owned();
    for v in vars {
        out.push_str(&format!("{}={}\n", v.name, v.value));
    }
    out
}

pub fn load() -> Result<Vec<Var>> {
    let Some(path) = path().filter(|p| p.exists()) else { return Ok(Vec::new()) };
    Ok(parse(&std::fs::read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?))
}

pub fn save(vars: &[Var]) -> Result<PathBuf> {
    let path = path().context("no config directory")?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, render(vars))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_skips_comments() {
        let vars = vec![
            Var { name: "EDITOR".into(), value: "nvim".into() },
            Var { name: "PATH".into(), value: "${PATH}:${HOME}/bin".into() },
        ];
        let text = render(&vars);
        assert!(text.starts_with('#'));
        assert_eq!(parse(&text), vars);
        assert_eq!(parse("# note\n\n  GOPATH=/opt/go  \nnot a var\n1BAD=x\n").len(), 1);
    }

    #[test]
    fn values_keep_equals_signs() {
        assert_eq!(parse("JAVA_OPTS=-Xmx=2g\n")[0].value, "-Xmx=2g");
    }

    #[test]
    fn hides_session_plumbing() {
        assert!(is_session_plumbing("DBUS_SESSION_BUS_ADDRESS") && is_session_plumbing("XDG_SESSION_TYPE"));
        assert!(!is_session_plumbing("PATH") && !is_session_plumbing("EDITOR") && !is_session_plumbing("LANG"));
    }

    #[test]
    fn validates_names() {
        assert!(valid_name("PATH") && valid_name("_X1") && valid_name("my_var"));
        assert!(!valid_name("") && !valid_name("1ABC") && !valid_name("MY-VAR") && !valid_name("A B"));
    }
}
