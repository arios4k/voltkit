//! The GTK4 / libadwaita control center.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::config::Config;
use crate::modules::{self, color, compress, ocr, snippets};
use crate::{daemon, doctor, portal};

const APP_ID: &str = "io.github.arios4k.Voltkit";

pub fn run(config: Config) {
    let app = adw::Application::builder().application_id(APP_ID).build();
    let config = Rc::new(config);
    app.connect_activate(move |app| build_window(app, &config));
    // Arguments are handled by clap; don't let GTK parse them again.
    app.run_with_args::<&str>(&[]);
}

fn build_window(app: &adw::Application, config: &Rc<Config>) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }

    let toasts = adw::ToastOverlay::new();
    let stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).build();
    stack.add_named(&ocr_page(config, &toasts), Some("ocr"));
    stack.add_named(&color_page(&toasts), Some("color"));
    stack.add_named(&compress_page(config, &toasts), Some("compress"));
    stack.add_named(&snippets_page(config), Some("snippets"));
    toasts.set_child(Some(&stack));

    let content_view = adw::ToolbarView::new();
    content_view.add_top_bar(&adw::HeaderBar::new());
    content_view.add_top_bar(&daemon_banner(&toasts));
    content_view.add_top_bar(&doctor_banner(config));
    content_view.set_content(Some(&toasts));
    let content = adw::NavigationPage::new(&content_view, modules::ALL[0].title);

    let list = gtk::ListBox::builder().css_classes(["navigation-sidebar"]).build();
    for m in modules::ALL {
        let row = adw::ActionRow::builder().title(m.title).tooltip_text(m.summary).activatable(true).build();
        row.add_prefix(&gtk::Image::from_icon_name(m.icon));
        row.set_widget_name(m.id);
        list.append(&row);
    }
    let sidebar_view = adw::ToolbarView::new();
    sidebar_view.add_top_bar(&adw::HeaderBar::new());
    sidebar_view.set_content(Some(&gtk::ScrolledWindow::builder().child(&list).build()));
    let sidebar = adw::NavigationPage::new(&sidebar_view, "Voltkit");

    let split = adw::NavigationSplitView::builder().sidebar(&sidebar).content(&content).build();

    {
        let (stack, content, split) = (stack.clone(), content.clone(), split.clone());
        list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let id = row.widget_name();
            if let Some(m) = modules::ALL.iter().find(|m| m.id == id) {
                stack.set_visible_child_name(m.id);
                content.set_title(m.title);
                split.set_show_content(true);
            }
        });
    }
    list.select_row(list.row_at_index(0).as_ref());

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Voltkit")
        .default_width(920)
        .default_height(660)
        .width_request(360)
        .height_request(400)
        .content(&split)
        .build();

    let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 600sp").unwrap());
    breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
    window.add_breakpoint(breakpoint);

    window.present();
}

fn toast(toasts: &adw::ToastOverlay, msg: impl AsRef<str>) {
    toasts.add_toast(adw::Toast::new(msg.as_ref()));
}

fn daemon_running() -> bool {
    let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else {
        return false;
    };
    bus.call_sync(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameHasOwner",
        Some(&(daemon::BUS_NAME,).to_variant()),
        Some(glib::VariantTy::new("(b)").unwrap()),
        gio::DBusCallFlags::NONE,
        500,
        gio::Cancellable::NONE,
    )
    .ok()
    .and_then(|v| v.get::<(bool,)>())
    .is_some_and(|(owned,)| owned)
}

