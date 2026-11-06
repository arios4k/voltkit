//! The rules for editing a fan curve with the mouse or keyboard: moving,
//! adding and removing points so the curve always stays valid (temperatures
//! going up by at least a degree, speeds that never drop) and inside the
//! chart's range. The chart widget only draws and reads the mouse; every
//! change goes through here, so the rules are tested without a screen.

use std::collections::BTreeSet;

use super::gpu_tune::{CurvePoint, speed_at};

pub const TEMP_MIN: f64 = 0.0;
pub const TEMP_MAX: f64 = 110.0;
pub const SPEED_MIN: f64 = 0.0;
pub const SPEED_MAX: f64 = 100.0;
/// Points are at least this many degrees apart.
pub const MIN_GAP_C: f64 = 1.0;

/// Maximal runs of consecutive selected indices, as `(first, last)`.
fn runs(selected: &BTreeSet<usize>) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for &i in selected {
        match out.last_mut() {
            Some((_, last)) if *last + 1 == i => *last = i,
            _ => out.push((i, i)),
        }
    }
    out
}

/// Moves the selected points by `dt` degrees and `ds` percent, as far as the
/// curve stays valid: the whole selection stops when any point reaches a
/// neighbour or the edge of the chart. With `snap`, results are whole numbers.
pub fn move_points(points: &[CurvePoint], selected: &BTreeSet<usize>, dt: f64, ds: f64, snap: bool) -> Vec<CurvePoint> {
    let selected: BTreeSet<usize> = selected.iter().copied().filter(|i| *i < points.len()).collect();
    if selected.is_empty() {
        return points.to_vec();
    }
    let (mut dt_lo, mut dt_hi) = (f64::MIN, f64::MAX);
    let (mut ds_lo, mut ds_hi) = (f64::MIN, f64::MAX);
    for (first, last) in runs(&selected) {
        let (t_first, t_last) = (points[first].temp_c, points[last].temp_c);
        let (s_first, s_last) = (points[first].speed_pct, points[last].speed_pct);
        // Neighbours outside the run, or the edges of the chart.
        let (before_t, before_s) = if first > 0 { (points[first - 1].temp_c + MIN_GAP_C, points[first - 1].speed_pct) } else { (TEMP_MIN, SPEED_MIN) };
        let (after_t, after_s) =
            if last + 1 < points.len() { (points[last + 1].temp_c - MIN_GAP_C, points[last + 1].speed_pct) } else { (TEMP_MAX, SPEED_MAX) };
        dt_lo = dt_lo.max(before_t - t_first);
        dt_hi = dt_hi.min(after_t - t_last);
        ds_lo = ds_lo.max(before_s - s_first);
        ds_hi = ds_hi.min(after_s - s_last);
    }
    let (dt, ds) = (dt.clamp(dt_lo.min(0.0), dt_hi.max(0.0)), ds.clamp(ds_lo.min(0.0), ds_hi.max(0.0)));
    let mut out = points.to_vec();
    for &i in &selected {
        let (mut t, mut s) = (points[i].temp_c + dt, points[i].speed_pct + ds);
        if snap {
            t = t.round();
            s = s.round();
        }
        out[i] = CurvePoint { temp_c: t, speed_pct: s };
    }
    // Rounding can close a gap by a fraction; give the neighbours back their space.
    for i in 1..out.len() {
        if out[i].temp_c < out[i - 1].temp_c + MIN_GAP_C {
            out[i].temp_c = out[i - 1].temp_c + MIN_GAP_C;
        }
        if out[i].speed_pct < out[i - 1].speed_pct {
            out[i].speed_pct = out[i - 1].speed_pct;
        }
    }
    out
}

/// Adds a point on the curve at `temp`. Returns the new points and the index of
/// the new point, or `None` if it would sit within a degree of another point or
/// outside the chart.
pub fn add_point(points: &[CurvePoint], temp: f64) -> Option<(Vec<CurvePoint>, usize)> {
    let temp = temp.round();
    if !(TEMP_MIN..=TEMP_MAX).contains(&temp) || points.iter().any(|p| (p.temp_c - temp).abs() < MIN_GAP_C) {
        return None;
    }
    let at = points.iter().position(|p| p.temp_c > temp).unwrap_or(points.len());
    let mut out = points.to_vec();
    out.insert(at, CurvePoint { temp_c: temp, speed_pct: speed_at(points, temp).round() });
    Some((out, at))
}

/// Removes the selected points, always leaving at least two.
pub fn remove_points(points: &[CurvePoint], selected: &BTreeSet<usize>) -> Vec<CurvePoint> {
    let kept: Vec<CurvePoint> = points.iter().enumerate().filter(|(i, _)| !selected.contains(i)).map(|(_, p)| p.clone()).collect();
    if kept.len() < 2 { points.to_vec() } else { kept }
}

/// The index of the point nearest to `(x, y)` within `radius`, all in the same units.
pub fn nearest(points: &[(f64, f64)], x: f64, y: f64, radius: f64) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(i, (px, py))| (i, ((px - x).powi(2) + (py - y).powi(2)).sqrt()))
        .filter(|(_, d)| *d <= radius)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// The indices of points inside a rectangle given by two corners.
