//! Smart Paste: a small window (`voltkit smart-paste`, Super+Shift+V) that
//! offers the conversions fitting what's on the clipboard and pastes the
//! chosen one into the app that had focus; and its page in the control
//! center, where conversions can be tried out and copied.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::app::{shortcut_label, toast};
use crate::config::Config;
use crate::modules::{ocr, smartpaste};
use crate::paste::{self, PasteShortcut};
use crate::portal::{self, APP_ID, Cancelled};
use crate::i18n::tr;

/// Lets the window hide and the previous app regain focus before pasting.
const REFOCUS_DELAY: Duration = Duration::from_millis(250);
/// How long to wait for the window to get focus before reading the clipboard.
const FOCUS_WAIT: Duration = Duration::from_secs(2);
/// Lines of each result shown under its name.
const PREVIEW_LINES: usize = 3;

pub fn run(config: Config) {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        // Open alongside the control center instead of activating it.
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let config = Rc::new(config);
    app.connect_activate(move |app| {
        Popup::build(app, config.clone());
    });
    app.run_with_args::<&str>(&[]);
}

struct Popup {
    app: adw::Application,
    window: adw::ApplicationWindow,
    list: gtk::ListBox,
    status: adw::StatusPage,
    stack: gtk::Stack,
    preview: gtk::TextView,
    config: Rc<Config>,
    /// Results in list order.
    results: RefCell<Vec<String>>,
    /// The clipboard text before pasting, put back afterwards.
    original: RefCell<Option<String>>,
}

impl Popup {
    fn build(app: &adw::Application, config: Rc<Config>) {
        crate::appearance::install();
        let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::Browse).css_classes(["boxed-list"]).build();
        let preview = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(10)
            .right_margin(10)
            .css_classes(["card"])
            .build();
        let lists = gtk::Box::new(gtk::Orientation::Vertical, 12);
        lists.append(
            &gtk::Label::builder()
                .label(tr("Choose how to paste what you copied, then press Enter. The preview shows exactly what will be pasted."))
                .wrap(true)
                .xalign(0.0)
                .build(),
        );
        lists.append(&gtk::ScrolledWindow::builder().child(&list).vexpand(true).min_content_height(200).build());
        lists.append(&gtk::ScrolledWindow::builder().child(&preview).min_content_height(130).max_content_height(130).build());
        let status = adw::StatusPage::builder().icon_name("edit-paste-symbolic").title(tr("Reading the Clipboard…")).css_classes(["compact"]).build();
        let stack = gtk::Stack::new();
        stack.add_named(&status, Some("status"));
        stack.add_named(&lists, Some("list"));

