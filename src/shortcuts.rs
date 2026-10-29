//! Global shortcut registration.
//!
//! The preferred backend is the `org.freedesktop.portal.GlobalShortcuts` portal
//! (see `daemon.rs`). Desktops without it (GNOME < 48, xdg-desktop-portal < 1.19)
//! can opt into custom keybindings in GNOME Settings that call `voltkit trigger`.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};

use crate::actions::Action;
use crate::config::Config;
use crate::system;

/// Converts a GTK accelerator (`<Super><Shift>t`) to the XDG shortcuts
/// format used by the portal (`LOGO+SHIFT+t`).
pub fn accel_to_portal(accel: &str) -> Result<String> {
    let mut parts = Vec::new();
    let mut rest = accel.trim();
    while let Some(tail) = rest.strip_prefix('<') {
        let Some(end) = tail.find('>') else { bail!("invalid accelerator {accel:?}") };
        parts.push(match tail[..end].to_ascii_lowercase().as_str() {
            "super" | "meta" | "logo" => "LOGO",
            "shift" => "SHIFT",
            "control" | "ctrl" | "primary" => "CTRL",
            "alt" => "ALT",
            m => bail!("unknown modifier <{m}> in {accel:?}"),
        });
        rest = &tail[end + 1..];
    }
    if rest.is_empty() {
        bail!("accelerator {accel:?} has no key");
    }
    Ok(parts.into_iter().chain([rest]).collect::<Vec<_>>().join("+"))
}

// ------------------------------------------------------------ GNOME fallback

const SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const KEY: &str = "custom-keybindings";
const BASE: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/";

fn binding_path(action: Action) -> String {
    format!("{BASE}voltkit-{}/", action.id())
}

fn is_ours(path: &str) -> bool {
    path.starts_with(&format!("{BASE}voltkit-"))
}

/// Parses a GVariant string array as printed by `gsettings get`.
fn parse_strv(text: &str) -> Vec<String> {
    let text = text.trim().trim_start_matches("@as").trim();
    let inner = text.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|s| s.trim().trim_matches(['\'', '"']).to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn format_strv(items: &[String]) -> String {
    let quoted: Vec<_> = items.iter().map(|s| format!("'{s}'")).collect();
    format!("[{}]", quoted.join(", "))
}

/// Quotes an argument for GLib's shell-style command parsing.
fn shell_quote(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "/._-+".contains(c)) {
        s.to_owned()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

/// The `gsettings` invocations that install Voltkit keybindings, given the
/// current value of the custom-keybindings list.
fn install_plan(current: &str, exe: &Path, config: &Config) -> Vec<Vec<String>> {
    let mut paths: Vec<String> = parse_strv(current).into_iter().filter(|p| !is_ours(p)).collect();
    let mut plan = Vec::new();
    for action in Action::ALL {
        let path = binding_path(action);
        let schema = format!("{SCHEMA}.custom-keybinding:{path}");
        let command = format!("{} trigger {}", shell_quote(&exe.to_string_lossy()), action.id());
        for (key, value) in [
            ("name", format!("Voltkit: {}", action.description())),
            ("command", command),
            ("binding", action.accelerator(config).to_owned()),
        ] {
            plan.push(vec!["set".into(), schema.clone(), key.into(), value]);
        }
        paths.push(path);
    }
    plan.push(vec!["set".into(), SCHEMA.into(), KEY.into(), format_strv(&paths)]);
    plan
}

fn remove_plan(current: &str) -> Vec<Vec<String>> {
    let all = parse_strv(current);
    let kept: Vec<String> = all.iter().filter(|p| !is_ours(p)).cloned().collect();
    let mut plan = Vec::new();
    for path in all.iter().filter(|p| is_ours(p)) {
        plan.push(vec!["reset-recursively".into(), format!("{SCHEMA}.custom-keybinding:{path}")]);
    }
    plan.push(vec!["set".into(), SCHEMA.into(), KEY.into(), format_strv(&kept)]);
    plan
}

fn current_bindings() -> Result<String> {
    if !system::has("gsettings") {
        bail!("gsettings not found; this fallback is for GNOME-based desktops");
    }
    let out = system::run(Command::new("gsettings").args(["get", SCHEMA, KEY]))?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

fn apply(plan: Vec<Vec<String>>, dry_run: bool) -> Result<()> {
    for args in plan {
        if dry_run {
            let shown: Vec<_> = args.iter().map(|a| shell_quote(a)).collect();
            println!("gsettings {}", shown.join(" "));
        } else {
            system::run(Command::new("gsettings").args(&args))?;
        }
    }
    Ok(())
}

pub fn gnome_install(config: &Config, dry_run: bool) -> Result<()> {
    let exe = std::env::current_exe()?;
    apply(install_plan(&current_bindings()?, &exe, config), dry_run)
}

pub fn gnome_remove(dry_run: bool) -> Result<()> {
    apply(remove_plan(&current_bindings()?), dry_run)
}

pub fn gnome_installed() -> bool {
    current_bindings().map(|b| parse_strv(&b).iter().any(|p| is_ours(p))).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_accelerators() {
        assert_eq!(accel_to_portal("<Super><Shift>t").unwrap(), "LOGO+SHIFT+t");
        assert_eq!(accel_to_portal("<Primary><Alt>Delete").unwrap(), "CTRL+ALT+Delete");
        assert_eq!(accel_to_portal("F9").unwrap(), "F9");
        assert!(accel_to_portal("<Super>").is_err());
        assert!(accel_to_portal("<Hyper>x").is_err());
        assert!(accel_to_portal("<Super x").is_err());
    }

    #[test]
    fn parses_gsettings_arrays() {
        assert!(parse_strv("@as []\n").is_empty());
        assert_eq!(parse_strv("['/a/', '/b/']\n"), vec!["/a/", "/b/"]);
        assert_eq!(format_strv(&["/a/".into()]), "['/a/']");
    }

    #[test]
    fn install_keeps_other_bindings_and_is_idempotent() {
        let current = format!("['{BASE}custom0/', '{BASE}voltkit-ocr/']");
        let plan = install_plan(&current, Path::new("/opt/volt kit/voltkit"), &Config::default());
        let list = &plan.last().unwrap()[3];
        assert_eq!(
            *list,
            format!(
                "['{BASE}custom0/', '{BASE}voltkit-ocr/', '{BASE}voltkit-color/', '{BASE}voltkit-snippets/', '{BASE}voltkit-qr/']"
            )
        );
        let command = plan.iter().find(|a| a[2] == "command").unwrap();
        assert_eq!(command[3], "'/opt/volt kit/voltkit' trigger ocr");
        let binding = plan.iter().find(|a| a[2] == "binding").unwrap();
        assert_eq!(binding[3], "<Super><Shift>t");
    }

    #[test]
    fn remove_only_touches_ours() {
        let current = format!("['{BASE}custom0/', '{BASE}voltkit-color/']");
        let plan = remove_plan(&current);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0][0], "reset-recursively");
        assert_eq!(plan[1][3], format!("['{BASE}custom0/']"));
    }
}
