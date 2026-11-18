//! Interactive disk usage view backed by a cancellable scan.
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, atomic::Ordering::Relaxed};
use std::time::Duration;

use adw::prelude::*;
use gtk::{gdk, gio, glib, graphene};

use crate::app::toast;
use crate::i18n::tr;
use crate::modules::storage_map::{self, Node, Progress, Rect, Scan};
use crate::workspace::Workspace;

const TILE_LIMIT: usize = 24;
const PAGE_SIZE: usize = 100;

struct Tile {
    node: Option<usize>,
    name: String,
    bytes: u64,
    folder: bool,
}

struct StorageMap {
    scan: RefCell<Option<Scan>>,
    path: RefCell<Option<PathBuf>>,
    progress: RefCell<Option<Arc<Progress>>>,
    current: Cell<usize>,
    limit: Cell<usize>,
    visible: RefCell<Vec<usize>>,
    tiles: RefCell<Vec<Tile>>,
    map: gtk::DrawingArea,
    choose: gtk::Button,
    rescan: gtk::Button,
    cancel: gtk::Button,
    up: gtk::Button,
    open: gtk::Button,
    status: gtk::Label,
    heading: gtk::Label,
    summary: gtk::Label,
    results: gtk::Box,
    mode: gtk::DropDown,
    items: gtk::StringList,
    list: gtk::ListView,
    more: gtk::Button,
    toasts: glib::WeakRef<adw::ToastOverlay>,
}

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let workspace = Workspace::new();
    let choose = gtk::Button::builder()
        .label(tr("Scan Folder…"))
        .css_classes(["suggested-action"])
        .build();
    let rescan = gtk::Button::with_label(tr("Rescan"));
    let cancel = gtk::Button::with_label(tr("Cancel"));
    let controls = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for button in [&choose, &rescan, &cancel] {
        buttons.append(button);
    }
    controls.append(&buttons);
    let status = gtk::Label::builder()
        .label(tr(
            "Choose a folder or drop one here to see what takes up space.",
        ))
        .xalign(0.0)
        .wrap(true)
        .build();
    controls.append(&status);
    workspace.add(&controls);

    let results = gtk::Box::new(gtk::Orientation::Vertical, 10);
    results.set_visible(false);
    let up = gtk::Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text(tr("Go to Parent Folder"))
        .build();
    let open = gtk::Button::builder()
        .icon_name("folder-open-symbolic")
        .tooltip_text(tr("Open in File Manager"))
        .build();
    let heading = gtk::Label::builder()
        .xalign(0.0)
        .hexpand(true)
        .selectable(true)
        .ellipsize(gtk::pango::EllipsizeMode::Middle)
        .build();
    let navigation = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    navigation.append(&up);
    navigation.append(&heading);
    navigation.append(&open);
    results.append(&navigation);
    let summary = gtk::Label::builder().xalign(0.0).wrap(true).build();
    results.append(&summary);

    let map = gtk::DrawingArea::builder()
        .content_height(260)
        .hexpand(true)
        .has_tooltip(true)
        .build();
    results.append(&map);
    let mode = gtk::DropDown::from_strings(&[tr("Folders and files"), tr("Largest files")]);
    mode.set_halign(gtk::Align::Start);
    results.append(&mode);
    let hint = gtk::Label::builder()
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label"])
        .label(tr(
            "Open a folder to drill down, or open a file’s location in the file manager.",
        ))
        .build();
    results.append(&hint);

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
                .margin_start(8)
                .margin_end(8)
                .margin_top(5)
                .margin_bottom(5)
                .build(),
        ));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let label = item.child().and_downcast::<gtk::Label>().unwrap();
        let text = item
            .item()
            .and_downcast::<gtk::StringObject>()
            .unwrap()
            .string();
        label.set_label(&text);
        label.set_tooltip_text(Some(&text));
    });
    let list = gtk::ListView::new(Some(selection), Some(factory));
    results.append(
        &gtk::ScrolledWindow::builder()
            .height_request(250)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&list)
            .build(),
    );
    let more = gtk::Button::with_label(tr("Show More"));
    results.append(&more);
    workspace.add(&results);

    let state = Rc::new(StorageMap {
        scan: RefCell::new(None),
        path: RefCell::new(None),
        progress: RefCell::new(None),
        current: Cell::new(0),
        limit: Cell::new(PAGE_SIZE),
        visible: RefCell::new(Vec::new()),
        tiles: RefCell::new(Vec::new()),
        map,
        choose,
        rescan,
        cancel,
        up,
        open,
        status,
        heading,
        summary,
        results,
        mode,
        items,
        list: list.clone(),
        more,
        toasts: toasts.downgrade(),
    });
    state.controls();

    let weak = Rc::downgrade(&state);
    state.choose.connect_clicked(move |button| {
        let parent = button.root().and_downcast::<gtk::Window>();
        let weak = weak.clone();
        gtk::FileDialog::builder()
            .title(tr("Choose a Folder"))
            .build()
            .select_folder(parent.as_ref(), gio::Cancellable::NONE, move |choice| {
                if let (Some(state), Ok(file)) = (weak.upgrade(), choice) {
                    if let Some(path) = file.path() {
                        state.start(path);
                    } else {
                        state.notify(tr("Choose a local folder."));
                    }
                }
            });
    });
    let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    let weak = Rc::downgrade(&state);
    target.connect_drop(move |_, value, _, _| {
        let Some(state) = weak.upgrade() else {
            return false;
        };
        let Ok(files) = value.get::<gdk::FileList>() else {
            return false;
        };
        if let [file] = files.files().as_slice()
            && let Some(path) = file.path()
            && state.progress.borrow().is_none()
        {
            state.start(path);
            true
        } else {
            false
        }
    });
    controls.add_controller(target);
    let weak = Rc::downgrade(&state);
    state.rescan.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            let path = state.path.borrow().clone();
            if let Some(path) = path {
                state.start(path);
            }
        }
    });
    let weak = Rc::downgrade(&state);
    state.cancel.connect_clicked(move |button| {
        if let Some(state) = weak.upgrade()
            && let Some(progress) = state.progress.borrow().as_ref()
        {
            progress.cancelled.store(true, Relaxed);
            button.set_sensitive(false);
            state.status.set_label(tr("Cancelling…"));
        }
    });
    let weak = Rc::downgrade(&state);
    state.up.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            let parent = state
                .scan
                .borrow()
                .as_ref()
                .and_then(|scan| scan.nodes[state.current.get()].parent);
            if let Some(parent) = parent {
                state.show_folder(parent);
            }
        }
    });
    let weak = Rc::downgrade(&state);
    state.open.connect_clicked(move |button| {
        if let Some(state) = weak.upgrade() {
            state.open_location(state.current.get(), button);
        }
    });
    let weak = Rc::downgrade(&state);
    state.mode.connect_selected_notify(move |_| {
        if let Some(state) = weak.upgrade() {
            state.limit.set(PAGE_SIZE);
            state.show_list();
        }
    });
    let weak = Rc::downgrade(&state);
    state.more.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            state.limit.set(state.limit.get().saturating_add(PAGE_SIZE));
            state.show_list();
        }
    });
    let weak = Rc::downgrade(&state);
    list.connect_activate(move |list, position| {
        if let Some(state) = weak.upgrade() {
            let id = state.visible.borrow().get(position as usize).copied();
            if let Some(id) = id {
                state.activate(id, list);
            }
        }
    });

    let weak = Rc::downgrade(&state);
    state.map.set_draw_func(move |area, cr, width, height| {
        if let Some(state) = weak.upgrade() {
            state.draw(area, cr, width, height);
        }
    });
    let weak = Rc::downgrade(&state);
    state
        .map
        .connect_query_tooltip(move |area, x, y, keyboard, tooltip| {
            let Some(state) = weak.upgrade().filter(|_| !keyboard) else {
                return false;
            };
            let tiles = state.tiles.borrow();
            let Some(index) = tile_at(&tiles, area, f64::from(x), f64::from(y)) else {
                return false;
            };
            let tile = &tiles[index];
            tooltip.set_text(Some(&format!(
                "{} · {}",
                tile.name,
                glib::format_size(tile.bytes)
            )));
            true
        });
    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_PRIMARY);
    let weak = Rc::downgrade(&state);
    click.connect_released(move |_, _, x, y| {
        if let Some(state) = weak.upgrade() {
            let id = {
                let tiles = state.tiles.borrow();
                tile_at(&tiles, &state.map, x, y).map(|index| tiles[index].node)
            };
            match id {
                Some(Some(id)) => state.activate(id, &state.map),
                Some(None) => {
                    state.mode.set_selected(0);
                    state
                        .list
                        .scroll_to(TILE_LIMIT as u32, gtk::ListScrollFlags::FOCUS, None);
                }
                None => {}
            }
        }
    });
    state.map.add_controller(click);
    state.map.set_cursor_from_name(Some("pointer"));

    workspace.connect_destroy(move |_| {
        if let Some(progress) = state.progress.borrow().as_ref() {
            progress.cancelled.store(true, Relaxed);
        }
    });
    workspace.upcast()
}

