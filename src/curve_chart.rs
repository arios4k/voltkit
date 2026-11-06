//! A fan curve you edit on a graph: drag points, rubber-band to select several,
//! nudge with the arrow keys, double-click the line to add a point, right-click
//! a point or press Delete to remove it. Every change goes through
//! `modules::gpu_curve`, so the curve stays valid while you drag. A marker
//! shows where the card is right now.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gdk, glib};

use crate::i18n::tr;
use crate::modules::gpu_curve::{self as rules, SPEED_MAX, SPEED_MIN, TEMP_MAX, TEMP_MIN};
use crate::modules::gpu_tune::{CurvePoint, SAFE_FAN_PCT, SAFE_TEMP_C};

const LEFT: f64 = 42.0;
const RIGHT: f64 = 14.0;
const TOP: f64 = 12.0;
const BOTTOM: f64 = 28.0;
const HIT_RADIUS: f64 = 12.0;

enum Drag {
    Move { start: Vec<CurvePoint> },
    Band { from: (f64, f64), to: (f64, f64), add: bool, before: BTreeSet<usize> },
}

struct State {
    points: Vec<CurvePoint>,
    selected: BTreeSet<usize>,
    hover: Option<usize>,
    marker: Option<(f64, f64)>,
    drag: Option<Drag>,
}

type Changed = Rc<RefCell<Option<Box<dyn Fn(Vec<CurvePoint>, Vec<CurvePoint>)>>>>;

#[derive(Clone)]
pub struct CurveChart {
    pub area: gtk::DrawingArea,
    state: Rc<RefCell<State>>,
    changed: Changed,
}

/// Where chart coordinates land on screen for a drawing area of this size.
struct Plot {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Plot {
    fn new(w: f64, h: f64) -> Self {
        Plot { x0: LEFT, y0: TOP, x1: (w - RIGHT).max(LEFT + 1.0), y1: (h - BOTTOM).max(TOP + 1.0) }
    }

    fn px(&self, t: f64, s: f64) -> (f64, f64) {
        (
            self.x0 + (t - TEMP_MIN) / (TEMP_MAX - TEMP_MIN) * (self.x1 - self.x0),
            self.y1 - (s - SPEED_MIN) / (SPEED_MAX - SPEED_MIN) * (self.y1 - self.y0),
        )
    }

    /// Degrees and percent per pixel.
    fn per_pixel(&self) -> (f64, f64) {
        ((TEMP_MAX - TEMP_MIN) / (self.x1 - self.x0), (SPEED_MAX - SPEED_MIN) / (self.y1 - self.y0))
    }

    fn value(&self, x: f64, y: f64) -> (f64, f64) {
        let (kt, ks) = self.per_pixel();
        (TEMP_MIN + (x - self.x0) * kt, SPEED_MIN + (self.y1 - y) * ks)
    }
}

impl CurveChart {
    pub fn new(points: Vec<CurvePoint>) -> Self {
        let area = gtk::DrawingArea::builder().content_height(230).hexpand(true).focusable(true).can_focus(true).build();
        area.set_tooltip_text(Some(&tr("Drag points to shape the curve. Double-click the line to add a point; right-click a point or press Delete to remove it.")));
        area.update_property(&[gtk::accessible::Property::Label(&tr("Fan curve chart"))]);
        let chart = CurveChart {
            area,
            state: Rc::new(RefCell::new(State { points, selected: BTreeSet::new(), hover: None, marker: None, drag: None })),
            changed: Rc::new(RefCell::new(None)),
        };
        chart.install_drawing();
        chart.install_mouse();
        chart.install_keyboard();
        chart
    }

    /// Called with the curve before and after each finished edit.
    pub fn connect_changed(&self, f: impl Fn(Vec<CurvePoint>, Vec<CurvePoint>) + 'static) {
        *self.changed.borrow_mut() = Some(Box::new(f));
    }

    /// Replaces the curve without announcing it as an edit.
    pub fn set_points(&self, points: Vec<CurvePoint>) {
        let mut s = self.state.borrow_mut();
        s.selected.retain(|i| *i < points.len());
        s.points = points;
        drop(s);
        self.area.queue_draw();
    }

