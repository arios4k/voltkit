//! The What's Using This? page: pick a file, folder, drive or port, see which
//! apps keep it busy, and close them (politely first, then by force).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::app::toast;
use crate::modules::usage::{self, Holder, Scan, Target};

/// How long an app gets to quit before Force Quit is offered.
const GRACE: Duration = Duration::from_secs(3);
/// How long Eject waits for all the apps to quit.
const EJECT_GRACE: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(200);
/// Uses listed per app before "and N more".
const SHOWN_USES: usize = 3;

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = adw::PreferencesPage::new();
    let usage = Usage::new(toasts);
    page.add(&usage.check_group());
    page.add(&usage.drives);
    page.add(&usage.results);
    if std::env::var_os("VOLTKIT_USAGE_DEMO").is_some() {
        usage.demo.set(true);
        *usage.target.borrow_mut() = Some(Target::Path(DEMO_DRIVE.into()));
        usage.show(demo_scan(), true);
    }
    usage.refresh_drives();

    // Files and folders can be dropped anywhere on the page.
    let drop = gtk::DropTarget::new(gio::File::static_type(), gdk::DragAction::COPY);
    {
        let usage = usage.clone();
        drop.connect_drop(move |_, value, _, _| {
            let Some(path) = value.get::<gio::File>().ok().and_then(|f| f.path()) else { return false };
            usage.check_path(&path.to_string_lossy());
            true
        });
    }
    page.add_controller(drop);

    let monitor = gio::VolumeMonitor::get();
    for signal in ["mount-added", "mount-removed", "mount-changed"] {
        let usage = Rc::downgrade(&usage);
        monitor.connect_local(signal, false, move |_| {
            if let Some(usage) = usage.upgrade() {
                usage.refresh_drives();
            }
            None
        });
    }
    // Rows only hold weak references; the page keeps the state alive.
    page.connect_destroy(move |_| {
        let _ = (&usage, &monitor);
    });
    page.upcast()
}

struct Usage {
    drives: adw::PreferencesGroup,
    drive_rows: RefCell<Vec<gtk::Widget>>,
    results: adw::PreferencesGroup,
    rows: RefCell<Vec<gtk::Widget>>,
    target: RefCell<Option<Target>>,
    /// The drive being checked, for Eject.
    mount: RefCell<Option<gio::Mount>>,
    eject: gtk::Button,
    toasts: adw::ToastOverlay,
    /// Showing demo results: the target is a drive, though no real one is mounted.
    demo: Cell<bool>,
}

impl Usage {
    fn new(toasts: &adw::ToastOverlay) -> Rc<Self> {
        let again = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Check Again")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let eject = gtk::Button::builder()
            .child(&adw::ButtonContent::builder().icon_name("media-eject-symbolic").label("Eject").build())
            .valign(gtk::Align::Center)
            .visible(false)
            .build();
        let header = gtk::Box::builder().spacing(6).build();
        header.append(&again);
        header.append(&eject);
        let results = adw::PreferencesGroup::builder().title("Results").header_suffix(&crate::app::spaced(&header)).visible(false).build();
        results.set_widget_name("usage-results");
        let drives = adw::PreferencesGroup::builder()
            .title("Drives")
            .description("USB sticks, memory cards and other disks you can eject")
            .build();
        let usage = Rc::new(Self {
            drives,
            drive_rows: RefCell::default(),
            results,
            rows: RefCell::default(),
            target: RefCell::default(),
            mount: RefCell::default(),
            eject: eject.clone(),
            toasts: toasts.clone(),
            demo: Cell::new(false),
        });
        {
            let usage = Rc::downgrade(&usage);
            again.connect_clicked(move |_| {
                if let Some(usage) = usage.upgrade() {
                    usage.rescan();
                }
            });
        }
        {
            let usage = Rc::downgrade(&usage);
            eject.connect_clicked(move |_| {
                if let Some(usage) = usage.upgrade() {
                    glib::spawn_future_local(usage.eject());
                }
            });
        }
        usage
    }

