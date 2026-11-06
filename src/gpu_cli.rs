//! GPU Tuner on the command line, plus the root helper and background service
//! the app uses to change settings and to keep fan curves running.

use std::io::{self, BufRead};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use crate::i18n::tr;
use crate::modules::gpu::{self, Card, Vendor};
use crate::modules::gpu_tune::{self, Governor, Hardware, NvmlOp, Op, Real, Sensor, Settings, Step};
use crate::modules::nvml::Nvml;

pub const PROFILE_PATH: &str = "/etc/voltkit/gpu.json";
pub const STATE_DIR: &str = "/var/lib/voltkit/gpu";
const UNIT: &str = "voltkit-gpu.service";

fn is_root() -> bool {
    std::fs::metadata("/proc/self").map(|m| m.uid() == 0).unwrap_or(false)
}

fn need_root() -> Result<()> {
    if is_root() {
        Ok(())
    } else {
        bail!("{}", tr("Changing GPU settings needs administrator rights. Run it with sudo."))
    }
}

pub fn show() {
    print!("{}", gpu::render_text(&gpu::inspect()));
}

pub fn load_profile(path: &Path) -> Vec<Settings> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_profile(path: &Path, settings: &Settings) -> Result<()> {
    let mut all = load_profile(path);
    all.retain(|s| s.gpu != settings.gpu);
    all.push(settings.clone());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&all)?).context(tr("Can't save the startup profile."))
}

fn find<'a>(cards: &'a [Card], address: &str) -> Result<&'a Card> {
    cards.iter().find(|c| c.gpu.address.eq_ignore_ascii_case(address)).with_context(|| tr("That graphics card wasn't found.").to_owned())
}

/// Applies settings read from `file` (or `-` for stdin) and asks for
/// confirmation on the terminal; without it, the change is reverted.
pub fn apply(file: &str, session: bool, save: bool, yes: bool, confirm_secs: u64) -> Result<()> {
    need_root()?;
    let text = if file == "-" {
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line)?;
        line
    } else {
        std::fs::read_to_string(file).with_context(|| file.to_owned())?
    };
    let settings: Settings = serde_json::from_str(text.trim()).context(tr("Can't read the settings.").to_owned())?;
    let cards = gpu::inspect();
    let card = find(&cards, &settings.gpu)?;
    let mut hw = Real { nvml: Nvml::open().ok() };

    if session {
        // The app's conversation: one JSON line in, "keep" to confirm.
        let stdin = io::stdin();
        let kept = gpu_tune::session(card, &settings, &mut stdin.lock(), &mut io::stdout(), &mut hw)?;
        if kept && save {
            persist(&settings)?;
        }
        return Ok(());
    }

    let plan = gpu_tune::plan(card, &settings)?;
    for line in plan.preview() {
        println!("  {line}");
    }
    if yes {
        let mut reply = io::Cursor::new(b"keep\n".to_vec());
        let kept = gpu_tune::session(card, &settings, &mut reply, &mut io::sink(), &mut hw)?;
        if kept && save {
            persist(&settings)?;
        }
        println!("{}", tr("Applied."));
        return Ok(());
    }
    // Ask on the terminal, and revert if there's no answer in time.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = io::stdin().lock().read_line(&mut line);
        let _ = tx.send(line);
    });
    let timeout = Duration::from_secs(confirm_secs);
    let answer = {
        let mut out = io::stdout();
        let outcome = gpu_tune::execute(&plan.steps, &mut hw);
        match outcome {
            gpu_tune::Outcome::Applied => {}
            gpu_tune::Outcome::RolledBack { failed, error } => bail!("{failed}: {error}"),
            gpu_tune::Outcome::Mixed { failed, error, stuck } => bail!("{failed}: {error} ({})", stuck.join(", ")),
        }
        use std::io::Write;
        writeln!(out, "{}", crate::i18n::fill(tr("Applied. Type keep and press Enter within {} seconds to keep it; otherwise it's undone."), &[&confirm_secs]))?;
        out.flush()?;
        rx.recv_timeout(timeout).unwrap_or_default()
    };
    if answer.trim() == "keep" {
        if save {
            persist(&settings)?;
        }
        println!("{}", tr("Kept."));
    } else {
        let stuck = gpu_tune::undo(&plan.steps, &mut hw);
        if stuck.is_empty() {
            println!("{}", tr("Not confirmed, so the change was undone."));
        } else {
            bail!("{} {}", tr("Couldn't undo:"), stuck.join(", "));
        }
    }
    Ok(())
}

/// Keeps the settings for the next start, and turns the service on, which
/// applies them at boot and runs any software fan curve.
fn persist(settings: &Settings) -> Result<()> {
    save_profile(Path::new(PROFILE_PATH), settings)?;
    gpu_tune::BootGuard::new(STATE_DIR).enable();
    let _ = std::process::Command::new("systemctl").args(["enable", "--now", UNIT]).status();
    let _ = std::process::Command::new("systemctl").args(["restart", UNIT]).status();
    Ok(())
}

/// Removes a card's saved settings and returns its fans to the card.
pub fn forget(address: &str) -> Result<()> {
    need_root()?;
    let path = Path::new(PROFILE_PATH);
    let mut all = load_profile(path);
    all.retain(|s| !s.gpu.eq_ignore_ascii_case(address));
    std::fs::write(path, serde_json::to_string_pretty(&all)?)?;
    let _ = std::process::Command::new("systemctl").args(["restart", UNIT]).status();
    Ok(())
}

// ---------------------------------------------------------------- Service

struct Running {
    address: String,
    sensor: Sensor,
    governor: Governor,
    fan: FanTarget,
    last: f64,
}

