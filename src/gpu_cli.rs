//! GPU Tuner on the command line, plus the root helper and background service
//! the app uses to change settings and to keep fan curves running.

use std::io::{self, BufRead};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use crate::i18n::tr;
use crate::modules::gpu::{self, Card, Vendor};
use crate::modules::gpu_session;
use crate::modules::gpu_profiles::{self, CardProfiles, Context as SwitchContext, Profile, Switcher};
use crate::modules::gpu_tune::{self, Governor, Hardware, NvmlOp, Op, Real, Sensor, Settings, Step};
use crate::modules::nvml::Nvml;

pub const PROFILE_PATH: &str = "/etc/voltkit/gpu.json";
pub const PROFILES_PATH: &str = "/etc/voltkit/gpu-profiles.json";
/// Written by the service, readable by everyone: which profile each card is on.
pub const ACTIVE_DIR: &str = "/run/voltkit";
pub const ACTIVE_PATH: &str = "/run/voltkit/gpu-active.json";
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

impl Applied {
    fn none() -> Self {
        Applied { steps: Vec::new(), running: Vec::new() }
    }
}

/// Applies one card's settings; fan curves Voltkit follows itself are returned to run.
fn start_one(settings: &Settings, cards: &[Card], hw: &mut dyn Hardware) -> Applied {
    let mut applied = Applied::none();
    let Ok(card) = find(cards, &settings.gpu) else {
        eprintln!("{} {}", tr("Skipped, card not found:"), settings.gpu);
        return applied;
    };
    let plan = match gpu_tune::plan(card, settings) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {e}", settings.gpu);
            return applied;
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
    applied
}

fn stop(applied: Applied, hw: &mut dyn Hardware) {
    for r in &applied.running {
        r.fan.auto(hw);
    }
    gpu_tune::undo(&applied.steps, hw);
}

/// The profile sets the service runs: the profiles file, or, for settings saved
/// before profiles existed, one profile called "Saved" per card.
pub fn load_sets(profiles: &Path, legacy: &Path) -> Vec<CardProfiles> {
    if let Some(sets) = std::fs::read_to_string(profiles).ok().and_then(|t| serde_json::from_str::<Vec<CardProfiles>>(&t).ok()) {
        return sets.into_iter().filter(|s| s.check().is_ok()).collect();
    }
    load_profile(legacy)
        .into_iter()
        .map(|settings| CardProfiles {
            gpu: settings.gpu.clone(),
            profiles: vec![Profile { name: "Saved".into(), settings }],
            rules: Vec::new(),
            default: Some("Saved".into()),
        })
        .collect()
}

struct CardRuntime {
    set: CardProfiles,
    switcher: Switcher,
    active: Option<String>,
    applied: Applied,
}

fn runtimes(sets: Vec<CardProfiles>) -> Vec<CardRuntime> {
    sets.into_iter().map(|set| CardRuntime { set, switcher: Switcher::default(), active: None, applied: Applied::none() }).collect()
}

/// Which profile each card is on, for the app to show.
fn publish_active(runtimes: &[CardRuntime]) {
    let map: std::collections::BTreeMap<&str, &str> =
        runtimes.iter().filter_map(|r| r.active.as_deref().map(|name| (r.set.gpu.as_str(), name))).collect();
    if let Ok(text) = serde_json::to_string(&map) {
        let _ = std::fs::create_dir_all(ACTIVE_DIR);
        let _ = std::fs::write(ACTIVE_PATH, text);
    }
}