/// Shown when `voltkit doctor` finds something a feature needs. The checks
/// run on a worker thread so the window opens immediately.
fn doctor_banner(config: &Config) -> adw::Banner {
    let banner = adw::Banner::builder().button_label("Details").revealed(false).build();
    let config = config.clone();
    let b = banner.clone();
    glib::spawn_future_local(async move {
        let Ok(report) = gio::spawn_blocking(move || doctor::run(&config)).await else { return };
        // The service banner already covers a stopped daemon.
        let missing: Vec<doctor::Check> = report
            .missing(doctor::Need::Required)
            .filter(|c| c.feature != "Background Service")
            .cloned()
            .collect();
        if missing.is_empty() {
            return;
        }
        let features: std::collections::BTreeSet<&str> = missing.iter().map(|c| c.feature).collect();
        b.set_title(&format!(
            "{} feature{} need{} attention",
            features.len(),
            if features.len() == 1 { "" } else { "s" },
            if features.len() == 1 { "s" } else { "" },
        ));
        b.set_revealed(true);

        let commands = report.install_commands();
        b.connect_button_clicked(move |b| {
            let body: String = missing
                .iter()
                .map(|c| match &c.fix {
                    Some(doctor::Fix::Advice(advice)) => format!("• {}: {} ({advice})\n", c.feature, c.what),
                    _ => format!("• {}: {}\n", c.feature, c.what),
                })
                .collect();
            let body = format!("{}\n\nRun `voltkit doctor` for the full report.", body.trim_end());
            let dialog = adw::AlertDialog::new(Some("Needs Attention"), Some(&body));
            if !commands.is_empty() {
                let label = gtk::Label::builder()
                    .label(commands.join("\n"))
                    .selectable(true)
                    .wrap(true)
                    .xalign(0.0)
                    .css_classes(["monospace", "card"])
                    .build();
                label.set_margin_top(6);
                dialog.set_extra_child(Some(&label));
                dialog.add_response("copy", "Copy Command");
                dialog.set_response_appearance("copy", adw::ResponseAppearance::Suggested);
            }
            dialog.add_response("close", "Close");
            dialog.set_default_response(Some("close"));
            let text = commands.join("\n");
            dialog.connect_response(Some("copy"), move |d, _| d.clipboard().set_text(&text));
            dialog.present(Some(b));
        });
    });
    banner
}

/// Shown while the background service is off, since global shortcuts need it.
fn daemon_banner(toasts: &adw::ToastOverlay) -> adw::Banner {
    let banner = adw::Banner::builder()
        .title("The background service is off, so keyboard shortcuts won't work")
        .button_label("Start")
        .revealed(!daemon_running())
        .build();
    let toasts = toasts.clone();
    banner.connect_button_clicked(move |banner| {
        let spawned = std::env::current_exe().and_then(|exe| {
            std::process::Command::new(exe)
                .arg("daemon")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .spawn()
        });
        match spawned {
            Ok(mut child) => {
                // Reap it when it exits so it doesn't linger as a zombie.
                std::thread::spawn(move || child.wait());
                let (banner, toasts) = (banner.clone(), toasts.clone());
                glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                    let running = daemon_running();
                    banner.set_revealed(!running);
                    if !running {
                        toast(&toasts, "The background service failed to start");
                    }
                });
            }
            Err(e) => toast(&toasts, format!("Could not start the service: {e}")),
        }
    });
    banner
}

// ---------------------------------------------------------------- OCR

fn ocr_page(config: &Config, toasts: &adw::ToastOverlay) -> gtk::Widget {
    let button = gtk::Button::builder()
        .label("Capture Region")
        .halign(gtk::Align::Center)
        .css_classes(["pill", "suggested-action"])
        .build();
    let view = gtk::TextView::builder()
        .editable(false)
        .wrap_mode(gtk::WrapMode::WordChar)
        .monospace(true)
        .top_margin(12)
        .bottom_margin(12)
        .left_margin(12)
        .right_margin(12)
        .build();
    let scroller = gtk::ScrolledWindow::builder()
        .child(&view)
        .min_content_height(160)
        .css_classes(["card"])
        .build();

    let body = gtk::Box::new(gtk::Orientation::Vertical, 24);
    body.append(&button);
    body.append(&scroller);

    let status = adw::StatusPage::builder()
        .icon_name("edit-select-all-symbolic")
        .title("Text Extractor")
        .description(format!(
            "Drag a box around any text on screen; it is copied to the clipboard.\nShortcut: {}",
            config.hotkeys.ocr
        ))
        .child(&adw::Clamp::builder().child(&body).build())
        .build();

    let lang = config.ocr.lang.clone();
    let toasts = toasts.clone();
    button.connect_clicked(move |button| {
        let (button, view, toasts, lang) = (button.clone(), view.clone(), toasts.clone(), lang.clone());
        let window = button.root().and_downcast::<gtk::Window>();
        // Keep our own window out of the capture.
        if let Some(w) = &window {
            w.minimize();
        }
        button.set_sensitive(false);
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || portal::block_on(ocr::extract_text(&lang))).await;
            button.set_sensitive(true);
            if let Some(w) = &window {
                w.present();
            }
            match result {
                Ok(Ok(text)) => {
                    view.buffer().set_text(&text);
                    button.clipboard().set_text(&text);
                    toast(&toasts, "Text copied to clipboard");
                }
                Ok(Err(e)) => toast(&toasts, e.to_string()),
                Err(_) => toast(&toasts, "OCR worker crashed"),
            }
        });
    });

    status.upcast()
}

