//! Branded About dialog, sharing the workspace palette and native dialog behavior.
use adw::prelude::*;
use gtk::{gio, glib};
use crate::i18n::tr;

pub fn dialog() -> adw::Dialog {
    let toasts = adw::ToastOverlay::new();
    let body = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(10)
        .css_classes(["about-body"]).build();
    let hero = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(6)
        .css_classes(["about-hero"]).build();
    hero.append(&gtk::Image::builder().icon_name(crate::portal::APP_ID).pixel_size(56)
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

    let project = adw::PreferencesGroup::builder().title(tr("Explore Voltkit")).build();
    project.add(&link(tr("Project & source code"), tr("Documentation, releases, and the code behind Voltkit"), "web-browser-symbolic",
        env!("CARGO_PKG_REPOSITORY"), &toasts));
    project.add(&link(tr("Report an issue"), tr("Share a bug or suggest an improvement"), "dialog-information-symbolic",
        concat!(env!("CARGO_PKG_REPOSITORY"), "/issues"), &toasts));

    // The author's other apps, with their own icons.
    crate::app::add_embedded_icons();
    project.add(&app_link("ThermalPrint", tr("Universal shipping label printer for Linux"), "org.thermalprint.ThermalPrint",
        "https://github.com/arios4k/thermalprint", &toasts));
    project.add(&app_link("Battery Health Center", tr("Battery health, charge limits and energy usage in one place, for GNOME"),
        "io.github.BatteryHealthCenter", "https://github.com/arios4k/battery-health-center", &toasts));
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
    toasts.set_child(Some(&body));
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&toasts));
    adw::Dialog::builder().title(tr("About Voltkit")).content_width(540).follows_content_size(true)
        .child(&view).css_classes(["voltkit-about"]).build()
}

/// A link to one of the author's other apps, shown with the app's own icon.
fn app_link(name: &'static str, subtitle: &str, icon: &str, uri: &'static str, toasts: &adw::ToastOverlay) -> adw::ActionRow {
    let image = gtk::Image::builder().icon_name(icon).pixel_size(32).accessible_role(gtk::AccessibleRole::Presentation).build();
    let row = link_row(name, subtitle, image, uri, toasts);
    row.set_tooltip_text(Some(&trf!("Open {} on GitHub", name)));
    row
}

fn link(title: &str, subtitle: &str, icon: &str, uri: &'static str, toasts: &adw::ToastOverlay) -> adw::ActionRow {
    link_row(title, subtitle, gtk::Image::from_icon_name(icon), uri, toasts)
}

fn link_row(title: &str, subtitle: &str, prefix: gtk::Image, uri: &'static str, toasts: &adw::ToastOverlay) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(glib::markup_escape_text(title)).subtitle(glib::markup_escape_text(subtitle)).subtitle_lines(1).activatable(true).build();
    row.add_prefix(&prefix);
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
