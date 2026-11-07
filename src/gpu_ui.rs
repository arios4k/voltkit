//! The GPU Tuner page: live readings with history, what each card's hardware
//! and driver support, and controls for power, fans, temperature and clocks.
//! Changes go through a root helper, are previewed first, and are undone
//! unless confirmed.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::app::toast;

fn tr(text: &str) -> String {
    crate::i18n::tr(text).to_owned()
}
use crate::modules::gpu::{self, Card, Control, Monitor, Sample, Support};
use crate::modules::gpu_profiles::{Auto, CardProfiles};
use crate::modules::gpu_session::{self, Recorder, Session};
use crate::modules::gpu_tune::{self, CurvePoint, FanMode, Sensor, Settings};
use crate::curve_chart::{Axes, CurveChart};
use crate::workspace::Workspace;

const HISTORY: usize = 90;
const POLL: Duration = Duration::from_secs(2);
const CONFIRM_SECS: u32 = 15;

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let cards = Rc::new(gpu::inspect());
    let outer = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
    if cards.is_empty() {
        let status = adw::StatusPage::builder()
            .icon_name("voltkit-gpu-symbolic")
            .title(tr("No graphics cards were found."))
            .description(tr("Voltkit looks for NVIDIA, AMD and Intel cards through the kernel's graphics devices."))
            .vexpand(true)
            .build();
        outer.append(&status);
        return outer.upcast();
    }

    let stack = gtk::Stack::builder().vexpand(true).build();
    let views: Vec<Rc<CardView>> = cards.iter().enumerate().map(|(i, card)| CardView::new(card, toasts, &stack, i)).collect();
    let mut picker = None;
    if cards.len() > 1 {
        let names = gtk::StringList::new(&cards.iter().map(|c| format!("{} ({})", c.gpu.name, c.gpu.vendor.name())).collect::<Vec<_>>().iter().map(String::as_str).collect::<Vec<_>>());
        let picker_row = adw::ComboRow::builder().title(tr("Graphics card")).model(&names).build();
        let stack_ = stack.clone();
        picker_row.connect_selected_notify(move |row| stack_.set_visible_child_name(&row.selected().to_string()));
        let group = adw::PreferencesGroup::new();
        group.add(&picker_row);
        group.set_margin_start(24);
        group.set_margin_end(24);
        group.set_margin_top(6);
        outer.append(&group);
        picker = Some(picker_row);
    }
    outer.append(&stack);
    // Open on the first card Voltkit can tune, not an integrated one.
    let first = cards.iter().position(|c| matches!(c.gpu.vendor, gpu::Vendor::Nvidia | gpu::Vendor::Amd)).unwrap_or(0);
    stack.set_visible_child_name(&first.to_string());
    if let Some(picker) = picker.as_ref() {
        picker.set_selected(first as u32);
    }

    // Read every card on a timer while the page is on screen.
    let monitor = Arc::new(Monitor::new(cards.iter().map(|c| c.gpu.clone()).collect()));
    {
        let (views, outer_w) = (views.clone(), outer.downgrade());
        glib::timeout_add_local(POLL, move || {
            let Some(outer) = outer_w.upgrade() else { return glib::ControlFlow::Break };
            if !outer.is_mapped() {
                return glib::ControlFlow::Continue;
            }
            let (monitor, views) = (monitor.clone(), views.clone());
            glib::spawn_future_local(async move {
                if let Ok(samples) = gio::spawn_blocking(move || monitor.sample()).await {
                    for (view, sample) in views.iter().zip(&samples) {
                        view.update(sample);
                    }
                }
            });
            glib::ControlFlow::Continue
        });
    }
    outer.upcast()
}

// ---------------------------------------------------------------- Sparkline

#[derive(Clone)]
struct Sparkline {
    area: gtk::DrawingArea,
    data: Rc<RefCell<VecDeque<f64>>>,
    ceiling: Rc<Cell<f64>>,
}

impl Sparkline {
    fn new(ceiling: f64) -> Self {
        let data: Rc<RefCell<VecDeque<f64>>> = Rc::new(RefCell::new(VecDeque::new()));
        let ceiling = Rc::new(Cell::new(ceiling));
        let area = gtk::DrawingArea::builder().content_width(120).content_height(28).valign(gtk::Align::Center).build();
        let (d, c) = (data.clone(), ceiling.clone());
        area.set_draw_func(move |area, cr, w, h| {
            let color = area.color();
            cr.set_source_rgba(f64::from(color.red()), f64::from(color.green()), f64::from(color.blue()), 0.9);
            cr.set_line_width(1.5);
            let data = d.borrow();
            // Scale to what was seen, but never magnify tiny wobbles into big swings.
            let low = data.iter().cloned().fold(f64::MAX, f64::min);
            let high = data.iter().cloned().fold(f64::MIN, f64::max);
            let span = (high - low).max(c.get().max(1.0) * 0.1);
            for (i, v) in data.iter().enumerate() {
                let x = f64::from(w) * i as f64 / (HISTORY - 1) as f64;
                let y = f64::from(h) - 2.0 - (f64::from(h) - 4.0) * ((v - low) / span).clamp(0.0, 1.0);
                if i == 0 { cr.move_to(x, y) } else { cr.line_to(x, y) }
            }
            let _ = cr.stroke();
        });
        Sparkline { area, data, ceiling }
    }

    fn push(&self, value: Option<f64>) {
        let mut data = self.data.borrow_mut();
        if let Some(v) = value {
            if v > self.ceiling.get() {
                self.ceiling.set(v);
            }
            if data.len() == HISTORY {
                data.pop_front();
            }
            data.push_back(v);
        }
        drop(data);
        self.area.queue_draw();
    }
}

// ---------------------------------------------------------------- One card

struct Reading {
    row: adw::ActionRow,
    spark: Sparkline,
    value: fn(&Sample) -> Option<f64>,
    text: fn(&Sample) -> String,
}

struct CardView {
    readings: Vec<Reading>,
    chart: Option<CurveChart>,
    profiles: Option<ProfilesView>,
    sessions: SessionsView,
}

#[allow(clippy::type_complexity)]
fn readings_for() -> Vec<(String, f64, fn(&Sample) -> Option<f64>, fn(&Sample) -> String)> {
    fn num(v: Option<f64>, unit: &str) -> String {
        v.map_or_else(|| "—".to_owned(), |v| format!("{v:.0} {unit}"))
    }
    vec![
        (tr("Temperature"), 100.0, |s| s.temp_c, |s| num(s.temp_c, "°C")),
        (tr("Utilization"), 100.0, |s| s.util_pct, |s| num(s.util_pct, "%")),
        (tr("Power"), 50.0, |s| s.power_w, |s| num(s.power_w, "W")),
        (tr("Core clock"), 500.0, |s| s.core_mhz, |s| num(s.core_mhz, "MHz")),
        (tr("Memory clock"), 500.0, |s| s.mem_mhz, |s| num(s.mem_mhz, "MHz")),
        (tr("Fan"), 100.0, |s| s.fan_pct, |s| match (s.fan_pct, s.fan_rpm) {
            (Some(p), Some(r)) => format!("{p:.0} % · {r:.0} RPM"),
            (Some(p), None) => format!("{p:.0} %"),
            (None, Some(r)) => format!("{r:.0} RPM"),
            _ => "—".to_owned(),
        }),
        (tr("Video memory"), 1024.0, |s| s.vram_used_mb, |s| match (s.vram_used_mb, s.vram_total_mb) {
            (Some(u), Some(t)) => format!("{u:.0} / {t:.0} MB"),
            _ => "—".to_owned(),
        }),
    ]
}