// ---------------------------------------------------------------- Color

/// An action row whose subtitle can be copied with a suffix button.
fn copy_row(title: &str, toasts: &adw::ToastOverlay) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).subtitle_selectable(true).build();
    let button = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let (r, toasts) = (row.clone(), toasts.clone());
    button.connect_clicked(move |b| {
        let value = r.subtitle().unwrap_or_default();
        b.clipboard().set_text(&value);
        toast(&toasts, format!("Copied {value}"));
    });
    row.add_suffix(&button);
    row
}

fn color_page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let initial = color::Rgb::parse("#3584E4").unwrap();
    let current = Rc::new(Cell::new(initial));

    let swatch = gtk::DrawingArea::builder()
        .content_width(40)
        .content_height(40)
        .valign(gtk::Align::Center)
        .css_classes(["card"])
        .build();
    {
        let current = current.clone();
        swatch.set_draw_func(move |_, cr, w, h| {
            let c = current.get();
            cr.set_source_rgb(c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0);
            cr.rectangle(0.0, 0.0, w as f64, h as f64);
            let _ = cr.fill();
        });
    }

    let entry = adw::EntryRow::builder().title("HEX or rgb()").text(initial.to_hex()).build();
    let pick = gtk::Button::builder()
        .icon_name("color-select-symbolic")
        .tooltip_text("Pick from Screen")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    entry.add_suffix(&pick);
    entry.add_suffix(&swatch);
    let input = adw::PreferencesGroup::builder()
        .title("Color")
        .description("Type a color, or press Enter to save it to history")
        .build();
    input.add(&entry);

    let hex = copy_row("HEX", toasts);
    let rgb = copy_row("RGB", toasts);
    let hsl = copy_row("HSL", toasts);
    let values = adw::PreferencesGroup::builder().title("Formats").build();
    for r in [&hex, &rgb, &hsl] {
        values.add(r);
    }

    let on_white = adw::ActionRow::builder().title("Text on white").build();
    let on_black = adw::ActionRow::builder().title("Text on black").build();
    let contrast = adw::PreferencesGroup::builder().title("WCAG Contrast").build();
    contrast.add(&on_white);
    contrast.add(&on_black);

    let history = adw::PreferencesGroup::builder().title("History").build();
    let history_rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::default();

    let refresh = move |c: color::Rgb| {
        hex.set_subtitle(&c.to_hex());
        rgb.set_subtitle(&c.to_rgb_string());
        hsl.set_subtitle(&c.to_hsl_string());
        for (row, bg) in [(&on_white, color::Rgb::WHITE), (&on_black, color::Rgb::BLACK)] {
            let ratio = c.contrast_ratio(bg);
            row.set_subtitle(&format!("{ratio:.2}:1 — {}", color::wcag_rating(ratio)));
        }
    };
    refresh(initial);

    {
        let (current, swatch) = (current.clone(), swatch.clone());
        entry.connect_changed(move |e| match color::Rgb::parse(&e.text()) {
            Ok(c) => {
                e.remove_css_class("error");
                current.set(c);
                swatch.queue_draw();
                refresh(c);
            }
            Err(_) => e.add_css_class("error"),
        });
    }

    // Adds `c` to the top of the history group, moving it if already present.
    let remember: Rc<dyn Fn(color::Rgb)> = {
        let (entry, history) = (entry.clone(), history.clone());
        Rc::new(move |c: color::Rgb| add_history_row(&history, &history_rows, &entry, c))
    };
    for c in color::history::load().into_iter().rev() {
        remember(c);
    }

    {
        let (remember, toasts) = (remember.clone(), toasts.clone());
        entry.connect_entry_activated(move |_| {
            let c = current.get();
            remember(c);
            if let Err(e) = color::history::push(c) {
                toast(&toasts, format!("Could not save history: {e}"));
            }
        });
    }

    {
        let (entry, toasts) = (entry.clone(), toasts.clone());
        pick.connect_clicked(move |button| {
            let (button, entry, toasts, remember) = (button.clone(), entry.clone(), toasts.clone(), remember.clone());
            button.set_sensitive(false);
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(|| portal::block_on(portal::pick_color())).await;
                button.set_sensitive(true);
                match result {
                    Ok(Ok(c)) => {
                        entry.set_text(&c.to_hex());
                        remember(c);
                        let _ = color::history::push(c);
                        button.clipboard().set_text(&c.to_hex());
                        toast(&toasts, format!("Copied {}", c.to_hex()));
                    }
                    Ok(Err(e)) if e.is::<portal::Cancelled>() => {}
                    Ok(Err(e)) => toast(&toasts, format!("Color picker unavailable: {e}")),
                    Err(_) => toast(&toasts, "Color picker crashed"),
                }
            });
        });
    }

    let page = adw::PreferencesPage::new();
    for g in [&input, &values, &contrast, &history] {
        page.add(g);
    }
    page.upcast()
}

