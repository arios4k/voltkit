//! Inserting text into whichever app has focus.
//!
//! Wayland has no API for typing into other apps, so expansions go through the
//! clipboard: copy the text, press the paste shortcut, then put the user's
//! previous clipboard back. The keypress comes from the RemoteDesktop portal
//! (snippet picker, no setup needed) or from a uinput virtual keyboard (typed
//! triggers, see `expander.rs`).

use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use anyhow::{Result, bail};
use ashpd::desktop::PersistMode;
use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions};
use ashpd::enumflags2::BitFlags;
use evdev::KeyCode;

use crate::portal::Cancelled;
use crate::system;

/// How long the target app gets to read the clipboard before it is restored.
const PASTE_SETTLE: Duration = Duration::from_millis(350);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteShortcut {
    CtrlV,
    CtrlShiftV,
    ShiftInsert,
}

impl FromStr for PasteShortcut {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s.trim().to_ascii_lowercase().replace(' ', "").as_str() {
            "ctrl+v" => Self::CtrlV,
            "ctrl+shift+v" | "shift+ctrl+v" => Self::CtrlShiftV,
            "shift+insert" => Self::ShiftInsert,
            other => bail!("unknown paste_shortcut {other:?} (use ctrl+v, ctrl+shift+v or shift+insert)"),
        })
    }
}

impl PasteShortcut {
    /// Keys in press order (X keysyms, for the portal).
    fn keysyms(self) -> &'static [i32] {
        const CTRL: i32 = 0xffe3;
        const SHIFT: i32 = 0xffe1;
        const INSERT: i32 = 0xff63;
        const V: i32 = 0x76;
        match self {
            Self::CtrlV => &[CTRL, V],
            Self::CtrlShiftV => &[CTRL, SHIFT, V],
            Self::ShiftInsert => &[SHIFT, INSERT],
        }
    }

    /// Keys in press order (evdev codes, for uinput).
    pub fn keycodes(self) -> &'static [KeyCode] {
        match self {
            Self::CtrlV => &[KeyCode::KEY_LEFTCTRL, KeyCode::KEY_V],
            Self::CtrlShiftV => &[KeyCode::KEY_LEFTCTRL, KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_V],
            Self::ShiftInsert => &[KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_INSERT],
        }
    }
}

fn token_path() -> Option<PathBuf> {
    dirs::state_dir().map(|d| d.join("voltkit").join("remote-desktop-token"))
}

fn map_portal_err(e: ashpd::Error) -> anyhow::Error {
    match e {
        ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled) => Cancelled.into(),
        other => anyhow::Error::new(other),
    }
}

/// Presses `shortcut` through the RemoteDesktop portal. The desktop asks for
/// permission the first time; the grant is remembered with a restore token.
pub async fn press_with_portal(shortcut: PasteShortcut) -> Result<()> {
    let token = token_path().and_then(|p| std::fs::read_to_string(p).ok());
    let portal = RemoteDesktop::new().await?;
    let session = portal.create_session(Default::default()).await?;
    let options = SelectDevicesOptions::default()
        .set_devices(BitFlags::from(DeviceType::Keyboard))
        .set_persist_mode(PersistMode::ExplicitlyRevoked)
        .set_restore_token(token.as_deref().map(str::trim));
    portal.select_devices(&session, options).await.map_err(map_portal_err)?;
    let started = portal
        .start(&session, None, Default::default())
        .await
        .map_err(map_portal_err)?
        .response()
        .map_err(map_portal_err)?;
    if let (Some(new_token), Some(path)) = (started.restore_token(), token_path()) {
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, new_token)?;
    }

    let keys = shortcut.keysyms();
    for &k in keys {
        portal.notify_keyboard_keysym(&session, k, KeyState::Pressed, Default::default()).await?;
    }
    for &k in keys.iter().rev() {
        portal.notify_keyboard_keysym(&session, k, KeyState::Released, Default::default()).await?;
    }
    let _ = session.close().await;
    Ok(())
}

/// Copies `text`, runs `press` to trigger the paste, then restores `previous`.
/// If pressing fails the text stays on the clipboard so the user can paste it.
pub async fn paste<F, Fut>(text: &str, previous: Option<String>, press: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    system::copy_to_clipboard(text)?;
    press().await?;
    tokio::time::sleep(PASTE_SETTLE).await;
    if let Some(previous) = previous {
        system::copy_to_clipboard(&previous)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shortcuts() {
        assert_eq!("Ctrl+V".parse::<PasteShortcut>().unwrap(), PasteShortcut::CtrlV);
        assert_eq!("shift + insert".parse::<PasteShortcut>().unwrap(), PasteShortcut::ShiftInsert);
        assert_eq!("shift+ctrl+v".parse::<PasteShortcut>().unwrap(), PasteShortcut::CtrlShiftV);
        assert!("cmd+v".parse::<PasteShortcut>().is_err());
    }

    #[test]
    fn keysyms_and_keycodes_agree() {
        for s in [PasteShortcut::CtrlV, PasteShortcut::CtrlShiftV, PasteShortcut::ShiftInsert] {
            assert_eq!(s.keysyms().len(), s.keycodes().len());
        }
    }
}
