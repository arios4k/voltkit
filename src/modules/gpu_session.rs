//! Recorded monitoring sessions and the numbers drawn from them, so two
//! profiles can be compared side by side on temperature, noise, power and
//! efficiency.
//!
//! A session is a CSV file anyone can open in a spreadsheet: a few `# key: value`
//! lines, then one row per second. The efficiency figure is a proxy: *active
//! megahertz per watt*, meaning the core clock weighted by how busy the card
//! was, divided by the power it drew. It says how much clock-work each watt
//! bought, not how many frames. If the file has an `fps` column (merged from a
//! tool such as MangoHud), frames per second and frames per watt are reported too.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::gpu::Sample;

const COLUMNS: [&str; 11] =
    ["t_s", "temp_c", "hotspot_c", "util_pct", "core_mhz", "mem_mhz", "power_w", "fan_pct", "fan_rpm", "vram_mb", "fps"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub t: f64,
    pub sample: Sample,
    pub fps: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Session {
    /// What was being tried: a profile name or a note.
    pub label: String,
    pub gpu: String,
    pub started: String,
    pub rows: Vec<Row>,
}

/// Collects samples with their time since the start.
pub struct Recorder {
    session: Session,
    last_t: f64,
}

impl Recorder {
    pub fn new(label: &str, gpu: &str, started: &str) -> Self {
        Recorder { session: Session { label: label.into(), gpu: gpu.into(), started: started.into(), rows: Vec::new() }, last_t: -1.0 }
    }

    /// Adds a reading taken `t` seconds in. Readings out of order are ignored.
    pub fn push(&mut self, t: f64, sample: Sample) {
        if t > self.last_t {
            self.last_t = t;
            self.session.rows.push(Row { t, sample, fps: None });
        }
    }

    pub fn finish(self) -> Session {
        self.session
    }
}

fn cell(v: Option<f64>) -> String {
    v.map_or_else(String::new, |v| format!("{v}"))
}

pub fn to_csv(s: &Session) -> String {
    let mut out = format!("# label: {}\n# gpu: {}\n# started: {}\n{}\n", s.label, s.gpu, s.started, COLUMNS.join(","));
    for r in &s.rows {
        let x = &r.sample;
        let cells = [
            Some(r.t),
            x.temp_c,
            x.hotspot_c,
            x.util_pct,
            x.core_mhz,
            x.mem_mhz,
            x.power_w,
            x.fan_pct,
            x.fan_rpm,
            x.vram_used_mb,
            r.fps,
        ];
        out.push_str(&cells.map(cell).join(","));
        out.push('\n');
    }
    out
}

pub fn from_csv(text: &str) -> Result<Session> {
    let mut meta: BTreeMap<String, String> = BTreeMap::new();
    let mut header: Option<Vec<String>> = None;
    let mut s = Session::default();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(rest) = line.strip_prefix('#') {
            if let Some((k, v)) = rest.split_once(':') {
                meta.insert(k.trim().to_owned(), v.trim().to_owned());
            }
            continue;
        }
        let cells: Vec<&str> = line.split(',').map(str::trim).collect();
        let Some(h) = &header else {
            header = Some(cells.iter().map(|c| c.to_ascii_lowercase()).collect());
            continue;
        };
        let get = |name: &str| h.iter().position(|c| c == name).and_then(|i| cells.get(i)).and_then(|v| v.parse::<f64>().ok());
        let Some(t) = get("t_s") else { bail!("a row has no time") };
        s.rows.push(Row {
            t,
            sample: Sample {
                temp_c: get("temp_c"),
                hotspot_c: get("hotspot_c"),
                util_pct: get("util_pct"),
                core_mhz: get("core_mhz"),
                mem_mhz: get("mem_mhz"),
                power_w: get("power_w"),
                fan_pct: get("fan_pct"),
                fan_rpm: get("fan_rpm"),
                vram_used_mb: get("vram_mb"),
                ..Sample::default()
            },
            fps: get("fps"),
        });
    }
    if header.is_none() {
        bail!("no header row");
    }
    s.label = meta.remove("label").unwrap_or_default();
    s.gpu = meta.remove("gpu").unwrap_or_default();
    s.started = meta.remove("started").unwrap_or_default();
    Ok(s)
}

pub fn save(dir: &Path, s: &Session) -> Result<std::path::PathBuf> {
    std::fs::create_dir_all(dir)?;
    let name: String = s.label.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
    let stamp: String = s.started.chars().filter(|c| c.is_ascii_digit() || *c == 'T').collect();
    let path = dir.join(format!("{stamp}-{name}.csv"));
    std::fs::write(&path, to_csv(s))?;
    Ok(path)
}

pub fn load(path: &Path) -> Result<Session> {
    from_csv(&std::fs::read_to_string(path).with_context(|| path.display().to_string())?)
}

/// Sessions in a folder, newest first by file name (which starts with the date).
pub fn list(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "csv")).collect();
    files.sort();
    files.reverse();
    files
}

