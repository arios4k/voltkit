//! The GTK4 / libadwaita control center.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::config::Config;
use crate::modules::{self, color, compress, ocr};
use crate::{actions, awake_ui, daemon, detective_ui, gpu_ui, doctor, permissions_ui, portal, qr_ui, sleep_ui, smartpaste_ui, snippets_ui, sysreport_ui, system_ui, usage_ui};
use crate::i18n::tr;

const APP_ID: &str = "io.github.arios4k.Voltkit";

pub fn run(config: Config) {
    let app = adw::Application::builder().application_id(APP_ID).build();
    let about = gio::ActionEntry::builder("about")
        .activate(|app: &adw::Application, _, _| {
            about_dialog().present(app.active_window().as_ref());
        })
        .build();
    // Closes the window; the background service keeps the hotkeys working.
    let quit = gio::ActionEntry::builder("quit").activate(|app: &adw::Application, _, _| app.quit()).build();
    app.add_action_entries([about, quit]);
    app.set_accels_for_action("app.quit", &["<Control>q"]);
    let config = Rc::new(config);
    app.connect_activate(move |app| {
        // Right after installing, the background service isn't running until
        // the next login: start it now, so the hotkeys work straight away.
        if app.active_window().is_none() && !daemon_running() {
            let _ = start_daemon();
        }
        build_window(app, &config);
    });
    // Arguments are handled by clap; don't let GTK parse them again.
    app.run_with_args::<&str>(&[]);
}

/// Lets builds run from the source tree find the app icon without installing
/// it; installed copies get it from the icon theme.
pub(crate) fn add_source_icon_path() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    let icons = concat!(env!("CARGO_MANIFEST_DIR"), "/data/icons");
    ONCE.call_once(|| {
        if let Some(display) = gdk::Display::default().filter(|_| std::path::Path::new(icons).is_dir()) {
            gtk::IconTheme::for_display(&display).add_search_path(icons);
        }
    });
}

/// Icons that live inside the binary, so they show up however Voltkit was
/// installed: written once to the cache folder and added to the icon search path.
const GPU_ICON: &str = include_str!("../data/icons/voltkit-gpu-symbolic.svg");

pub(crate) fn add_embedded_icons() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let Some(display) = gdk::Display::default() else { return };
        let root = glib::user_cache_dir().join("voltkit/icons");
        let dir = root.join("hicolor/symbolic/apps");
        let file = dir.join("voltkit-gpu-symbolic.svg");
        if std::fs::read_to_string(&file).ok().as_deref() != Some(GPU_ICON) {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&file, GPU_ICON);
        }
        // The colour icons of the author's other apps, shown in the About box.
        let apps = root.join("hicolor/scalable/apps");
        for (name, svg) in SIBLING_ICONS {
            let file = apps.join(format!("{name}.svg"));
            if std::fs::read_to_string(&file).ok().as_deref() != Some(svg) {
                let _ = std::fs::create_dir_all(&apps);
                let _ = std::fs::write(&file, svg);
            }
        }
        gtk::IconTheme::for_display(&display).add_search_path(&root);
    });
}

/// The official icons of ThermalPrint and Battery Health Center, copied from their repositories.
const SIBLING_ICONS: [(&str, &str); 2] = [
    ("org.thermalprint.ThermalPrint", include_str!("../data/icons/org.thermalprint.ThermalPrint.svg")),
    ("io.github.BatteryHealthCenter", include_str!("../data/icons/io.github.BatteryHealthCenter.svg")),
];

