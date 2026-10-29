//! The Text Expansion page: ways to insert snippets, and a manager to add,
//! edit and delete them without touching the config file.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::{gio, glib};

use crate::app::{shortcut_label, toast};
use crate::config::{Config, Snippet};
use crate::daemon;
use crate::modules::snippets;

pub fn page(config: &Config, toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = adw::PreferencesPage::new();
    page.add(&usage_group(config));
    let manager = Manager::new(config.snippets.clone(), toasts);
    page.add(&manager.group);
    // The list's buttons only hold weak references, so the page keeps the manager alive.
    page.connect_destroy(move |_| {
        let _ = &manager;
    });
    page.upcast()
}

fn usage_group(config: &Config) -> adw::PreferencesGroup {
    let usage = adw::PreferencesGroup::builder().title("Insert Snippets").build();
    let open_picker = gtk::Button::builder().label("Open").valign(gtk::Align::Center).build();
    open_picker.connect_clicked(|_| {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).arg("picker").spawn().map(|mut c| {
                std::thread::spawn(move || c.wait());
            });
        }
    });
    let picker_row = adw::ActionRow::builder()
        .title("Snippet picker")
        .subtitle(format!("Search and paste into any app. Shortcut: {}", shortcut_label(&config.hotkeys.snippets)))
        .build();
    picker_row.add_suffix(&open_picker);
    usage.add(&picker_row);
    let typed = if config.expander.typed_triggers {
        "On: type a snippet's trigger in any app to replace it with the text"
    } else {
        "Off: run `voltkit expander setup`, then `voltkit expander enable`"
    };
    usage.add(&adw::ActionRow::builder().title("Typed triggers").subtitle(typed).build());
    usage
}

/// The "Your Snippets" list. Every change is saved to the config file at once.
struct Manager {
    group: adw::PreferencesGroup,
    rows: RefCell<Vec<gtk::Widget>>,
    snippets: RefCell<Vec<Snippet>>,
    toasts: adw::ToastOverlay,
}

impl Manager {
    fn new(snippets: Vec<Snippet>, toasts: &adw::ToastOverlay) -> Rc<Self> {
        let add = gtk::Button::builder()
            .child(&adw::ButtonContent::builder().icon_name("list-add-symbolic").label("Add Snippet").build())
            .valign(gtk::Align::Center)
            .css_classes(["suggested-action"])
            .build();
        let group = adw::PreferencesGroup::builder()
            .title("Your Snippets")
            .description("Click a snippet to edit it.")
            .header_suffix(&add)
            .build();
        let manager = Rc::new(Self {
            group,
            rows: RefCell::default(),
            snippets: RefCell::new(snippets),
            toasts: toasts.clone(),
        });
        manager.rebuild();
        let weak = Rc::downgrade(&manager);
        add.connect_clicked(move |button| {
            if let Some(m) = weak.upgrade() {
                m.edit(button, None);
            }
        });
        manager
    }

    fn rebuild(self: &Rc<Self>) {
        for row in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        let snippets = self.snippets.borrow();
        let mut rows = self.rows.borrow_mut();
        if snippets.is_empty() {
            let row = adw::ActionRow::builder()
                .title("No snippets yet")
                .subtitle("Click Add Snippet to create your first one.")
                .build();
            self.group.add(&row);
            rows.push(row.upcast());
            return;
        }
        for (index, snippet) in snippets.iter().enumerate() {
            let preview = snippets::preview(&snippet.template).unwrap_or_else(|e| format!("⚠ {e}"));
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&snippet.trigger))
                .subtitle(glib::markup_escape_text(&preview))
                .subtitle_lines(2)
                .activatable(true)
                .build();
            let edit = icon_button("document-edit-symbolic", "Edit");
            let delete = icon_button("user-trash-symbolic", "Delete");
            row.add_suffix(&edit);
            row.add_suffix(&delete);

            let weak = Rc::downgrade(self);
            row.connect_activated(move |row| with(&weak, |m| m.edit(row, Some(index))));
            let weak = Rc::downgrade(self);
            edit.connect_clicked(move |button| with(&weak, |m| m.edit(button, Some(index))));
            let weak = Rc::downgrade(self);
            delete.connect_clicked(move |_| with(&weak, |m| m.delete(index)));

