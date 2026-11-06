//! Named GPU profiles and the rules that switch between them by themselves:
//! on battery or on mains, while an application runs, under a sustained load,
//! or when the card runs hot. The choice is made by [`Switcher`], a plain state
//! machine fed with readings, so it is tested without a GPU or a clock.
//!
//! To keep a profile from flapping, a rule switches in at once but a profile
//! stays at least [`MIN_DWELL_SECS`] before another replaces it, a load must
//! last [`Condition::Workload::for_secs`], and a temperature rule lets go only
//! after the card has cooled [`HEAT_HYSTERESIS_C`] below the trigger.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::gpu::Sample;
use super::gpu_tune::Settings;

pub const MIN_DWELL_SECS: f64 = 20.0;
pub const HEAT_HYSTERESIS_C: f64 = 5.0;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Profile {
    pub name: String,
    /// What to apply to the card.
    pub settings: Settings,
}

/// One thing that has to be true for a rule to apply.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "when", rename_all = "snake_case")]
pub enum Condition {
    /// The computer runs on battery (`true`) or on mains power (`false`).
    PowerSource { on_battery: bool },
    /// Any of these programs is running, by its name (`blender`, `steam`).
    App { names: Vec<String> },
    /// The card has been busier than this for this long.
    Workload { util_above_pct: f64, for_secs: f64 },
    /// The card is hotter than this (until it cools by the hysteresis).
    Temperature { above_c: f64 },
}

/// Use `profile` when all `conditions` hold. Rules are tried in order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Rule {
    pub profile: String,
    pub conditions: Vec<Condition>,
}

/// The profiles and rules of one card.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct CardProfiles {
    /// The card's PCI address.
    pub gpu: String,
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// The profile that applies when no rule does.
    #[serde(default)]
    pub default: Option<String>,
}

impl CardProfiles {
    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Problems that would make the set useless: rules or a default naming a
    /// profile that doesn't exist, duplicate names, empty conditions.
    pub fn check(&self) -> Result<(), String> {
        let mut seen = HashSet::new();
        for p in &self.profiles {
            if p.name.trim().is_empty() {
                return Err("a profile has no name".into());
            }
            if !seen.insert(p.name.as_str()) {
                return Err(format!("two profiles are called {}", p.name));
            }
            if !p.settings.gpu.eq_ignore_ascii_case(&self.gpu) {
                return Err(format!("{} is for another card", p.name));
            }
        }
        for r in &self.rules {
            if self.profile(&r.profile).is_none() {
                return Err(format!("a rule uses {}, which doesn't exist", r.profile));
            }
            if r.conditions.is_empty() {
                return Err(format!("the rule for {} has no condition", r.profile));
            }
        }
        match &self.default {
            Some(d) if self.profile(d).is_none() => Err(format!("the default {d} doesn't exist")),
            _ => Ok(()),
        }
    }
}

/// What the computer is doing right now.
#[derive(Clone, Debug, Default)]
pub struct Context {
    /// `None` when it can't be told (a desktop without a battery reports mains).
    pub on_battery: Option<bool>,
    /// Lowercase names of the running programs.
    pub apps: HashSet<String>,
    pub sample: Sample,
}

/// Decides which profile is active, remembering what it needs between calls.
#[derive(Default)]
pub struct Switcher {
    busy_since: HashMap<(usize, usize), f64>,
    hot: HashSet<(usize, usize)>,
    current: Option<String>,
    since: f64,
}

impl Switcher {
    fn holds(&mut self, key: (usize, usize), c: &Condition, ctx: &Context, now: f64) -> bool {
        match c {
            Condition::PowerSource { on_battery } => ctx.on_battery == Some(*on_battery),
            Condition::App { names } => names.iter().any(|n| ctx.apps.contains(&n.to_lowercase())),
            Condition::Workload { util_above_pct, for_secs } => match ctx.sample.util_pct {
                Some(u) if u > *util_above_pct => *now_since(&mut self.busy_since, key, now) + for_secs <= now,
                _ => {
                    self.busy_since.remove(&key);
                    false
                }
            },
            Condition::Temperature { above_c } => {
                let Some(t) = ctx.sample.temp_c.into_iter().chain(ctx.sample.hotspot_c).reduce(f64::max) else {
                    self.hot.remove(&key);
                    return false;
                };
                if t >= *above_c {
                    self.hot.insert(key);
                } else if t <= *above_c - HEAT_HYSTERESIS_C {
                    self.hot.remove(&key);
                }
                self.hot.contains(&key)
            }
        }
    }

