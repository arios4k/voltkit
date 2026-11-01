//! The Why Didn't It Sleep? page: what keeps the computer awake now, what
//! happened during recent sleeps, and which devices may wake it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::app::{desktop_app, toast};
use crate::modules::sleep::{self, Blocker, Effect, Event, Note, Report, WakeDevice};
use crate::{daemon, portal};

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = adw::PreferencesPage::new();
    let view = View::new(toasts);
    for group in [&view.now, &view.history, &view.devices, &view.mode] {
        page.add(group);
    }
    view.reload();
    // Reload when the page is shown again: things change while it's hidden.
    {
        let view = Rc::downgrade(&view);
        page.connect_map(move |_| {
            if let Some(view) = view.upgrade()
                && view.loaded.get()
            {
                view.reload();
            }
        });
    }
    page.connect_destroy(move |_| {
        let _ = &view;
    });
    page.upcast()
}

struct View {
    now: adw::PreferencesGroup,
    history: adw::PreferencesGroup,
    devices: adw::PreferencesGroup,
    mode: adw::PreferencesGroup,
    rows: RefCell<Vec<(adw::PreferencesGroup, gtk::Widget)>>,
    loaded: Cell<bool>,
    toasts: adw::ToastOverlay,
}

impl View {
    fn new(toasts: &adw::ToastOverlay) -> Rc<Self> {
        let refresh = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Check Again")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let now = adw::PreferencesGroup::builder()
            .title("Right Now")
            .description("Apps and settings that keep the computer awake at the moment.")
            .header_suffix(&refresh)
            .build();
        let history = adw::PreferencesGroup::builder()
            .title("Recent Sleeps")
            .description(
                "The last week, newest first. Linux only records what woke the computer when it was the lid or the \
                 power button.",
            )
            .build();
        let devices = adw::PreferencesGroup::builder()
            .title("Devices That Can Wake It")
            .description(
                "USB devices allowed to wake the computer from sleep. A mouse or receiver that wakes it in your bag \
                 can be stopped here. Changing it asks for your password and lasts after restarts.",
            )
            .build();
        let mode = adw::PreferencesGroup::builder().title("Sleep Mode").visible(false).build();
        let view = Rc::new(Self {
            now,
            history,
            devices,
            mode,
            rows: RefCell::default(),
            loaded: Cell::new(false),
            toasts: toasts.clone(),
        });
        let weak = Rc::downgrade(&view);
        refresh.connect_clicked(move |_| {
            if let Some(view) = weak.upgrade() {
                view.reload();
            }
        });
        view
    }

    fn reload(self: &Rc<Self>) {
        let view = self.clone();
        glib::spawn_future_local(async move {
            if let Ok(report) = gio::spawn_blocking(|| portal::block_on(sleep::report())).await {
                view.show(report);
            }
            view.loaded.set(true);
        });
    }

    fn add(&self, group: &adw::PreferencesGroup, row: impl IsA<gtk::Widget>) {
        group.add(&row);
        self.rows.borrow_mut().push((group.clone(), row.upcast()));
    }

