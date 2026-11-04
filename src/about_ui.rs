//! Branded About dialog, sharing the workspace palette and native dialog behavior.
use adw::prelude::*;
use gtk::{gio, glib};
use crate::i18n::tr;

pub fn dialog() -> adw::Dialog {
    let toasts = adw::ToastOverlay::new();
    let body = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(14)
        .css_classes(["about-body"]).build();
    let hero = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6)
        .css_classes(["about-hero"]).build();
    hero.append(&gtk::Image::builder().icon_name(crate::portal::APP_ID).pixel_size(72)
        .tooltip_text("Voltkit").build());
    hero.append(&gtk::Label::builder().label("Voltkit").css_classes(["about-title"]).build());
    hero.append(&gtk::Label::builder().label(tr("Power tools for your Linux desktop."))
        .wrap(true).justify(gtk::Justification::Center).css_classes(["dim-label"]).build());
    let version = gtk::Button::builder().halign(gtk::Align::Center)
        .tooltip_text(tr("Copy version information"))
        .css_classes(["flat", "about-version"])
        .child(&adw::ButtonContent::builder().label(trf!("Version {}", env!("CARGO_PKG_VERSION")))
            .icon_name("edit-copy-symbolic").build()).build();
    let toast_overlay = toasts.clone();
    version.connect_clicked(move |button| {
        button.clipboard().set_text(concat!("Voltkit ", env!("CARGO_PKG_VERSION")));
        toast_overlay.add_toast(adw::Toast::new(tr("Version information copied")));
    });
    hero.append(&version);
    body.append(&hero);

    let highlights = gtk::FlowBox::builder().selection_mode(gtk::SelectionMode::None)
        .homogeneous(true).min_children_per_line(1).max_children_per_line(3)
        .column_spacing(8).row_spacing(8).halign(gtk::Align::Center).build();
    for (icon, text) in [("applications-utilities-symbolic", trf!("{} everyday tools", crate::modules::ALL.len())),
        ("computer-symbolic", tr("Native Linux app").into()), ("emblem-shared-symbolic", tr("Open source").into())] {
        let item = gtk::Box::builder().spacing(6).css_classes(["about-chip"]).build();
        item.append(&gtk::Image::from_icon_name(icon));
        item.append(&gtk::Label::new(Some(&text)));
        highlights.insert(&item, -1);
    }
    body.append(&highlights);

    let project = adw::PreferencesGroup::builder().title(tr("Explore Voltkit")).build();
    project.add(&link(tr("Project & source code"), tr("Documentation, releases, and the code behind Voltkit"), "web-browser-symbolic",
        env!("CARGO_PKG_REPOSITORY"), &toasts));
    project.add(&link(tr("Report an issue"), tr("Share a bug or suggest an improvement"), "dialog-information-symbolic",
        concat!(env!("CARGO_PKG_REPOSITORY"), "/issues"), &toasts));
    body.append(&project);

    let credits = adw::PreferencesGroup::builder().title(tr("Credits and license")).build();
    credits.add(&adw::ActionRow::builder().title(tr("Created by"))
        .subtitle(glib::markup_escape_text(env!("CARGO_PKG_AUTHORS"))).subtitle_selectable(true).build());
    let license = adw::ActionRow::builder().title(tr("MIT License")).subtitle(tr("Free to use, modify, and share"))
        .activatable(true).build();
    license.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    license.connect_activated(show_license);
    credits.add(&license);
    body.append(&credits);
    body.append(&gtk::Label::builder().label(format!("© 2026 {}", env!("CARGO_PKG_AUTHORS")))
        .wrap(true).justify(gtk::Justification::Center).css_classes(["caption", "dim-label"]).build());

    let scroll = gtk::ScrolledWindow::builder().child(&body)
        .hscrollbar_policy(gtk::PolicyType::Never).build();
    toasts.set_child(Some(&scroll));
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&toasts));
    adw::Dialog::builder().title(tr("About Voltkit")).content_width(540).content_height(680)
        .child(&view).css_classes(["voltkit-about"]).build()
}

fn link(title: &str, subtitle: &str, icon: &str, uri: &'static str, toasts: &adw::ToastOverlay) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(glib::markup_escape_text(title)).subtitle(glib::markup_escape_text(subtitle)).subtitle_lines(2).activatable(true).build();
    row.add_prefix(&gtk::Image::from_icon_name(icon));
    row.add_suffix(&gtk::Image::builder().icon_name("adw-external-link-symbolic").tooltip_text(tr("Open in browser")).build());
    let toasts = toasts.clone();
    row.connect_activated(move |row| {
        let parent = row.root().and_downcast::<gtk::Window>();
        let toasts = toasts.clone();
        gtk::UriLauncher::new(uri).launch(parent.as_ref(), gio::Cancellable::NONE, move |result| {
            if let Err(error) = result {
                toasts.add_toast(adw::Toast::new(&trf!("Could not open link: {}", error)));
            }
        });
    });
    row
}

fn show_license(parent: &impl IsA<gtk::Widget>) {
    let text = gtk::TextView::builder().editable(false).cursor_visible(false)
        .wrap_mode(gtk::WrapMode::WordChar).top_margin(20).bottom_margin(20).left_margin(24).right_margin(24).build();
    text.buffer().set_text(include_str!("../LICENSE"));
    let scroll = gtk::ScrolledWindow::builder().child(&text).hscrollbar_policy(gtk::PolicyType::Never).build();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&scroll));
    adw::Dialog::builder().title(tr("MIT License")).content_width(500).content_height(460)
        .child(&view).build().present(Some(parent));
}
