//! Hotkey actions, shared by the daemon and `voltkit trigger`.
//!
//! These run without a terminal, so results and errors are reported as notifications.

use anyhow::Result;
use clap::ValueEnum;

use crate::config::Config;
use crate::modules::{color, ocr, qr};
use crate::portal::{self, Cancelled};
use crate::system;
use crate::i18n::tr;

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
    /// Measure distances on screen
    Ruler,
    /// Paste the clipboard converted
    SmartPaste,
}

impl Action {
    pub const ALL: [Action; 6] = [Action::Ocr, Action::Color, Action::Snippets, Action::Qr, Action::Ruler, Action::SmartPaste];

    pub fn id(self) -> &'static str {
        match self {
            Action::Ocr => "ocr",
            Action::Color => "color",
            Action::Snippets => "snippets",
            Action::Qr => "qr",
            Action::Ruler => "ruler",
            Action::SmartPaste => "smart-paste",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.id() == id)
    }

    pub fn description(self) -> &'static str {
        match self {
            Action::Ocr => tr("Extract text from screen"),
            Action::Color => tr("Pick a color from screen"),
            Action::Snippets => tr("Insert a snippet"),
            Action::Qr => tr("Scan a QR code on screen"),
            Action::Ruler => tr("Measure on screen"),
            Action::SmartPaste => tr("Paste the clipboard converted"),
        }
    }

    pub fn accelerator(self, config: &Config) -> &str {
        match self {
            Action::Ocr => &config.hotkeys.ocr,
            Action::Color => &config.hotkeys.color_picker,
            Action::Snippets => &config.hotkeys.snippets,
            Action::Qr => &config.hotkeys.qr,
            Action::Ruler => &config.hotkeys.ruler,
            Action::SmartPaste => &config.hotkeys.smart_paste,
        }
    }
}

/// Hotkey results are announced on the session bus, so an open control center
/// can show them: signal `Captured(kind, value)`, kind "ocr", "qr" or "color".
pub const CAPTURE_PATH: &str = "/io/github/arios4k/Voltkit/Captures";
pub const CAPTURE_INTERFACE: &str = "io.github.arios4k.Voltkit.Captures";
pub const CAPTURE_SIGNAL: &str = "Captured";

async fn announce(kind: &str, value: &str) {
    if let Ok(conn) = zbus::Connection::session().await {
        let _ = conn.emit_signal(None::<&str>, CAPTURE_PATH, CAPTURE_INTERFACE, CAPTURE_SIGNAL, &(kind, value)).await;
    }
}

/// Copies to the clipboard, returning a note for the notification on failure.
fn copy(text: &str) -> Option<String> {
    system::copy_to_clipboard(text).err().map(|e| trf!("\n\nNot copied: {}", e))
}

async fn extract_text(config: &Config) -> Result<()> {
    let text = ocr::extract_text(&config.ocr.lang, None).await?;
    let note = copy(&text);
    announce("ocr", &text).await;
    let mut preview: String = text.chars().take(160).collect();
    if preview.len() < text.len() {
        preview.push('…');
    }
    let title = if note.is_none() { tr("Text copied") } else { tr("Text extracted") };
    portal::notify(title, &(preview + note.as_deref().unwrap_or(""))).await
}

async fn pick_color() -> Result<()> {
    let c = portal::pick_color(None).await?;
    let note = copy(&c.to_hex());
    color::history::push(c)?;
    announce("color", &c.to_hex()).await;
    let title = match note {
        None => trf!("{} copied", c.to_hex()),
        Some(_) => c.to_hex(),
    };
    let body = format!("{}\n{}{}", c.to_rgb_string(), c.to_hsl_string(), note.as_deref().unwrap_or(""));
    portal::notify(&title, &body).await
}

async fn scan_qr() -> Result<()> {
    let text = qr::scan(None).await?.join("\n");
    let note = copy(&text);
    announce("qr", &text).await;
    let title = if note.is_none() { tr("QR code copied") } else { tr("QR code read") };
    portal::notify(title, &(text + note.as_deref().unwrap_or(""))).await
}

/// Opens the picker (a GTK window, so it runs as its own process) and waits for it to close.
/// With `snippet`, it opens straight on that snippet's parameter form.
pub async fn open_picker(snippet: Option<String>) -> Result<()> {
    let mut args = vec!["picker".to_owned()];
    if let Some(trigger) = snippet {
        args.extend(["--snippet".to_owned(), trigger]);
    }
    run_window_tool(args, N_!("snippet picker")).await
}

/// Runs `voltkit ARGS` for a tool with its own window and waits for it to close.
async fn run_window_tool(args: Vec<String>, name: &'static str) -> Result<()> {
    let exe = std::env::current_exe()?;
    let status = tokio::task::spawn_blocking(move || std::process::Command::new(exe).args(args).status()).await??;
    if !status.success() {
        anyhow::bail!("{}", trf!("{} exited with {}", tr(name), status));
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
        Action::Ruler => run_window_tool(vec!["ruler".into()], N_!("screen ruler")).await,
        Action::SmartPaste => run_window_tool(vec!["smart-paste".into()], N_!("Smart Paste")).await,
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
