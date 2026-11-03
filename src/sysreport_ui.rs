//! The System Report page: checks with fixes, what changed, the hardware in
//! plain words, temperatures, firmware updates, and memory upgrades.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::app::toast;
use crate::modules::sysreport::{self, Change, Check, Level, MemoryInfo, MemoryModule, Report};

/// Changes listed per day before "and N more".
const CHANGES_PER_DAY: usize = 6;
const DAYS_SHOWN: usize = 7;

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = crate::workspace::Workspace::new();
    let view = View::new(toasts);
    page.banner(&view.intro);
    for group in [&view.checks, &view.changes] { page.add(group); }
    for group in [&view.computer, &view.temperatures, &view.firmware, &view.memory] { page.aside(group); }
    view.reload();
    page.connect_destroy(move |_| {
        let _ = &view;
    });
    page.upcast()
}

struct View {
    intro: adw::PreferencesGroup,
    checks: adw::PreferencesGroup,
    changes: adw::PreferencesGroup,
    computer: adw::PreferencesGroup,
    temperatures: adw::PreferencesGroup,
    firmware: adw::PreferencesGroup,
    memory: adw::PreferencesGroup,
    memory_button: gtk::Button,
    rows: RefCell<Vec<(adw::PreferencesGroup, gtk::Widget)>>,
    memory_rows: RefCell<Vec<gtk::Widget>>,
    /// The latest report and changes, for "Copy Report".
    report: RefCell<Option<(Report, Vec<Change>)>>,
    toasts: adw::ToastOverlay,
}

fn header_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder().icon_name(icon).tooltip_text(tooltip).valign(gtk::Align::Center).css_classes(["flat"]).build()
}

impl View {
    fn new(toasts: &adw::ToastOverlay) -> Rc<Self> {
        let refresh = header_button("view-refresh-symbolic", "Check Again");
        let copy = gtk::Button::builder()
            .child(&adw::ButtonContent::builder().icon_name("edit-copy-symbolic").label("Copy Report").build())
            .valign(gtk::Align::Center)
            .css_classes(["suggested-action"])
            .build();
        let header = gtk::Box::builder().spacing(6).build();
        header.append(&refresh);
        header.append(&copy);
        let intro = adw::PreferencesGroup::builder()
            .title("Overview")
            .description(
                "What's in this computer and how it's doing, in plain words. Copy the report when asking for help \
                 in a forum or bug report: it has no serial numbers, names or network addresses.",
            )
            .header_suffix(&crate::app::spaced(&header))
            .build();
        let checks = adw::PreferencesGroup::builder().title("Checks").build();
        let changes = adw::PreferencesGroup::builder()
            .title("What Changed")
            .description("Hardware, drivers, kernel and firmware, compared day by day: handy when something stops working after an update.")
            .build();
        let computer = adw::PreferencesGroup::builder().title("This Computer").build();
        let temperatures = adw::PreferencesGroup::builder().title("Temperatures").visible(false).build();
        let check_online = gtk::Button::builder().label("Check Online").valign(gtk::Align::Center).build();
        let firmware = adw::PreferencesGroup::builder()
            .title("Firmware Updates")
            .description("Updates for the BIOS, drives, docks and other devices, from the makers through the Linux Vendor Firmware Service. They often fix battery, sleep and stability problems.")
            .header_suffix(&crate::app::spaced(&check_online))
            .build();
        let memory_button = gtk::Button::builder().label("Show…").valign(gtk::Align::Center).build();
        let memory = adw::PreferencesGroup::builder()
            .title("Memory and Upgrades")
            .description("Memory slots, what's in them, and how much this computer can take. Reading them needs the administrator password.")
            .header_suffix(&crate::app::spaced(&memory_button))
            .build();

        let view = Rc::new(Self {
            intro,
            checks,
            changes,
            computer,
            temperatures,
            firmware,
            memory,
            memory_button: memory_button.clone(),
            rows: RefCell::default(),
            memory_rows: RefCell::default(),
            report: RefCell::default(),
            toasts: toasts.clone(),
        });
        let weak = Rc::downgrade(&view);
        refresh.connect_clicked(move |_| {
            if let Some(v) = weak.upgrade() {
                v.reload();
            }
        });
        let weak = Rc::downgrade(&view);
        copy.connect_clicked(move |b| {
            if let Some(v) = weak.upgrade() {
                v.copy_report(b);
            }
        });
        let weak = Rc::downgrade(&view);
        check_online.connect_clicked(move |b| {
            let Some(v) = weak.upgrade() else { return };
            b.set_sensitive(false);
            let b = b.clone();
            glib::spawn_future_local(async move {
                match gio::spawn_blocking(sysreport::refresh_firmware_list).await {
                    Ok(Ok(())) => v.reload(),
                    Ok(Err(e)) => toast(&v.toasts, format!("Could not check online: {e}")),
                    Err(_) => {}
                }
                b.set_sensitive(true);
            });
        });
        let weak = Rc::downgrade(&view);
        memory_button.connect_clicked(move |b| {
            let Some(v) = weak.upgrade() else { return };
            b.set_sensitive(false);
            let b = b.clone();
            glib::spawn_future_local(async move {
                match gio::spawn_blocking(sysreport::memory_info_as_root).await {
                    Ok(Ok(info)) => {
                        v.show_memory(&info);
                        b.set_visible(false);
                    }
                    Ok(Err(e)) => toast(&v.toasts, format!("Not shown: {e}")),
                    Err(_) => {}
                }
                b.set_sensitive(true);
            });
        });
        if std::env::var_os("VOLTKIT_REPORT_DEMO").is_some() {
            view.show_memory(&demo_memory());
            memory_button.set_visible(false);
        }
        view
    }

