//! System Report: what's in this computer, explained. Hardware and drivers
//! with plain-language checks (and fixes), drive health, temperatures,
//! firmware updates, what changed since earlier days, and a report that's
//! safe to share (no serial numbers, names or network addresses).
//!
//! Sources: /proc and /sys, `lspci`, UDisks2 (drive health without a
//! password), hwmon sensors, fwupd, and, with the administrator password,
//! the SMBIOS tables for memory slots.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use gtk::gio;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};

use crate::doctor::{Distro, detect_distro};
use crate::i18n::tr;

/// How long daily snapshots are kept.
const SNAPSHOT_DAYS: usize = 90;

// ---------------------------------------------------------------- Model

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Ok,
    Info,
    Warning,
    Problem,
}

/// A finding about this computer, with what to do about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub level: Level,
    pub title: String,
    pub detail: String,
    /// A command that fixes it, to copy into a terminal.
    pub command: Option<String>,
    /// A guide to follow when there's no single safe command.
    pub link: Option<String>,
}

impl Check {
    fn new(level: Level, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Check { level, title: title.into(), detail: detail.into(), command: None, link: None }
    }
    fn command(mut self, command: Option<String>) -> Self {
        self.command = command;
        self
    }
    fn link(mut self, link: Option<&str>) -> Self {
        self.link = link.map(str::to_owned);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Graphics,
    Network,
    Audio,
    Storage,
    Usb,
    Other,
}

/// A PCI device, as `lspci` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub kind: Kind,
    pub class: String,
    pub vendor: String,
    pub name: String,
    pub driver: Option<String>,
}

