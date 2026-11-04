//! The App Permissions page: what the desktop remembers about each app's
//! requests, with a switch to change an answer and a button to forget it.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::app::toast;
use crate::modules::permissions::{self, Permission, State};
use crate::portal;
use crate::i18n::tr;

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = crate::workspace::Workspace::new();
    let view = View::new(toasts);
    page.add(&view.group);
    let search = gtk::SearchEntry::builder().placeholder_text(tr("Find an app or permission…")).build();
    page.banner(&search);
    let weak = Rc::downgrade(&view);
    search.connect_search_changed(move |entry| {
        if let Some(view) = weak.upgrade() {
            *view.filter.borrow_mut() = entry.text().to_lowercase();
            view.filter_rows();
        }
    });
    page.aside(&crate::workspace::note(tr("Choose what apps can access"), tr("Expand an app to review its remembered permissions. A switch changes the saved answer; Forget removes that answer so the app can ask again."), "security-high-symbolic"));
    page.aside(&crate::workspace::note(tr("Desktop permission history"), tr("These are answers stored by the desktop portal, including for apps installed outside Flatpak. An app appears after it has requested access. This is not a list of every capability an app may have."), "dialog-information-symbolic"));
    view.reload();
    // Rows only hold weak references; the page keeps the state alive.
    page.connect_destroy(move |_| {
        let _ = &view;
    });
    page.upcast()
}

struct View {
    group: adw::PreferencesGroup,
    rows: RefCell<Vec<gtk::Widget>>,
    filter: RefCell<String>,
    toasts: adw::ToastOverlay,
}

