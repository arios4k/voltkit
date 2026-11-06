//! GPU Tuner engine: validates what the user asks for, turns it into a preview
//! (a list of steps with the value each one will restore), and applies it
//! transactionally: if any step fails, the steps already done are undone.
//!
//! Nothing here claims to bypass the hardware, firmware or driver: every limit
//! comes from what the card reports, and a request outside it is refused.
//! AMD cards are changed through the amdgpu sysfs files, NVIDIA cards through
//! NVML. Hardware access goes through [`Hardware`] so the logic is tested with
//! a fake that can fail on any step.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::gpu::{Card, Control, Sample, Support, Vendor};
use super::nvml::{Nvml, Threshold};
use crate::i18n::tr;

fn t(text: &str) -> String {
    tr(text).to_owned()
}

// ---------------------------------------------------------------- Settings

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CurvePoint {
    pub temp_c: f64,
    pub speed_pct: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum FanMode {
    /// The card's own fan control.
    Auto,
    Fixed { speed_pct: f64 },
    Curve { points: Vec<CurvePoint> },
}

/// What to apply to one card. A field left out isn't touched.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Settings {
    /// The card's PCI address.
    pub gpu: String,
    pub power_limit_w: Option<f64>,
    pub fan: Option<FanMode>,
    pub temperature_target_c: Option<f64>,
    pub core_offset_mhz: Option<i32>,
    pub mem_offset_mhz: Option<i32>,
    pub voltage_offset_mv: Option<i32>,
}

// ---------------------------------------------------------------- Curves

/// The lowest fixed fan speed Voltkit allows, so a card is never left without airflow.
pub const MIN_FIXED_FAN_PCT: f64 = 20.0;
/// A curve must reach at least this speed by [`SAFE_TEMP_C`].
pub const SAFE_FAN_PCT: f64 = 60.0;
pub const SAFE_TEMP_C: f64 = 85.0;

/// Checks a fan curve: sorted by temperature, speeds that never drop as it
/// gets hotter, and enough airflow before the card gets hot.
pub fn validate_curve(points: &[CurvePoint]) -> Result<()> {
    if points.len() < 2 {
        bail!("{}", t("A fan curve needs at least two points."));
    }
    for p in points {
        if !(0.0..=110.0).contains(&p.temp_c) || !(0.0..=100.0).contains(&p.speed_pct) {
            bail!("{}", t("Curve points must be between 0 and 110 °C and between 0 and 100 %."));
        }
    }
    for w in points.windows(2) {
        if w[1].temp_c <= w[0].temp_c {
            bail!("{}", t("Curve points must go up in temperature."));
        }
        if w[1].speed_pct < w[0].speed_pct {
            bail!("{}", t("A hotter point can't have a slower fan than a cooler one."));
        }
    }
    if speed_at(points, SAFE_TEMP_C) < SAFE_FAN_PCT {
        bail!("{}", trf!("The fan must reach at least {} % by {} °C so the card stays safe.", SAFE_FAN_PCT, SAFE_TEMP_C));
    }
    Ok(())
}

/// The fan speed a curve gives at `temp`, by straight lines between points.
pub fn speed_at(points: &[CurvePoint], temp: f64) -> f64 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else { return 100.0 };
    if temp <= first.temp_c {
        return first.speed_pct;
    }
    if temp >= last.temp_c {
        return last.speed_pct;
    }
    for w in points.windows(2) {
        if temp <= w[1].temp_c {
            let t = (temp - w[0].temp_c) / (w[1].temp_c - w[0].temp_c);
            return w[0].speed_pct + t * (w[1].speed_pct - w[0].speed_pct);
        }
    }
    last.speed_pct
}

/// `n` evenly spaced points along a curve, for drivers with a fixed number.
pub fn resample(points: &[CurvePoint], n: usize) -> Vec<CurvePoint> {
    let (Some(first), Some(last)) = (points.first(), points.last()) else { return Vec::new() };
    (0..n)
        .map(|i| {
            let temp = first.temp_c + (last.temp_c - first.temp_c) * i as f64 / (n.max(2) - 1) as f64;
            CurvePoint { temp_c: temp.round(), speed_pct: speed_at(points, temp).round() }
        })
        .collect()
}

/// A curve as text, one `temperature speed` pair per line, for import and export.
pub fn export_curve(points: &[CurvePoint]) -> String {
    points.iter().map(|p| format!("{} {}\n", p.temp_c, p.speed_pct)).collect()
}

pub fn import_curve(text: &str) -> Result<Vec<CurvePoint>> {
    let mut points = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let mut parts = line.split([' ', ',', '\t', ';']).filter(|p| !p.is_empty());
        let (Some(t), Some(s)) = (parts.next(), parts.next()) else { bail!("{}", trf!("Can't read the curve line “{}”.", line)) };
        let (Ok(temp_c), Ok(speed_pct)) = (t.parse(), s.parse()) else { bail!("{}", trf!("Can't read the curve line “{}”.", line)) };
        points.push(CurvePoint { temp_c, speed_pct });
    }
    validate_curve(&points)?;
    Ok(points)
}