    /// The profile that should be active at `now` (seconds, any steady clock).
    /// `None` means no profile applies: leave the card as it is.
    pub fn choose(&mut self, set: &CardProfiles, ctx: &Context, now: f64) -> Option<String> {
        let mut wanted: Option<String> = None;
        // Every condition is evaluated each time so timers and latches keep running.
        for (ri, rule) in set.rules.iter().enumerate() {
            let mut all = true;
            for (ci, c) in rule.conditions.iter().enumerate() {
                all &= self.holds((ri, ci), c, ctx, now);
            }
            if all && wanted.is_none() {
                wanted = Some(rule.profile.clone());
            }
        }
        let wanted = wanted.or_else(|| set.default.clone()).filter(|n| set.profile(n).is_some());
        if wanted != self.current {
            // Switch at once the first time; afterwards wait out the minimum stay.
            if self.current.is_none() || now - self.since >= MIN_DWELL_SECS {
                self.current = wanted;
                self.since = now;
            }
        }
        self.current.clone()
    }
}

/// When the condition became true: remembered from the first time it was seen.
fn now_since(map: &mut HashMap<(usize, usize), f64>, key: (usize, usize), now: f64) -> &mut f64 {
    map.entry(key).or_insert(now)
}

// ---------------------------------------------------------------- Editing

/// The single automatic rule the app offers for a profile.
#[derive(Clone, Debug, PartialEq)]
pub enum Auto {
    Never,
    OnBattery,
    OnMains,
    App(Vec<String>),
    Busy(f64),
    Hot(f64),
}

impl CardProfiles {
    /// What switches to `profile` by itself, as far as one simple rule can say.
    pub fn auto_of(&self, profile: &str) -> Auto {
        let Some(rule) = self.rules.iter().find(|r| r.profile == profile && r.conditions.len() == 1) else { return Auto::Never };
        match &rule.conditions[0] {
            Condition::PowerSource { on_battery: true } => Auto::OnBattery,
            Condition::PowerSource { on_battery: false } => Auto::OnMains,
            Condition::App { names } => Auto::App(names.clone()),
            Condition::Workload { util_above_pct, .. } => Auto::Busy(*util_above_pct),
            Condition::Temperature { above_c } => Auto::Hot(*above_c),
        }
    }

    /// Replaces the rule for `profile`. Heat rules go first: a card that is too
    /// hot must cool down whatever else is going on.
    pub fn set_auto(&mut self, profile: &str, auto: Auto) {
        self.rules.retain(|r| !(r.profile == profile && r.conditions.len() == 1));
        let condition = match auto {
            Auto::Never => return,
            Auto::OnBattery => Condition::PowerSource { on_battery: true },
            Auto::OnMains => Condition::PowerSource { on_battery: false },
            Auto::App(names) => Condition::App { names },
            Auto::Busy(util_above_pct) => Condition::Workload { util_above_pct, for_secs: 30.0 },
            Auto::Hot(above_c) => Condition::Temperature { above_c },
        };
        let hot = matches!(condition, Condition::Temperature { .. });
        let rule = Rule { profile: profile.to_owned(), conditions: vec![condition] };
        if hot {
            self.rules.insert(0, rule);
        } else {
            self.rules.push(rule);
        }
    }

    /// Adds or replaces a profile by name.
    pub fn save_profile(&mut self, name: &str, settings: Settings) {
        match self.profiles.iter_mut().find(|p| p.name == name) {
            Some(p) => p.settings = settings,
            None => self.profiles.push(Profile { name: name.to_owned(), settings }),
        }
    }

    /// Removes a profile and every rule or default that pointed at it.
    pub fn remove_profile(&mut self, name: &str) {
        self.profiles.retain(|p| p.name != name);
        self.rules.retain(|r| r.profile != name);
        if self.default.as_deref() == Some(name) {
            self.default = None;
        }
    }
}

// ---------------------------------------------------------------- System

/// `Some(true)` on battery, `Some(false)` on mains, `None` if unknown.
pub fn power_source(sys: &Path) -> Option<bool> {
    let dir = std::fs::read_dir(sys.join("class/power_supply")).ok()?;
    let (mut mains_online, mut discharging, mut known) = (false, false, false);
    for entry in dir.flatten() {
        let p = entry.path();
        let read = |n: &str| std::fs::read_to_string(p.join(n)).ok().map(|s| s.trim().to_owned());
        match read("type").as_deref() {
            Some("Mains") | Some("USB") => {
                known = true;
                mains_online |= read("online").as_deref() == Some("1");
            }
            Some("Battery") => {
                known = true;
                discharging |= read("status").as_deref() == Some("Discharging");
            }
            _ => {}
        }
    }
    known.then_some(discharging && !mains_online)
}