    fn check_group(self: &Rc<Self>) -> adw::PreferencesGroup {
        let group = adw::PreferencesGroup::builder()
            .title("What's Using This?")
            .description(
                "Find the apps that keep a file, folder or drive busy (\u{201c}target is busy\u{201d}), or that use a \
                 network port (\u{201c}address already in use\u{201d}), and close them.",
            )
            .build();

        let choose_file = gtk::Button::builder().label("File…").valign(gtk::Align::Center).build();
        let choose_folder = gtk::Button::builder().label("Folder…").valign(gtk::Align::Center).build();
        let place = adw::ActionRow::builder().title("File or folder").subtitle("Or drop one on this window").build();
        place.add_suffix(&choose_file);
        place.add_suffix(&choose_folder);
        for (button, folder) in [(&choose_file, false), (&choose_folder, true)] {
            let usage = Rc::downgrade(self);
            button.connect_clicked(move |b| {
                let usage = usage.clone();
                let window = b.root().and_downcast::<gtk::Window>();
                let dialog = gtk::FileDialog::builder().title(if folder { "Choose a Folder" } else { "Choose a File" }).build();
                let done = move |res: Result<gio::File, glib::Error>| {
                    let (Some(usage), Some(path)) = (usage.upgrade(), res.ok().and_then(|f| f.path())) else { return };
                    usage.check_path(&path.to_string_lossy());
                };
                if folder {
                    dialog.select_folder(window.as_ref(), gio::Cancellable::NONE, done);
                } else {
                    dialog.open(window.as_ref(), gio::Cancellable::NONE, done);
                }
            });
        }

        let port = adw::EntryRow::builder()
            .title("Network port, such as 3000")
            .show_apply_button(true)
            .input_purpose(gtk::InputPurpose::Digits)
            .build();
        {
            let usage = Rc::downgrade(self);
            port.connect_apply(move |entry| {
                let Some(usage) = usage.upgrade() else { return };
                match Target::parse(&entry.text()) {
                    Ok(target @ Target::Port(_)) => usage.check(target, None),
                    _ => toast(&usage.toasts, "Enter a port number from 1 to 65535"),
                }
            });
        }
        group.add(&place);
        group.add(&port);
        group
    }

