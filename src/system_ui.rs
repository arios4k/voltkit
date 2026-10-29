//! The Hosts & Environment page: friendly editors for /etc/hosts and for the
//! desktop session's environment variables.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use adw::prelude::*;
use gtk::{gio, glib};

use crate::app::toast;
use crate::modules::envvars::{self, Var};
use crate::modules::hosts::{self, Entry, HostsFile};

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = adw::PreferencesPage::new();
    let hosts = HostsEditor::new(toasts);
    let env = EnvEditor::new(toasts);
    page.add(&hosts.group);
    page.add(&env.group);
    page.add(&env.session_group);
    // Rows only hold weak references; the page keeps the editors alive.
    page.connect_destroy(move |_| {
        let _ = (&hosts, &env);
    });
    page.upcast()
}

// ---------------------------------------------------------------- Hosts file

struct HostsEditor {
    group: adw::PreferencesGroup,
    rows: RefCell<Vec<gtk::Widget>>,
    file: RefCell<HostsFile>,
    /// Unsaved changes: saving needs the administrator password, so edits are
    /// collected and saved together.
    dirty: Cell<bool>,
    /// System entries are locked unless the user turns this on.
    unlocked: Cell<bool>,
    save: gtk::Button,
    discard: gtk::Button,
    toasts: adw::ToastOverlay,
}

impl HostsEditor {
    fn new(toasts: &adw::ToastOverlay) -> Rc<Self> {
        let add = header_button("list-add-symbolic", "Add Entry");
        let group = adw::PreferencesGroup::builder()
            .title("Hosts File")
            .description(
                "Point names at addresses on this computer, such as myapp.test → 127.0.0.1. Turn entries off \
                 instead of deleting them to keep them for later. Saving asks for your password and keeps a \
                 backup in /etc/hosts.voltkit-backup.",
            )
            .header_suffix(&add)
            .build();
        let save = gtk::Button::builder().label("Save Changes").css_classes(["suggested-action"]).sensitive(false).build();
        let discard = gtk::Button::builder().label("Discard").sensitive(false).build();
        let editor = Rc::new(Self {
            group,
            rows: RefCell::default(),
            file: RefCell::new(HostsFile::default()),
            dirty: Cell::new(false),
            unlocked: Cell::new(false),
            save,
            discard,
            toasts: toasts.clone(),
        });
        let unlock = adw::SwitchRow::builder()
            .title("Allow editing system entries")
            .subtitle("Changing localhost or your computer's name can break networking and sudo. A backup is kept either way.")
            .build();
        editor.group.add(&unlock);
        let weak = Rc::downgrade(&editor);
        unlock.connect_active_notify(move |s| {
            with(&weak, |e| {
                e.unlocked.set(s.is_active());
                e.rebuild();
            })
        });
        editor.reload();

        let bar = gtk::Box::builder().spacing(12).halign(gtk::Align::End).margin_top(12).build();
        bar.append(&editor.discard);
        bar.append(&editor.save);
        editor.group.add(&bar);

        let weak = Rc::downgrade(&editor);
        add.connect_clicked(move |b| with(&weak, |e| e.edit(b, None)));
        let weak = Rc::downgrade(&editor);
        editor.save.connect_clicked(move |_| with(&weak, |e| e.save()));
        let weak = Rc::downgrade(&editor);
        editor.discard.connect_clicked(move |_| with(&weak, |e| e.reload()));
        editor
    }

    fn reload(self: &Rc<Self>) {
        match HostsFile::load() {
            Ok(file) => *self.file.borrow_mut() = file,
            Err(e) => toast(&self.toasts, format!("{e:#}")),
        }
        self.set_dirty(false);
        self.rebuild();
    }

    fn set_dirty(&self, dirty: bool) {
        self.dirty.set(dirty);
        self.save.set_sensitive(dirty);
        self.discard.set_sensitive(dirty);
    }

    fn changed(self: &Rc<Self>) {
        self.set_dirty(true);
        self.rebuild();
    }