impl Device {
    /// "Intel UHD Graphics", "NVIDIA Quadro P520".
    pub fn display_name(&self) -> String {
        let vendor = short_vendor(&self.vendor);
        // lspci puts the marketing name in brackets: "GP108GLM [Quadro P520]".
        let name = match (self.name.find('['), self.name.rfind(']')) {
            (Some(a), Some(b)) if b > a + 1 => &self.name[a + 1..b],
            _ => &self.name,
        };
        if name.starts_with(&vendor) { name.to_owned() } else { format!("{vendor} {name}") }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Drive {
    pub model: String,
    pub size: u64,
    pub kind: String,
    pub removable: bool,
    /// Critical warnings or a failing self-assessment.
    pub failing: bool,
    pub wear_percent: Option<u8>,
    pub power_on_hours: Option<u64>,
    pub bad_sectors: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sensor {
    pub label: String,
    pub celsius: f64,
    /// Above this it's running hot.
    pub high: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareUpdate {
    pub device: String,
    pub current: String,
    pub available: String,
    pub summary: String,
}

/// One installed memory module or empty slot, from SMBIOS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryModule {
    pub locator: String,
    /// None for an empty slot.
    pub size_mb: Option<u64>,
    pub kind: String,
    pub speed: Option<u32>,
    pub soldered: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub max_mb: Option<u64>,
    pub slots: Vec<MemoryModule>,
}

#[derive(Debug, Default)]
pub struct Report {
    /// (label, value) rows for "This Computer", in order.
    pub overview: Vec<(String, String)>,
    pub devices: Vec<Device>,
    pub drives: Vec<Drive>,
    pub sensors: Vec<Sensor>,
    pub fans: Vec<(String, u32)>,
    /// Times the processor slowed down because of heat since the computer started.
    pub throttled: u64,
    /// None when fwupd isn't available.
    pub firmware: Option<Vec<FirmwareUpdate>>,
    pub checks: Vec<Check>,
    /// What snapshots record, to find what changed.
    pub facts: BTreeMap<String, String>,
}

// ---------------------------------------------------------------- Collecting

pub fn collect() -> Report {
    if std::env::var_os("VOLTKIT_REPORT_DEMO").is_some() {
        return demo();
    }
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let devices = pci_devices();
    let drives = drives();
    let (sensors, fans) = sensors();
    let throttled = throttle_count();
    let firmware = firmware_updates();

    let mut overview = Vec::new();
    let mut add = |label: &str, value: String| {
        if !value.is_empty() {
            overview.push((tr(label).to_owned(), value));
        }
    };
    add(N_!("Computer"), computer_name());
    add(N_!("System"), os_value(&os_release, "PRETTY_NAME"));
    add(N_!("Kernel"), read_trim("/proc/sys/kernel/osrelease"));
    add(N_!("Desktop"), desktop());
    add(N_!("Processor"), processor());
    add(N_!("Memory"), memory());
    let list = |kind: Kind| devices.iter().filter(|d| d.kind == kind).map(device_line).collect::<Vec<_>>().join("\n");
    add(N_!("Graphics"), list(Kind::Graphics));
    add(N_!("Storage"), drives.iter().map(drive_line).collect::<Vec<_>>().join("\n"));
    add(N_!("Network"), list(Kind::Network));
    add(N_!("Audio"), list(Kind::Audio));
    add(N_!("Bluetooth"), bluetooth());
    add(N_!("Camera"), cameras().join("\n"));
    add("BIOS", bios());

    // Snapshots are compared day by day, so they're kept in English whatever the
    // language: a change of language mustn't show up as everything changing.
    // [`fact_label`] and [`fact_value`] translate them for display.
    let facts = crate::i18n::in_english(|| {
        let mut facts = BTreeMap::new();
        let mut fact = |label: String, value: String| {
            if !value.is_empty() {
                facts.insert(label, value);
            }
        };
        fact("Computer".into(), computer_name());
        fact("System".into(), os_value(&os_release, "PRETTY_NAME"));
        fact("Kernel".into(), read_trim("/proc/sys/kernel/osrelease"));
        fact("Processor".into(), processor());
        fact("Memory".into(), memory());
        fact("BIOS".into(), bios());
        for d in &devices {
            if matches!(d.kind, Kind::Graphics | Kind::Network | Kind::Audio | Kind::Storage) {
                fact(format!("Driver for {}", d.display_name()), driver_with_version(d.driver.as_deref()));
            }
        }
        for d in &drives {
            fact(format!("Drive {}", d.model), format_size(d.size));
        }
        for (name, version) in firmware_versions() {
            fact(format!("Firmware of {name}"), version);
        }
        facts
    });

    let mut report = Report { overview, devices, drives, sensors, fans, throttled, firmware, checks: Vec::new(), facts };
    report.checks = checks(&report, &os_release);
    report
}

fn read_trim(path: impl AsRef<Path>) -> String {
    std::fs::read_to_string(path).map(|s| s.trim().to_owned()).unwrap_or_default()
}

fn os_value(os_release: &str, key: &str) -> String {
    os_release
        .lines()
        .find_map(|l| l.strip_prefix(&format!("{key}=")))
        .map(|v| v.trim_matches('"').to_owned())
        .unwrap_or_default()
}

fn short_vendor(vendor: &str) -> String {
    let v = vendor.to_lowercase();
    let known = [
        ("intel", "Intel"),
        ("nvidia", "NVIDIA"),
        ("advanced micro devices", "AMD"),
        ("amd", "AMD"),
        ("realtek", "Realtek"),
        ("qualcomm", "Qualcomm"),
        ("broadcom", "Broadcom"),
        ("mediatek", "MediaTek"),
        ("lenovo", "Lenovo"),
        ("dell", "Dell"),
        ("hewlett", "HP"),
        ("hp", "HP"),
        ("asustek", "ASUS"),
        ("micro-star", "MSI"),
        ("acer", "Acer"),
        ("samsung", "Samsung"),
        ("apple", "Apple"),
        ("framework", "Framework"),
        ("microsoft", "Microsoft"),
        ("sandisk", "SanDisk"),
    ];
    known
        .iter()
        .find(|(k, _)| v.starts_with(k))
        .map(|(_, n)| (*n).to_owned())
        .unwrap_or_else(|| vendor.split_whitespace().next().unwrap_or(vendor).trim_end_matches(',').to_owned())
}

/// "Lenovo ThinkPad P14s Gen 1": some makers put the model in
/// product_version (Lenovo), most in product_name.
fn computer_name() -> String {
    let dmi = |f: &str| read_trim(format!("/sys/class/dmi/id/{f}"));
    let vendor = short_vendor(&dmi("sys_vendor"));
    let placeholder = |s: &str| {
        let l = s.to_lowercase();
        s.is_empty() || l.contains("to be filled") || l.contains("system version") || l == "none" || l == "default string"
            || l.chars().all(|c| c.is_ascii_digit() || c == '.')
    };
    let version = dmi("product_version");
    let name = dmi("product_name");
    let model = if !placeholder(&version) && version.chars().any(char::is_alphabetic) && version.contains(' ') {
        version
    } else {
        name
    };
    if placeholder(&model) {
        return vendor;
    }
    if model.to_lowercase().starts_with(&vendor.to_lowercase()) { model } else { format!("{vendor} {model}") }
}

fn desktop() -> String {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let desktop = desktop.split(':').next_back().unwrap_or("").to_owned();
    let session = match std::env::var("XDG_SESSION_TYPE").unwrap_or_default().as_str() {
        "wayland" => "Wayland",
        "x11" => "X11",
        _ => "",
    };
    match (desktop.is_empty(), session.is_empty()) {
        (false, false) => format!("{desktop} ({session})"),
        (false, true) => desktop,
        (true, false) => session.to_owned(),
        (true, true) => String::new(),
    }
}

/// "Intel Core i7-10610U · 4 cores, 8 threads, up to 4.9 GHz".
fn processor() -> String {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let model = cpuinfo.lines().find_map(|l| l.strip_prefix("model name")).and_then(|l| l.split_once(':')).map(|(_, v)| v.trim()).unwrap_or("");
    let model = clean_cpu_name(model);
    let threads = cpuinfo.lines().filter(|l| l.starts_with("processor")).count();
    let mut cores: Vec<(String, String)> = Vec::new();
    let mut physical = String::new();
    for line in cpuinfo.lines() {
        if let Some((k, v)) = line.split_once(':') {
            match k.trim() {
                "physical id" => physical = v.trim().to_owned(),
                "core id" => cores.push((physical.clone(), v.trim().to_owned())),
                _ => {}
            }
        }
    }
    cores.sort();
    cores.dedup();
    let max_khz: u64 = read_trim("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq").parse().unwrap_or(0);
    let mut parts = Vec::new();
    if !cores.is_empty() && cores.len() != threads {
        parts.push(trf!("{} cores, {} threads", cores.len(), threads));
    } else if threads > 0 {
        parts.push(trf!("{} cores", threads));
    }
    if max_khz > 0 {
        parts.push(trf!("up to {} GHz", format!("{:.1}", max_khz as f64 / 1e6)));
    }
    if parts.is_empty() { model } else { format!("{model} \u{00b7} {}", parts.join(", ")) }
}

fn clean_cpu_name(name: &str) -> String {
    let mut n = name.replace("(R)", "").replace("(TM)", "").replace("(tm)", "");
    for cut in [" CPU @", " @ ", " with Radeon", " w/ Radeon"] {
        if let Some(i) = n.find(cut) {
            n.truncate(i);
        }
    }
    let n = n.trim_end_matches(" Processor").trim_end_matches(" CPU");
    n.split_whitespace().filter(|w| !w.ends_with("-Core")).collect::<Vec<_>>().join(" ")
}

fn meminfo_kb(key: &str) -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .unwrap_or_default()
        .lines()
        .find_map(|l| l.strip_prefix(key))
        .and_then(|l| l.trim_start_matches(':').split_whitespace().next()?.parse().ok())
        .unwrap_or(0)
}

/// "47 GB usable" (some memory goes to graphics and firmware).
fn memory() -> String {
    let total = meminfo_kb("MemTotal") as f64 / 1024.0 / 1024.0;
    if total <= 0.0 {
        return String::new();
    }
    let swap = meminfo_kb("SwapTotal") as f64 / 1024.0 / 1024.0;
    let mut out = trf!("{} GB usable", format!("{:.0}", total));
    if swap >= 0.5 {
        out.push_str(&trf!(" · {} GB swap", format!("{:.0}", swap)));
    }
    out
}

/// Devices from `lspci -vmmk`, one block per device.
fn pci_devices() -> Vec<Device> {
    let Ok(out) = Command::new("lspci").arg("-vmmk").output() else { return Vec::new() };
    parse_lspci(&String::from_utf8_lossy(&out.stdout))
}

pub fn parse_lspci(text: &str) -> Vec<Device> {
    text.split("\n\n")
        .filter_map(|block| {
            let field = |key: &str| block.lines().find_map(|l| l.strip_prefix(&format!("{key}:\t"))).map(str::to_owned);
            let class = field("Class")?;
            let kind = match class.as_str() {
                "VGA compatible controller" | "3D controller" | "Display controller" => Kind::Graphics,
                "Network controller" | "Ethernet controller" => Kind::Network,
                "Audio device" | "Multimedia audio controller" => Kind::Audio,
                "Non-Volatile memory controller" | "SATA controller" | "RAID bus controller" => Kind::Storage,
                "USB controller" => Kind::Usb,
                _ => Kind::Other,
            };
            Some(Device { kind, class, vendor: field("Vendor").unwrap_or_default(), name: field("Device").unwrap_or_default(), driver: field("Driver") })
        })
        .collect()
}

fn device_line(d: &Device) -> String {
    match &d.driver {
        Some(driver) => format!("{} ({driver})", d.display_name()),
        None => trf!("{} (no driver)", d.display_name()),
    }
}

fn driver_with_version(driver: Option<&str>) -> String {
    match driver {
        Some(d) => {
            let version = read_trim(format!("/sys/module/{d}/version"));
            if version.is_empty() { d.to_owned() } else { format!("{d} {version}") }
        }
        None => tr("no driver").into(),
    }
}

fn bluetooth() -> String {
    let Ok(dir) = std::fs::read_dir("/sys/class/bluetooth") else { return String::new() };
    let adapters: Vec<String> = dir
        .flatten()
        .filter(|e| !e.file_name().to_string_lossy().contains(':'))
        .map(|e| {
            let driver = std::fs::read_link(e.path().join("device/driver")).ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
            match driver {
                Some(d) => trf!("Available ({})", d),
                None => tr("Available").into(),
            }
        })
        .collect();
    adapters.join("\n")
}

fn cameras() -> Vec<String> {
    let Ok(dir) = std::fs::read_dir("/sys/class/video4linux") else { return Vec::new() };
    let mut names: Vec<String> = dir
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("video"))
        .map(|e| read_trim(e.path().join("name")))
        .map(|n| n.split(':').next().unwrap_or("").trim().to_owned())
        .filter(|n| !n.is_empty())
        .collect();
    names.sort();
    names.dedup();
    names
}

fn bios() -> String {
    let version = read_trim("/sys/class/dmi/id/bios_version").split_whitespace().collect::<Vec<_>>().join(" ").replace(" )", ")");
    let date = read_trim("/sys/class/dmi/id/bios_date");
    match (version.is_empty(), date.is_empty()) {
        (false, false) => format!("{version} ({date})"),
        (false, true) => version,
        _ => String::new(),
    }
}

pub fn format_size(bytes: u64) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 1000.0 { format!("{:.1} TB", gb / 1000.0).replace(".0 TB", " TB") } else { format!("{gb:.0} GB") }
}