/// Decides fan speeds for cards without a built-in curve. It rises quickly,
/// falls slowly, and ignores small temperature swings, so the fan doesn't
/// hunt up and down. With no reading it runs the fan flat out.
pub struct Governor {
    points: Vec<CurvePoint>,
    hysteresis_c: f64,
    max_rise_pct: f64,
    max_fall_pct: f64,
    anchor_c: Option<f64>,
    speed: f64,
}

impl Governor {
    pub fn new(points: Vec<CurvePoint>, start_speed: f64) -> Self {
        Governor { points, hysteresis_c: 3.0, max_rise_pct: 15.0, max_fall_pct: 3.0, anchor_c: None, speed: start_speed }
    }

    /// The speed for the next tick, given the latest temperature.
    pub fn update(&mut self, temp: Option<f64>) -> f64 {
        let target = match temp {
            None => 100.0,
            Some(t) => {
                // Follow the temperature up at once, but down only once it has
                // fallen by the hysteresis below the temperature that set the speed.
                let anchor = match self.anchor_c {
                    Some(a) if t > a || a - t < self.hysteresis_c => a.max(t),
                    _ => t,
                };
                self.anchor_c = Some(anchor);
                speed_at(&self.points, anchor)
            }
        };
        let step = if target > self.speed { (target - self.speed).min(self.max_rise_pct) } else { -(self.speed - target).min(self.max_fall_pct) };
        self.speed = (self.speed + step).clamp(0.0, 100.0);
        self.speed
    }
}

// ---------------------------------------------------------------- Plan

#[derive(Clone, Debug, PartialEq)]
pub enum NvmlOp {
    PowerLimit { address: String, watts: f64 },
    FanSpeed { address: String, fans: u32, percent: u32 },
    FanAuto { address: String, fans: u32 },
    ClockOffset { address: String, memory: bool, mhz: i32 },
    Threshold { address: String, celsius: i32 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Write { path: PathBuf, value: String },
    Nvml(NvmlOp),
}

/// One change, with what undoes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub label: String,
    pub op: Op,
    pub undo: Op,
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub steps: Vec<Step>,
    /// A fan curve Voltkit has to run itself, because the card has no built-in one.
    pub software_curve: Option<Vec<CurvePoint>>,
}

impl Plan {
    /// The preview: what will change.
    pub fn preview(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.steps.iter().map(|s| s.label.clone()).collect();
        if self.software_curve.is_some() {
            lines.push(t("Voltkit's GPU service will follow the fan curve."));
        }
        lines
    }
}

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn write_step(label: String, path: PathBuf, value: String, undo_value: String) -> Step {
    Step { label, op: Op::Write { path: path.clone(), value }, undo: Op::Write { path, value: undo_value } }
}

/// Parses `MIN`/`MAX` style ranges from an overdrive table line such as
/// `SCLK_OFFSET:    -500Mhz       1000Mhz`.
fn od_range(table: &str, key: &str) -> Option<(f64, f64)> {
    let line = table.lines().find(|l| l.trim_start().starts_with(key))?;
    let nums: Vec<f64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|w| w.trim_end_matches(|c: char| c.is_ascii_alphabetic()).parse().ok())
        .collect();
    (nums.len() >= 2).then(|| (nums[0], nums[1]))
}

/// The number on the line after `header` (`OD_SCLK_OFFSET:` then `0Mhz`).
fn od_value(table: &str, header: &str) -> Option<f64> {
    let mut lines = table.lines();
    lines.find(|l| l.trim() == header)?;
    lines.next()?.trim().trim_end_matches(|c: char| c.is_ascii_alphabetic()).parse().ok()
}

/// The clock on an indexed line (`1: 1250Mhz`) under `header`.
fn od_indexed(table: &str, header: &str, index: u32) -> Option<f64> {
    let mut lines = table.lines();
    lines.find(|l| l.trim() == header)?;
    lines
        .take_while(|l| !l.trim_end().ends_with(':'))
        .find_map(|l| l.trim().strip_prefix(&format!("{index}:")).and_then(|v| v.trim().trim_end_matches(|c: char| c.is_ascii_alphabetic()).parse().ok()))
}

fn require(card: &Card, control: Control) -> Result<()> {
    match card.capabilities.iter().find(|c| c.control == control).map(|c| &c.support) {
        Some(Support::Detected(_)) => Ok(()),
        Some(Support::Unavailable { reason, how_to }) => {
            let how = how_to.as_deref().map(|h| format!(" {h}")).unwrap_or_default();
            bail!("{}: {reason}{how}", control.title())
        }
        _ => bail!("{}: {}", control.title(), t("this card doesn't offer it")),
    }
}