impl StorageMap {
    fn controls(&self) {
        let running = self.progress.borrow().is_some();
        self.choose.set_sensitive(!running);
        self.rescan
            .set_visible(!running && self.path.borrow().is_some());
        self.rescan.set_sensitive(!running);
        self.cancel.set_visible(running);
        self.cancel.set_sensitive(running);
        self.results
            .set_visible(!running && self.scan.borrow().is_some());
    }

    fn notify(&self, text: impl AsRef<str>) {
        if let Some(toasts) = self.toasts.upgrade() {
            toast(&toasts, text);
        }
    }

    fn start(self: &Rc<Self>, path: PathBuf) {
        if self.progress.borrow().is_some() {
            return;
        }
        self.path.replace(Some(path.clone()));
        self.scan.take();
        self.visible.borrow_mut().clear();
        self.tiles.borrow_mut().clear();
        self.items.splice(0, self.items.n_items(), &[]);
        let progress = Arc::new(Progress::default());
        self.progress.replace(Some(progress.clone()));
        self.controls();
        self.status.set_label(tr("Scanning storage…"));
        let weak = Rc::downgrade(self);
        let ticker = glib::timeout_add_local(Duration::from_millis(180), {
            let weak = weak.clone();
            let progress = progress.clone();
            let last = Cell::new(0);
            move || {
                if let Some(state) = weak.upgrade()
                    && !progress.cancelled.load(Relaxed)
                {
                    let scanned = progress.scanned.load(Relaxed);
                    if last.replace(scanned) != scanned {
                        state
                            .status
                            .set_label(&trf!("{} entries scanned…", scanned));
                    }
                }
                glib::ControlFlow::Continue
            }
        });
        glib::spawn_future_local(async move {
            let worker = progress.clone();
            let result = gio::spawn_blocking(move || storage_map::scan(&path, &worker)).await;
            ticker.remove();
            let Some(state) = weak.upgrade() else { return };
            state.progress.take();
            match result {
                _ if progress.cancelled.load(Relaxed) => {
                    state.status.set_label(tr("Scan cancelled."))
                }
                Ok(Ok(scan)) => {
                    state.scan.replace(Some(scan));
                    state.status.set_label("");
                    state.show_folder(0);
                }
                Ok(Err(error)) => state
                    .status
                    .set_label(&trf!("Could not scan the folder: {}", error)),
                Err(_) => state.status.set_label(tr("The scan stopped unexpectedly.")),
            }
            state.controls();
        });
    }

