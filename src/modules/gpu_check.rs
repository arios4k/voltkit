//! Checks Voltkit's own readings against a second source, and measures what
//! monitoring costs, so claims about accuracy and resource use rest on numbers.
//!
//! The comparison is only as independent as the second source: on NVIDIA cards
//! Voltkit reads NVML while `nvidia-smi` is a separate process asking the same
//! driver, which catches parsing and unit mistakes but not a driver that is wrong.
//! For AMD and Intel there is no second tool on most computers, so the check
//! says so instead of pretending. The targets come from docs/GPU_TUNER.md.

use super::gpu::Sample;

#[derive(Clone, Debug, PartialEq)]
pub struct Accuracy {
    pub metric: &'static str,
    pub unit: &'static str,
    pub samples: usize,
    pub mean_abs: f64,
    pub max_abs: f64,
    /// The largest difference that counts as agreeing.
    pub target: f64,
}

impl Accuracy {
    pub fn within_target(&self) -> bool {
        self.max_abs <= self.target
    }
}

/// Compares two series of readings taken at the same moments, metric by metric.
/// A metric is left out when either source doesn't report it.
pub fn compare_sources(ours: &[Sample], theirs: &[Sample]) -> Vec<Accuracy> {
    type Pick = fn(&Sample) -> Option<f64>;
    let metrics: [(&'static str, &'static str, f64, Pick); 6] = [
        ("Temperature", "°C", 1.0, |s| s.temp_c),
        ("Utilization", "%", 5.0, |s| s.util_pct),
        ("Core clock", "MHz", 15.0, |s| s.core_mhz),
        ("Memory clock", "MHz", 15.0, |s| s.mem_mhz),
        ("Power", "W", 2.0, |s| s.power_w),
        ("Video memory", "MB", 8.0, |s| s.vram_used_mb),
    ];
    metrics
        .into_iter()
        .filter_map(|(metric, unit, target, pick)| {
            let diffs: Vec<f64> = ours.iter().zip(theirs).filter_map(|(a, b)| Some((pick(a)? - pick(b)?).abs())).collect();
            (!diffs.is_empty()).then(|| Accuracy {
                metric,
                unit,
                samples: diffs.len(),
                mean_abs: diffs.iter().sum::<f64>() / diffs.len() as f64,
                max_abs: diffs.iter().cloned().fold(0.0, f64::max),
                target,
            })
        })
        .collect()
}

/// CPU time this process has used, in seconds, and its memory in megabytes.
pub fn process_usage(stat: &str, statm: &str, ticks_per_second: f64, page_bytes: f64) -> Option<(f64, f64)> {
    // Fields after the command name, which may contain spaces inside parentheses.
    let after = stat.rsplit_once(')')?.1;
    let f: Vec<&str> = after.split_whitespace().collect();
    // utime and stime are fields 14 and 15 of the line, which are 11 and 12 after the name.
    let cpu = (f.get(11)?.parse::<f64>().ok()? + f.get(12)?.parse::<f64>().ok()?) / ticks_per_second;
    let rss_pages: f64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some((cpu, rss_pages * page_bytes / (1024.0 * 1024.0)))
}

pub fn read_usage() -> Option<(f64, f64)> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    process_usage(&stat, &statm, 100.0, 4096.0)
}

/// CPU use in percent of one core over a stretch of wall-clock time.
pub fn cpu_percent(cpu_before: f64, cpu_after: f64, wall_seconds: f64) -> f64 {
    if wall_seconds <= 0.0 { 0.0 } else { (cpu_after - cpu_before) / wall_seconds * 100.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(temp: f64, util: f64, power: Option<f64>) -> Sample {
        Sample { temp_c: Some(temp), util_pct: Some(util), power_w: power, ..Sample::default() }
    }

    #[test]
    fn measures_how_far_two_sources_disagree() {
        let ours = [s(50.0, 10.0, Some(100.0)), s(52.0, 20.0, Some(110.0)), s(53.0, 30.0, Some(120.0))];
        let theirs = [s(50.0, 12.0, Some(101.0)), s(51.0, 20.0, Some(110.0)), s(55.0, 31.0, Some(119.0))];
        let r = compare_sources(&ours, &theirs);
        let temp = r.iter().find(|a| a.metric == "Temperature").unwrap();
        assert_eq!((temp.samples, temp.max_abs), (3, 2.0));
        assert!((temp.mean_abs - 1.0).abs() < 1e-9);
        assert!(!temp.within_target(), "2 °C apart is outside the 1 °C target");
        let power = r.iter().find(|a| a.metric == "Power").unwrap();
        assert!(power.within_target());
        assert!(r.iter().all(|a| a.metric != "Core clock"), "neither source reported it");
    }

    #[test]
    fn a_metric_one_source_lacks_is_left_out() {
        let r = compare_sources(&[s(50.0, 10.0, Some(100.0))], &[s(50.0, 10.0, None)]);
        assert!(r.iter().all(|a| a.metric != "Power"));
        assert!(compare_sources(&[], &[]).is_empty());
    }

    #[test]
    fn reads_cpu_time_and_memory_from_proc() {
        // The command name may hold spaces and parentheses.
        let stat = "1234 (voltkit (gpu)) S 1 1 1 0 -1 4194560 100 0 0 0 250 50 0 0 20 0 5 0 100 1000000 5000 18446744073709551615";
        let statm = "6000 5000 2000 100 0 3000 0";
        let (cpu, rss) = process_usage(stat, statm, 100.0, 4096.0).unwrap();
        assert!((cpu - 3.0).abs() < 1e-9, "250 + 50 ticks at 100 per second");
        assert!((rss - 5000.0 * 4096.0 / 1048576.0).abs() < 1e-9);
        assert!(process_usage("garbage", "1", 100.0, 4096.0).is_none());
        assert!(read_usage().is_some(), "this process can read its own numbers");
    }

    #[test]
    fn cpu_percent_is_a_share_of_one_core() {
        assert_eq!(cpu_percent(1.0, 1.5, 10.0), 5.0);
        assert_eq!(cpu_percent(0.0, 1.0, 0.0), 0.0);
    }
}
