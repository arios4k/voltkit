//! `voltkit doctor`: checks everything Voltkit relies on outside its own
//! binary and says how to install what's missing on this distro.

use std::collections::HashMap;
use std::process::Command;

use crate::config::Config;
use crate::{daemon, expander, portal, shortcuts, system};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distro {
    Apt,
    Dnf,
    Pacman,
    Zypper,
    Unknown,
}

/// Picks the package manager from /etc/os-release's ID and ID_LIKE.
pub fn detect_distro(os_release: &str) -> Distro {
    let mut ids = Vec::new();
    for line in os_release.lines() {
        if let Some(v) = line.strip_prefix("ID=").or_else(|| line.strip_prefix("ID_LIKE=")) {
            ids.extend(v.trim_matches('"').split_whitespace().map(str::to_owned));
        }
    }
    for id in ids {
        match id.as_str() {
            "debian" | "ubuntu" => return Distro::Apt,
            "fedora" | "rhel" | "centos" => return Distro::Dnf,
            "arch" => return Distro::Pacman,
            "opensuse" | "suse" | "opensuse-tumbleweed" | "opensuse-leap" => return Distro::Zypper,
            _ => {}
        }
    }
    Distro::Unknown
}

impl Distro {
    fn install_prefix(self) -> Option<&'static str> {
        match self {
            Distro::Apt => Some("sudo apt install"),
            Distro::Dnf => Some("sudo dnf install"),
            Distro::Pacman => Some("sudo pacman -S"),
            Distro::Zypper => Some("sudo zypper install"),
            Distro::Unknown => None,
        }
    }
}

/// Something installable that a check can point to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tool {
    Tesseract,
    TesseractLang(String),
    WlClipboard,
    Xclip,
    Oxipng,
    Jpegoptim,
    Cwebp,
    Ffmpeg,
}

impl Tool {
    /// The package providing this tool, or `None` when the distro doesn't ship it.
    pub fn package(&self, distro: Distro) -> Option<String> {
        use Distro::*;
        let p = |s: &str| Some(s.to_owned());
        match (self, distro) {
            (Tool::Tesseract, Apt) => p("tesseract-ocr"),
            (Tool::Tesseract, Zypper) => p("tesseract-ocr"),
            (Tool::Tesseract, _) => p("tesseract"),
            (Tool::TesseractLang(l), Apt) => Some(format!("tesseract-ocr-{l}")),
            (Tool::TesseractLang(l), Dnf) => Some(format!("tesseract-langpack-{l}")),
            (Tool::TesseractLang(l), Pacman) => Some(format!("tesseract-data-{l}")),
            (Tool::TesseractLang(l), _) => Some(format!("tesseract language data for \"{l}\"")),
            (Tool::WlClipboard, _) => p("wl-clipboard"),
            (Tool::Xclip, _) => p("xclip"),
            (Tool::Oxipng, Apt) => None,
            (Tool::Oxipng, _) => p("oxipng"),
            (Tool::Jpegoptim, _) => p("jpegoptim"),
            (Tool::Cwebp, Apt) => p("webp"),
            (Tool::Cwebp, Pacman) => p("libwebp"),
            (Tool::Cwebp, _) => p("libwebp-tools"),
            (Tool::Ffmpeg, Dnf) => p("ffmpeg-free"),
            (Tool::Ffmpeg, _) => p("ffmpeg"),
        }
    }