fn drive_line(d: &Drive) -> String {
    let mut line = format!("{} \u{00b7} {} {}", d.model, format_size(d.size), d.kind);
    if d.failing {
        line.push_str(tr(" \u{00b7} failing"));
    } else if let Some(w) = d.wear_percent {
        line.push_str(&trf!(" · {}% worn", w));
    }
    line
}

// ---------------------------------------------------------------- Drives (UDisks2)

type ManagedObjects =
    std::collections::HashMap<zbus::zvariant::OwnedObjectPath, std::collections::HashMap<String, std::collections::HashMap<String, zbus::zvariant::OwnedValue>>>;

fn drives() -> Vec<Drive> {
    crate::portal::block_on(async { drives_async().await.unwrap_or_default() })
}

async fn drives_async() -> Result<Vec<Drive>> {
    let conn = zbus::Connection::system().await?;
    let reply = conn
        .call_method(Some("org.freedesktop.UDisks2"), "/org/freedesktop/UDisks2", Some("org.freedesktop.DBus.ObjectManager"), "GetManagedObjects", &())
        .await?;
    let (objects,): (ManagedObjects,) = reply.body().deserialize()?;
    let mut drives = Vec::new();
    for (path, interfaces) in &objects {
        let Some(drive) = interfaces.get("org.freedesktop.UDisks2.Drive") else { continue };
        let get_str = |m: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>, k: &str| {
            m.get(k).and_then(|v| String::try_from(v.try_clone().ok()?).ok()).unwrap_or_default()
        };
        let get_u64 = |m: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>, k: &str| m.get(k).and_then(|v| u64::try_from(v).ok());
        let get_bool = |m: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>, k: &str| m.get(k).and_then(|v| bool::try_from(v).ok()).unwrap_or(false);
        let size = get_u64(drive, "Size").unwrap_or(0);
        if size == 0 {
            continue; // empty card readers and optical drives
        }
        let model = [get_str(drive, "Vendor"), get_str(drive, "Model")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
        let rotation = drive.get("RotationRate").and_then(|v| i32::try_from(v).ok()).unwrap_or(0);
        let usb = get_str(drive, "ConnectionBus") == "usb";
        let removable = get_bool(drive, "Removable") || usb;
        let nvme = interfaces.get("org.freedesktop.UDisks2.NVMe.Controller");
        let ata = interfaces.get("org.freedesktop.UDisks2.Drive.Ata");
        let kind = if nvme.is_some() {
            tr("NVMe SSD")
        } else if usb {
            tr("USB drive")
        } else if rotation > 0 {
            tr("hard disk")
        } else {
            "SSD"
        }
        .to_owned();
        let mut d = Drive { model, size, kind, removable, failing: false, wear_percent: None, power_on_hours: None, bad_sectors: None };
        if let Some(nvme) = nvme {
            let warnings = nvme.get("SmartCriticalWarning").and_then(|v| Vec::<String>::try_from(v.try_clone().ok()?).ok()).unwrap_or_default();
            d.failing = !warnings.is_empty();
            d.power_on_hours = get_u64(nvme, "SmartPowerOnHours").filter(|h| *h > 0);
            if let Ok(reply) = conn
                .call_method(
                    Some("org.freedesktop.UDisks2"),
                    path.as_str(),
                    Some("org.freedesktop.UDisks2.NVMe.Controller"),
                    "SmartGetAttributes",
                    &(std::collections::HashMap::<String, zbus::zvariant::Value>::new(),),
                )
                .await
                && let Ok((attrs,)) = reply.body().deserialize::<(std::collections::HashMap<String, zbus::zvariant::OwnedValue>,)>()
            {
                d.wear_percent = attrs.get("percent_used").and_then(|v| u8::try_from(v).ok());
            }
        } else if let Some(ata) = ata.filter(|a| get_bool(a, "SmartEnabled")) {
            d.failing = get_bool(ata, "SmartFailing");
            d.power_on_hours = get_u64(ata, "SmartPowerOnSeconds").map(|s| s / 3600).filter(|h| *h > 0);
            d.bad_sectors = ata.get("SmartNumBadSectors").and_then(|v| i64::try_from(v).ok()).map(|n| n.max(0) as u64);
        }
        drives.push(d);
    }
    drives.sort_by(|a, b| a.removable.cmp(&b.removable).then(b.size.cmp(&a.size)));
    Ok(drives)
}

// ---------------------------------------------------------------- Sensors

fn sensors() -> (Vec<Sensor>, Vec<(String, u32)>) {
    let mut sensors = Vec::new();
    let mut fans = Vec::new();
    let Ok(dir) = std::fs::read_dir("/sys/class/hwmon") else { return (sensors, fans) };
    let mut chips: Vec<PathBuf> = dir.flatten().map(|e| e.path()).collect();
    chips.sort();
    for chip in chips {
        let name = read_trim(chip.join("name"));
        let temp = |i: u32, f: &str| read_trim(chip.join(format!("temp{i}_{f}"))).parse::<f64>().ok().map(|v| v / 1000.0);
        let label_of = |i: u32| read_trim(chip.join(format!("temp{i}_label")));
        // One meaningful reading per chip.
        let pick: Option<(u32, &str, f64)> = match name.as_str() {
            "coretemp" => (1..=8).find(|i| label_of(*i).starts_with("Package")).map(|i| (i, tr("Processor"), 90.0)),
            "k10temp" | "zenpower" => (1..=8).find(|i| matches!(label_of(*i).as_str(), "Tctl" | "Tdie")).or(Some(1)).map(|i| (i, tr("Processor"), 90.0)),
            "amdgpu" | "nouveau" | "radeon" => Some((1, tr("Graphics"), 85.0)),
            "nvme" => Some((1, tr("Drive"), 70.0)),
            "iwlwifi_1" | "iwlwifi" | "mt7921_phy0" | "ath11k_hwmon" => Some((1, tr("Wi-Fi"), 80.0)),
            _ => None,
        };
        if let Some((i, label, default_high)) = pick
            && let Some(c) = temp(i, "input")
            && c > 0.0
        {
            let high = temp(i, "max").filter(|m| *m > 40.0 && *m < 120.0).unwrap_or(default_high);
            let label = if sensors.iter().any(|s: &Sensor| s.label == label) { format!("{label} 2") } else { label.to_owned() };
            sensors.push(Sensor { label, celsius: c, high });
        }
        for i in 1..=4 {
            if let Ok(rpm) = read_trim(chip.join(format!("fan{i}_input"))).parse::<u32>() {
                let label = if fans.is_empty() { tr("Fan").to_owned() } else { trf!("Fan {}", fans.len() + 1) };
                fans.push((label, rpm));
            }
        }
    }
    (sensors, fans)
}

fn throttle_count() -> u64 {
    let Ok(dir) = std::fs::read_dir("/sys/devices/system/cpu") else { return 0 };
    let mut packages = std::collections::HashSet::new();
    let mut total = 0;
    for e in dir.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !name.starts_with("cpu") || !name[3..].chars().all(|c| c.is_ascii_digit()) || name.len() == 3 {
            continue;
        }
        let package = read_trim(e.path().join("topology/physical_package_id"));
        if packages.insert(package) {
            total += read_trim(e.path().join("thermal_throttle/package_throttle_count")).parse::<u64>().unwrap_or(0);
        }
    }
    total
}

