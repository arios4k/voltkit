//! Running as a snap from the Snap Store: which permissions (snap
//! "interfaces") each tool needs, which features the sandbox can't allow,
//! and the commands that fix what can be fixed.
//!
//! Strictly confined snaps can't ask for the administrator password
//! (pkexec) or change system files, so features that need either work only
//! in the APT and Fedora packages. Everything else works once the user
//! connects the interfaces that aren't connected automatically.

use std::path::PathBuf;

use crate::i18n::tr;

/// Where the manual explains how to install the full version.
pub const INSTALL_URL: &str = "https://arios4k.github.io/voltkit/manual/install.html";

/// True when Voltkit runs from a snap. `VOLTKIT_SNAP_PREVIEW` shows the
/// snap notices outside a snap, for screenshots and testing.
pub fn snap() -> bool {
    std::env::var_os("SNAP_NAME").is_some() || std::env::var_os("VOLTKIT_SNAP_PREVIEW").is_some()
}

/// The snap's name as typed in `snap connect` (normally "voltkit").
pub fn snap_name() -> String {
    std::env::var("SNAP_INSTANCE_NAME").or_else(|_| std::env::var("SNAP_NAME")).unwrap_or_else(|_| "voltkit".into())
}

/// The command the desktop should run to start Voltkit (in autostart entries
/// and keyboard shortcuts). Inside a snap the binary's own path isn't
/// runnable from outside the sandbox; `/snap/bin/NAME` is.
pub fn host_exe() -> std::io::Result<PathBuf> {
    if std::env::var_os("SNAP_NAME").is_some() {
        return Ok(PathBuf::from("/snap/bin").join(snap_name()));
    }
    std::env::current_exe()
}

/// A snap interface a tool needs, and what it lets the tool do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plug {
    pub name: &'static str,
    /// Untranslated; pass through `tr`.
    pub enables: &'static str,
}

/// What a tool needs in the snap, by its page id.
#[derive(Debug, Clone, Copy, Default)]
pub struct Limits {
    pub plugs: &'static [Plug],
    /// What only the APT and Fedora versions can do (untranslated).
    pub full_version_only: Option<&'static str>,
    /// Why the snap can't do it (untranslated).
    pub reason: Option<&'static str>,
    /// The whole tool is unavailable, so its page is shown read-only.
    pub whole_tool: bool,
}

const REMOVABLE: Plug = Plug { name: "removable-media", enables: N_!("use files on USB drives and other external disks") };
const SYSTEM_OBSERVE: Plug = Plug { name: "system-observe", enables: N_!("see which apps have files, folders and ports open") };
const MOUNT_OBSERVE: Plug = Plug { name: "mount-observe", enables: N_!("list the drives that are connected") };
const PROCESS_CONTROL: Plug = Plug { name: "process-control", enables: N_!("close the apps that keep something busy") };
const LOG_OBSERVE: Plug = Plug { name: "log-observe", enables: N_!("read the sleep history from the system log") };
const SESSION_OBSERVE: Plug = Plug { name: "login-session-observe", enables: N_!("see what is keeping the computer awake") };
const HARDWARE_OBSERVE: Plug = Plug { name: "hardware-observe", enables: N_!("read hardware details, temperatures and fans") };
const SYSTEM_STATE: Plug = Plug { name: "system-observe", enables: N_!("read drivers, the kernel and the system's state") };