    /// How to get the tool when the distro has no package for it.
    fn fallback_command(&self) -> Option<&'static str> {
        match self {
            Tool::Oxipng => Some("cargo install oxipng"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// The feature doesn't work without it.
    Required,
    /// Part of a feature, or something with a fallback.
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fix {
    Install(Tool),
    Advice(String),
}

#[derive(Debug, Clone)]
pub struct Check {
    pub feature: &'static str,
    pub what: String,
    pub ok: bool,
    pub need: Need,
    pub fix: Option<Fix>,
}

pub struct Report {
    pub distro: Distro,
    pub checks: Vec<Check>,
}

impl Report {
    pub fn missing(&self, need: Need) -> impl Iterator<Item = &Check> {
        self.checks.iter().filter(move |c| !c.ok && c.need == need)
    }

    /// Commands that install everything missing, packages first.
    pub fn install_commands(&self) -> Vec<String> {
        let mut packages: Vec<String> = Vec::new();
        let mut commands: Vec<String> = Vec::new();
        for check in self.checks.iter().filter(|c| !c.ok) {
            let Some(Fix::Install(tool)) = &check.fix else { continue };
            match (tool.package(self.distro), self.distro.install_prefix()) {
                (Some(pkg), Some(_)) if !packages.contains(&pkg) => packages.push(pkg),
                (Some(_), Some(_)) => {}
                (Some(pkg), None) => commands.push(format!("install {pkg} with your package manager")),
                (None, _) => commands.extend(tool.fallback_command().map(str::to_owned)),
            }
        }
        let mut out = Vec::new();
        if let (false, Some(prefix)) = (packages.is_empty(), self.distro.install_prefix()) {
            out.push(format!("{prefix} {}", packages.join(" ")));
        }
        commands.dedup();
        out.extend(commands);
        out
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        let mut feature = "";
        for c in &self.checks {
            if c.feature != feature {
                feature = c.feature;
                out.push_str(&format!("\n{feature}\n"));
            }
            let mark = match (c.ok, c.need) {
                (true, _) => "✓",
                (false, Need::Required) => "✗",
                (false, Need::Optional) => "–",
            };
            out.push_str(&format!("  {mark} {}\n", c.what));
            if let (false, Some(Fix::Advice(advice))) = (c.ok, &c.fix) {
                out.push_str(&format!("      {advice}\n"));
            }
        }

        let required = self.missing(Need::Required).count();
        let optional = self.missing(Need::Optional).count();
        out.push('\n');
        if required + optional == 0 {
            out.push_str("Everything is in place.\n");
        } else {
            out.push_str(&format!(
                "{required} required and {optional} optional item(s) missing (✗ breaks a feature, – limits it).\n"
            ));
            let commands = self.install_commands();
            if !commands.is_empty() {
                out.push_str("\nTo install them:\n");
                for c in commands {
                    out.push_str(&format!("  {c}\n"));
                }
            }
        }
        out
    }
}

/// Versions of the portal interfaces Voltkit uses; absent ones are missing.
async fn portal_interfaces() -> HashMap<&'static str, u32> {
    let mut found = HashMap::new();
    let Ok(conn) = zbus::Connection::session().await else { return found };
    for iface in ["Screenshot", "RemoteDesktop", "GlobalShortcuts", "Inhibit"] {
        let reply = conn
            .call_method(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.DBus.Properties"),
                "Get",
                &(format!("org.freedesktop.portal.{iface}"), "version"),
            )
            .await;
        let version = reply
            .ok()
            .and_then(|r| r.body().deserialize::<zbus::zvariant::OwnedValue>().ok())
            .and_then(|v| u32::try_from(v).ok());
        if let Some(v) = version {
            found.insert(iface, v);
        }
    }
    found
}

fn tesseract_langs() -> Vec<String> {
    let Ok(out) = Command::new("tesseract").arg("--list-langs").output() else { return Vec::new() };
    // The first line is a header ("List of available languages in ...").
    String::from_utf8_lossy(&out.stdout).lines().skip(1).map(|l| l.trim().to_owned()).collect()
}

fn check(feature: &'static str, what: impl Into<String>, ok: bool, need: Need, fix: Option<Fix>) -> Check {
    Check { feature, what: what.into(), ok, need, fix }
}

/// Runs every check. Blocking; call it from a worker thread in the GUI.
pub fn run(config: &Config) -> Report {
    let distro = detect_distro(&std::fs::read_to_string("/etc/os-release").unwrap_or_default());
    let portals = portal::block_on(portal_interfaces());
    let wayland = system::is_wayland();
    let mut checks = Vec::new();

    // OCR
    const OCR: &str = "Text Extractor (OCR)";
    let tesseract = system::has("tesseract");
    checks.push(check(OCR, "tesseract", tesseract, Need::Required, Some(Fix::Install(Tool::Tesseract))));
    if tesseract {
        let installed = tesseract_langs();
        for lang in config.ocr.lang.split('+').filter(|l| !l.is_empty()) {
            checks.push(check(
                OCR,
                format!("language data: {lang}"),
                installed.iter().any(|l| l == lang),
                Need::Required,
                Some(Fix::Install(Tool::TesseractLang(lang.to_owned()))),
            ));
        }
    }
    let fallback_capture = ["grim", "gnome-screenshot", "spectacle", "maim"].into_iter().find(|t| system::has(t));
    let (capture_ok, capture_what) = match (portals.contains_key("Screenshot"), fallback_capture) {
        (true, _) => (true, "region capture: Screenshot portal".to_owned()),
        (false, Some(tool)) => (true, format!("region capture: {tool}")),
        (false, None) => (false, "region capture".to_owned()),
    };
    checks.push(check(
        OCR,
        capture_what,
        capture_ok,
        Need::Required,
        Some(Fix::Advice("install xdg-desktop-portal and your desktop's portal backend".into())),
    ));

    // Color picker
    const COLOR: &str = "Color Picker";
    checks.push(check(
        COLOR,
        "screen eyedropper: Screenshot portal",
        portals.contains_key("Screenshot"),
        Need::Required,
        Some(Fix::Advice("install xdg-desktop-portal and your desktop's portal backend".into())),
    ));

    // Clipboard
    const CLIPBOARD: &str = "Clipboard (hotkeys and command line)";
    let (tool, name) = if wayland { (Tool::WlClipboard, "wl-copy") } else { (Tool::Xclip, "xclip") };
    checks.push(check(CLIPBOARD, name, system::has(name), Need::Required, Some(Fix::Install(tool))));

    // Compressor
    const COMPRESS: &str = "Asset Compressor";
    for (program, format, tool) in [
        ("oxipng", "PNG", Tool::Oxipng),
        ("jpegoptim", "JPEG", Tool::Jpegoptim),
        ("cwebp", "WebP", Tool::Cwebp),
        ("ffmpeg", "MP4", Tool::Ffmpeg),
    ] {
        checks.push(check(COMPRESS, format!("{program} ({format})"), system::has(program), Need::Optional, Some(Fix::Install(tool))));
    }

    // Text expansion
    const EXPAND: &str = "Text Expansion";
    checks.push(check(
        EXPAND,
        "picker pastes for you: RemoteDesktop portal",
        portals.contains_key("RemoteDesktop"),
        Need::Optional,
        Some(Fix::Advice("without it the snippet is copied and you press Ctrl+V".into())),
    ));
    if config.expander.typed_triggers {
        let missing = expander::missing_permissions();
        checks.push(check(
            EXPAND,
            "typed triggers: keyboard access",
            missing.is_empty(),
            Need::Required,
            Some(Fix::Advice(expander::next_step().to_owned())),
        ));
    } else {
        checks.push(check(
            EXPAND,
            "typed triggers (off)",
            false,
            Need::Optional,
            Some(Fix::Advice("turn on with `voltkit expander setup`, then `voltkit expander enable`".into())),
        ));
    }

    // Background service
    // Keep Awake and Hosts & Environment
    const AWAKE: &str = "Keep Awake";
    checks.push(check(
        AWAKE,
        "stop sleeping and locking: Inhibit portal",
        portals.contains_key("Inhibit"),
        Need::Required,
        Some(Fix::Advice("install xdg-desktop-portal and your desktop's portal backend".into())),
    ));
    const SYSTEM: &str = "Hosts & Environment";
    checks.push(check(
        SYSTEM,
        "saving /etc/hosts: pkexec",
        system::has("pkexec"),
        Need::Optional,
        Some(Fix::Advice("install polkit (pkexec) to save the hosts file; environment variables work without it".into())),
    ));

    const SERVICE: &str = "Background Service";
    let status = daemon::status();
    checks.push(check(
        SERVICE,
        "daemon running",
        status.is_some(),
        Need::Required,
        Some(Fix::Advice("start it with `voltkit daemon enable-autostart`, then log in again or run `voltkit daemon`".into())),
    ));
    let (shortcuts_ok, shortcuts_what) = if portals.contains_key("GlobalShortcuts") {
        (true, "hotkeys: GlobalShortcuts portal".to_owned())
    } else if shortcuts::gnome_installed() {
        (true, "hotkeys: GNOME custom shortcuts".to_owned())
    } else {
        (false, "hotkeys".to_owned())
    };
    checks.push(check(
        SERVICE,
        shortcuts_what,
        shortcuts_ok,
        Need::Required,
        Some(Fix::Advice(
            "on GNOME run `voltkit shortcuts install`; elsewhere bind `voltkit trigger ocr|color|snippets`".into(),
        )),
    ));

    Report { distro, checks }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_distros_through_id_like() {
        assert_eq!(detect_distro("ID=zorin\nID_LIKE=\"ubuntu debian\"\n"), Distro::Apt);
        assert_eq!(detect_distro("ID=fedora\n"), Distro::Dnf);
        assert_eq!(detect_distro("ID=endeavouros\nID_LIKE=arch\n"), Distro::Pacman);
        assert_eq!(detect_distro("ID=\"opensuse-tumbleweed\"\nID_LIKE=\"opensuse suse\"\n"), Distro::Zypper);
        assert_eq!(detect_distro("ID=nixos\n"), Distro::Unknown);
    }

    #[test]
    fn package_names_follow_each_distro() {
        assert_eq!(Tool::Tesseract.package(Distro::Apt).unwrap(), "tesseract-ocr");
        assert_eq!(Tool::TesseractLang("deu".into()).package(Distro::Dnf).unwrap(), "tesseract-langpack-deu");
        assert_eq!(Tool::Cwebp.package(Distro::Apt).unwrap(), "webp");
        assert_eq!(Tool::Oxipng.package(Distro::Apt), None);
    }

    fn report(distro: Distro, missing: Vec<Tool>) -> Report {
        let checks = missing
            .into_iter()
            .map(|t| check("F", format!("{t:?}"), false, Need::Required, Some(Fix::Install(t))))
            .collect();
        Report { distro, checks }
    }

    #[test]
    fn combines_packages_into_one_command() {
        let r = report(Distro::Apt, vec![Tool::Tesseract, Tool::WlClipboard, Tool::Oxipng, Tool::Tesseract]);
        assert_eq!(
            r.install_commands(),
            vec!["sudo apt install tesseract-ocr wl-clipboard".to_owned(), "cargo install oxipng".to_owned()]
        );
    }

    #[test]
    fn unknown_distro_names_packages() {
        let r = report(Distro::Unknown, vec![Tool::Jpegoptim]);
        assert_eq!(r.install_commands(), vec!["install jpegoptim with your package manager".to_owned()]);
    }

    #[test]
    fn render_marks_and_summarises() {
        let mut r = report(Distro::Pacman, vec![Tool::Ffmpeg]);
        r.checks.push(check("F", "ok thing", true, Need::Required, None));
        let text = r.render();
        assert!(text.contains("✗ Ffmpeg"));
        assert!(text.contains("✓ ok thing"));
        assert!(text.contains("sudo pacman -S ffmpeg"));
        assert!(report(Distro::Apt, vec![]).render().contains("Everything is in place."));
    }
}
