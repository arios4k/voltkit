//! Why Didn't It Sleep?: what keeps the computer awake right now, what
//! happened during recent sleeps (and what woke it), and which devices may
//! wake it.
//!
//! Sources: logind and GNOME session inhibitors, the power settings, the
//! system journal (systemd-sleep and logind messages), UPower's battery
//! history and USB devices in sysfs. Nothing needs root except changing which
//! devices may wake the computer.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use gtk::gio;
use gtk::prelude::*;

use super::usage;
use crate::i18n::tr;

/// The reason Voltkit's own Keep Awake gives (see awake.rs).
pub const KEEP_AWAKE_REASON: &str = "Keep Awake is on in Voltkit";
/// How far back the sleep history goes.
const HISTORY_DAYS: u32 = 7;
const MAX_SLEEPS: usize = 12;
/// Battery use while asleep above this, in percent per hour, is flagged.
const HIGH_DRAIN: f64 = 1.5;
/// A lid closed for longer than this without sleeping is reported.
const LID_AWAKE_MIN: f64 = 120.0;
const UDEV_RULES: &str = "/etc/udev/rules.d/90-voltkit-wakeup.rules";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// The computer can't sleep at all.
    Sleep,
    /// The screen stays on and automatic sleep doesn't happen.
    Idle,
    /// Closing the lid doesn't put it to sleep.
    Lid,
}

impl Effect {
    pub fn describe(self) -> &'static str {
        match self {
            Effect::Sleep => tr("Stops the computer from sleeping"),
            Effect::Idle => tr("Keeps the screen on and stops automatic sleep"),
            Effect::Lid => tr("Stops closing the lid from putting it to sleep"),
        }
    }
}

/// Something holding the computer awake right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocker {
    /// What to call it when it isn't an installed app.
    pub name: String,
    pub app_id: Option<String>,
    pub reason: String,
    pub effect: Effect,
    /// Voltkit's own Keep Awake, which the page can turn off.
    pub keep_awake: bool,
}

/// A setting that stops the computer from sleeping by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub title: String,
    pub detail: String,
    /// Changed in the desktop's power settings (rather than a system file).
    pub in_power_settings: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A sleep, with why it started and what ended it.
    Slept {
        start: f64,
        end: f64,
        cause: String,
        woke: String,
        /// Woken by something other than the lid or power button.
        unexpected: bool,
        /// Battery percentage before and after.
        battery: Option<(f64, f64)>,
    },
    /// The lid was closed but the computer stayed awake.
    LidAwake { start: f64, end: Option<f64> },
}

impl Event {
    pub fn start(&self) -> f64 {
        match self {
            Event::Slept { start, .. } | Event::LidAwake { start, .. } => *start,
        }
    }
}

/// A USB device that may be allowed to wake the computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeDevice {
    pub sysfs: PathBuf,
    pub name: String,
    pub vendor: String,
    pub product: String,
    pub enabled: bool,
    /// Times it woke the computer (or stopped it from sleeping).
    pub wakeups: u64,
}

#[derive(Debug, Default)]
pub struct Report {
    pub blockers: Vec<Blocker>,
    pub notes: Vec<Note>,
    /// Newest first.
    pub history: Vec<Event>,
    /// Why the history is missing, when it is.
    pub history_problem: Option<String>,
    pub devices: Vec<WakeDevice>,
    /// The kind of sleep this computer uses, explained, when it matters.
    pub sleep_mode: Option<Note>,
}

pub async fn report() -> Report {
    if std::env::var_os("VOLTKIT_SLEEP_DEMO").is_some() {
        return demo();
    }
    let on_battery = on_battery().await;
    let mut report = Report { blockers: blockers().await, ..Default::default() };
    report.notes = power_notes(on_battery);
    report.notes.extend(lid_notes(on_battery).await);
    match journal_events() {
        Ok(lines) => {
            let battery = battery_history().await;
            report.history = parse_history(&lines, &battery);
        }
        Err(e) => report.history_problem = Some(e.to_string()),
    }
    report.devices = wake_devices();
    report.sleep_mode = sleep_mode(&std::fs::read_to_string("/sys/power/mem_sleep").unwrap_or_default());
    report
}

// ---------------------------------------------------------------- Right now

/// logind's (what, who, why, mode, uid, pid).
type Inhibitor = (String, String, String, String, u32, u32);