        let hint = gtk::Label::builder().label(tr("↑↓ choose · Enter paste · Esc cancel")).css_classes(["dim-label", "caption"]).build();
        let body = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();
        body.append(&stack);
        body.append(&hint);
        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&body));
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(tr("Smart Paste"))
            .default_width(540)
            .default_height(560)
            .content(&view)
            .build();

        let popup = Rc::new(Popup {
            app: app.clone(),
            window: window.clone(),
            list: list.clone(),
            status,
            stack,
            preview,
            config,
            results: RefCell::default(),
            original: RefCell::default(),
        });
        {
            let popup = popup.clone();
            list.connect_row_selected(move |_, row| {
                let text = row.and_then(|r| popup.results.borrow().get(r.index() as usize).cloned()).unwrap_or_default();
                popup.preview.buffer().set_text(&text);
            });
        }
        {
            let popup = popup.clone();
            list.connect_row_activated(move |_, row| popup.paste(row.index() as usize));
        }
        let keys = gtk::EventControllerKey::new();
        {
            let popup = popup.clone();
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    popup.app.quit();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
        }
        window.add_controller(keys);
        window.present();

        glib::spawn_future_local(async move { popup.load().await });
    }

    async fn load(self: Rc<Self>) {
        // Wayland only lets the focused window read the clipboard.
        let started = std::time::Instant::now();
        while !self.window.is_active() && started.elapsed() < FOCUS_WAIT {
            glib::timeout_future(Duration::from_millis(50)).await;
        }
        let clipboard = self.window.clipboard();
        if let Ok(Some(text)) = clipboard.read_text_future().await
            && !text.trim().is_empty()
        {
            *self.original.borrow_mut() = Some(text.to_string());
            self.show(smartpaste::available(&text).into_iter().map(|(c, out)| (tr(c.title).to_owned(), out)).collect());
            return;
        }
        if clipboard.formats().contains_type(gdk::Texture::static_type())
            && let Ok(texture) = clipboard.read_texture_future().await
            && let Some(texture) = texture
        {
            self.status.set_title(tr("Reading the Text in the Image…"));
            let lang = self.config.ocr.lang.clone();
            let path = std::env::temp_dir().join(format!("voltkit-smartpaste-{}.png", std::process::id()));
            let saved = texture.save_to_png(&path).is_ok();
            let text = gio::spawn_blocking(move || {
                let text = if saved { ocr::recognize(&path, &lang).ok() } else { None };
                let _ = std::fs::remove_file(&path);
                text
            })
            .await
            .ok()
            .flatten()
            .filter(|t| !t.trim().is_empty());
            match text {
                Some(text) => {
                    let mut items = vec![(tr("Text in the image").to_owned(), text.trim().to_owned())];
                    items.extend(
                        smartpaste::available(text.trim()).into_iter().filter(|(c, _)| c.id != "plain").map(|(c, out)| (tr(c.title).to_owned(), out)),
                    );
                    self.show(items);
                }
                None => self.empty(tr("No Text in the Image"), tr("The image on the clipboard has no text Voltkit can read.")),
            }
            return;
        }
        self.empty(tr("Nothing to Paste"), tr("Copy some text or an image first, then press the shortcut again."));
    }

    fn empty(&self, title: &str, description: &str) {
        self.status.set_icon_name(Some("edit-paste-symbolic"));
        self.status.set_title(title);
        self.status.set_description(Some(description));
    }

    fn show(&self, items: Vec<(String, String)>) {
        for (title, result) in &items {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(title))
                .subtitle(glib::markup_escape_text(&preview(result)))
                .subtitle_lines(PREVIEW_LINES as i32)
                .activatable(true)
                .build();
            self.list.append(&row);
        }
        *self.results.borrow_mut() = items.into_iter().map(|(_, r)| r).collect();
        self.stack.set_visible_child_name("list");
        if let Some(first) = self.list.row_at_index(0) {
            self.list.select_row(Some(&first));
            first.grab_focus();
        }
    }

    fn paste(self: &Rc<Self>, index: usize) {
        let Some(text) = self.results.borrow().get(index).cloned() else { return };
        let this = self.clone();
        glib::spawn_future_local(async move {
            let previous = this.original.borrow().clone();
            this.window.set_visible(false);
            glib::timeout_future(REFOCUS_DELAY).await;
            let shortcut: PasteShortcut = this.config.expander.paste_shortcut.parse().unwrap_or(PasteShortcut::CtrlV);
            let result =
                gio::spawn_blocking(move || portal::block_on(paste::paste(&text, previous, || paste::press_with_portal(shortcut)))).await;
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    // The converted text is still on the clipboard; say so.
                    if !e.is::<Cancelled>() {
                        eprintln!("{}", trf!("paste failed: {}", format!("{:#}", e)));
                    }
                    let _ = gio::spawn_blocking(|| portal::block_on(portal::notify(tr("Converted text copied"), tr("Press Ctrl+V to paste it.")))).await;
                }
                Err(_) => eprintln!("{}", tr("paste worker crashed")),
            }
            this.app.quit();
        });
    }
}

/// The first lines of a result, for a row's subtitle.
fn preview(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().take(PREVIEW_LINES).collect();
    if text.lines().count() > PREVIEW_LINES {
        lines.push("…");
    }
    lines.join("\n")
}

// ---------------------------------------------------------------- Page

/// What the page's "Try It" box starts with.
const SAMPLE: &str = "Distribution\tDesktop\tPackages\nUbuntu\tGNOME\tapt\nFedora\tGNOME\tdnf\nLinux Mint\tCinnamon\tapt";