impl CardView {
    fn new(card: &Card, toasts: &adw::ToastOverlay, stack: &gtk::Stack, index: usize) -> Rc<Self> {
        let page = Workspace::new();

        let group = adw::PreferencesGroup::builder().title(tr("Right now")).description(card.gpu.name.as_str()).build();
        let readings: Vec<Reading> = readings_for()
            .into_iter()
            .map(|(title, ceiling, value, text)| {
                let spark = Sparkline::new(ceiling);
                let row = adw::ActionRow::builder().title(title).subtitle("—").build();
                row.add_suffix(&spark.area);
                group.add(&row);
                let reading = Reading { row, spark, value, text };
                reading.row.set_subtitle(&(reading.text)(&card.sample));
                if gpu::demo() {
                    // Show a minute and a half of history without waiting for it.
                    for tick in 0..HISTORY as u32 {
                        reading.spark.push((reading.value)(&gpu::demo_sample(index, tick)));
                    }
                }
                reading
            })
            .collect();
        page.add(&group);

        let support = adw::PreferencesGroup::builder()
            .title(tr("What this card supports"))
            .description(tr("Each control is shown as the driver reports it. Voltkit can't go beyond what the hardware, firmware and driver allow."))
            .build();
        for cap in &card.capabilities {
            let (icon, css, text) = match &cap.support {
                Support::Detected(note) => ("emblem-ok-symbolic", "success", note.clone()),
                Support::Unavailable { reason, how_to } => {
                    ("dialog-information-symbolic", "dim-label", how_to.as_ref().map_or(reason.clone(), |h| format!("{reason}\n{h}")))
                }
            };
            let row = adw::ActionRow::builder().title(cap.control.title()).subtitle(glib::markup_escape_text(&text)).subtitle_lines(4).build();
            let image = gtk::Image::from_icon_name(icon);
            image.add_css_class(css);
            row.add_prefix(&image);
            support.add(&row);
        }
        page.add(&support);
        let (tune, chart, collect) = tune_group(card, toasts, gpu::demo());
        page.aside(&tune);
        let profiles = collect.map(|collect| {
            let view = profiles_group(card, collect, toasts);
            page.aside(&view.group);
            view
        });
        let sessions = sessions_group(card, toasts);
        page.aside(&sessions.group);

        stack.add_named(&*page, Some(&index.to_string()));
        if let Some(chart) = &chart {
            chart.set_marker(card.sample.temp_c.zip(card.sample.fan_pct));
        }
        Rc::new(CardView { readings, chart, profiles, sessions })
    }

    fn update(&self, sample: &Sample) {
        self.sessions.push(sample);
        if let Some(p) = &self.profiles {
            p.refresh_active();
        }
        if let Some(chart) = &self.chart {
            chart.set_marker(sample.temp_c.zip(sample.fan_pct));
        }
        for r in &self.readings {
            r.row.set_subtitle(&(r.text)(sample));
            r.spark.push((r.value)(sample));
        }
    }
}

// ---------------------------------------------------------------- Controls

fn supported(card: &Card, control: Control) -> bool {
    card.capabilities.iter().any(|c| c.control == control && matches!(c.support, Support::Detected(_)))
}

fn spin(title: String, subtitle: Option<String>, value: f64, lo: f64, hi: f64, step: f64) -> adw::SpinRow {
    let row = adw::SpinRow::new(Some(&gtk::Adjustment::new(value.clamp(lo, hi), lo, hi, step, step * 5.0, 0.0)), 1.0, 0);
    row.set_title(&title);
    if let Some(s) = subtitle {
        row.set_subtitle(&s);
    }
    row
}

/// A control this card doesn't offer: still listed, greyed out, with the reason.
fn unavailable_row(card: &Card, control: Control) -> adw::ActionRow {
    let text = card.capabilities.iter().find(|c| c.control == control).and_then(|c| match &c.support {
        Support::Unavailable { reason, how_to } => Some(how_to.as_ref().map_or(reason.clone(), |h| format!("{reason}\n{h}"))),
        Support::Detected(_) => None,
    });
    let row = adw::ActionRow::builder()
        .title(control.title())
        .subtitle(glib::markup_escape_text(&text.unwrap_or_else(|| tr("This card doesn't offer this control.").to_string())))
        .subtitle_lines(4)
        .sensitive(false)
        .build();
    let lock = gtk::Image::from_icon_name("changes-prevent-symbolic");
    row.add_prefix(&lock);
    row.add_suffix(&gtk::Switch::builder().valign(gtk::Align::Center).sensitive(false).build());
    row
}

fn expander(title: String, subtitle: String, on: bool) -> adw::ExpanderRow {
    adw::ExpanderRow::builder().title(title).subtitle(subtitle).show_enable_switch(true).enable_expansion(on).expanded(on).build()
}

type Rebuild = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// The undo history of the fan curve being edited.
struct CurveEdit {
    points: Vec<CurvePoint>,
    undo: Vec<Vec<CurvePoint>>,
    redo: Vec<Vec<CurvePoint>>,
}

type Collect = Rc<dyn Fn() -> Settings>;