async fn blockers() -> Vec<Blocker> {
    let gnome = gnome_inhibitors().await;
    let mut list = Vec::new();
    if let Ok(conn) = zbus::Connection::system().await {
        let inhibitors: Option<(Vec<Inhibitor>,)> = conn
            .call_method(
                Some("org.freedesktop.login1"),
                "/org/freedesktop/login1",
                Some("org.freedesktop.login1.Manager"),
                "ListInhibitors",
                &(),
            )
            .await
            .ok()
            .and_then(|r| r.body().deserialize().ok());
        for (what, who, why, mode, _uid, pid) in inhibitors.map(|(l,)| l).unwrap_or_default() {
            if let Some(b) = logind_blocker(&what, &who, &why, &mode, pid, gnome.is_some()) {
                list.push(b);
            }
        }
    }
    list.extend(gnome.unwrap_or_default());
    list.dedup();
    list
}

fn logind_blocker(what: &str, who: &str, why: &str, mode: &str, pid: u32, have_gnome: bool) -> Option<Blocker> {
    if mode != "block" {
        return None;
    }
    let effect = if what.split(':').any(|w| w == "sleep") {
        Effect::Sleep
    } else if what.split(':').any(|w| w == "handle-lid-switch") {
        Effect::Lid
    } else if what.split(':').any(|w| w == "idle") {
        Effect::Idle
    } else {
        // Power key handling, shutdown delays and the like: normal.
        return None;
    };
    // GNOME passes its apps' requests on as one "user session" inhibitor;
    // the apps themselves come from GNOME's own list.
    if have_gnome && why == "user session inhibited" {
        return None;
    }
    let (name, app_id) = match usage::process(pid) {
        // Inhibitors taken for the user are named after the user; the process says more.
        Some((command, app)) if who == usage::user_name(usage::current_uid()) || who.is_empty() => (command, app),
        Some((_, app)) => (who.to_owned(), app),
        None => (who.to_owned(), None),
    };
    let (name, reason) = if effect == Effect::Lid && why.contains("External monitor") {
        ("GNOME".to_owned(), tr("An external monitor is connected (or was just changed)").to_owned())
    } else {
        (name, why.to_owned())
    };
    Some(Blocker { name, app_id, keep_awake: why == KEEP_AWAKE_REASON, reason, effect })
}

/// GNOME session inhibitors; `None` when not running GNOME.
async fn gnome_inhibitors() -> Option<Vec<Blocker>> {
    const SUSPEND: u32 = 4;
    const IDLE: u32 = 8;
    let conn = zbus::Connection::session().await.ok()?;
    let reply = conn
        .call_method(Some("org.gnome.SessionManager"), "/org/gnome/SessionManager", Some("org.gnome.SessionManager"), "GetInhibitors", &())
        .await
        .ok()?;
    let (paths,): (Vec<zbus::zvariant::OwnedObjectPath>,) = reply.body().deserialize().ok()?;
    let mut list = Vec::new();
    for path in paths {
        let get = |method: &'static str| {
            let (conn, path) = (conn.clone(), path.clone());
            async move {
                conn.call_method(Some("org.gnome.SessionManager"), path.as_str(), Some("org.gnome.SessionManager.Inhibitor"), method, &())
                    .await
                    .ok()
            }
        };
        let Some(app) = get("GetAppId").await.and_then(|r| r.body().deserialize::<(String,)>().ok()).map(|(a,)| a) else { continue };
        let reason = get("GetReason").await.and_then(|r| r.body().deserialize::<(String,)>().ok()).map(|(r,)| r).unwrap_or_default();
        let flags = get("GetFlags").await.and_then(|r| r.body().deserialize::<(u32,)>().ok()).map(|(f,)| f).unwrap_or(0);
        let effect = if flags & SUSPEND != 0 {
            Effect::Sleep
        } else if flags & IDLE != 0 {
            Effect::Idle
        } else {
            continue;
        };
        list.push(gnome_blocker(&app, &reason, effect));
    }
    Some(list)
}

fn gnome_blocker(app: &str, reason: &str, effect: Effect) -> Blocker {
    let keep_awake = reason == KEEP_AWAKE_REASON;
    // GNOME's app ID is a desktop file name, a program path, or "mutter" for
    // windows that asked to keep the screen on (usually a playing video).
    let (name, app_id, reason) = if app == "mutter" {
        (tr("A window").to_owned(), None, tr("Asked to keep the screen on, usually for a playing video").to_owned())
    } else if keep_awake {
        ("Voltkit".to_owned(), Some("io.github.arios4k.Voltkit".to_owned()), tr("Keep Awake is on").to_owned())
    } else {
        let id = app.strip_suffix(".desktop").unwrap_or(app);
        let name = Path::new(id).file_name().map_or_else(|| id.to_owned(), |n| n.to_string_lossy().into_owned());
        // Electron and Chromium apps give their toolkit's name as the reason.
        let reason = if reason.is_empty() || reason == "Electron" || reason == "Chromium" {
            tr("Asked the computer to stay awake").to_owned()
        } else {
            reason.to_owned()
        };
        (name, (!id.contains('/')).then(|| id.to_owned()), reason)
    };
    Blocker { name, app_id, reason, effect, keep_awake }
}

