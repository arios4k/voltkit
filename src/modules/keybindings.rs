//! Shortcut Detective: every system-wide keyboard shortcut, who owns it,
//! which ones clash, free combinations, and changing them.
//!
//! GNOME and the desktops built like it (Cinnamon, Budgie, MATE) keep their
//! shortcuts in GSettings: the window manager's and shell's keybinding
//! schemas, media keys, custom shortcuts (relocatable schemas listed in
//! media-keys' custom-keybindings), and enabled shell extensions.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use gtk::gdk;
use gtk::gio;
use gtk::glib::translate::{FromGlib, IntoGlib};
use gtk::prelude::*;
use crate::i18n::tr;

const MEDIA_KEYS: &str = "org.gnome.settings-daemon.plugins.media-keys";
const CUSTOM: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";

/// One action with its shortcuts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub schema: String,
    /// For custom shortcuts, the schema's path.
    pub path: Option<String>,
    /// For extensions installed in the home folder: where their schema is.
    pub schema_dir: Option<std::path::PathBuf>,
    pub key: String,
    /// What it does, in words.
    pub name: String,
    /// Who it belongs to: "Windows", "GNOME Shell", "Voltkit", an extension…
    pub owner: String,
    pub accels: Vec<String>,
    pub defaults: Vec<String>,
    /// The key holds a list of shortcuts (most do) rather than one.
    pub list: bool,
    pub writable: bool,
}

impl Binding {
    /// The Voltkit action this is the hotkey of, if any.
    pub fn voltkit_action(&self) -> Option<&str> {
        self.path.as_deref()?.trim_end_matches('/').rsplit('/').next()?.strip_prefix("voltkit-")
    }

    pub fn is_default(&self) -> bool {
        self.path.is_some() || self.accels == self.defaults
    }
}

/// A key and modifiers, so "<Super>s", "<Super>S" and "<Mod4>s" compare equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Combo {
    pub keyval: u32,
    pub mods: u32,
}

/// Modifiers in the order they're written: GSettings names, and labels.
const MODIFIERS: [(gdk::ModifierType, &str, &str); 6] = [
    (gdk::ModifierType::SHIFT_MASK, "<Shift>", N_!("Shift")),
    (gdk::ModifierType::CONTROL_MASK, "<Control>", N_!("Ctrl")),
    (gdk::ModifierType::ALT_MASK, "<Alt>", N_!("Alt")),
    (gdk::ModifierType::SUPER_MASK, "<Super>", "Super"),
    (gdk::ModifierType::META_MASK, "<Meta>", "Meta"),
    (gdk::ModifierType::HYPER_MASK, "<Hyper>", "Hyper"),
];

/// Parses GSettings' "<Shift><Super>t" form. Done here rather than with
/// gtk::accelerator_parse, which needs a display (and so can't run in tests).
pub fn combo(accel: &str) -> Option<Combo> {
    let mut rest = accel.trim();
    let mut mods = gdk::ModifierType::empty();
    while let Some(after) = rest.strip_prefix('<') {
        let end = after.find('>')?;
        mods |= match after[..end].to_lowercase().as_str() {
            "shift" => gdk::ModifierType::SHIFT_MASK,
            "control" | "ctrl" | "ctl" | "primary" => gdk::ModifierType::CONTROL_MASK,
            "alt" | "mod1" => gdk::ModifierType::ALT_MASK,
            "super" | "mod4" => gdk::ModifierType::SUPER_MASK,
            "meta" => gdk::ModifierType::META_MASK,
            "hyper" => gdk::ModifierType::HYPER_MASK,
            _ => return None,
        };
        rest = &after[end + 1..];
    }
    let key = gdk::Key::from_name(rest)?;
    let keyval = key.to_lower().into_glib();
    // 0xFFFFFF is VoidSymbol: an unknown key name.
    (keyval != 0 && keyval != 0xFF_FFFF).then_some(Combo { keyval, mods: mods.bits() })
}

fn key_of(c: Combo) -> gdk::Key {
    // SAFETY: any number is a keyval; unknown ones are just named by their value.
    unsafe { gdk::Key::from_glib(c.keyval) }
}