fn tune_group(card: &Card, toasts: &adw::ToastOverlay, demo: bool) -> (adw::PreferencesGroup, Option<CurveChart>, Option<Collect>) {
    let group = adw::PreferencesGroup::builder()
        .title(tr("Tune"))
        .description(tr("Turn on the controls to change. Preview shows exactly what will happen. Nothing stays unless you confirm it."))
        .build();
    let any = Control::ALL.iter().any(|c| *c != Control::Monitoring && supported(card, *c));
    if !any {
        group.add(&adw::ActionRow::builder().title(tr("Nothing on this card can be changed")).subtitle(tr("See what this card supports for the reasons.")).build());
        for control in Control::ALL.iter().filter(|c| !matches!(c, Control::Monitoring | Control::FanCurve)) {
            group.add(&unavailable_row(card, *control));
        }
        return (group, None, None);
    }
    let experimental = adw::ActionRow::builder().title(tr("Experimental")).subtitle(tr("Changing GPU settings hasn't been confirmed on real hardware for every card yet. Everything you don't confirm is undone, but a wrong voltage can still crash the computer.")).subtitle_lines(6).build();
    let warning = gtk::Image::from_icon_name("dialog-warning-symbolic");
    warning.add_css_class("warning");
    experimental.add_prefix(&warning);
    group.add(&experimental);
    let s = &card.sample;

    let only_vf = demo && std::env::var("VOLTKIT_GPU_DEMO").as_deref() == Ok("vf");
    let power = (supported(card, Control::PowerLimit) && !only_vf).then(|| {
        let (lo, hi) = (s.limit_min_w.unwrap_or(0.0), s.limit_max_w.unwrap_or(0.0));
        let power_on = demo && !matches!(std::env::var("VOLTKIT_GPU_DEMO").as_deref(), Ok("curve") | Ok("vf"));
        let row = expander(tr("Power limit"), trf!("Between {} W and {} W.", format!("{lo:.0}"), format!("{hi:.0}")), power_on);
        let value = spin(tr("Watts"), None, s.limit_w.unwrap_or(hi), lo, hi, 5.0);
        row.add_row(&value);
        if demo {
            value.set_value(s.limit_w.unwrap_or(hi) - 15.0);
        }
        group.add(&row);
        (row, value)
    });

    if !supported(card, Control::PowerLimit) && !only_vf {
        group.add(&unavailable_row(card, Control::PowerLimit));
    }
    let fan = (supported(card, Control::FanSpeed) && !only_vf).then(|| {
        let row = expander(tr("Fan"), tr("Fixed speed, or a curve that follows the temperature."), demo && std::env::var("VOLTKIT_GPU_DEMO").as_deref() != Ok("vf"));
        let modes = gtk::StringList::new(&[&tr("The card's own control"), &tr("Fixed speed"), &tr("Curve")]);
        let mode = adw::ComboRow::builder().title(tr("Mode")).model(&modes).build();
        let fixed = spin(tr("Speed (%)"), None, s.fan_pct.unwrap_or(50.0).max(gpu_tune::MIN_FIXED_FAN_PCT), gpu_tune::MIN_FIXED_FAN_PCT, 100.0, 5.0);
        row.add_row(&mode);
        row.add_row(&fixed);
        // Which temperature the curve follows and which fans it drives, when there's a choice.
        let sensors = Sensor::available(s);
        let sensor_names: Vec<String> = sensors.iter().map(|x| x.title()).collect();
        let sensor_row = adw::ComboRow::builder()
            .title(tr("Temperature source"))
            .subtitle(tr("The reading the curve follows."))
            .model(&gtk::StringList::new(&sensor_names.iter().map(String::as_str).collect::<Vec<_>>()))
            .build();
        row.add_row(&sensor_row);
        let fan_count = card.probe.as_ref().and_then(|p| p.fans.as_ref().ok().map(|f| f.0)).unwrap_or(1);
        let fans_row = adw::ExpanderRow::builder().title(tr("Fans")).subtitle(tr("The fans the curve drives.")).build();
        let fan_checks: Vec<gtk::CheckButton> = (0..fan_count)
            .map(|i| {
                let check = gtk::CheckButton::builder().active(true).valign(gtk::Align::Center).build();
                let r = adw::ActionRow::builder().title(trf!("Fan {}", i + 1)).activatable_widget(&check).build();
                r.add_prefix(&check);
                fans_row.add_row(&r);
                check
            })
            .collect();
        row.add_row(&fans_row);
        let edit = Rc::new(RefCell::new(CurveEdit {
            points: vec![
                CurvePoint { temp_c: 30.0, speed_pct: 25.0 },
                CurvePoint { temp_c: 50.0, speed_pct: 40.0 },
                CurvePoint { temp_c: 70.0, speed_pct: 65.0 },
                CurvePoint { temp_c: 85.0, speed_pct: 100.0 },
            ],
            undo: Vec::new(),
            redo: Vec::new(),
        }));
        let curve_rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::new(RefCell::new(Vec::new()));
        let curve_box = adw::ExpanderRow::builder().title(tr("Curve points")).subtitle(tr("Temperature in °C, fan speed in %.")).expanded(true).build();
        let chart = CurveChart::new(edit.borrow().points.clone(), Axes::fan());
        let chart_row = gtk::Box::builder().margin_top(8).margin_bottom(8).margin_start(12).margin_end(12).build();
        chart_row.append(&chart.area);
        row.add_row(&chart_row);
        row.add_row(&curve_box);
        // Show only the controls of the chosen mode.
        let sync = {
            let (mode, fixed, curve_box, chart_row) = (mode.clone(), fixed.clone(), curve_box.clone(), chart_row.clone());
            let (sensor_row, fans_row, many_sensors, many_fans) = (sensor_row.clone(), fans_row.clone(), sensors.len() > 1, fan_count > 1);
            Rc::new(move || {
                fixed.set_visible(mode.selected() == 1);
                curve_box.set_visible(mode.selected() == 2);
                chart_row.set_visible(mode.selected() == 2);
                sensor_row.set_visible(mode.selected() == 2 && many_sensors);
                fans_row.set_visible(mode.selected() == 2 && many_fans);
            })
        };
        sync();
        {
            let sync = sync.clone();
            mode.connect_selected_notify(move |_| sync());
        }
        let rebuild: Rebuild = Rc::new(RefCell::new(None));
        let build = {
            let (edit, curve_rows, curve_box, rebuild, chart) = (edit.clone(), curve_rows.clone(), curve_box.clone(), rebuild.clone(), chart.clone());
            Rc::new(move || {
                for r in curve_rows.borrow_mut().drain(..) {
                    curve_box.remove(&r);
                }
                let points = edit.borrow().points.clone();
                for (i, p) in points.iter().enumerate() {
                    let r = adw::ActionRow::builder().title(trf!("Point {}", i + 1)).build();
                    let temp = gtk::SpinButton::with_range(0.0, 110.0, 1.0);
                    temp.set_value(p.temp_c);
                    temp.set_valign(gtk::Align::Center);
                    let speed = gtk::SpinButton::with_range(0.0, 100.0, 1.0);
                    speed.set_value(p.speed_pct);
                    speed.set_valign(gtk::Align::Center);
                    let remove = gtk::Button::builder().icon_name("user-trash-symbolic").valign(gtk::Align::Center).css_classes(["flat"]).tooltip_text(tr("Remove")).build();
                    for (spin, is_temp) in [(&temp, true), (&speed, false)] {
                        let (edit, chart) = (edit.clone(), chart.clone());
                        spin.connect_value_changed(move |s| {
                            let mut e = edit.borrow_mut();
                            let before = e.points.clone();
                            if let Some(p) = e.points.get_mut(i) {
                                if is_temp { p.temp_c = s.value() } else { p.speed_pct = s.value() }
                            }
                            e.undo.push(before);
                            e.redo.clear();
                            chart.set_points(e.points.clone());
                        });
                    }
                    {
                        let (edit, rebuild) = (edit.clone(), rebuild.clone());
                        remove.connect_clicked(move |_| {
                            let mut e = edit.borrow_mut();
                            if e.points.len() > 2 {
                                let before = e.points.clone();
                                e.points.remove(i);
                                e.undo.push(before);
                                e.redo.clear();
                                drop(e);
                                if let Some(f) = rebuild.borrow().clone() {
                                    f();
                                }
                            }
                        });
                    }
                    r.add_suffix(&temp);
                    r.add_suffix(&speed);
                    r.add_suffix(&remove);
                    curve_box.add_row(&r);
                    curve_rows.borrow_mut().push(r);
                }
                chart.set_points(points);
            })
        };
        *rebuild.borrow_mut() = Some(build.clone() as Rc<dyn Fn()>);
        build();
        {
            // An edit on the chart is one undo step, like an edit in the rows.
            let (edit, build) = (edit.clone(), build.clone());
            chart.connect_changed(move |before, after| {
                let mut e = edit.borrow_mut();
                e.undo.push(before);
                e.redo.clear();
                e.points = after;
                drop(e);
                build();
            });
        }

        let tools = adw::ActionRow::builder().title(tr("Edit curve")).build();
        let button = |icon: &str, tip: String| gtk::Button::builder().icon_name(icon).tooltip_text(tip).valign(gtk::Align::Center).css_classes(["flat"]).build();
        let (add, undo_b, redo_b, copy, paste) = (
            button("list-add-symbolic", tr("Add point")),
            button("edit-undo-symbolic", tr("Undo")),
            button("edit-redo-symbolic", tr("Redo")),
            button("edit-copy-symbolic", tr("Copy curve")),
            button("edit-paste-symbolic", tr("Paste curve")),
        );
        for b in [&add, &undo_b, &redo_b, &copy, &paste] {
            tools.add_suffix(b);
        }
        curve_box.add_row(&tools);
        {
            let (edit, build) = (edit.clone(), build.clone());
            add.connect_clicked(move |_| {
                let mut e = edit.borrow_mut();
                let before = e.points.clone();
                let last = e.points.last().cloned().unwrap_or(CurvePoint { temp_c: 60.0, speed_pct: 50.0 });
                e.points.push(CurvePoint { temp_c: (last.temp_c + 5.0).min(110.0), speed_pct: last.speed_pct.min(100.0) });
                e.undo.push(before);
                e.redo.clear();
                drop(e);
                build();
            });
        }
        {
            let (edit, build) = (edit.clone(), build.clone());
            undo_b.connect_clicked(move |_| {
                let mut e = edit.borrow_mut();
                if let Some(prev) = e.undo.pop() {
                    let now = std::mem::replace(&mut e.points, prev);
                    e.redo.push(now);
                }
                drop(e);
                build();
            });
        }
        {
            let (edit, build) = (edit.clone(), build.clone());
            redo_b.connect_clicked(move |_| {
                let mut e = edit.borrow_mut();
                if let Some(next) = e.redo.pop() {
                    let now = std::mem::replace(&mut e.points, next);
                    e.undo.push(now);
                }
                drop(e);
                build();
            });
        }
        {
            let (edit, toasts) = (edit.clone(), toasts.clone());
            copy.connect_clicked(move |b| {
                b.clipboard().set_text(&gpu_tune::export_curve(&edit.borrow().points));
                toast(&toasts, tr("Curve copied"));
            });
        }
        {
            let (edit, build, toasts) = (edit.clone(), build.clone(), toasts.clone());
            paste.connect_clicked(move |b| {
                let (edit, build, toasts) = (edit.clone(), build.clone(), toasts.clone());
                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    b,
                    async move {
                        let text = b.clipboard().read_text_future().await.ok().flatten().map(|t| t.to_string()).unwrap_or_default();
                        match gpu_tune::import_curve(&text) {
                            Ok(points) => {
                                let mut e = edit.borrow_mut();
                                let before = std::mem::replace(&mut e.points, points);
                                e.undo.push(before);
                                e.redo.clear();
                                drop(e);
                                build();
                            }
                            Err(err) => toast(&toasts, err.to_string()),
                        }
                    }
                ));
            });
        }
        if demo {
            mode.set_selected(2);
            sync();
        }
        group.add(&row);
        (row, mode, fixed, edit, chart, sensor_row, sensors, fan_checks)
    });

    if !supported(card, Control::FanSpeed) && !only_vf {
        group.add(&unavailable_row(card, Control::FanSpeed));
    }
    let target = (supported(card, Control::TemperatureTarget) && !only_vf).then(|| {
        let row = expander(tr("Temperature target"), tr("The temperature the card's fan control aims for."), false);
        let value = spin(tr("Degrees (°C)"), None, 75.0, 40.0, 100.0, 1.0);
        row.add_row(&value);
        group.add(&row);
        (row, value)
    });

    if !supported(card, Control::TemperatureTarget) && !only_vf {
        group.add(&unavailable_row(card, Control::TemperatureTarget));
    }
    let clocks = (supported(card, Control::ClockOffsets) && !only_vf).then(|| {
        let row = expander(tr("Clock offsets"), tr("Raise or lower the clocks from their defaults."), false);
        let (core, mem) = (spin(tr("Core (MHz)"), None, 0.0, -500.0, 500.0, 5.0), spin(tr("Memory (MHz)"), None, 0.0, -1000.0, 1000.0, 10.0));
        row.add_row(&core);
        row.add_row(&mem);
        group.add(&row);
        (row, core, mem)
    });

    if !supported(card, Control::ClockOffsets) && !only_vf {
        group.add(&unavailable_row(card, Control::ClockOffsets));
    }
    let limits = supported(card, Control::ClockLimits).then(|| {
        let (lo, hi) = crate::modules::gpu::intel_freq(&card.gpu).map_or((0.0, 4000.0), |f| {
            let num = |p: &Option<std::path::PathBuf>| p.as_deref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|v| v.trim().parse::<f64>().ok());
            (num(&f.rpn).unwrap_or(0.0), num(&f.rp0).unwrap_or(4000.0))
        });
        let row = expander(tr("Clock limits"), trf!("Keep the clock between a minimum and a maximum, from {} to {} MHz.", format!("{lo:.0}"), format!("{hi:.0}")), false);
        let (min, max) = (spin(tr("Minimum (MHz)"), None, lo, lo, hi, 50.0), spin(tr("Maximum (MHz)"), None, hi, lo, hi, 50.0));
        row.add_row(&min);
        row.add_row(&max);
        group.add(&row);
        (row, min, max)
    });

    if !supported(card, Control::ClockLimits) && !only_vf {
        group.add(&unavailable_row(card, Control::ClockLimits));
    }
    // Cards with a full table get a point per clock; the others, a single offset.
    let od = if demo { Some(gpu::DEMO_VF.to_owned()) } else { std::fs::read_to_string(card.gpu.device.join("pp_od_clk_voltage")).ok() };
    let vf_table = od.and_then(|t| gpu_tune::parse_vf_table(&t));
    let vf = vf_table.filter(|_| supported(card, Control::VoltageCurve)).map(|table| {
        let on = demo && std::env::var("VOLTKIT_GPU_DEMO").as_deref() == Ok("vf");
        let row = expander(tr("Voltage and frequency curve"), tr("Set the voltage for each clock the card runs at. Lower voltages run cooler but can crash the card if too low."), on);
        // The chart and the number rows edit the same points.
        let as_points = |pts: &[gpu_tune::VfPoint]| -> Vec<CurvePoint> { pts.iter().map(|p| CurvePoint { temp_c: f64::from(p.mhz), speed_pct: f64::from(p.mv) }).collect() };
        let chart = CurveChart::new(as_points(&table.points), Axes::voltage(table.limits.clone()));
        let chart_row = gtk::Box::builder().margin_top(8).margin_bottom(8).margin_start(12).margin_end(12).build();
        chart_row.append(&chart.area);
        row.add_row(&chart_row);
        let spins: Vec<(gtk::SpinButton, gtk::SpinButton)> = table
            .points
            .iter()
            .zip(&table.limits)
            .enumerate()
            .map(|(i, (p, ((f_lo, f_hi), (v_lo, v_hi))))| {
                let r = adw::ActionRow::builder().title(trf!("Point {}", i + 1)).subtitle(tr("MHz · mV")).build();
                let mhz = gtk::SpinButton::with_range(*f_lo, *f_hi, 10.0);
                mhz.set_value(f64::from(p.mhz));
                mhz.set_valign(gtk::Align::Center);
                let mv = gtk::SpinButton::with_range(*v_lo, *v_hi, 5.0);
                mv.set_value(f64::from(p.mv));
                mv.set_valign(gtk::Align::Center);
                r.add_suffix(&mhz);
                r.add_suffix(&mv);
                row.add_row(&r);
                (mhz, mv)
            })
            .collect();
        let syncing = Rc::new(Cell::new(false));
        {
            // Dragging on the chart moves the number rows.
            let (spins, syncing) = (spins.clone(), syncing.clone());
            chart.connect_changed(move |_, after| {
                syncing.set(true);
                for ((mhz, mv), p) in spins.iter().zip(&after) {
                    mhz.set_value(p.temp_c);
                    mv.set_value(p.speed_pct);
                }
                syncing.set(false);
            });
        }
        for (mhz, mv) in &spins {
            // Typing a number moves the chart.
            for spin in [mhz, mv] {
                let (spins, chart, syncing) = (spins.clone(), chart.clone(), syncing.clone());
                spin.connect_value_changed(move |_| {
                    if !syncing.get() {
                        chart.set_points(spins.iter().map(|(f, v)| CurvePoint { temp_c: f.value(), speed_pct: v.value() }).collect());
                    }
                });
            }
        }
        group.add(&row);
        (row, spins)
    });
    let vf_present = vf.is_some();

    if !supported(card, Control::VoltageCurve) && !only_vf {
        group.add(&unavailable_row(card, Control::VoltageCurve));
    }
    let voltage = (supported(card, Control::VoltageCurve) && !vf_present).then(|| {
        let row = expander(tr("Voltage offset"), tr("Lower the voltage for less heat, or raise it for stability."), false);
        let value = spin(tr("Millivolts (mV)"), None, 0.0, -200.0, 0.0, 5.0);
        row.add_row(&value);
        group.add(&row);
        (row, value)
    });

    let keep = adw::SwitchRow::builder().title(tr("Keep after restart")).subtitle(tr("Saves these settings and applies them at startup. Fan curves need this.")).build();
    group.add(&keep);

    let chart_out = fan.as_ref().map(|f| f.4.clone());
    let collect = {
        let address = card.gpu.address.clone();
        Rc::new(move || {
            let mut s = Settings { gpu: address.clone(), ..Settings::default() };
            if let Some((row, v)) = &power
                && row.enables_expansion()
            {
                s.power_limit_w = Some(v.value());
            }
            if let Some((row, mode, fixed, edit, _, sensor_row, sensors, fan_checks)) = &fan
                && row.enables_expansion()
            {
                s.fan = Some(match mode.selected() {
                    0 => FanMode::Auto,
                    1 => FanMode::Fixed { speed_pct: fixed.value() },
                    _ => {
                        let sensor = sensors.get(sensor_row.selected() as usize).copied().unwrap_or_default();
                        let chosen: Vec<u32> = fan_checks.iter().enumerate().filter(|(_, c)| c.is_active()).map(|(i, _)| i as u32).collect();
                        // All fans, or none ticked, mean every fan.
                        let fans = if chosen.len() == fan_checks.len() { Vec::new() } else { chosen };
                        FanMode::Curve { points: edit.borrow().points.clone(), sensor, fans }
                    }
                });
            }
            if let Some((row, v)) = &target
                && row.enables_expansion()
            {
                s.temperature_target_c = Some(v.value());
            }
            if let Some((row, core, mem)) = &clocks
                && row.enables_expansion()
            {
                s.core_offset_mhz = Some(core.value() as i32);
                s.mem_offset_mhz = Some(mem.value() as i32);
            }
            if let Some((row, min, max)) = &limits
                && row.enables_expansion()
            {
                s.core_clock_min_mhz = Some(min.value() as u32);
                s.core_clock_max_mhz = Some(max.value() as u32);
            }
            if let Some((row, spins)) = &vf
                && row.enables_expansion()
            {
                s.voltage_curve = Some(spins.iter().map(|(f, v)| gpu_tune::VfPoint { mhz: f.value() as u32, mv: v.value() as u32 }).collect());
            }
            if let Some((row, v)) = &voltage
                && row.enables_expansion()
            {
                s.voltage_offset_mv = Some(v.value() as i32);
            }
            s
        })
    };

    let buttons = gtk::Box::builder().spacing(8).halign(gtk::Align::End).margin_top(8).build();
    let preview = gtk::Button::with_label(&tr("Preview"));
    let apply = gtk::Button::builder().label(tr("Apply")).css_classes(["suggested-action"]).build();
    buttons.append(&preview);
    buttons.append(&apply);
    let holder = gtk::Box::new(gtk::Orientation::Vertical, 0);
    holder.append(&buttons);
    group.add(&holder);

    {
        let (card, collect) = (card.clone(), collect.clone());
        preview.connect_clicked(move |b| {
            let settings = collect();
            let (heading, body) = match gpu_tune::plan(&card, &settings) {
                Ok(plan) if plan.preview().is_empty() => (tr("Nothing to change"), tr("Turn on at least one control.")),
                Ok(plan) => (tr("What will change"), plan.preview().join("\n")),
                Err(e) => (tr("This can't be applied"), e.to_string()),
            };
            let dialog = adw::AlertDialog::new(Some(&heading), Some(&body));
            dialog.add_response("ok", &tr("Close"));
            dialog.present(Some(b));
        });
    }
    {
        let (card, collect, toasts, keep) = (card.clone(), collect.clone(), toasts.clone(), keep.clone());
        apply.connect_clicked(move |b| {
            let settings = collect();
            match gpu_tune::plan(&card, &settings) {
                Ok(plan) if plan.preview().is_empty() => toast(&toasts, tr("Turn on at least one control.")),
                Ok(plan) if plan.software_curve.is_some() && !keep.is_active() => {
                    toast(&toasts, tr("This card has no built-in fan curve, so Voltkit's background service follows it. Turn on Keep after restart."));
                }
                Ok(_) => run_helper(b, &settings, keep.is_active(), &toasts),
                Err(e) => toast(&toasts, e.to_string()),
            }
        });
    }
    (group, chart_out, Some(collect as Collect))
}