// ---------------------------------------------------------------- Numbers

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub seconds: f64,
    pub avg_temp_c: Option<f64>,
    pub max_temp_c: Option<f64>,
    pub avg_power_w: Option<f64>,
    pub max_power_w: Option<f64>,
    pub energy_wh: Option<f64>,
    pub avg_util_pct: Option<f64>,
    pub avg_core_mhz: Option<f64>,
    pub avg_fan_pct: Option<f64>,
    /// Core clock weighted by how busy the card was, on average.
    pub active_mhz: Option<f64>,
    /// `active_mhz` per watt: how much clock-work each watt bought.
    pub active_mhz_per_w: Option<f64>,
    pub avg_fps: Option<f64>,
    pub fps_per_w: Option<f64>,
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, n) = values.fold((0.0, 0u32), |(s, n), v| (s + v, n + 1));
    (n > 0).then(|| sum / f64::from(n))
}

fn max(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.reduce(f64::max)
}

pub fn stats(s: &Session) -> Stats {
    let rows = &s.rows;
    let seconds = match (rows.first(), rows.last()) {
        (Some(a), Some(b)) => b.t - a.t,
        _ => 0.0,
    };
    let avg_power_w = mean(rows.iter().filter_map(|r| r.sample.power_w));
    // Energy by the trapezoid rule over the rows that have a power reading.
    let energy_wh = {
        let pts: Vec<(f64, f64)> = rows.iter().filter_map(|r| r.sample.power_w.map(|p| (r.t, p))).collect();
        (pts.len() >= 2).then(|| pts.windows(2).map(|w| (w[1].0 - w[0].0) * (w[0].1 + w[1].1) / 2.0).sum::<f64>() / 3600.0)
    };
    let active_mhz = mean(rows.iter().filter_map(|r| Some(r.sample.core_mhz? * r.sample.util_pct? / 100.0)));
    let avg_fps = mean(rows.iter().filter_map(|r| r.fps));
    let per_w = |v: Option<f64>| match (v, avg_power_w) {
        (Some(v), Some(p)) if p > 0.0 => Some(v / p),
        _ => None,
    };
    Stats {
        seconds,
        avg_temp_c: mean(rows.iter().filter_map(|r| r.sample.temp_c)),
        max_temp_c: max(rows.iter().filter_map(|r| r.sample.temp_c)),
        avg_power_w,
        max_power_w: max(rows.iter().filter_map(|r| r.sample.power_w)),
        energy_wh,
        avg_util_pct: mean(rows.iter().filter_map(|r| r.sample.util_pct)),
        avg_core_mhz: mean(rows.iter().filter_map(|r| r.sample.core_mhz)),
        avg_fan_pct: mean(rows.iter().filter_map(|r| r.sample.fan_pct)),
        active_mhz,
        active_mhz_per_w: per_w(active_mhz),
        avg_fps,
        fps_per_w: per_w(avg_fps),
    }
}

/// Which way is better for a measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Better {
    Higher,
    Lower,
    Neither,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub label: &'static str,
    pub unit: &'static str,
    pub a: f64,
    pub b: f64,
    pub better: Better,
}

impl Line {
    /// How much `b` differs from `a`, in percent of `a`.
    pub fn change_pct(&self) -> Option<f64> {
        (self.a.abs() > f64::EPSILON).then(|| (self.b - self.a) / self.a * 100.0)
    }

    /// `Some(true)` if `b` is the better one, `Some(false)` if `a`, `None` for a tie or no ranking.
    pub fn b_wins(&self) -> Option<bool> {
        match self.better {
            Better::Neither => None,
            _ if (self.a - self.b).abs() < f64::EPSILON => None,
            Better::Higher => Some(self.b > self.a),
            Better::Lower => Some(self.b < self.a),
        }
    }
}