pub(crate) fn build_window(app: &adw::Application, config: &Rc<Config>) -> gtk::Window {
    add_source_icon_path();
    add_embedded_icons();
    crate::appearance::install();
    if let Some(window) = app.active_window() {
        window.present();
        return window;
    }

    let toasts = adw::ToastOverlay::new();
    let stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).build();
    stack.add_named(&ocr_page(config, &toasts), Some("ocr"));
    stack.add_named(&qr_ui::page(config, &toasts), Some("qr"));
    stack.add_named(&color_page(config, &toasts), Some("color"));
    stack.add_named(&ruler_page(config), Some("ruler"));
    stack.add_named(&compress_page(config, &toasts), Some("compress"));
    stack.add_named(&snippets_ui::page(config, &toasts), Some("snippets"));
    stack.add_named(&smartpaste_ui::page(config, &toasts), Some("smartpaste"));
    stack.add_named(&awake_ui::page(&toasts), Some("awake"));
    stack.add_named(&system_ui::page(&toasts), Some("system"));
    stack.add_named(&usage_ui::page(&toasts), Some("using"));
    stack.add_named(&permissions_ui::page(&toasts), Some("permissions"));
    stack.add_named(&sleep_ui::page(&toasts), Some("sleep"));
    stack.add_named(&gpu_ui::page(&toasts), Some("gpu"));
    stack.add_named(&sysreport_ui::page(&toasts), Some("sysreport"));
    stack.add_named(&detective_ui::page(&toasts), Some("shortcuts"));
    toasts.set_child(Some(&stack));

    let content_view = adw::ToolbarView::new();
    content_view.add_css_class("tool-workspace");
    let tools = modules::sorted();
    let tool_heading = gtk::Label::builder().label(tools[0].title()).xalign(0.0).wrap(true).css_classes(["title-1"]).build();
    let tool_summary = gtk::Label::builder().label(tools[0].summary()).xalign(0.0).wrap(true).css_classes(["dim-label"]).build();
    let tool_icon = gtk::Image::builder().icon_name(tools[0].icon).pixel_size(32).valign(gtk::Align::Start).css_classes(["tool-icon"]).build();
    let titles = gtk::Box::new(gtk::Orientation::Vertical, 6);
    titles.append(&tool_heading);
    titles.append(&tool_summary);
    let hero = gtk::Box::builder().spacing(18).margin_start(24).margin_end(24).margin_top(12).margin_bottom(20).build();
    hero.append(&tool_icon);
    hero.append(&titles);
    content_view.add_top_bar(&adw::HeaderBar::new());
    content_view.add_top_bar(&hero);
    content_view.add_top_bar(&daemon_banner(&toasts));
    content_view.add_top_bar(&doctor_banner(config));
    content_view.set_content(Some(&toasts));
    let content = adw::NavigationPage::new(&content_view, tools[0].title());

    let list = gtk::ListBox::builder().css_classes(["navigation-sidebar"]).build();
    for m in &tools {
        // Compact rows, like other GNOME sidebars, so every tool fits without scrolling.
        let content = gtk::Box::builder().spacing(12).margin_top(2).margin_bottom(2).margin_start(4).build();
        content.append(&gtk::Image::from_icon_name(m.icon));
        content.append(&gtk::Label::builder().label(m.title()).xalign(0.0).ellipsize(gtk::pango::EllipsizeMode::End).build());
        let row = gtk::ListBoxRow::builder().child(&content).tooltip_text(m.summary()).build();
        row.set_widget_name(m.id);
        list.append(&row);
    }
    let sidebar_view = adw::ToolbarView::new();
    sidebar_view.add_css_class("tool-sidebar");
    let menu = gio::Menu::new();
    menu.append(Some(tr("About Voltkit")), Some("app.about"));
    let end = gio::Menu::new();
    end.append(Some(tr("Quit")), Some("app.quit"));
    menu.append_section(None, &end);
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(tr("Main Menu"))
        .menu_model(&menu)
        .primary(true)
        .build();
    let sidebar_header = adw::HeaderBar::new();
    sidebar_header.pack_end(&menu_button);
    sidebar_view.add_top_bar(&sidebar_header);
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
                content.set_title(m.title());
                tool_heading.set_label(m.title());
                tool_summary.set_label(m.summary());
                tool_icon.set_icon_name(Some(m.icon));
                split.set_show_content(true);
            }
        });
    }
    list.select_row(list.row_at_index(0).as_ref());

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Voltkit")
        .default_width(1240)
        .default_height(840)
        .width_request(360)
        .height_request(400)
        .content(&split)
        .build();

    let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 600sp").unwrap());
    breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
    window.add_breakpoint(breakpoint);

    window.present();
    window.upcast()
}