/// "Super+Shift+S" for showing to people.
pub fn label(accel: &str) -> String {
    let Some(c) = combo(accel) else { return accel.to_owned() };
    let mods = gdk::ModifierType::from_bits_truncate(c.mods);
    let key = key_of(c);
    let key_label = match key.to_upper().to_unicode().filter(|ch| !ch.is_whitespace() && !ch.is_control()) {
        Some(ch) => ch.to_string(),
        None => {
            let name = key.name().map(|n| n.to_string()).unwrap_or_default();
            let mut chars = name.chars();
            chars.next().map(|f| f.to_uppercase().chain(chars).collect()).unwrap_or_default()
        }
    };
    // People say "Super+Shift+S": Super first, Shift last.
    let order = [gdk::ModifierType::SUPER_MASK, gdk::ModifierType::CONTROL_MASK, gdk::ModifierType::ALT_MASK, gdk::ModifierType::SHIFT_MASK, gdk::ModifierType::META_MASK, gdk::ModifierType::HYPER_MASK];
    let mut parts: Vec<String> = order
        .iter()
        .filter(|m| mods.contains(**m))
        .filter_map(|m| MODIFIERS.iter().find(|(mm, _, _)| mm == m).map(|(_, _, l)| tr(l).to_owned()))
        .collect();
    parts.push(key_label);
    parts.join("+")
}

/// Reads what people type, "super+shift+s" or "Ctrl Alt T", as a combination.
pub fn parse_typed(text: &str) -> Option<Combo> {
    let parts: Vec<&str> = text.split(['+', ' ']).map(str::trim).filter(|p| !p.is_empty()).collect();
    let (key, mods) = parts.split_last()?;
    let mut accel = String::new();
    for m in mods {
        accel.push_str(match m.to_lowercase().as_str() {
            "shift" => "<Shift>",
            "ctrl" | "control" => "<Control>",
            "alt" => "<Alt>",
            "super" | "win" | "windows" | "meta" | "cmd" => "<Super>",
            _ => return None,
        });
    }
    // Key names are case-sensitive ("F4", "Print"); single letters are lowercase.
    let key = if key.chars().count() == 1 { key.to_lowercase() } else { (*key).to_owned() };
    let capitalized = {
        let mut c = key.chars();
        c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default()
    };
    combo(&format!("{accel}{key}")).or_else(|| combo(&format!("{accel}{capitalized}"))).or_else(|| combo(&format!("{accel}{}", key.to_lowercase())))
}

/// Whether a key alone is a modifier (pressed while typing a combination).
pub fn is_modifier(key: gdk::Key) -> bool {
    key.name().is_some_and(|n| {
        ["Shift", "Control", "Alt", "Super", "Meta", "Hyper", "ISO_Level3", "ISO_Level5", "Caps_Lock", "Num_Lock"].iter().any(|m| n.starts_with(m))
    })
}

/// A combination from a key press, or None for a bare letter or number
/// (those type text; a shortcut needs a modifier).
pub fn from_press(key: gdk::Key, state: gdk::ModifierType) -> Option<Combo> {
    let wanted = gdk::ModifierType::SHIFT_MASK | gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK | gdk::ModifierType::SUPER_MASK;
    let mods = state & wanted;
    let key = key.to_lower();
    let printable = key.to_unicode().is_some_and(|c| !c.is_control());
    if mods.is_empty() && printable || mods == gdk::ModifierType::SHIFT_MASK && printable {
        return None;
    }
    Some(Combo { keyval: key.into_glib(), mods: mods.bits() })
}

/// The GSettings form of a combination, "<Shift><Super>s".
pub fn accel_name(c: Combo) -> String {
    let mods = gdk::ModifierType::from_bits_truncate(c.mods);
    let mut out: String = MODIFIERS.iter().filter(|(m, _, _)| mods.contains(*m)).map(|(_, name, _)| *name).collect();
    out.push_str(&key_of(c).name().map(|n| n.to_string()).unwrap_or_default());
    out
}