            self.group.add(&row);
            rows.push(row.upcast());
        }
    }

    /// Writes the list to the config file and tells the daemon, which runs
    /// typed triggers, to pick it up.
    fn save(&self) -> bool {
        let snippets = self.snippets.borrow().clone();
        // Re-read the file so settings changed elsewhere (e.g. `voltkit expander enable`) survive.
        let result = Config::load().and_then(|mut config| {
            config.snippets = snippets;
            config.save()
        });
        if let Err(e) = result {
            toast(&self.toasts, format!("Could not save snippets: {e}"));
            return false;
        }
        gio::spawn_blocking(|| {
            let _ = daemon::reload(); // nothing to do if the daemon isn't running
        });
        true
    }

    fn delete(self: &Rc<Self>, index: usize) {
        let removed = self.snippets.borrow_mut().remove(index);
        if !self.save() {
            self.snippets.borrow_mut().insert(index, removed);
            return;
        }
        self.rebuild();

        let undo = adw::Toast::builder()
            .title(format!("Deleted {}", removed.trigger))
            .button_label("Undo")
            .build();
        let weak = Rc::downgrade(self);
        undo.connect_button_clicked(move |_| {
            with(&weak, |m| {
                let at = index.min(m.snippets.borrow().len());
                m.snippets.borrow_mut().insert(at, removed.clone());
                m.save();
                m.rebuild();
            })
        });
        self.toasts.add_toast(undo);
    }

    fn edit(self: &Rc<Self>, parent: &impl IsA<gtk::Widget>, index: Option<usize>) {
        let snippets = self.snippets.borrow();
        let existing = index.and_then(|i| snippets.get(i).cloned());
        let others: Vec<String> = snippets
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != index)
            .map(|(_, s)| s.trigger.clone())
            .collect();
        drop(snippets);

        let weak = Rc::downgrade(self);
        open_editor(parent.as_ref(), existing, others, move |snippet| {
            with(&weak, |m| {
                let trigger = snippet.trigger.clone();
                match index {
                    Some(i) => m.snippets.borrow_mut()[i] = snippet,
                    None => m.snippets.borrow_mut().push(snippet),
                }
                if m.save() {
                    m.rebuild();
                    toast(&m.toasts, format!("Saved {trigger}"));
                }
            })
        });
    }
}

fn with(weak: &Weak<Manager>, f: impl FnOnce(Rc<Manager>)) {
    if let Some(manager) = weak.upgrade() {
        f(manager);
    }
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
}

/// What's wrong with a trigger, if anything, given the other snippets' triggers.
fn trigger_problem(trigger: &str, others: &[String]) -> Option<String> {
    if trigger.is_empty() {
        Some("Enter a trigger, such as ;sig.".into())
    } else if trigger.chars().any(char::is_whitespace) {
        Some("Triggers can't contain spaces.".into())
    } else if others.iter().any(|o| o == trigger) {
        Some(format!("Another snippet already uses {trigger}."))
    } else {
        None
    }
}

/// A typed-trigger clash that doesn't block saving: the shorter trigger always
/// fires first, so the longer one can never be typed.
fn trigger_warning(trigger: &str, others: &[String]) -> Option<String> {
    let other = others.iter().find(|o| !o.is_empty() && (o.starts_with(trigger) || trigger.starts_with(o.as_str())))?;
    let (short, long) = if other.len() < trigger.len() { (other.as_str(), trigger) } else { (trigger, other.as_str()) };
    Some(format!("When typing, {short} expands before you can finish {long}. They still work from the picker."))
}

/// Date and time formats offered by the editor's Insert buttons.
const DATE_FORMATS: &[&str] =
    &["{date}", "{date:%B %d, %Y}", "{date:%A, %B %d}", "{date:%d/%m/%Y}", "{date:%m/%d/%Y}", "{date:%A}"];
const TIME_FORMATS: &[&str] = &["{time}", "{date:%I:%M %p}", "{date:%H:%M:%S}"];