    fn rebuild(self: &Rc<Self>) {
        for row in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        let file = self.file.borrow();
        let mut rows = self.rows.borrow_mut();
        for (index, entry) in file.entries() {
            let mut subtitle = entry.ip.clone();
            if !entry.comment.is_empty() {
                subtitle.push_str(&format!("  ·  {}", entry.comment));
            }
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&entry.names.join(", ")))
                .subtitle(glib::markup_escape_text(&subtitle))
                .build();
            let enabled = gtk::Switch::builder()
                .active(entry.enabled)
                .valign(gtk::Align::Center)
                .tooltip_text(if entry.enabled { "On" } else { "Off" })
                .build();
            if entry.is_system() && !self.unlocked.get() {
                // Shown for completeness, but changing these breaks networking or sudo.
                row.add_suffix(
                    &gtk::Image::builder()
                        .icon_name("changes-prevent-symbolic")
                        .tooltip_text("System entry, kept as it is")
                        .build(),
                );
                enabled.set_sensitive(false);
                row.add_suffix(&enabled);
            } else {
                let edit = flat_button("document-edit-symbolic", "Edit");
                let delete = flat_button("user-trash-symbolic", "Delete");
                row.add_suffix(&enabled);
                row.add_suffix(&edit);
                row.add_suffix(&delete);
                let weak = Rc::downgrade(self);
                enabled.connect_active_notify(move |s| {
                    with(&weak, |e| {
                        e.file.borrow_mut().set_enabled(index, s.is_active());
                        e.set_dirty(true);
                    })
                });
                let weak = Rc::downgrade(self);
                edit.connect_clicked(move |b| with(&weak, |e| e.edit(b, Some(index))));
                let weak = Rc::downgrade(self);
                delete.connect_clicked(move |_| {
                    with(&weak, |e| {
                        e.file.borrow_mut().remove(index);
                        e.changed();
                    })
                });
            }
            self.group.add(&row);
            rows.push(row.upcast());
        }
        drop(rows);
        drop(file);
        // Move the save/discard bar below the rows.
        if let Some(bar) = self.save.parent() {
            self.group.remove(&bar);
            self.group.add(&bar);
        }
    }

    fn edit(self: &Rc<Self>, parent: &impl IsA<gtk::Widget>, index: Option<usize>) {
        let existing = index.and_then(|i| self.file.borrow().get(i).cloned());
        let fields = [
            ("IP address", existing.as_ref().map_or("127.0.0.1".into(), |e| e.ip.clone())),
            ("Host names, separated by spaces", existing.as_ref().map_or(String::new(), |e| e.names.join(" "))),
            ("Note (optional)", existing.as_ref().map_or(String::new(), |e| e.comment.clone())),
        ];
        let weak = Rc::downgrade(self);
        form_dialog(
            parent.as_ref(),
            if existing.is_some() { "Edit Host Entry" } else { "New Host Entry" },
            &fields,
            |values| {
                if !hosts::valid_ip(values[0].trim()) {
                    return Err("Enter an IP address, such as 127.0.0.1 or ::1.".into());
                }
                let names: Vec<&str> = values[1].split_whitespace().collect();
                if names.is_empty() {
                    return Err("Enter at least one host name, such as myapp.test.".into());
                }
                if let Some(bad) = names.iter().find(|n| !hosts::valid_name(n)) {
                    return Err(format!("“{bad}” isn't a valid host name (letters, digits, dots and dashes)."));
                }
                Ok(())
            },
            move |values| {
                with(&weak, |e| {
                    let mut entry = Entry::new(
                        values[0].trim(),
                        values[1].split_whitespace().map(str::to_owned).collect(),
                        &values[2],
                    );
                    match index {
                        Some(i) => {
                            entry.enabled = e.file.borrow().get(i).is_none_or(|old| old.enabled);
                            e.file.borrow_mut().set(i, entry);
                        }
                        None => e.file.borrow_mut().push(entry),
                    }
                    e.changed();
                })
            },
        );
    }

    fn save(self: &Rc<Self>) {
        let file = self.file.borrow().clone();
        let this = self.clone();
        self.save.set_sensitive(false);
        glib::spawn_future_local(async move {
            let result = gio::spawn_blocking(move || hosts::save(&file)).await;
            match result {
                Ok(Ok(())) => {
                    toast(&this.toasts, format!("Saved {}. The previous version is in {}.", hosts::PATH, hosts::BACKUP));
                    this.reload();
                }
                Ok(Err(e)) => {
                    toast(&this.toasts, format!("{e:#}"));
                    this.set_dirty(true);
                }
                Err(_) => this.set_dirty(true),
            }
        });
    }
}

// ---------------------------------------------------------------- Environment variables

struct EnvEditor {
    group: adw::PreferencesGroup,
    /// Variables the session already has from elsewhere, offered for overriding.
    session_group: adw::PreferencesGroup,
    session_rows: RefCell<Vec<gtk::Widget>>,
    search: gtk::SearchEntry,
    rows: RefCell<Vec<gtk::Widget>>,
    vars: RefCell<Vec<Var>>,
    toasts: adw::ToastOverlay,
}