// ---------------------------------------------------------------- Reading

/// Whether this desktop keeps its shortcuts where Voltkit can read them.
pub fn supported() -> bool {
    if std::env::var_os("VOLTKIT_SHORTCUTS_DEMO").is_some() {
        return true;
    }
    schema_source().is_some_and(|s| s.lookup("org.gnome.desktop.wm.keybindings", true).is_some())
}

fn schema_source() -> Option<gio::SettingsSchemaSource> {
    gio::SettingsSchemaSource::default()
}

pub fn all() -> Vec<Binding> {
    if std::env::var_os("VOLTKIT_SHORTCUTS_DEMO").is_some() {
        return demo();
    }
    let Some(source) = schema_source() else { return Vec::new() };
    let (schemas, _) = source.list_schemas(true);
    let extensions = enabled_extensions();
    let mut bindings = Vec::new();
    for id in schemas {
        let id = id.to_string();
        let owner = match system_owner(&id) {
            Some(o) => o.to_owned(),
            None => match extensions.iter().find(|e| e.schema.as_deref() == Some(id.as_str())) {
                Some(e) => trf!("Extension: {}", e.name),
                None => continue,
            },
        };
        let Some(schema) = source.lookup(&id, true) else { continue };
        read_schema(&schema, &gio::Settings::new(&id), &owner, None, &mut bindings);
    }
    // Extensions installed in the home folder bring their own schemas.
    for e in &extensions {
        let (Some(dir), Some(id)) = (&e.schemas_dir, &e.schema) else { continue };
        if bindings.iter().any(|b| &b.schema == id) {
            continue;
        }
        let Ok(source) = gio::SettingsSchemaSource::from_directory(dir, schema_source().as_ref(), false) else { continue };
        let Some(schema) = source.lookup(id, false) else { continue };
        let settings = gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None);
        read_schema(&schema, &settings, &trf!("Extension: {}", e.name), Some(dir.clone()), &mut bindings);
    }
    bindings.extend(custom_bindings());
    bindings.sort_by(|a, b| (&a.owner, a.name.to_lowercase()).cmp(&(&b.owner, b.name.to_lowercase())));
    bindings
}

/// Adds the shortcut keys of one schema.
fn read_schema(schema: &gio::SettingsSchema, settings: &gio::Settings, owner: &str, schema_dir: Option<std::path::PathBuf>, bindings: &mut Vec<Binding>) {
    let id = schema.id().to_string();
    for key in schema.list_keys() {
        let key = key.to_string();
        // Fixed hardware keys ("-static") can't be changed, and aren't combinations people press.
        if key.ends_with("-static") || key == "custom-keybindings" {
            continue;
        }
        let info = schema.key(&key);
        let summary = info.summary().map(|s| s.to_string()).unwrap_or_default();
        // GNOME keeps some old keys around that it ignores.
        if summary.to_lowercase().starts_with("deprecated") {
            continue;
        }
        let list = match info.value_type().as_str() {
            "as" => true,
            "s" if id == MEDIA_KEYS => false,
            _ => continue,
        };
        let read = |v: &gtk::glib::Variant| -> Vec<String> {
            if list { v.get::<Vec<String>>().unwrap_or_default() } else { v.get::<String>().into_iter().collect() }
        };
        let accels: Vec<String> = read(&settings.value(&key)).into_iter().filter(|a| !a.is_empty()).collect();
        let defaults: Vec<String> = read(&info.default_value()).into_iter().filter(|a| !a.is_empty()).collect();
        // Extensions keep other lists too: only keep ones that look like shortcuts.
        let looks_like = |v: &[String]| !v.is_empty() && v.iter().all(|a| combo(a).is_some());
        if !id.contains("keybindings") && id != MEDIA_KEYS && !(looks_like(&accels) || looks_like(&defaults)) {
            continue;
        }
        bindings.push(Binding {
            name: if summary.is_empty() { humanize(&key) } else { tidy_summary(&summary) },
            writable: settings.is_writable(&key),
            schema: id.clone(),
            path: None,
            schema_dir: schema_dir.clone(),
            key,
            owner: owner.to_owned(),
            accels,
            defaults,
            list,
        });
    }
}