fn open_editor(parent: &gtk::Widget, existing: Option<Snippet>, others: Vec<String>, on_save: impl Fn(Snippet) + 'static) {
    let is_new = existing.is_none();
    let existing = existing.unwrap_or_else(|| Snippet { trigger: ";".into(), template: String::new() });

    let dialog = adw::Dialog::builder()
        .title(if is_new { "New Snippet" } else { "Edit Snippet" })
        .content_width(580)
        .content_height(640)
        .build();
    let cancel = gtk::Button::with_label("Cancel");
    let save = gtk::Button::builder()
        .label(if is_new { "Add" } else { "Save" })
        .css_classes(["suggested-action"])
        .sensitive(false)
        .build();
    let header = adw::HeaderBar::builder().show_start_title_buttons(false).show_end_title_buttons(false).build();
    header.pack_start(&cancel);
    header.pack_end(&save);

    // Trigger
    let trigger = adw::EntryRow::builder().title("Trigger").text(&existing.trigger).build();
    let trigger_note = gtk::Label::builder().xalign(0.0).wrap(true).visible(false).css_classes(["caption"]).build();
    let trigger_group = adw::PreferencesGroup::builder()
        .title("Trigger")
        .description("A short code that names the snippet. Find it by this name in the picker, or type it in any app when typed triggers are on. Starting with ; avoids clashes with normal words.")
        .build();
    trigger_group.add(&trigger);
    trigger_group.add(&trigger_note);

    // Text, with buttons that insert the variables so nobody has to learn the syntax
    let buffer = gtk::TextBuffer::new(None);
    buffer.set_text(&existing.template);
    let view = gtk::TextView::builder()
        .buffer(&buffer)
        .wrap_mode(gtk::WrapMode::WordChar)
        .accepts_tab(false)
        .top_margin(12)
        .bottom_margin(12)
        .left_margin(12)
        .right_margin(12)
        .build();
    let scroller = gtk::ScrolledWindow::builder()
        .child(&view)
        .min_content_height(130)
        .css_classes(["card"])
        .build();

    let inserts = gtk::Box::builder().spacing(6).margin_bottom(8).build();
    inserts.append(&gtk::Label::builder().label("Insert:").css_classes(["dim-label"]).build());
    inserts.append(&format_menu("Date", "x-office-calendar-symbolic", DATE_FORMATS, &buffer, &view));
    inserts.append(&format_menu("Time", "alarm-symbolic", TIME_FORMATS, &buffer, &view));
    let clipboard = insert_button("Clipboard", "edit-paste-symbolic", "Whatever you copied last");
    let fill_in = insert_button("Fill-in Field", "document-edit-symbolic", "A blank you fill in each time you use the snippet");
    inserts.append(&clipboard);
    inserts.append(&fill_in);
    {
        let (buffer, view) = (buffer.clone(), view.clone());
        clipboard.connect_clicked(move |_| {
            buffer.insert_at_cursor("{clipboard}");
            view.grab_focus();
        });
    }
    {
        let (buffer, view) = (buffer.clone(), view.clone());
        fill_in.connect_clicked(move |_| {
            buffer.insert_at_cursor("{arg:name}");
            // Select "name" so typing replaces it with the field's real name.
            let end = buffer.iter_at_mark(&buffer.get_insert());
            let (mut start, mut stop) = (end, end);
            start.backward_chars(5);
            stop.backward_char();
            buffer.select_range(&start, &stop);
            view.grab_focus();
        });
    }

    let text_group = adw::PreferencesGroup::builder()
        .title("Text")
        .description("What gets pasted. Use Insert to add values that are filled in when you use the snippet.")
        .build();
    text_group.add(&inserts);
    text_group.add(&scroller);

    // Preview
    let preview = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    let preview_card = gtk::Box::builder().css_classes(["card"]).build();
    preview_card.append(&preview);
    let preview_group = adw::PreferencesGroup::builder()
        .title("Preview")
        .description("Exactly what will be pasted right now. ‹Fill-in› fields are asked for when you use the snippet, and ‹clipboard› becomes whatever you copied last.")
        .build();
    preview_group.add(&preview_card);

    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(18)
        .margin_end(18)
        .build();
    body.append(&trigger_group);
    body.append(&text_group);
    body.append(&preview_group);
    let content = adw::ToolbarView::new();
    content.add_top_bar(&header);
    content.set_content(Some(&gtk::ScrolledWindow::builder().child(&body).vexpand(true).build()));
    dialog.set_child(Some(&content));

    // Validation and preview, on every change
    let validate = {
        let (trigger, buffer, trigger_note, preview, save) =
            (trigger.clone(), buffer.clone(), trigger_note.clone(), preview.clone(), save.clone());
        let others = others.clone();
        move || {
            let code = trigger.text().trim().to_owned();
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            let problem = trigger_problem(&code, &others);
            let warning = if problem.is_none() { trigger_warning(&code, &others) } else { None };
            match problem.as_ref().or(warning.as_ref()) {
                Some(note) => {
                    trigger_note.set_label(note);
                    trigger_note.set_css_classes(if problem.is_some() { &["caption", "error"] } else { &["caption", "warning"] });
                    trigger_note.set_visible(true);
                }
                None => trigger_note.set_visible(false),
            }
            if problem.is_some() {
                trigger.add_css_class("error");
            } else {
                trigger.remove_css_class("error");
            }

            let rendered = if text.trim().is_empty() { Err("Type the text to paste above.".to_owned()) } else {
                snippets::preview(&text).map_err(|e| format!("This text can't be expanded: {e}."))
            };
            match &rendered {
                Ok(out) => {
                    preview.set_label(out);
                    preview.set_css_classes(&[]);
                }
                Err(msg) => {
                    preview.set_label(msg);
                    preview.set_css_classes(&["dim-label"]);
                }
            }
            save.set_sensitive(problem.is_none() && rendered.is_ok());
        }
    };
    validate();
    let validate = Rc::new(validate);
    {
        let validate = validate.clone();
        trigger.connect_changed(move |_| validate());
    }
    {
        let validate = validate.clone();
        buffer.connect_changed(move |_| validate());
    }
    {
        let view = view.clone();
        trigger.connect_entry_activated(move |_| {
            view.grab_focus();
        });
    }

    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog.close();
        });
    }
    {
        let (dialog, trigger, buffer) = (dialog.clone(), trigger.clone(), buffer.clone());
        save.connect_clicked(move |_| {
            on_save(Snippet {
                trigger: trigger.text().trim().to_owned(),
                template: buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string(),
            });
            dialog.close();
        });
    }

    dialog.present(Some(parent));
    if is_new {
        trigger.grab_focus();
        trigger.set_position(-1);
    } else {
        view.grab_focus();
    }
}