pub(crate) fn about_dialog() -> adw::Dialog {
    crate::about_ui::dialog()
}

/// "<Super><Shift>t" → "Super+Shift+T". The raw accelerator would also be read
/// as markup tags by labels that support markup, which blanks them.
pub(crate) fn shortcut_label(accel: &str) -> String {
    // "Super+Shift+T": the same wording as Shortcut Detective.
    glib::markup_escape_text(&modules::keybindings::label(accel)).to_string()
}

/// Starts `voltkit ARGS` for a tool with its own window (ruler, snippet picker),
/// passing an activation token from the click. GNOME raises a new window only
/// with such a token; without it the window opened behind the others.
pub(crate) fn spawn_window_tool(widget: &impl IsA<gtk::Widget>, args: &[&str]) {
    let Ok(exe) = std::env::current_exe() else { return };
    let command_line = std::iter::once(exe.as_os_str())
        .chain(args.iter().map(std::ffi::OsStr::new))
        .map(|a| glib::shell_quote(a).to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    // Launching through GIO with startup notification has GTK request the token
    // and hand it to the new process in XDG_ACTIVATION_TOKEN.
    let launched = gio::AppInfo::create_from_commandline(
        &command_line,
        Some("Voltkit"),
        gio::AppInfoCreateFlags::SUPPORTS_STARTUP_NOTIFICATION,
    )
    .and_then(|info| info.launch(&[], Some(&widget.display().app_launch_context())));
    if let Err(e) = launched {
        eprintln!("{}", trf!("could not start {}: {}", command_line, e));
    }
}

/// The installed app with this ID (the desktop file name without `.desktop`).
pub(crate) fn desktop_app(id: &str) -> Option<gio::AppInfo> {
    let file = format!("{id}.desktop");
    gio::AppInfo::all().into_iter().find(|a| a.id().is_some_and(|i| i.eq_ignore_ascii_case(&file)))
}

/// A group's header button, with room between it and the description beside it.
pub(crate) fn spaced<W: IsA<gtk::Widget>>(widget: &W) -> W {
    widget.set_margin_start(24);
    widget.clone()
}

pub(crate) fn toast(toasts: &adw::ToastOverlay, msg: impl AsRef<str>) {
    toasts.add_toast(adw::Toast::new(msg.as_ref()));
}

/// Calls `show` with each value the hotkey for `kind` captures ("ocr", "qr" or
/// "color"; see actions::announce) while `widget` exists.
pub(crate) fn on_captured(widget: &impl IsA<gtk::Widget>, kind: &str, show: impl Fn(String) + 'static) {
    let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE) else { return };
    let kind = kind.to_owned();
    #[allow(deprecated)]
    let id = bus.signal_subscribe(
        None,
        Some(actions::CAPTURE_INTERFACE),
        Some(actions::CAPTURE_SIGNAL),
        Some(actions::CAPTURE_PATH),
        Some(&kind),
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, params| {
            if let Some((_, value)) = params.get::<(String, String)>() {
                show(value);
            }
        },
    );
    let id = std::cell::Cell::new(Some(id));
    widget.connect_destroy(move |_| {
        if let Some(id) = id.take() {
            #[allow(deprecated)]
            bus.signal_unsubscribe(id);
        }
    });
}