    /// The card's current temperature and fan speed.
    pub fn set_marker(&self, marker: Option<(f64, f64)>) {
        self.state.borrow_mut().marker = marker;
        self.area.queue_draw();
    }

    fn announce(&self, before: Vec<CurvePoint>, after: Vec<CurvePoint>) {
        if before != after
            && let Some(f) = self.changed.borrow().as_ref()
        {
            f(before, after);
        }
    }

    fn plot(&self) -> Plot {
        Plot::new(f64::from(self.area.width()), f64::from(self.area.height()))
    }

    fn pixels(&self) -> Vec<(f64, f64)> {
        let plot = self.plot();
        self.state.borrow().points.iter().map(|p| plot.px(p.temp_c, p.speed_pct)).collect()
    }

    // ---- Drawing

    fn install_drawing(&self) {
        let state = self.state.clone();
        self.area.set_draw_func(move |area, cr, w, h| {
            let plot = Plot::new(f64::from(w), f64::from(h));
            let fg = area.color();
            let ink = |cr: &gtk::cairo::Context, alpha: f64| cr.set_source_rgba(f64::from(fg.red()), f64::from(fg.green()), f64::from(fg.blue()), alpha);
            let accent = area
                .style_context()
                .lookup_color("accent_bg_color")
                .unwrap_or_else(|| gdk::RGBA::new(0.21, 0.52, 0.89, 1.0));
            let accent_rgb = (f64::from(accent.red()), f64::from(accent.green()), f64::from(accent.blue()));

            // The region no safe curve may enter: too slow while already hot.
            let (sx, sy) = plot.px(SAFE_TEMP_C, SAFE_FAN_PCT);
            cr.set_source_rgba(0.88, 0.22, 0.2, 0.12);
            cr.rectangle(sx, sy, plot.x1 - sx, plot.y1 - sy);
            let _ = cr.fill();

            // Grid and labels.
            cr.set_font_size(11.0);
            cr.set_line_width(1.0);
            for t in (0..=110).step_by(10) {
                let (x, _) = plot.px(f64::from(t), 0.0);
                ink(cr, 0.12);
                cr.move_to(x, plot.y0);
                cr.line_to(x, plot.y1);
                let _ = cr.stroke();
                if t % 20 == 0 {
                    ink(cr, 0.6);
                    cr.move_to(x - 8.0, plot.y1 + 16.0);
                    let _ = cr.show_text(&format!("{t}°"));
                }
            }
            for s in (0..=100).step_by(20) {
                let (_, y) = plot.px(0.0, f64::from(s));
                ink(cr, 0.12);
                cr.move_to(plot.x0, y);
                cr.line_to(plot.x1, y);
                let _ = cr.stroke();
                ink(cr, 0.6);
                cr.move_to(6.0, y + 4.0);
                let _ = cr.show_text(&format!("{s}%"));
            }

            let st = state.borrow();
            let px: Vec<(f64, f64)> = st.points.iter().map(|p| plot.px(p.temp_c, p.speed_pct)).collect();
            if let (Some(first), Some(last)) = (px.first(), px.last()) {
                // Under the curve, then the curve, flat to the edges like the driver reads it.
                cr.set_source_rgba(accent_rgb.0, accent_rgb.1, accent_rgb.2, 0.14);
                cr.move_to(plot.x0, plot.y1);
                cr.line_to(plot.x0, first.1);
                for p in &px {
                    cr.line_to(p.0, p.1);
                }
                cr.line_to(plot.x1, last.1);
                cr.line_to(plot.x1, plot.y1);
                cr.close_path();
                let _ = cr.fill();
                cr.set_source_rgba(accent_rgb.0, accent_rgb.1, accent_rgb.2, 1.0);
                cr.set_line_width(2.0);
                cr.move_to(plot.x0, first.1);
                for p in &px {
                    cr.line_to(p.0, p.1);
                }
                cr.line_to(plot.x1, last.1);
                let _ = cr.stroke();
            }

            // Where the card is now.
            if let Some((t, s)) = st.marker {
                let (x, y) = plot.px(t.clamp(TEMP_MIN, TEMP_MAX), s.clamp(SPEED_MIN, SPEED_MAX));
                ink(cr, 0.55);
                cr.set_line_width(1.0);
                cr.set_dash(&[3.0, 3.0], 0.0);
                cr.move_to(x, plot.y1);
                cr.line_to(x, y);
                let _ = cr.stroke();
                cr.set_dash(&[], 0.0);
                ink(cr, 0.9);
                cr.arc(x, y, 4.0, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
            }

            for (i, (x, y)) in px.iter().enumerate() {
                let selected = st.selected.contains(&i);
                let radius = if selected || st.hover == Some(i) { 7.0 } else { 5.5 };
                cr.arc(*x, *y, radius, 0.0, std::f64::consts::TAU);
                if selected {
                    cr.set_source_rgba(accent_rgb.0, accent_rgb.1, accent_rgb.2, 1.0);
                } else {
                    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
                }
                let _ = cr.fill_preserve();
                cr.set_source_rgba(accent_rgb.0, accent_rgb.1, accent_rgb.2, 1.0);
                cr.set_line_width(2.0);
                let _ = cr.stroke();
            }

            if let Some(Drag::Band { from, to, .. }) = &st.drag {
                cr.set_source_rgba(accent_rgb.0, accent_rgb.1, accent_rgb.2, 0.15);
                cr.rectangle(from.0.min(to.0), from.1.min(to.1), (from.0 - to.0).abs(), (from.1 - to.1).abs());
                let _ = cr.fill_preserve();
                cr.set_source_rgba(accent_rgb.0, accent_rgb.1, accent_rgb.2, 0.8);
                cr.set_line_width(1.0);
                let _ = cr.stroke();
            }
            if area.has_focus() {
                ink(cr, 0.35);
                cr.set_line_width(1.0);
                cr.rectangle(plot.x0 - 0.5, plot.y0 - 0.5, plot.x1 - plot.x0 + 1.0, plot.y1 - plot.y0 + 1.0);
                let _ = cr.stroke();
            }
        });
    }

    // ---- Mouse

    fn install_mouse(&self) {
        let drag = gtk::GestureDrag::builder().button(1).build();
        {
            let chart = self.clone();
            drag.connect_drag_begin(move |g, x, y| {
                chart.area.grab_focus();
                let add = g.current_event_state().intersects(gdk::ModifierType::SHIFT_MASK | gdk::ModifierType::CONTROL_MASK);
                let hit = rules::nearest(&chart.pixels(), x, y, HIT_RADIUS);
                let mut s = chart.state.borrow_mut();
                match hit {
                    Some(i) => {
                        if add {
                            if !s.selected.remove(&i) {
                                s.selected.insert(i);
                            }
                        } else if !s.selected.contains(&i) {
                            s.selected = BTreeSet::from([i]);
                        }
                        if s.selected.contains(&i) {
                            let start = s.points.clone();
                            s.drag = Some(Drag::Move { start });
                        }
                    }
                    None => {
                        let before = if add { s.selected.clone() } else { BTreeSet::new() };
                        s.selected = before.clone();
                        s.drag = Some(Drag::Band { from: (x, y), to: (x, y), add, before });
                    }
                }
                drop(s);
                chart.area.queue_draw();
            });
        }
        {
            let chart = self.clone();
            drag.connect_drag_update(move |g, dx, dy| {
                let Some((sx, sy)) = g.start_point() else { return };
                let plot = chart.plot();
                let (kt, ks) = plot.per_pixel();
                let px = chart.pixels();
                let mut s = chart.state.borrow_mut();
                let selected = s.selected.clone();
                match &mut s.drag {
                    Some(Drag::Move { start }) => {
                        let moved = rules::move_points(start, &selected, dx * kt, -dy * ks, true);
                        s.points = moved;
                    }
                    Some(Drag::Band { to, from, add, before }) => {
                        *to = (sx + dx, sy + dy);
                        let mut inside = rules::inside(&px, *from, *to);
                        if *add {
                            inside.extend(before.iter().copied());
                        }
                        s.selected = inside;
                    }
                    None => {}
                }
                drop(s);
                chart.area.queue_draw();
            });
        }
        {
            let chart = self.clone();
            drag.connect_drag_end(move |_, _, _| {
                let finished = chart.state.borrow_mut().drag.take();
                if let Some(Drag::Move { start }) = finished {
                    let now = chart.state.borrow().points.clone();
                    chart.announce(start, now);
                }
                chart.area.queue_draw();
            });
        }
        self.area.add_controller(drag);

        let click = gtk::GestureClick::builder().button(0).build();
        {
            let chart = self.clone();
            click.connect_pressed(move |g, presses, x, y| {
                let hit = rules::nearest(&chart.pixels(), x, y, HIT_RADIUS);
                if g.current_button() == 3 {
                    // Right-click removes the point under the pointer.
                    if let Some(i) = hit {
                        let before = chart.state.borrow().points.clone();
                        let after = rules::remove_points(&before, &BTreeSet::from([i]));
                        chart.apply(before, after);
                    }
                } else if g.current_button() == 1 && presses == 2 && hit.is_none() {
                    let (t, _) = chart.plot().value(x, y);
                    let before = chart.state.borrow().points.clone();
                    if let Some((after, at)) = rules::add_point(&before, t) {
                        chart.state.borrow_mut().selected = BTreeSet::from([at]);
                        chart.apply(before, after);
                    }
                }
            });
        }
        self.area.add_controller(click);

        let motion = gtk::EventControllerMotion::new();
        {
            let chart = self.clone();
            motion.connect_motion(move |_, x, y| {
                let hit = rules::nearest(&chart.pixels(), x, y, HIT_RADIUS);
                let tooltip = hit.map(|i| {
                    let p = &chart.state.borrow().points[i];
                    format!("{:.0} °C · {:.0} %", p.temp_c, p.speed_pct)
                });
                let mut s = chart.state.borrow_mut();
                if s.hover != hit {
                    s.hover = hit;
                    drop(s);
                    chart.area.queue_draw();
                }
                if let Some(t) = tooltip {
                    chart.area.set_tooltip_text(Some(&t));
                }
            });
        }
        {
            let chart = self.clone();
            motion.connect_leave(move |_| {
                chart.state.borrow_mut().hover = None;
                chart.area.queue_draw();
            });
        }
        self.area.add_controller(motion);
    }

    fn apply(&self, before: Vec<CurvePoint>, after: Vec<CurvePoint>) {
        self.state.borrow_mut().points = after.clone();
        self.area.queue_draw();
        self.announce(before, after);
    }

    // ---- Keyboard

    fn install_keyboard(&self) {
        let keys = gtk::EventControllerKey::new();
        let chart = self.clone();
        keys.connect_key_pressed(move |_, key, _, state| {
            let big = state.contains(gdk::ModifierType::SHIFT_MASK);
            let step = if big { 5.0 } else { 1.0 };
            let (dt, ds) = match key {
                gdk::Key::Left => (-step, 0.0),
                gdk::Key::Right => (step, 0.0),
                gdk::Key::Up => (0.0, step),
                gdk::Key::Down => (0.0, -step),
                gdk::Key::Delete | gdk::Key::BackSpace => {
                    let before = chart.state.borrow().points.clone();
                    let selected = chart.state.borrow().selected.clone();
                    let after = rules::remove_points(&before, &selected);
                    chart.state.borrow_mut().selected.clear();
                    chart.apply(before, after);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Escape => {
                    chart.state.borrow_mut().selected.clear();
                    chart.area.queue_draw();
                    return glib::Propagation::Stop;
                }
                gdk::Key::a if state.contains(gdk::ModifierType::CONTROL_MASK) => {
                    let n = chart.state.borrow().points.len();
                    chart.state.borrow_mut().selected = (0..n).collect();
                    chart.area.queue_draw();
                    return glib::Propagation::Stop;
                }
                _ => return glib::Propagation::Proceed,
            };
            let before = chart.state.borrow().points.clone();
            let selected = chart.state.borrow().selected.clone();
            if selected.is_empty() {
                return glib::Propagation::Proceed;
            }
            let after = rules::move_points(&before, &selected, dt, ds, true);
            chart.apply(before, after);
            glib::Propagation::Stop
        });
        self.area.add_controller(keys);
    }
}