/// An enabled GNOME Shell extension, from its metadata.json.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Extension {
    name: String,
    schema: Option<String>,
    /// For extensions with their own compiled schemas.
    schemas_dir: Option<std::path::PathBuf>,
}

fn extension_from_metadata(json: &str, dir: &std::path::Path) -> Option<Extension> {
    let meta: serde_json::Value = serde_json::from_str(json).ok()?;
    let schemas = dir.join("schemas");
    Some(Extension {
        name: meta["name"].as_str()?.to_owned(),
        schema: meta["settings-schema"].as_str().map(str::to_owned),
        schemas_dir: schemas.join("gschemas.compiled").exists().then_some(schemas),
    })
}

fn enabled_extensions() -> Vec<Extension> {
    enabled_extension_ids()
        .iter()
        .filter_map(|uuid| {
            let dirs = [dirs::data_dir().map(|d| d.join("gnome-shell/extensions")), Some("/usr/share/gnome-shell/extensions".into())];
            dirs.into_iter().flatten().map(|d| d.join(uuid)).find_map(|dir| {
                let json = std::fs::read_to_string(dir.join("metadata.json")).ok()?;
                extension_from_metadata(&json, &dir)
            })
        })
        .collect()
}

fn enabled_extension_ids() -> Vec<String> {
    let Some(source) = schema_source() else { return Vec::new() };
    if source.lookup("org.gnome.shell", true).is_none() {
        return Vec::new();
    }
    let shell = gio::Settings::new("org.gnome.shell");
    if shell.boolean("disable-user-extensions") {
        return Vec::new();
    }
    shell.strv("enabled-extensions").iter().map(|s| s.to_string()).collect()
}

/// Who the desktop's own shortcut schemas belong to.
fn system_owner(id: &str) -> Option<&'static str> {
    Some(match id {
        "org.gnome.desktop.wm.keybindings" | "org.gnome.mutter.keybindings" | "org.cinnamon.desktop.keybindings.wm" | "org.mate.Marco.global-keybindings" => tr("Windows"),
        "org.gnome.mutter.wayland.keybindings" => tr("Sessions"),
        "org.gnome.shell.keybindings" => "GNOME Shell",
        MEDIA_KEYS => tr("System"),
        _ => return None,
    })
}

fn custom_bindings() -> Vec<Binding> {
    let Some(source) = schema_source() else { return Vec::new() };
    if source.lookup(MEDIA_KEYS, true).is_none() || source.lookup(CUSTOM, true).is_none() {
        return Vec::new();
    }
    let media = gio::Settings::new(MEDIA_KEYS);
    media
        .strv("custom-keybindings")
        .iter()
        .map(|path| {
            let path = path.to_string();
            let s = gio::Settings::with_path(CUSTOM, &path);
            let name = s.string("name").to_string();
            let accel = s.string("binding").to_string();
            let voltkit = path.contains("/voltkit-");
            Binding {
                schema: CUSTOM.into(),
                key: "binding".into(),
                name: if name.is_empty() { s.string("command").to_string() } else { name.trim_start_matches("Voltkit: ").to_owned() },
                owner: if voltkit { "Voltkit".into() } else { tr("Custom shortcuts").into() },
                accels: if accel.is_empty() { Vec::new() } else { vec![accel] },
                defaults: Vec::new(),
                list: false,
                writable: s.is_writable("binding"),
                path: Some(path),
                schema_dir: None,
            }
        })
        .collect()
}

