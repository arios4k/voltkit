//! Folder selection, comparison results, and backup verification.
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, atomic::Ordering::Relaxed};
use std::time::Duration;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::app::toast;
use crate::i18n::tr;
use crate::modules::folder_compare::{self, Progress, ResultSet, Status, status_label};
use crate::workspace::Workspace;

struct Compare {
    roots: RefCell<[Option<PathBuf>; 2]>,
    result: RefCell<Option<Arc<ResultSet>>>,
    progress: RefCell<Option<Arc<Progress>>>,
    rows: [adw::ActionRow; 2],
    inputs: adw::PreferencesGroup,
    summary: gtk::Label,
    quick: gtk::Button,
    verify: gtk::Button,
    cancel: gtk::Button,
    results: gtk::Box,
    filter: gtk::DropDown,
    export: gtk::Button,
    items: gtk::StringList,
    visible: RefCell<Vec<usize>>,
    toasts: glib::WeakRef<adw::ToastOverlay>,
}

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let workspace = Workspace::new();
    let inputs = adw::PreferencesGroup::builder()
        .title(tr("Folders"))
        .description(tr("Choose or drop a folder onto each row."))
        .build();
    let rows = [tr("Original"), tr("Backup or comparison folder")].map(|title| {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(tr("No folder selected"))
            .subtitle_lines(2)
            .use_markup(false)
            .build();
        inputs.add(&row);
        row
    });

    let quick = gtk::Button::with_label(tr("Quick Compare"));
    let verify = gtk::Button::builder()
        .label(tr("Verify Contents"))
        .css_classes(["suggested-action"])
        .build();
    let cancel = gtk::Button::with_label(tr("Cancel"));
    let buttons = gtk::Box::new(gtk::Orientation::Vertical, 6);
    for button in [&quick, &verify, &cancel] {
        buttons.append(button);
    }
    let summary = gtk::Label::builder().wrap(true).xalign(0.0).build();
    let controls = gtk::Box::new(gtk::Orientation::Vertical, 12);
    controls.append(&inputs);
    controls.append(&buttons);
    controls.append(&summary);
    workspace.add(&controls);

    let filter = gtk::DropDown::from_strings(&[
        tr("Differences and unreadable"),
        tr("All results"),
        status_label(Status::Changed),
        status_label(Status::OriginalOnly),
        status_label(Status::BackupOnly),
        status_label(Status::TypeMismatch),
        status_label(Status::Skipped),
    ]);
    let export = gtk::Button::with_label(tr("Export Report…"));
    let items = gtk::StringList::new(&[]);
    let selection = gtk::SingleSelection::new(Some(items.clone()));
    selection.set_autoselect(false);
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        item.set_child(Some(
            &gtk::Label::builder()
                .xalign(0.0)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .margin_top(6)
                .margin_bottom(6)
                .margin_start(8)
                .margin_end(8)
                .build(),
        ));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let label = item.child().and_downcast::<gtk::Label>().unwrap();
        let value = item
            .item()
            .and_downcast::<gtk::StringObject>()
            .unwrap()
            .string();
        label.set_label(&value);
        label.set_tooltip_text(Some(&value));
    });
    let list = gtk::ListView::new(Some(selection), Some(factory));
    let scroll = gtk::ScrolledWindow::builder()
        .height_request(300)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&list)
        .build();
    let results = gtk::Box::new(gtk::Orientation::Vertical, 8);
    results.append(&filter);
    results.append(
        &gtk::Label::builder()
            .label(tr("Open a changed file to preview text side by side."))
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build(),
    );
    results.append(&scroll);
    results.append(&export);
    workspace.add(&results);

    let state = Rc::new(Compare {
        roots: RefCell::new([None, None]),
        result: RefCell::new(None),
        progress: RefCell::new(None),
        rows,
        inputs,
        summary,
        quick,
        verify,
        cancel,
        results,
        filter,
        export,
        items,
        visible: RefCell::new(Vec::new()),
        toasts: toasts.downgrade(),
    });
    state.controls();
    for (index, row) in state.rows.iter().enumerate() {
        let button = gtk::Button::builder()
            .label(tr("Choose…"))
            .valign(gtk::Align::Center)
            .build();
        row.add_suffix(&button);
        let weak = Rc::downgrade(&state);
        button.connect_clicked(move |button| {
            let window = button.root().and_downcast::<gtk::Window>();
            let weak = weak.clone();
            gtk::FileDialog::builder()
                .title(tr("Choose a Folder"))
                .build()
                .select_folder(window.as_ref(), gio::Cancellable::NONE, move |choice| {
                    if let (Some(state), Ok(file)) = (weak.upgrade(), choice) {
                        if let Some(path) = file.path() {
                            state.set_folder(index, path);
                        } else {
                            state.notify(tr("Choose a local folder."));
                        }
                    }
                });
        });
        let drop = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let weak = Rc::downgrade(&state);
        drop.connect_drop(move |_, value, _, _| {
            let Some(state) = weak.upgrade() else {
                return false;
            };
            let Ok(files) = value.get::<gdk::FileList>() else {
                return false;
            };
            let files = files.files();
            if let [file] = files.as_slice()
                && let Some(path) = file.path()
            {
                state.set_folder(index, path);
                return true;
            }
            state.notify(tr("Drop one local folder at a time."));
            false
        });
        row.add_controller(drop);
    }
    for (button, verify) in [(&state.quick, false), (&state.verify, true)] {
        let weak = Rc::downgrade(&state);
        button.connect_clicked(move |_| {
            if let Some(state) = weak.upgrade() {
                state.run(verify);
            }
        });
    }
    let weak = Rc::downgrade(&state);
    state.cancel.connect_clicked(move |button| {
        if let Some(state) = weak.upgrade()
            && let Some(progress) = state.progress.borrow().as_ref()
        {
            progress.cancelled.store(true, Relaxed);
            button.set_sensitive(false);
            state.summary.set_label(tr("Cancelling…"));
        }
    });
    let weak = Rc::downgrade(&state);
    state.filter.connect_selected_notify(move |_| {
        if let Some(state) = weak.upgrade() {
            state.show_items();
        }
    });
    let weak = Rc::downgrade(&state);
    list.connect_activate(move |list, position| {
        if let Some(state) = weak.upgrade() {
            state.preview(list, position);
        }
    });
    let weak = Rc::downgrade(&state);
    state.export.connect_clicked(move |button| {
        if let Some(state) = weak.upgrade() {
            state.export(button);
        }
    });
    workspace.connect_destroy(move |_| {
        if let Some(progress) = state.progress.borrow().as_ref() {
            progress.cancelled.store(true, Relaxed);
        }
    });
    workspace.upcast()
}