async fn on_battery() -> bool {
    let Ok(conn) = zbus::Connection::system().await else { return false };
    let Ok(proxy) = zbus::Proxy::new(&conn, "org.freedesktop.UPower", "/org/freedesktop/UPower", "org.freedesktop.UPower").await else {
        return false;
    };
    proxy.get_property::<bool>("OnBattery").await.unwrap_or(false)
}

/// Automatic sleep turned off in the desktop's power settings.
fn power_notes(on_battery: bool) -> Vec<Note> {
    const SCHEMA: &str = "org.gnome.settings-daemon.plugins.power";
    let Some(source) = gio::SettingsSchemaSource::default() else { return Vec::new() };
    if source.lookup(SCHEMA, true).is_none() {
        return Vec::new();
    }
    let settings = gio::Settings::new(SCHEMA);
    let mut notes = Vec::new();
    for (key, plugged_in, now) in [("sleep-inactive-ac-type", true, !on_battery), ("sleep-inactive-battery-type", false, on_battery)] {
        if settings.string(key) == "nothing" {
            let (title, detail) = match (plugged_in, now) {
                (true, true) => (
                    tr("Never sleeps by itself when plugged in"),
                    tr("Automatic sleep is turned off for when the computer is plugged in, which it is now. Turn it on under Power Saving in the power settings."),
                ),
                (true, false) => (
                    tr("Never sleeps by itself when plugged in"),
                    tr("Automatic sleep is turned off for when the computer is plugged in. Turn it on under Power Saving in the power settings."),
                ),
                (false, true) => (
                    tr("Never sleeps by itself when on battery"),
                    tr("Automatic sleep is turned off for when the computer is on battery, which it is now. Turn it on under Power Saving in the power settings."),
                ),
                (false, false) => (
                    tr("Never sleeps by itself when on battery"),
                    tr("Automatic sleep is turned off for when the computer is on battery. Turn it on under Power Saving in the power settings."),
                ),
            };
            notes.push(Note { title: title.to_owned(), detail: detail.to_owned(), in_power_settings: true });
        }
    }
    notes
}

/// What closing the lid does, from logind's settings.
async fn lid_notes(on_battery: bool) -> Vec<Note> {
    let Ok(conn) = zbus::Connection::system().await else { return Vec::new() };
    let Ok(proxy) = zbus::Proxy::new(&conn, "org.freedesktop.login1", "/org/freedesktop/login1", "org.freedesktop.login1.Manager").await else {
        return Vec::new();
    };
    let get = |name: &'static str| {
        let proxy = proxy.clone();
        async move { proxy.get_property::<String>(name).await.unwrap_or_default() }
    };
    let (lid, external, docked_action) = (get("HandleLidSwitch").await, get("HandleLidSwitchExternalPower").await, get("HandleLidSwitchDocked").await);
    let docked = proxy.get_property::<bool>("Docked").await.unwrap_or(false);
    lid_note(&lid, &external, &docked_action, docked, on_battery).into_iter().collect()
}

fn lid_note(lid: &str, external: &str, docked_action: &str, docked: bool, on_battery: bool) -> Option<Note> {
    let sleeps = |action: &str| matches!(action, "suspend" | "hibernate" | "hybrid-sleep" | "suspend-then-hibernate");
    let file = tr("This is set in /etc/systemd/logind.conf.");
    if !sleeps(lid) {
        return Some(Note {
            title: tr("Closing the lid never puts it to sleep").into(),
            detail: trf!("The lid is set to “{}”. {}", if lid.is_empty() { "ignore" } else { lid }, file),
            in_power_settings: false,
        });
    }
    if docked && !sleeps(docked_action) {
        return Some(Note {
            title: tr("Closing the lid doesn't sleep while a monitor or dock is connected").into(),
            detail: trf!("An external monitor or dock is connected now, so closing the lid keeps the computer running (so you can keep using the big screen). Disconnect it to sleep. {}", file),
            in_power_settings: false,
        });
    }
    if !on_battery && !external.is_empty() && !sleeps(external) {
        return Some(Note {
            title: tr("Closing the lid doesn't sleep while plugged in").into(),
            detail: trf!("The lid is set to “{}” on power. {}", external, file),
            in_power_settings: false,
        });
    }
    None
}

fn sleep_mode(mem_sleep: &str) -> Option<Note> {
    let current = mem_sleep.split_whitespace().find(|m| m.starts_with('['))?.trim_matches(['[', ']']);
    let deep_available = mem_sleep.split_whitespace().any(|m| m.trim_matches(['[', ']']) == "deep");
    (current == "s2idle").then(|| Note {
        title: tr("Uses light sleep (s2idle)").into(),
        detail: if deep_available {
            tr("Light sleep wakes up instantly but uses more battery than deep sleep, which this computer also \
             supports. Deep sleep can be chosen with the kernel option mem_sleep_default=deep.")
                .into()
        } else {
            tr("Light sleep wakes up instantly but uses more battery than deep sleep. Some apps and devices can \
             also wake it more easily.")
                .into()
        },
        in_power_settings: false,
    })
}

