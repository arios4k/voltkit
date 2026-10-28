//! Typed triggers: type `;date` in any app and it is replaced by the expansion.
//!
//! Wayland lets no app see another app's keystrokes, so this reads keyboards
//! directly through evdev and replies through a uinput virtual keyboard: it
//! erases the trigger with Backspace and pastes the expansion (see `paste.rs`).
//! Both need the one-time setup from `powerlinux expander setup`.
//!
//! Privacy: only the last few characters are kept in memory, to match triggers.
//! Nothing typed is logged, stored or sent anywhere.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Device, EventType, InputEvent, KeyCode};
use tokio::sync::mpsc;

use crate::actions;
use crate::config::{Config, Snippet};
use crate::modules::snippets;
use crate::paste::{self, PasteShortcut};
use crate::system;

const VIRTUAL_KEYBOARD: &str = "PowerLinux virtual keyboard";
const BUFFER_LEN: usize = 64;
const RESCAN_EVERY: Duration = Duration::from_secs(5);
const KEY_DELAY: Duration = Duration::from_millis(4);

// ------------------------------------------------------------ setup

const UDEV_RULE_PATH: &str = "/etc/udev/rules.d/99-powerlinux-uinput.rules";
const UDEV_RULE: &str = r#"KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput""#;
const MODULES_PATH: &str = "/etc/modules-load.d/powerlinux-uinput.conf";

/// Shell commands (run as root) that let `user` read keyboards and create a virtual one.
pub fn setup_script(user: &str) -> String {
    format!(
        "set -e\n\
         usermod -aG input {user}\n\
         printf '%s\\n' '{UDEV_RULE}' > {UDEV_RULE_PATH}\n\
         printf 'uinput\\n' > {MODULES_PATH}\n\
         modprobe uinput\n\
         udevadm control --reload-rules\n\
         udevadm trigger --sysname-match=uinput\n"
    )
}

/// What still blocks typed triggers on this machine, if anything.
pub fn missing_permissions() -> Vec<String> {
    let mut missing = Vec::new();
    if std::fs::OpenOptions::new().write(true).open("/dev/uinput").is_err() {
        missing.push("cannot write /dev/uinput (needs the udev rule and the input group)".into());
    }
    if keyboards().next().is_none() {
        missing.push("cannot read any keyboard in /dev/input (needs the input group)".into());
    }
    missing
}

// ------------------------------------------------------------ key decoding

/// Maps a key to the character it types on a US layout.
fn key_char(key: KeyCode, shift: bool) -> Option<char> {
    let pair = |lower: char, upper: char| Some(if shift { upper } else { lower });
    let letter = |c: char| Some(if shift { c.to_ascii_uppercase() } else { c });
    match key {
        KeyCode::KEY_A => letter('a'),
        KeyCode::KEY_B => letter('b'),
        KeyCode::KEY_C => letter('c'),
        KeyCode::KEY_D => letter('d'),
        KeyCode::KEY_E => letter('e'),
        KeyCode::KEY_F => letter('f'),
        KeyCode::KEY_G => letter('g'),
        KeyCode::KEY_H => letter('h'),
        KeyCode::KEY_I => letter('i'),
        KeyCode::KEY_J => letter('j'),
        KeyCode::KEY_K => letter('k'),
        KeyCode::KEY_L => letter('l'),
        KeyCode::KEY_M => letter('m'),
        KeyCode::KEY_N => letter('n'),
        KeyCode::KEY_O => letter('o'),
        KeyCode::KEY_P => letter('p'),
        KeyCode::KEY_Q => letter('q'),
        KeyCode::KEY_R => letter('r'),
        KeyCode::KEY_S => letter('s'),
        KeyCode::KEY_T => letter('t'),
        KeyCode::KEY_U => letter('u'),
        KeyCode::KEY_V => letter('v'),
        KeyCode::KEY_W => letter('w'),
        KeyCode::KEY_X => letter('x'),
        KeyCode::KEY_Y => letter('y'),
        KeyCode::KEY_Z => letter('z'),
        KeyCode::KEY_1 => pair('1', '!'),
        KeyCode::KEY_2 => pair('2', '@'),
        KeyCode::KEY_3 => pair('3', '#'),
        KeyCode::KEY_4 => pair('4', '$'),
        KeyCode::KEY_5 => pair('5', '%'),
        KeyCode::KEY_6 => pair('6', '^'),
        KeyCode::KEY_7 => pair('7', '&'),
        KeyCode::KEY_8 => pair('8', '*'),
        KeyCode::KEY_9 => pair('9', '('),
        KeyCode::KEY_0 => pair('0', ')'),
        KeyCode::KEY_MINUS => pair('-', '_'),
        KeyCode::KEY_EQUAL => pair('=', '+'),
        KeyCode::KEY_LEFTBRACE => pair('[', '{'),
        KeyCode::KEY_RIGHTBRACE => pair(']', '}'),
        KeyCode::KEY_SEMICOLON => pair(';', ':'),
        KeyCode::KEY_APOSTROPHE => pair('\'', '"'),
        KeyCode::KEY_GRAVE => pair('`', '~'),
        KeyCode::KEY_BACKSLASH => pair('\\', '|'),
        KeyCode::KEY_COMMA => pair(',', '<'),
        KeyCode::KEY_DOT => pair('.', '>'),
        KeyCode::KEY_SLASH => pair('/', '?'),
        KeyCode::KEY_SPACE => Some(' '),
        _ => None,
    }
}