/// The measures both sessions have, side by side.
pub fn compare(a: &Stats, b: &Stats) -> Vec<Line> {
    let mut out = Vec::new();
    let mut add = |label, unit, x: Option<f64>, y: Option<f64>, better| {
        if let (Some(a), Some(b)) = (x, y) {
            out.push(Line { label, unit, a, b, better });
        }
    };
    add("Average temperature", "°C", a.avg_temp_c, b.avg_temp_c, Better::Lower);
    add("Highest temperature", "°C", a.max_temp_c, b.max_temp_c, Better::Lower);
    add("Average power", "W", a.avg_power_w, b.avg_power_w, Better::Lower);
    add("Highest power", "W", a.max_power_w, b.max_power_w, Better::Lower);
    add("Energy used", "Wh", a.energy_wh, b.energy_wh, Better::Lower);
    add("Average fan speed", "%", a.avg_fan_pct, b.avg_fan_pct, Better::Lower);
    add("Average core clock", "MHz", a.avg_core_mhz, b.avg_core_mhz, Better::Higher);
    add("Active clock", "MHz", a.active_mhz, b.active_mhz, Better::Higher);
    add("Active clock per watt", "MHz/W", a.active_mhz_per_w, b.active_mhz_per_w, Better::Higher);
    add("Average frames per second", "fps", a.avg_fps, b.avg_fps, Better::Higher);
    add("Frames per second per watt", "fps/W", a.fps_per_w, b.fps_per_w, Better::Higher);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(temp: f64, util: f64, clock: f64, power: f64, fan: f64) -> Sample {
        Sample { temp_c: Some(temp), util_pct: Some(util), core_mhz: Some(clock), power_w: Some(power), fan_pct: Some(fan), ..Sample::default() }
    }

    fn session(label: &str, power: f64, clock: f64, temp: f64) -> Session {
        let mut r = Recorder::new(label, "0000:03:00.0", "2026-11-07T10:00:00");
        for t in 0..=60 {
            r.push(f64::from(t), sample(temp, 100.0, clock, power, 40.0));
        }
        r.finish()
    }

    #[test]
    fn computes_energy_and_efficiency() {
        let st = stats(&session("Stock", 200.0, 2400.0, 70.0));
        assert_eq!(st.seconds, 60.0);
        assert!((st.energy_wh.unwrap() - 200.0 * 60.0 / 3600.0).abs() < 1e-9, "200 W for a minute is 3.33 Wh");
        assert_eq!(st.active_mhz, Some(2400.0));
        assert_eq!(st.active_mhz_per_w, Some(12.0));
        assert_eq!((st.avg_fps, st.fps_per_w), (None, None));
    }

    #[test]
    fn an_idle_card_does_less_work_per_watt() {
        let mut r = Recorder::new("Idle", "g", "2026-11-07T10:00:00");
        for t in 0..10 {
            r.push(f64::from(t), sample(40.0, 5.0, 2400.0, 40.0, 20.0));
        }
        let st = stats(&r.finish());
        assert_eq!(st.active_mhz, Some(120.0), "clock weighted by utilization");
    }

    #[test]
    fn a_undervolted_profile_wins_on_efficiency() {
        let stock = stats(&session("Stock", 250.0, 2500.0, 78.0));
        let tuned = stats(&session("Tuned", 200.0, 2450.0, 70.0));
        let lines = compare(&stock, &tuned);
        let by = |name: &str| lines.iter().find(|l| l.label == name).unwrap().clone();
        let eff = by("Active clock per watt");
        assert_eq!(eff.b_wins(), Some(true));
        assert!(eff.change_pct().unwrap() > 20.0, "{:?}", eff.change_pct());
        assert_eq!(by("Average power").b_wins(), Some(true), "lower is better for power");
        assert_eq!(by("Average core clock").b_wins(), Some(false), "but the stock clock is higher");
        assert!(by("Average temperature").change_pct().unwrap() < 0.0);
    }

    #[test]
    fn frames_per_watt_appear_when_frames_were_recorded() {
        let mut s = session("Game", 200.0, 2400.0, 70.0);
        for r in &mut s.rows {
            r.fps = Some(100.0);
        }
        let st = stats(&s);
        assert_eq!((st.avg_fps, st.fps_per_w), (Some(100.0), Some(0.5)));
        assert!(compare(&st, &st).iter().any(|l| l.label == "Frames per second per watt"));
    }

    #[test]
    fn sessions_round_trip_through_csv_and_files() {
        let mut s = session("Quiet profile", 180.0, 2300.0, 66.0);
        s.rows[3].fps = Some(60.0);
        s.rows[4].sample.power_w = None;
        let back = from_csv(&to_csv(&s)).unwrap();
        assert_eq!(back, s);
        let dir = std::env::temp_dir().join(format!("voltkit-session-{}", std::process::id()));
        let path = save(&dir, &s).unwrap();
        assert!(path.file_name().unwrap().to_string_lossy().contains("Quiet_profile"));
        assert_eq!(load(&path).unwrap(), s);
        assert_eq!(list(&dir), vec![path]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_csv_from_other_tools_with_extra_columns() {
        let text = "# label: MangoHud run\nT_S,fps,power_w,core_mhz,util_pct,extra\n0,60,100,2000,50,x\n1,62,110,2100,60,y\n";
        let s = from_csv(text).unwrap();
        assert_eq!(s.label, "MangoHud run");
        assert_eq!((s.rows.len(), s.rows[1].fps, s.rows[0].sample.core_mhz), (2, Some(62.0), Some(2000.0)));
        assert!(from_csv("1,2,3\n").is_err() || from_csv("1,2,3\n").unwrap().rows.is_empty());
        assert!(from_csv("").is_err());
    }

    #[test]
    fn out_of_order_readings_are_ignored() {
        let mut r = Recorder::new("x", "g", "t");
        r.push(1.0, Sample::default());
        r.push(1.0, Sample::default());
        r.push(0.5, Sample::default());
        r.push(2.0, Sample::default());
        assert_eq!(r.finish().rows.len(), 2);
    }
}