fn insert_button(label: &str, icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .child(&adw::ButtonContent::builder().icon_name(icon).label(label).build())
        .tooltip_text(tooltip)
        .build()
}

/// A menu button listing formats by their example output ("October 29, 2026");
/// choosing one inserts its variable at the cursor.
fn format_menu(label: &str, icon: &str, formats: &[&str], buffer: &gtk::TextBuffer, view: &gtk::TextView) -> gtk::MenuButton {
    let list = gtk::Box::builder().orientation(gtk::Orientation::Vertical).build();
    let popover = gtk::Popover::builder().child(&list).build();
    let now = snippets::Context::default();
    for &format in formats {
        let example = snippets::expand(format, &now).unwrap_or_else(|_| format.to_owned());
        let item = gtk::Button::builder()
            .label(&example)
            .tooltip_text(format)
            .css_classes(["flat"])
            .build();
        if let Some(text) = item.child().and_downcast::<gtk::Label>() {
            text.set_xalign(0.0);
        }
        let (buffer, view, popover, format) = (buffer.clone(), view.clone(), popover.clone(), format.to_owned());
        item.connect_clicked(move |_| {
            buffer.insert_at_cursor(&format);
            popover.popdown();
            view.grab_focus();
        });
        list.append(&item);
    }
    gtk::MenuButton::builder()
        .child(&adw::ButtonContent::builder().icon_name(icon).label(label).build())
        .always_show_arrow(true)
        .popover(&popover)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triggers(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn trigger_problems() {
        let others = triggers(&[";sig", ";date"]);
        assert!(trigger_problem("", &others).is_some());
        assert!(trigger_problem(";my sig", &others).is_some());
        assert!(trigger_problem(";sig", &others).unwrap().contains("already"));
        assert_eq!(trigger_problem(";addr", &others), None);
    }

    #[test]
    fn prefix_clash_is_only_a_warning() {
        let others = triggers(&[";date"]);
        let warning = trigger_warning(";d", &others).unwrap();
        assert!(warning.contains(";d expands before you can finish ;date"));
        let warning = trigger_warning(";datetime", &others).unwrap();
        assert!(warning.contains(";date expands before you can finish ;datetime"));
        assert_eq!(trigger_warning(";sig", &others), None);
    }

    #[test]
    fn offered_formats_all_expand() {
        let now = snippets::Context::default();
        for format in DATE_FORMATS.iter().chain(TIME_FORMATS) {
            assert!(snippets::expand(format, &now).is_ok(), "{format}");
        }
    }
}