    fn add(&self, group: &adw::PreferencesGroup, row: impl IsA<gtk::Widget>) {
        group.add(&row);
        self.rows.borrow_mut().push((group.clone(), row.upcast()));
    }

    fn clear(&self) {
        for (group, row) in self.rows.take() {
            group.remove(&row);
        }
    }

    fn reload(self: &Rc<Self>) {
        self.clear();
        let waiting = adw::ActionRow::builder().title("Checking this computer…").build();
        waiting.add_prefix(&gtk::Spinner::builder().spinning(true).build());
        self.add(&self.checks, waiting);
        let view = self.clone();
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(|| {
                let report = sysreport::collect();
                let _ = sysreport::save_snapshot(&report.facts);
                let changes = sysreport::changes();
                (report, changes)
            })
            .await;
            if let Ok((report, changes)) = result {
                view.show(&report, &changes);
                *view.report.borrow_mut() = Some((report, changes));
            }
        });
    }

    fn show(self: &Rc<Self>, report: &Report, changes: &[Change]) {
        self.clear();

        for check in &report.checks {
            self.add(&self.checks, self.check_row(check));
        }

        self.show_changes(changes);

        for (label, value) in &report.overview {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(label))
                .subtitle(glib::markup_escape_text(value))
                .subtitle_lines(value.lines().count() as i32)
                .subtitle_selectable(true)
                .css_classes(["property"])
                .build();
            self.add(&self.computer, row);
        }

        self.temperatures.set_visible(!report.sensors.is_empty() || !report.fans.is_empty());
        for s in &report.sensors {
            let verdict = sysreport::temperature_verdict(s);
            let row = adw::ActionRow::builder().title(&s.label).subtitle(format!("{:.0} °C \u{00b7} {verdict}", s.celsius)).build();
            let icon = gtk::Image::from_icon_name(match verdict {
                "hot" => "dialog-warning-symbolic",
                "warm" => "weather-clear-symbolic",
                _ => "emblem-ok-symbolic",
            });
            icon.add_css_class(match verdict {
                "hot" => "error",
                "warm" => "warning",
                _ => "success",
            });
            row.add_prefix(&icon);
            self.add(&self.temperatures, row);
        }
        for (label, rpm) in &report.fans {
            let row = adw::ActionRow::builder().title(label).subtitle(if *rpm == 0 { "Off".to_owned() } else { format!("{rpm} RPM") }).build();
            row.add_prefix(&gtk::Image::from_icon_name("weather-windy-symbolic"));
            self.add(&self.temperatures, row);
        }
        if report.throttled > 0 {
            let row = adw::ActionRow::builder()
                .title("Slowed down because of heat")
                .subtitle(format!("{} times since the computer started", report.throttled))
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
            self.add(&self.temperatures, row);
        }

        match &report.firmware {
            None => {
                let row = adw::ActionRow::builder()
                    .title("Firmware updates aren't available")
                    .subtitle("Install fwupd to get firmware updates from the makers of your hardware.")
                    .build();
                self.add(&self.firmware, row);
            }
            Some(updates) if updates.is_empty() => {
                let row = adw::ActionRow::builder()
                    .title("Up to date")
                    .subtitle("As of the last check. Check Online downloads the latest list.")
                    .build();
                row.add_prefix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));
                self.add(&self.firmware, row);
            }
            Some(updates) => {
                for u in updates {
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(&u.device))
                        .subtitle(glib::markup_escape_text(format!("{} \u{2192} {}\n{}", u.current, u.available, u.summary).trim()))
                        .subtitle_lines(3)
                        .build();
                    row.add_prefix(&gtk::Image::from_icon_name("software-update-available-symbolic"));
                    let install = gtk::Button::builder().label("Install…").valign(gtk::Align::Center).build();
                    install.connect_clicked(open_updates);
                    row.add_suffix(&install);
                    self.add(&self.firmware, row);
                }
            }
        }
    }

    fn check_row(&self, check: &Check) -> adw::ActionRow {
        let mut subtitle = glib::markup_escape_text(&check.detail).to_string();
        if let Some(command) = &check.command {
            subtitle.push_str(&format!("\nFix: <tt>{}</tt>", glib::markup_escape_text(command)));
        }
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&check.title))
            .subtitle(subtitle)
            .subtitle_lines(5)
            .use_markup(true)
            .build();
        let (icon, class) = match check.level {
            Level::Ok => ("emblem-ok-symbolic", "success"),
            Level::Info => ("dialog-information-symbolic", "accent"),
            Level::Warning => ("dialog-warning-symbolic", "warning"),
            Level::Problem => ("dialog-error-symbolic", "error"),
        };
        let image = gtk::Image::from_icon_name(icon);
        image.add_css_class(class);
        row.add_prefix(&image);
        if let Some(command) = check.command.clone() {
            let copy = header_button("edit-copy-symbolic", "Copy the command, to paste in a terminal");
            let toasts = self.toasts.clone();
            copy.connect_clicked(move |b| {
                b.clipboard().set_text(&command);
                toast(&toasts, "Command copied: paste it in a terminal");
            });
            row.add_suffix(&copy);
        }
        if let Some(link) = check.link.clone() {
            let open = gtk::Button::builder().label("Open Guide").valign(gtk::Align::Center).build();
            open.connect_clicked(move |b| {
                gtk::UriLauncher::new(&link).launch(b.root().and_downcast::<gtk::Window>().as_ref(), gio::Cancellable::NONE, |_| {});
            });
            row.add_suffix(&open);
        }
        row
    }

    fn show_changes(&self, changes: &[Change]) {
        if changes.is_empty() {
            let row = adw::ActionRow::builder()
                .title("Nothing to compare yet")
                .subtitle("Voltkit saves a snapshot each day while it runs, so changes show up here from tomorrow on.")
                .subtitle_lines(2)
                .build();
            self.add(&self.changes, row);
            return;
        }
        // Group by day, newest first.
        let mut days: Vec<(&str, Vec<&Change>)> = Vec::new();
        for c in changes {
            match days.last_mut() {
                Some((date, list)) if *date == c.date => list.push(c),
                _ => days.push((&c.date, vec![c])),
            }
        }
        for (date, list) in days.into_iter().take(DAYS_SHOWN) {
            let mut lines: Vec<String> = list.iter().take(CHANGES_PER_DAY).map(|c| c.describe()).collect();
            if list.len() > CHANGES_PER_DAY {
                lines.push(format!("and {} more", list.len() - CHANGES_PER_DAY));
            }
            let row = adw::ActionRow::builder()
                .title(sysreport::short_date(date))
                .subtitle(glib::markup_escape_text(&lines.join("\n")))
                .subtitle_lines(lines.len() as i32)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("document-open-recent-symbolic"));
            self.add(&self.changes, row);
        }
    }

    fn show_memory(&self, info: &MemoryInfo) {
        for row in self.memory_rows.take() {
            self.memory.remove(&row);
        }
        let mut rows = Vec::new();
        for (label, value) in sysreport::memory_advice(info) {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&label))
                .subtitle(glib::markup_escape_text(&value))
                .css_classes(["property"])
                .build();
            self.memory.add(&row);
            rows.push(row.upcast());
        }
        *self.memory_rows.borrow_mut() = rows;
        self.memory_button.set_visible(false);
    }

    /// Shows what will be copied, then copies it.
    fn copy_report(self: &Rc<Self>, button: &gtk::Button) {
        let text = match &*self.report.borrow() {
            Some((report, changes)) => sysreport::share_text(report, changes),
            None => return,
        };
        let preview = gtk::TextView::builder()
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(10)
            .right_margin(10)
            .build();
        preview.buffer().set_text(&text);
        let scroller = gtk::ScrolledWindow::builder().child(&preview).min_content_height(300).min_content_width(480).css_classes(["card"]).build();
        let dialog = adw::AlertDialog::new(
            Some("Copy System Report"),
            Some("This is what will be copied. It has no serial numbers, user or computer names, or network addresses."),
        );
        dialog.set_extra_child(Some(&scroller));
        dialog.add_responses(&[("cancel", "Cancel"), ("copy", "Copy")]);
        dialog.set_response_appearance("copy", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("copy"));
        dialog.set_close_response("cancel");
        let (toasts, button2) = (self.toasts.clone(), button.clone());
        dialog.connect_response(None, move |_, response| {
            if response == "copy" {
                button2.clipboard().set_text(&text);
                toast(&toasts, "Report copied: paste it where you're asking for help");
            }
        });
        dialog.present(Some(button));
    }
}

/// Opens the desktop's software updates, where firmware updates are installed.
fn open_updates(widget: &gtk::Button) {
    let ctx = widget.display().app_launch_context();
    let launched = gio::AppInfo::create_from_commandline("gnome-software --mode=updates", Some("Software"), gio::AppInfoCreateFlags::NONE)
        .and_then(|info| info.launch(&[], Some(&ctx)));
    if launched.is_err() {
        widget.clipboard().set_text("fwupdmgr update");
        if let Some(overlay) = widget.ancestor(adw::ToastOverlay::static_type()).and_downcast::<adw::ToastOverlay>() {
            toast(&overlay, "Command copied: paste “fwupdmgr update” in a terminal");
        }
    }
}

/// Two slots with one free, for the README screenshots.
fn demo_memory() -> MemoryInfo {
    MemoryInfo {
        max_mb: Some(96 * 1024),
        slots: vec![
            MemoryModule { locator: "DIMM A".into(), size_mb: Some(32 * 1024), kind: "DDR5".into(), speed: Some(5600), soldered: false },
            MemoryModule { locator: "DIMM B".into(), size_mb: None, kind: String::new(), speed: None, soldered: false },
        ],
    }
}