const HISTORY_ROWS: usize = 8;

fn add_history_row(
    group: &adw::PreferencesGroup,
    rows: &RefCell<Vec<adw::ActionRow>>,
    entry: &adw::EntryRow,
    c: color::Rgb,
) {
    let mut rows = rows.borrow_mut();
    if let Some(i) = rows.iter().position(|r| r.title() == c.to_hex()) {
        group.remove(&rows.remove(i));
    }
    {
        let row = adw::ActionRow::builder().title(c.to_hex()).activatable(true).build();
        let dot = gtk::DrawingArea::builder().content_width(16).content_height(16).build();
        dot.set_draw_func(move |_, cr, w, h| {
            cr.set_source_rgb(c.r as f64 / 255.0, c.g as f64 / 255.0, c.b as f64 / 255.0);
            cr.arc(w as f64 / 2.0, h as f64 / 2.0, w.min(h) as f64 / 2.0, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
        });
        row.add_prefix(&dot);
        let e = entry.clone();
        row.connect_activated(move |r| e.set_text(&r.title()));
        // PreferencesGroup only appends, so rebuild to keep newest first.
        for r in rows.iter() {
            group.remove(r);
        }
        rows.insert(0, row);
        rows.truncate(HISTORY_ROWS);
        for r in rows.iter() {
            group.add(r);
        }
    }
}

// ---------------------------------------------------------------- Compressor

fn compress_page(config: &Config, toasts: &adw::ToastOverlay) -> gtk::Widget {
    let quality = adw::SpinRow::with_range(1.0, 100.0, 1.0);
    quality.set_title("Quality");
    quality.set_subtitle("Lower is smaller. PNG is always lossless.");
    quality.set_value(config.compress.quality as f64);

    let choose = gtk::Button::builder()
        .label("Choose Files…")
        .halign(gtk::Align::Center)
        .css_classes(["pill"])
        .build();
    let drop_zone = adw::StatusPage::builder()
        .icon_name("folder-download-symbolic")
        .title("Drop Files Here")
        .description("PNG, JPEG, WebP and MP4. Metadata is stripped; originals are kept.")
        .child(&choose)
        .css_classes(["card", "compact"])
        .build();

    let settings = adw::PreferencesGroup::new();
    settings.add(&quality);
    let results = adw::PreferencesGroup::builder().title("Results").build();

    let process: Rc<dyn Fn(PathBuf)> = {
        let (quality, results, toasts) = (quality.clone(), results.clone(), toasts.clone());
        Rc::new(move |path: PathBuf| {
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let row = adw::ActionRow::builder().title(&name).subtitle("Compressing…").build();
            let spinner = gtk::Spinner::builder().spinning(true).build();
            row.add_suffix(&spinner);
            results.add(&row);
            let q = quality.value() as u8;
            let toasts = toasts.clone();
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || compress::compress(&path, q)).await;
                row.remove(&spinner);
                match result {
                    Ok(Ok(r)) => row.set_subtitle(&format!(
                        "{} → {} ({:+.0}%)",
                        glib::format_size(r.input_size),
                        glib::format_size(r.output_size),
                        r.change_percent()
                    )),
                    Ok(Err(e)) => {
                        row.set_subtitle("Failed");
                        toast(&toasts, format!("{name}: {e}"));
                    }
                    Err(_) => row.set_subtitle("Failed"),
                }
            });
        })
    };

    let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    {
        let process = process.clone();
        target.connect_drop(move |_, value, _, _| {
            let Ok(list) = value.get::<gdk::FileList>() else { return false };
            for path in list.files().iter().filter_map(|f| f.path()) {
                process(path);
            }
            true
        });
    }
    drop_zone.add_controller(target);

    choose.connect_clicked(move |b| {
        let process = process.clone();
        let parent = b.root().and_downcast::<gtk::Window>();
        gtk::FileDialog::builder().title("Compress Files").build().open_multiple(
            parent.as_ref(),
            gio::Cancellable::NONE,
            move |res| {
                let Ok(model) = res else { return };
                for i in 0..model.n_items() {
                    if let Some(path) = model.item(i).and_downcast::<gio::File>().and_then(|f| f.path()) {
                        process(path);
                    }
                }
            },
        );
    });

    let body = gtk::Box::new(gtk::Orientation::Vertical, 24);
    body.append(&drop_zone);
    body.append(&settings);
    body.append(&results);

    let clamp = adw::Clamp::builder()
        .child(&body)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(12)
        .margin_end(12)
        .build();
    gtk::ScrolledWindow::builder().child(&clamp).build().upcast()
}

