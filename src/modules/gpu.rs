//! GPU Tuner foundation: finds the graphics cards, reads their sensors and says,
//! for each card, which controls its hardware and driver expose and why the
//! others aren't available. Nothing here changes a setting.
//!
//! AMD cards are read from the amdgpu driver's sysfs files; NVIDIA cards from
//! `nvidia-smi`, one call for all cards. The sysfs root is a parameter so the
//! logic can be tested against a fake tree.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::nvml::{Nvml, NvmlError, Threshold};


/// A translated message as an owned string.
fn tr(text: &str) -> String {
    crate::i18n::tr(text).to_owned()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

impl Vendor {
    fn from_pci(id: &str) -> Self {
        match id.trim().to_ascii_lowercase().as_str() {
            "0x10de" => Vendor::Nvidia,
            "0x1002" => Vendor::Amd,
            "0x8086" => Vendor::Intel,
            _ => Vendor::Other,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Vendor::Nvidia => "NVIDIA",
            Vendor::Amd => "AMD",
            Vendor::Intel => "Intel",
            Vendor::Other => "GPU",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Gpu {
    /// PCI address such as `0000:2d:00.0`, the same for sysfs and nvidia-smi.
    pub address: String,
    pub vendor: Vendor,
    pub name: String,
    /// The kernel driver in use: `amdgpu`, `nvidia`, `nouveau`, `i915`...
    pub driver: Option<String>,
    /// The `device` directory in sysfs.
    pub device: PathBuf,
}

/// One reading of a card's sensors. A missing value means the card or its
/// driver doesn't report it, never zero.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sample {
    pub temp_c: Option<f64>,
    pub hotspot_c: Option<f64>,
    pub mem_temp_c: Option<f64>,
    pub util_pct: Option<f64>,
    pub core_mhz: Option<f64>,
    pub mem_mhz: Option<f64>,
    pub power_w: Option<f64>,
    pub limit_w: Option<f64>,
    pub limit_min_w: Option<f64>,
    pub limit_max_w: Option<f64>,
    pub fan_pct: Option<f64>,
    pub fan_rpm: Option<f64>,
    pub vram_used_mb: Option<f64>,
    pub vram_total_mb: Option<f64>,
    /// Why the card is holding its clock back right now: a mix of [`THROTTLE_POWER`],
    /// [`THROTTLE_HEAT`] and [`THROTTLE_OTHER`]; `Some(0)` means nothing is. `None` when not reported.
    pub throttle: Option<u32>,
}

/// The card is at its power limit.
pub const THROTTLE_POWER: u32 = 1;
/// The card is too hot, or the hardware cut the clock to cool it.
pub const THROTTLE_HEAT: u32 = 2;
/// Any other cause: voltage, a sync boost, an application clock.
pub const THROTTLE_OTHER: u32 = 4;

/// What a card might let the user adjust.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    Monitoring,
    FanSpeed,
    FanCurve,
    PowerLimit,
    TemperatureTarget,
    ClockOffsets,
    ClockLimits,
    VoltageCurve,
}

impl Control {
    pub const ALL: [Control; 8] = [
        Control::Monitoring,
        Control::FanSpeed,
        Control::FanCurve,
        Control::PowerLimit,
        Control::TemperatureTarget,
        Control::ClockOffsets,
        Control::ClockLimits,
        Control::VoltageCurve,
    ];

    pub fn title(self) -> String {
        tr(match self {
            Control::Monitoring => N_!("Monitoring"),
            Control::FanSpeed => N_!("Fan speed"),
            Control::FanCurve => N_!("Fan curve"),
            Control::PowerLimit => N_!("Power limit"),
            Control::TemperatureTarget => N_!("Temperature target"),
            Control::ClockOffsets => N_!("Clock offsets"),
            Control::ClockLimits => N_!("Clock limits"),
            Control::VoltageCurve => N_!("Voltage and frequency curve"),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Support {
    /// The hardware and driver expose it, so Voltkit can read or drive it.
    Detected(String),
    /// Not available, with the reason and, when there is one, what to do.
    Unavailable { reason: String, how_to: Option<String> },
}

#[derive(Clone, Debug)]
pub struct Capability {
    pub control: Control,
    pub support: Support,
}

#[derive(Clone, Debug)]
pub struct Card {
    pub gpu: Gpu,
    pub sample: Sample,
    pub capabilities: Vec<Capability>,
    /// What NVML said about an NVIDIA card: current values to restore, and fans.
    pub probe: Option<NvidiaProbe>,
}

// ---------------------------------------------------------------- Detection

pub fn detect() -> Vec<Gpu> {
    detect_in(Path::new("/sys"))
}

pub fn detect_in(sys: &Path) -> Vec<Gpu> {
    let Ok(entries) = std::fs::read_dir(sys.join("class/drm")) else { return Vec::new() };
    let mut cards: Vec<(u32, Gpu)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(number) = name.strip_prefix("card").and_then(|n| n.parse::<u32>().ok()) else { continue };
        let device = entry.path().join("device");
        let Some(vendor_id) = read(&device.join("vendor")) else { continue };
        let vendor = Vendor::from_pci(&vendor_id);
        let address = device
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .filter(|a| a.contains(':'))
            .unwrap_or_else(|| name.clone());
        let driver = std::fs::read_link(device.join("driver")).ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
        let device_id = read(&device.join("device")).unwrap_or_default();
        let gpu_name = read(&device.join("product_name"))
            .or_else(|| lspci_name(&address))
            .unwrap_or_else(|| format!("{} {}", vendor.name(), device_id));
        cards.push((number, Gpu { address, vendor, name: gpu_name, driver, device }));
    }
    cards.sort_by_key(|(n, _)| *n);
    cards.into_iter().map(|(_, g)| g).collect()
}

/// The card's marketing name from `lspci`, when it's installed.
fn lspci_name(address: &str) -> Option<String> {
    let out = Command::new("lspci").args(["-mm", "-s", address]).output().ok()?;
    let line = String::from_utf8_lossy(&out.stdout).into_owned();
    // 2d:00.0 "3D controller" "NVIDIA Corporation" "GP108GLM [Quadro P520]" ...
    let fields: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
    let device = fields.get(2)?;
    Some(match (device.find('['), device.rfind(']')) {
        (Some(a), Some(b)) if a < b => device[a + 1..b].to_owned(),
        _ => (*device).to_owned(),
    })
}

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn read_num(path: &Path) -> Option<f64> {
    read(path)?.parse().ok()
}

// ---------------------------------------------------------------- NVIDIA

const NVIDIA_FIELDS: &str = "pci.bus_id,name,temperature.gpu,utilization.gpu,clocks.gr,clocks.mem,power.draw,power.limit,\
                             power.min_limit,power.max_limit,fan.speed,memory.used,memory.total";

/// Readings from one `nvidia-smi` call for every NVIDIA card, by PCI address.
pub fn nvidia_query() -> HashMap<String, (String, Sample)> {
    if !crate::system::has("nvidia-smi") {
        return HashMap::new();
    }
    let mut cmd = Command::new("nvidia-smi");
    cmd.args(["--query-gpu", NVIDIA_FIELDS, "--format=csv,noheader,nounits"]);
    match crate::system::run(&mut cmd) {
        Ok(out) => parse_nvidia(&String::from_utf8_lossy(&out)),
        Err(_) => HashMap::new(),
    }
}

/// Parses `nvidia-smi --format=csv,noheader,nounits`. Values the card doesn't
/// report come as `[N/A]` or `[Not Supported]` and become `None`.
pub fn parse_nvidia(csv: &str) -> HashMap<String, (String, Sample)> {
    let mut out = HashMap::new();
    for line in csv.lines() {
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() < 13 {
            continue;
        }
        let n = |i: usize| f[i].parse::<f64>().ok();
        // "00000000:2D:00.0" in nvidia-smi is "0000:2d:00.0" in sysfs.
        let id = f[0];
        let address = id[id.len().saturating_sub(12)..].to_ascii_lowercase();
        let sample = Sample {
            temp_c: n(2),
            util_pct: n(3),
            core_mhz: n(4),
            mem_mhz: n(5),
            power_w: n(6),
            limit_w: n(7),
            limit_min_w: n(8),
            limit_max_w: n(9),
            fan_pct: n(10),
            vram_used_mb: n(11),
            vram_total_mb: n(12),
            ..Sample::default()
        };
        out.insert(address, (f[1].to_owned(), sample));
    }
    out
}

// ---------------------------------------------------------------- AMD

fn amd_hwmon(device: &Path) -> Option<PathBuf> {
    std::fs::read_dir(device.join("hwmon")).ok()?.flatten().map(|e| e.path()).min()
}

fn amd_sample(device: &Path) -> Sample {
    let mut s = Sample::default();
    if let Some(hw) = amd_hwmon(device) {
        let milli = |name: &str| read_num(&hw.join(name)).map(|v| v / 1000.0);
        let micro = |name: &str| read_num(&hw.join(name)).map(|v| v / 1e6);
        s.temp_c = milli("temp1_input").or_else(|| milli("temp2_input"));
        s.hotspot_c = milli("temp2_input").filter(|_| hw.join("temp1_input").exists());
        s.mem_temp_c = milli("temp3_input");
        s.power_w = micro("power1_average").or_else(|| micro("power1_input"));
        s.limit_w = micro("power1_cap");
        s.limit_min_w = micro("power1_cap_min");
        s.limit_max_w = micro("power1_cap_max");
        s.fan_rpm = read_num(&hw.join("fan1_input"));
        s.fan_pct = read_num(&hw.join("pwm1")).map(|v| v / 255.0 * 100.0);
        s.core_mhz = micro("freq1_input");
        s.mem_mhz = micro("freq2_input");
    }
    s.util_pct = read_num(&device.join("gpu_busy_percent"));
    let mib = |name: &str| read_num(&device.join(name)).map(|v| v / (1024.0 * 1024.0));
    s.vram_used_mb = mib("mem_info_vram_used");
    s.vram_total_mb = mib("mem_info_vram_total");
    s
}

fn amd_capabilities(gpu: &Gpu, sample: &Sample, sys: &Path) -> Vec<Capability> {
    let device = &gpu.device;
    let hw = amd_hwmon(device);
    let in_hw = |name: &str| hw.as_ref().is_some_and(|h| h.join(name).exists());
    let od_table = read(&device.join("pp_od_clk_voltage")).unwrap_or_default();
    let od_present = device.join("pp_od_clk_voltage").exists();
    let mask = read(&sys.join("module/amdgpu/parameters/ppfeaturemask"))
        .and_then(|m| u64::from_str_radix(m.trim_start_matches("0x"), if m.starts_with("0x") { 16 } else { 10 }).ok());
    let overdrive_off = !od_present && mask.is_some_and(|m| m & 0x4000 == 0);
    let overdrive_missing = |what: &str| {
        if overdrive_off {
            Support::Unavailable {
                reason: trf!("Overdrive is turned off in the amdgpu driver, so {} can't be changed.", what),
                how_to: Some(tr("Add amdgpu.ppfeaturemask=0xffffffff to the kernel command line and restart.")),
            }
        } else {
            Support::Unavailable {
                reason: trf!("The amdgpu driver doesn't expose {} for this card.", what),
                how_to: None,
            }
        }
    };

    let mut out = Vec::new();
    for control in Control::ALL {
        let support = match control {
            Control::Monitoring if hw.is_some() => Support::Detected(tr("Temperature, power, clocks, fan and memory are readable.")),
            Control::Monitoring => Support::Unavailable { reason: tr("The driver exposes no hardware sensors for this card."), how_to: None },
            Control::FanSpeed if in_hw("pwm1") && in_hw("pwm1_enable") => Support::Detected(tr("The driver accepts a manual fan speed.")),
            Control::FanSpeed | Control::FanCurve if !in_hw("fan1_input") && !in_hw("pwm1") => Support::Unavailable {
                reason: tr("No fan is visible to the driver. Laptop and integrated GPUs usually share the system fan, which firmware controls."),
                how_to: None,
            },
            Control::FanSpeed => Support::Unavailable { reason: tr("The driver shows the fan speed but doesn't allow setting it."), how_to: None },
            Control::FanCurve if device.join("gpu_od/fan_ctrl/fan_curve").exists() => {
                Support::Detected(tr("The driver has a built-in fan curve."))
            }
            Control::FanCurve if in_hw("pwm1") && in_hw("pwm1_enable") => {
                Support::Detected(tr("Voltkit can follow a curve by adjusting the fan speed itself."))
            }
            Control::FanCurve => Support::Unavailable { reason: tr("The driver doesn't allow setting the fan speed."), how_to: None },
            Control::PowerLimit => match (sample.limit_min_w, sample.limit_max_w) {
                (Some(min), Some(max)) if max > min && in_hw("power1_cap") => {
                    Support::Detected(trf!("Adjustable from {} W to {} W.", format!("{min:.0}"), format!("{max:.0}")))
                }
                _ => Support::Unavailable { reason: tr("The driver doesn't report an adjustable power limit."), how_to: None },
            },
            Control::TemperatureTarget if device.join("gpu_od/fan_ctrl/fan_target_temperature").exists() => {
                Support::Detected(tr("The driver lets you set the temperature the card's fan control aims for."))
            }
            Control::TemperatureTarget if overdrive_off => overdrive_missing(&tr("the temperature target")),
            Control::TemperatureTarget => Support::Unavailable {
                reason: tr("The driver doesn't expose a temperature target for this card. It needs a recent card (RDNA 3 or newer) and Linux 6.7 or newer."),
                how_to: None,
            },
            Control::ClockOffsets if od_present && od_table.contains("OD_") => {
                Support::Detected(tr("The driver's overdrive table is available."))
            }
            Control::ClockOffsets => overdrive_missing(&tr("clock offsets")),
            Control::ClockLimits => Support::Unavailable { reason: tr("This driver offers clock offsets, not clock limits."), how_to: None },
            Control::VoltageCurve if od_table.contains("OD_VDDC_CURVE") => {
                Support::Detected(tr("The card exposes a voltage and frequency curve."))
            }
            Control::VoltageCurve if od_table.contains("OD_VDDGFX_OFFSET") => {
                Support::Detected(tr("The card allows a voltage offset, not a full curve."))
            }
            Control::VoltageCurve if od_present => {
                Support::Unavailable { reason: tr("This card's driver exposes clocks but no voltage controls."), how_to: None }
            }
            Control::VoltageCurve => overdrive_missing(&tr("the voltage curve")),
        };
        out.push(Capability { control, support });
    }
    out
}

// ---------------------------------------------------------------- NVIDIA and others

/// What NVML says a card allows. Each answer is the driver's own, so a missing
/// control comes with its real reason.
#[derive(Clone, Debug)]
pub struct NvidiaProbe {
    pub power: Result<(f64, f64), NvmlError>,
    pub fans: Result<(u32, (u32, u32)), NvmlError>,
    pub core_offset: Result<i32, NvmlError>,
    pub mem_offset: Result<i32, NvmlError>,
    pub target: Result<u32, NvmlError>,
}

pub fn probe_nvidia(nvml: &Nvml, address: &str) -> NvidiaProbe {
    NvidiaProbe {
        // Some cards answer the range call with nonsense while having no power
        // management at all, so the range only counts when the current limit
        // can be read too and the numbers are plausible for a graphics card.
        power: nvml.power_limit_range(address).and_then(|(min, max)| {
            if max > min && max <= 1500.0 && nvml.power_limit(address).is_ok() { Ok((min, max)) } else { Err(NvmlError::Unsupported) }
        }),
        fans: nvml.fan_count(address).and_then(|n| if n == 0 { Err(NvmlError::Unsupported) } else { nvml.fan_range(address).map(|r| (n, r)) }),
        core_offset: nvml.clock_offset(address, false),
        mem_offset: nvml.clock_offset(address, true),
        target: nvml.threshold(address, Threshold::AcousticTarget),
    }
}

fn nvidia_capabilities(gpu: &Gpu, reported: bool, probe: Option<&NvidiaProbe>) -> Vec<Capability> {
    let proprietary = gpu.driver.as_deref() == Some("nvidia");
    let wrong_driver = |what: &str| Support::Unavailable {
        reason: trf!("The open-source nouveau driver doesn't allow changing {} on most cards.", what),
        how_to: Some(tr("Install NVIDIA's proprietary driver.")),
    };
    let why = |e: &NvmlError, what: &str, laptop_note: bool| match e {
        NvmlError::Unsupported => Support::Unavailable {
            reason: if laptop_note {
                trf!("The driver doesn't allow changing {} on this card. This is common on laptop GPUs.", what)
            } else {
                trf!("The driver doesn't allow changing {} on this card.", what)
            },
            how_to: None,
        },
        NvmlError::TooOld => Support::Unavailable {
            reason: trf!("The installed NVIDIA driver is too old to change {}.", what),
            how_to: Some(tr("Update the NVIDIA driver.")),
        },
        other => Support::Unavailable { reason: other.to_string(), how_to: None },
    };
    let mut out = Vec::new();
    for control in Control::ALL {
        let support = match control {
            Control::Monitoring if reported => Support::Detected(tr("Temperature, power, clocks, fan and memory are readable.")),
            Control::Monitoring if !proprietary => Support::Unavailable {
                reason: tr("Without NVIDIA's proprietary driver, only basic sensors are available."),
                how_to: Some(tr("Install NVIDIA's proprietary driver.")),
            },
            Control::Monitoring => Support::Unavailable {
                reason: tr("nvidia-smi isn't available, so the card can't be read."),
                how_to: Some(tr("Install the NVIDIA driver's utilities (nvidia-utils).")),
            },
            _ if !proprietary => wrong_driver(&control.title().to_lowercase()),
            _ if probe.is_none() => Support::Unavailable {
                reason: NvmlError::Missing.to_string(),
                how_to: Some(tr("Install the NVIDIA driver's utilities (nvidia-utils).")),
            },
            _ => {
                let probe = probe.unwrap();
                match control {
                    Control::FanSpeed | Control::FanCurve => match &probe.fans {
                        Ok((n, (min, max))) => Support::Detected(trf!("{} fan(s), adjustable from {} % to {} %.", n, min, max)),
                        Err(e) => why(e, &tr("the fan"), true),
                    },
                    Control::PowerLimit => match &probe.power {
                        Ok((min, max)) if max > min => {
                            Support::Detected(trf!("Adjustable from {} W to {} W.", format!("{min:.0}"), format!("{max:.0}")))
                        }
                        Ok(_) => Support::Unavailable { reason: tr("The card's power limit is fixed."), how_to: None },
                        Err(e) => why(e, &tr("the power limit"), true),
                    },
                    Control::TemperatureTarget => match &probe.target {
                        Ok(c) => Support::Detected(trf!("The card's fan control aims for {} °C.", c)),
                        Err(e) => why(e, &tr("the temperature target"), true),
                    },
                    Control::ClockOffsets => match (&probe.core_offset, &probe.mem_offset) {
                        (Ok(_), _) | (_, Ok(_)) => Support::Detected(tr("The driver reports core and memory clock offsets.")),
                        (Err(e), _) => why(e, &tr("clock offsets"), true),
                    },
                    Control::ClockLimits => {
                        Support::Unavailable { reason: tr("This driver offers clock offsets, not clock limits."), how_to: None }
                    }
                    Control::VoltageCurve => Support::Unavailable {
                        reason: tr("NVIDIA's driver offers clock offsets, not a voltage and frequency curve."),
                        how_to: None,
                    },
                    Control::Monitoring => unreachable!(),
                }
            }
        };
        out.push(Capability { control, support });
    }
    out
}

// ---------------------------------------------------------------- Intel

/// The frequency files of an Intel card. Different driver generations put them
/// in different places, so the first set that has a minimum and a maximum wins.
#[derive(Clone, Debug)]
pub struct IntelFreq {
    pub min: PathBuf,
    pub max: PathBuf,
    pub cur: Option<PathBuf>,
    pub act: Option<PathBuf>,
    pub rp0: Option<PathBuf>,
    pub rpn: Option<PathBuf>,
}

pub fn intel_freq(gpu: &Gpu) -> Option<IntelFreq> {
    let card = gpu.device.parent()?;
    let set = |dir: PathBuf, names: [&str; 6]| {
        let [min, max, cur, act, rp0, rpn] = names.map(|n| dir.join(n));
        (min.exists() && max.exists()).then(|| {
            let some = |p: PathBuf| p.exists().then_some(p);
            IntelFreq { min, max, cur: some(cur), act: some(act), rp0: some(rp0), rpn: some(rpn) }
        })
    };
    set(card.to_path_buf(), ["gt_min_freq_mhz", "gt_max_freq_mhz", "gt_cur_freq_mhz", "gt_act_freq_mhz", "gt_RP0_freq_mhz", "gt_RPn_freq_mhz"])
        .or_else(|| {
            set(card.join("gt/gt0"), ["rps_min_freq_mhz", "rps_max_freq_mhz", "rps_cur_freq_mhz", "rps_act_freq_mhz", "rps_RP0_freq_mhz", "rps_RPn_freq_mhz"])
        })
        .or_else(|| {
            set(gpu.device.join("tile0/gt0/freq0"), ["min_freq", "max_freq", "cur_freq", "act_freq", "rp0_freq", "rpn_freq"])
        })
}

/// How long the card has spent idle, in milliseconds, for working out utilization.
fn intel_idle_ms(gpu: &Gpu) -> Option<f64> {
    let card = gpu.device.parent()?;
    [card.join("gt/gt0/rc6_residency_ms"), card.join("power/rc6_residency_ms"), gpu.device.join("tile0/gt0/gtidle/idle_residency_ms")]
        .iter()
        .find_map(|p| read_num(p))
}

fn intel_sample(gpu: &Gpu) -> Sample {
    let mut s = Sample::default();
    if let Some(f) = intel_freq(gpu) {
        s.core_mhz = f.act.as_deref().and_then(read_num).or_else(|| f.cur.as_deref().and_then(read_num));
    }
    if let Some(hw) = amd_hwmon(&gpu.device) {
        let micro = |name: &str| read_num(&hw.join(name)).map(|v| v / 1e6);
        s.temp_c = read_num(&hw.join("temp1_input")).map(|v| v / 1000.0);
        s.fan_rpm = read_num(&hw.join("fan1_input"));
        // 0 means no limit is set.
        s.limit_w = micro("power1_max").filter(|w| *w > 0.0);
        s.limit_max_w = micro("power1_rated_max").filter(|w| *w > 0.0);
        s.limit_min_w = s.limit_max_w.map(|_| 1.0);
    }
    s
}

fn intel_capabilities(gpu: &Gpu, sample: &Sample) -> Vec<Capability> {
    let freq = intel_freq(gpu);
    let hw = amd_hwmon(&gpu.device);
    let in_hw = |name: &str| hw.as_ref().is_some_and(|h| h.join(name).exists());
    let mut out = Vec::new();
    for control in Control::ALL {
        let support = match control {
            Control::Monitoring if freq.is_some() => Support::Detected(tr("Clocks and utilization are readable; discrete cards also report temperature and fan.")),
            Control::Monitoring => Support::Unavailable { reason: tr("The driver exposes no frequency information for this card."), how_to: None },
            Control::ClockLimits => match &freq {
                Some(f) => {
                    let n = |p: &Option<PathBuf>| p.as_deref().and_then(read_num);
                    match (n(&f.rpn), n(&f.rp0)) {
                        (Some(lo), Some(hi)) => Support::Detected(trf!("Minimum and maximum clock, between {} MHz and {} MHz.", format!("{lo:.0}"), format!("{hi:.0}"))),
                        _ => Support::Detected(tr("Minimum and maximum clock.")),
                    }
                }
                None => Support::Unavailable { reason: tr("The driver doesn't expose clock limits for this card."), how_to: None },
            },
            Control::PowerLimit if in_hw("power1_max") && sample.limit_max_w.is_some() => {
                Support::Detected(trf!("Adjustable up to {} W.", format!("{:.0}", sample.limit_max_w.unwrap_or_default())))
            }
            Control::PowerLimit => Support::Unavailable {
                reason: tr("Integrated Intel graphics share the processor's power budget, which firmware sets. Only discrete Arc cards have their own limit."),
                how_to: None,
            },
            Control::FanSpeed | Control::FanCurve => Support::Unavailable {
                reason: if in_hw("fan1_input") {
                    tr("The card's fan is controlled by its firmware.")
                } else {
                    tr("No fan is visible to the driver. Integrated graphics use the system fan, which firmware controls.")
                },
                how_to: None,
            },
            Control::TemperatureTarget | Control::ClockOffsets | Control::VoltageCurve => Support::Unavailable {
                reason: tr("Intel's driver doesn't allow changing this."),
                how_to: None,
            },
        };
        out.push(Capability { control, support });
    }
    out
}

fn unsupported_capabilities() -> Vec<Capability> {
    Control::ALL
        .into_iter()
        .map(|control| Capability {
            control,
            support: Support::Unavailable {
                reason: tr("GPU Tuner covers NVIDIA, AMD and Intel cards; this one isn't."),
                how_to: None,
            },
        })
        .collect()
}

// ---------------------------------------------------------------- Putting it together

/// Set `VOLTKIT_GPU_DEMO` to show made-up cards and readings, for the README
/// screenshots on a computer without the right hardware.
pub fn demo() -> bool {
    std::env::var_os("VOLTKIT_GPU_DEMO").is_some()
}

/// A reading for demo card `card` at time `tick`: plausible numbers that move.
pub fn demo_sample(card: usize, tick: u32) -> Sample {
    let wave = |period: f64, phase: f64| ((f64::from(tick) / period + phase).sin() + 1.0) / 2.0;
    let load = 0.55 + 0.4 * wave(9.0, card as f64) * wave(23.0, 1.0);
    let (power, limit, clock, mem_clock, vram_total, fan_rpm) =
        if card == 0 { (265.0, 263.0, 2450.0, 1218.0, 16384.0, 2600.0) } else { (200.0, 200.0, 2700.0, 1313.0, 12288.0, 2300.0) };
    Sample {
        temp_c: Some((44.0 + 30.0 * load).round()),
        hotspot_c: (card == 0).then(|| (52.0 + 34.0 * load).round()),
        mem_temp_c: (card == 0).then(|| (46.0 + 24.0 * load).round()),
        util_pct: Some((100.0 * load).round()),
        core_mhz: Some((clock * (0.45 + 0.55 * load)).round()),
        mem_mhz: Some(mem_clock),
        power_w: Some((power * (0.3 + 0.7 * load)).round()),
        limit_w: Some(limit),
        limit_min_w: Some((limit * 0.8).round()),
        limit_max_w: Some((limit * 1.1).round()),
        fan_pct: Some((25.0 + 50.0 * load).round()),
        fan_rpm: Some((fan_rpm * (0.3 + 0.7 * load)).round()),
        vram_used_mb: Some((vram_total * (0.3 + 0.35 * wave(40.0, card as f64))).round()),
        vram_total_mb: Some(vram_total),
        // The big card brushes its power limit at the top of the load wave.
        throttle: Some(if card == 0 && load > 0.85 { THROTTLE_POWER } else if load > 0.93 { THROTTLE_HEAT } else { 0 }),
    }
}

/// A made-up voltage and frequency table, as an AMD card with a full curve shows it.
pub const DEMO_VF: &str = "OD_SCLK:\n0: 800Mhz\n1: 2100Mhz\nOD_VDDC_CURVE:\n0: 800MHz 700mV\n1: 1450MHz 790mV\n2: 2100MHz 1010mV\nOD_RANGE:\nSCLK: 800MHz 2150MHz\nVDDC_CURVE_SCLK[0]: 800MHz 1100MHz\nVDDC_CURVE_VOLT[0]: 650mV 900mV\nVDDC_CURVE_SCLK[1]: 1100MHz 1800MHz\nVDDC_CURVE_VOLT[1]: 700mV 1000mV\nVDDC_CURVE_SCLK[2]: 1800MHz 2150MHz\nVDDC_CURVE_VOLT[2]: 800mV 1200mV\n";

/// Two made-up cards: an AMD card with every control and an NVIDIA desktop card.
pub fn demo_cards() -> Vec<Card> {
    let gpu = |address: &str, vendor, name: &str, driver: &str| Gpu {
        address: address.into(),
        vendor,
        name: name.into(),
        driver: Some(driver.into()),
        device: PathBuf::from("/nonexistent"),
    };
    let amd = gpu("0000:03:00.0", Vendor::Amd, "AMD Radeon RX 7800 XT", "amdgpu");
    let sample = demo_sample(0, 90);
    let detected = |control, note: String| Capability { control, support: Support::Detected(note) };
    let amd_caps = vec![
        detected(Control::Monitoring, tr("Temperature, power, clocks, fan and memory are readable.")),
        detected(Control::FanSpeed, tr("The driver accepts a manual fan speed.")),
        detected(Control::FanCurve, tr("The driver has a built-in fan curve.")),
        detected(Control::PowerLimit, trf!("Adjustable from {} W to {} W.", "210", "290")),
        detected(Control::TemperatureTarget, tr("The driver lets you set the temperature the card's fan control aims for.")),
        detected(Control::ClockOffsets, tr("The driver's overdrive table is available.")),
        Capability {
            control: Control::ClockLimits,
            support: Support::Unavailable { reason: tr("This driver offers clock offsets, not clock limits."), how_to: None },
        },
        detected(Control::VoltageCurve, tr("The card allows a voltage offset, not a full curve.")),
    ];
    let nvidia = gpu("0000:01:00.0", Vendor::Nvidia, "NVIDIA GeForce RTX 4070", "nvidia");
    let probe = NvidiaProbe { power: Ok((100.0, 220.0)), fans: Ok((2, (30, 100))), core_offset: Ok(0), mem_offset: Ok(0), target: Ok(83) };
    let nvidia_caps = nvidia_capabilities(&nvidia, true, Some(&probe));
    vec![
        Card { gpu: amd, sample, capabilities: amd_caps, probe: None },
        Card { gpu: nvidia, sample: demo_sample(1, 90), capabilities: nvidia_caps, probe: Some(probe) },
    ]
}

/// Every card with its current reading and what it lets Voltkit control.
pub fn inspect() -> Vec<Card> {
    if demo() {
        return demo_cards();
    }
    let nvml = Nvml::open().ok();
    inspect_in(Path::new("/sys"), &nvidia_query(), nvml.as_ref())
}

pub fn inspect_in(sys: &Path, smi: &HashMap<String, (String, Sample)>, nvml: Option<&Nvml>) -> Vec<Card> {
    inspect_with(sys, smi, &|address| nvml.map(|n| probe_nvidia(n, address)), &|address| nvml.and_then(|n| n.sample(address).ok()))
}

/// [`inspect_in`] with the NVML answers supplied, so tests can make them up.
pub fn inspect_with(
    sys: &Path,
    smi: &HashMap<String, (String, Sample)>,
    probe: &dyn Fn(&str) -> Option<NvidiaProbe>,
    nvml_sample: &dyn Fn(&str) -> Option<Sample>,
) -> Vec<Card> {
    detect_in(sys)
        .into_iter()
        .map(|mut gpu| {
            let mut probe_out = None;
            let (sample, capabilities) = match gpu.vendor {
                Vendor::Amd => {
                    let sample = amd_sample(&gpu.device);
                    let capabilities = amd_capabilities(&gpu, &sample, sys);
                    (sample, capabilities)
                }
                Vendor::Nvidia => {
                    let reading = smi.get(&gpu.address);
                    if let Some((name, _)) = reading {
                        gpu.name = name.clone();
                    }
                    let sample = nvml_sample(&gpu.address).or_else(|| reading.map(|(_, s)| s.clone())).unwrap_or_default();
                    probe_out = probe(&gpu.address);
                    let capabilities = nvidia_capabilities(&gpu, reading.is_some() || sample != Sample::default(), probe_out.as_ref());
                    (sample, capabilities)
                }
                Vendor::Intel => {
                    let sample = intel_sample(&gpu);
                    let capabilities = intel_capabilities(&gpu, &sample);
                    (sample, capabilities)
                }
                Vendor::Other => (Sample::default(), unsupported_capabilities()),
            };
            Card { gpu, sample, capabilities, probe: probe_out }
        })
        .collect()
}

/// Reads every card repeatedly without reopening anything: NVML when it's
/// there (no process to start), `nvidia-smi` otherwise.
pub struct Monitor {
    gpus: Vec<Gpu>,
    nvml: Option<Nvml>,
    /// The last idle time of each Intel card, to work out utilization.
    idle: std::sync::Mutex<HashMap<String, (std::time::Instant, f64)>>,
    ticks: std::sync::atomic::AtomicU32,
}

impl Monitor {
    pub fn new(gpus: Vec<Gpu>) -> Self {
        Monitor { gpus, nvml: Nvml::open().ok(), idle: std::sync::Mutex::new(HashMap::new()), ticks: std::sync::atomic::AtomicU32::new(0) }
    }

    pub fn sample(&self) -> Vec<Sample> {
        if demo() {
            let tick = self.ticks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 91;
            return (0..self.gpus.len()).map(|i| demo_sample(i, tick)).collect();
        }
        let need_smi = self.nvml.is_none() && self.gpus.iter().any(|g| g.vendor == Vendor::Nvidia);
        let smi = if need_smi { nvidia_query() } else { HashMap::new() };
        self.gpus
            .iter()
            .map(|g| match g.vendor {
                Vendor::Amd => amd_sample(&g.device),
                Vendor::Nvidia => self
                    .nvml
                    .as_ref()
                    .and_then(|n| n.sample(&g.address).ok())
                    .or_else(|| smi.get(&g.address).map(|(_, s)| s.clone()))
                    .unwrap_or_default(),
                Vendor::Intel => {
                    let mut s = intel_sample(g);
                    if let Some(idle) = intel_idle_ms(g) {
                        let now = std::time::Instant::now();
                        let mut last = self.idle.lock().unwrap_or_else(|e| e.into_inner());
                        if let Some((then, was)) = last.insert(g.address.clone(), (now, idle)) {
                            let elapsed = now.duration_since(then).as_secs_f64() * 1000.0;
                            if elapsed > 0.0 {
                                s.util_pct = Some((100.0 * (1.0 - (idle - was) / elapsed)).clamp(0.0, 100.0));
                            }
                        }
                    }
                    s
                }
                Vendor::Other => Sample::default(),
            })
            .collect()
    }
}

// ---------------------------------------------------------------- Text

fn value(v: Option<f64>, unit: &str, digits: usize) -> String {
    v.map_or_else(|| "—".to_owned(), |v| format!("{v:.digits$} {unit}"))
}

/// The readings as label and text pairs, for the command line and the app.
pub fn readings(s: &Sample) -> Vec<(String, String)> {
    let mut out = vec![
        (tr("Temperature"), value(s.temp_c, "°C", 0)),
        (tr("Utilization"), value(s.util_pct, "%", 0)),
        (tr("Power"), value(s.power_w, "W", 0)),
        (tr("Core clock"), value(s.core_mhz, "MHz", 0)),
        (tr("Memory clock"), value(s.mem_mhz, "MHz", 0)),
    ];
    let fan = match (s.fan_pct, s.fan_rpm) {
        (Some(p), Some(r)) => format!("{p:.0} % · {r:.0} RPM"),
        (Some(p), None) => format!("{p:.0} %"),
        (None, Some(r)) => format!("{r:.0} RPM"),
        (None, None) => "—".to_owned(),
    };
    out.push((tr("Fan"), fan));
    let memory = match (s.vram_used_mb, s.vram_total_mb) {
        (Some(u), Some(t)) => format!("{u:.0} / {t:.0} MB"),
        _ => "—".to_owned(),
    };
    out.push((tr("Video memory"), memory));
    out
}

pub fn render_text(cards: &[Card]) -> String {
    if cards.is_empty() {
        return format!("{}\n", tr("No graphics cards were found."));
    }
    let mut out = String::new();
    for (i, card) in cards.iter().enumerate() {
        let driver = card.gpu.driver.as_deref().unwrap_or("—");
        out.push_str(&format!("{} {}: {} ({}, {})\n", tr("GPU"), i + 1, card.gpu.name, card.gpu.vendor.name(), driver));
        for (label, v) in readings(&card.sample) {
            out.push_str(&format!("  {label}: {v}\n"));
        }
        out.push_str(&format!("  {}\n", tr("Controls:")));
        for c in &card.capabilities {
            let (mark, text) = match &c.support {
                Support::Detected(note) => ("✓", note.clone()),
                Support::Unavailable { reason, how_to: Some(how) } => ("✗", format!("{reason} {how}")),
                Support::Unavailable { reason, how_to: None } => ("✗", reason.clone()),
            };
            out.push_str(&format!("    {mark} {}: {text}\n", c.control.title()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A fake `/sys` with one amdgpu card; `files` are paths under the card's
    /// `device` directory.
    fn amd_tree(files: &[(&str, &str)], mask: Option<&str>) -> tempdir::Tree {
        let tree = tempdir::Tree::new();
        let device = tree.path().join("class/drm/card0/device");
        fs::create_dir_all(device.join("hwmon/hwmon3")).unwrap();
        fs::write(device.join("vendor"), "0x1002\n").unwrap();
        fs::write(device.join("device"), "0x73bf\n").unwrap();
        fs::write(device.join("product_name"), "Radeon RX 6800 XT\n").unwrap();
        for (path, content) in files {
            let p = device.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }
        if let Some(mask) = mask {
            let p = tree.path().join("module/amdgpu/parameters/ppfeaturemask");
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, mask).unwrap();
        }
        tree
    }

    mod tempdir {
        use std::path::{Path, PathBuf};

        pub struct Tree(PathBuf);

        impl Tree {
            pub fn new() -> Self {
                static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let dir = std::env::temp_dir().join(format!("voltkit-gpu-test-{}-{n}", std::process::id()));
                std::fs::create_dir_all(&dir).unwrap();
                Tree(dir)
            }

            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for Tree {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    fn support(card: &Card, control: Control) -> &Support {
        &card.capabilities.iter().find(|c| c.control == control).unwrap().support
    }

    const FULL_AMD: &[(&str, &str)] = &[
        ("hwmon/hwmon3/temp1_input", "54000\n"),
        ("hwmon/hwmon3/temp2_input", "61000\n"),
        ("hwmon/hwmon3/power1_average", "92000000\n"),
        ("hwmon/hwmon3/power1_cap", "255000000\n"),
        ("hwmon/hwmon3/power1_cap_min", "0\n"),
        ("hwmon/hwmon3/power1_cap_max", "300000000\n"),
        ("hwmon/hwmon3/fan1_input", "1200\n"),
        ("hwmon/hwmon3/pwm1", "128\n"),
        ("hwmon/hwmon3/pwm1_enable", "2\n"),
        ("hwmon/hwmon3/freq1_input", "1500000000\n"),
        ("hwmon/hwmon3/freq2_input", "1000000000\n"),
        ("gpu_busy_percent", "37\n"),
        ("mem_info_vram_used", "1073741824\n"),
        ("mem_info_vram_total", "17179869184\n"),
        ("pp_od_clk_voltage", "OD_SCLK:\n0: 500Mhz\nOD_VDDC_CURVE:\n0: 500Mhz 700mV\n"),
        ("gpu_od/fan_ctrl/fan_target_temperature", "OD_FAN_TARGET_TEMPERATURE:\n75\n"),
    ];

    #[test]
    fn reads_an_amd_card() {
        let tree = amd_tree(FULL_AMD, Some("0xffffffff"));
        let cards = inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None);
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!(card.gpu.vendor, Vendor::Amd);
        assert_eq!(card.gpu.name, "Radeon RX 6800 XT");
        let s = &card.sample;
        assert_eq!((s.temp_c, s.hotspot_c, s.power_w, s.limit_max_w), (Some(54.0), Some(61.0), Some(92.0), Some(300.0)));
        assert_eq!((s.core_mhz, s.mem_mhz, s.util_pct, s.fan_rpm), (Some(1500.0), Some(1000.0), Some(37.0), Some(1200.0)));
        assert_eq!((s.vram_used_mb, s.vram_total_mb), (Some(1024.0), Some(16384.0)));
        assert!((s.fan_pct.unwrap() - 50.2).abs() < 0.1);
    }

    #[test]
    fn a_full_amd_card_offers_every_control() {
        let tree = amd_tree(FULL_AMD, Some("0xffffffff"));
        let card = &inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None)[0];
        // Clock limits are an Intel control; AMD cards take offsets.
        for c in Control::ALL.into_iter().filter(|c| *c != Control::ClockLimits) {
            assert!(matches!(support(card, c), Support::Detected(_)), "{c:?}: {:?}", support(card, c));
        }
    }

    #[test]
    fn explains_how_to_turn_on_amd_overdrive() {
        let tree = amd_tree(&FULL_AMD[..14], Some("0xfff7bfff"));
        let card = &inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None)[0];
        match support(card, Control::ClockOffsets) {
            Support::Unavailable { reason, how_to: Some(how) } => {
                assert!(reason.contains("Overdrive"));
                assert!(how.contains("ppfeaturemask"));
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(support(card, Control::PowerLimit), Support::Detected(_)));
    }

    #[test]
    fn an_amd_card_without_a_fan_says_why() {
        let files: Vec<_> = FULL_AMD.iter().filter(|(p, _)| !p.contains("fan1") && !p.contains("pwm1")).copied().collect();
        let tree = amd_tree(&files, Some("0xffffffff"));
        let card = &inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None)[0];
        for c in [Control::FanSpeed, Control::FanCurve] {
            match support(card, c) {
                Support::Unavailable { reason, .. } => assert!(reason.contains("No fan"), "{reason}"),
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn a_voltage_offset_card_isnt_called_a_curve() {
        let mut files = FULL_AMD[..14].to_vec();
        files.push(("pp_od_clk_voltage", "OD_SCLK:\nOD_VDDGFX_OFFSET:\n0mV\n"));
        let tree = amd_tree(&files, Some("0xffffffff"));
        let card = &inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None)[0];
        match support(card, Control::VoltageCurve) {
            Support::Detected(note) => assert!(note.contains("offset")),
            other => panic!("{other:?}"),
        }
    }

    fn laptop_probe() -> NvidiaProbe {
        NvidiaProbe {
            power: Err(NvmlError::Unsupported),
            fans: Err(NvmlError::Unsupported),
            core_offset: Err(NvmlError::Unsupported),
            mem_offset: Err(NvmlError::Unsupported),
            target: Err(NvmlError::Unsupported),
        }
    }

    fn desktop_probe() -> NvidiaProbe {
        NvidiaProbe {
            power: Ok((100.0, 240.0)),
            fans: Ok((2, (30, 100))),
            core_offset: Ok(0),
            mem_offset: Ok(0),
            target: Ok(83),
        }
    }

    const SMI_LAPTOP: &str = "00000000:2D:00.0, Quadro P520, 39, 4, 139, 405, [N/A], [N/A], [N/A], [N/A], [N/A], 5, 2048\n";
    const SMI_DESKTOP: &str = "00000000:01:00.0, NVIDIA GeForce RTX 3070, 61, 23, 1725, 7000, 104.20, 220.00, 100.00, 240.00, 45, 1021, 8192\n";

    fn nvidia_tree(address: &str, driver: &str) -> tempdir::Tree {
        let tree = tempdir::Tree::new();
        let pci = tree.path().join("devices").join(address);
        fs::create_dir_all(pci.join("hwmon")).unwrap();
        fs::write(pci.join("vendor"), "0x10de\n").unwrap();
        fs::write(pci.join("device"), "0x1d34\n").unwrap();
        fs::create_dir_all(tree.path().join("drivers").join(driver)).unwrap();
        std::os::unix::fs::symlink(tree.path().join("drivers").join(driver), pci.join("driver")).unwrap();
        fs::create_dir_all(tree.path().join("class/drm/card0")).unwrap();
        std::os::unix::fs::symlink(&pci, tree.path().join("class/drm/card0/device")).unwrap();
        tree
    }

    #[test]
    fn parses_nvidia_smi_and_matches_the_pci_address() {
        let parsed = parse_nvidia(SMI_LAPTOP);
        let (name, s) = &parsed["0000:2d:00.0"];
        assert_eq!(name, "Quadro P520");
        assert_eq!((s.temp_c, s.util_pct, s.core_mhz, s.power_w, s.fan_pct), (Some(39.0), Some(4.0), Some(139.0), None, None));
        assert_eq!((s.vram_used_mb, s.vram_total_mb), (Some(5.0), Some(2048.0)));
    }

    #[test]
    fn a_laptop_nvidia_card_explains_what_its_driver_hides() {
        let tree = nvidia_tree("0000:2d:00.0", "nvidia");
        let cards = inspect_with(tree.path(), &parse_nvidia(SMI_LAPTOP), &|_| Some(laptop_probe()), &|_| None);
        let card = &cards[0];
        assert_eq!(card.gpu.name, "Quadro P520");
        assert!(matches!(support(card, Control::Monitoring), Support::Detected(_)));
        for c in [Control::FanSpeed, Control::FanCurve, Control::PowerLimit, Control::TemperatureTarget, Control::ClockOffsets] {
            match support(card, c) {
                Support::Unavailable { reason, .. } => assert!(reason.contains("laptop"), "{c:?}: {reason}"),
                other => panic!("{c:?}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_desktop_nvidia_card_offers_fans_and_power() {
        let tree = nvidia_tree("0000:01:00.0", "nvidia");
        let card = &inspect_with(tree.path(), &parse_nvidia(SMI_DESKTOP), &|_| Some(desktop_probe()), &|_| None)[0];
        for c in [Control::FanSpeed, Control::FanCurve, Control::TemperatureTarget, Control::ClockOffsets] {
            assert!(matches!(support(card, c), Support::Detected(_)), "{c:?}");
        }
        match support(card, Control::PowerLimit) {
            Support::Detected(note) => assert!(note.contains("100") && note.contains("240")),
            other => panic!("{other:?}"),
        }
        assert!(matches!(support(card, Control::VoltageCurve), Support::Unavailable { .. }));
    }

    #[test]
    fn nouveau_points_to_the_proprietary_driver() {
        let tree = nvidia_tree("0000:2d:00.0", "nouveau");
        let card = &inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None)[0];
        match support(card, Control::PowerLimit) {
            Support::Unavailable { how_to: Some(how), .. } => assert!(how.contains("proprietary")),
            other => panic!("{other:?}"),
        }
    }

    fn intel_tree(files: &[(&str, &str)]) -> tempdir::Tree {
        let tree = tempdir::Tree::new();
        let card = tree.path().join("class/drm/card0");
        fs::create_dir_all(card.join("device")).unwrap();
        fs::write(card.join("device/vendor"), "0x8086\n").unwrap();
        for (path, content) in files {
            let p = card.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, content).unwrap();
        }
        tree
    }

    const IGPU: &[(&str, &str)] = &[
        ("gt_min_freq_mhz", "300\n"),
        ("gt_max_freq_mhz", "1150\n"),
        ("gt_cur_freq_mhz", "350\n"),
        ("gt_act_freq_mhz", "300\n"),
        ("gt_RP0_freq_mhz", "1150\n"),
        ("gt_RPn_freq_mhz", "300\n"),
    ];

    #[test]
    fn reads_and_describes_an_integrated_intel_card() {
        let tree = intel_tree(IGPU);
        let card = &inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None)[0];
        assert_eq!(card.gpu.vendor, Vendor::Intel);
        assert_eq!(card.sample.core_mhz, Some(300.0));
        assert!(matches!(support(card, Control::Monitoring), Support::Detected(_)));
        match support(card, Control::ClockLimits) {
            Support::Detected(note) => assert!(note.contains("300") && note.contains("1150"), "{note}"),
            other => panic!("{other:?}"),
        }
        match support(card, Control::PowerLimit) {
            Support::Unavailable { reason, .. } => assert!(reason.contains("Integrated")),
            other => panic!("{other:?}"),
        }
        for c in [Control::FanSpeed, Control::TemperatureTarget, Control::ClockOffsets, Control::VoltageCurve] {
            assert!(matches!(support(card, c), Support::Unavailable { .. }), "{c:?}");
        }
    }

    #[test]
    fn finds_frequency_files_in_newer_driver_layouts() {
        let gt0 = intel_tree(&[("gt/gt0/rps_min_freq_mhz", "300"), ("gt/gt0/rps_max_freq_mhz", "1100"), ("gt/gt0/rps_act_freq_mhz", "450")]);
        assert_eq!(inspect_with(gt0.path(), &HashMap::new(), &|_| None, &|_| None)[0].sample.core_mhz, Some(450.0));
        let xe = tempdir::Tree::new();
        let device = xe.path().join("class/drm/card0/device");
        fs::create_dir_all(device.join("tile0/gt0/freq0")).unwrap();
        fs::write(device.join("vendor"), "0x8086").unwrap();
        for (n, v) in [("min_freq", "400"), ("max_freq", "2400"), ("act_freq", "800")] {
            fs::write(device.join("tile0/gt0/freq0").join(n), v).unwrap();
        }
        assert_eq!(inspect_with(xe.path(), &HashMap::new(), &|_| None, &|_| None)[0].sample.core_mhz, Some(800.0));
    }

    #[test]
    fn a_discrete_arc_card_offers_a_power_limit() {
        let mut files = IGPU.to_vec();
        files.extend([("device/hwmon/hwmon2/power1_max", "120000000"), ("device/hwmon/hwmon2/power1_rated_max", "190000000"), ("device/hwmon/hwmon2/temp1_input", "58000")]);
        let tree = intel_tree(&files);
        let card = &inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None)[0];
        assert_eq!((card.sample.limit_w, card.sample.limit_max_w, card.sample.temp_c), (Some(120.0), Some(190.0), Some(58.0)));
        assert!(matches!(support(card, Control::PowerLimit), Support::Detected(_)));
    }

    #[test]
    fn other_vendors_are_listed_but_not_tuned() {
        let tree = tempdir::Tree::new();
        let device = tree.path().join("class/drm/card0/device");
        fs::create_dir_all(&device).unwrap();
        fs::write(device.join("vendor"), "0x1234\n").unwrap();
        let card = &inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None)[0];
        assert_eq!(card.gpu.vendor, Vendor::Other);
        assert!(card.capabilities.iter().all(|c| matches!(c.support, Support::Unavailable { .. })));
    }

    #[test]
    fn text_report_marks_each_control() {
        let tree = amd_tree(FULL_AMD, Some("0xffffffff"));
        let text = render_text(&inspect_with(tree.path(), &HashMap::new(), &|_| None, &|_| None));
        assert!(text.contains("Radeon RX 6800 XT"));
        assert!(text.contains("Temperature: 54 °C"));
        assert!(text.contains("✓ Power limit: Adjustable from 0 W to 300 W."));
    }
}