// ---------------------------------------------------------------- Applying

enum Message {
    Line(String),
    Done(bool, String),
}

/// Starts the root helper, shows its confirmation countdown, and tells it
/// to keep or undo the change.
fn run_helper(anchor: &gtk::Button, settings: &Settings, save: bool, toasts: &adw::ToastOverlay) {
    let Ok(exe) = std::env::current_exe() else { return };
    let mut command = Command::new("pkexec");
    command.arg(exe).args(["gpu", "apply", "-", "--session"]);
    if save {
        command.arg("--save");
    }
    let Ok(mut child) = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() else {
        toast(toasts, tr("pkexec is needed to change GPU settings."));
        return;
    };
    let stdin = Arc::new(Mutex::new(child.stdin.take()));
    if let (Some(pipe), Ok(json)) = (stdin.lock().unwrap().as_mut(), serde_json::to_string(settings)) {
        let _ = writeln!(pipe, "{json}");
    }
    let (tx, rx) = mpsc::channel();
    if let Some(out) = child.stdout.take() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let _ = tx.send(Message::Line(line));
            }
        });
    }
    let errors = child.stderr.take();
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(e) = errors {
            let _ = std::io::Read::read_to_string(&mut BufReader::new(e), &mut text);
        }
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        let _ = tx.send(Message::Done(ok, text));
    });

    let (anchor, toasts) = (anchor.downgrade(), toasts.clone());
    let dialog: Rc<RefCell<Option<adw::AlertDialog>>> = Rc::new(RefCell::new(None));
    glib::timeout_add_local(Duration::from_millis(150), move || {
        while let Ok(message) = rx.try_recv() {
            match message {
                Message::Line(line) if line == "applied" => {
                    let Some(anchor) = anchor.upgrade() else { continue };
                    let d = adw::AlertDialog::new(Some(&tr("Keep these settings?")), None);
                    d.add_responses(&[("revert", &tr("Revert")), ("keep", &tr("Keep"))]);
                    d.set_response_appearance("keep", adw::ResponseAppearance::Suggested);
                    d.set_default_response(Some("revert"));
                    d.set_close_response("revert");
                    let left = Rc::new(Cell::new(CONFIRM_SECS));
                    d.set_body(&trf!("If you don't confirm, the card goes back to how it was in {} seconds.", left.get()));
                    {
                        let (d, left, stdin) = (d.clone(), left.clone(), stdin.clone());
                        glib::timeout_add_seconds_local(1, move || {
                            left.set(left.get().saturating_sub(1));
                            if left.get() == 0 {
                                d.force_close();
                                return glib::ControlFlow::Break;
                            }
                            d.set_body(&trf!("If you don't confirm, the card goes back to how it was in {} seconds.", left.get()));
                            let _ = &stdin;
                            glib::ControlFlow::Continue
                        });
                    }
                    let stdin = stdin.clone();
                    d.connect_response(None, move |_, response| {
                        if let Some(pipe) = stdin.lock().unwrap().as_mut() {
                            // Anything but "keep" makes the helper undo the change.
                            let _ = writeln!(pipe, "{}", if response == "keep" { "keep" } else { "" });
                            let _ = pipe.flush();
                        }
                    });
                    d.present(Some(&anchor));
                    *dialog.borrow_mut() = Some(d);
                }
                Message::Line(line) if line == "kept" => toast(&toasts, tr("Settings applied and kept.")),
                Message::Line(line) if line == "reverted" => toast(&toasts, tr("Not confirmed, so the change was undone.")),
                Message::Line(_) => {}
                Message::Done(ok, errors) => {
                    if !ok && !errors.trim().is_empty() {
                        toast(&toasts, errors.trim().lines().last().unwrap_or_default());
                    } else if !ok {
                        toast(&toasts, tr("Cancelled, nothing was changed."));
                    }
                    return glib::ControlFlow::Break;
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

// ---------------------------------------------------------------- Profiles

fn profiles_file() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("voltkit/gpu-profiles.json"))
}

fn load_profiles() -> Vec<CardProfiles> {
    profiles_file().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save_profiles(sets: &[CardProfiles]) {
    if let Some(p) = profiles_file() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(sets) {
            let _ = std::fs::write(p, text);
        }
    }
}

/// A profile's settings in a few words.
fn summarize(s: &Settings) -> String {
    let mut parts = Vec::new();
    if let Some(w) = s.power_limit_w {
        parts.push(trf!("Power limit {} W", format!("{w:.0}")));
    }
    match &s.fan {
        Some(FanMode::Auto) => parts.push(tr("Fan: the card's own control")),
        Some(FanMode::Fixed { speed_pct }) => parts.push(trf!("Fan {} %", format!("{speed_pct:.0}"))),
        Some(FanMode::Curve { .. }) => parts.push(tr("Fan curve")),
        None => {}
    }
    if let Some(t) = s.temperature_target_c {
        parts.push(trf!("Target {} °C", format!("{t:.0}")));
    }
    if let Some(c) = s.core_offset_mhz.filter(|c| *c != 0) {
        parts.push(trf!("Core {} MHz", format!("{c:+}")));
    }
    if let Some(m) = s.mem_offset_mhz.filter(|m| *m != 0) {
        parts.push(trf!("Memory {} MHz", format!("{m:+}")));
    }
    if let (Some(lo), Some(hi)) = (s.core_clock_min_mhz, s.core_clock_max_mhz) {
        parts.push(trf!("Clock {}–{} MHz", lo, hi));
    }
    if parts.is_empty() { tr("No changes") } else { parts.join(" · ") }
}

struct ProfilesView {
    group: adw::PreferencesGroup,
    address: String,
    plain: String,
}

impl ProfilesView {
    /// Shows which profile the background service has switched this card to.
    fn refresh_active(&self) {
        let map: std::collections::BTreeMap<String, String> =
            std::fs::read_to_string(crate::gpu_cli::ACTIVE_PATH).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        match map.get(&self.address) {
            Some(name) => self.group.set_description(Some(&trf!("In use now: {}", name))),
            None => self.group.set_description(Some(&self.plain)),
        }
    }
}

fn profiles_group(card: &Card, collect: Collect, toasts: &adw::ToastOverlay) -> ProfilesView {
    let plain = tr("Save settings as named profiles and let Voltkit switch between them: on battery, while an app runs, under load, or when the card runs hot.");
    let group = adw::PreferencesGroup::builder().title(tr("Profiles")).description(plain.as_str()).build();
    let address = card.gpu.address.clone();
    let mut loaded = if gpu::demo() { None } else { load_profiles().into_iter().find(|s| s.gpu.eq_ignore_ascii_case(&address)) }
        .unwrap_or(CardProfiles { gpu: address.clone(), ..CardProfiles::default() });
    if gpu::demo() {
        // Made-up profiles for the README screenshots.
        let base = Settings { gpu: address.clone(), ..Settings::default() };
        loaded.save_profile("Quiet", Settings { power_limit_w: Some(220.0), fan: Some(FanMode::Fixed { speed_pct: 35.0 }), ..base.clone() });
        loaded.save_profile("Balanced", Settings { power_limit_w: Some(263.0), temperature_target_c: Some(75.0), ..base.clone() });
        loaded.save_profile("Performance", Settings { power_limit_w: Some(289.0), core_offset_mhz: Some(100), ..base });
        loaded.default = Some("Balanced".into());
        loaded.set_auto("Quiet", Auto::OnBattery);
        loaded.set_auto("Performance", Auto::App(vec!["blender".into(), "steam".into()]));
    }
    let state = Rc::new(RefCell::new(loaded));
    let rows: Rc<RefCell<Vec<adw::ExpanderRow>>> = Rc::new(RefCell::new(Vec::new()));
    let render: Rebuild = Rc::new(RefCell::new(None));

    let build = {
        let (group, state, rows, render) = (group.clone(), state.clone(), rows.clone(), render.clone());
        Rc::new(move || {
            for r in rows.borrow_mut().drain(..) {
                group.remove(&r);
            }
            let set = state.borrow().clone();
            for profile in &set.profiles {
                let name = profile.name.clone();
                let row = adw::ExpanderRow::builder().title(glib::markup_escape_text(&name)).subtitle(glib::markup_escape_text(&summarize(&profile.settings))).build();
                let kinds = gtk::StringList::new(&[
                    &tr("Not automatic"),
                    &tr("On battery"),
                    &tr("On mains power"),
                    &tr("While an app runs"),
                    &tr("When the GPU is busy"),
                    &tr("When the GPU runs hot"),
                ]);
                let auto = set.auto_of(&name);
                let kind = adw::ComboRow::builder()
                    .title(tr("Switch to it"))
                    .model(&kinds)
                    .selected(match &auto {
                        Auto::Never => 0,
                        Auto::OnBattery => 1,
                        Auto::OnMains => 2,
                        Auto::App(_) => 3,
                        Auto::Busy(_) => 4,
                        Auto::Hot(_) => 5,
                    })
                    .build();
                let apps = adw::EntryRow::builder().title(tr("Programs, separated by commas")).build();
                let busy = spin(tr("Busier than (%)"), None, if let Auto::Busy(v) = auto { v } else { 80.0 }, 10.0, 100.0, 5.0);
                let hot = spin(tr("Hotter than (°C)"), None, if let Auto::Hot(v) = auto { v } else { 85.0 }, 50.0, 100.0, 1.0);
                if let Auto::App(names) = &auto {
                    apps.set_text(&names.join(", "));
                }
                let show = {
                    let (kind, apps, busy, hot) = (kind.clone(), apps.clone(), busy.clone(), hot.clone());
                    Rc::new(move || {
                        apps.set_visible(kind.selected() == 3);
                        busy.set_visible(kind.selected() == 4);
                        hot.set_visible(kind.selected() == 5);
                    })
                };
                show();
                let update = {
                    let (state, name, kind, apps, busy, hot, show) = (state.clone(), name.clone(), kind.clone(), apps.clone(), busy.clone(), hot.clone(), show.clone());
                    Rc::new(move || {
                        show();
                        let auto = match kind.selected() {
                            1 => Auto::OnBattery,
                            2 => Auto::OnMains,
                            3 => Auto::App(apps.text().split(',').map(|n| n.trim().to_owned()).filter(|n| !n.is_empty()).collect()),
                            4 => Auto::Busy(busy.value()),
                            5 => Auto::Hot(hot.value()),
                            _ => Auto::Never,
                        };
                        state.borrow_mut().set_auto(&name, auto);
                    })
                };
                {
                    let update = update.clone();
                    kind.connect_selected_notify(move |_| update());
                }
                {
                    let update = update.clone();
                    apps.connect_changed(move |_| update());
                }
                {
                    let update = update.clone();
                    busy.connect_value_notify(move |_| update());
                }
                {
                    let update = update.clone();
                    hot.connect_value_notify(move |_| update());
                }
                let default = adw::SwitchRow::builder().title(tr("Use when no rule applies")).active(set.default.as_deref() == Some(name.as_str())).build();
                {
                    let (state, name, render) = (state.clone(), name.clone(), render.clone());
                    default.connect_active_notify(move |s| {
                        let changed = {
                            let mut st = state.borrow_mut();
                            let is = st.default.as_deref() == Some(name.as_str());
                            if s.is_active() && !is {
                                st.default = Some(name.clone());
                                true
                            } else if !s.is_active() && is {
                                st.default = None;
                                true
                            } else {
                                false
                            }
                        };
                        if changed && let Some(f) = render.borrow().clone() {
                            glib::idle_add_local_once(move || f());
                        }
                    });
                }
                let remove = adw::ActionRow::builder().title(tr("Delete this profile")).activatable(true).build();
                remove.add_prefix(&gtk::Image::from_icon_name("user-trash-symbolic"));
                {
                    let (state, name, render) = (state.clone(), name.clone(), render.clone());
                    remove.connect_activated(move |_| {
                        state.borrow_mut().remove_profile(&name);
                        if let Some(f) = render.borrow().clone() {
                            glib::idle_add_local_once(move || f());
                        }
                    });
                }
                for w in [kind.upcast_ref::<gtk::Widget>(), apps.upcast_ref(), busy.upcast_ref(), hot.upcast_ref(), default.upcast_ref(), remove.upcast_ref()] {
                    row.add_row(w);
                }
                group.add(&row);
                rows.borrow_mut().push(row);
            }
        })
    };
    *render.borrow_mut() = Some(build.clone() as Rc<dyn Fn()>);
    build();

    let buttons = gtk::Box::builder().spacing(8).halign(gtk::Align::End).margin_top(8).build();
    let save = gtk::Button::with_label(&tr("Save current settings as a profile…"));
    let apply = gtk::Button::builder().label(tr("Apply profiles")).css_classes(["suggested-action"]).tooltip_text(tr("Installs the profiles in Voltkit's background service, which switches between them.")).build();
    buttons.append(&save);
    buttons.append(&apply);
    group.add(&buttons);
    {
        let (state, collect, render) = (state.clone(), collect.clone(), render.clone());
        save.connect_clicked(move |b| {
            let entry = gtk::Entry::builder().placeholder_text(tr("Name, for example Quiet")).activates_default(true).build();
            let dialog = adw::AlertDialog::new(Some(&tr("Save as a profile")), Some(&tr("The settings now turned on in Tune are stored under this name.")));
            dialog.set_extra_child(Some(&entry));
            dialog.add_responses(&[("cancel", &tr("Cancel")), ("save", &tr("Save"))]);
            dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("save"));
            let (state, collect, render) = (state.clone(), collect.clone(), render.clone());
            dialog.connect_response(None, move |_, response| {
                let name = entry.text().trim().to_owned();
                if response == "save" && !name.is_empty() {
                    state.borrow_mut().save_profile(&name, collect());
                    if let Some(f) = render.borrow().clone() {
                        f();
                    }
                }
            });
            dialog.present(Some(b));
        });
    }
    {
        let (state, toasts, address) = (state.clone(), toasts.clone(), address.clone());
        apply.connect_clicked(move |_| {
            let mut all: Vec<CardProfiles> = load_profiles().into_iter().filter(|s| !s.gpu.eq_ignore_ascii_case(&address)).collect();
            let mine = state.borrow().clone();
            if let Err(e) = mine.check() {
                toast(&toasts, e);
                return;
            }
            all.push(mine);
            save_profiles(&all);
            let toasts = toasts.clone();
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || install_profiles(&all)).await;
                match result {
                    Ok(Ok(())) => toast(&toasts, tr("Profiles installed.")),
                    Ok(Err(e)) => toast(&toasts, e),
                    Err(_) => toast(&toasts, tr("Cancelled, nothing was changed.")),
                }
            });
        });
    }
    ProfilesView { group, address, plain }
}

