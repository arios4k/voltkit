//! Hotkey actions, shared by the daemon and `voltkit trigger`.
//!
//! These run without a terminal, so results and errors are reported as notifications.

use anyhow::Result;
use clap::ValueEnum;

use crate::config::Config;
use crate::modules::{color, ocr, qr};
use crate::portal::{self, Cancelled};
use crate::system;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Action {
    /// Extract text from a screen region
    Ocr,
    /// Pick a color from the screen
    Color,
    /// Open the snippet picker
    Snippets,
    /// Read a QR code on screen
    Qr,
}

impl Action {
    pub const ALL: [Action; 4] = [Action::Ocr, Action::Color, Action::Snippets, Action::Qr];

    pub fn id(self) -> &'static str {
        match self {
            Action::Ocr => "ocr",
            Action::Color => "color",
            Action::Snippets => "snippets",
            Action::Qr => "qr",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.id() == id)
    }

    pub fn description(self) -> &'static str {
        match self {
            Action::Ocr => "Extract text from screen",
            Action::Color => "Pick a color from screen",
            Action::Snippets => "Insert a snippet",
            Action::Qr => "Scan a QR code on screen",
        }
    }

    pub fn accelerator(self, config: &Config) -> &str {
        match self {
            Action::Ocr => &config.hotkeys.ocr,
            Action::Color => &config.hotkeys.color_picker,
            Action::Snippets => &config.hotkeys.snippets,
            Action::Qr => &config.hotkeys.qr,
        }
    }
}

/// Copies to the clipboard, returning a note for the notification on failure.
fn copy(text: &str) -> Option<String> {
    system::copy_to_clipboard(text).err().map(|e| format!("\n\nNot copied: {e}"))
}

async fn extract_text(config: &Config) -> Result<()> {
    let text = ocr::extract_text(&config.ocr.lang).await?;
    let note = copy(&text);
    let mut preview: String = text.chars().take(160).collect();
    if preview.len() < text.len() {
        preview.push('…');
    }
    let title = if note.is_none() { "Text copied" } else { "Text extracted" };
    portal::notify(title, &(preview + note.as_deref().unwrap_or(""))).await
}

async fn pick_color() -> Result<()> {
    let c = portal::pick_color().await?;
    let note = copy(&c.to_hex());
    color::history::push(c)?;
    let title = match note {
        None => format!("{} copied", c.to_hex()),
        Some(_) => c.to_hex(),
    };
    let body = format!("{}\n{}{}", c.to_rgb_string(), c.to_hsl_string(), note.as_deref().unwrap_or(""));
    portal::notify(&title, &body).await
}

async fn scan_qr() -> Result<()> {
    let text = qr::scan().await?.join("\n");
    let note = copy(&text);
    let title = if note.is_none() { "QR code copied" } else { "QR code read" };
    portal::notify(title, &(text + note.as_deref().unwrap_or(""))).await
}

/// Opens the picker (a GTK window, so it runs as its own process) and waits for it to close.
/// With `snippet`, it opens straight on that snippet's parameter form.
pub async fn open_picker(snippet: Option<String>) -> Result<()> {
    let exe = std::env::current_exe()?;
    let status = tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("picker");
        if let Some(trigger) = snippet {
            cmd.args(["--snippet", &trigger]);
        }
        cmd.status()
    })
    .await??;
    if !status.success() {
        anyhow::bail!("snippet picker exited with {status}");
    }
    Ok(())
}

/// Runs an action and reports any failure as a notification.
pub async fn run(action: Action, config: &Config) {
    let result = match action {
        Action::Ocr => extract_text(config).await,
        Action::Color => pick_color().await,
        Action::Snippets => open_picker(None).await,
        Action::Qr => scan_qr().await,
    };
    match result {
        Ok(()) => {}
        Err(e) if e.is::<Cancelled>() => {}
        Err(e) => {
            eprintln!("{}: {e:#}", action.id());
            let _ = portal::notify(action.description(), &format!("{e:#}")).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for a in Action::ALL {
            assert_eq!(Action::from_id(a.id()), Some(a));
        }
        assert_eq!(Action::from_id("nope"), None);
    }
}