enum FanTarget {
    Amd { enable: PathBuf, pwm: PathBuf },
    Nvidia { address: String, fans: Vec<u32> },
}

impl FanTarget {
    fn set(&self, hw: &mut dyn Hardware, percent: f64) -> Result<()> {
        match self {
            FanTarget::Amd { pwm, .. } => hw.run(&Op::Write { path: pwm.clone(), value: format!("{}", (percent / 100.0 * 255.0).round() as u32) }),
            FanTarget::Nvidia { address, fans } => {
                hw.run(&Op::Nvml(NvmlOp::FanSpeed { address: address.clone(), fans: fans.clone(), percent: percent.round() as u32 }))
            }
        }
    }

    fn manual(&self, hw: &mut dyn Hardware) -> Result<()> {
        match self {
            FanTarget::Amd { enable, .. } => hw.run(&Op::Write { path: enable.clone(), value: "1".into() }),
            FanTarget::Nvidia { .. } => Ok(()),
        }
    }

    fn auto(&self, hw: &mut dyn Hardware) {
        let _ = match self {
            FanTarget::Amd { enable, .. } => hw.run(&Op::Write { path: enable.clone(), value: "2".into() }),
            FanTarget::Nvidia { address, fans } => hw.run(&Op::Nvml(NvmlOp::FanAuto { address: address.clone(), fans: fans.clone() })),
        };
    }
}

struct Applied {
    steps: Vec<Step>,
    running: Vec<Running>,
}

fn start(profile: &[Settings], hw: &mut dyn Hardware) -> Applied {
    let cards = gpu::inspect();
    let mut applied = Applied { steps: Vec::new(), running: Vec::new() };
    for settings in profile {
        let Ok(card) = find(&cards, &settings.gpu) else {
            eprintln!("{} {}", tr("Skipped, card not found:"), settings.gpu);
            continue;
        };
        let plan = match gpu_tune::plan(card, settings) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{}: {e}", settings.gpu);
                continue;
            }
        };
        match gpu_tune::execute(&plan.steps, hw) {
            gpu_tune::Outcome::Applied => {
                applied.steps.extend(plan.steps);
                if let Some(curve) = plan.software_curve {
                    let fan = match card.gpu.vendor {
                        Vendor::Amd => {
                            let hwmon = std::fs::read_dir(card.gpu.device.join("hwmon")).ok().and_then(|d| d.flatten().map(|e| e.path()).min());
                            hwmon.map(|h| FanTarget::Amd { enable: h.join("pwm1_enable"), pwm: h.join("pwm1") })
                        }
                        Vendor::Nvidia => {
                            let count = card.probe.as_ref().and_then(|p| p.fans.as_ref().ok().map(|f| f.0)).unwrap_or(1);
                            let fans = if curve.fans.is_empty() { (0..count).collect() } else { curve.fans.clone() };
                            Some(FanTarget::Nvidia { address: card.gpu.address.clone(), fans })
                        }
                        _ => None,
                    };
                    if let Some(fan) = fan
                        && fan.manual(hw).is_ok()
                    {
                        let start = card.sample.fan_pct.unwrap_or(50.0);
                        applied.running.push(Running { address: card.gpu.address.clone(), governor: Governor::new(curve.points.clone(), start), sensor: curve.sensor, fan, last: -1.0 });
                    }
                }
            }
            other => eprintln!("{}: {other:?}", settings.gpu),
        }
    }
    applied
}

fn stop(applied: Applied, hw: &mut dyn Hardware) {
    for r in &applied.running {
        r.fan.auto(hw);
    }
    gpu_tune::undo(&applied.steps, hw);
}

/// The root background service: applies the saved profile at start (unless the
/// last start didn't finish), follows software fan curves, and gives the fans
/// back to the card when it stops.
pub async fn service() -> Result<()> {
    need_root()?;
    let guard = gpu_tune::BootGuard::new(STATE_DIR);
    let mut hw = Real { nvml: Nvml::open().ok() };
    let profile_path = Path::new(PROFILE_PATH);
    let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let mut seen = mtime(profile_path);
    let mut applied = if guard.begin() {
        start(&load_profile(profile_path), &mut hw)
    } else {
        eprintln!("{}", tr("The last startup apply didn't finish, so the saved GPU settings are off. Open GPU Tuner to turn them back on."));
        Applied { steps: Vec::new(), running: Vec::new() }
    };
    let confirm_at = Instant::now() + Duration::from_secs(60);
    let mut confirmed = false;
    let monitor = gpu::Monitor::new(gpu::detect());
    let gpus = gpu::detect();
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            _ = tick.tick() => {
                if !confirmed && Instant::now() >= confirm_at {
                    guard.confirm();
                    confirmed = true;
                }
                let now = mtime(profile_path);
                if now != seen && !guard.is_disabled() {
                    seen = now;
                    stop(std::mem::replace(&mut applied, Applied { steps: Vec::new(), running: Vec::new() }), &mut hw);
                    applied = start(&load_profile(profile_path), &mut hw);
                }
                if applied.running.is_empty() {
                    continue;
                }
                let samples = monitor.sample();
                for r in &mut applied.running {
                    let temp = gpus.iter().position(|g| g.address == r.address).and_then(|i| samples.get(i)).and_then(|s| r.sensor.read(s));
                    let speed = r.governor.update(temp);
                    if (speed - r.last).abs() >= 1.0 && r.fan.set(&mut hw, speed).is_ok() {
                        r.last = speed;
                    }
                }
            }
            _ = term.recv() => break,
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    stop(applied, &mut hw);
    Ok(())
}
