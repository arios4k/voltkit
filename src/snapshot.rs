//! Tools that work on a frozen screenshot (Screen Ruler, Zoom & Annotate).
//!
//! Wayland doesn't let apps draw over other windows, so these tools take a
//! screenshot through the desktop's portal and show it full screen.

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::i18n::tr;
use crate::portal::{self, APP_ID, Cancelled};

/// What differs between the tools that open on a screenshot.
pub struct Tool {
    /// The window title, and the title of notifications. Mark with `N_!`.
    pub title: &'static str,
    /// The heading and the text of the permission window shown the first time.
    pub allow_title: &'static str,
    pub allow_text: &'static str,
    /// Shown when the screenshot was refused.
    pub denied_text: &'static str,
    /// Opens the tool's window on the screenshot.
    pub show: fn(&adw::Application, gdk::Texture),
}

pub fn run(tool: &'static Tool) {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| {
        // Hold the app open while the screenshot is taken, before any window exists.
        let (app, hold) = (app.clone(), app.hold());
        glib::spawn_future_local(async move {
            // When opened from the control center, let its window finish
            // minimizing so it isn't in the screenshot.
            glib::timeout_future(SETTLE).await;
            match capture(None).await {
                Ok(texture) => (tool.show)(&app, texture),
                // GNOME asks "Allow screenshots?" only for the focused app, and
                // the ruler has no window yet, so a first request is refused
                // without a prompt. Ask from a window of our own.
                Err(e) if portal::is_refused(&e) => ask_permission(&app, tool),
                Err(e) => fail(tool, &format!("{e:#}")).await,
            }
            drop(hold);
        });
    });
    app.run_with_args::<&str>(&[]);
}

/// Time for windows to finish hiding before a screenshot.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(400);

async fn capture(parent: Option<String>) -> anyhow::Result<gdk::Texture> {
    let path = gio::spawn_blocking(move || portal::block_on(portal::screenshot_full(parent.as_deref())))
        .await
        .map_err(|_| anyhow::anyhow!("{}", tr("screenshot worker crashed")))??;
    let texture = gdk::Texture::from_filename(&path);
    let _ = std::fs::remove_file(&path);
    Ok(texture?)
}

/// Asks for the screenshot permission from a focused window, then takes a
/// clean screenshot without it and opens the ruler.
fn ask_permission(app: &adw::Application, tool: &'static Tool) {
    let button = gtk::Button::builder()
        .label(tr("Continue"))
        .halign(gtk::Align::Center)
        .css_classes(["suggested-action", "pill"])
        .build();
    let status = adw::StatusPage::builder()
        .icon_name("video-display-symbolic")
        .title(tr(tool.allow_title))
        .description(tr(tool.allow_text))
        .child(&button)
        .build();
    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&status));
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title(tr(tool.title))
        .default_width(520)
        .default_height(560)
        .content(&view)
        .build();

    let (app, prompt) = (app.clone(), window.clone());
    button.connect_clicked(move |button| {
        let (app, window, status, button) = (app.clone(), prompt.clone(), status.clone(), button.clone());
        button.set_sensitive(false);
        glib::spawn_future_local(async move {
            let (parent, guard) = portal::parent_handle(&window).await;
            // This first screenshot shows this window, so it's only for the permission.
            match capture(parent).await {
                Ok(_) => {
                    drop(guard);
                    let hold = app.hold();
                    window.close();
                    glib::timeout_future(SETTLE).await;
                    match capture(None).await {
                        Ok(texture) => (tool.show)(&app, texture),
                        Err(e) if e.is::<Cancelled>() => {}
                        Err(e) => fail(tool, &format!("{e:#}")).await,
                    }
                    drop(hold);
                }
                Err(e) => {
                    status.set_description(Some(&if portal::is_refused(&e) {
                        tr(tool.denied_text).to_owned()
                    } else {
                        glib::markup_escape_text(&format!("{e:#}")).to_string()
                    }));
                    button.set_label(tr("Try Again"));
                    button.set_sensitive(true);
                }
            }
        });
    });
    window.present();
}

async fn fail(tool: &'static Tool, message: &str) {
    let message = message.to_owned();
    let _ = gio::spawn_blocking(move || portal::block_on(portal::notify(tr(tool.title), &message))).await;
}