pub fn page(config: &Config, toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = crate::workspace::Workspace::new();
    let shortcut = shortcut_label(&config.hotkeys.smart_paste);
    let intro = adw::PreferencesGroup::builder()
        .title(tr("Use it in any app"))
        .description(
            tr("Like Ctrl+V, but you choose how it's pasted: tidy JSON, a spreadsheet table as Markdown or CSV, a link \
             without tracking, text from a PDF without broken lines, a new letter case, or the text in a screenshot."),
        )
        .build();
    let steps = [
        (tr("Copy something"), tr("Text, cells from a spreadsheet, JSON, a link, or a screenshot.").to_owned()),
        (tr("Click where it should go"), tr("In any app: an email, a document, a chat, a terminal.").to_owned()),
        (
            tr("Press the Smart Paste shortcut"),
            trf!("{} instead of Ctrl+V. A small window lists the ways it can be pasted, each with a preview.", shortcut),
        ),
        (
            tr("Choose one and press Enter"),
            tr("It's pasted where you clicked, and your clipboard is left as it was. The first time, your desktop asks \
             to let Voltkit type for you.")
                .to_owned(),
        ),
    ];
    for (i, (title, detail)) in steps.iter().enumerate() {
        let row = adw::ActionRow::builder().title(*title).subtitle(glib::markup_escape_text(detail)).subtitle_lines(3).build();
        let number = gtk::Label::builder()
            .label((i + 1).to_string())
            .width_request(28)
            .css_classes(["title-3", "accent", "numeric"])
            .build();
        row.add_prefix(&number);
        intro.add(&row);
    }

    let input = gtk::TextView::builder()
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .top_margin(10)
        .bottom_margin(10)
        .left_margin(12)
        .right_margin(12)
        .height_request(260)
        .build();
    input.set_widget_name("smartpaste-input");
    // Wide, even tab stops so pasted spreadsheet cells line up as columns.
    let mut tabs = gtk::pango::TabArray::new(1, true);
    tabs.set_tab(0, gtk::pango::TabAlign::Left, 110);
    input.set_tabs(&tabs);
    input.buffer().set_text(SAMPLE);
    let frame = gtk::Frame::builder().child(&input).css_classes(["view"]).build();
    let try_group = adw::PreferencesGroup::builder()
        .title(tr("Source text"))
        .description(tr("See what Smart Paste can do without leaving this window: type or paste text here, then copy a converted result."))
        .build();
    try_group.add(&frame);
    let results = adw::PreferencesGroup::builder().title(tr("Conversions")).build();

    let rows: Rc<RefCell<Vec<gtk::Widget>>> = Rc::default();
    let update = {
        let (results, rows, toasts) = (results.clone(), rows.clone(), toasts.clone());
        move |text: &str| {
            for row in rows.take() {
                results.remove(&row);
            }
            let mut new_rows = Vec::new();
            for (conversion, result) in smartpaste::available(text).into_iter().filter(|(c, _)| c.id != "plain") {
                let row = adw::ActionRow::builder()
                    .title(tr(conversion.title))
                    .subtitle(glib::markup_escape_text(&preview(&result)))
                    .subtitle_lines(PREVIEW_LINES as i32)
                    .build();
                let copy = gtk::Button::builder()
                    .icon_name("edit-copy-symbolic")
                    .tooltip_text(tr("Copy"))
                    .valign(gtk::Align::Center)
                    .css_classes(["flat"])
                    .build();
                let toasts = toasts.clone();
                copy.connect_clicked(move |b| {
                    b.clipboard().set_text(&result);
                    toast(&toasts, tr("Copied"));
                });
                row.add_suffix(&copy);
                new_rows.push(row.upcast::<gtk::Widget>());
            }
            if new_rows.is_empty() {
                new_rows.push(adw::ActionRow::builder().title(tr("Only plain text fits this")).build().upcast());
            }
            for row in &new_rows {
                results.add(row);
            }
            *rows.borrow_mut() = new_rows;
        }
    };
    update(SAMPLE);
    input.buffer().connect_changed(move |buffer| {
        let (start, end) = buffer.bounds();
        update(&buffer.text(&start, &end, false));
    });

    page.add(&try_group);
    page.aside(&results);
    page.aside(&intro);
    page.upcast()
}
