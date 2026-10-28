//! `powerlinux picker`: the snippet picker popup (Super+Shift+E by default).
//!
//! Search snippets, press Enter, fill in any {arg:NAME} parameters, and the
//! expansion is pasted into the app that had focus before the picker opened.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::config::{Config, Snippet};
use crate::modules::snippets;
use crate::paste::{self, PasteShortcut};
use crate::portal::{self, APP_ID, Cancelled};

/// Lets the picker window hide and the previous app regain focus before pasting.
const REFOCUS_DELAY: Duration = Duration::from_millis(250);

pub fn run(config: Config, preselect: Option<String>) {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        // Open alongside the control center instead of activating it.
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let config = Rc::new(config);
    app.connect_activate(move |app| build(app, config.clone(), preselect.clone()));
    app.run_with_args::<&str>(&[]);
}

struct Picker {
    app: adw::Application,
    window: adw::ApplicationWindow,
    stack: gtk::Stack,
    list: gtk::ListBox,
    form: adw::PreferencesGroup,
    error: gtk::Label,
    config: Rc<Config>,
    /// Snippets currently shown, in list order.
    shown: RefCell<Vec<Snippet>>,
    /// Entry rows of the open parameter form, by parameter name.
    fields: RefCell<Vec<(String, adw::EntryRow)>>,
    chosen: RefCell<Option<Snippet>>,
}

fn build(app: &adw::Application, config: Rc<Config>, preselect: Option<String>) {
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search snippets")
        .hexpand(true)
        .build();
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Browse)
        .css_classes(["boxed-list"])
        .build();
    let list_page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    list_page.append(&search);
    list_page.append(&gtk::ScrolledWindow::builder().child(&list).vexpand(true).build());

    let form = adw::PreferencesGroup::new();
    let insert = gtk::Button::builder()
        .label("Insert")
        .halign(gtk::Align::End)
        .css_classes(["suggested-action", "pill"])
        .build();
    let form_page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    form_page.append(&form);
    form_page.append(&insert);

    let stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::SlideLeft).build();
    stack.add_named(&list_page, Some("list"));
    stack.add_named(&form_page, Some("form"));

    let error = gtk::Label::builder().css_classes(["error"]).wrap(true).visible(false).build();
    let hint = gtk::Label::builder()
        .label("↑↓ choose · Enter insert · Esc cancel")
        .css_classes(["dim-label", "caption"])
        .build();

    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    body.append(&stack);
    body.append(&error);
    body.append(&hint);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::builder().show_end_title_buttons(true).build());
    view.set_content(Some(&body));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Insert Snippet")
        .default_width(520)
        .default_height(460)
        .content(&view)
        .build();

    let picker = Rc::new(Picker {
        app: app.clone(),
        window: window.clone(),
        stack,
        list: list.clone(),
        form,
        error,
        config,
        shown: RefCell::default(),
        fields: RefCell::default(),
        chosen: RefCell::default(),
    });

    picker.fill_list("");
    {
        let picker = picker.clone();
        search.connect_search_changed(move |s| picker.fill_list(&s.text()));
    }
    {
        let picker = picker.clone();
        search.connect_activate(move |_| picker.choose_selected());
    }
    {
        let picker = picker.clone();
        list.connect_row_activated(move |_, row| {
            picker.list.select_row(Some(row));
            picker.choose_selected();
        });
    }
    {
        let picker = picker.clone();
        insert.connect_clicked(move |_| picker.insert_from_form());
    }

    // Arrow keys move through the list while typing in the search box; Esc backs out.
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let picker = picker.clone();
        keys.connect_key_pressed(move |_, key, _, _| picker.on_key(key));
    }
    window.add_controller(keys);

    window.present();
    match preselect.and_then(|t| picker.config.snippets.iter().find(|s| s.trigger == t).cloned()) {
        Some(snippet) => picker.choose(snippet),
        None => {
            search.grab_focus();
        }
    }
}