impl View {
    fn new(toasts: &adw::ToastOverlay) -> Rc<Self> {
        let refresh = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text(tr("Refresh"))
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let group = adw::PreferencesGroup::builder()
            .title(tr("Applications"))
            .description(
                tr("Answers you gave when apps asked to take screenshots, use the camera, see your location and more. \
                 Change an answer, or forget it so the app asks again next time."),
            )
            .header_suffix(&crate::app::spaced(&refresh))
            .build();
        let view = Rc::new(Self { group, rows: RefCell::default(), filter: RefCell::default(), toasts: toasts.clone() });
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
            match gio::spawn_blocking(|| portal::block_on(permissions::load())).await {
                Ok(Ok(list)) => view.show(list),
                Ok(Err(e)) => {
                    view.show(Vec::new());
                    toast(&view.toasts, trf!("Could not read permissions: {}", e));
                }
                Err(_) => view.show(Vec::new()),
            }
        });
    }

    fn show(self: &Rc<Self>, list: Vec<Permission>) {
        for row in self.rows.take() {
            self.group.remove(&row);
        }
        let mut rows: Vec<gtk::Widget> = Vec::new();
        if list.is_empty() {
            let row = adw::ActionRow::builder()
                .title(tr("No app has asked for anything yet"))
                .subtitle(tr("Answers appear here after an app asks, for example to take a screenshot"))
                .build();
            rows.push(row.upcast());
        }
        // `list` is sorted by app: one expandable row per app.
        let mut apps: Vec<(String, Vec<Permission>)> = Vec::new();
        for p in list {
            match apps.last_mut() {
                Some((app, perms)) if *app == p.app => perms.push(p),
                _ => apps.push((p.app.clone(), vec![p])),
            }
        }
        let mut apps: Vec<(String, gio::Icon, Vec<Permission>)> = apps
            .into_iter()
            .map(|(app, perms)| {
                let (name, icon) = app_name(&app);
                (name, icon, perms)
            })
            .collect();
        apps.sort_by_key(|(name, _, _)| name.to_lowercase());
        for (name, icon, perms) in apps {
            let expander = adw::ExpanderRow::builder()
                .title(glib::markup_escape_text(&name))
                .subtitle(glib::markup_escape_text(&summary(&perms)))
                .build();
            expander.add_prefix(&gtk::Image::builder().gicon(&icon).pixel_size(32).build());
            for p in perms {
                expander.add_row(&self.permission_row(p, &name));
            }
            // The README screenshots show the first app opened up.
            if rows.is_empty() && std::env::var_os("VOLTKIT_PERMISSIONS_FILE").is_some() {
                expander.set_expanded(true);
            }
            rows.push(expander.upcast());
        }
        for row in &rows {
            self.group.add(row);
        }
        *self.rows.borrow_mut() = rows;
        self.filter_rows();
    }

    fn filter_rows(&self) {
        let filter = self.filter.borrow();
        let mut matches = 0;
        for row in self.rows.borrow().iter() {
            let text = row.clone().downcast::<adw::ExpanderRow>().map(|r| {
                format!("{} {}", r.title(), r.subtitle()).to_lowercase()
            }).unwrap_or_default();
            let visible = filter.is_empty() || text.contains(filter.as_str());
            row.set_visible(visible);
            if visible { matches += 1; }
        }
        if !filter.is_empty() && matches == 0 {
            self.group.set_description(Some(tr("No apps match this search. Try an app name or a permission such as camera.")));
        } else {
            self.group.set_description(Some(tr("Expand an app to review its saved answers.")));
        }
    }

    fn permission_row(self: &Rc<Self>, p: Permission, app: &str) -> adw::ActionRow {
        let row = adw::ActionRow::builder().title(p.title()).subtitle(p.describe_state()).build();
        if p.state() != State::Other {
            let switch = gtk::Switch::builder()
                .active(p.state() == State::Allowed)
                .valign(gtk::Align::Center)
                .tooltip_text(tr("Allowed"))
                .build();
            let (weak, p, row_) = (Rc::downgrade(self), p.clone(), row.clone());
            switch.connect_state_set(move |_, allowed| {
                let Some(view) = weak.upgrade() else { return glib::Propagation::Proceed };
                let (p, row) = (p.clone(), row_.clone());
                glib::spawn_future_local(async move {
                    let q = p.clone();
                    match gio::spawn_blocking(move || portal::block_on(permissions::set_allowed(&q, allowed))).await {
                        Ok(Ok(())) => row.set_subtitle(if allowed { tr("Allowed") } else { tr("Denied") }),
                        Ok(Err(e)) => {
                            toast(&view.toasts, trf!("Not changed: {}", e));
                            view.reload();
                        }
                        Err(_) => view.reload(),
                    }
                });
                glib::Propagation::Proceed
            });
            row.add_suffix(&switch);
        }
        let forget = gtk::Button::builder()
            .icon_name("edit-undo-symbolic")
            .tooltip_text(tr("Forget, so the app asks again"))
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let (weak, app) = (Rc::downgrade(self), app.to_owned());
        forget.connect_clicked(move |_| {
            let Some(view) = weak.upgrade() else { return };
            let (p, app) = (p.clone(), app.clone());
            glib::spawn_future_local(async move {
                let title = p.title();
                match gio::spawn_blocking(move || portal::block_on(permissions::forget(&p))).await {
                    Ok(Ok(())) => toast(&view.toasts, trf!("{} will ask again: {}", app, title.to_lowercase())),
                    Ok(Err(e)) => toast(&view.toasts, trf!("Not forgotten: {}", e)),
                    Err(_) => {}
                }
                view.reload();
            });
        });
        row.add_suffix(&forget);
        row
    }
}

/// "Allowed: take screenshots · Denied: use the camera", shortened.
fn summary(perms: &[Permission]) -> String {
    let count = |s| perms.iter().filter(|p| p.state() == s).count();
    let mut parts = Vec::new();
    for (state, label) in [(State::Allowed, N_!("{} allowed")), (State::Denied, N_!("{} denied")), (State::Other, N_!("{} remembered"))] {
        match count(state) {
            0 => {}
            n => parts.push(crate::i18n::fill(tr(label), &[&n])),
        }
    }
    parts.join(" · ")
}

/// The app's name and icon from its desktop file, or a readable fallback.
fn app_name(app: &str) -> (String, gio::Icon) {
    let fallback: gio::Icon = gio::ThemedIcon::new("application-x-executable").upcast();
    if app.is_empty() {
        return (tr("Apps without an ID").into(), fallback);
    }
    match crate::app::desktop_app(app) {
        Some(info) => (info.display_name().to_string(), info.icon().unwrap_or(fallback)),
        None => (app.to_owned(), fallback),
    }
}