impl EnvEditor {
    fn new(toasts: &adw::ToastOverlay) -> Rc<Self> {
        let add = header_button("list-add-symbolic", "Add Variable");
        let group = adw::PreferencesGroup::builder()
            .title("Environment Variables")
            .description(
                "Set for every app in your desktop session. They apply the next time you log in (restart if \
                 logging out isn't enough). To add to a list like PATH, use ${PATH}:/new/folder.",
            )
            .header_suffix(&add)
            .build();
        let vars = envvars::load().unwrap_or_else(|e| {
            toast(toasts, format!("{e:#}"));
            Vec::new()
        });
        let search = gtk::SearchEntry::builder().placeholder_text("Filter variables").margin_bottom(6).build();
        let session_group = adw::PreferencesGroup::builder()
            .title("Already in Your Session")
            .description(
                "Set by the system or your shell. Override one to give it a new value from Voltkit. A value set in \
                 ~/.bashrc still wins in terminals.",
            )
            .build();
        session_group.add(&search);
        let editor = Rc::new(Self {
            group,
            session_group,
            session_rows: RefCell::default(),
            search,
            rows: RefCell::default(),
            vars: RefCell::new(vars),
            toasts: toasts.clone(),
        });
        editor.rebuild();
        let weak = Rc::downgrade(&editor);
        add.connect_clicked(move |b| with_env(&weak, |e| e.edit(b, None, None)));
        let weak = Rc::downgrade(&editor);
        editor.search.connect_search_changed(move |_| with_env(&weak, |e| e.rebuild_session()));
        editor
    }

    /// Lists the session's variables that Voltkit doesn't set itself.
    fn rebuild_session(self: &Rc<Self>) {
        for row in self.session_rows.borrow_mut().drain(..) {
            self.session_group.remove(&row);
        }
        let query = self.search.text().to_lowercase();
        let managed: Vec<String> = self.vars.borrow().iter().map(|v| v.name.clone()).collect();
        let mut session: Vec<(String, String)> = std::env::vars()
            .filter(|(name, _)| !envvars::is_session_plumbing(name) && !managed.contains(name))
            .filter(|(name, value)| query.is_empty() || name.to_lowercase().contains(&query) || value.to_lowercase().contains(&query))
            .collect();
        session.sort();
        let mut rows = self.session_rows.borrow_mut();
        for (name, value) in session {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&name))
                .subtitle(glib::markup_escape_text(&value))
                .subtitle_lines(1)
                .build();
            let override_button = gtk::Button::builder().label("Override").valign(gtk::Align::Center).build();
            row.add_suffix(&override_button);
            let weak = Rc::downgrade(self);
            let prefill = Var { name, value };
            override_button.connect_clicked(move |b| with_env(&weak, |e| e.edit(b, None, Some(prefill.clone()))));
            self.session_group.add(&row);
            rows.push(row.upcast());
        }
    }

    fn rebuild(self: &Rc<Self>) {
        self.rebuild_session();
        for row in self.rows.borrow_mut().drain(..) {
            self.group.remove(&row);
        }
        let vars = self.vars.borrow();
        let mut rows = self.rows.borrow_mut();
        if vars.is_empty() {
            let row = adw::ActionRow::builder()
                .title("No variables yet")
                .subtitle("Click Add Variable to set one, such as EDITOR=nano.")
                .build();
            self.group.add(&row);
            rows.push(row.upcast());
            return;
        }
        for (index, var) in vars.iter().enumerate() {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(&var.name))
                .subtitle(glib::markup_escape_text(&var.value))
                .subtitle_selectable(true)
                .build();
            let edit = flat_button("document-edit-symbolic", "Edit");
            let delete = flat_button("user-trash-symbolic", "Delete");
            row.add_suffix(&edit);
            row.add_suffix(&delete);
            let weak = Rc::downgrade(self);
            edit.connect_clicked(move |b| with_env(&weak, |e| e.edit(b, Some(index), None)));
            let weak = Rc::downgrade(self);
            delete.connect_clicked(move |_| with_env(&weak, |e| e.delete(index)));
            self.group.add(&row);
            rows.push(row.upcast());
        }
    }

    fn save(&self) -> bool {
        match envvars::save(&self.vars.borrow()) {
            Ok(_) => true,
            Err(e) => {
                toast(&self.toasts, format!("Could not save: {e:#}"));
                false
            }
        }
    }

    fn delete(self: &Rc<Self>, index: usize) {
        let removed = self.vars.borrow_mut().remove(index);
        if !self.save() {
            self.vars.borrow_mut().insert(index, removed);
            return;
        }
        self.rebuild();
        let undo = adw::Toast::builder().title(format!("Deleted {}", removed.name)).button_label("Undo").build();
        let weak = Rc::downgrade(self);
        undo.connect_button_clicked(move |_| {
            with_env(&weak, |e| {
                let at = index.min(e.vars.borrow().len());
                e.vars.borrow_mut().insert(at, removed.clone());
                e.save();
                e.rebuild();
            })
        });
        self.toasts.add_toast(undo);
    }

    /// Opens the editor on a Voltkit variable (`index`), or on a new one,
    /// optionally filled in from a session variable being overridden.
    fn edit(self: &Rc<Self>, parent: &impl IsA<gtk::Widget>, index: Option<usize>, prefill: Option<Var>) {
        let existing = index.and_then(|i| self.vars.borrow().get(i).cloned()).or(prefill);
        let others: Vec<String> =
            self.vars.borrow().iter().enumerate().filter(|(i, _)| Some(*i) != index).map(|(_, v)| v.name.clone()).collect();
        let fields = [
            ("Name", existing.as_ref().map_or(String::new(), |v| v.name.clone())),
            ("Value", existing.as_ref().map_or(String::new(), |v| v.value.clone())),
        ];
        let weak = Rc::downgrade(self);
        form_dialog(
            parent.as_ref(),
            if index.is_some() { "Edit Variable" } else if existing.is_some() { "Override Variable" } else { "New Variable" },
            &fields,
            move |values| {
                let name = values[0].trim();
                if !envvars::valid_name(name) {
                    return Err("Names use letters, digits and underscores, and don't start with a digit.".into());
                }
                if others.iter().any(|o| o == name) {
                    return Err(format!("{name} is already set; edit that one instead."));
                }
                if values[1].contains('\n') {
                    return Err("Values must fit on one line.".into());
                }
                Ok(())
            },
            move |values| {
                with_env(&weak, |e| {
                    let var = Var { name: values[0].trim().to_owned(), value: values[1].clone() };
                    let name = var.name.clone();
                    match index {
                        Some(i) => e.vars.borrow_mut()[i] = var,
                        None => e.vars.borrow_mut().push(var),
                    }
                    if e.save() {
                        e.rebuild();
                        toast(&e.toasts, format!("Saved {name}. Log out and back in to apply it."));
                    }
                })
            },
        );
    }
}