/// "switch-to-workspace-up" → "Switch to workspace up".
fn humanize(key: &str) -> String {
    let words = key.replace(['-', '_'], " ");
    let mut c = words.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// GSettings summaries: "Keybinding to open the overview." → "Open the overview".
fn tidy_summary(s: &str) -> String {
    let s = s.trim().trim_end_matches('.');
    let lower = s.to_lowercase();
    let s = ["keybinding to ", "keybinding for ", "shortcut to ", "shortcut for ", "hotkey to ", "hotkey for ", "key to ", "key for "]
        .iter()
        .find(|p| lower.starts_with(*p))
        .map_or(s, |p| &s[p.len()..]);
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

// ---------------------------------------------------------------- Finding

/// Every binding that uses `accel`.
pub fn who<'a>(accel: &str, bindings: &'a [Binding]) -> Vec<&'a Binding> {
    let Some(c) = combo(accel) else { return Vec::new() };
    bindings.iter().filter(|b| b.accels.iter().any(|a| combo(a) == Some(c))).collect()
}

/// Combinations used by more than one action, as (combination, users).
pub fn conflicts(bindings: &[Binding]) -> Vec<(Combo, Vec<&Binding>)> {
    let mut by_combo: BTreeMap<Combo, Vec<&Binding>> = BTreeMap::new();
    for b in bindings {
        let mut seen = Vec::new();
        for a in &b.accels {
            if let Some(c) = combo(a)
                && !seen.contains(&c)
            {
                seen.push(c);
                by_combo.entry(c).or_default().push(b);
            }
        }
    }
    by_combo.into_iter().filter(|(_, users)| users.len() > 1).collect()
}

