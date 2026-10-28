//! System-wide OCR: capture a screen region, then run Tesseract on it.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, bail};

use crate::system::{self, has};

/// Interactive region capture backends, tried in order.
fn region_capture_command(out: &Path) -> Option<Command> {
    let out = out.to_string_lossy().into_owned();
    if system::is_wayland() {
        if has("grim") && has("slurp") {
            // wlroots compositors (Sway, Hyprland, river, ...)
            let mut c = Command::new("sh");
            c.args(["-c", "grim -g \"$(slurp)\" \"$1\"", "sh", &out]);
            return Some(c);
        }
        if has("gnome-screenshot") {
            let mut c = Command::new("gnome-screenshot");
            c.args(["-a", "-f", &out]);
            return Some(c);
        }
        if has("spectacle") {
            let mut c = Command::new("spectacle");
            c.args(["-r", "-b", "-n", "-o", &out]);
            return Some(c);
        }
    } else if has("maim") {
        let mut c = Command::new("maim");
        c.args(["-s", &out]);
        return Some(c);
    }
    None
}

pub fn capture_region() -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("powerlinux-ocr-{}.png", std::process::id()));
    let Some(mut cmd) = region_capture_command(&path) else {
        bail!("no region screenshot tool found (install grim+slurp, gnome-screenshot, spectacle or maim)");
    };
    system::run(&mut cmd)?;
    if !path.exists() {
        bail!("capture cancelled");
    }
    Ok(path)
}

pub fn recognize(image: &Path, lang: &str) -> Result<String> {
    let out = system::run(Command::new("tesseract").arg(image).arg("stdout").args(["-l", lang]))?;
    Ok(clean(&String::from_utf8_lossy(&out)))
}

pub fn capture_and_recognize(lang: &str) -> Result<String> {
    let image = capture_region()?;
    let text = recognize(&image, lang);
    let _ = std::fs::remove_file(&image);
    text
}

/// Trims trailing whitespace and form feeds Tesseract appends to each page.
fn clean(raw: &str) -> String {
    raw.lines().map(str::trim_end).collect::<Vec<_>>().join("\n").trim_matches(['\n', '\x0c']).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_tesseract_output() {
        assert_eq!(clean("Hello  \nWorld\n\n\x0c"), "Hello\nWorld");
    }
}