impl Compare {
    fn notify(&self, message: impl AsRef<str>) {
        if let Some(toasts) = self.toasts.upgrade() {
            toast(&toasts, message);
        }
    }

    fn set_folder(&self, index: usize, path: PathBuf) {
        if self.progress.borrow().is_some() {
            return;
        }
        self.rows[index].set_subtitle(&path.to_string_lossy());
        self.roots.borrow_mut()[index] = Some(path);
        self.result.take();
        self.summary.set_label("");
        self.show_items();
        self.controls();
    }

    fn controls(&self) {
        let running = self.progress.borrow().is_some();
        let ready = !running && self.roots.borrow().iter().all(Option::is_some);
        self.inputs.set_sensitive(!running);
        self.quick.set_sensitive(ready);
        self.verify.set_sensitive(ready);
        self.cancel.set_visible(running);
        self.cancel.set_sensitive(running);
        self.results.set_visible(self.result.borrow().is_some());
    }

    fn run(self: &Rc<Self>, verified: bool) {
        let [Some(original), Some(backup)] = self.roots.borrow().clone() else {
            return;
        };
        let roots = [original, backup];
        let progress = Arc::new(Progress::default());
        self.progress.replace(Some(progress.clone()));
        self.result.take();
        self.show_items();
        self.controls();
        self.summary.set_label(tr("Comparing folders…"));

        let weak = Rc::downgrade(self);
        let ticker = glib::timeout_add_local(Duration::from_millis(180), {
            let weak = weak.clone();
            let progress = progress.clone();
            let previous = Cell::new(0);
            move || {
                if let Some(state) = weak.upgrade()
                    && !progress.cancelled.load(Relaxed)
                {
                    let checked = progress.checked.load(Relaxed);
                    if previous.replace(checked) != checked {
                        state
                            .summary
                            .set_label(&trf!("{} entries checked…", checked));
                    }
                }
                glib::ControlFlow::Continue
            }
        });
        glib::spawn_future_local(async move {
            let worker_progress = progress.clone();
            let result = gio::spawn_blocking(move || {
                let entries = folder_compare::compare(
                    roots.each_ref().map(PathBuf::as_path),
                    verified,
                    &worker_progress,
                )?;
                Ok::<_, std::io::Error>(ResultSet {
                    roots,
                    verified,
                    entries,
                })
            })
            .await;
            ticker.remove();
            let Some(state) = weak.upgrade() else { return };
            state.progress.take();
            match result {
                _ if progress.cancelled.load(Relaxed) => {
                    state.summary.set_label(tr("Comparison cancelled."))
                }
                Ok(Ok(result)) => {
                    state.summary.set_label(&result.summary());
                    state.result.replace(Some(Arc::new(result)));
                    state.show_items();
                }
                Ok(Err(error)) => state
                    .summary
                    .set_label(&trf!("Could not compare folders: {}", error)),
                Err(_) => state
                    .summary
                    .set_label(tr("The comparison stopped unexpectedly.")),
            }
            state.controls();
        });
    }