// ---------------------------------------------------------------- helpers

fn with(weak: &Weak<HostsEditor>, f: impl FnOnce(Rc<HostsEditor>)) {
    if let Some(e) = weak.upgrade() {
        f(e);
    }
}

fn with_env(weak: &Weak<EnvEditor>, f: impl FnOnce(Rc<EnvEditor>)) {
    if let Some(e) = weak.upgrade() {
        f(e);
    }
}

fn header_button(icon: &str, label: &str) -> gtk::Button {
    gtk::Button::builder()
        .child(&adw::ButtonContent::builder().icon_name(icon).label(label).build())
        .valign(gtk::Align::Center)
        .css_classes(["suggested-action"])
        .build()
}

fn flat_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
}

/// A small dialog of text fields with Cancel and Save. Save stays disabled
/// while `validate` reports a problem, which is shown under the fields.
fn form_dialog(
    parent: &gtk::Widget,
    title: &str,
    fields: &[(&str, String)],
    validate: impl Fn(&[String]) -> Result<(), String> + 'static,
    on_save: impl Fn(Vec<String>) + 'static,
) {
    let dialog = adw::Dialog::builder().title(title).content_width(460).build();
    let cancel = gtk::Button::with_label("Cancel");
    let save = gtk::Button::builder().label("Save").css_classes(["suggested-action"]).build();
    let header = adw::HeaderBar::builder().show_start_title_buttons(false).show_end_title_buttons(false).build();
    header.pack_start(&cancel);
    header.pack_end(&save);

    let group = adw::PreferencesGroup::new();
    let rows: Vec<adw::EntryRow> = fields
        .iter()
        .map(|(label, value)| {
            let row = adw::EntryRow::builder().title(*label).text(value).build();
            group.add(&row);
            row
        })
        .collect();
    let problem = gtk::Label::builder().xalign(0.0).wrap(true).css_classes(["caption", "error"]).visible(false).build();
    let body = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(24)
        .margin_start(18)
        .margin_end(18)
        .build();
    body.append(&group);
    body.append(&problem);
    let content = adw::ToolbarView::new();
    content.add_top_bar(&header);
    content.set_content(Some(&body));
    dialog.set_child(Some(&content));

    let values = {
        let rows = rows.clone();
        move || rows.iter().map(|r| r.text().to_string()).collect::<Vec<_>>()
    };
    let check = {
        let (values, save, problem) = (values.clone(), save.clone(), problem.clone());
        move || {
            let result = validate(&values());
            save.set_sensitive(result.is_ok());
            match result {
                Ok(()) => problem.set_visible(false),
                Err(msg) => {
                    problem.set_label(&msg);
                    problem.set_visible(true);
                }
            }
        }
    };
    let check = Rc::new(check);
    check();
    for row in &rows {
        let check = check.clone();
        row.connect_changed(move |_| check());
    }
    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog.close();
        });
    }
    {
        let dialog = dialog.clone();
        save.connect_clicked(move |_| {
            on_save(values());
            dialog.close();
        });
    }
    dialog.present(Some(parent));
    if let Some(first) = rows.first() {
        first.grab_focus();
    }
}