    fn refresh_drives(self: &Rc<Self>) {
        for row in self.drive_rows.take() {
            self.drives.remove(&row);
        }
        let mut rows = Vec::new();
        if self.demo.get() {
            let row = adw::ActionRow::builder().title("USB DRIVE").subtitle(DEMO_DRIVE).build();
            row.add_prefix(&gtk::Image::from_icon_name("media-removable-symbolic"));
            row.add_suffix(&gtk::Button::builder().label("Check").valign(gtk::Align::Center).build());
            self.drives.add(&row);
            *self.drive_rows.borrow_mut() = vec![row.upcast()];
            return;
        }
        for mount in gio::VolumeMonitor::get().mounts() {
            let Some(path) = mount.root().path() else { continue };
            let removable = mount.can_eject() || mount.drive().is_some_and(|d| d.is_removable() || d.can_eject());
            if !removable || mount.is_shadowed() {
                continue;
            }
            let check = gtk::Button::builder().label("Check").valign(gtk::Align::Center).build();
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&mount.name()))
                .subtitle(glib::markup_escape_text(&path.to_string_lossy()))
                .build();
            row.add_prefix(&gtk::Image::from_gicon(&mount.symbolic_icon()));
            row.add_suffix(&check);
            row.set_activatable_widget(Some(&check));
            let usage = Rc::downgrade(self);
            check.connect_clicked(move |_| {
                if let Some(usage) = usage.upgrade() {
                    usage.check(Target::Path(path.clone()), Some(mount.clone()));
                }
            });
            rows.push(row.upcast::<gtk::Widget>());
        }
        if rows.is_empty() {
            let row = adw::ActionRow::builder().title("No drives to eject are connected").css_classes(["dim-label"]).build();
            rows.push(row.upcast());
        }
        for row in &rows {
            self.drives.add(row);
        }
        *self.drive_rows.borrow_mut() = rows;
    }

    fn check_path(self: &Rc<Self>, path: &str) {
        match Target::parse(path) {
            Ok(target) => self.check(target, None),
            Err(e) => toast(&self.toasts, e.to_string()),
        }
    }

    fn check(self: &Rc<Self>, target: Target, mount: Option<gio::Mount>) {
        *self.target.borrow_mut() = Some(target);
        *self.mount.borrow_mut() = mount;
        self.rescan();
    }

    fn rescan(self: &Rc<Self>) {
        let Some(target) = self.target.borrow().clone() else { return };
        let usage = self.clone();
        glib::spawn_future_local(async move {
            let scan = gio::spawn_blocking(move || usage::scan(&target)).await.unwrap_or_default();
            usage.show(scan, false);
        });
    }

    fn title(&self) -> String {
        match (&*self.mount.borrow(), &*self.target.borrow()) {
            (Some(mount), _) => mount.name().to_string(),
            (None, Some(Target::Path(p))) => p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned()),
            (None, Some(Target::Port(p))) => format!("Port {p}"),
            (None, None) => String::new(),
        }
    }

    /// Shows a scan; `complete` when it included every user's processes.
    fn show(self: &Rc<Self>, scan: Scan, complete: bool) {
        for row in self.rows.take() {
            self.results.remove(&row);
        }
        let is_drive = self.mount.borrow().is_some() || self.demo.get();
        self.results.set_title(&glib::markup_escape_text(&format!("Using \u{201c}{}\u{201d}", self.title())));
        self.results.set_description(
            self.target.borrow().as_ref().filter(|t| matches!(t, Target::Path(_))).map(|t| glib::markup_escape_text(&t.describe()).to_string()).as_deref(),
        );
        self.eject.set_visible(is_drive);
        self.results.set_visible(true);

        let mut rows: Vec<gtk::Widget> = Vec::new();
        if scan.holders.is_empty() {
            let row = adw::ActionRow::builder()
                .title("Nothing is using it")
                .subtitle(if is_drive { "You can eject it now." } else { "No app has it open." })
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));
            rows.push(row.upcast());
        }
        for holder in scan.holders {
            rows.push(self.holder_row(holder).upcast());
        }
        if scan.unreadable > 0 && !complete {
            let button = gtk::Button::builder().label("Check with Password").valign(gtk::Align::Center).build();
            let row = adw::ActionRow::builder()
                .title(format!(
                    "{} {} of other users or the system weren't checked",
                    scan.unreadable,
                    if scan.unreadable == 1 { "program" } else { "programs" }
                ))
                .subtitle("Servers and system services often run that way")
                .build();
            row.add_prefix(&gtk::Image::from_icon_name("dialog-password-symbolic"));
            row.add_suffix(&button);
            let usage = Rc::downgrade(self);
            button.connect_clicked(move |b| {
                let Some(usage) = usage.upgrade() else { return };
                let Some(target) = usage.target.borrow().clone() else { return };
                b.set_sensitive(false);
                let b = b.clone();
                glib::spawn_future_local(async move {
                    match gio::spawn_blocking(move || usage::scan_as_root(&target)).await {
                        Ok(Ok(scan)) => usage.show(scan, true),
                        Ok(Err(e)) => {
                            toast(&usage.toasts, format!("Not checked: {e}"));
                            b.set_sensitive(true);
                        }
                        Err(_) => b.set_sensitive(true),
                    }
                });
            });
            rows.push(row.upcast());
        }
        for row in &rows {
            self.results.add(row);
        }
        *self.rows.borrow_mut() = rows;
    }

    fn holder_row(self: &Rc<Self>, holder: Holder) -> adw::ActionRow {
        let (name, icon, app) = describe(&holder);
        let mut lines = vec![match &app {
            Some(app) if *app != name => format!("in {app} · PID {}", holder.pid),
            _ => format!("{} · PID {}", holder.command, holder.pid),
        }];
        if holder.uid != usage::current_uid() {
            lines[0].push_str(&format!(" · {}", usage::user_name(holder.uid)));
        }
        lines.extend(holder.uses.iter().take(SHOWN_USES).cloned());
        if holder.uses.len() > SHOWN_USES {
            lines.push(format!("and {} more", holder.uses.len() - SHOWN_USES));
        }
        if let Some(what) = holder.protected() {
            lines.push(format!("This is {what}, so Voltkit won't close it."));
        }
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&name))
            .subtitle(glib::markup_escape_text(&lines.join("\n")))
            .subtitle_lines(lines.len() as i32)
            .build();
        row.add_prefix(&gtk::Image::builder().gicon(&icon).pixel_size(32).build());
        if holder.protected().is_none() {
            let button = gtk::Button::builder().label("Close").valign(gtk::Align::Center).build();
            let forced = Rc::new(Cell::new(false));
            let usage = Rc::downgrade(self);
            button.connect_clicked(move |b| {
                if let Some(usage) = usage.upgrade() {
                    glib::spawn_future_local(usage.close(holder.clone(), name.clone(), b.clone(), forced.clone()));
                }
            });
            row.add_suffix(&button);
        }
        row
    }

    async fn close(self: Rc<Self>, holder: Holder, name: String, button: gtk::Button, forced: Rc<Cell<bool>>) {
        let force = forced.get();
        let (heading, body, action) = if force {
            (
                format!("Force Quit {name}?"),
                "It stops immediately, without a chance to save. Anything unsaved in it is lost.".to_owned(),
                "Force Quit",
            )
        } else {
            (
                format!("Close {name}?"),
                format!("Voltkit asks {name} to quit, like closing its window. Unsaved work in it may be lost."),
                "Close",
            )
        };
        if !confirm(&button, &heading, &body, action).await {
            return;
        }
        button.set_sensitive(false);
        let h = holder.clone();
        match gio::spawn_blocking(move || usage::close(&h, force)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                toast(&self.toasts, e.to_string());
                button.set_sensitive(true);
                return;
            }
            Err(_) => {
                button.set_sensitive(true);
                return;
            }
        }
        button.set_label(if force { "Stopping…" } else { "Closing…" });
        if wait_until_gone(std::slice::from_ref(&holder), GRACE).await {
            toast(&self.toasts, format!("{name} closed"));
            self.rescan();
        } else {
            // Still running: offer the stronger option.
            forced.set(true);
            button.set_label("Force Quit");
            button.add_css_class("destructive-action");
            button.set_sensitive(true);
        }
    }

    async fn eject(self: Rc<Self>) {
        let (Some(mount), Some(target)) = (self.mount.borrow().clone(), self.target.borrow().clone()) else { return };
        let scan = gio::spawn_blocking(move || usage::scan(&target)).await.unwrap_or_default();
        let closable: Vec<Holder> = scan.holders.into_iter().filter(|h| h.protected().is_none()).collect();
        if !closable.is_empty() {
            let names: Vec<String> = closable.iter().map(|h| describe(h).0).collect();
            let body = format!(
                "These apps are using {}: {}. Voltkit asks them to quit, like closing their windows, then ejects. \
                 Unsaved work in them may be lost.",
                mount.name(),
                names.join(", ")
            );
            if !confirm(&self.eject, &format!("Close {} and Eject?", plural_apps(closable.len())), &body, "Close and Eject").await {
                return;
            }
            for holder in &closable {
                let h = holder.clone();
                if let Ok(Err(e)) = gio::spawn_blocking(move || usage::close(&h, false)).await {
                    toast(&self.toasts, e.to_string());
                }
            }
            if !wait_until_gone(&closable, EJECT_GRACE).await {
                toast(&self.toasts, "Some apps didn't close. Use Force Quit on them, then eject.");
                self.rescan();
                return;
            }
        }
        let window = self.eject.root().and_downcast::<gtk::Window>();
        let operation = gtk::MountOperation::new(window.as_ref());
        let result = if mount.can_eject() {
            mount.eject_with_operation_future(gio::MountUnmountFlags::NONE, Some(&operation)).await
        } else {
            mount.unmount_with_operation_future(gio::MountUnmountFlags::NONE, Some(&operation)).await
        };
        match result {
            Ok(()) => {
                toast(&self.toasts, format!("{} ejected: you can unplug it", mount.name()));
                *self.mount.borrow_mut() = None;
                *self.target.borrow_mut() = None;
                self.results.set_visible(false);
                self.refresh_drives();
            }
            Err(e) => {
                toast(&self.toasts, format!("Could not eject: {}", e.message()));
                self.rescan();
            }
        }
    }
}