fn is_shift(key: KeyCode) -> bool {
    matches!(key, KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT)
}

/// Ctrl, Alt and Super: while held, keys are shortcuts, not text.
fn is_command_modifier(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::KEY_LEFTCTRL
            | KeyCode::KEY_RIGHTCTRL
            | KeyCode::KEY_LEFTALT
            | KeyCode::KEY_RIGHTALT
            | KeyCode::KEY_LEFTMETA
            | KeyCode::KEY_RIGHTMETA
    )
}

/// Tracks recently typed text and reports when it ends with a trigger.
#[derive(Default)]
pub struct TriggerBuffer {
    text: String,
    shift: u8,
    commands: u8,
}

impl TriggerBuffer {
    /// Feeds one key event (`value`: 1 press, 2 repeat, 0 release). Returns the
    /// matching snippet when the typed text now ends with its trigger.
    pub fn feed<'a>(&mut self, key: KeyCode, value: i32, snippets: &'a [Snippet]) -> Option<&'a Snippet> {
        if is_shift(key) || is_command_modifier(key) {
            let counter = if is_shift(key) { &mut self.shift } else { &mut self.commands };
            match value {
                1 => *counter += 1,
                0 => *counter = counter.saturating_sub(1),
                _ => {}
            }
            return None;
        }
        if value == 0 {
            return None;
        }
        if self.commands > 0 {
            // Ctrl+A, Alt+Tab, ... may move the cursor or switch apps.
            self.text.clear();
            return None;
        }
        match key {
            KeyCode::KEY_BACKSPACE => {
                self.text.pop();
                None
            }
            _ => match key_char(key, self.shift > 0) {
                Some(c) => {
                    self.text.push(c);
                    if self.text.len() > BUFFER_LEN {
                        let cut = self.text.len() - BUFFER_LEN;
                        self.text.drain(..cut);
                    }
                    let hit = snippets
                        .iter()
                        .filter(|s| !s.trigger.is_empty() && self.text.ends_with(&s.trigger))
                        .max_by_key(|s| s.trigger.len());
                    if hit.is_some() {
                        self.text.clear();
                    }
                    hit
                }
                // Enter, Tab, arrows, Escape, F-keys...: the cursor or context changed.
                None => {
                    self.text.clear();
                    None
                }
            },
        }
    }
}

// ------------------------------------------------------------ devices

fn is_keyboard(device: &Device) -> bool {
    device.name() != Some(VIRTUAL_KEYBOARD)
        && device
            .supported_keys()
            .is_some_and(|k| k.contains(KeyCode::KEY_A) && k.contains(KeyCode::KEY_ENTER))
}

