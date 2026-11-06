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
use crate::modules::gpu_tune::{self, CurvePoint, FanMode, Settings};
use crate::workspace::Workspace;

const HISTORY: usize = 90;
const POLL: Duration = Duration::from_secs(2);
const CONFIRM_SECS: u32 = 15;

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let cards = Rc::new(gpu::inspect());
    let outer = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
    if cards.is_empty() {
        let status = adw::StatusPage::builder()
            .icon_name("applications-graphics-symbolic")
            .title(tr("No graphics cards were found."))
            .description(tr("Voltkit looks for NVIDIA and AMD cards through the kernel's graphics devices."))
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
            let top = c.get().max(1.0);
            for (i, v) in data.iter().enumerate() {
                let x = f64::from(w) * i as f64 / (HISTORY - 1) as f64;
                let y = f64::from(h) - 2.0 - (f64::from(h) - 4.0) * (v / top).clamp(0.0, 1.0);
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
        page.aside(&tune_group(card, toasts));

        stack.add_named(&*page, Some(&index.to_string()));
        Rc::new(CardView { readings })
    }

    fn update(&self, sample: &Sample) {
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

fn expander(title: String, subtitle: String) -> adw::ExpanderRow {
    adw::ExpanderRow::builder().title(title).subtitle(subtitle).show_enable_switch(true).enable_expansion(false).build()
}

type Rebuild = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// The undo history of the fan curve being edited.
struct CurveEdit {
    points: Vec<CurvePoint>,
    undo: Vec<Vec<CurvePoint>>,
    redo: Vec<Vec<CurvePoint>>,
}

fn tune_group(card: &Card, toasts: &adw::ToastOverlay) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(tr("Tune"))
        .description(tr("Turn on the controls to change. Preview shows exactly what will happen. Nothing stays unless you confirm it."))
        .build();
    let any = Control::ALL.iter().any(|c| *c != Control::Monitoring && *c != Control::VoltageCurve && supported(card, *c))
        || supported(card, Control::VoltageCurve);
    if !any {
        group.add(&adw::ActionRow::builder().title(tr("Nothing on this card can be changed")).subtitle(tr("See what this card supports for the reasons.")).build());
        return group;
    }
    let s = &card.sample;

    let power = supported(card, Control::PowerLimit).then(|| {
        let (lo, hi) = (s.limit_min_w.unwrap_or(0.0), s.limit_max_w.unwrap_or(0.0));
        let row = expander(tr("Power limit"), trf!("Between {} W and {} W.", format!("{lo:.0}"), format!("{hi:.0}")));
        let value = spin(tr("Watts"), None, s.limit_w.unwrap_or(hi), lo, hi, 5.0);
        row.add_row(&value);
        group.add(&row);
        (row, value)
    });

    let fan = supported(card, Control::FanSpeed).then(|| {
        let row = expander(tr("Fan"), tr("Fixed speed, or a curve that follows the temperature."));
        let modes = gtk::StringList::new(&[&tr("The card's own control"), &tr("Fixed speed"), &tr("Curve")]);
        let mode = adw::ComboRow::builder().title(tr("Mode")).model(&modes).build();
        let fixed = spin(tr("Speed (%)"), None, s.fan_pct.unwrap_or(50.0).max(gpu_tune::MIN_FIXED_FAN_PCT), gpu_tune::MIN_FIXED_FAN_PCT, 100.0, 5.0);
        row.add_row(&mode);
        row.add_row(&fixed);
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
        row.add_row(&curve_box);
        let rebuild: Rebuild = Rc::new(RefCell::new(None));
        let build = {
            let (edit, curve_rows, curve_box, rebuild) = (edit.clone(), curve_rows.clone(), curve_box.clone(), rebuild.clone());
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
                        let edit = edit.clone();
                        spin.connect_value_changed(move |s| {
                            let mut e = edit.borrow_mut();
                            let before = e.points.clone();
                            if let Some(p) = e.points.get_mut(i) {
                                if is_temp { p.temp_c = s.value() } else { p.speed_pct = s.value() }
                            }
                            e.undo.push(before);
                            e.redo.clear();
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
            })
        };
        *rebuild.borrow_mut() = Some(build.clone() as Rc<dyn Fn()>);
        build();

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
        group.add(&row);
        (row, mode, fixed, edit)
    });

    let target = supported(card, Control::TemperatureTarget).then(|| {
        let row = expander(tr("Temperature target"), tr("The temperature the card's fan control aims for."));
        let value = spin(tr("Degrees (°C)"), None, 75.0, 40.0, 100.0, 1.0);
        row.add_row(&value);
        group.add(&row);
        (row, value)
    });

    let clocks = supported(card, Control::ClockOffsets).then(|| {
        let row = expander(tr("Clock offsets"), tr("Raise or lower the clocks from their defaults."));
        let (core, mem) = (spin(tr("Core (MHz)"), None, 0.0, -500.0, 500.0, 5.0), spin(tr("Memory (MHz)"), None, 0.0, -1000.0, 1000.0, 10.0));
        row.add_row(&core);
        row.add_row(&mem);
        group.add(&row);
        (row, core, mem)
    });

    let voltage = supported(card, Control::VoltageCurve).then(|| {
        let row = expander(tr("Voltage offset"), tr("Lower the voltage for less heat, or raise it for stability."));
        let value = spin(tr("Millivolts (mV)"), None, 0.0, -200.0, 0.0, 5.0);
        row.add_row(&value);
        group.add(&row);
        (row, value)
    });

    let keep = adw::SwitchRow::builder().title(tr("Keep after restart")).subtitle(tr("Saves these settings and applies them at startup. Fan curves need this.")).build();
    group.add(&keep);

    let collect = {
        let address = card.gpu.address.clone();
        Rc::new(move || {
            let mut s = Settings { gpu: address.clone(), ..Settings::default() };
            if let Some((row, v)) = &power
                && row.enables_expansion()
            {
                s.power_limit_w = Some(v.value());
            }
            if let Some((row, mode, fixed, edit)) = &fan
                && row.enables_expansion()
            {
                s.fan = Some(match mode.selected() {
                    0 => FanMode::Auto,
                    1 => FanMode::Fixed { speed_pct: fixed.value() },
                    _ => FanMode::Curve { points: edit.borrow().points.clone() },
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
    group
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
