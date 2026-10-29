//! `voltkit screenshots DIR` (hidden): renders the control center and the
//! snippet picker with demo content to PNGs, in light and dark, for the README.
//!
//! Run it through `docs/screenshots/capture.sh`.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use anyhow::{Context, Result};
use gtk::{gdk, gio, glib, graphene, gsk};

use crate::config::Config;
use crate::modules;
use crate::{app, picker};

/// Rendered at 2× so the images stay sharp on high-density screens.
const SCALE: f32 = 2.0;
/// Time for layout, icons and the theme switch to settle before a capture.
const SETTLE: Duration = Duration::from_millis(600);

/// Text "read" from a photo of a conference slide: clearly fictional, and the
/// kind of thing people actually want to copy (a password, a link).
const OCR_DEMO: &str = "\
Keynote: The Future of the Linux Desktop
Room B · Saturday 10:30 AM

Wi-Fi network: LinuxFest-Guest
Password: open-source-rocks-2026

Slides and recording:
https://linuxfest.example.org/keynote";

/// File name, size before, size after, as measured on test files.
const COMPRESS_DEMO: &[(&str, &str)] = &[
    ("vacation-sunset.jpg", "4.8 MB → 2.7 MB (−43%)"),
    ("dashboard-screenshot.png", "1.1 MB → 847 kB (−23%)"),
    ("hero-banner.webp", "560 kB → 224 kB (−60%)"),
    ("product-demo.mp4", "38.2 MB → 8.8 MB (−77%)"),
];

pub fn run(config: Config, dir: PathBuf) -> Result<()> {
    std::fs::create_dir_all(&dir)?;
    let app = adw::Application::builder()
        .application_id("io.github.arios4k.Voltkit.Screenshots")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let config = Rc::new(config);
    let result: Rc<std::cell::RefCell<Result<()>>> = Rc::new(std::cell::RefCell::new(Ok(())));
    {
        let result = result.clone();
        app.connect_activate(move |app| {
            let (app, config, dir, result) = (app.clone(), config.clone(), dir.clone(), result.clone());
            // Keep the app alive while the async capture runs.
            let hold = app.hold();
            glib::spawn_future_local(async move {
                *result.borrow_mut() = capture(&app, &config, &dir).await;
                drop(hold);
                app.quit();
            });
        });
    }
    app.run_with_args::<&str>(&[]);
    result.replace(Ok(()))
}

async fn capture(app: &adw::Application, config: &Rc<Config>, dir: &Path) -> Result<()> {
    let style = adw::StyleManager::default();
    // Kept at its default size: resizing a shown window on Broadway stalls its drawing.
    let window = app::build_window(app, config);
    fill_demo(&window);
    let sidebar = descendants(&window)
        .into_iter()
        .find_map(|w| w.downcast::<gtk::ListBox>().ok().filter(|l| l.has_css_class("navigation-sidebar")))
        .context("sidebar not found")?;

    for (scheme, theme) in [(adw::ColorScheme::ForceLight, "light"), (adw::ColorScheme::ForceDark, "dark")] {
        style.set_color_scheme(scheme);
        // Distributions that theme libadwaita (e.g. Zorin OS) need the matching
        // light or dark theme too, or the two passes mix styles.
        let var = if theme == "light" { "VOLTKIT_LIGHT_THEME" } else { "VOLTKIT_DARK_THEME" };
        if let (Ok(name), Some(settings)) = (std::env::var(var), gtk::Settings::default()) {
            settings.set_gtk_theme_name(Some(&name));
        }
        for (i, module) in modules::ALL.iter().enumerate() {
            sidebar.select_row(sidebar.row_at_index(i as i32).as_ref());
            hide_banners(&window);
            glib::timeout_future(SETTLE).await;
            save(&window, &dir.join(format!("{}-{theme}.png", module.id)), theme == "dark")?;
        }

        // The snippet editor, opened on a snippet with fill-in fields.
        sidebar.select_row(sidebar.row_at_index(3).as_ref());
        glib::timeout_future(SETTLE).await;
        if let Some(row) = descendants(&window)
            .into_iter()
            .filter_map(|w| w.downcast::<adw::ActionRow>().ok())
            .find(|r| r.title() == ";meet")
        {
            row.emit_by_name::<()>("activated", &[]);
            hide_banners(&window);
            glib::timeout_future(SETTLE).await;
            save(&window, &dir.join(format!("snippet-editor-{theme}.png")), theme == "dark")?;
            for dialog in descendants(&window).into_iter().filter_map(|w| w.downcast::<adw::Dialog>().ok()) {
                dialog.force_close();
            }
        }

        let about = app::about_dialog();
        about.present(Some(&window));
        hide_banners(&window);
        glib::timeout_future(SETTLE).await;
        save(&window, &dir.join(format!("about-{theme}.png")), theme == "dark")?;
        about.force_close();

        let popup = picker::build(app, config.clone(), None);
        resize(&popup, 560, 480);
        glib::timeout_future(SETTLE).await;
        save(&popup, &dir.join(format!("picker-{theme}.png")), theme == "dark")?;
        popup.close();
    }
    window.close();
    Ok(())
}