/// Starts `voltkit daemon` in the background.
fn start_daemon() -> std::io::Result<()> {
    let mut child = std::process::Command::new(std::env::current_exe()?)
        .arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .spawn()?;
    // Reap it when it exits so it doesn't linger as a zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
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
    let banner = adw::Banner::builder().button_label(tr("Details")).revealed(false).build();
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
        b.set_title(&trnf!("{} feature needs attention", "{} features need attention", features.len()));
        b.set_revealed(true);

        let commands = report.install_commands();
        b.connect_button_clicked(move |b| {
            let body: String = missing
                .iter()
                .map(|c| match &c.fix {
                    Some(doctor::Fix::Advice(advice)) => format!("• {}: {} ({advice})\n", tr(c.feature), c.what),
                    _ => format!("• {}: {}\n", tr(c.feature), c.what),
                })
                .collect();
            let body = trf!("{}\n\nRun `voltkit doctor` for the full report.", body.trim_end());
            let dialog = adw::AlertDialog::new(Some(tr("Needs Attention")), Some(&body));
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
                dialog.add_response("copy", tr("Copy Command"));
                dialog.set_response_appearance("copy", adw::ResponseAppearance::Suggested);
            }
            dialog.add_response("close", tr("Close"));
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
        .title(tr("The background service is off, so keyboard shortcuts won't work"))
        .button_label(tr("Start"))
        .revealed(!daemon_running())
        .build();
    // Started a moment ago by `run`: look again once it has had time to start.
    {
        let banner = banner.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || banner.set_revealed(!daemon_running()));
    }
    let toasts = toasts.clone();
    banner.connect_button_clicked(move |banner| {
        match start_daemon() {
            Ok(()) => {
                let (banner, toasts) = (banner.clone(), toasts.clone());
                glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                    let running = daemon_running();
                    banner.set_revealed(!running);
                    if !running {
                        toast(&toasts, tr("The background service failed to start"));
                    }
                });
            }
            Err(e) => toast(&toasts, trf!("Could not start the service: {}", e)),
        }
    });
    banner
}

// ---------------------------------------------------------------- OCR