/// The root background service: applies saved profiles at start (unless the
/// last start didn't finish), switches between them by the rules, follows
/// software fan curves, and gives the fans back to the card when it stops.
pub async fn service() -> Result<()> {
    need_root()?;
    let guard = gpu_tune::BootGuard::new(STATE_DIR);
    let mut hw = Real { nvml: Nvml::open().ok() };
    let (profiles_path, legacy_path) = (Path::new(PROFILES_PATH), Path::new(PROFILE_PATH));
    let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let mut seen = (mtime(profiles_path), mtime(legacy_path));
    let enabled = guard.begin();
    let mut cards: Vec<CardRuntime> = if enabled {
        runtimes(load_sets(profiles_path, legacy_path))
    } else {
        eprintln!("{}", tr("The last startup apply didn't finish, so the saved GPU settings are off. Open GPU Tuner to turn them back on."));
        Vec::new()
    };
    let confirm_at = Instant::now() + Duration::from_secs(60);
    let began = Instant::now();
    let mut confirmed = false;
    let gpus = gpu::detect();
    let monitor = gpu::Monitor::new(gpus.clone());
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let (mut power, mut apps, mut count) = (None, std::collections::HashSet::new(), 0u32);
    loop {
        tokio::select! {
            _ = tick.tick() => {
                if !confirmed && Instant::now() >= confirm_at {
                    guard.confirm();
                    confirmed = true;
                }
                let now_seen = (mtime(profiles_path), mtime(legacy_path));
                if now_seen != seen && !guard.is_disabled() {
                    seen = now_seen;
                    for c in cards.drain(..) {
                        stop(c.applied, &mut hw);
                    }
                    cards = runtimes(load_sets(profiles_path, legacy_path));
                }
                if cards.is_empty() {
                    continue;
                }
                // Cheap to read often, costlier to scan: the process list only every few seconds.
                if count.is_multiple_of(3) {
                    power = gpu_profiles::power_source(Path::new("/sys"));
                    if cards.iter().any(|c| c.set.rules.iter().any(|r| r.conditions.iter().any(|x| matches!(x, gpu_profiles::Condition::App { .. })))) {
                        apps = gpu_profiles::running_apps(Path::new("/proc"));
                    }
                }
                count = count.wrapping_add(1);
                let samples = monitor.sample();
                let elapsed = began.elapsed().as_secs_f64();
                let mut changed = false;
                for c in &mut cards {
                    let sample = gpus.iter().position(|g| g.address.eq_ignore_ascii_case(&c.set.gpu)).and_then(|i| samples.get(i)).cloned().unwrap_or_default();
                    let ctx = SwitchContext { on_battery: power, apps: apps.clone(), sample };
                    let want = c.switcher.choose(&c.set, &ctx, elapsed);
                    if want != c.active {
                        stop(std::mem::replace(&mut c.applied, Applied::none()), &mut hw);
                        c.active = want.clone();
                        if let Some(profile) = want.as_deref().and_then(|n| c.set.profile(n)) {
                            eprintln!("{} {}: {}", tr("GPU"), c.set.gpu, profile.name);
                            c.applied = start_one(&profile.settings, &gpu::inspect(), &mut hw);
                        }
                        changed = true;
                    }
                }
                if changed {
                    publish_active(&cards);
                }
                for c in &mut cards {
                    for r in &mut c.applied.running {
                        let temp = gpus.iter().position(|g| g.address == r.address).and_then(|i| samples.get(i)).and_then(|s| r.sensor.read(s));
                        let speed = r.governor.update(temp);
                        if (speed - r.last).abs() >= 1.0 && r.fan.set(&mut hw, speed).is_ok() {
                            r.last = speed;
                        }
                    }
                }
            }
            _ = term.recv() => break,
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    for c in cards {
        stop(c.applied, &mut hw);
    }
    let _ = std::fs::remove_file(ACTIVE_PATH);
    Ok(())
}

// ---------------------------------------------------------------- Profiles

/// A rule's condition in words.
pub fn describe_condition(c: &gpu_profiles::Condition) -> String {
    use gpu_profiles::Condition::*;
    match c {
        PowerSource { on_battery: true } => tr("on battery").to_owned(),
        PowerSource { on_battery: false } => tr("on mains power").to_owned(),
        App { names } => crate::i18n::fill(tr("while {} is running"), &[&names.join(", ")]),
        Workload { util_above_pct, for_secs } => crate::i18n::fill(tr("when busier than {} % for {} s"), &[util_above_pct, for_secs]),
        Temperature { above_c } => crate::i18n::fill(tr("when hotter than {} °C"), &[above_c]),
    }
}

pub fn profiles_list() {
    let sets = load_sets(Path::new(PROFILES_PATH), Path::new(PROFILE_PATH));
    if sets.is_empty() {
        println!("{}", tr("No profiles are saved."));
    }
    for set in &sets {
        println!("{} {}", tr("GPU"), set.gpu);
        for p in &set.profiles {
            let default = if set.default.as_deref() == Some(p.name.as_str()) { format!(" ({})", tr("default")) } else { String::new() };
            println!("  {}{default}", p.name);
        }
        for r in &set.rules {
            let when: Vec<String> = r.conditions.iter().map(describe_condition).collect();
            println!("  {} → {}", when.join(&format!(" {} ", tr("and"))), r.profile);
        }
    }
}

pub fn profiles_active() {
    let map: std::collections::BTreeMap<String, String> =
        std::fs::read_to_string(ACTIVE_PATH).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    if map.is_empty() {
        println!("{}", tr("No profile is active. The service may not be running."));
    }
    for (gpu, name) in map {
        println!("{gpu}: {name}");
    }
}

/// Checks and installs profiles, and turns the service on to use them.
pub fn profiles_install(file: &str) -> Result<()> {
    need_root()?;
    let text = if file == "-" {
        let mut t = String::new();
        io::Read::read_to_string(&mut io::stdin(), &mut t)?;
        t
    } else {
        std::fs::read_to_string(file).with_context(|| file.to_owned())?
    };
    let sets: Vec<CardProfiles> = serde_json::from_str(&text).context(tr("Can't read the profiles.").to_owned())?;
    for set in &sets {
        set.check().map_err(|e| anyhow::anyhow!("{}: {e}", set.gpu))?;
    }
    if let Some(dir) = Path::new(PROFILES_PATH).parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(PROFILES_PATH, serde_json::to_string_pretty(&sets)?)?;
    gpu_tune::BootGuard::new(STATE_DIR).enable();
    let _ = std::process::Command::new("systemctl").args(["enable", "--now", UNIT]).status();
    println!("{}", tr("Profiles installed."));
    Ok(())
}

// ---------------------------------------------------------------- Sessions

pub fn sessions_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("voltkit/gpu-sessions"))
}

