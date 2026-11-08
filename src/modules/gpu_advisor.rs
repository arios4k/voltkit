//! The efficiency advisor: from sessions recorded under different power limits,
//! find the lowest limit that keeps nearly all of the performance.
//!
//! Cards lose very little speed over the last tens of watts, so the best limit is
//! usually well below the stock one. The advisor groups recorded sessions by the
//! power limit they ran under, scores each group by frames per second (or, without
//! frame data, active megahertz), and picks the group that draws the least power
//! while staying within a tolerance of the fastest. It only reads what was
//! measured: it never suggests a limit nobody tried.

use super::gpu_session::{Session, stats};

/// Sessions shorter than this, or on a mostly idle card, say nothing about a limit.
pub const MIN_SECONDS: f64 = 30.0;
pub const MIN_UTIL_PCT: f64 = 50.0;
/// Limits closer than this count as the same.
const BUCKET_W: f64 = 5.0;

/// What performance was measured in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Metric {
    Fps,
    ActiveMhz,
}

impl Metric {
    pub fn unit(self) -> &'static str {
        match self {
            Metric::Fps => "fps",
            Metric::ActiveMhz => "MHz",
        }
    }
}

/// The sessions that ran under one power limit.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub limit_w: f64,
    pub perf: f64,
    pub power_w: f64,
    pub runs: usize,
}

