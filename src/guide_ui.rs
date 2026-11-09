//! The Shortcut Guide window: a cheat sheet of the shortcuts, opened by a hotkey
//! or from Shortcut Detective, read at a glance and closed with Esc.

use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::i18n::tr;
use crate::modules::guide::{self, Section};
use crate::modules::keybindings as kb;
use crate::portal::APP_ID;

pub fn run() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        // Open alongside the control center instead of activating it.
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(|app| {
        crate::appearance::install();
        let _ = build(app);
    });
    app.run_with_args::<&str>(&[]);
}

/// The keys of one shortcut as key caps, with alternatives side by side.
fn keys_widget(keys: &[String]) -> gtk::Box {
    let out = gtk::Box::builder().spacing(10).halign(gtk::Align::End).valign(gtk::Align::Center).build();
    for (n, accel) in keys.iter().enumerate() {
        if n > 0 {
            out.append(&gtk::Label::builder().label(tr("or")).css_classes(["dim-label", "caption"]).build());
        }
        let group = gtk::Box::builder().spacing(4).build();
        for key in kb::label(accel).split('+') {
            group.append(&gtk::Label::builder().label(key.trim()).css_classes(["keycap"]).build());
        }
        out.append(&group);
    }
    out
}

fn card(section: &Section) -> gtk::Box {
    let card = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8).css_classes(["detective-card"]).valign(gtk::Align::Start).build();
    card.append(&gtk::Label::builder().label(&section.title).xalign(0.0).wrap(true).css_classes(["heading"]).build());
    for row in &section.rows {
        let line = gtk::Box::builder().spacing(12).build();
        line.append(&gtk::Label::builder().label(&row.action).xalign(0.0).hexpand(true).wrap(true).build());
        line.append(&keys_widget(&row.keys));
        card.append(&line);
    }
    card
}

pub fn build(app: &adw::Application) -> adw::ApplicationWindow {
    let bindings = Rc::new(if kb::supported() { kb::all() } else { Vec::new() });
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(false)
        .min_children_per_line(1)
        .max_children_per_line(3)
        .column_spacing(16)
        .row_spacing(16)
        .valign(gtk::Align::Start)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    let empty = adw::StatusPage::builder().icon_name("edit-find-symbolic").title(tr("No shortcuts match")).description(tr("Try another action, group or key.")).build();
    let stack = gtk::Stack::new();
    stack.add_named(&gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&flow).build(), Some("list"));
    stack.add_named(&empty, Some("empty"));

    let search = gtk::SearchEntry::builder().placeholder_text(tr("Type to find a shortcut…")).hexpand(true).width_chars(36).build();
    let fill = {
        let (flow, stack, bindings) = (flow.clone(), stack.clone(), bindings.clone());
        move |filter: &str| {
            while let Some(child) = flow.first_child() {
                flow.remove(&child);
            }
            let sections = guide::sections(&bindings, filter);
            for section in &sections {
                flow.insert(&card(section), -1);
            }
            stack.set_visible_child_name(if sections.is_empty() { "empty" } else { "list" });
        }
    };
    fill("");
    {
        let fill = fill.clone();
        search.connect_search_changed(move |s| fill(&s.text()));
    }

    let header = adw::HeaderBar::builder().title_widget(&search).build();
    let note = gtk::Label::builder()
        .label(tr("System-wide shortcuts. Apps can have more of their own. Press Esc to close."))
        .css_classes(["dim-label", "caption"])
        .margin_bottom(8)
        .build();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(&stack));
    view.add_bottom_bar(&note);
    let window = adw::ApplicationWindow::builder().application(app).title(tr("Shortcut Guide")).default_width(1180).default_height(780).content(&view).build();

    let keys = gtk::EventControllerKey::new();
    {
        let window = window.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                window.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    window.add_controller(keys);
    window.present();
    search.grab_focus();
    window
}