/// Lowercase names of every running program: the process name and, where it
/// can be read, the name of its executable (the process name is cut at 15 letters).
pub fn running_apps(proc_root: &Path) -> HashSet<String> {
    let mut out = HashSet::new();
    let Ok(dir) = std::fs::read_dir(proc_root) else { return out };
    for entry in dir.flatten() {
        if !entry.file_name().to_string_lossy().bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let p = entry.path();
        if let Ok(comm) = std::fs::read_to_string(p.join("comm")) {
            out.insert(comm.trim().to_lowercase());
        }
        if let Some(name) = std::fs::read_link(p.join("exe")).ok().and_then(|e| e.file_name().map(|n| n.to_string_lossy().to_lowercase())) {
            out.insert(name.trim_end_matches(" (deleted)").to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(gpu: &str, watts: f64) -> Settings {
        Settings { gpu: gpu.into(), power_limit_w: Some(watts), ..Settings::default() }
    }

    fn set() -> CardProfiles {
        CardProfiles {
            gpu: "0000:03:00.0".into(),
            profiles: ["Quiet", "Balanced", "Performance", "Cool"]
                .iter()
                .zip([150.0, 200.0, 260.0, 120.0])
                .map(|(n, w)| Profile { name: (*n).into(), settings: settings("0000:03:00.0", w) })
                .collect(),
            rules: vec![
                Rule { profile: "Cool".into(), conditions: vec![Condition::Temperature { above_c: 85.0 }] },
                Rule { profile: "Quiet".into(), conditions: vec![Condition::PowerSource { on_battery: true }] },
                Rule { profile: "Performance".into(), conditions: vec![Condition::App { names: vec!["Blender".into(), "steam".into()] }] },
            ],
            default: Some("Balanced".into()),
        }
    }

    fn ctx(on_battery: Option<bool>, apps: &[&str], util: f64, temp: f64) -> Context {
        Context {
            on_battery,
            apps: apps.iter().map(|a| a.to_string()).collect(),
            sample: Sample { util_pct: Some(util), temp_c: Some(temp), ..Sample::default() },
        }
    }

    #[test]
    fn falls_back_to_the_default_and_checks_the_set() {
        let mut s = Switcher::default();
        assert_eq!(s.choose(&set(), &ctx(Some(false), &[], 10.0, 50.0), 0.0).as_deref(), Some("Balanced"));
        assert!(set().check().is_ok());
        let mut bad = set();
        bad.rules.push(Rule { profile: "Missing".into(), conditions: vec![Condition::PowerSource { on_battery: true }] });
        assert!(bad.check().unwrap_err().contains("Missing"));
        let mut other = set();
        other.profiles[0].settings.gpu = "0000:99:00.0".into();
        assert!(other.check().is_err(), "a profile for another card");
    }

    #[test]
    fn rules_are_tried_in_order_and_apps_match_case_insensitively() {
        let mut s = Switcher::default();
        assert_eq!(s.choose(&set(), &ctx(Some(false), &["blender"], 10.0, 50.0), 0.0).as_deref(), Some("Performance"));
        // The battery rule comes before the app rule, so it wins once the stay is over.
        assert_eq!(s.choose(&set(), &ctx(Some(true), &["blender"], 10.0, 50.0), 30.0).as_deref(), Some("Quiet"));
    }

    #[test]
    fn a_profile_stays_for_a_minimum_time() {
        let mut s = Switcher::default();
        assert_eq!(s.choose(&set(), &ctx(Some(true), &[], 10.0, 50.0), 0.0).as_deref(), Some("Quiet"));
        // Plugged in at 5 s: too soon to switch.
        assert_eq!(s.choose(&set(), &ctx(Some(false), &[], 10.0, 50.0), 5.0).as_deref(), Some("Quiet"));
        assert_eq!(s.choose(&set(), &ctx(Some(false), &[], 10.0, 50.0), 21.0).as_deref(), Some("Balanced"));
    }

    #[test]
    fn heat_switches_in_and_lets_go_only_after_cooling() {
        let mut s = Switcher::default();
        let mut now = 0.0;
        let mut at = |temp: f64| {
            now += 25.0;
            s.choose(&set(), &ctx(Some(false), &[], 10.0, temp), now)
        };
        assert_eq!(at(70.0).as_deref(), Some("Balanced"));
        assert_eq!(at(86.0).as_deref(), Some("Cool"));
        assert_eq!(at(83.0).as_deref(), Some("Cool"), "still within the hysteresis");
        assert_eq!(at(81.0).as_deref(), Some("Cool"), "one degree above the release point");
        assert_eq!(at(80.0).as_deref(), Some("Balanced"), "cooled five degrees");
    }

    #[test]
    fn a_workload_must_last_before_it_counts() {
        let mut set = set();
        set.rules = vec![Rule { profile: "Performance".into(), conditions: vec![Condition::Workload { util_above_pct: 80.0, for_secs: 30.0 }] }];
        let mut s = Switcher::default();
        assert_eq!(s.choose(&set, &ctx(None, &[], 95.0, 50.0), 0.0).as_deref(), Some("Balanced"));
        assert_eq!(s.choose(&set, &ctx(None, &[], 95.0, 50.0), 20.0).as_deref(), Some("Balanced"));
        assert_eq!(s.choose(&set, &ctx(None, &[], 95.0, 50.0), 31.0).as_deref(), Some("Performance"));
        // A dip resets the clock.
        assert_eq!(s.choose(&set, &ctx(None, &[], 20.0, 50.0), 40.0).as_deref(), Some("Performance"), "stays for the minimum time");
        assert_eq!(s.choose(&set, &ctx(None, &[], 20.0, 50.0), 62.0).as_deref(), Some("Balanced"));
        assert_eq!(s.choose(&set, &ctx(None, &[], 95.0, 50.0), 70.0).as_deref(), Some("Balanced"), "busy again, but not for 30 s yet");
    }

    #[test]
    fn all_conditions_of_a_rule_must_hold() {
        let mut set = set();
        set.rules = vec![Rule {
            profile: "Performance".into(),
            conditions: vec![Condition::PowerSource { on_battery: false }, Condition::App { names: vec!["steam".into()] }],
        }];
        let mut s = Switcher::default();
        assert_eq!(s.choose(&set, &ctx(Some(true), &["steam"], 10.0, 50.0), 0.0).as_deref(), Some("Balanced"));
        assert_eq!(s.choose(&set, &ctx(Some(false), &["steam"], 10.0, 50.0), 30.0).as_deref(), Some("Performance"));
    }

    #[test]
    fn no_default_and_no_match_means_leave_the_card_alone() {
        let mut set = set();
        set.default = None;
        assert_eq!(Switcher::default().choose(&set, &ctx(Some(false), &[], 10.0, 50.0), 0.0), None);
    }

    #[test]
    fn one_simple_rule_per_profile_can_be_set_and_read_back() {
        let mut s = CardProfiles { gpu: "g".into(), ..CardProfiles::default() };
        s.save_profile("Quiet", settings("g", 150.0));
        s.save_profile("Cool", settings("g", 120.0));
        s.save_profile("Quiet", settings("g", 140.0));
        assert_eq!(s.profiles.len(), 2, "same name replaces");
        assert_eq!(s.profile("Quiet").unwrap().settings.power_limit_w, Some(140.0));
        s.set_auto("Quiet", Auto::OnBattery);
        s.set_auto("Cool", Auto::Hot(85.0));
        assert_eq!(s.auto_of("Quiet"), Auto::OnBattery);
        assert_eq!(s.auto_of("Cool"), Auto::Hot(85.0));
        assert_eq!(s.rules[0].profile, "Cool", "heat rules come first");
        s.set_auto("Quiet", Auto::App(vec!["steam".into()]));
        assert_eq!(s.rules.len(), 2, "replaced, not added");
        s.set_auto("Quiet", Auto::Never);
        assert_eq!(s.auto_of("Quiet"), Auto::Never);
        s.default = Some("Cool".into());
        s.remove_profile("Cool");
        assert!(s.rules.is_empty() && s.default.is_none() && s.profiles.len() == 1);
        assert!(s.check().is_ok());
    }

    #[test]
    fn profiles_round_trip_through_json() {
        let json = serde_json::to_string(&set()).unwrap();
        let back: CardProfiles = serde_json::from_str(&json).unwrap();
        assert_eq!(back, set());
        assert!(json.contains("\"when\":\"power_source\""));
    }

    fn tree(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("voltkit-prof-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn put(root: &Path, path: &str, text: &str) {
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn reads_the_power_source_from_the_power_supplies() {
        let t = tree("power");
        assert_eq!(power_source(&t), None, "no power supplies listed");
        put(&t, "class/power_supply/AC/type", "Mains");
        put(&t, "class/power_supply/AC/online", "0");
        put(&t, "class/power_supply/BAT0/type", "Battery");
        put(&t, "class/power_supply/BAT0/status", "Discharging");
        assert_eq!(power_source(&t), Some(true));
        put(&t, "class/power_supply/AC/online", "1");
        put(&t, "class/power_supply/BAT0/status", "Charging");
        assert_eq!(power_source(&t), Some(false));
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn lists_running_programs_by_name() {
        let t = tree("proc");
        put(&t, "4242/comm", "Blender\n");
        put(&t, "self/comm", "ignored\n");
        put(&t, "cpuinfo", "not a process");
        let apps = running_apps(&t);
        assert!(apps.contains("blender") && !apps.contains("ignored"));
        // The real /proc always has at least this process.
        assert!(!running_apps(Path::new("/proc")).is_empty());
        let _ = std::fs::remove_dir_all(&t);
    }
}