// ---------------------------------------------------------------- History

/// (Unix time, process, message) from systemd-sleep and logind, oldest first.
fn journal_events() -> Result<Vec<(f64, String, String)>> {
    let output = Command::new("journalctl")
        .args(["-q", "-o", "short-unix", "--no-pager", "--since", &format!("-{HISTORY_DAYS} days")])
        .args(["SYSLOG_IDENTIFIER=systemd-sleep", "SYSLOG_IDENTIFIER=systemd-logind"])
        .output()
        .context(tr("journalctl isn't available"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<(f64, String, String)> = text.lines().filter_map(parse_journal_line).collect();
    if lines.is_empty() && String::from_utf8_lossy(&output.stderr).contains("insufficient permissions") {
        bail!("{}", tr("Your account can't read the system log. Being in the \u{201c}adm\u{201d} group allows it."));
    }
    Ok(lines)
}

/// `1793389737.346531 host systemd-sleep[73912]: message`
fn parse_journal_line(line: &str) -> Option<(f64, String, String)> {
    let mut parts = line.splitn(3, ' ');
    let time: f64 = parts.next()?.parse().ok()?;
    let rest = parts.nth(1)?;
    let (ident, message) = rest.split_once(": ")?;
    let ident = ident.split('[').next()?.to_owned();
    Some((time, ident, message.to_owned()))
}

/// Battery percentage over time, (Unix time, percent), oldest first.
async fn battery_history() -> Vec<(f64, f64)> {
    let Ok(conn) = zbus::Connection::system().await else { return Vec::new() };
    let Ok(reply) = conn
        .call_method(Some("org.freedesktop.UPower"), "/org/freedesktop/UPower", Some("org.freedesktop.UPower"), "EnumerateDevices", &())
        .await
    else {
        return Vec::new();
    };
    let Ok((devices,)) = reply.body().deserialize::<(Vec<zbus::zvariant::OwnedObjectPath>,)>() else { return Vec::new() };
    let Some(battery) = devices.into_iter().find(|p| p.as_str().contains("/battery_")) else { return Vec::new() };
    let span = HISTORY_DAYS * 24 * 3600 + 3600;
    let Ok(reply) = conn
        .call_method(Some("org.freedesktop.UPower"), battery.as_str(), Some("org.freedesktop.UPower.Device"), "GetHistory", &("charge", span, 1000u32))
        .await
    else {
        return Vec::new();
    };
    let Ok((points,)) = reply.body().deserialize::<(Vec<(u32, f64, u32)>,)>() else { return Vec::new() };
    let mut points: Vec<(f64, f64)> = points.into_iter().filter(|(_, v, state)| *v > 0.0 && *state != 0).map(|(t, v, _)| (t as f64, v)).collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    points
}

/// Turns journal messages into sleeps and lid-closed-but-awake periods,
/// newest first.
pub fn parse_history(lines: &[(f64, String, String)], battery: &[(f64, f64)]) -> Vec<Event> {
    let is = |line: &(f64, String, String), ident: &str, prefix: &str| line.1 == ident && line.2.starts_with(prefix);
    let lid_closed = |l: &(f64, String, String)| is(l, "systemd-logind", "Lid closed");
    let lid_opened = |l: &(f64, String, String)| is(l, "systemd-logind", "Lid opened");
    let power_key = |l: &(f64, String, String)| is(l, "systemd-logind", "Power key pressed");
    let sleep_start = |l: &(f64, String, String)| is(l, "systemd-sleep", "Performing sleep operation") || is(l, "systemd-sleep", "Entering sleep state");
    let sleep_end = |l: &(f64, String, String)| is(l, "systemd-sleep", "System returned from sleep") || is(l, "systemd-sleep", "System resumed");
    let boot = |l: &(f64, String, String)| is(l, "systemd-logind", "New seat") || is(l, "systemd-logind", "Watching system buttons");

    let mut events = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let t = line.0;
        if sleep_start(line) {
            let before = lines[..i].iter().rev().take_while(|l| t - l.0 <= 15.0);
            let cause = before
                .clone()
                .find_map(|l| {
                    if lid_closed(l) {
                        Some(N_!("you closed the lid"))
                    } else if power_key(l) {
                        Some(N_!("you pressed the power button"))
                    } else {
                        None
                    }
                })
                .unwrap_or(N_!("it was idle, or you chose Suspend"));
            let Some((j, end)) = lines[i + 1..].iter().enumerate().find(|(_, l)| sleep_end(l) || sleep_start(l) || boot(l)) else { continue };
            if !sleep_end(end) {
                continue;
            }
            // Linux doesn't log what woke it, except the lid and the power button.
            let after: Vec<_> = lines[i + 1 + j + 1..].iter().take_while(|l| l.0 - end.0 <= 60.0).collect();
            let opened = after.iter().any(|l| lid_opened(l));
            let pressed = after.iter().any(|l| power_key(l) && l.0 - end.0 <= 20.0);
            let lid_cause = cause == "you closed the lid";
            let (woke, unexpected) = if opened {
                (N_!("you opening the lid"), false)
            } else if pressed {
                (N_!("the power button"), false)
            } else if end.0 - t < 60.0 {
                (N_!("something right away: often a USB device, the network or a driver"), true)
            } else if lid_cause {
                (N_!("something while the lid was still closed: often a USB device or the network"), true)
            } else {
                (N_!("a key, the touchpad, a USB device or a timer"), false)
            };
            events.push(Event::Slept {
                start: t,
                end: end.0,
                cause: cause.to_owned(),
                woke: woke.to_owned(),
                unexpected,
                battery: battery_change(battery, t, end.0),
            });
        } else if lid_closed(line) {
            // Did it sleep shortly after?
            let slept = lines[i + 1..].iter().take_while(|l| l.0 - t <= 60.0).any(sleep_start);
            if slept {
                continue;
            }
            let end = lines[i + 1..].iter().find(|l| lid_opened(l) || boot(l) || sleep_start(l)).filter(|l| lid_opened(l)).map(|l| l.0);
            if end.is_none_or(|e| e - t >= LID_AWAKE_MIN) {
                events.push(Event::LidAwake { start: t, end });
            }
        }
    }
    events.sort_by(|a, b| b.start().total_cmp(&a.start()));
    events.truncate(MAX_SLEEPS);
    events
}

/// Battery percentage just before `start` and just after `end`.
fn battery_change(points: &[(f64, f64)], start: f64, end: f64) -> Option<(f64, f64)> {
    let before = points.iter().rev().find(|(t, _)| *t <= start && start - t <= 3600.0)?.1;
    let after = points.iter().find(|(t, _)| *t >= end && t - end <= 1800.0)?.1;
    Some((before, after))
}

/// Battery use in percent per hour, when the sleep was long enough to tell.
pub fn drain_per_hour(start: f64, end: f64, battery: Option<(f64, f64)>) -> Option<f64> {
    let (before, after) = battery?;
    let hours = (end - start) / 3600.0;
    (hours >= 0.5 && after <= before).then(|| (before - after) / hours)
}

pub fn is_high_drain(rate: f64) -> bool {
    rate > HIGH_DRAIN
}

// ---------------------------------------------------------------- Wake devices

pub fn wake_devices() -> Vec<WakeDevice> {
    let Ok(dir) = std::fs::read_dir("/sys/bus/usb/devices") else { return Vec::new() };
    let read = |p: &Path, f: &str| std::fs::read_to_string(p.join(f)).map(|s| s.trim().to_owned()).unwrap_or_default();
    let mut devices: Vec<WakeDevice> = dir
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Interfaces (1-4:1.0) and the root hubs (usb1) aren't devices you plug in.
            if name.contains(':') || name.starts_with("usb") {
                return None;
            }
            let path = entry.path();
            let wakeup = read(&path, "power/wakeup");
            if wakeup.is_empty() || read(&path, "bDeviceClass") == "09" {
                return None;
            }
            let (vendor, product) = (read(&path, "idVendor"), read(&path, "idProduct"));
            let label = [read(&path, "manufacturer"), read(&path, "product")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
            Some(WakeDevice {
                name: if label.is_empty() { trf!("USB device {}:{}", vendor, product) } else { label },
                enabled: wakeup == "enabled",
                wakeups: read(&path, "power/wakeup_count").parse().unwrap_or(0),
                sysfs: path,
                vendor,
                product,
            })
        })
        .collect();
    devices.sort_by_key(|d| d.name.to_lowercase());
    devices
}