impl Point {
    pub fn per_watt(&self) -> f64 {
        self.perf / self.power_w
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Advice {
    pub metric: Metric,
    /// Every limit that was tried, highest limit first.
    pub points: Vec<Point>,
    /// The fastest limit tried.
    pub fastest: Point,
    /// The recommendation. Equal to `fastest` when nothing cheaper keeps up.
    pub pick: Point,
}

impl Advice {
    /// How much slower the pick is than the fastest, in percent.
    pub fn performance_lost_pct(&self) -> f64 {
        100.0 * (1.0 - self.pick.perf / self.fastest.perf)
    }

    /// How much less power the pick draws than the fastest, in percent.
    pub fn power_saved_pct(&self) -> f64 {
        100.0 * (1.0 - self.pick.power_w / self.fastest.power_w)
    }

    /// Whether the pick differs from just running flat out.
    pub fn is_saving(&self) -> bool {
        self.pick.limit_w < self.fastest.limit_w - 0.5 && self.power_saved_pct() >= 1.0
    }
}

/// Averages over sessions that all ran under one limit.
fn point(limit_w: f64, runs: &[(f64, f64)]) -> Point {
    let n = runs.len() as f64;
    Point { limit_w, perf: runs.iter().map(|r| r.0).sum::<f64>() / n, power_w: runs.iter().map(|r| r.1).sum::<f64>() / n, runs: runs.len() }
}

/// Advice from `sessions`, accepting up to `tolerance_pct` less performance than
/// the fastest limit. `None` unless at least two different limits were measured.
pub fn advise(sessions: &[Session], tolerance_pct: f64) -> Option<Advice> {
    let usable: Vec<_> = sessions
        .iter()
        .map(stats)
        .filter(|s| s.seconds >= MIN_SECONDS && s.avg_util_pct.is_some_and(|u| u >= MIN_UTIL_PCT))
        .filter(|s| s.avg_limit_w.is_some() && s.avg_power_w.is_some_and(|p| p > 0.0))
        .collect();
    // Frames if every usable session has them, otherwise the clock work done.
    let metric = if !usable.is_empty() && usable.iter().all(|s| s.avg_fps.is_some()) { Metric::Fps } else { Metric::ActiveMhz };
    let mut buckets: Vec<(f64, Vec<(f64, f64)>)> = Vec::new();
    for s in &usable {
        let (Some(limit), Some(power)) = (s.avg_limit_w, s.avg_power_w) else { continue };
        let Some(perf) = (if metric == Metric::Fps { s.avg_fps } else { s.active_mhz }).filter(|p| *p > 0.0) else { continue };
        let key = (limit / BUCKET_W).round() * BUCKET_W;
        match buckets.iter_mut().find(|b| b.0 == key) {
            Some(b) => b.1.push((perf, power)),
            None => buckets.push((key, vec![(perf, power)])),
        }
    }
    if buckets.len() < 2 {
        return None;
    }
    buckets.sort_by(|a, b| b.0.total_cmp(&a.0));
    let points: Vec<Point> = buckets.iter().map(|(limit, runs)| point(*limit, runs)).collect();
    let fastest = points.iter().max_by(|a, b| a.perf.total_cmp(&b.perf))?.clone();
    let floor = fastest.perf * (1.0 - tolerance_pct / 100.0);
    let pick = points.iter().filter(|p| p.perf >= floor).min_by(|a, b| a.power_w.total_cmp(&b.power_w))?.clone();
    Some(Advice { metric, points, fastest, pick })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::gpu::Sample;
    use crate::modules::gpu_session::Recorder;

    fn run(limit: f64, power: f64, mhz: f64, fps: Option<f64>) -> Session {
        let mut r = Recorder::new("t", "0000:03:00.0", "2026-01-01T00:00:00");
        for t in 0..=60 {
            r.push(f64::from(t), Sample { util_pct: Some(98.0), core_mhz: Some(mhz), power_w: Some(power), limit_w: Some(limit), ..Sample::default() });
        }
        let mut s = r.finish();
        for row in &mut s.rows {
            row.fps = fps;
        }
        s
    }

    #[test]
    fn it_picks_the_cheapest_limit_that_keeps_up() {
        // Stock 300 W: 100 fps. 240 W: 98.5 fps. 180 W: 90 fps.
        let sessions = [run(300.0, 295.0, 2500.0, Some(100.0)), run(240.0, 238.0, 2450.0, Some(98.5)), run(180.0, 178.0, 2200.0, Some(90.0))];
        let a = advise(&sessions, 3.0).unwrap();
        assert_eq!(a.metric, Metric::Fps);
        assert_eq!(a.fastest.limit_w, 300.0);
        assert_eq!(a.pick.limit_w, 240.0);
        assert!(a.is_saving());
        assert!((a.performance_lost_pct() - 1.5).abs() < 0.01);
        assert!((a.power_saved_pct() - 19.3).abs() < 0.1);
    }

    #[test]
    fn a_wider_tolerance_goes_lower_and_a_zero_one_keeps_stock() {
        let sessions = [run(300.0, 295.0, 2500.0, Some(100.0)), run(240.0, 238.0, 2450.0, Some(98.5)), run(180.0, 178.0, 2200.0, Some(90.0))];
        assert_eq!(advise(&sessions, 12.0).unwrap().pick.limit_w, 180.0);
        let strict = advise(&sessions, 0.0).unwrap();
        assert_eq!(strict.pick.limit_w, 300.0);
        assert!(!strict.is_saving());
    }

    #[test]
    fn without_frames_it_uses_the_clock_work_done() {
        let sessions = [run(300.0, 295.0, 2500.0, None), run(250.0, 245.0, 2480.0, None)];
        let a = advise(&sessions, 2.0).unwrap();
        assert_eq!(a.metric, Metric::ActiveMhz);
        assert_eq!(a.pick.limit_w, 250.0);
    }

    #[test]
    fn it_needs_two_limits_and_ignores_short_or_idle_runs() {
        assert!(advise(&[run(300.0, 295.0, 2500.0, None), run(301.0, 290.0, 2500.0, None)], 3.0).is_none(), "limits within 5 W are one");
        let mut short = run(200.0, 190.0, 2000.0, None);
        short.rows.truncate(10);
        assert!(advise(&[run(300.0, 295.0, 2500.0, None), short], 3.0).is_none());
        let mut idle = run(200.0, 60.0, 500.0, None);
        for r in &mut idle.rows {
            r.sample.util_pct = Some(5.0);
        }
        assert!(advise(&[run(300.0, 295.0, 2500.0, None), idle], 3.0).is_none());
        assert!(advise(&[], 3.0).is_none());
    }

    #[test]
    fn repeated_runs_at_one_limit_are_averaged() {
        let sessions = [run(300.0, 300.0, 2500.0, Some(100.0)), run(298.0, 290.0, 2500.0, Some(96.0)), run(200.0, 195.0, 2300.0, Some(95.0))];
        let a = advise(&sessions, 3.0).unwrap();
        let top = &a.points[0];
        assert_eq!((top.runs, top.perf, top.power_w), (2, 98.0, 295.0));
    }
}