/// The name to show, its icon, and the desktop app it belongs to.
fn describe(holder: &Holder) -> (String, gio::Icon, Option<String>) {
    let info = holder.app_id.as_deref().and_then(crate::app::desktop_app);
    let fallback: gio::Icon = gio::ThemedIcon::new("application-x-executable").upcast();
    let Some(info) = info else { return (holder.command.clone(), fallback, None) };
    let app = info.display_name().to_string();
    let icon = info.icon().unwrap_or(fallback);
    // The app's own process, or something it started (a shell in a terminal)?
    let program = info.executable();
    let program = program.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let command = holder.command.to_lowercase();
    let main = !program.is_empty() && (program.starts_with(&command) || command.starts_with(&program));
    let name = if main { app.clone() } else { holder.command.clone() };
    (name, icon, Some(app))
}

fn plural_apps(n: usize) -> String {
    if n == 1 { "1 App".into() } else { format!("{n} Apps") }
}

/// Waits until none of `holders` is running, for at most `limit`.
async fn wait_until_gone(holders: &[Holder], limit: Duration) -> bool {
    let started = std::time::Instant::now();
    loop {
        if holders.iter().all(|h| !usage::is_running(h.pid, h.start)) {
            return true;
        }
        if started.elapsed() >= limit {
            return false;
        }
        glib::timeout_future(POLL).await;
    }
}