pub fn limits(page: &str) -> Limits {
    match page {
        "compress" | "rename" | "resize" => Limits { plugs: &[REMOVABLE], ..Default::default() },
        "using" => Limits {
            plugs: &[SYSTEM_OBSERVE, MOUNT_OBSERVE, PROCESS_CONTROL],
            full_version_only: Some(N_!("Seeing and closing apps that belong to other users or to the system")),
            reason: Some(N_!("Other users' and the system's apps are visible only to the administrator, and closing them needs the administrator password, which snaps can't ask for.")),
            ..Default::default()
        },
        "sleep" => Limits {
            plugs: &[LOG_OBSERVE, SESSION_OBSERVE],
            full_version_only: Some(N_!("Stopping a USB device from waking the computer")),
            reason: Some(N_!("Which devices may wake the computer is a system setting: changing it, and keeping it after a restart, needs the administrator password, which snaps can't ask for.")),
            ..Default::default()
        },
        "sysreport" => Limits {
            plugs: &[HARDWARE_OBSERVE, SYSTEM_STATE, MOUNT_OBSERVE],
            full_version_only: Some(N_!("Drive health, memory slots and firmware update checks")),
            reason: Some(N_!("Drive health comes from the system's disk service, which snaps aren't allowed to query; memory slot details are in a table only the administrator can read; and firmware checks use the fwupd tool, which isn't inside the snap.")),
            ..Default::default()
        },
        "gpu" => Limits {
            plugs: &[HARDWARE_OBSERVE],
            full_version_only: Some(N_!(
                "Changing GPU settings (power limit, fans, clocks and voltage) and applying profiles at startup; monitoring works"
            )),
            reason: Some(N_!(
                "GPU settings are written to the graphics driver by a helper that runs as administrator after asking for your password. Snaps can't ask for it, and a profile applied at startup needs a system service, which a snap can't install."
            )),
            ..Default::default()
        },
        "system" => Limits {
            full_version_only: Some(N_!("Editing /etc/hosts and your session's environment variables")),
            reason: Some(N_!("/etc/hosts is a system file only the administrator can change, and snaps can't ask for your password. Environment variables are saved in your hidden settings folder (~/.config), which a snap can't write to: it only sees its own private copy.")),
            whole_tool: true,
            ..Default::default()
        },
        "permissions" => Limits {
            full_version_only: Some(N_!("Reviewing and changing the permissions you gave apps")),
            reason: Some(N_!("The answers you gave apps are kept by the desktop's permission store, a service snaps aren't allowed to talk to.")),
            whole_tool: true,
            ..Default::default()
        },
        "snippets" => Limits {
            full_version_only: Some(N_!("Typed triggers (typing a word such as ;date in any app); the snippet picker works")),
            reason: Some(N_!("Typed triggers read the keyboard and type for you, which needs a one-time setup by the administrator (a rule in /etc/udev). Snaps can't make that change, and the Snap Store reserves keyboard access for apps it reviews by hand.")),
            ..Default::default()
        },
        _ => Limits::default(),
    }
}

/// Whether a plug is connected. Only meaningful inside a snap; in preview
/// mode `VOLTKIT_SNAP_CONNECTED` lists the connected plugs (comma-separated).
pub fn connected(plug: &str) -> bool {
    if std::env::var_os("SNAP_NAME").is_none() {
        let list = std::env::var("VOLTKIT_SNAP_CONNECTED").unwrap_or_default();
        return list.split(',').any(|p| p.trim() == plug);
    }
    std::process::Command::new("snapctl")
        .args(["is-connected", plug])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// The tool's plugs that still need connecting, without duplicates.
pub fn missing(page: &str, is_connected: impl Fn(&str) -> bool) -> Vec<Plug> {
    let mut out: Vec<Plug> = Vec::new();
    for plug in limits(page).plugs {
        if !out.iter().any(|p| p.name == plug.name) && !is_connected(plug.name) {
            out.push(*plug);
        }
    }
    out
}

/// `sudo snap connect voltkit:PLUG`.
pub fn connect_command(plug: &str) -> String {
    format!("sudo snap connect {}:{plug}", snap_name())
}

/// Why some features need the APT or Fedora version, in one paragraph.
pub fn why_full_version() -> &'static str {
    tr("This is a limitation of snaps, not of Voltkit. Apps from the Snap Store run in a sandbox that can't ask for your \
        administrator password, change system files or talk to some desktop services. Installed with APT (Ubuntu, Debian \
        and derivatives) or DNF (Fedora), Voltkit has none of these limits.")
}

/// Every connect command Voltkit can use, for the manual and `voltkit doctor`.
pub fn all_plugs() -> Vec<Plug> {
    let mut out: Vec<Plug> = Vec::new();
    for page in ["compress", "using", "sleep", "sysreport", "gpu"] {
        for plug in limits(page).plugs {
            if !out.iter().any(|p| p.name == plug.name) {
                out.push(*plug);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_unconnected_plugs_once() {
        let missing = missing("sysreport", |p| p == "hardware-observe");
        let names: Vec<&str> = missing.iter().map(|p| p.name).collect();
        assert_eq!(names, ["system-observe", "mount-observe"]);
        assert!(super::missing("ocr", |_| false).is_empty());
    }

    #[test]
    fn root_only_features_are_named() {
        assert!(limits("gpu").full_version_only.is_some());
        assert!(limits("system").whole_tool);
        assert!(limits("snippets").full_version_only.is_some());
        assert!(limits("permissions").whole_tool);
        assert!(limits("qr").full_version_only.is_none());
        // Every limit says why.
        for page in ["gpu", "system", "permissions", "snippets", "using", "sleep", "sysreport"] {
            assert!(limits(page).reason.is_some(), "{page} has no reason");
        }
    }

    #[test]
    fn every_plug_appears_once() {
        let plugs = all_plugs();
        let mut names: Vec<&str> = plugs.iter().map(|p| p.name).collect();
        let len = names.len();
        names.dedup();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len);
        assert!(names.contains(&"removable-media"));
    }

    #[test]
    fn connect_commands_name_the_snap() {
        assert!(connect_command("log-observe").ends_with(":log-observe"));
        assert!(connect_command("log-observe").starts_with("sudo snap connect "));
    }
}