/// Free combinations near `accel`: the same key with other modifiers first,
/// then nearby keys. Leaves out ones apps use for themselves (Ctrl+S and the
/// like), so a suggestion never steals an app's shortcut.
pub fn suggest(accel: &str, bindings: &[Binding], count: usize) -> Vec<String> {
    let used: Vec<Combo> = bindings.iter().flat_map(|b| b.accels.iter().filter_map(|a| combo(a))).collect();
    let key = combo(accel).map(|c| c.keyval);
    let s = gdk::ModifierType::SUPER_MASK;
    let sh = gdk::ModifierType::SHIFT_MASK;
    let ctrl = gdk::ModifierType::CONTROL_MASK;
    let alt = gdk::ModifierType::ALT_MASK;
    let mod_sets = [s | sh, s | alt, s | ctrl, ctrl | alt, s | ctrl | sh, s | alt | sh, ctrl | alt | sh];
    let letters: Vec<u32> = "abcdefghijklmnopqrstuvwxyz0123456789".chars().filter_map(|c| gdk::Key::from_name(c.to_string())).map(|k| k.into_glib()).collect();
    let mut keys: Vec<u32> = key.into_iter().collect();
    // Then keys near it in the alphabet (or the start of it).
    let start = key.and_then(|k| letters.iter().position(|l| *l == k)).unwrap_or(0);
    for d in 1..letters.len() {
        for i in [start + d, start.wrapping_sub(d)] {
            if let Some(l) = letters.get(i)
                && !keys.contains(l)
            {
                keys.push(*l);
            }
        }
    }
    let mut out = Vec::new();
    for k in keys {
        for m in mod_sets {
            let c = Combo { keyval: k, mods: m.bits() };
            if Some(c) != combo(accel) && !used.contains(&c) && !out.contains(&accel_name(c)) {
                out.push(accel_name(c));
                if out.len() == count {
                    return out;
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------- Changing

fn settings_for(b: &Binding) -> Result<gio::Settings> {
    if let Some(dir) = &b.schema_dir {
        let source = gio::SettingsSchemaSource::from_directory(dir, schema_source().as_ref(), false)?;
        let schema = source.lookup(&b.schema, false).context(tr("the extension's settings are gone"))?;
        return Ok(gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None));
    }
    Ok(match &b.path {
        Some(path) => gio::Settings::with_path(&b.schema, path),
        None => gio::Settings::new(&b.schema),
    })
}

/// Replaces `old` with `new` in the binding (or removes `old` with `None`),
/// keeping its other shortcuts.
pub fn change(b: &Binding, old: Option<&str>, new: Option<&str>) -> Result<()> {
    if !b.writable {
        bail!("{}", tr("this shortcut is locked by the system administrator"));
    }
    if let Some(new) = new
        && combo(new).is_none()
    {
        bail!("{}", trf!("{} isn't a valid shortcut", new));
    }
    let mut accels: Vec<String> = b.accels.clone();
    match old.and_then(|o| accels.iter().position(|a| combo(a) == combo(o))) {
        Some(i) => match new {
            Some(n) => accels[i] = n.to_owned(),
            None => {
                accels.remove(i);
            }
        },
        None => {
            if let Some(n) = new {
                if b.list {
                    accels.insert(0, n.to_owned());
                } else {
                    accels = vec![n.to_owned()];
                }
            } else if old.is_none() {
                accels.clear();
            }
        }
    }
    let settings = settings_for(b)?;
    let result = if b.list {
        let refs: Vec<&str> = accels.iter().map(String::as_str).collect();
        settings.set_strv(&b.key, refs.as_slice())
    } else {
        settings.set_string(&b.key, accels.first().map(String::as_str).unwrap_or(""))
    };
    result.context(tr("could not save the shortcut"))?;
    gio::Settings::sync();
    if let Some(action) = b.voltkit_action() {
        save_voltkit_hotkey(action, accels.first().map(String::as_str).unwrap_or(""))?;
    }
    Ok(())
}

/// Puts the shortcut back to how the desktop came.
pub fn reset(b: &Binding) -> Result<()> {
    if !b.writable {
        bail!("{}", tr("this shortcut is locked by the system administrator"));
    }
    if b.path.is_some() {
        bail!("{}", tr("custom shortcuts have no default"));
    }
    settings_for(b)?.reset(&b.key);
    gio::Settings::sync();
    Ok(())
}

/// Voltkit's own hotkeys also live in its config, which `voltkit shortcuts
/// install` and the background service bind: keep it in step.
fn save_voltkit_hotkey(action: &str, accel: &str) -> Result<()> {
    let mut config = crate::config::Config::load()?;
    let h = &mut config.hotkeys;
    let slot = match action {
        "ocr" => &mut h.ocr,
        "color" => &mut h.color_picker,
        "snippets" => &mut h.snippets,
        "qr" => &mut h.qr,
        "ruler" => &mut h.ruler,
        "smart-paste" => &mut h.smart_paste,
        "zoom" => &mut h.zoom,
        "shortcut-guide" => &mut h.shortcut_guide,
        _ => return Ok(()),
    };
    *slot = accel.to_owned();
    config.save()
}

/// A typical desktop with one clash, for the README screenshots.
fn demo() -> Vec<Binding> {
    let b = |owner: &str, name: &str, accels: &[&str]| Binding {
        schema: "demo".into(),
        path: None,
        schema_dir: None,
        key: name.into(),
        name: name.into(),
        owner: owner.into(),
        accels: accels.iter().map(|s| s.to_string()).collect(),
        defaults: accels.iter().map(|s| s.to_string()).collect(),
        list: true,
        writable: true,
    };
    vec![
        b("Extension: Clipboard History", "Show the clipboard history", &["<Super>v"]),
        b("GNOME Shell", "Show the notification list", &["<Super>v", "<Super>m"]),
        b("GNOME Shell", "Show all apps", &["<Super>a"]),
        b("GNOME Shell", "Take a screenshot interactively", &["Print"]),
        b("System", "Lock the screen", &["<Super>l"]),
        b("System", "Open the terminal", &["<Primary><Alt>t"]),
        b("Voltkit", "Extract text from screen", &["<Shift><Super>t"]),
        b("Voltkit", "Paste the clipboard converted", &["<Shift><Super>v"]),
        b("Windows", "Close window", &["<Alt>F4"]),
        b("Windows", "Switch applications", &["<Super>Tab", "<Alt>Tab"]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(name: &str, owner: &str, accels: &[&str]) -> Binding {
        Binding {
            schema: "test".into(),
            path: None,
            schema_dir: None,
            key: name.into(),
            name: name.into(),
            owner: owner.into(),
            accels: accels.iter().map(|s| s.to_string()).collect(),
            defaults: Vec::new(),
            list: true,
            writable: true,
        }
    }

    #[test]
    fn compares_combinations() {
        assert_eq!(combo("<Super>s"), combo("<Super>S"));
        assert_eq!(combo("<Mod4>s"), combo("<Super>s"));
        assert_eq!(combo("<Shift><Super>t"), combo("<Super><Shift>T"));
        assert_eq!(combo("<Primary><Alt>F1"), combo("<Control><Alt>F1"));
        assert_ne!(combo("<Super>s"), combo("<Super><Shift>s"));
        assert_eq!(combo("not a shortcut <<"), None);
    }

    #[test]
    fn finds_owners_and_conflicts() {
        let list = vec![
            binding("Show notifications", "GNOME Shell", &["<Super>v", "<Super>m"]),
            binding("Smart Paste", "Voltkit", &["<Super><Shift>v"]),
            binding("Clipboard", "Custom shortcuts", &["<Super>V"]),
            binding("Close", "Windows", &["<Alt>F4"]),
        ];
        let owners: Vec<&str> = who("<Super>v", &list).iter().map(|b| b.name.as_str()).collect();
        assert_eq!(owners, vec!["Show notifications", "Clipboard"]);
        let c = conflicts(&list);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].1.len(), 2);
        assert!(who("<Super>F12", &list).is_empty());
    }

    #[test]
    fn suggests_free_combinations() {
        let list = vec![binding("A", "Windows", &["<Super><Shift>v", "<Super><Alt>v"])];
        let s = suggest("<Super><Shift>v", &list, 4);
        assert_eq!(s.len(), 4);
        let used: Vec<Combo> = list[0].accels.iter().filter_map(|a| combo(a)).collect();
        for a in &s {
            let c = combo(a).unwrap();
            assert!(!used.contains(&c), "{a} is taken");
            // Never a plain Ctrl+key or Alt+key: apps use those.
            let m = gdk::ModifierType::from_bits_truncate(c.mods);
            assert!(m.contains(gdk::ModifierType::SUPER_MASK) || m.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK), "{a}");
        }
        // The same key comes first.
        assert_eq!(combo(&s[0]).unwrap().keyval, combo("v").unwrap().keyval);
    }

    #[test]
    fn labels_and_names() {
        assert_eq!(humanize("switch-to-workspace-up"), "Switch to workspace up");
        assert_eq!(accel_name(combo("<Super><Shift>T").unwrap()), "<Shift><Super>t");
        assert_eq!(label("<Shift><Super>t"), "Super+Shift+T");
        assert_eq!(label("<Alt>F4"), "Alt+F4");
        assert_eq!(label("<Alt>space"), "Alt+Space");
        assert_eq!(label("<Primary><Alt>Delete"), "Ctrl+Alt+Delete");
        assert_eq!(parse_typed("super+shift+s"), combo("<Shift><Super>s"));
        assert_eq!(parse_typed("Ctrl Alt T"), combo("<Control><Alt>t"));
        assert_eq!(parse_typed("alt+f4"), combo("<Alt>F4"));
        assert_eq!(parse_typed("print"), combo("Print"));
        assert_eq!(parse_typed("hello world"), None);
        let mut b = binding("x", "Voltkit", &[]);
        b.path = Some("/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/voltkit-smart-paste/".into());
        assert_eq!(b.voltkit_action(), Some("smart-paste"));
    }

    #[test]
    fn reads_extensions_and_owners() {
        let e = extension_from_metadata(r#"{"name":"Dash to Dock","settings-schema":"org.gnome.shell.extensions.dash-to-dock"}"#, std::path::Path::new("/nowhere")).unwrap();
        assert_eq!((e.name.as_str(), e.schema.as_deref(), e.schemas_dir), ("Dash to Dock", Some("org.gnome.shell.extensions.dash-to-dock"), None));
        assert_eq!(system_owner("org.gnome.desktop.wm.keybindings"), Some("Windows"));
        assert_eq!(system_owner("org.gnome.TextEditor"), None);
        assert_eq!(tidy_summary("Keybinding to open the overview."), "Open the overview");
        assert_eq!(tidy_summary("Switch to workspace 1"), "Switch to workspace 1");
    }

}