    fn show_items(&self) {
        let result = self.result.borrow();
        let filter = self.filter.selected();
        let mut visible = self.visible.borrow_mut();
        visible.clear();
        let mut labels = Vec::new();
        if let Some(result) = &*result {
            for (index, entry) in result.entries.iter().enumerate() {
                let show = match filter {
                    1 => true,
                    2 => entry.status == Status::Changed,
                    3 => entry.status == Status::OriginalOnly,
                    4 => entry.status == Status::BackupOnly,
                    5 => entry.status == Status::TypeMismatch,
                    6 => entry.status == Status::Skipped,
                    _ => !matches!(entry.status, Status::Match | Status::QuickMatch),
                };
                if show {
                    visible.push(index);
                    labels.push(ResultSet::entry_label(entry));
                }
            }
        }
        if labels.is_empty() && result.is_some() {
            labels.push(tr("No entries for this filter.").to_string());
        }
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        self.items.splice(0, self.items.n_items(), &labels);
    }

    fn preview(&self, list: &gtk::ListView, position: u32) {
        let result = self.result.borrow();
        let visible = self.visible.borrow();
        let Some((result, index)) = result.as_ref().zip(visible.get(position as usize)) else {
            return;
        };
        let entry = &result.entries[*index];
        if !entry.can_preview {
            self.notify(tr("Text preview requires a regular file on both sides."));
            return;
        }
        let paths = result.roots.each_ref().map(|root| root.join(&entry.path));
        let title = entry.path.to_string_lossy().into_owned();
        let parent = list
            .root()
            .and_downcast::<gtk::Window>()
            .map(|window| window.downgrade());
        glib::spawn_future_local(async move {
            let Ok(texts) =
                gio::spawn_blocking(move || paths.map(|path| folder_compare::preview(&path))).await
            else {
                return;
            };
            let Some(window) = parent.and_then(|parent| parent.upgrade()) else {
                return;
            };
            let dialog = adw::Dialog::builder()
                .title(&title)
                .content_width(840)
                .content_height(480)
                .build();
            let toolbar = adw::ToolbarView::new();
            toolbar.add_top_bar(&adw::HeaderBar::new());
            let columns = gtk::Box::builder().homogeneous(true).spacing(12).build();
            for (heading, text) in [tr("Original"), tr("Backup or comparison folder")]
                .into_iter()
                .zip(texts)
            {
                let column = gtk::Box::new(gtk::Orientation::Vertical, 6);
                column.append(&gtk::Label::builder().label(heading).build());
                let view = gtk::TextView::builder()
                    .editable(false)
                    .monospace(true)
                    .wrap_mode(gtk::WrapMode::WordChar)
                    .build();
                view.buffer().set_text(&text.unwrap_or_else(|_| {
                    tr("Preview unavailable. Choose readable UTF-8 text up to 128 KiB.").into()
                }));
                column.append(
                    &gtk::ScrolledWindow::builder()
                        .vexpand(true)
                        .child(&view)
                        .build(),
                );
                columns.append(&column);
            }
            toolbar.set_content(Some(&columns));
            dialog.set_child(Some(&toolbar));
            dialog.present(Some(&window));
        });
    }

    fn export(self: &Rc<Self>, button: &gtk::Button) {
        let Some(result) = self.result.borrow().clone() else {
            return;
        };
        let parent = button.root().and_downcast::<gtk::Window>();
        let weak = Rc::downgrade(self);
        glib::spawn_future_local(async move {
            let dialog = gtk::FileDialog::builder()
                .title(tr("Export Report…"))
                .initial_name("folder-compare.txt")
                .build();
            let Ok(file) = dialog.save_future(parent.as_ref()).await else {
                return;
            };
            let Ok(report) = gio::spawn_blocking(move || result.report(true)).await else {
                return;
            };
            let written = file
                .replace_contents_future(
                    report.into_bytes(),
                    None,
                    false,
                    gio::FileCreateFlags::REPLACE_DESTINATION,
                )
                .await;
            if let Some(state) = weak.upgrade() {
                match written {
                    Ok(_) => state.notify(tr("Comparison report saved.")),
                    Err((_, error)) => state.notify(trf!("Could not save the report: {}", error)),
                }
            }
        });
    }
}
