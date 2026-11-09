//! The Batch Rename page: add files, set what to change, watch the preview, rename, undo.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::app::toast;
use crate::i18n::tr;
use crate::modules::rename::{self, Case, Options, Scope, Status};

/// Rows shown in the preview; the rest are counted but not listed.
const SHOWN: usize = 200;

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let files: Rc<RefCell<Vec<PathBuf>>> = Rc::default();

    // ---- Files
    let files_group = adw::PreferencesGroup::builder()
        .title(tr("Files and folders"))
        .description(tr("Drop files or folders here, or add them with the buttons. They are numbered in the order shown."))
        .build();
    let add_files = gtk::Button::builder().label(tr("Add Files…")).valign(gtk::Align::Center).build();
    let add_folders = gtk::Button::builder().label(tr("Add Folders…")).valign(gtk::Align::Center).build();
    let clear = gtk::Button::builder().label(tr("Clear")).valign(gtk::Align::Center).sensitive(false).build();
    let buttons = gtk::Box::builder().spacing(8).margin_top(8).halign(gtk::Align::Start).build();
    buttons.append(&add_files);
    buttons.append(&add_folders);
    buttons.append(&clear);
    let count = adw::ActionRow::builder().title(tr("Nothing added yet")).subtitle(tr("Drop files here to start")).build();
    count.add_prefix(&gtk::Image::builder().icon_name("folder-download-symbolic").accessible_role(gtk::AccessibleRole::Presentation).build());
    files_group.add(&count);
    files_group.add(&buttons);

    // ---- What to change
    let rules = adw::PreferencesGroup::builder()
        .title(tr("Search and replace"))
        .description(tr("In the replacement, {n} is the item's number and {n:3} pads it to 3 digits. Leave the search empty to replace the whole name."))
        .build();
    let search = adw::EntryRow::builder().title(tr("Search for")).build();
    let replace = adw::EntryRow::builder().title(tr("Replace with")).build();
    let regex = adw::SwitchRow::builder().title(tr("Regular expression")).subtitle(tr("Use $1, $2… in the replacement for the parts you grouped with ( )")).build();
    let case_sensitive = adw::SwitchRow::builder().title(tr("Match case")).active(true).build();
    let all = adw::SwitchRow::builder().title(tr("Replace every match")).subtitle(tr("Off: only the first match in each name")).active(true).build();
    let scope = adw::ComboRow::builder()
        .title(tr("Change"))
        .model(&gtk::StringList::new(&[tr("The name, not the extension"), tr("Only the extension"), tr("The whole name")]))
        .build();
    let letter = adw::ComboRow::builder()
        .title(tr("Letter case"))
        .model(&gtk::StringList::new(&[tr("Keep as it is"), tr("UPPERCASE"), tr("lowercase"), tr("Title Case")]))
        .build();
    let numbering = adw::ExpanderRow::builder().title(tr("Numbering")).subtitle(tr("Where {n} starts and how much it grows")).build();
    let start = adw::SpinRow::with_range(-1_000_000.0, 1_000_000.0, 1.0);
    start.set_title(tr("Start at"));
    start.set_value(1.0);
    let step = adw::SpinRow::with_range(-1000.0, 1000.0, 1.0);
    step.set_title(tr("Step"));
    step.set_value(1.0);
    numbering.add_row(&start);
    numbering.add_row(&step);
    for w in [search.upcast_ref::<gtk::Widget>(), replace.upcast_ref(), regex.upcast_ref(), case_sensitive.upcast_ref(), all.upcast_ref(), scope.upcast_ref(), letter.upcast_ref(), numbering.upcast_ref()] {
        rules.add(w);
    }

    // ---- Preview
    let preview = adw::PreferencesGroup::builder().title(tr("Preview")).description(tr("Add files to see what would be renamed.")).build();
    let rows: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::default();
    let rename_button = gtk::Button::builder().label(tr("Rename")).css_classes(["suggested-action"]).sensitive(false).build();
    let undo_button = gtk::Button::builder().label(tr("Undo Last Rename")).sensitive(false).tooltip_text(tr("Puts the names of the last batch back")).build();
    let actions = gtk::Box::builder().spacing(8).margin_top(8).halign(gtk::Align::End).build();
    actions.append(&undo_button);
    actions.append(&rename_button);

    let options = {
        let (search, replace, regex, case_sensitive, all, scope, letter, start, step) =
            (search.clone(), replace.clone(), regex.clone(), case_sensitive.clone(), all.clone(), scope.clone(), letter.clone(), start.clone(), step.clone());
        Rc::new(move || Options {
            search: search.text().to_string(),
            replace: replace.text().to_string(),
            regex: regex.is_active(),
            case_sensitive: case_sensitive.is_active(),
            all_matches: all.is_active(),
            scope: match scope.selected() {
                1 => Scope::Extension,
                2 => Scope::Full,
                _ => Scope::Name,
            },
            case: match letter.selected() {
                1 => Case::Upper,
                2 => Case::Lower,
                3 => Case::Title,
                _ => Case::Keep,
            },
            start: start.value() as i64,
            step: step.value() as i64,
        })
    };

    let refresh_undo = {
        let undo_button = undo_button.clone();
        Rc::new(move || undo_button.set_sensitive(!rename::last_batch().is_empty()))
    };
    refresh_undo();

    let ready_count = Rc::new(Cell::new(0usize));
    let refresh: Rc<dyn Fn()> = {
        let (files, rows, preview, options, count, clear, rename_button, ready_count) =
            (files.clone(), rows.clone(), preview.clone(), options.clone(), count.clone(), clear.clone(), rename_button.clone(), ready_count.clone());
        Rc::new(move || {
            for r in rows.borrow_mut().drain(..) {
                preview.remove(&r);
            }
            let list = files.borrow();
            clear.set_sensitive(!list.is_empty());
            if list.is_empty() {
                count.set_title(tr("Nothing added yet"));
                count.set_subtitle(tr("Drop files here to start"));
                preview.set_description(Some(tr("Add files to see what would be renamed.")));
                rename_button.set_sensitive(false);
                ready_count.set(0);
                return;
            }
            count.set_title(&trnf!("{} item", "{} items", list.len()));
            count.set_subtitle(tr("Drop more to add them. Clear starts again."));
            let items = match rename::plan(&list, &options()) {
                Ok(items) => items,
                Err(e) => {
                    preview.set_description(Some(&glib::markup_escape_text(&format!("{e:#}"))));
                    rename_button.set_sensitive(false);
                    ready_count.set(0);
                    return;
                }
            };
            let ready = items.iter().filter(|i| i.status == Status::Ready).count();
            let problems = items.iter().filter(|i| matches!(i.status, Status::Problem(_))).count();
            let unchanged = items.len() - ready - problems;
            preview.set_description(Some(&crate::i18n::fill(tr("{} to rename, {} unchanged, {} with problems"), &[&ready, &unchanged, &problems])));
            rename_button.set_sensitive(ready > 0 && problems == 0);
            ready_count.set(ready);
            for item in items.iter().take(SHOWN) {
                let from = item.from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let to = item.to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let (icon, subtitle, css) = match &item.status {
                    Status::Ready => ("emblem-ok-symbolic", format!("→ {to}"), "success"),
                    Status::Unchanged => ("content-loading-symbolic", tr("Unchanged").to_string(), "dim-label"),
                    Status::Problem(why) => ("dialog-warning-symbolic", format!("→ {to}\n{why}"), "warning"),
                };
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&from))
                    .subtitle(glib::markup_escape_text(&subtitle))
                    .subtitle_lines(3)
                    .tooltip_text(item.from.display().to_string())
                    .build();
                let image = gtk::Image::builder().icon_name(icon).accessible_role(gtk::AccessibleRole::Presentation).build();
                image.add_css_class(css);
                row.add_prefix(&image);
                preview.add(&row);
                rows.borrow_mut().push(row);
            }
            if items.len() > SHOWN {
                let more = adw::ActionRow::builder().title(crate::i18n::fill(tr("and {} more"), &[&(items.len() - SHOWN)])).build();
                preview.add(&more);
                rows.borrow_mut().push(more);
            }
        })
    };

    // Change anything and the preview follows.
    for w in [&search, &replace] {
        let refresh = refresh.clone();
        w.connect_changed(move |_| refresh());
    }
    for w in [&regex, &case_sensitive, &all] {
        let refresh = refresh.clone();
        w.connect_active_notify(move |_| refresh());
    }
    for w in [&scope, &letter] {
        let refresh = refresh.clone();
        w.connect_selected_notify(move |_| refresh());
    }
    for w in [&start, &step] {
        let refresh = refresh.clone();
        w.connect_value_notify(move |_| refresh());
    }

    // Adding files.
    let add: Rc<dyn Fn(Vec<PathBuf>)> = {
        let (files, refresh) = (files.clone(), refresh.clone());
        Rc::new(move |paths: Vec<PathBuf>| {
            {
                let mut list = files.borrow_mut();
                for p in paths {
                    if !list.contains(&p) {
                        list.push(p);
                    }
                }
            }
            refresh();
        })
    };
    let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    {
        let add = add.clone();
        target.connect_drop(move |_, value, _, _| {
            let Ok(list) = value.get::<gdk::FileList>() else { return false };
            add(list.files().iter().filter_map(|f| f.path()).collect());
            true
        });
    }
    files_group.add_controller(target);
    {
        let add = add.clone();
        add_files.connect_clicked(move |b| {
            let add = add.clone();
            let parent = b.root().and_downcast::<gtk::Window>();
            gtk::FileDialog::builder().title(tr("Add Files")).build().open_multiple(parent.as_ref(), gio::Cancellable::NONE, move |res| {
                let Ok(model) = res else { return };
                add((0..model.n_items()).filter_map(|i| model.item(i).and_downcast::<gio::File>().and_then(|f| f.path())).collect());
            });
        });
    }
    {
        let add = add.clone();
        add_folders.connect_clicked(move |b| {
            let add = add.clone();
            let parent = b.root().and_downcast::<gtk::Window>();
            gtk::FileDialog::builder().title(tr("Add Folders")).build().select_multiple_folders(parent.as_ref(), gio::Cancellable::NONE, move |res| {
                let Ok(model) = res else { return };
                add((0..model.n_items()).filter_map(|i| model.item(i).and_downcast::<gio::File>().and_then(|f| f.path())).collect());
            });
        });
    }
    {
        let (files, refresh) = (files.clone(), refresh.clone());
        clear.connect_clicked(move |_| {
            files.borrow_mut().clear();
            refresh();
        });
    }

    // Rename and undo.
    {
        let (files, options, refresh, refresh_undo, toasts) = (files.clone(), options.clone(), refresh.clone(), refresh_undo.clone(), toasts.clone());
        rename_button.connect_clicked(move |_| {
            let items = match rename::plan(&files.borrow(), &options()) {
                Ok(i) => i,
                Err(e) => return toast(&toasts, format!("{e:#}")),
            };
            match rename::apply(&items) {
                Ok(done) => {
                    if let Err(e) = rename::remember(&done) {
                        toast(&toasts, format!("{}: {e:#}", tr("The rename worked, but it can't be undone")));
                    } else {
                        toast(&toasts, trnf!("Renamed {} item", "Renamed {} items", done.len()));
                    }
                    // Keep following the same items under their new names.
                    let moved: std::collections::HashMap<PathBuf, PathBuf> = done.into_iter().map(|d| (d.from, d.to)).collect();
                    for p in files.borrow_mut().iter_mut() {
                        if let Some(to) = moved.get(p) {
                            *p = to.clone();
                        }
                    }
                    refresh();
                    refresh_undo();
                }
                Err(e) => toast(&toasts, format!("{e:#}")),
            }
        });
    }
    {
        let (files, refresh, refresh_undo, toasts) = (files.clone(), refresh.clone(), refresh_undo.clone(), toasts.clone());
        undo_button.connect_clicked(move |_| {
            let batch = rename::last_batch();
            match rename::undo() {
                Ok(n) => {
                    toast(&toasts, trnf!("Undone: {} item has its old name again", "Undone: {} items have their old names again", n));
                    let back: std::collections::HashMap<PathBuf, PathBuf> = batch.into_iter().map(|d| (d.to, d.from)).collect();
                    for p in files.borrow_mut().iter_mut() {
                        if let Some(from) = back.get(p) {
                            *p = from.clone();
                        }
                    }
                    refresh();
                    refresh_undo();
                }
                Err(e) => toast(&toasts, format!("{e:#}")),
            }
        });
    }

    // Made-up files for the manual's screenshots.
    if std::env::var_os("VOLTKIT_RENAME_DEMO").is_some() {
        let dir = std::env::temp_dir().join(format!("voltkit-rename-demo-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let made: Vec<PathBuf> = ["IMG_0412.jpg", "IMG_0413.jpg", "IMG_0414.jpg", "IMG_0415.jpg", "IMG_0416.jpg", "notes.txt"]
            .iter()
            .map(|n| {
                let p = dir.join(n);
                let _ = std::fs::write(&p, n);
                p
            })
            .collect();
        search.set_text("IMG");
        replace.set_text("Holiday-{n:2}");
        add(made);
    }

    let page = crate::workspace::Workspace::new();
    page.add(&files_group);
    page.add(&rules);
    preview.add(&actions);
    page.aside(&preview);
    page.upcast()
}