/// Hands the profiles to the root helper, which checks and installs them.
fn install_profiles(all: &[CardProfiles]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = Command::new("pkexec")
        .arg(exe)
        .args(["gpu", "profiles", "install", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| tr("pkexec is needed to change GPU settings."))?;
    if let Some(mut pipe) = child.stdin.take() {
        let json = serde_json::to_string(all).map_err(|e| e.to_string())?;
        let _ = pipe.write_all(json.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        let text = String::from_utf8_lossy(&out.stderr);
        Err(text.trim().lines().last().map_or_else(|| tr("Cancelled, nothing was changed."), str::to_owned))
    }
}

// ---------------------------------------------------------------- Sessions

type Recording = Rc<RefCell<Option<(Recorder, std::time::Instant)>>>;

struct SessionsView {
    group: adw::PreferencesGroup,
    recording: Recording,
}

impl SessionsView {
    /// Adds a reading to the session being recorded, if one is.
    fn push(&self, sample: &Sample) {
        if let Some((recorder, began)) = self.recording.borrow_mut().as_mut() {
            recorder.push(began.elapsed().as_secs_f64(), sample.clone());
        }
    }
}

/// Two made-up sessions for the screenshots: stock settings and an undervolt.
fn demo_sessions() -> Vec<Session> {
    let make = |label: &str, power: f64, clock: f64, temp: f64| {
        let mut r = Recorder::new(label, "0000:03:00.0", "2026-11-06T20:00:00");
        for t in 0..=120 {
            let wobble = (f64::from(t) / 7.0).sin();
            let s = Sample {
                temp_c: Some(temp + wobble * 2.0),
                util_pct: Some(97.0),
                core_mhz: Some(clock + wobble * 20.0),
                power_w: Some(power + wobble * 6.0),
                fan_pct: Some(temp - 18.0),
                ..Sample::default()
            };
            r.push(f64::from(t), s);
        }
        r.finish()
    };
    vec![make("Stock", 258.0, 2480.0, 76.0), make("Undervolted", 206.0, 2440.0, 68.0)]
}

fn show_comparison(anchor: &impl IsA<gtk::Widget>, a: &Session, b: &Session) {
    let lines = gpu_session::compare(&gpu_session::stats(a), &gpu_session::stats(b));
    let tally = gpu_session::tally(&lines);
    let wrapped = |text: String, css: &[&str]| {
        gtk::Label::builder().label(text).wrap(true).xalign(0.0).hexpand(true).css_classes(css.to_vec()).build()
    };

    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12)
        .margin_top(12).margin_bottom(18).margin_start(18).margin_end(18).build();
    content.append(&wrapped(trf!("{} compared with {}", a.label.clone(), b.label.clone()), &["title-3"]));
    if !lines.is_empty() {
        let verdict = match tally.leader() {
            Some(b_leads) => {
                let (name, wins) = if b_leads { (b.label.clone(), tally.b) } else { (a.label.clone(), tally.a) };
                trf!("{} is better on {} of {} measures.", name, wins, lines.len())
            }
            None => tr("Neither session is clearly better: they win the same number of measures.").to_string(),
        };
        content.append(&wrapped(verdict, &["heading"]));
    }
    content.append(&wrapped(tr("The ▶ marks the better session. Active MHz per watt is a proxy for work per watt, not frames.").to_string(), &["dim-label"]));

    let group = adw::PreferencesGroup::new();
    for l in &lines {
        let change = l.change_pct().map_or_else(String::new, |c| format!("  ({c:+.0} %)"));
        let row = adw::ActionRow::builder().title(tr(l.label)).subtitle(format!("{:.1} → {:.1} {}{change}", l.a, l.b, l.unit)).build();
        let (text, css) = match l.b_wins() {
            Some(true) => (format!("{} ▶", b.label), "success"),
            Some(false) => (format!("◀ {}", a.label), "success"),
            None => (tr("Same").to_string(), "dim-label"),
        };
        row.add_suffix(&gtk::Label::builder().label(text).ellipsize(gtk::pango::EllipsizeMode::End).max_width_chars(28).justify(gtk::Justification::Right)
            .css_classes([css]).valign(gtk::Align::Center).build());
        group.add(&row);
    }
    if lines.is_empty() {
        group.add(&adw::ActionRow::builder().title(tr("These sessions have no readings in common.")).build());
    }
    content.append(&group);

    let scroll = gtk::ScrolledWindow::builder().child(&content).vexpand(true).hscrollbar_policy(gtk::PolicyType::Never).build();
    let dialog = adw::Dialog::builder().title(tr("Compare sessions")).content_width(760).content_height(640).build();
    let close = gtk::Button::builder().label(tr("Close")).halign(gtk::Align::End).css_classes(["suggested-action"]).build();
    close.connect_clicked({
        let dialog = dialog.clone();
        move |_| {
            dialog.close();
        }
    });
    let bar = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).margin_top(8).margin_bottom(8).margin_start(18).margin_end(18).build();
    bar.append(&close);
    bar.set_halign(gtk::Align::End);
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&scroll));
    view.add_bottom_bar(&bar);
    dialog.set_child(Some(&view));
    dialog.present(Some(anchor));
}