/// Builds the steps for `settings` on `card`, refusing anything the card
/// doesn't offer or reports outside its range.
pub fn plan(card: &Card, settings: &Settings) -> Result<Plan> {
    let probe = card.probe.as_ref();
    let fan_count = probe.and_then(|p| p.fans.as_ref().ok().map(|f| f.0)).unwrap_or(0);
    let mut plan = Plan::default();
    let s = &card.sample;
    let amd = card.gpu.vendor == Vendor::Amd;
    let nvidia = card.gpu.vendor == Vendor::Nvidia;
    if !amd && !nvidia {
        bail!("{}", t("GPU Tuner covers NVIDIA and AMD cards."));
    }
    let address = card.gpu.address.clone();
    let dev = &card.gpu.device;
    let hwmon = std::fs::read_dir(dev.join("hwmon")).ok().and_then(|d| d.flatten().map(|e| e.path()).min());
    let od = read(&dev.join("pp_od_clk_voltage")).unwrap_or_default();

    if let Some(watts) = settings.power_limit_w {
        require(card, Control::PowerLimit)?;
        let (Some(min), Some(max)) = (s.limit_min_w, s.limit_max_w) else { bail!("{}", t("The card doesn't report its power limit range.")) };
        if watts < min || watts > max {
            bail!("{}", trf!("The power limit must be between {} W and {} W.", format!("{min:.0}"), format!("{max:.0}")));
        }
        let old = s.limit_w.unwrap_or(max);
        let label = trf!("Power limit: {} W (now {} W)", format!("{watts:.0}"), format!("{old:.0}"));
        if amd {
            let path = hwmon.clone().map(|h| h.join("power1_cap")).unwrap_or_default();
            plan.steps.push(write_step(label, path, format!("{}", (watts * 1e6).round() as u64), format!("{}", (old * 1e6).round() as u64)));
        } else {
            plan.steps.push(Step {
                label,
                op: Op::Nvml(NvmlOp::PowerLimit { address: address.clone(), watts }),
                undo: Op::Nvml(NvmlOp::PowerLimit { address: address.clone(), watts: old }),
            });
        }
    }

    if let Some(mode) = &settings.fan {
        require(card, Control::FanSpeed)?;
        match mode {
            FanMode::Auto => {
                let label = t("Fan: the card's own control");
                if amd {
                    let enable = hwmon.clone().map(|h| h.join("pwm1_enable")).unwrap_or_default();
                    let was = read(&enable).unwrap_or_else(|| "2".into());
                    plan.steps.push(write_step(label, enable, "2".into(), was));
                } else {
                    let fans = fan_count.max(1);
                    plan.steps.push(Step {
                        label,
                        op: Op::Nvml(NvmlOp::FanAuto { address: address.clone(), fans }),
                        undo: Op::Nvml(NvmlOp::FanAuto { address: address.clone(), fans }),
                    });
                }
            }
            FanMode::Fixed { speed_pct } => {
                if !(MIN_FIXED_FAN_PCT..=100.0).contains(speed_pct) {
                    bail!("{}", trf!("A fixed fan speed must be between {} % and 100 %.", MIN_FIXED_FAN_PCT));
                }
                let label = trf!("Fan: {} % fixed", format!("{speed_pct:.0}"));
                if amd {
                    let h = hwmon.clone().unwrap_or_default();
                    let (enable, pwm) = (h.join("pwm1_enable"), h.join("pwm1"));
                    let was_enable = read(&enable).unwrap_or_else(|| "2".into());
                    let was_pwm = read(&pwm).unwrap_or_else(|| "128".into());
                    plan.steps.push(write_step(t("Fan: manual control"), enable, "1".into(), was_enable));
                    plan.steps.push(write_step(label, pwm, format!("{}", (speed_pct / 100.0 * 255.0).round() as u32), was_pwm));
                } else {
                    let fans = fan_count.max(1);
                    plan.steps.push(Step {
                        label,
                        op: Op::Nvml(NvmlOp::FanSpeed { address: address.clone(), fans, percent: speed_pct.round() as u32 }),
                        undo: Op::Nvml(NvmlOp::FanAuto { address: address.clone(), fans }),
                    });
                }
            }
            FanMode::Curve { points } => {
                require(card, Control::FanCurve)?;
                validate_curve(points)?;
                let native = dev.join("gpu_od/fan_ctrl/fan_curve");
                if amd && native.exists() {
                    // The driver's own curve: a fixed number of points, committed with "c".
                    let count = read(&native).map(|t| t.lines().filter(|l| l.contains('%') && l.contains(':') && l.contains('C')).count()).unwrap_or(5).max(2);
                    for (i, p) in resample(points, count).iter().enumerate() {
                        plan.steps.push(write_step(
                            trf!("Fan curve point {}: {} °C → {} %", i + 1, p.temp_c, p.speed_pct),
                            native.clone(),
                            format!("{i} {} {}", p.temp_c, p.speed_pct),
                            "r".into(),
                        ));
                    }
                    plan.steps.push(write_step(t("Fan curve: apply"), native.clone(), "c".into(), "c".into()));
                } else {
                    plan.software_curve = Some(points.clone());
                }
            }
        }
    }

    if let Some(target) = settings.temperature_target_c {
        require(card, Control::TemperatureTarget)?;
        if !(40.0..=100.0).contains(&target) {
            bail!("{}", t("The temperature target must be between 40 and 100 °C."));
        }
        let label = trf!("Temperature target: {} °C", format!("{target:.0}"));
        if amd {
            let file = dev.join("gpu_od/fan_ctrl/fan_target_temperature");
            let was = od_value(&read(&file).unwrap_or_default(), "OD_FAN_TARGET_TEMPERATURE:").map_or("r".to_owned(), |v| format!("{v:.0}"));
            plan.steps.push(write_step(label, file.clone(), format!("{target:.0}"), was));
            plan.steps.push(write_step(t("Temperature target: apply"), file, "c".into(), "c".into()));
        } else {
            let was = probe.and_then(|p| p.target.as_ref().ok().copied()).map_or(83, |v| v as i32);
            plan.steps.push(Step {
                label,
                op: Op::Nvml(NvmlOp::Threshold { address: address.clone(), celsius: target.round() as i32 }),
                undo: Op::Nvml(NvmlOp::Threshold { address: address.clone(), celsius: was }),
            });
        }
    }

    let wants_clocks = settings.core_offset_mhz.is_some() || settings.mem_offset_mhz.is_some();
    if wants_clocks {
        require(card, Control::ClockOffsets)?;
        if nvidia {
            for (memory, value, current) in [
                (false, settings.core_offset_mhz, probe.and_then(|p| p.core_offset.as_ref().ok().copied())),
                (true, settings.mem_offset_mhz, probe.and_then(|p| p.mem_offset.as_ref().ok().copied())),
            ] {
                let Some(mhz) = value else { continue };
                if mhz.abs() > 1000 {
                    bail!("{}", t("Clock offsets must be between -1000 and 1000 MHz."));
                }
                let name = if memory { t("Memory clock offset") } else { t("Core clock offset") };
                plan.steps.push(Step {
                    label: trf!("{}: {} MHz", name, mhz),
                    op: Op::Nvml(NvmlOp::ClockOffset { address: address.clone(), memory, mhz }),
                    undo: Op::Nvml(NvmlOp::ClockOffset { address: address.clone(), memory, mhz: current.unwrap_or(0) }),
                });
            }
        } else {
            let file = dev.join("pp_od_clk_voltage");
            if let Some(mhz) = settings.core_offset_mhz {
                if let Some((lo, hi)) = od_range(&od, "SCLK_OFFSET:") {
                    // Cards that take a clock offset directly.
                    if f64::from(mhz) < lo || f64::from(mhz) > hi {
                        bail!("{}", trf!("The core clock offset must be between {} and {} MHz.", lo, hi));
                    }
                    let was = od_value(&od, "OD_SCLK_OFFSET:").unwrap_or(0.0);
                    plan.steps.push(write_step(trf!("Core clock offset: {} MHz", mhz), file.clone(), format!("s 0 {mhz}"), format!("s 0 {was:.0}")));
                } else if let (Some((_, hi)), Some(max)) = (od_range(&od, "SCLK:"), od_indexed(&od, "OD_SCLK:", 1)) {
                    // Cards that take a maximum clock.
                    let new = max + f64::from(mhz);
                    if new > hi {
                        bail!("{}", trf!("The card's highest allowed core clock is {} MHz.", hi));
                    }
                    plan.steps.push(write_step(trf!("Core clock offset: {} MHz", mhz), file.clone(), format!("s 1 {new:.0}"), format!("s 1 {max:.0}")));
                } else {
                    bail!("{}", t("Can't read this card's core clock limits."));
                }
            }
            if let Some(mhz) = settings.mem_offset_mhz {
                let (Some((_, hi)), Some(max)) = (od_range(&od, "MCLK:"), od_indexed(&od, "OD_MCLK:", 1)) else {
                    bail!("{}", t("Can't read this card's memory clock limits."));
                };
                let new = max + f64::from(mhz);
                if new > hi {
                    bail!("{}", trf!("The card's highest allowed memory clock is {} MHz.", hi));
                }
                plan.steps.push(write_step(trf!("Memory clock offset: {} MHz", mhz), file.clone(), format!("m 1 {new:.0}"), format!("m 1 {max:.0}")));
            }
        }
    }

    if let Some(mv) = settings.voltage_offset_mv {
        require(card, Control::VoltageCurve)?;
        let file = dev.join("pp_od_clk_voltage");
        let (lo, hi) = od_range(&od, "VDDGFX_OFFSET:").unwrap_or((-200.0, 0.0));
        if f64::from(mv) < lo || f64::from(mv) > hi {
            bail!("{}", trf!("The voltage offset must be between {} and {} mV.", lo, hi));
        }
        let was = od_value(&od, "OD_VDDGFX_OFFSET:").unwrap_or(0.0);
        plan.steps.push(write_step(trf!("Voltage offset: {} mV", mv), file, format!("vo {mv}"), format!("vo {was:.0}")));
    }

    // Overdrive tables take their values only after a commit.
    if amd && plan.steps.iter().any(|s| matches!(&s.op, Op::Write { path, .. } if path.ends_with("pp_od_clk_voltage"))) {
        let file = dev.join("pp_od_clk_voltage");
        plan.steps.push(write_step(t("Clocks and voltage: apply"), file, "c".into(), "c".into()));
    }
    Ok(plan)
}