impl Picker {
    fn fill_list(&self, query: &str) {
        self.list.remove_all();
        let found: Vec<Snippet> = snippets::search(&self.config.snippets, query).into_iter().cloned().collect();
        for s in &found {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&s.trigger))
                .subtitle(glib::markup_escape_text(&preview(&s.template)))
                .subtitle_lines(2)
                .activatable(true)
                .build();
            self.list.append(&row);
        }
        if found.is_empty() {
            let row = adw::ActionRow::builder().title("No matching snippets").sensitive(false).build();
            self.list.append(&row);
        }
        self.list.select_row(self.list.row_at_index(0).as_ref());
        *self.shown.borrow_mut() = found;
    }

    fn on_key(self: &Rc<Self>, key: gdk::Key) -> glib::Propagation {
        let on_list = self.stack.visible_child_name().as_deref() == Some("list");
        match key {
            gdk::Key::Escape if on_list => self.window.close(),
            gdk::Key::Escape => {
                self.error.set_visible(false);
                self.stack.set_visible_child_name("list");
            }
            gdk::Key::Down | gdk::Key::Up if on_list => {
                let current = self.list.selected_row().map(|r| r.index()).unwrap_or(0);
                let next = if key == gdk::Key::Down { current + 1 } else { (current - 1).max(0) };
                // Focus stays in the search box so the user can keep typing.
                if let Some(row) = self.list.row_at_index(next) {
                    self.list.select_row(Some(&row));
                }
            }
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    }

    fn choose_selected(self: &Rc<Self>) {
        let Some(index) = self.list.selected_row().map(|r| r.index() as usize) else { return };
        let snippet = self.shown.borrow().get(index).cloned();
        if let Some(snippet) = snippet {
            self.choose(snippet);
        }
    }

    fn choose(self: &Rc<Self>, snippet: Snippet) {
        let args = snippets::required_args(&snippet.template);
        *self.chosen.borrow_mut() = Some(snippet.clone());
        if args.is_empty() {
            self.insert(Default::default());
            return;
        }

        // Rebuild the parameter form.
        for (_, row) in self.fields.borrow_mut().drain(..) {
            self.form.remove(&row);
        }
        self.form.set_title(&glib::markup_escape_text(&snippet.trigger));
        self.form.set_description(Some(&glib::markup_escape_text(&preview(&snippet.template))));
        let mut fields = self.fields.borrow_mut();
        for (i, name) in args.iter().enumerate() {
            let row = adw::EntryRow::builder().title(name).build();
            let last = i + 1 == args.len();
            let this = self.clone();
            row.connect_entry_activated(move |row| {
                if last {
                    this.insert_from_form();
                } else if let Some(next) = row.next_sibling() {
                    next.grab_focus();
                }
            });
            self.form.add(&row);
            fields.push((name.clone(), row));
        }
        self.stack.set_visible_child_name("form");
        if let Some((_, first)) = fields.first() {
            first.grab_focus();
        }
    }

    fn insert_from_form(self: &Rc<Self>) {
        let args = self.fields.borrow().iter().map(|(name, row)| (name.clone(), row.text().to_string())).collect();
        self.insert(args);
    }

    fn insert(self: &Rc<Self>, args: std::collections::HashMap<String, String>) {
        let Some(snippet) = self.chosen.borrow().clone() else { return };
        let this = self.clone();
        glib::spawn_future_local(async move {
            // Our window has focus, so GTK can read the clipboard here.
            let previous = this.window.clipboard().read_text_future().await.ok().flatten().map(|s| s.to_string());
            let ctx = snippets::Context { clipboard: previous.clone(), args, ..Default::default() };
            let text = match snippets::expand(&snippet.template, &ctx) {
                Ok(text) => text,
                Err(e) => {
                    this.error.set_label(&e.to_string());
                    this.error.set_visible(true);
                    return;
                }
            };

            this.window.set_visible(false);
            glib::timeout_future(REFOCUS_DELAY).await;
            let shortcut: PasteShortcut = this.config.expander.paste_shortcut.parse().unwrap_or(PasteShortcut::CtrlV);
            let result = gio::spawn_blocking(move || {
                portal::block_on(paste::paste(&text, previous, || paste::press_with_portal(shortcut)))
            })
            .await;
            match result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    // The expansion is still on the clipboard; tell the user to paste it.
                    if !e.is::<Cancelled>() {
                        eprintln!("paste failed: {e:#}");
                    }
                    let _ = gio::spawn_blocking(|| {
                        portal::block_on(portal::notify("Snippet copied", "Press Ctrl+V to paste it."))
                    })
                    .await;
                }
                Err(_) => eprintln!("paste worker crashed"),
            }
            this.app.quit();
        });
    }
}

/// Template preview with parameters and the clipboard shown as placeholders.
fn preview(template: &str) -> String {
    let ctx = snippets::Context {
        clipboard: Some("‹clipboard›".into()),
        args: snippets::required_args(template).into_iter().map(|a| (a.clone(), format!("‹{a}›"))).collect(),
        ..Default::default()
    };
    snippets::expand(template, &ctx).unwrap_or_else(|e| format!("⚠ {e}"))
}