fn sessions_group(card: &Card, toasts: &adw::ToastOverlay) -> SessionsView {
    let group = adw::PreferencesGroup::builder()
        .title(tr("Sessions"))
        .description(tr("Record how a profile behaves, then compare two sessions side by side: temperature, power, energy and efficiency."))
        .build();
    let recording: Recording = Rc::new(RefCell::new(None));
    let label = adw::EntryRow::builder().title(tr("Label")).text(tr("Session")).build();
    let record = gtk::Button::builder().label(tr("Start recording")).valign(gtk::Align::Center).build();
    label.add_suffix(&record);
    group.add(&label);

    let rows: Rc<RefCell<Vec<(adw::ActionRow, gtk::CheckButton, Session)>>> = Rc::new(RefCell::new(Vec::new()));
    let shown: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::new(RefCell::new(Vec::new()));
    let compare = gtk::Button::builder().label(tr("Compare the two ticked")).halign(gtk::Align::End).margin_top(8).sensitive(false).build();
    let refresh: Rebuild = Rc::new(RefCell::new(None));
    let rebuild = {
        let (group, rows, shown, compare, refresh) = (group.clone(), rows.clone(), shown.clone(), compare.clone(), refresh.clone());
        Rc::new(move || {
            for r in shown.borrow_mut().drain(..) {
                group.remove(&r);
            }
            rows.borrow_mut().clear();
            let sessions: Vec<Session> = if gpu::demo() {
                demo_sessions()
            } else {
                crate::gpu_cli::sessions_dir()
                    .as_deref()
                    .map(gpu_session::list)
                    .unwrap_or_default()
                    .iter()
                    .take(8)
                    .filter_map(|p| gpu_session::load(p).ok())
                    .collect()
            };
            for s in sessions {
                let st = gpu_session::stats(&s);
                let power = st.avg_power_w.map_or_else(String::new, |p| format!(" · {p:.0} W"));
                let row = adw::ActionRow::builder().title(glib::markup_escape_text(&s.label)).subtitle(format!("{} · {:.0} s{power}", s.started.replace('T', " "), st.seconds)).build();
                let check = gtk::CheckButton::builder().valign(gtk::Align::Center).build();
                row.add_prefix(&check);
                row.set_activatable_widget(Some(&check));
                {
                    let (rows, compare) = (rows.clone(), compare.clone());
                    check.connect_toggled(move |_| {
                        let ticked = rows.borrow().iter().filter(|(_, c, _)| c.is_active()).count();
                        compare.set_sensitive(ticked == 2);
                    });
                }
                group.add(&row);
                shown.borrow_mut().push(row.clone());
                rows.borrow_mut().push((row, check, s));
            }
            let _ = &refresh;
        })
    };
    *refresh.borrow_mut() = Some(rebuild.clone() as Rc<dyn Fn()>);
    rebuild();
    group.add(&compare);
    if std::env::var("VOLTKIT_GPU_DEMO").as_deref() == Ok("compare") {
        // For checking the comparison window: tick both demo sessions and open it.
        let (rows, compare) = (rows.clone(), compare.clone());
        glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
            for (_, check, _) in rows.borrow().iter().take(2) {
                check.set_active(true);
            }
            compare.emit_clicked();
        });
    }
    {
        let rows = rows.clone();
        compare.connect_clicked(move |b| {
            let picked: Vec<Session> = rows.borrow().iter().filter(|(_, c, _)| c.is_active()).map(|(_, _, s)| s.clone()).collect();
            if let [a, c] = picked.as_slice() {
                // Older first, so "after" is the newer try.
                let (a, c) = if a.started <= c.started { (a, c) } else { (c, a) };
                show_comparison(b, a, c);
            }
        });
    }
    {
        let (recording, toasts, label, rebuild) = (recording.clone(), toasts.clone(), label.clone(), rebuild.clone());
        let address = card.gpu.address.clone();
        record.connect_clicked(move |b| {
            let finished = recording.borrow_mut().take();
            match finished {
                None => {
                    let started = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
                    let name = label.text().trim().to_owned();
                    *recording.borrow_mut() = Some((Recorder::new(if name.is_empty() { "session" } else { &name }, &address, &started), std::time::Instant::now()));
                    b.set_label(&tr("Stop and save"));
                    b.add_css_class("destructive-action");
                }
                Some((recorder, _)) => {
                    b.set_label(&tr("Start recording"));
                    b.remove_css_class("destructive-action");
                    let session = recorder.finish();
                    match crate::gpu_cli::sessions_dir().map(|d| gpu_session::save(&d, &session)) {
                        Some(Ok(_)) if !gpu::demo() => {
                            toast(&toasts, tr("Session saved."));
                            rebuild();
                        }
                        Some(Err(e)) => toast(&toasts, e.to_string()),
                        _ => {}
                    }
                }
            }
        });
    }
    SessionsView { group, recording }
}