fn keyboards() -> impl Iterator<Item = (PathBuf, Device)> {
    evdev::enumerate().filter(|(_, d)| is_keyboard(d))
}

fn virtual_keyboard() -> Result<VirtualDevice> {
    let mut keys = AttributeSet::<KeyCode>::new();
    for k in [
        KeyCode::KEY_BACKSPACE,
        KeyCode::KEY_LEFTCTRL,
        KeyCode::KEY_LEFTSHIFT,
        KeyCode::KEY_V,
        KeyCode::KEY_INSERT,
    ] {
        keys.insert(k);
    }
    VirtualDevice::builder()
        .context("cannot open /dev/uinput; run `powerlinux expander setup`")?
        .name(VIRTUAL_KEYBOARD)
        .with_keys(&keys)?
        .build()
        .context("cannot create the virtual keyboard")
}

async fn tap(device: &mut VirtualDevice, keys: &[KeyCode]) -> Result<()> {
    for &k in keys {
        device.emit(&[InputEvent::new(EventType::KEY.0, k.0, 1)])?;
        tokio::time::sleep(KEY_DELAY).await;
    }
    for &k in keys.iter().rev() {
        device.emit(&[InputEvent::new(EventType::KEY.0, k.0, 0)])?;
        tokio::time::sleep(KEY_DELAY).await;
    }
    Ok(())
}

/// Streams key events from one keyboard until it is unplugged.
fn watch(path: PathBuf, device: Device, tx: mpsc::UnboundedSender<(KeyCode, i32)>, open: Arc<Mutex<HashSet<PathBuf>>>) {
    tokio::spawn(async move {
        if let Ok(mut events) = device.into_event_stream() {
            while let Ok(event) = events.next_event().await {
                if event.event_type() == EventType::KEY
                    && tx.send((KeyCode::new(event.code()), event.value())).is_err()
                {
                    break;
                }
            }
        }
        open.lock().unwrap().remove(&path);
    });
}

fn watch_new_keyboards(tx: &mpsc::UnboundedSender<(KeyCode, i32)>, open: &Arc<Mutex<HashSet<PathBuf>>>) {
    for (path, device) in keyboards() {
        if open.lock().unwrap().insert(path.clone()) {
            watch(path, device, tx.clone(), open.clone());
        }
    }
}

// ------------------------------------------------------------ expansion

async fn expand(snippet: &Snippet, device: &mut VirtualDevice, shortcut: PasteShortcut) -> Result<()> {
    // Let the user's last key release reach the app before we erase.
    tokio::time::sleep(Duration::from_millis(30)).await;
    for _ in snippet.trigger.chars() {
        tap(device, &[KeyCode::KEY_BACKSPACE]).await?;
    }
    if !snippets::required_args(&snippet.template).is_empty() {
        // Parameters need the picker's form.
        return actions::open_picker(Some(snippet.trigger.clone())).await;
    }
    let previous = system::read_clipboard().ok();
    let ctx = snippets::Context { clipboard: previous.clone(), ..Default::default() };
    let text = snippets::expand(&snippet.template, &ctx)?;
    paste::paste(&text, previous, move || async move { tap(device, shortcut.keycodes()).await }).await
}

/// Runs until the daemon stops. Fails immediately if permissions are missing.
pub async fn run(config: Arc<Config>) -> Result<()> {
    let missing = missing_permissions();
    if !missing.is_empty() {
        bail!("{}; {}", missing.join("; "), next_step());
    }
    let shortcut: PasteShortcut = config.expander.paste_shortcut.parse()?;
    let mut device = virtual_keyboard()?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    let open = Arc::new(Mutex::new(HashSet::new()));
    watch_new_keyboards(&tx, &open);
    let mut rescan = tokio::time::interval(RESCAN_EVERY);
    let mut buffer = TriggerBuffer::default();
    println!("Typed triggers on ({} keyboard(s))", open.lock().unwrap().len());

    loop {
        tokio::select! {
            _ = rescan.tick() => watch_new_keyboards(&tx, &open),
            Some((key, value)) = rx.recv() => {
                if let Some(snippet) = buffer.feed(key, value, &config.snippets) {
                    let snippet = snippet.clone();
                    if let Err(e) = expand(&snippet, &mut device, shortcut).await {
                        eprintln!("{}: {e:#}", snippet.trigger);
                    }
                    // Drop keys typed during the expansion; they belong to the old context.
                    while rx.try_recv().is_ok() {}
                    buffer = TriggerBuffer::default();
                }
            }
        }
    }
}