pub fn inside(points: &[(f64, f64)], a: (f64, f64), b: (f64, f64)) -> BTreeSet<usize> {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    points.iter().enumerate().filter(|(_, (x, y))| (x0..=x1).contains(x) && (y0..=y1).contains(y)).map(|(i, _)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::gpu_tune::validate_curve;

    fn pt(t: f64, s: f64) -> CurvePoint {
        CurvePoint { temp_c: t, speed_pct: s }
    }

    fn curve() -> Vec<CurvePoint> {
        vec![pt(30.0, 20.0), pt(50.0, 40.0), pt(70.0, 65.0), pt(85.0, 100.0)]
    }

    fn sel(items: &[usize]) -> BTreeSet<usize> {
        items.iter().copied().collect()
    }

    #[test]
    fn a_point_moves_freely_between_its_neighbours() {
        let moved = move_points(&curve(), &sel(&[1]), 5.0, 10.0, true);
        assert_eq!(moved[1], pt(55.0, 50.0));
        assert_eq!(moved[0], curve()[0]);
    }

    #[test]
    fn a_point_stops_at_its_neighbours() {
        let right = move_points(&curve(), &sel(&[1]), 100.0, 0.0, true);
        assert_eq!(right[1].temp_c, 69.0, "one degree short of the next point");
        let left = move_points(&curve(), &sel(&[1]), -100.0, 0.0, true);
        assert_eq!(left[1].temp_c, 31.0);
        // The speed can't pass the cooler or the hotter neighbour either.
        let up = move_points(&curve(), &sel(&[1]), 0.0, 100.0, true);
        assert_eq!(up[1].speed_pct, 65.0);
        let down = move_points(&curve(), &sel(&[1]), 0.0, -100.0, true);
        assert_eq!(down[1].speed_pct, 20.0);
    }

    #[test]
    fn the_ends_stop_at_the_edges_of_the_chart() {
        let m = move_points(&curve(), &sel(&[3]), 100.0, 100.0, true);
        assert_eq!((m[3].temp_c, m[3].speed_pct), (110.0, 100.0));
        let m = move_points(&curve(), &sel(&[0]), -100.0, -100.0, true);
        assert_eq!((m[0].temp_c, m[0].speed_pct), (0.0, 0.0));
    }

    #[test]
    fn several_points_move_together_and_stop_together() {
        let m = move_points(&curve(), &sel(&[1, 2]), 10.0, 0.0, true);
        assert_eq!((m[1].temp_c, m[2].temp_c), (60.0, 80.0));
        // The run stops when its first point to reach a neighbour does.
        let m = move_points(&curve(), &sel(&[1, 2]), 100.0, 0.0, true);
        assert_eq!((m[1].temp_c, m[2].temp_c), (64.0, 84.0), "the gap between them is kept");
    }

    #[test]
    fn separate_selections_move_by_the_same_amount() {
        let m = move_points(&curve(), &sel(&[0, 2]), 6.0, 0.0, true);
        assert_eq!((m[0].temp_c, m[2].temp_c), (36.0, 76.0));
        let m = move_points(&curve(), &sel(&[0, 2]), 100.0, 0.0, true);
        assert_eq!((m[0].temp_c, m[2].temp_c), (44.0, 84.0), "limited by the nearer wall, here the 85 °C point");
    }

    #[test]
    fn dragging_never_produces_an_invalid_curve() {
        // Every combination of a point, a direction and a distance leaves a valid shape
        // (the safety rule about 85 °C is checked on save, not while dragging).
        for i in 0..4 {
            for dt in [-200.0, -7.0, 0.0, 3.5, 200.0] {
                for ds in [-200.0, -9.0, 0.0, 4.5, 200.0] {
                    let m = move_points(&curve(), &sel(&[i]), dt, ds, true);
                    assert!(m.windows(2).all(|w| w[1].temp_c >= w[0].temp_c + MIN_GAP_C && w[1].speed_pct >= w[0].speed_pct), "{i} {dt} {ds}: {m:?}");
                    assert!(m.iter().all(|p| (TEMP_MIN..=TEMP_MAX).contains(&p.temp_c) && (SPEED_MIN..=SPEED_MAX).contains(&p.speed_pct)));
                }
            }
        }
        assert!(validate_curve(&move_points(&curve(), &sel(&[1]), 4.0, 3.0, true)).is_ok());
    }

    #[test]
    fn adding_a_point_sits_on_the_curve() {
        let (c, at) = add_point(&curve(), 60.0).unwrap();
        assert_eq!(at, 2);
        assert_eq!(c[2], pt(60.0, 53.0));
        assert!(add_point(&curve(), 50.4).is_none(), "too close to an existing point");
        assert!(add_point(&curve(), 120.0).is_none());
        let (c, at) = add_point(&curve(), 95.0).unwrap();
        assert_eq!((at, c[4].speed_pct), (4, 100.0));
    }

    #[test]
    fn removing_keeps_at_least_two_points() {
        assert_eq!(remove_points(&curve(), &sel(&[1])).len(), 3);
        assert_eq!(remove_points(&curve(), &sel(&[0, 1, 2])).len(), 4, "refused: only one would be left");
        assert_eq!(remove_points(&curve(), &sel(&[0, 1])).len(), 2);
    }

    #[test]
    fn hit_testing_and_rubber_band_selection() {
        let px = [(10.0, 10.0), (50.0, 50.0), (90.0, 20.0)];
        assert_eq!(nearest(&px, 52.0, 48.0, 8.0), Some(1));
        assert_eq!(nearest(&px, 70.0, 70.0, 8.0), None);
        assert_eq!(inside(&px, (0.0, 0.0), (60.0, 60.0)), sel(&[0, 1]));
        assert_eq!(inside(&px, (95.0, 5.0), (60.0, 25.0)), sel(&[2]), "corners in any order");
    }
}