// ---------------------------------------------------------------- Snippets

fn snippets_page(config: &Config) -> gtk::Widget {
    let list = adw::PreferencesGroup::builder()
        .title("Snippets")
        .description("Edit ~/.config/voltkit/config.toml to add your own")
        .build();
    for s in &config.snippets {
        let row = adw::ActionRow::builder()
            .title(&s.trigger)
            .subtitle(&s.template)
            .css_classes(["property"])
            .build();
        list.add(&row);
    }

    let template = adw::EntryRow::builder()
        .title("Template")
        .text("Meeting notes — {date:%A, %B %d} ({arg:topic})")
        .build();
    let preview = adw::ActionRow::builder().title("Preview").subtitle_selectable(true).build();
    let playground = adw::PreferencesGroup::builder()
        .title("Playground")
        .description("Variables: {date}, {date:FMT}, {time}, {clipboard}, {arg:NAME}")
        .build();
    playground.add(&template);
    playground.add(&preview);

    let update = move |e: &adw::EntryRow| {
        let text = e.text();
        let ctx = snippets::Context {
            clipboard: Some("‹clipboard›".into()),
            args: snippets::required_args(&text).into_iter().map(|a| (a.clone(), format!("‹{a}›"))).collect(),
            ..Default::default()
        };
        match snippets::expand(&text, &ctx) {
            Ok(s) => {
                e.remove_css_class("error");
                preview.set_subtitle(&glib::markup_escape_text(&s));
            }
            Err(err) => {
                e.add_css_class("error");
                preview.set_subtitle(&glib::markup_escape_text(&err.to_string()));
            }
        }
    };
    update(&template);
    template.connect_changed(update);

    let usage = adw::PreferencesGroup::builder().title("Insert Snippets").build();
    let open_picker = gtk::Button::builder()
        .label("Open")
        .valign(gtk::Align::Center)
        .build();
    open_picker.connect_clicked(|_| {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).arg("picker").spawn().map(|mut c| {
                std::thread::spawn(move || c.wait());
            });
        }
    });
    let picker_row = adw::ActionRow::builder()
        .title("Snippet picker")
        .subtitle(format!("Search and paste into any app. Shortcut: {}", config.hotkeys.snippets))
        .build();
    picker_row.add_suffix(&open_picker);
    usage.add(&picker_row);
    let typed = if config.expander.typed_triggers {
        "On: type a trigger in any app to expand it"
    } else {
        "Off: run `voltkit expander setup`, then `voltkit expander enable`"
    };
    usage.add(&adw::ActionRow::builder().title("Typed triggers").subtitle(typed).build());

    let page = adw::PreferencesPage::new();
    page.add(&usage);
    page.add(&playground);
    page.add(&list);
    page.upcast()
}