pub fn temperature_verdict(s: &Sensor) -> &'static str {
    if s.celsius >= s.high + 5.0 {
        "hot"
    } else if s.celsius >= s.high - 10.0 {
        "warm"
    } else {
        "normal"
    }
}

// ---------------------------------------------------------------- Firmware (fwupd)

fn fwupdmgr(args: &[&str]) -> Option<String> {
    let out = Command::new("timeout").arg("30").arg("fwupdmgr").args(args).output().ok()?;
    // Exit code 2 means "nothing to do", with valid JSON.
    matches!(out.status.code(), Some(0) | Some(2)).then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn firmware_updates() -> Option<Vec<FirmwareUpdate>> {
    let text = fwupdmgr(&["get-updates", "--json", "--no-metadata-check"])?;
    Some(parse_firmware_updates(&text))
}

pub fn parse_firmware_updates(text: &str) -> Vec<FirmwareUpdate> {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else { return Vec::new() };
    json["Devices"]
        .as_array()
        .map(|devices| {
            devices
                .iter()
                .filter_map(|d| {
                    let release = d["Releases"].as_array()?.first()?;
                    Some(FirmwareUpdate {
                        device: d["Name"].as_str()?.to_owned(),
                        current: d["Version"].as_str().unwrap_or("").to_owned(),
                        available: release["Version"].as_str().unwrap_or("").to_owned(),
                        summary: release["Summary"].as_str().unwrap_or("").to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// (device, firmware version) for every device fwupd knows, for snapshots.
fn firmware_versions() -> Vec<(String, String)> {
    let Some(text) = fwupdmgr(&["get-devices", "--json"]) else { return Vec::new() };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return Vec::new() };
    let mut out: Vec<(String, String)> = json["Devices"]
        .as_array()
        .map(|devices| {
            devices
                .iter()
                .filter(|d| d["Flags"].as_array().is_some_and(|f| f.iter().any(|x| x == "updatable")))
                .filter_map(|d| Some((d["Name"].as_str()?.to_owned(), d["Version"].as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

/// Downloads the latest firmware list (needs the network).
pub fn refresh_firmware_list() -> Result<()> {
    let out = Command::new("timeout").args(["120", "fwupdmgr", "refresh", "--force"]).output().context(tr("fwupd isn't installed"))?;
    if out.status.success() || out.status.code() == Some(2) {
        Ok(())
    } else {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim())
    }
}

// ---------------------------------------------------------------- Checks

fn ubuntu_like(os_release: &str) -> bool {
    os_value(os_release, "ID") == "ubuntu" || os_value(os_release, "ID_LIKE").split_whitespace().any(|i| i == "ubuntu")
}

/// A driver fix for this distribution: a command where one is safe and
/// standard, otherwise the distribution's guide.
fn driver_fix(os_release: &str, nvidia: bool) -> (Option<String>, Option<&'static str>) {
    if ubuntu_like(os_release) {
        return (Some("sudo ubuntu-drivers install".into()), None);
    }
    match detect_distro(os_release) {
        Distro::Dnf if nvidia => (None, Some("https://rpmfusion.org/Howto/NVIDIA")),
        Distro::Pacman if nvidia => (None, Some("https://wiki.archlinux.org/title/NVIDIA")),
        _ => (None, None),
    }
}

fn checks(report: &Report, os_release: &str) -> Vec<Check> {
    let mut checks = Vec::new();

    for d in report.devices.iter().filter(|d| d.kind == Kind::Graphics) {
        let name = d.display_name();
        let nvidia = d.vendor.to_lowercase().starts_with("nvidia");
        let (command, link) = driver_fix(os_release, nvidia);
        match d.driver.as_deref() {
            None => checks.push(
                Check::new(Level::Problem, trf!("{} has no driver", name), tr("The screen works with a basic fallback, but video, games and external monitors may be slow or not work."))
                    .command(command)
                    .link(link),
            ),
            Some("simpledrm" | "efifb" | "vesafb" | "vesa" | "fbdev") => checks.push(
                Check::new(Level::Warning, trf!("{} uses a basic fallback driver", name), tr("Video and games are slow, and the computer may use more power than it should."))
                    .command(command)
                    .link(link),
            ),
            Some("nouveau") => checks.push(
                Check::new(
                    Level::Info,
                    trf!("{} uses the open-source driver (nouveau)", name),
                    tr("It works for everyday use, but NVIDIA's own driver is much faster for games and graphics work, and on laptops it can switch the card off to save battery."),
                )
                .command(command)
                .link(link),
            ),
            Some(driver) => checks.push(Check::new(Level::Ok, trf!("{} works", name), trf!("Driver: {}", driver_with_version(Some(driver))))),
        }
    }

    for d in report.devices.iter().filter(|d| d.kind == Kind::Network) {
        let name = d.display_name();
        let wifi = d.class == "Network controller";
        match &d.driver {
            None => {
                let (command, _) = driver_fix(os_release, false);
                checks.push(
                    Check::new(
                        Level::Problem,
                        trf!("{} has no driver", name),
                        if wifi { tr("Wi-Fi won't work until a driver is installed. Connect with a cable or a USB Wi-Fi adapter to install one.") } else { tr("This network connection won't work until a driver is installed.") },
                    )
                    .command(command),
                );
            }
            Some(driver) => checks.push(Check::new(
                Level::Ok,
                trf!("{} works", if wifi { tr("Wi-Fi") } else { tr("Wired network") }),
                trf!("{}, driver: {}", name, driver),
            )),
        }
    }

    for d in report.devices.iter().filter(|d| matches!(d.kind, Kind::Audio | Kind::Storage | Kind::Usb) && d.driver.is_none()) {
        checks.push(Check::new(Level::Warning, trf!("{} has no driver", d.display_name()), trf!("This device ({}) may not work.", d.class)));
    }

    for d in &report.drives {
        if d.failing {
            checks.push(Check::new(
                Level::Problem,
                trf!("{} may fail soon", d.model),
                tr("The drive reports a problem with its health. Back up your files now, and plan to replace it."),
            ));
        } else if let Some(bad) = d.bad_sectors.filter(|b| *b > 0) {
            checks.push(Check::new(Level::Warning, trf!("{} has {} damaged sectors", d.model, bad), tr("Damaged sectors often grow over time. Keep a backup of your files.")));
        } else if let Some(w) = d.wear_percent.filter(|w| *w >= 80) {
            checks.push(Check::new(Level::Warning, trf!("{} is {}% worn", d.model, w), tr("SSDs wear out with writes. It still works, but plan to replace it and keep a backup.")));
        } else if !d.removable {
            let mut detail = String::from(tr("No problems reported"));
            if let Some(w) = d.wear_percent {
                detail.push_str(&trf!(", {}% worn", w));
            }
            if let Some(h) = d.power_on_hours {
                detail.push_str(&trf!(", {} hours of use", h));
            }
            checks.push(Check::new(Level::Ok, trf!("{} is healthy", d.model), detail));
        }
    }

    for (label, path) in [(tr("The system drive"), PathBuf::from("/")), (tr("Your home folder"), dirs::home_dir().unwrap_or_default())] {
        let Ok(info) = gio::File::for_path(&path).query_filesystem_info("filesystem::size,filesystem::free", gio::Cancellable::NONE) else { continue };
        let (size, free) = (info.attribute_uint64("filesystem::size"), info.attribute_uint64("filesystem::free"));
        if size == 0 || (path != Path::new("/") && checks.iter().any(|c| c.detail.contains(&format_size(free)))) {
            continue;
        }
        let pct = free as f64 / size as f64 * 100.0;
        let detail = trf!("{} free of {}", format_size(free), format_size(size));
        if pct < 5.0 {
            checks.push(Check::new(Level::Problem, trf!("{} is almost full", label), trf!("{}. Apps and updates can fail; delete or move some files.", detail)));
        } else if pct < 10.0 {
            checks.push(Check::new(Level::Warning, trf!("{} is getting full", label), detail));
        }
    }

    for s in &report.sensors {
        if temperature_verdict(s) == "hot" {
            checks.push(Check::new(
                Level::Warning,
                trf!("{} is running hot ({} °C)", s.label, format!("{:.0}", s.celsius)),
                tr("Check that the air vents aren't blocked or dusty, and that it isn't on a soft surface."),
            ));
        }
    }
    if report.throttled > 0 {
        checks.push(Check::new(
            Level::Info,
            tr("The processor has slowed down because of heat"),
            trnf!(
                "{} time since the computer started. A few is normal under heavy work; many means it's overheating (dust, blocked vents, or old thermal paste).",
                "{} times since the computer started. A few is normal under heavy work; many means it's overheating (dust, blocked vents, or old thermal paste).",
                report.throttled
            ),
        ));
    }

    if Path::new("/var/run/reboot-required").exists() {
        checks.push(Check::new(Level::Info, tr("Restart to finish installing updates"), tr("Some updates only take effect after a restart.")));
    } else if let Some(newest) = newest_kernel()
        && newest != read_trim("/proc/sys/kernel/osrelease")
    {
        checks.push(Check::new(Level::Info, tr("A newer Linux kernel is installed"), trf!("Restart to start using {}.", newest)));
    }

    if let Some(updates) = &report.firmware {
        for u in updates {
            checks.push(
                Check::new(
                    Level::Info,
                    trf!("Firmware update for {}", u.device),
                    format!("{} \u{2192} {}. {}", u.current, u.available, u.summary).trim_end_matches(". ").to_owned(),
                )
                .command(Some("fwupdmgr update".into())),
            );
        }
    }

    checks.sort_by_key(|c| std::cmp::Reverse(c.level));
    checks
}

/// The highest kernel version with installed modules.
fn newest_kernel() -> Option<String> {
    let dir = std::fs::read_dir("/lib/modules").ok()?;
    let mut versions: Vec<String> = dir.flatten().filter(|e| e.path().join("modules.dep").exists()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    versions.sort_by_key(|v| version_key(v));
    versions.pop()
}

fn version_key(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit()).filter_map(|n| n.parse().ok()).collect()
}

// ---------------------------------------------------------------- Memory slots (SMBIOS, as root)

/// Reads the memory slots from the SMBIOS tables. Needs root.
pub fn memory_info() -> Result<MemoryInfo> {
    let table = std::fs::read("/sys/firmware/dmi/tables/DMI").context(tr("can't read the SMBIOS table"))?;
    Ok(parse_smbios_memory(&table))
}

/// Runs `voltkit report --privileged` as root through pkexec.
pub fn memory_info_as_root() -> Result<MemoryInfo> {
    let exe = std::env::current_exe()?;
    let out = Command::new("pkexec").arg(exe).args(["report", "--privileged"]).output().context(tr("pkexec is needed"))?;
    match out.status.code() {
        Some(0) => Ok(serde_json::from_slice(&out.stdout)?),
        Some(126) | Some(127) => bail!("{}", tr("permission was not granted")),
        _ => bail!("{}", String::from_utf8_lossy(&out.stderr).trim()),
    }
}

pub fn parse_smbios_memory(table: &[u8]) -> MemoryInfo {
    let mut info = MemoryInfo::default();
    let mut pos = 0;
    while pos + 4 <= table.len() {
        let kind = table[pos];
        let len = table[pos + 1] as usize;
        if len < 4 || pos + len > table.len() {
            break;
        }
        let data = &table[pos..pos + len];
        // Strings follow the formatted area, ending with two zero bytes.
        let mut end = pos + len;
        while end + 1 < table.len() && !(table[end] == 0 && table[end + 1] == 0) {
            end += 1;
        }
        let strings: Vec<String> =
            table[pos + len..end].split(|b| *b == 0).filter(|s| !s.is_empty()).map(|s| String::from_utf8_lossy(s).trim().to_owned()).collect();
        let string = |offset: usize| data.get(offset).and_then(|&i| (i as usize).checked_sub(1)).and_then(|i| strings.get(i).cloned()).unwrap_or_default();
        let word = |o: usize| (o + 2 <= data.len()).then(|| u16::from_le_bytes([data[o], data[o + 1]]));
        let dword = |o: usize| (o + 4 <= data.len()).then(|| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]));
        match kind {
            // Physical Memory Array: maximum capacity in KB, or bytes in the extended field.
            16 => {
                if let Some(max_kb) = dword(0x07) {
                    let mb = if max_kb == 0x8000_0000 {
                        (0x0F + 8 <= data.len()).then(|| u64::from_le_bytes(data[0x0F..0x17].try_into().unwrap()) / 1024 / 1024)
                    } else {
                        Some(max_kb as u64 / 1024)
                    };
                    info.max_mb = Some(info.max_mb.unwrap_or(0) + mb.unwrap_or(0)).filter(|m| *m > 0);
                }
            }
            // Memory Device: one per slot.
            17 => {
                let size_mb = match word(0x0C) {
                    None | Some(0) | Some(0xFFFF) => None,
                    Some(0x7FFF) => dword(0x1C).map(|mb| (mb & 0x7FFF_FFFF) as u64),
                    Some(s) if s & 0x8000 != 0 => Some(((s & 0x7FFF) as u64) / 1024),
                    Some(s) => Some(s as u64),
                };
                let kind = match data.get(0x12) {
                    Some(0x12) => "DDR",
                    Some(0x13) => "DDR2",
                    Some(0x18) => "DDR3",
                    Some(0x1A) => "DDR4",
                    Some(0x1B) => "LPDDR",
                    Some(0x1C) => "LPDDR2",
                    Some(0x1D) => "LPDDR3",
                    Some(0x1E) => "LPDDR4",
                    Some(0x22) => "DDR5",
                    Some(0x23) => "LPDDR5",
                    _ => "",
                }
                .to_owned();
                let speed = word(0x15).filter(|s| *s > 0 && *s != 0xFFFF).map(u32::from);
                // Form factor 0x0B is "row of chips": memory soldered to the board.
                let soldered = data.get(0x0E) == Some(&0x0B) || kind.starts_with("LP");
                info.slots.push(MemoryModule { locator: string(0x10), size_mb, kind, speed, soldered });
            }
            127 => break,
            _ => {}
        }
        pos = end + 2;
    }
    info
}

/// Plain-language upgrade advice from the memory slots.
pub fn memory_advice(info: &MemoryInfo) -> Vec<(String, String)> {
    let installed: Vec<&MemoryModule> = info.slots.iter().filter(|m| m.size_mb.is_some()).collect();
    if info.slots.is_empty() {
        return vec![(tr("Memory").into(), tr("This computer doesn't describe its memory slots").into())];
    }
    let total_gb = installed.iter().filter_map(|m| m.size_mb).sum::<u64>() as f64 / 1024.0;
    let kind = installed.iter().find(|m| !m.kind.is_empty()).map(|m| m.kind.clone()).unwrap_or_default();
    let speed = installed.iter().filter_map(|m| m.speed).max();
    let kind_speed = match (kind.is_empty(), speed) {
        (false, Some(s)) => format!(" {kind}-{s}"),
        (false, None) => format!(" {kind}"),
        _ => String::new(),
    };
    let slots: Vec<&MemoryModule> = info.slots.iter().filter(|m| !m.soldered).collect();
    let free = slots.iter().filter(|m| m.size_mb.is_none()).count();
    let soldered_gb = info.slots.iter().filter(|m| m.soldered).filter_map(|m| m.size_mb).sum::<u64>() as f64 / 1024.0;
    let mut rows = vec![(tr("Installed").into(), format!("{total_gb:.0} GB{kind_speed}"))];
    let layout = if slots.is_empty() {
        tr("All memory is soldered to the board: it can't be upgraded").to_owned()
    } else {
        let mut s = trnf!("{} slot, {} free", "{} slots, {} free", slots.len(), free);
        if soldered_gb > 0.0 {
            s.push_str(&trf!(", plus {} GB soldered to the board", format!("{:.0}", soldered_gb)));
        }
        s
    };
    rows.push((tr("Slots").into(), layout));
    for m in &info.slots {
        let what = match m.size_mb {
            Some(mb) => format!("{:.0} GB{}{}", mb as f64 / 1024.0, if m.kind.is_empty() { String::new() } else { format!(" {}", m.kind) }, if m.soldered { tr(", soldered") } else { "" }),
            None => tr("empty").into(),
        };
        let name = if m.locator.is_empty() { tr("Slot").to_owned() } else { m.locator.clone() };
        rows.push((name, what));
    }
    if let Some(max) = info.max_mb.filter(|_| !slots.is_empty()) {
        let max_gb = max as f64 / 1024.0;
        rows.push((tr("Maximum").into(), trf!("Up to {} GB{} (as the computer reports it)", format!("{:.0}", max_gb), kind_speed)));
    }
    rows
}

// ---------------------------------------------------------------- Shareable report

/// A summary for forums, bug reports or support: no serial numbers, user or
/// computer names, or network addresses.
pub fn share_text(report: &Report, changes: &[Change]) -> String {
    let mut out = format!("{}\n\n", trf!("System report (Voltkit {})", env!("CARGO_PKG_VERSION")));
    for (label, value) in &report.overview {
        let mut lines = value.lines();
        out.push_str(&format!("{label}: {}\n", lines.next().unwrap_or("")));
        for l in lines {
            out.push_str(&format!("{}  {l}\n", " ".repeat(label.chars().count())));
        }
    }
    let notable: Vec<&Check> = report.checks.iter().filter(|c| c.level != Level::Ok).collect();
    if !notable.is_empty() {
        out.push_str(&format!("\n{}\n", tr("Checks:")));
        for c in notable {
            let mark = match c.level {
                Level::Problem => "✗",
                Level::Warning => "⚠",
                _ => "ℹ",
            };
            out.push_str(&format!("{mark} {} ({})\n", c.title, c.detail));
        }
    }
    if !changes.is_empty() {
        out.push_str(&format!("\n{}\n", tr("Recent changes:")));
        for c in changes.iter().take(10) {
            out.push_str(&format!("- {}: {}\n", c.date, c.describe()));
        }
    }
    out
}

// ---------------------------------------------------------------- Snapshots

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// The day the change was first seen (YYYY-MM-DD).
    pub date: String,
    pub what: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

impl Change {
    /// What changed, in the user's language.
    pub fn label(&self) -> String {
        fact_label(&self.what)
    }

    pub fn describe(&self) -> String {
        let what = self.label();
        match (self.before.as_deref().map(fact_value), self.after.as_deref().map(fact_value)) {
            (Some(b), Some(a)) => format!("{what}: {b} \u{2192} {a}"),
            (None, Some(a)) => trf!("{}: added ({})", what, a),
            (Some(b), None) => trf!("{}: removed (was {})", what, b),
            (None, None) => what,
        }
    }
}

/// The name of a snapshot fact (kept in English) in the user's language.
pub fn fact_label(key: &str) -> String {
    if let Some(name) = key.strip_prefix("Driver for ") {
        trf!("Driver for {}", name)
    } else if let Some(name) = key.strip_prefix("Firmware of ") {
        trf!("Firmware of {}", name)
    } else if let Some(name) = key.strip_prefix("Drive ") {
        trf!("Drive {}", name)
    } else {
        tr(key).to_owned()
    }
}

/// A snapshot value (kept in English) in the user's language: the words in it
/// are translated, names and numbers are left as they are.
pub fn fact_value(value: &str) -> String {
    crate::i18n::translate_english(value, &[
        N_!("no driver"),
        N_!("{} cores, {} threads"),
        N_!("{} cores"),
        N_!("up to {} GHz"),
        N_!("{} GB usable"),
        N_!(" · {} GB swap"),
        N_!(" · {}% worn"),
    ])
}

fn snapshot_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("voltkit").join("snapshots"))
}

/// Saves today's snapshot (replacing an earlier one from today) and deletes
/// old ones.
pub fn save_snapshot(facts: &BTreeMap<String, String>) -> Result<()> {
    if std::env::var_os("VOLTKIT_REPORT_DEMO").is_some() || facts.is_empty() {
        return Ok(());
    }
    let dir = snapshot_dir().context(tr("no data folder"))?;
    std::fs::create_dir_all(&dir)?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    std::fs::write(dir.join(format!("{today}.json")), serde_json::to_vec_pretty(facts)?)?;
    let mut files = snapshot_files(&dir);
    while files.len() > SNAPSHOT_DAYS {
        let _ = std::fs::remove_file(files.remove(0));
    }
    Ok(())
}

/// Saves today's snapshot unless there already is one.
pub fn snapshot_if_due() -> Result<()> {
    let dir = snapshot_dir().context(tr("no data folder"))?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if dir.join(format!("{today}.json")).exists() {
        return Ok(());
    }
    save_snapshot(&collect().facts)
}

fn snapshot_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect())
        .unwrap_or_default();
    files.sort();
    files
}

/// Changes between consecutive daily snapshots, newest first.
pub fn changes() -> Vec<Change> {
    if std::env::var_os("VOLTKIT_REPORT_DEMO").is_some() {
        return demo_changes();
    }
    let Some(dir) = snapshot_dir() else { return Vec::new() };
    let snapshots: Vec<(String, BTreeMap<String, String>)> = snapshot_files(&dir)
        .into_iter()
        .filter_map(|p| {
            let date = p.file_stem()?.to_string_lossy().into_owned();
            Some((date, serde_json::from_slice(&std::fs::read(&p).ok()?).ok()?))
        })
        .collect();
    diff_snapshots(&snapshots)
}

pub fn diff_snapshots(snapshots: &[(String, BTreeMap<String, String>)]) -> Vec<Change> {
    let mut changes = Vec::new();
    for pair in snapshots.windows(2) {
        let ((_, before), (date, after)) = (&pair[0], &pair[1]);
        let keys: std::collections::BTreeSet<&String> = before.keys().chain(after.keys()).collect();
        for key in keys {
            let (b, a) = (before.get(key), after.get(key));
            if b != a {
                changes.push(Change { date: date.clone(), what: key.clone(), before: b.cloned(), after: a.cloned() });
            }
        }
    }
    changes.reverse();
    changes
}

/// "Nov 1" for a snapshot date.
pub fn short_date(date: &str) -> String {
    use chrono::Datelike;
    const MONTHS: [&str; 12] = [
        N_!("Jan"), N_!("Feb"), N_!("Mar"), N_!("Apr"), N_!("May"), N_!("Jun"),
        N_!("Jul"), N_!("Aug"), N_!("Sep"), N_!("Oct"), N_!("Nov"), N_!("Dec"),
    ];
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|d| {
            let month = tr(MONTHS[d.month0() as usize]);
            // TRANSLATORS: a short date; {} is the month's abbreviation, the rest is a
            // chrono date format (%-d is the day of the month).
            crate::i18n::fill(&d.format(tr("{} %-d")).to_string(), &[&month])
        })
        .unwrap_or_else(|_| date.to_owned())
}

// ---------------------------------------------------------------- Demo

/// A typical laptop, for the README screenshots (VOLTKIT_REPORT_DEMO).
fn demo() -> Report {
    let gpu = |vendor: &str, name: &str, driver: &str| Device {
        kind: Kind::Graphics,
        class: "VGA compatible controller".into(),
        vendor: vendor.into(),
        name: name.into(),
        driver: Some(driver.into()),
    };
    let devices = vec![
        gpu("Intel Corporation", "Alder Lake-P GT2 [Iris Xe Graphics]", "i915"),
        gpu("NVIDIA Corporation", "GA107M [GeForce RTX 3050 Mobile]", "nouveau"),
        Device { kind: Kind::Network, class: "Network controller".into(), vendor: "Intel Corporation".into(), name: "Alder Lake-P PCH CNVi WiFi [Wi-Fi 6E]".into(), driver: Some("iwlwifi".into()) },
    ];
    let drives = vec![Drive {
        model: "Samsung SSD 980 PRO 1TB".into(),
        size: 1_000_204_886_016,
        kind: "NVMe SSD".into(),
        removable: false,
        failing: false,
        wear_percent: Some(3),
        power_on_hours: Some(2140),
        bad_sectors: None,
    }];
    let sensors = vec![
        Sensor { label: "Processor".into(), celsius: 52.0, high: 90.0 },
        Sensor { label: "Graphics".into(), celsius: 47.0, high: 85.0 },
        Sensor { label: "Drive".into(), celsius: 38.0, high: 70.0 },
    ];
    let mut report = Report {
        overview: vec![
            ("Computer".into(), "Framework Laptop 16".into()),
            ("System".into(), "Fedora Linux 44 (Workstation Edition)".into()),
            ("Kernel".into(), "6.19.4-200.fc44.x86_64".into()),
            ("Desktop".into(), "GNOME (Wayland)".into()),
            ("Processor".into(), "AMD Ryzen 7 7840HS \u{00b7} 8 cores, 16 threads, up to 5.1 GHz".into()),
            ("Memory".into(), "31 GB usable \u{00b7} 8 GB swap".into()),
            ("Graphics".into(), "Intel Iris Xe Graphics (i915)\nNVIDIA GeForce RTX 3050 Mobile (nouveau)".into()),
            ("Storage".into(), "Samsung SSD 980 PRO 1TB \u{00b7} 1 TB NVMe SSD \u{00b7} 3% worn".into()),
            ("Network".into(), "Intel Wi-Fi 6E (iwlwifi)".into()),
        ],
        devices,
        drives,
        sensors,
        fans: vec![("Fan".into(), 2350)],
        throttled: 0,
        firmware: Some(vec![FirmwareUpdate {
            device: "System Firmware".into(),
            current: "3.03".into(),
            available: "3.05".into(),
            summary: "Improves battery life during sleep".into(),
        }]),
        checks: Vec::new(),
        facts: BTreeMap::new(),
    };
    let os = "ID=fedora\n";
    // Leave out checks that depend on the machine taking the screenshots, found by
    // their English titles; the list is the same in every language.
    let english = crate::i18n::in_english(|| checks(&report, os));
    report.checks = checks(&report, os)
        .into_iter()
        .zip(english)
        .filter(|(_, c)| !c.title.contains("full") && !c.title.contains("kernel") && !c.title.contains("Restart"))
        .map(|(c, _)| c)
        .collect();
    // The demo is written in English, like a saved snapshot: show it in the user's language.
    for (label, value) in &mut report.overview {
        *label = tr(label).to_owned();
        *value = fact_value(value);
    }
    for s in &mut report.sensors {
        s.label = tr(&s.label).to_owned();
    }
    for (label, _) in &mut report.fans {
        *label = tr(label).to_owned();
    }
    report
}

fn demo_changes() -> Vec<Change> {
    let day = |n: i64| (chrono::Local::now().date_naive() - chrono::Duration::days(n)).format("%Y-%m-%d").to_string();
    vec![
        Change { date: day(1), what: "Kernel".into(), before: Some("6.18.9-200.fc44.x86_64".into()), after: Some("6.19.4-200.fc44.x86_64".into()) },
        Change { date: day(1), what: "Firmware of Wi-Fi".into(), before: Some("89.202a2f7b".into()), after: Some("92.0x10b3".into()) },
        Change { date: day(9), what: "Driver for NVIDIA GeForce RTX 3050 Mobile".into(), before: Some("nvidia 570.86".into()), after: Some("nouveau".into()) },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_lspci() {
        let text = "Slot:\t00:02.0\nClass:\tVGA compatible controller\nVendor:\tIntel Corporation\nDevice:\tCometLake-U GT2 [UHD Graphics]\nDriver:\ti915\n\n\
                    Slot:\t3d:00.0\nClass:\t3D controller\nVendor:\tNVIDIA Corporation\nDevice:\tGP108GLM [Quadro P520]\nDriver:\tnouveau\n\n\
                    Slot:\t00:14.3\nClass:\tNetwork controller\nVendor:\tIntel Corporation\nDevice:\tComet Lake PCH-LP CNVi WiFi\n";
        let d = parse_lspci(text);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].display_name(), "Intel UHD Graphics");
        assert_eq!(d[1].display_name(), "NVIDIA Quadro P520");
        assert_eq!((d[2].kind, d[2].driver.clone()), (Kind::Network, None));
    }

    #[test]
    fn cleans_processor_names() {
        assert_eq!(clean_cpu_name("Intel(R) Core(TM) i7-10610U CPU @ 1.80GHz"), "Intel Core i7-10610U");
        assert_eq!(clean_cpu_name("AMD Ryzen 7 7840HS w/ Radeon 780M Graphics"), "AMD Ryzen 7 7840HS");
        assert_eq!(clean_cpu_name("AMD Ryzen 9 5950X 16-Core Processor"), "AMD Ryzen 9 5950X");
    }

    fn report_with(devices: Vec<Device>) -> Report {
        Report { devices, ..Default::default() }
    }

    fn titles(checks: &[Check]) -> Vec<String> {
        checks.iter().map(|c| c.title.clone()).collect()
    }

    #[test]
    fn checks_graphics_drivers() {
        let gpu = |vendor: &str, driver: Option<&str>| Device {
            kind: Kind::Graphics,
            class: "VGA compatible controller".into(),
            vendor: vendor.into(),
            name: "X [Card]".into(),
            driver: driver.map(str::to_owned),
        };
        let ubuntu = "ID=zorin\nID_LIKE=\"ubuntu debian\"\n";
        let c = checks(&report_with(vec![gpu("NVIDIA Corporation", Some("nouveau"))]), ubuntu);
        assert_eq!(c[0].level, Level::Info);
        assert_eq!(c[0].command.as_deref(), Some("sudo ubuntu-drivers install"));
        let c = checks(&report_with(vec![gpu("NVIDIA Corporation", None)]), "ID=fedora\n");
        assert_eq!((c[0].level, c[0].command.clone()), (Level::Problem, None));
        assert_eq!(c[0].link.as_deref(), Some("https://rpmfusion.org/Howto/NVIDIA"));
        let c = checks(&report_with(vec![gpu("Intel Corporation", Some("simpledrm"))]), "ID=arch\n");
        assert_eq!((c[0].level, c[0].command.clone(), c[0].link.clone()), (Level::Warning, None, None));
        let c = checks(&report_with(vec![gpu("Intel Corporation", Some("i915"))]), ubuntu);
        assert!(titles(&c).contains(&"Intel Card works".to_owned()));
    }

    #[test]
    fn checks_sort_worst_first() {
        let report = Report {
            devices: vec![Device { kind: Kind::Graphics, class: "VGA compatible controller".into(), vendor: "AMD".into(), name: "GPU".into(), driver: Some("amdgpu".into()) }],
            drives: vec![Drive { model: "Disk".into(), size: 1, kind: "SSD".into(), removable: false, failing: true, wear_percent: None, power_on_hours: None, bad_sectors: None }],
            ..Default::default()
        };
        let c = checks(&report, "");
        assert_eq!(c[0].level, Level::Problem);
        assert!(c[0].title.contains("may fail soon"));
    }

    #[test]
    fn rates_temperatures() {
        let s = |c: f64| Sensor { label: "Processor".into(), celsius: c, high: 90.0 };
        assert_eq!(temperature_verdict(&s(50.0)), "normal");
        assert_eq!(temperature_verdict(&s(85.0)), "warm");
        assert_eq!(temperature_verdict(&s(96.0)), "hot");
    }

    #[test]
    fn reads_firmware_updates() {
        let json = r#"{"Devices":[{"Name":"System Firmware","Version":"0.1.36","Releases":[{"Version":"0.1.38","Summary":"Lenovo ThinkPad System Update"}]}]}"#;
        assert_eq!(
            parse_firmware_updates(json),
            vec![FirmwareUpdate {
                device: "System Firmware".into(),
                current: "0.1.36".into(),
                available: "0.1.38".into(),
                summary: "Lenovo ThinkPad System Update".into()
            }]
        );
        assert!(parse_firmware_updates(r#"{"Devices":[]}"#).is_empty());
    }

    /// A tiny SMBIOS table: a memory array (max 64 GB, 2 slots), one 16 GB
    /// DDR4-3200 SO-DIMM and one empty slot.
    fn smbios() -> Vec<u8> {
        let mut t = Vec::new();
        // Type 16, length 0x17.
        let mut a = vec![16, 0x17, 0, 0, 3, 3, 3];
        a.extend_from_slice(&(64u32 * 1024 * 1024).to_le_bytes()); // max capacity in KB
        a.extend_from_slice(&[0xFE, 0xFF, 2, 0]); // error handle, 2 devices
        a.extend_from_slice(&[0; 8]);
        t.extend_from_slice(&a);
        t.extend_from_slice(&[0, 0]);
        let device = |size_mb: u16, kind: u8, speed: u16, locator: &str| {
            let mut d = vec![17, 0x28, 0, 0, 0, 0, 0xFE, 0xFF, 64, 0, 64, 0];
            d.extend_from_slice(&size_mb.to_le_bytes()); // 0x0C size
            d.push(0x0D); // 0x0E form factor: SO-DIMM
            d.push(0); // 0x0F set
            d.push(1); // 0x10 locator string
            d.push(2); // 0x11 bank string
            d.push(kind); // 0x12 type
            d.extend_from_slice(&[0, 0]); // 0x13 type detail
            d.extend_from_slice(&speed.to_le_bytes()); // 0x15 speed
            d.resize(0x28, 0);
            d.extend_from_slice(locator.as_bytes());
            d.push(0);
            d.extend_from_slice(b"BANK 0\0\0");
            d
        };
        t.extend(device(16384, 0x1A, 3200, "DIMM A"));
        t.extend(device(0, 0x02, 0, "DIMM B"));
        t.extend_from_slice(&[127, 4, 0, 0, 0, 0]);
        t
    }

    #[test]
    fn reads_memory_slots() {
        let info = parse_smbios_memory(&smbios());
        assert_eq!(info.max_mb, Some(64 * 1024));
        assert_eq!(info.slots.len(), 2);
        assert_eq!(info.slots[0], MemoryModule { locator: "DIMM A".into(), size_mb: Some(16384), kind: "DDR4".into(), speed: Some(3200), soldered: false });
        assert_eq!(info.slots[1].size_mb, None);
        let advice = memory_advice(&info);
        assert_eq!(advice[0], ("Installed".into(), "16 GB DDR4-3200".into()));
        assert_eq!(advice[1], ("Slots".into(), "2 slots, 1 free".into()));
        assert!(advice.last().unwrap().1.starts_with("Up to 64 GB"));
    }

    #[test]
    fn diffs_snapshots() {
        let snap = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>();
        let snapshots = vec![
            ("2026-10-30".to_owned(), snap(&[("Kernel", "6.18"), ("Drive A", "1 TB")])),
            ("2026-10-31".to_owned(), snap(&[("Kernel", "6.18"), ("Drive A", "1 TB")])),
            ("2026-11-01".to_owned(), snap(&[("Kernel", "6.19"), ("Drive B", "2 TB")])),
        ];
        let changes = diff_snapshots(&snapshots);
        assert_eq!(changes.len(), 3);
        assert!(changes.iter().all(|c| c.date == "2026-11-01"));
        let kernel = changes.iter().find(|c| c.what == "Kernel").unwrap();
        assert_eq!(kernel.describe(), "Kernel: 6.18 \u{2192} 6.19");
        assert!(changes.iter().any(|c| c.describe() == "Drive A: removed (was 1 TB)"));
        assert!(changes.iter().any(|c| c.describe() == "Drive B: added (2 TB)"));
    }

    #[test]
    fn share_text_has_no_private_details() {
        let report = demo();
        let text = share_text(&report, &demo_changes());
        assert!(text.contains("Processor: AMD Ryzen 7 7840HS"));
        let host = std::fs::read_to_string("/etc/hostname").unwrap_or_default();
        let user = std::env::var("USER").unwrap_or_default();
        for private in [host.trim(), user.as_str()] {
            if private.len() > 2 {
                assert!(!text.contains(private), "report contains {private}");
            }
        }
    }
}