    fn show_folder(&self, id: usize) {
        self.current.set(id);
        self.limit.set(PAGE_SIZE);
        {
            let scan = self.scan.borrow();
            let Some(scan) = scan.as_ref() else { return };
            let node = &scan.nodes[id];
            let path = node.path.to_string_lossy();
            self.heading.set_label(&path);
            self.heading.set_tooltip_text(Some(&path));
            self.up.set_sensitive(node.parent.is_some());
            let total = trf!("{} on disk", glib::format_size(node.bytes));
            let summary = if node.skipped == 0 {
                total
            } else {
                format!(
                    "{total}\n{}",
                    trf!("{} entries could not be scanned.", node.skipped)
                )
            };
            self.summary.set_label(&summary);
            let mut tiles = self.tiles.borrow_mut();
            tiles.clear();
            let mut other = 0_u64;
            for &child in &node.children {
                let item = &scan.nodes[child];
                if item.bytes == 0 {
                    break;
                }
                if tiles.len() == TILE_LIMIT {
                    other = other.saturating_add(item.bytes);
                } else {
                    tiles.push(Tile {
                        node: Some(child),
                        name: name(item).into(),
                        bytes: item.bytes,
                        folder: item.directory,
                    });
                }
            }
            if other > 0 {
                let index = tiles.partition_point(|tile| tile.bytes >= other);
                tiles.insert(
                    index,
                    Tile {
                        node: None,
                        name: tr("Other items").into(),
                        bytes: other,
                        folder: false,
                    },
                );
            }
            self.map.set_visible(!tiles.is_empty());
            self.map.queue_draw();
        }
        self.show_list();
        self.list.scroll_to(0, gtk::ListScrollFlags::NONE, None);
    }

    fn show_list(&self) {
        let scan = self.scan.borrow();
        let Some(scan) = scan.as_ref() else { return };
        let folder = &scan.nodes[self.current.get()];
        let files_only = self.mode.selected() == 1;
        let ids = if files_only {
            &scan.files
        } else {
            &folder.children
        };
        let mut visible = self.visible.borrow_mut();
        visible.clear();
        visible.extend(
            ids.iter()
                .copied()
                .filter(|&id| !files_only || scan.nodes[id].path.starts_with(&folder.path))
                .take(self.limit.get().saturating_add(1)),
        );
        let more = visible.len() > self.limit.get();
        visible.truncate(self.limit.get());
        self.more.set_visible(more);
        let labels: Vec<_> = visible
            .iter()
            .map(|&id| {
                let item = &scan.nodes[id];
                let path = item.path.strip_prefix(&folder.path).unwrap_or(&item.path);
                let suffix = if item.directory { "/" } else { "" };
                format!(
                    "{} · {}{suffix}",
                    glib::format_size(item.bytes),
                    path.display()
                )
            })
            .collect();
        let mut labels: Vec<_> = labels.iter().map(String::as_str).collect();
        if labels.is_empty() {
            labels.push(tr("No items to show."));
        }
        self.items.splice(0, self.items.n_items(), &labels);
    }