// ---------------------------------------------------------------- Applying

pub trait Hardware {
    fn run(&mut self, op: &Op) -> Result<()>;
}

/// The real thing: sysfs writes and NVML calls. Needs administrator rights.
pub struct Real {
    pub nvml: Option<Nvml>,
}

impl Hardware for Real {
    fn run(&mut self, op: &Op) -> Result<()> {
        match op {
            Op::Write { path, value } => {
                std::fs::write(path, format!("{value}\n")).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
            }
            Op::Nvml(call) => {
                let nvml = self.nvml.as_ref().ok_or_else(|| anyhow::anyhow!("{}", super::nvml::NvmlError::Missing))?;
                match call {
                    NvmlOp::PowerLimit { address, watts } => nvml.set_power_limit(address, *watts),
                    NvmlOp::FanSpeed { address, fans, percent } => (0..*fans).try_for_each(|f| nvml.set_fan_speed(address, f, *percent)),
                    NvmlOp::FanAuto { address, fans } => (0..*fans).try_for_each(|f| nvml.set_fan_auto(address, f)),
                    NvmlOp::ClockOffset { address, memory, mhz } => nvml.set_clock_offset(address, *memory, *mhz),
                    NvmlOp::Threshold { address, celsius } => nvml.set_threshold(address, Threshold::AcousticTarget, *celsius),
                }
                .map_err(Into::into)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Applied,
    /// A step failed; the earlier ones were undone.
    RolledBack { failed: String, error: String },
    /// A step failed and so did undoing an earlier one: the card may be in a
    /// mixed state and the user must be told.
    Mixed { failed: String, error: String, stuck: Vec<String> },
}

/// Runs the steps in order; on a failure, undoes the finished ones in reverse.
pub fn execute(steps: &[Step], hw: &mut dyn Hardware) -> Outcome {
    for (i, step) in steps.iter().enumerate() {
        if let Err(e) = hw.run(&step.op) {
            let stuck = undo(&steps[..i], hw);
            let (failed, error) = (step.label.clone(), e.to_string());
            return if stuck.is_empty() { Outcome::RolledBack { failed, error } } else { Outcome::Mixed { failed, error, stuck } };
        }
    }
    Outcome::Applied
}

/// Undoes steps in reverse; returns the labels of any that couldn't be undone.
pub fn undo(steps: &[Step], hw: &mut dyn Hardware) -> Vec<String> {
    steps.iter().rev().filter(|s| hw.run(&s.undo).is_err()).map(|s| s.label.clone()).collect()
}

/// The conversation with the privileged helper, which the app starts through
/// `pkexec`. It reads one line of settings, answers with its preview and
/// "applied", then waits for "keep". Anything else, or the app closing or
/// crashing, undoes the change: an unconfirmed setting never stays.
pub fn session(
    card: &Card,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    hw: &mut dyn Hardware,
) -> Result<()> {
    let mut line = String::new();
    input.read_line(&mut line)?;
    let settings: Settings = serde_json::from_str(line.trim())?;
    let plan = plan(card, &settings)?;
    for l in plan.preview() {
        writeln!(out, "step {l}")?;
    }
    match execute(&plan.steps, hw) {
        Outcome::Applied => writeln!(out, "applied")?,
        Outcome::RolledBack { failed, error } => bail!("{}", trf!("{} failed: {}. Earlier changes were undone.", failed, error)),
        Outcome::Mixed { failed, error, stuck } => {
            bail!("{}", trf!("{} failed: {}. Couldn't undo: {}. Restart the computer to reset the card.", failed, error, stuck.join(", ")))
        }
    }
    out.flush()?;
    let mut answer = String::new();
    let kept = input.read_line(&mut answer).is_ok() && answer.trim() == "keep";
    if kept {
        writeln!(out, "kept")?;
    } else {
        let stuck = undo(&plan.steps, hw);
        writeln!(out, "{}", if stuck.is_empty() { "reverted".to_owned() } else { format!("stuck {}", stuck.join(", ")) })?;
    }
    out.flush()?;
    Ok(())
}

// ---------------------------------------------------------------- Startup guard

/// Stops a startup profile that crashes the computer from applying again and
/// again: it leaves a marker before applying and removes it once the system
/// has stayed up. If the marker is still there at the next start, the profile
/// is skipped and disabled.
pub struct BootGuard {
    dir: PathBuf,
}

impl BootGuard {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        BootGuard { dir: dir.into() }
    }

    fn marker(&self) -> PathBuf {
        self.dir.join("applying")
    }

    pub fn disabled_marker(&self) -> PathBuf {
        self.dir.join("disabled")
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled_marker().exists()
    }

    /// True when the profile may be applied now.
    pub fn begin(&self) -> bool {
        let _ = std::fs::create_dir_all(&self.dir);
        if self.is_disabled() {
            return false;
        }
        if self.marker().exists() {
            let _ = std::fs::write(self.disabled_marker(), "the last startup apply didn't finish\n");
            let _ = std::fs::remove_file(self.marker());
            return false;
        }
        std::fs::write(self.marker(), "").is_ok()
    }

    /// The system stayed up after applying.
    pub fn confirm(&self) {
        let _ = std::fs::remove_file(self.marker());
    }

    /// The user looked at the profile again and wants it back on.
    pub fn enable(&self) {
        let _ = std::fs::remove_file(self.disabled_marker());
        let _ = std::fs::remove_file(self.marker());
    }
}

/// Sensor values for the software fan loop; `None` runs the fan at full speed.
pub fn loop_temperature(s: &Sample) -> Option<f64> {
    match (s.temp_c, s.hotspot_c) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::gpu::{Capability, Gpu};

    fn pt(t: f64, s: f64) -> CurvePoint {
        CurvePoint { temp_c: t, speed_pct: s }
    }

    fn curve() -> Vec<CurvePoint> {
        vec![pt(30.0, 20.0), pt(60.0, 40.0), pt(80.0, 70.0), pt(90.0, 100.0)]
    }

    #[test]
    fn curve_interpolates_and_clamps() {
        let c = curve();
        assert_eq!(speed_at(&c, 20.0), 20.0);
        assert_eq!(speed_at(&c, 45.0), 30.0);
        assert_eq!(speed_at(&c, 70.0), 55.0);
        assert_eq!(speed_at(&c, 120.0), 100.0);
    }

    #[test]
    fn rejects_unsafe_or_malformed_curves() {
        assert!(validate_curve(&curve()).is_ok());
        assert!(validate_curve(&[pt(30.0, 20.0)]).is_err());
        assert!(validate_curve(&[pt(60.0, 40.0), pt(30.0, 20.0)]).is_err());
        assert!(validate_curve(&[pt(30.0, 50.0), pt(60.0, 30.0), pt(90.0, 100.0)]).is_err());
        // Too slow at 85 °C: refuses to let the card cook.
        assert!(validate_curve(&[pt(30.0, 20.0), pt(90.0, 50.0)]).is_err());
        assert!(validate_curve(&[pt(30.0, 20.0), pt(120.0, 100.0)]).is_err());
    }

    #[test]
    fn curves_round_trip_through_text() {
        let text = export_curve(&curve());
        assert_eq!(import_curve(&text).unwrap(), curve());
        assert_eq!(import_curve("# my curve\n30,20\n60;40\n85 70\n90\t100\n").unwrap().len(), 4);
        assert!(import_curve("hot cold").is_err());
    }

    #[test]
    fn resampling_keeps_the_shape() {
        let r = resample(&curve(), 5);
        assert_eq!(r.len(), 5);
        assert_eq!((r[0].temp_c, r[4].temp_c), (30.0, 90.0));
        assert!(r.windows(2).all(|w| w[1].speed_pct >= w[0].speed_pct));
    }

    #[test]
    fn the_fan_ignores_small_temperature_swings() {
        let mut g = Governor::new(curve(), 40.0);
        let mut speeds = Vec::new();
        for t in [60.0, 61.0, 60.0, 61.5, 59.0, 60.5, 60.0, 61.0, 59.5, 60.0] {
            speeds.push(g.update(Some(t)));
        }
        let settled = &speeds[3..];
        let spread = settled.iter().cloned().fold(f64::MIN, f64::max) - settled.iter().cloned().fold(f64::MAX, f64::min);
        assert!(spread < 1.0, "{speeds:?}");
    }

    #[test]
    fn the_fan_rises_fast_and_falls_slowly() {
        let mut g = Governor::new(curve(), 20.0);
        assert!(g.update(Some(85.0)) - 20.0 >= 15.0 - 1e-9);
        let high = (0..10).map(|_| g.update(Some(85.0))).last().unwrap();
        let drop = high - g.update(Some(40.0));
        assert!(drop > 0.0 && drop <= 3.0 + 1e-9, "{drop}");
    }

    #[test]
    fn the_fan_runs_flat_out_without_a_reading() {
        let mut g = Governor::new(curve(), 30.0);
        for _ in 0..10 {
            g.update(None);
        }
        assert_eq!(g.update(None), 100.0);
    }

    // ---- Planning and applying on a fake AMD card

    struct Tree(PathBuf);

    impl Tree {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("voltkit-tune-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Tree(dir)
        }

        fn put(&self, path: &str, content: &str) {
            let p = self.0.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        }

        fn get(&self, path: &str) -> String {
            std::fs::read_to_string(self.0.join(path)).unwrap().trim().to_owned()
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const OD: &str = "OD_SCLK:\n0: 500Mhz\n1: 2500Mhz\nOD_MCLK:\n0: 97Mhz\n1: 1000Mhz\nOD_VDDGFX_OFFSET:\n0mV\nOD_RANGE:\nSCLK:     500Mhz      2800Mhz\nMCLK:      97Mhz      1200Mhz\nVDDGFX_OFFSET: -200mV 0mV\n";

    fn amd_card(tree: &Tree) -> Card {
        tree.put("hwmon/hwmon1/power1_cap", "200000000");
        tree.put("hwmon/hwmon1/pwm1", "100");
        tree.put("hwmon/hwmon1/pwm1_enable", "2");
        tree.put("pp_od_clk_voltage", OD);
        tree.put("gpu_od/fan_ctrl/fan_target_temperature", "OD_FAN_TARGET_TEMPERATURE:\n75\n");
        let detected = |note: &str| Support::Detected(note.into());
        Card {
            gpu: Gpu { address: "0000:03:00.0".into(), vendor: Vendor::Amd, name: "Test".into(), driver: Some("amdgpu".into()), device: tree.0.clone() },
            sample: Sample { limit_w: Some(200.0), limit_min_w: Some(100.0), limit_max_w: Some(300.0), ..Sample::default() },
            capabilities: Control::ALL.into_iter().map(|control| Capability { control, support: detected("ok") }).collect(),
            probe: None,
        }
    }

    fn settings() -> Settings {
        Settings { gpu: "0000:03:00.0".into(), ..Settings::default() }
    }

    #[test]
    fn plans_and_applies_power_fan_and_clock_changes() {
        let tree = Tree::new("apply");
        let card = amd_card(&tree);
        let s = Settings {
            power_limit_w: Some(250.0),
            fan: Some(FanMode::Fixed { speed_pct: 50.0 }),
            core_offset_mhz: Some(100),
            voltage_offset_mv: Some(-50),
            ..settings()
        };
        let plan = plan(&card, &s).unwrap();
        assert!(plan.preview().iter().any(|l| l.contains("250")));
        let mut hw = Real { nvml: None };
        assert_eq!(execute(&plan.steps, &mut hw), Outcome::Applied);
        assert_eq!(tree.get("hwmon/hwmon1/power1_cap"), "250000000");
        assert_eq!(tree.get("hwmon/hwmon1/pwm1_enable"), "1");
        assert_eq!(tree.get("hwmon/hwmon1/pwm1"), "128");
        // The overdrive file ends with the commit.
        assert_eq!(tree.get("pp_od_clk_voltage"), "c");
        assert!(undo(&plan.steps, &mut hw).is_empty());
        assert_eq!(tree.get("hwmon/hwmon1/power1_cap"), "200000000");
        assert_eq!(tree.get("hwmon/hwmon1/pwm1_enable"), "2");
        assert_eq!(tree.get("hwmon/hwmon1/pwm1"), "100");
    }

    #[test]
    fn clock_offsets_become_maximum_clocks_within_the_cards_range() {
        let tree = Tree::new("clocks");
        let card = amd_card(&tree);
        let p = plan(&card, &Settings { core_offset_mhz: Some(200), mem_offset_mhz: Some(100), ..settings() }).unwrap();
        let writes: Vec<String> = p.steps.iter().filter_map(|s| if let Op::Write { value, .. } = &s.op { Some(value.clone()) } else { None }).collect();
        assert!(writes.contains(&"s 1 2700".to_owned()), "{writes:?}");
        assert!(writes.contains(&"m 1 1100".to_owned()), "{writes:?}");
        assert!(plan(&card, &Settings { core_offset_mhz: Some(400), ..settings() }).is_err());
    }

    #[test]
    fn refuses_values_outside_what_the_card_reports() {
        let tree = Tree::new("range");
        let card = amd_card(&tree);
        for s in [
            Settings { power_limit_w: Some(400.0), ..settings() },
            Settings { power_limit_w: Some(50.0), ..settings() },
            Settings { fan: Some(FanMode::Fixed { speed_pct: 5.0 }), ..settings() },
            Settings { voltage_offset_mv: Some(50), ..settings() },
            Settings { temperature_target_c: Some(10.0), ..settings() },
        ] {
            assert!(plan(&card, &s).is_err(), "{s:?}");
        }
    }

    #[test]
    fn refuses_controls_the_card_doesnt_offer() {
        let tree = Tree::new("unavailable");
        let mut card = amd_card(&tree);
        card.capabilities.iter_mut().find(|c| c.control == Control::PowerLimit).unwrap().support =
            Support::Unavailable { reason: "fixed by the vendor".into(), how_to: None };
        let err = plan(&card, &Settings { power_limit_w: Some(250.0), ..settings() }).unwrap_err();
        assert!(err.to_string().contains("fixed by the vendor"));
    }

    /// Fails on the nth operation, recording what ran.
    struct Flaky {
        ran: Vec<Op>,
        fail_at: usize,
    }

    impl Hardware for Flaky {
        fn run(&mut self, op: &Op) -> Result<()> {
            self.ran.push(op.clone());
            if self.ran.len() == self.fail_at { bail!("driver said no") } else { Ok(()) }
        }
    }

    fn three_steps() -> Vec<Step> {
        (1..=3)
            .map(|i| {
                let p = |v: &str| Op::Write { path: PathBuf::from(format!("/x/{i}")), value: v.into() };
                Step { label: format!("step {i}"), op: p("new"), undo: p("old") }
            })
            .collect()
    }

    #[test]
    fn a_failed_step_undoes_the_earlier_ones_in_reverse() {
        let mut hw = Flaky { ran: Vec::new(), fail_at: 3 };
        let out = execute(&three_steps(), &mut hw);
        assert_eq!(out, Outcome::RolledBack { failed: "step 3".into(), error: "driver said no".into() });
        let values: Vec<_> = hw.ran.iter().map(|op| if let Op::Write { path, value } = op { format!("{}={value}", path.display()) } else { String::new() }).collect();
        assert_eq!(values, ["/x/1=new", "/x/2=new", "/x/3=new", "/x/2=old", "/x/1=old"]);
    }

    #[test]
    fn says_so_when_an_undo_fails_too() {
        let mut hw = Flaky { ran: Vec::new(), fail_at: 3 };
        // The undo of step 2 is the fourth call; make it fail as well.
        struct Twice(Flaky);
        impl Hardware for Twice {
            fn run(&mut self, op: &Op) -> Result<()> {
                let n = self.0.ran.len() + 1;
                self.0.ran.push(op.clone());
                if n == 3 || n == 4 { bail!("nope") } else { Ok(()) }
            }
        }
        let _ = &mut hw;
        let mut hw = Twice(Flaky { ran: Vec::new(), fail_at: 0 });
        match execute(&three_steps(), &mut hw) {
            Outcome::Mixed { failed, stuck, .. } => {
                assert_eq!(failed, "step 3");
                assert_eq!(stuck, ["step 2"]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_unconfirmed_change_is_reverted() {
        let tree = Tree::new("session");
        let card = amd_card(&tree);
        let request = serde_json::to_string(&Settings { power_limit_w: Some(250.0), ..settings() }).unwrap();
        for (reply, expect, cap) in [("keep\n", "kept", "250000000"), ("\n", "reverted", "200000000"), ("", "reverted", "200000000")] {
            tree.put("hwmon/hwmon1/power1_cap", "200000000");
            let mut input = std::io::Cursor::new(format!("{request}\n{reply}"));
            let mut out = Vec::new();
            session(&card, &mut input, &mut out, &mut Real { nvml: None }).unwrap();
            let text = String::from_utf8(out).unwrap();
            assert!(text.contains("applied") && text.trim_end().ends_with(expect), "{text}");
            assert_eq!(tree.get("hwmon/hwmon1/power1_cap"), cap, "{reply:?}");
        }
    }

    #[test]
    fn a_startup_profile_that_never_finished_is_skipped_next_time() {
        let tree = Tree::new("guard");
        let guard = BootGuard::new(tree.0.join("state"));
        assert!(guard.begin());
        guard.confirm();
        assert!(guard.begin(), "a clean start applies again");
        // The computer froze before confirm(): the next start must not apply.
        assert!(!guard.begin());
        assert!(guard.is_disabled());
        assert!(!guard.begin());
        guard.enable();
        assert!(guard.begin());
    }

    #[test]
    fn nvidia_changes_go_through_nvml_and_undo_to_the_old_values() {
        let tree = Tree::new("nvidia");
        let mut card = amd_card(&tree);
        card.gpu.vendor = Vendor::Nvidia;
        card.sample.limit_w = Some(150.0);
        let probe = crate::modules::gpu::NvidiaProbe {
            power: Ok((100.0, 240.0)),
            fans: Ok((2, (30, 100))),
            core_offset: Ok(25),
            mem_offset: Ok(0),
            target: Ok(83),
        };
        let s = Settings {
            power_limit_w: Some(200.0),
            fan: Some(FanMode::Fixed { speed_pct: 60.0 }),
            core_offset_mhz: Some(100),
            temperature_target_c: Some(75.0),
            ..settings()
        };
        card.probe = Some(probe);
        let p = plan(&card, &s).unwrap();
        let ops: Vec<&Op> = p.steps.iter().map(|s| &s.op).collect();
        assert!(ops.iter().all(|o| matches!(o, Op::Nvml(_))));
        let undo_core = p.steps.iter().find_map(|s| match &s.undo {
            Op::Nvml(NvmlOp::ClockOffset { mhz, memory: false, .. }) => Some(*mhz),
            _ => None,
        });
        assert_eq!(undo_core, Some(25));
        assert!(p.steps.iter().any(|s| s.undo == Op::Nvml(NvmlOp::PowerLimit { address: "0000:03:00.0".into(), watts: 150.0 })));
        assert!(p.steps.iter().any(|s| s.undo == Op::Nvml(NvmlOp::Threshold { address: "0000:03:00.0".into(), celsius: 83 })));
    }
}