/// The builders show their windows right away, when a default size no longer
/// applies; a minimum size makes the shown window grow to the wanted size.
fn resize(window: &gtk::Window, width: i32, height: i32) {
    window.set_size_request(width, height);
}

/// Room around the window for its shadow, in logical pixels.
const MARGIN: f32 = 36.0;
const RADIUS: f32 = 12.0;

/// Renders a window's contents to a PNG, framed like a window: rounded
/// corners and a soft shadow on a transparent background. GTK won't draw a
/// toplevel itself into a texture, so the frame is added here.
fn save(window: &gtk::Window, path: &Path, dark: bool) -> Result<()> {
    let content = window.child().context("window has no content")?;
    let (w, h) = (content.width() as f32, content.height() as f32);
    let paintable = gtk::WidgetPaintable::new(Some(&content));
    let snapshot = gtk::Snapshot::new();
    snapshot.scale(SCALE, SCALE);
    snapshot.translate(&graphene::Point::new(MARGIN, MARGIN));
    let frame = gsk::RoundedRect::from_rect(graphene::Rect::new(0.0, 0.0, w, h), RADIUS);
    snapshot.append_outset_shadow(&frame, &gdk::RGBA::new(0.0, 0.0, 0.0, 0.32), 0.0, 10.0, 0.0, 26.0);
    snapshot.push_rounded_clip(&frame);
    // libadwaita paints the window background on the toplevel, which isn't
    // part of the content; use Adwaita's window_bg_color.
    let bg = if dark { gdk::RGBA::new(0.141, 0.141, 0.141, 1.0) } else { gdk::RGBA::new(0.98, 0.98, 0.98, 1.0) };
    snapshot.append_color(&bg, &graphene::Rect::new(0.0, 0.0, w, h));
    paintable.snapshot(&snapshot, w as f64, h as f64);
    snapshot.pop();
    snapshot.append_border(
        &frame,
        &[1.0; 4],
        &[gdk::RGBA::new(0.0, 0.0, 0.0, 0.12); 4],
    );
    let node = snapshot.to_node().context("nothing was drawn")?;
    let renderer = window.renderer().context("window has no renderer")?;
    let (full_w, full_h) = ((w + 2.0 * MARGIN) * SCALE, (h + 2.0 * MARGIN) * SCALE);
    let texture = renderer.render_texture(&node, Some(&graphene::Rect::new(0.0, 0.0, full_w, full_h)));
    texture.save_to_png(path).with_context(|| format!("cannot write {}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

fn descendants(root: &impl IsA<gtk::Widget>) -> Vec<gtk::Widget> {
    let mut out = Vec::new();
    let mut stack = vec![root.clone().upcast::<gtk::Widget>()];
    while let Some(w) = stack.pop() {
        let mut child = w.first_child();
        while let Some(c) = child {
            child = c.next_sibling();
            stack.push(c.clone());
        }
        out.push(w);
    }
    out
}

fn hide_banners(window: &gtk::Window) {
    for banner in descendants(window).into_iter().filter_map(|w| w.downcast::<adw::Banner>().ok()) {
        banner.set_revealed(false);
    }
}

/// Puts realistic content on the pages that start out empty.
fn fill_demo(window: &gtk::Window) {
    for w in descendants(window) {
        if let Some(view) = w.downcast_ref::<gtk::TextView>() {
            view.buffer().set_text(OCR_DEMO);
        } else if let Some(entry) = w.downcast_ref::<adw::EntryRow>().filter(|e| e.title() == "HEX or rgb()") {
            entry.set_text("#7C3AED");
        } else if let Some(entry) = w.downcast_ref::<adw::EntryRow>().filter(|e| e.title() == "Text or link") {
            entry.set_text("https://arios4k.github.io/voltkit");
        } else if let Some(group) = w.downcast_ref::<adw::PreferencesGroup>().filter(|g| g.title() == "Results") {
            for (name, sizes) in COMPRESS_DEMO {
                let row = adw::ActionRow::builder().title(*name).subtitle(*sizes).build();
                row.add_prefix(&gtk::Image::from_icon_name("emblem-ok-symbolic"));
                group.add(&row);
            }
        }
    }
}