fn is_usb_id(id: &str) -> bool {
    id.len() == 4 && id.chars().all(|c| c.is_ascii_hexdigit())
}

/// Allows or stops a device from waking the computer, now and after
/// restarts (through a udev rule). Asks for the administrator password.
pub fn set_wake(device: &WakeDevice, enabled: bool) -> Result<()> {
    if !is_usb_id(&device.vendor) || !is_usb_id(&device.product) {
        bail!("{}", tr("this device can't be changed"));
    }
    let sysfs = device.sysfs.canonicalize().context(tr("the device was unplugged"))?;
    let wakeup = sysfs.join("power/wakeup");
    if !sysfs.starts_with("/sys/devices") || !wakeup.is_file() {
        bail!("{}", tr("this device can't be changed"));
    }
    let script = wake_script(&device.vendor, &device.product, &wakeup, enabled);
    let output = Command::new("pkexec").args(["sh", "-c", &script]).output().context(tr("pkexec is needed"))?;
    match output.status.code() {
        Some(0) => Ok(()),
        Some(126) | Some(127) => bail!("{}", tr("not changed: permission was not granted")),
        _ => bail!("{}", trf!("could not change it: {}", String::from_utf8_lossy(&output.stderr).trim())),
    }
}

fn wake_script(vendor: &str, product: &str, wakeup: &Path, enabled: bool) -> String {
    let matcher = format!("ATTRS{{idVendor}}==\"{vendor}\", ATTRS{{idProduct}}==\"{product}\"");
    let rule = format!("ACTION==\"add\", SUBSYSTEM==\"usb\", {matcher}, ATTR{{power/wakeup}}=\"disabled\"");
    let mut script = format!("set -e; f={UDEV_RULES}; touch \"$f\"; grep -vF '{matcher}' \"$f\" > \"$f.new\" || true; ");
    if !enabled {
        script.push_str(&format!("echo '{rule}' >> \"$f.new\"; "));
    }
    script.push_str(&format!(
        "mv \"$f.new\" \"$f\"; chmod 644 \"$f\"; echo {} > '{}'",
        if enabled { "enabled" } else { "disabled" },
        wakeup.display()
    ));
    script
}