fn now_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn describe(stats: &gpu_session::Stats) -> Vec<String> {
    let f = |v: Option<f64>, unit: &str| v.map_or_else(|| "—".to_owned(), |v| format!("{v:.1} {unit}"));
    vec![
        format!("{}: {:.0} s", tr("Duration"), stats.seconds),
        format!("{}: {} / {}", tr("Temperature (average / highest)"), f(stats.avg_temp_c, "°C"), f(stats.max_temp_c, "°C")),
        format!("{}: {} / {}", tr("Power (average / highest)"), f(stats.avg_power_w, "W"), f(stats.max_power_w, "W")),
        format!("{}: {}", tr("Energy used"), f(stats.energy_wh, "Wh")),
        format!("{}: {}", tr("Active clock per watt"), f(stats.active_mhz_per_w, "MHz/W")),
    ]
}

/// Records one card's readings and saves them as a session.
pub fn record(seconds: u64, label: &str, gpu: Option<&str>) -> Result<()> {
    let gpus: Vec<gpu::Gpu> = gpu::detect();
    let picked = match gpu {
        Some(address) => gpus.iter().position(|g| g.address.eq_ignore_ascii_case(address)),
        None => gpus.iter().position(|g| matches!(g.vendor, Vendor::Nvidia | Vendor::Amd)).or(if gpus.is_empty() { None } else { Some(0) }),
    }
    .with_context(|| tr("That graphics card wasn't found.").to_owned())?;
    let address = gpus[picked].address.clone();
    let monitor = gpu::Monitor::new(gpus);
    let mut recorder = gpu_session::Recorder::new(label, &address, &now_stamp());
    let began = Instant::now();
    println!("{}", crate::i18n::fill(tr("Recording {} for {} seconds…"), &[&address, &seconds]));
    while began.elapsed() < Duration::from_secs(seconds) {
        if let Some(sample) = monitor.sample().into_iter().nth(picked) {
            recorder.push(began.elapsed().as_secs_f64(), sample);
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    let session = recorder.finish();
    let dir = sessions_dir().context(tr("No data folder").to_owned())?;
    let path = gpu_session::save(&dir, &session)?;
    for line in describe(&gpu_session::stats(&session)) {
        println!("  {line}");
    }
    println!("{} {}", tr("Saved"), path.display());
    Ok(())
}

pub fn sessions() {
    let dir = sessions_dir();
    let files = dir.as_deref().map(gpu_session::list).unwrap_or_default();
    if files.is_empty() {
        println!("{}", tr("No sessions are recorded yet. Run `voltkit gpu record`."));
    }
    for (i, path) in files.iter().enumerate() {
        match gpu_session::load(path) {
            Ok(s) => {
                let st = gpu_session::stats(&s);
                println!("{:>3}  {}  {}  {:.0} s  {}", i + 1, s.started, s.label, st.seconds, path.display());
            }
            Err(_) => println!("{:>3}  {}", i + 1, path.display()),
        }
    }
}

fn resolve(arg: &str) -> Result<PathBuf> {
    if let Ok(n) = arg.parse::<usize>()
        && n >= 1
        && let Some(path) = sessions_dir().as_deref().map(gpu_session::list).and_then(|f| f.into_iter().nth(n - 1))
    {
        return Ok(path);
    }
    Ok(PathBuf::from(arg))
}

/// The comparison as lines of text.
pub fn render_comparison(a: &gpu_session::Session, b: &gpu_session::Session) -> String {
    let (sa, sb) = (gpu_session::stats(a), gpu_session::stats(b));
    let mut out = format!("{} → {}\n", a.label, b.label);
    for l in gpu_session::compare(&sa, &sb) {
        let change = l.change_pct().map_or_else(String::new, |c| format!(" ({c:+.0} %)"));
        let mark = match l.b_wins() {
            Some(true) => "▶",
            Some(false) => "◀",
            None => " ",
        };
        out.push_str(&format!("  {mark} {} ({}): {:.1} → {:.1}{change}\n", tr(l.label), l.unit, l.a, l.b));
    }
    out.push_str(&format!("{}\n", tr("◀ and ▶ mark the better session; active MHz per watt is a proxy for work per watt, not frames.")));
    out
}

pub fn compare(first: &str, second: &str) -> Result<()> {
    let (a, b) = (gpu_session::load(&resolve(first)?)?, gpu_session::load(&resolve(second)?)?);
    print!("{}", render_comparison(&a, &b));
    Ok(())
}