async fn confirm(parent: &impl IsA<gtk::Widget>, heading: &str, body: &str, action: &str) -> bool {
    let dialog = adw::AlertDialog::new(Some(heading), Some(body));
    dialog.add_responses(&[("cancel", "Cancel"), ("ok", action)]);
    dialog.set_response_appearance("ok", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");
    dialog.choose_future(Some(parent)).await == "ok"
}

const DEMO_DRIVE: &str = "/media/you/USB DRIVE";

/// A typical case, shown when VOLTKIT_USAGE_DEMO is set (for the README screenshots).
fn demo_scan() -> Scan {
    let holder = |pid, command: &str, app: &str, uses: &[&str]| Holder {
        pid,
        start: 0,
        uid: usage::current_uid(),
        command: command.into(),
        app_id: Some(app.into()),
        uses: uses.iter().map(|u| u.to_string()).collect(),
    };
    Scan {
        holders: vec![
            holder(4182, "nautilus", "org.gnome.Nautilus", &["is working in /media/you/USB DRIVE/Photos"]),
            holder(5310, "gnome-text-edit", "org.gnome.TextEditor", &["has /media/you/USB DRIVE/notes.txt open"]),
            holder(6027, "bash", "org.gnome.Terminal", &["is working in /media/you/USB DRIVE/projects"]),
        ],
        unreadable: 0,
    }
}