// ---------------------------------------------------------------- Text

/// Local time like "Tue 11:42 PM".
pub fn when(t: f64) -> String {
    use chrono::TimeZone;
    use chrono::Datelike;
    const DAYS: [&str; 7] = [N_!("Mon"), N_!("Tue"), N_!("Wed"), N_!("Thu"), N_!("Fri"), N_!("Sat"), N_!("Sun")];
    chrono::Local.timestamp_opt(t as i64, 0).single().map_or_else(String::new, |d| {
        let day = tr(DAYS[d.weekday().num_days_from_monday() as usize]);
        // TRANSLATORS: a weekday and a time; {} is the weekday, the rest is a chrono time format
        // (%H:%M for 24-hour time).
        let time = d.format(tr("{} %-I:%M %p")).to_string();
        crate::i18n::fill(&time, &[&day])
    })
}

/// "7 h 23 min", "3 min".
pub fn duration(seconds: f64) -> String {
    let minutes = (seconds / 60.0).round() as i64;
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

/// One line about a sleep or a lid-closed period, and details under it.
pub fn describe_event(event: &Event) -> (String, Vec<String>) {
    match event {
        Event::Slept { start, end, cause, woke, battery, .. } => {
            let mut details = vec![trf!("Went to sleep because {}", tr(cause)), trf!("Woken by {}", tr(woke))];
            if let Some((before, after)) = battery {
                let mut text = trf!("Battery {}% → {}%", format!("{:.0}", before), format!("{:.0}", after));
                if let Some(rate) = drain_per_hour(*start, *end, *battery) {
                    text.push_str(&trf!(" ({}% an hour{})", format!("{:.1}", rate), if is_high_drain(rate) { tr(", more than usual") } else { "" }));
                }
                details.push(text);
            }
            (trf!("Slept {} → {} · {}", when(*start), when(*end), duration(end - start)), details)
        }
        Event::LidAwake { start, end } => (
            match end {
                Some(end) => trf!("Lid closed {} but stayed awake for {}", when(*start), duration(end - start)),
                None => trf!("Lid closed {} but it stayed awake", when(*start)),
            },
            vec![tr("Usually because a monitor or dock was connected, it was plugged in, or an app kept it awake").into()],
        ),
    }
}

pub fn render_text(report: &Report) -> String {
    let mut out = format!("{}\n", tr("Right now"));
    if report.blockers.is_empty() && report.notes.is_empty() {
        out.push_str(&format!("  {}\n", tr("Nothing is keeping the computer awake.")));
    }
    for b in &report.blockers {
        out.push_str(&format!("  {}: {}\n      {}\n", b.name, b.effect.describe(), b.reason));
    }
    for n in &report.notes {
        out.push_str(&format!("  {}\n      {}\n", n.title, n.detail));
    }
    out.push_str(&format!("\n{}\n", tr("Recent sleeps")));
    if let Some(problem) = &report.history_problem {
        out.push_str(&format!("  {problem}\n"));
    } else if report.history.is_empty() {
        out.push_str(&format!("  {}\n", tr("No sleeps in the last week.")));
    }
    for event in &report.history {
        let (title, details) = describe_event(event);
        out.push_str(&format!("  {title}\n"));
        for d in details {
            out.push_str(&format!("      {d}\n"));
        }
    }
    out.push_str(&format!("\n{}\n", tr("USB devices that can wake it")));
    let allowed: Vec<_> = report.devices.iter().filter(|d| d.enabled).collect();
    if allowed.is_empty() {
        out.push_str(&format!("  {}\n", tr("None.")));
    }
    for d in allowed {
        let woke = if d.wakeups > 0 { format!(" ({})", trnf!("woke it {} time", "woke it {} times", d.wakeups)) } else { String::new() };
        out.push_str(&format!("  {}{woke}\n", d.name));
    }
    if let Some(mode) = &report.sleep_mode {
        out.push_str(&format!("\n{}\n  {}\n", mode.title, mode.detail));
    }
    out
}

// ---------------------------------------------------------------- Demo

/// Typical content for the README screenshots (VOLTKIT_SLEEP_DEMO).
fn demo() -> Report {
    // 11:30 PM local time, some days ago.
    let night = |days_ago: f64| {
        use chrono::{Duration, Local, NaiveTime};
        let day = Local::now().date_naive() - Duration::days(days_ago as i64 + 1);
        day.and_time(NaiveTime::from_hms_opt(23, 30, 0).unwrap()).and_local_timezone(Local).single().map_or(0.0, |t| t.timestamp() as f64)
    };
    Report {
        blockers: vec![
            Blocker {
                name: "Videos".into(),
                app_id: Some("org.gnome.Totem".into()),
                reason: "Playing video".into(),
                effect: Effect::Idle,
                keep_awake: false,
            },
            Blocker {
                name: "Voltkit".into(),
                app_id: Some("io.github.arios4k.Voltkit".into()),
                reason: "Keep Awake is on".into(),
                effect: Effect::Sleep,
                keep_awake: true,
            },
        ],
        notes: vec![Note {
            title: "Never sleeps by itself when plugged in".into(),
            detail: "Automatic sleep is turned off for when the computer is plugged in, which it is now. Turn it on \
                     under Power Saving in the power settings."
                .into(),
            in_power_settings: true,
        }],
        history: vec![
            Event::Slept {
                start: night(0.0),
                end: night(0.0) + 20.0,
                cause: "you closed the lid".into(),
                woke: "something right away: often a USB device, the network or a driver".into(),
                unexpected: true,
                battery: None,
            },
            Event::Slept {
                start: night(1.0),
                end: night(1.0) + 8.4 * 3600.0,
                cause: "you closed the lid".into(),
                woke: "you opening the lid".into(),
                unexpected: false,
                battery: Some((82.0, 61.0)),
            },
            Event::LidAwake { start: night(2.0) + 3600.0, end: Some(night(2.0) + 3.0 * 3600.0) },
        ],
        history_problem: None,
        devices: vec![
            WakeDevice {
                sysfs: PathBuf::new(),
                name: "Logitech USB Receiver".into(),
                vendor: "046d".into(),
                product: "c52b".into(),
                enabled: true,
                wakeups: 3,
            },
            WakeDevice {
                sysfs: PathBuf::new(),
                name: "Keychron K2".into(),
                vendor: "05ac".into(),
                product: "024f".into(),
                enabled: false,
                wakeups: 0,
            },
        ],
        sleep_mode: sleep_mode("[s2idle] deep"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(t: f64, ident: &str, msg: &str) -> (f64, String, String) {
        (t, ident.to_owned(), msg.to_owned())
    }

    #[test]
    fn reads_journal_lines() {
        let l = "1793389737.346531 my-laptop systemd-sleep[73912]: Performing sleep operation 'suspend'...";
        assert_eq!(parse_journal_line(l), Some((1793389737.346531, "systemd-sleep".into(), "Performing sleep operation 'suspend'...".into())));
        assert_eq!(parse_journal_line("garbage"), None);
    }

    #[test]
    fn explains_sleeps_and_wakes() {
        let lines = vec![
            line(1000.0, "systemd-logind", "Lid closed."),
            line(1000.1, "systemd-logind", "Suspending..."),
            line(1001.0, "systemd-sleep", "Performing sleep operation 'suspend'..."),
            line(9000.0, "systemd-sleep", "System returned from sleep operation 'suspend'."),
            line(9002.0, "systemd-logind", "Lid opened."),
            // Woken by something else a few minutes after sleeping again.
            line(10000.0, "systemd-logind", "Power key pressed short."),
            line(10001.0, "systemd-sleep", "Performing sleep operation 'suspend'..."),
            line(10030.0, "systemd-sleep", "System returned from sleep operation 'suspend'."),
            // Woken by a key after an idle sleep: nothing unusual.
            line(20000.0, "systemd-sleep", "Performing sleep operation 'suspend'..."),
            line(23000.0, "systemd-sleep", "System returned from sleep operation 'suspend'."),
        ];
        let battery = [(900.0, 80.0), (9100.0, 70.0)];
        let events = parse_history(&lines, &battery);
        assert_eq!(events.len(), 3);
        match &events[0] {
            Event::Slept { cause, woke, unexpected, .. } => {
                assert_eq!(cause, "it was idle, or you chose Suspend");
                assert!(woke.starts_with("a key") && !unexpected);
            }
            other => panic!("{other:?}"),
        }
        match &events[2] {
            Event::Slept { cause, woke, unexpected, battery, .. } => {
                assert_eq!(cause, "you closed the lid");
                assert_eq!(woke, "you opening the lid");
                assert!(!unexpected);
                assert_eq!(*battery, Some((80.0, 70.0)));
            }
            other => panic!("{other:?}"),
        }
        match &events[1] {
            Event::Slept { cause, woke, unexpected, .. } => {
                assert_eq!(cause, "you pressed the power button");
                assert!(woke.contains("right away") && *unexpected);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn notices_a_lid_closed_without_sleep() {
        let lines = vec![line(1000.0, "systemd-logind", "Lid closed."), line(5000.0, "systemd-logind", "Lid opened.")];
        assert_eq!(parse_history(&lines, &[]), vec![Event::LidAwake { start: 1000.0, end: Some(5000.0) }]);
        // Opened again right away: not worth reporting.
        let quick = vec![line(1000.0, "systemd-logind", "Lid closed."), line(1010.0, "systemd-logind", "Lid opened.")];
        assert!(parse_history(&quick, &[]).is_empty());
    }

    #[test]
    fn ignores_a_sleep_cut_off_by_a_reboot() {
        let lines = vec![
            line(1000.0, "systemd-sleep", "Performing sleep operation 'suspend'..."),
            line(2000.0, "systemd-logind", "New seat seat0."),
        ];
        assert!(parse_history(&lines, &[]).is_empty());
    }

    #[test]
    fn measures_battery_drain() {
        let rate = drain_per_hour(0.0, 8.0 * 3600.0, Some((80.0, 64.0))).unwrap();
        assert!((rate - 2.0).abs() < 1e-9 && is_high_drain(rate));
        assert_eq!(drain_per_hour(0.0, 600.0, Some((80.0, 79.0))), None);
    }

    #[test]
    fn explains_lid_settings() {
        assert!(lid_note("suspend", "", "ignore", false, true).is_none());
        assert!(lid_note("suspend", "", "ignore", true, true).unwrap().title.contains("monitor"));
        assert!(lid_note("suspend", "ignore", "ignore", false, false).unwrap().title.contains("plugged in"));
        assert!(lid_note("ignore", "", "ignore", false, true).unwrap().title.contains("never"));
    }

    #[test]
    fn explains_sleep_modes() {
        assert!(sleep_mode("[s2idle] deep").unwrap().detail.contains("mem_sleep_default=deep"));
        assert!(sleep_mode("s2idle [deep]").is_none());
        assert!(sleep_mode("").is_none());
    }

    #[test]
    fn filters_inhibitors() {
        assert!(logind_blocker("handle-power-key:handle-suspend-key", "alexis", "GNOME handling keypresses", "block", 1, true).is_none());
        assert!(logind_blocker("sleep", "UPower", "Pause device polling", "delay", 1, true).is_none());
        assert!(logind_blocker("sleep", "alexis", "user session inhibited", "block", 1, true).is_none());
        let lid = logind_blocker("handle-lid-switch", "alexis", "External monitor attached or configuration changed recently", "block", 0, true).unwrap();
        assert_eq!(lid.effect, Effect::Lid);
        let b = gnome_blocker("/usr/lib/claude-desktop/claude-desktop", "Electron", Effect::Sleep);
        assert_eq!((b.name.as_str(), b.app_id, b.reason.as_str()), ("claude-desktop", None, "Asked the computer to stay awake"));
        assert!(gnome_blocker("", KEEP_AWAKE_REASON, Effect::Sleep).keep_awake);
    }

    #[test]
    fn writes_safe_wake_rules() {
        let s = wake_script("046d", "c52b", Path::new("/sys/devices/pci0000:00/usb1/1-4/power/wakeup"), false);
        assert!(s.contains(r#"ATTRS{idVendor}=="046d", ATTRS{idProduct}=="c52b""#));
        assert!(s.contains("ATTR{power/wakeup}=\"disabled\""));
        assert!(s.ends_with("echo disabled > '/sys/devices/pci0000:00/usb1/1-4/power/wakeup'"));
        let on = wake_script("046d", "c52b", Path::new("/sys/x/power/wakeup"), true);
        assert!(!on.contains(">> ") && on.contains("echo enabled"));
        assert!(!is_usb_id("04'6") && is_usb_id("046d"));
    }
}