    fn activate(&self, id: usize, anchor: &impl IsA<gtk::Widget>) {
        let folder = self
            .scan
            .borrow()
            .as_ref()
            .is_some_and(|scan| scan.nodes[id].directory);
        if folder {
            self.show_folder(id);
        } else {
            self.open_location(id, anchor);
        }
    }

    fn open_location(&self, id: usize, anchor: &impl IsA<gtk::Widget>) {
        let scan = self.scan.borrow();
        let Some(scan) = scan.as_ref() else { return };
        let item = &scan.nodes[id];
        let parent = anchor.root().and_downcast::<gtk::Window>();
        let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(&item.path)));
        let toasts = self.toasts.clone();
        let done = move |result| {
            if let (Some(toasts), Err(error)) = (toasts.upgrade(), result) {
                toast(&toasts, trf!("Could not open the location: {}", error));
            }
        };
        if item.directory {
            launcher.launch(parent.as_ref(), gio::Cancellable::NONE, done);
        } else {
            launcher.open_containing_folder(parent.as_ref(), gio::Cancellable::NONE, done);
        }
    }

    fn draw(&self, area: &gtk::DrawingArea, cr: &gtk::cairo::Context, width: i32, height: i32) {
        let tiles = self.tiles.borrow();
        let dark = adw::StyleManager::default().is_dark();
        let snapshot = gtk::Snapshot::new();
        for (tile, rect) in tiles.iter().zip(rectangles(&tiles, width, height)) {
            let (x, y, w, h) = (
                rect.x + 1.0,
                rect.y + 1.0,
                rect.width - 2.0,
                rect.height - 2.0,
            );
            if w <= 0.0 || h <= 0.0 {
                continue;
            }
            let color = match (dark, tile.folder, tile.node.is_none()) {
                (true, _, true) => (0.25, 0.27, 0.31),
                (true, true, _) => (0.16, 0.32, 0.53),
                (true, false, _) => (0.14, 0.39, 0.34),
                (false, _, true) => (0.84, 0.85, 0.88),
                (false, true, _) => (0.68, 0.81, 0.98),
                (false, false, _) => (0.66, 0.87, 0.80),
            };
            cr.set_source_rgb(color.0, color.1, color.2);
            cr.rectangle(x, y, w, h);
            let _ = cr.fill();

            if w >= 64.0 && h >= 25.0 {
                let text = if h >= 46.0 {
                    format!("{}\n{}", tile.name, glib::format_size(tile.bytes))
                } else {
                    tile.name.clone()
                };
                let layout = area.create_pango_layout(Some(&text));
                layout.set_width(((w - 16.0) * f64::from(gtk::pango::SCALE)) as i32);
                layout.set_height(-1);
                layout.set_ellipsize(gtk::pango::EllipsizeMode::End);
                let foreground = if dark { 1.0 } else { 0.10 };
                snapshot.save();
                snapshot.push_clip(&graphene::Rect::new(x as f32, y as f32, w as f32, h as f32));
                snapshot.translate(&graphene::Point::new((x + 8.0) as f32, (y + 5.0) as f32));
                snapshot.append_layout(
                    &layout,
                    &gdk::RGBA::new(foreground, foreground, foreground, 1.0),
                );
                snapshot.pop();
                snapshot.restore();
            }
        }
        if let Some(node) = snapshot.to_node() {
            node.draw(cr);
        }
    }
}

fn name(node: &Node) -> std::borrow::Cow<'_, str> {
    node.path
        .file_name()
        .map(Path::new)
        .unwrap_or(&node.path)
        .to_string_lossy()
}

fn rectangles(tiles: &[Tile], width: i32, height: i32) -> Vec<Rect> {
    let weights: Vec<_> = tiles.iter().map(|tile| tile.bytes).collect();
    storage_map::treemap(
        &weights,
        Rect {
            x: 0.0,
            y: 0.0,
            width: f64::from(width),
            height: f64::from(height),
        },
    )
}

fn tile_at(tiles: &[Tile], area: &gtk::DrawingArea, x: f64, y: f64) -> Option<usize> {
    rectangles(tiles, area.width(), area.height())
        .iter()
        .position(|rect| rect.contains(x, y))
}