    fn show(self: &Rc<Self>, report: Report) {
        for (group, row) in self.rows.take() {
            group.remove(&row);
        }

        if report.blockers.is_empty() && report.notes.is_empty() {
            let row = adw::ActionRow::builder()
                .title("Nothing is keeping it awake")
                .subtitle("It sleeps when you close the lid, choose Suspend, or leave it idle.")
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));
            self.add(&self.now, row);
        }
        for blocker in report.blockers {
            let row = self.blocker_row(blocker);
            self.add(&self.now, row);
        }
        for note in report.notes {
            let row = note_row(&note);
            if note.in_power_settings {
                let button = gtk::Button::builder().label("Power Settings").valign(gtk::Align::Center).build();
                button.connect_clicked(open_power_settings);
                row.add_suffix(&button);
            }
            self.add(&self.now, row);
        }

        if let Some(problem) = &report.history_problem {
            let row = adw::ActionRow::builder().title("The sleep history isn't available").subtitle(glib::markup_escape_text(problem)).build();
            row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
            self.add(&self.history, row);
        } else if report.history.is_empty() {
            let row = adw::ActionRow::builder().title("No sleeps in the last week").build();
            self.add(&self.history, row);
        }
        for event in &report.history {
            self.add(&self.history, event_row(event));
        }

        if report.devices.is_empty() {
            let row = adw::ActionRow::builder().title("No USB devices that could wake it are connected").build();
            self.add(&self.devices, row);
        }
        for device in report.devices {
            let row = self.device_row(device);
            self.add(&self.devices, row);
        }

        self.mode.set_visible(report.sleep_mode.is_some());
        if let Some(mode) = report.sleep_mode {
            self.add(&self.mode, note_row(&mode));
        }
    }

    fn blocker_row(self: &Rc<Self>, blocker: Blocker) -> adw::ActionRow {
        let app = blocker.app_id.as_deref().and_then(desktop_app);
        let name = app.as_ref().map_or_else(|| blocker.name.clone(), |a| a.display_name().to_string());
        let fallback = match blocker.effect {
            Effect::Sleep => "media-playback-pause-symbolic",
            Effect::Idle => "video-display-symbolic",
            Effect::Lid => "computer-symbolic",
        };
        let icon = app.and_then(|a| a.icon()).unwrap_or_else(|| gio::ThemedIcon::new(fallback).upcast());
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&name))
            .subtitle(glib::markup_escape_text(&format!("{}\n{}", blocker.effect.describe(), blocker.reason)))
            .subtitle_lines(3)
            .build();
        row.add_prefix(&gtk::Image::builder().gicon(&icon).pixel_size(32).build());
        if blocker.keep_awake {
            let button = gtk::Button::builder().label("Turn Off").valign(gtk::Align::Center).build();
            let view = Rc::downgrade(self);
            button.connect_clicked(move |b| {
                let Some(view) = view.upgrade() else { return };
                b.set_sensitive(false);
                glib::spawn_future_local(async move {
                    match gio::spawn_blocking(daemon::stop_awake).await {
                        Ok(Ok(())) => toast(&view.toasts, "Keep Awake is off"),
                        Ok(Err(e)) => toast(&view.toasts, e.to_string()),
                        Err(_) => {}
                    }
                    view.reload();
                });
            });
            row.add_suffix(&button);
        }
        row
    }

    fn device_row(self: &Rc<Self>, device: WakeDevice) -> adw::SwitchRow {
        let row = adw::SwitchRow::builder()
            .title(glib::markup_escape_text(&device.name))
            .subtitle(match device.wakeups {
                0 => "Can wake the computer".to_owned(),
                1 => "Woke the computer once since it started".to_owned(),
                n => format!("Woke the computer {n} times since it started"),
            })
            .active(device.enabled)
            .build();
        if !device.enabled {
            row.set_subtitle("Can't wake the computer");
        }
        row.add_prefix(&gtk::Image::from_icon_name("drive-removable-media-usb-symbolic"));
        // Set while the switch is put back after a failure, so that isn't taken as a click.
        let reverting = Rc::new(Cell::new(false));
        let view = Rc::downgrade(self);
        row.connect_active_notify(move |row| {
            if reverting.get() {
                return;
            }
            let Some(view) = view.upgrade() else { return };
            let enabled = row.is_active();
            let (row, device, reverting) = (row.clone(), device.clone(), reverting.clone());
            if std::env::var_os("VOLTKIT_SLEEP_DEMO").is_some() {
                return;
            }
            glib::spawn_future_local(async move {
                let d = device.clone();
                let result = gio::spawn_blocking(move || sleep::set_wake(&d, enabled)).await;
                match result {
                    Ok(Ok(())) => {
                        row.set_subtitle(if enabled { "Can wake the computer" } else { "Can't wake the computer" });
                        toast(&view.toasts, if enabled { format!("{} can wake the computer", device.name) } else { format!("{} won't wake the computer", device.name) });
                    }
                    Ok(Err(e)) => {
                        toast(&view.toasts, e.to_string());
                        reverting.set(true);
                        row.set_active(!enabled);
                        reverting.set(false);
                    }
                    Err(_) => {}
                }
            });
        });
        row
    }
}

fn note_row(note: &Note) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(&note.title))
        .subtitle(glib::markup_escape_text(&note.detail))
        .subtitle_lines(4)
        .build();
    row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
    row
}

fn event_row(event: &Event) -> adw::ActionRow {
    let (title, details) = sleep::describe_event(event);
    let warning = match event {
        Event::Slept { start, end, unexpected, battery, .. } => {
            *unexpected || sleep::drain_per_hour(*start, *end, *battery).is_some_and(sleep::is_high_drain)
        }
        Event::LidAwake { .. } => true,
    };
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(&title))
        .subtitle(glib::markup_escape_text(&details.join("\n")))
        .subtitle_lines(details.len() as i32)
        .build();
    let icon = gtk::Image::from_icon_name(if warning { "dialog-warning-symbolic" } else { "weather-clear-night-symbolic" });
    if warning {
        icon.add_css_class("warning");
    }
    row.add_prefix(&icon);
    row
}

fn open_power_settings(widget: &gtk::Button) {
    let launched = gio::AppInfo::create_from_commandline("gnome-control-center power", Some("Settings"), gio::AppInfoCreateFlags::NONE)
        .and_then(|info| info.launch(&[], Some(&widget.display().app_launch_context())));
    if launched.is_err()
        && let Some(info) = desktop_app("org.gnome.Settings")
    {
        let _ = info.launch(&[], Some(&widget.display().app_launch_context()));
    }
}