fn ocr_page(config: &Config, toasts: &adw::ToastOverlay) -> gtk::Widget {
    let button = gtk::Button::builder()
        .label(tr("Capture Region"))
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

    let status = crate::workspace::Workspace::new();
    let capture = adw::PreferencesGroup::builder().title(tr("Capture text"))
        .description(trf!("Select text anywhere on screen. Recognition language: {}. Shortcut: {}", config.ocr.lang, shortcut_label(&config.hotkeys.ocr)))
        .header_suffix(&button).build();
    button.set_halign(gtk::Align::End);
    status.banner(&capture);
    scroller.set_min_content_height(300);
    let copy = gtk::Button::builder().label(tr("Copy Text")).valign(gtk::Align::Center).sensitive(false).build();
    let result = adw::PreferencesGroup::builder().title(tr("Extracted text")).description(tr("Capture a region to see and copy its text here."))
        .header_suffix(&copy).build();
    result.add(&scroller);
    status.add(&result);
    let count = gtk::Label::builder().label(tr("No text captured yet")).xalign(0.0).css_classes(["dim-label"]).build();
    result.add(&count);
    let buffer = view.buffer();
    let copy_buffer = buffer.clone();
    copy.connect_clicked(move |button| {
        let (start, end) = copy_buffer.bounds();
        button.clipboard().set_text(&copy_buffer.text(&start, &end, false));
    });
    buffer.connect_changed(move |buffer| {
        let chars = buffer.char_count();
        copy.set_sensitive(chars > 0);
        count.set_label(&trnf!("{} character", "{} characters", chars));
        result.set_description(Some(if chars > 0 { tr("Text is also copied automatically after capture.") } else { tr("Capture a region to see and copy its text here.") }));
    });
    status.aside(&crate::workspace::note(tr("Capture from anything"), tr("Select a paragraph, an error message, or text in an image. The desktop asks you to choose a screen region; recognition happens locally on your computer."), "edit-select-all-symbolic"));

    {
        // Text captured with the hotkey shows up here too.
        let (view, toasts) = (view.clone(), toasts.clone());
        on_captured(status.root(), "ocr", move |text| {
            view.buffer().set_text(&text);
            toast(&toasts, tr("Text from the shortcut, copied to the clipboard"));
        });
    }

    let lang = config.ocr.lang.clone();
    let toasts = toasts.clone();
    button.connect_clicked(move |button| {
        let (button, view, toasts, lang) = (button.clone(), view.clone(), toasts.clone(), lang.clone());
        // The window stays put: GNOME won't bring back a minimized window on its
        // own, so the result would land in a hidden window.
        button.set_sensitive(false);
        glib::spawn_future_local(async move {
            // Attach the screenshot dialogs to this window (the guard keeps it exported).
            let (parent, _guard) = portal::parent_handle(&button.native().expect("button in a window")).await;
            let result =
                gio::spawn_blocking(move || portal::block_on(ocr::extract_text(&lang, parent.as_deref()))).await;
            button.set_sensitive(true);
            match result {
                Ok(Ok(text)) => {
                    view.buffer().set_text(&text);
                    button.clipboard().set_text(&text);
                    toast(&toasts, tr("Text copied to clipboard"));
                }
                Ok(Err(e)) if e.is::<portal::Cancelled>() => toast(&toasts, tr("No screenshot was taken")),
                Ok(Err(e)) => toast(&toasts, e.to_string()),
                Err(_) => toast(&toasts, tr("OCR worker crashed")),
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
        .tooltip_text(tr("Copy"))
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    let (r, toasts) = (row.clone(), toasts.clone());
    button.connect_clicked(move |b| {
        let value = r.subtitle().unwrap_or_default();
        b.clipboard().set_text(&value);
        toast(&toasts, trf!("Copied {}", value));
    });
    row.add_suffix(&button);
    row
}

fn color_page(config: &Config, toasts: &adw::ToastOverlay) -> gtk::Widget {
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

    let entry = adw::EntryRow::builder().title(tr("HEX or rgb()")).text(initial.to_hex()).build();

    // A labeled button in the group header, away from the entry: as a small icon
    // inside the entry row it was easy to miss once the text field had focus.
    let pick = gtk::Button::builder()
        .child(&adw::ButtonContent::builder().icon_name("color-select-symbolic").label(tr("Pick from Screen")).build())
        .tooltip_text(tr("Pick a color anywhere on screen"))
        .valign(gtk::Align::Center)
        .css_classes(["suggested-action"])
        .build();
    let input = adw::PreferencesGroup::builder()
        .title(tr("Color"))
        .description(trf!("Type a color or pick one from your screen. Press Enter to save it to history. From any app, press {} to pick a color: its HEX value is copied and saved here.", shortcut_label(&config.hotkeys.color_picker)))
        .header_suffix(&spaced(&pick))
        .build();
    input.add(&entry);

    let hex = copy_row("HEX", toasts);
    let rgb = copy_row("RGB", toasts);
    let hsl = copy_row("HSL", toasts);
    let values = adw::PreferencesGroup::builder().title(tr("Formats")).build();
    for r in [&hex, &rgb, &hsl] {
        values.add(r);
    }

    let on_white = adw::ActionRow::builder().title(tr("Text on white")).build();
    let on_black = adw::ActionRow::builder().title(tr("Text on black")).build();
    let contrast = adw::PreferencesGroup::builder().title(tr("WCAG Contrast")).build();
    contrast.add(&on_white);
    contrast.add(&on_black);

    let history = adw::PreferencesGroup::builder().title(tr("History")).description(tr("Select a saved color to reuse it. Colors picked from your screen appear here.")).build();
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
    swatch.update_property(&[gtk::accessible::Property::Label(&trf!("Selected color {}", initial.to_hex()))]);

    {
        let (current, swatch) = (current.clone(), swatch.clone());
        entry.connect_changed(move |e| match color::Rgb::parse(&e.text()) {
            Ok(c) => {
                e.remove_css_class("error");
                current.set(c);
                swatch.update_property(&[gtk::accessible::Property::Label(&trf!("Selected color {}", c.to_hex()))]);
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
                toast(&toasts, trf!("Could not save history: {}", e));
            }
        });
    }

    let remember_picked = remember.clone();
    {
        let (entry, toasts) = (entry.clone(), toasts.clone());
        pick.connect_clicked(move |button| {
            let (button, entry, toasts, remember) = (button.clone(), entry.clone(), toasts.clone(), remember.clone());
            button.set_sensitive(false);
            glib::spawn_future_local(async move {
                let (parent, _guard) = portal::parent_handle(&button.native().expect("button in a window")).await;
                let result = gio::spawn_blocking(move || portal::block_on(portal::pick_color(parent.as_deref()))).await;
                button.set_sensitive(true);
                match result {
                    Ok(Ok(c)) => {
                        entry.set_text(&c.to_hex());
                        remember(c);
                        let _ = color::history::push(c);
                        button.clipboard().set_text(&c.to_hex());
                        toast(&toasts, trf!("Copied {}", c.to_hex()));
                    }
                    Ok(Err(e)) if e.is::<portal::Cancelled>() => {}
                    Ok(Err(e)) => toast(&toasts, trf!("Color picker unavailable: {}", e)),
                    Err(_) => toast(&toasts, tr("Color picker crashed")),
                }
            });
        });
    }

    let page = crate::workspace::Workspace::new();
    page.banner(&input);
    let preview = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(16).build();
    preview.append(&gtk::Label::builder().label(tr("Live color preview")).xalign(0.0).css_classes(["heading"]).build());
    swatch.set_content_height(180);
    swatch.set_hexpand(true);
    preview.append(&swatch);
    page.add(&preview);
    page.add(&values);
    page.aside(&contrast);
    page.aside(&history);
    {
        // Colors picked with the hotkey show up here too (already saved to history).
        let (entry, toasts) = (entry.clone(), toasts.clone());
        on_captured(page.root(), "color", move |hex| {
            if let Ok(c) = color::Rgb::parse(&hex) {
                entry.set_text(&c.to_hex());
                remember_picked(c);
                toast(&toasts, trf!("Picked {} with the shortcut, copied", c.to_hex()));
            }
        });
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
        let dot = gtk::DrawingArea::builder().content_width(16).content_height(16).accessible_role(gtk::AccessibleRole::Presentation).build();
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

// ---------------------------------------------------------------- Screen Ruler

fn ruler_page(config: &Config) -> gtk::Widget {
    let open = gtk::Button::builder()
        .label(tr("Open Ruler"))
        .halign(gtk::Align::Center)
        .css_classes(["pill", "suggested-action"])
        .build();
    open.connect_clicked(|button| {
        // Get the activation token before minimizing, while this window has focus.
        spawn_window_tool(button, &["ruler"]);
        // Keep this window out of the screenshot the ruler measures on.
        if let Some(w) = button.root().and_downcast::<gtk::Window>() {
            w.minimize();
        }
    });
    let page = crate::workspace::Workspace::new();
    let preview = crate::workspace::note(tr("Measure in real pixels"), tr("Freeze your screen, then drag a line or a rectangle over anything you want to measure."), "zoom-fit-best-symbolic");
    let diagram = gtk::DrawingArea::builder().content_height(200).hexpand(true).accessible_role(gtk::AccessibleRole::Presentation).build();
    diagram.set_draw_func(|area, cr, w, h| {
        let color = area.color();
        cr.set_source_rgba(color.red() as f64, color.green() as f64, color.blue() as f64, 0.55);
        let left = 28.0;
        let right = (w as f64 - 28.0).max(left);
        let bottom = h as f64 - 30.0;
        cr.set_line_width(1.5);
        cr.rectangle(left, 35.0, right - left, bottom - 35.0);
        let _ = cr.stroke();
        for i in 0..=10 {
            let x = left + (right - left) * i as f64 / 10.0;
            cr.move_to(x, 35.0);
            cr.line_to(x, if i % 5 == 0 { 52.0 } else { 44.0 });
        }
        let _ = cr.stroke();
        cr.set_font_size(16.0);
        cr.move_to(left + 18.0, 110.0);
        let _ = cr.show_text(tr("Drag to measure width × height"));
    });
    preview.append(&diagram);
    preview.append(&open);
    page.add(&preview);
    let controls = adw::PreferencesGroup::builder().title(tr("Measurement controls")).build();
    for (title, description) in [(tr("Drag"), tr("Measure a rectangle")), (tr("Shift + drag"), tr("Measure a straight line")), (tr("Ctrl+C"), tr("Copy the measured size")), (tr("Esc"), tr("Close the ruler"))] {
        controls.add(&adw::ActionRow::builder().title(title).subtitle(description).build());
    }
    page.aside(&controls);
    page.aside(&crate::workspace::note(tr("Ready when you need it"), &trf!("Global shortcut: {}. Measurements use screenshot pixels, including on high-density displays.", shortcut_label(&config.hotkeys.ruler)), "input-keyboard-symbolic"));
    page.upcast()
}

// ---------------------------------------------------------------- Compressor

fn compress_page(config: &Config, toasts: &adw::ToastOverlay) -> gtk::Widget {
    let quality = adw::SpinRow::with_range(1.0, 100.0, 1.0);
    quality.set_title(tr("Quality"));
    quality.set_subtitle(tr("Lower is smaller. PNG is always lossless."));
    quality.set_value(config.compress.quality as f64);

    let choose = gtk::Button::builder()
        .label(tr("Choose Files…"))
        .halign(gtk::Align::Center)
        .css_classes(["pill", "suggested-action"])
        .build();
    // A plain box rather than AdwStatusPage, which scrolls internally and
    // clipped the button when placed in a card.
    let drop_zone = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .css_classes(["card"])
        .build();
    let drop_icon = gtk::Image::builder()
        .icon_name("folder-download-symbolic")
        .pixel_size(64)
        .margin_top(24)
        .css_classes(["dim-label"])
        .build();
    let drop_title = gtk::Label::builder().label(tr("Drop Files Here")).css_classes(["title-2"]).build();
    let drop_text = gtk::Label::builder()
        .label(tr("PNG, JPEG, WebP and MP4. Metadata is stripped; originals are kept."))
        .wrap(true)
        .justify(gtk::Justification::Center)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["dim-label"])
        .build();
    choose.set_margin_top(6);
    choose.set_margin_bottom(24);
    drop_zone.append(&drop_icon);
    drop_zone.append(&drop_title);
    drop_zone.append(&drop_text);
    drop_zone.append(&choose);

    let settings = adw::PreferencesGroup::new();
    settings.add(&quality);
    let results = adw::PreferencesGroup::builder().title(tr("Results")).build();

    let process: Rc<dyn Fn(PathBuf)> = {
        let (quality, results, toasts) = (quality.clone(), results.clone(), toasts.clone());
        Rc::new(move |path: PathBuf| {
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let row = adw::ActionRow::builder().title(&name).subtitle(tr("Compressing…")).build();
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
                        row.set_subtitle(tr("Failed"));
                        toast(&toasts, format!("{name}: {e}"));
                    }
                    Err(_) => row.set_subtitle(tr("Failed")),
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
        gtk::FileDialog::builder().title(tr("Compress Files")).build().open_multiple(
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

    let page = crate::workspace::Workspace::new();
    page.add(&drop_zone);
    settings.set_title(tr("Compression settings"));
    page.aside(&settings);
    page.aside(&crate::workspace::note(tr("Your originals stay intact"), tr("Compressed copies are saved beside each source file as name.min.ext. Metadata is removed from the copy. PNG compression is lossless; quality controls apply to JPEG, WebP, and video."), "document-save-symbolic"));
    results.set_description(Some(tr("Choose or drop files to see progress and size savings here.")));
    page.add(&results);
    page.upcast()
}

// ---------------------------------------------------------------- Snippets