fn udev_rule_installed() -> bool {
    Path::new(UDEV_RULE_PATH).exists()
}

/// What the user should do about missing permissions. After setup, the only
/// thing left is a new login, which picks up the input group.
pub fn next_step() -> &'static str {
    if udev_rule_installed() {
        "setup is done; log out and back in so your session joins the input group"
    } else {
        "run `powerlinux expander setup`"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippets() -> Vec<Snippet> {
        let s = |t: &str| Snippet { trigger: t.into(), template: "x".into() };
        vec![s(";d"), s(";date"), s(";sig")]
    }

    fn type_keys(buf: &mut TriggerBuffer, keys: &[KeyCode], list: &[Snippet]) -> Option<String> {
        let mut hit = None;
        for &k in keys {
            if let Some(s) = buf.feed(k, 1, list) {
                hit = Some(s.trigger.clone());
            }
            buf.feed(k, 0, list);
        }
        hit
    }

    use KeyCode as K;

    #[test]
    fn matches_trigger_at_end_of_text() {
        let list = snippets();
        let mut b = TriggerBuffer::default();
        assert_eq!(type_keys(&mut b, &[K::KEY_H, K::KEY_I, K::KEY_SPACE, K::KEY_SEMICOLON, K::KEY_S, K::KEY_I, K::KEY_G], &list), Some(";sig".into()));
    }

    #[test]
    fn shorter_trigger_fires_first() {
        // ";d" is a prefix of ";date", so it expands as soon as it is typed.
        let list = snippets();
        let mut b = TriggerBuffer::default();
        assert_eq!(type_keys(&mut b, &[K::KEY_SEMICOLON, K::KEY_D], &list), Some(";d".into()));
    }

    #[test]
    fn backspace_and_shift_are_respected() {
        let list = snippets();
        let mut b = TriggerBuffer::default();
        // ";sx<BS>ig" -> ";sig"
        assert!(type_keys(&mut b, &[K::KEY_SEMICOLON, K::KEY_S, K::KEY_X, K::KEY_BACKSPACE, K::KEY_I, K::KEY_G], &list).is_some());
        // Shift+; is ':', so ":sig" must not match.
        let mut b = TriggerBuffer::default();
        b.feed(K::KEY_LEFTSHIFT, 1, &list);
        b.feed(K::KEY_SEMICOLON, 1, &list);
        b.feed(K::KEY_LEFTSHIFT, 0, &list);
        assert!(type_keys(&mut b, &[K::KEY_S, K::KEY_I, K::KEY_G], &list).is_none());
    }

    #[test]
    fn shortcuts_and_navigation_reset_the_buffer() {
        let list = snippets();
        let mut b = TriggerBuffer::default();
        type_keys(&mut b, &[K::KEY_SEMICOLON, K::KEY_S], &list);
        b.feed(K::KEY_LEFTCTRL, 1, &list);
        b.feed(K::KEY_A, 1, &list);
        b.feed(K::KEY_LEFTCTRL, 0, &list);
        assert!(type_keys(&mut b, &[K::KEY_I, K::KEY_G], &list).is_none());

        let mut b = TriggerBuffer::default();
        type_keys(&mut b, &[K::KEY_SEMICOLON, K::KEY_S], &list);
        assert!(type_keys(&mut b, &[K::KEY_LEFT, K::KEY_I, K::KEY_G], &list).is_none());
    }

    #[test]
    fn setup_script_is_complete() {
        let script = setup_script("sam");
        assert!(script.contains("usermod -aG input sam"));
        assert!(script.contains(UDEV_RULE_PATH));
        assert!(script.contains("modprobe uinput"));
    }
}
