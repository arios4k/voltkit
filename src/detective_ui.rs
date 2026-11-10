//! The Shortcut Detective page: look up a shortcut by pressing it, see
//! clashes, browse every system-wide shortcut, and change them.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, glib};

use crate::app::toast;
use crate::modules::keybindings::{self as kb, Binding};
use crate::i18n::tr;

const SUGGESTIONS: usize = 6;

/// Reports a change's result and closes the dialog when it worked.
type Finish = dyn Fn(anyhow::Result<()>, &str);

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    if !kb::supported() {
        return adw::StatusPage::builder().icon_name("input-keyboard-symbolic")
            .title(tr("Shortcuts aren't available on this desktop"))
            .description(tr("Shortcut Detective supports GNOME, Cinnamon, Budgie and MATE. On KDE Plasma, open System Settings → Shortcuts."))
            .build().upcast();
    }
    let view = View::new(toasts);
    if std::env::var_os("VOLTKIT_SHORTCUTS_DEMO").is_some() {
        *view.looked_up.borrow_mut() = Some("<Super>v".into());
    }
    view.reload();
    let page = view.root.clone();
    page.connect_destroy(move |_| { let _ = &view; });
    page.upcast()
}

struct View {
    root: adw::BreakpointBin,
    search: gtk::SearchEntry,
    list: gtk::ListBox,
    detail: gtk::Box,
    count: gtk::Label,
    recorded: gtk::Label,
    conflicts_only: gtk::ToggleButton,
    bindings: RefCell<Vec<Binding>>,
    shown: RefCell<Vec<Binding>>,
    looked_up: RefCell<Option<String>>,
    toasts: adw::ToastOverlay,
}

/// The "Keyboard keys" section: Caps Lock, Alt and Super, and the Compose key, as desktop
/// keyboard options. They apply at once and are undone by choosing the first entry.
fn keyboard_keys(toasts: &adw::ToastOverlay) -> Option<adw::PreferencesGroup> {
    use crate::modules::xkb;
    if !xkb::supported() {
        return None;
    }
    let group = adw::PreferencesGroup::builder()
        .title(tr("Keyboard keys"))
        .description(tr("Change what a key sends, in every app. It takes effect at once, and choosing the first entry puts the key back."))
        .margin_top(6)
        .build();
    let options = std::rc::Rc::new(RefCell::new(xkb::read()));
    let rows: Vec<(&'static xkb::Group, adw::ComboRow)> = xkb::GROUPS
        .iter()
        .map(|g| {
            let labels: Vec<&str> = g.choices.iter().map(|(_, name)| tr(name)).collect();
            let row = adw::ComboRow::builder().title(xkb::title(g)).subtitle(tr(g.subtitle)).model(&gtk::StringList::new(&labels)).build();
            row.set_selected(xkb::selected(&options.borrow(), g) as u32);
            group.add(&row);
            (g, row)
        })
        .collect();
    // Choosing in one row can change another (Compose on Caps Lock takes Caps Lock's other jobs).
    let syncing = std::rc::Rc::new(Cell::new(false));
    for (g, row) in &rows {
        let (rows, options, toasts, syncing, id) = (rows.iter().map(|(g, r)| (*g, r.clone())).collect::<Vec<_>>(), options.clone(), toasts.clone(), syncing.clone(), g.id);
        row.connect_selected_notify(move |row| {
            if syncing.get() {
                return;
            }
            let updated = xkb::with_choice(&options.borrow(), id, row.selected() as usize);
            match xkb::write(&updated) {
                Ok(()) => {
                    *options.borrow_mut() = updated;
                    syncing.set(true);
                    for (g, r) in &rows {
                        r.set_selected(xkb::selected(&options.borrow(), g) as u32);
                    }
                    syncing.set(false);
                    toast(&toasts, tr("Keyboard keys changed"));
                }
                Err(e) => toast(&toasts, format!("{e:#}")),
            }
        });
    }
    let reset = adw::ActionRow::builder().title(tr("Put all keys back")).subtitle(tr("Caps Lock, Alt and Super and the Compose key return to how they come.")).activatable(true).build();
    reset.add_prefix(&gtk::Image::builder().icon_name("edit-undo-symbolic").accessible_role(gtk::AccessibleRole::Presentation).build());
    {
        let (rows, options, toasts, syncing) = (rows.iter().map(|(g, r)| (*g, r.clone())).collect::<Vec<_>>(), options.clone(), toasts.clone(), syncing.clone());
        reset.connect_activated(move |_| {
            let mut updated = options.borrow().clone();
            for g in xkb::GROUPS {
                updated = xkb::with_choice(&updated, g.id, 0);
            }
            match xkb::write(&updated) {
                Ok(()) => {
                    *options.borrow_mut() = updated;
                    syncing.set(true);
                    for (_, r) in &rows {
                        r.set_selected(0);
                    }
                    syncing.set(false);
                    toast(&toasts, tr("The keys are back as they come"));
                }
                Err(e) => toast(&toasts, format!("{e:#}")),
            }
        });
    }
    group.add(&reset);
    Some(group)
}

fn label(text: &str, class: &str) -> gtk::Label {
    gtk::Label::builder().label(text).xalign(0.0).wrap(class != "keycap").css_classes([class]).build()
}

fn keycaps(accel: &str) -> gtk::FlowBox {
    let keys = gtk::FlowBox::builder().selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(2).max_children_per_line(7).column_spacing(4).row_spacing(4)
        .halign(gtk::Align::Start).valign(gtk::Align::Center).build();
    for key in kb::label(accel).split('+') {
        keys.insert(&label(key.trim(), "keycap"), -1);
    }
    keys
}

impl View {
    fn new(toasts: &adw::ToastOverlay) -> Rc<Self> {
        let body = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).css_classes(["detective-workspace"]).build();
        let intro = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).css_classes(["detective-card"]).build();
        intro.append(&label(tr("Press a shortcut to investigate"), "heading"));
        let capture_bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let recorded = label(tr("Choose Record Shortcut to inspect a key combination"), "dim-label");
        recorded.set_hexpand(true);
        let press = gtk::Button::builder().label(tr("Record Shortcut")).valign(gtk::Align::Center).css_classes(["suggested-action"]).build();
        let guide = gtk::Button::builder().label(tr("Cheat Sheet")).tooltip_text(tr("Open a list of your shortcuts to read at a glance")).valign(gtk::Align::Center).build();
        guide.connect_clicked(|b| crate::app::spawn_window_tool(b, &["shortcut-guide"]));
        capture_bar.append(&recorded);
        capture_bar.append(&guide);
        capture_bar.append(&press);
        intro.append(&capture_bar);
        body.append(&intro);

        let toolbar = gtk::Box::builder().spacing(8).css_classes(["detective-toolbar"]).build();
        let all = gtk::ToggleButton::builder().label(tr("All Shortcuts")).active(true).build();
        let conflicts_only = gtk::ToggleButton::builder().label(tr("Conflicts")).group(&all).build();
        let refresh = gtk::Button::builder().icon_name("view-refresh-symbolic").tooltip_text(tr("Refresh shortcuts")).build();
        toolbar.append(&all);
        toolbar.append(&conflicts_only);
        toolbar.append(&refresh);
        body.append(&toolbar);
        let search = gtk::SearchEntry::builder().placeholder_text(tr("Filter shortcuts by action, owner, or keys…")).hexpand(true).build();
        body.append(&search);

        let workspace = gtk::Box::builder().spacing(16).valign(gtk::Align::Start).build();
        let catalog = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).hexpand(true).css_classes(["detective-card"]).build();
        let count = label(tr("Registered shortcuts"), "heading");
        catalog.append(&count);
        let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::Single).css_classes(["detective-list"]).build();
        let list_scroll = gtk::ScrolledWindow::builder().child(&list)
            .hscrollbar_policy(gtk::PolicyType::Never).min_content_height(280)
            .max_content_height(440).propagate_natural_height(true).build();
        catalog.append(&list_scroll);
        let detail = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(16).hexpand(true).css_classes(["detective-card"]).build();
        workspace.append(&catalog);
        workspace.append(&detail);
        let responsive = adw::BreakpointBin::builder().height_request(240).width_request(280).build();
        let narrow = adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 760sp").unwrap());
        narrow.add_setter(&workspace, "orientation", Some(&gtk::Orientation::Vertical.to_value()));
        responsive.add_breakpoint(narrow);
        body.append(&workspace);
        body.append(&label(tr("System-wide shortcuts · Select an action to inspect or change it"), "dim-label"));
        if let Some(keys) = keyboard_keys(toasts) {
            body.append(&keys);
        }
        let scroll = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&body).build();
        responsive.set_child(Some(&scroll));
        let root = responsive;
        let view = Rc::new(Self { root, search: search.clone(), list: list.clone(), detail, count, recorded,
            conflicts_only: conflicts_only.clone(), bindings: RefCell::default(), shown: RefCell::default(),
            looked_up: RefCell::default(), toasts: toasts.clone() });
        let weak = Rc::downgrade(&view);
        press.connect_clicked(move |button| {
            let Some(v) = weak.upgrade() else { return };
            v.recorded.set_label(tr("Press the keys now · Esc to cancel"));
            let weak = Rc::downgrade(&v);
            capture(button, move |accel| {
                if let Some(v) = weak.upgrade() {
                    if let Some(accel) = accel {
                        *v.looked_up.borrow_mut() = Some(accel);
                        v.show();
                    } else {
                        v.recorded.set_label(tr("Recording cancelled. Choose Record Shortcut to try again."));
                    }
                }
            });
        });
        let weak = Rc::downgrade(&view);
        search.connect_search_changed(move |_| { if let Some(v) = weak.upgrade() { v.show(); } });
        let weak = Rc::downgrade(&view);
        conflicts_only.connect_toggled(move |_| { if let Some(v) = weak.upgrade() { v.show(); } });
        let weak = Rc::downgrade(&view);
        refresh.connect_clicked(move |_| { if let Some(v) = weak.upgrade() { v.reload(); } });
        let weak = Rc::downgrade(&view);
        list.connect_row_selected(move |_, row| {
            if let (Some(v), Some(row)) = (weak.upgrade(), row) {
                let binding = v.shown.borrow().get(row.index() as usize).cloned();
                if let Some(b) = binding {
                    *v.looked_up.borrow_mut() = None;
                    let accel = {
                        let bindings = v.bindings.borrow();
                        b.accels.iter().find(|a| kb::who(a, &bindings).len() > 1).or_else(|| b.accels.first()).cloned()
                    };
                    v.recorded.set_label(&accel.as_deref().map(kb::label).unwrap_or_else(|| tr("No shortcut assigned").into()));
                    v.inspect(accel.as_deref(), Some(&b));
                }
            }
        });
        view
    }

    fn reload(self: &Rc<Self>) {
        *self.bindings.borrow_mut() = kb::all();
        self.show();
    }

    fn show(self: &Rc<Self>) {
        let selected = self.list.selected_row().and_then(|row| self.shown.borrow().get(row.index() as usize).cloned());
        while let Some(row) = self.list.first_child() { self.list.remove(&row); }
        let bindings = self.bindings.borrow();
        let conflicts = kb::conflicts(&bindings);
        self.conflicts_only.set_label(&trf!("Conflicts · {}", conflicts.len()));
        let filter = self.search.text().to_lowercase();
        let shown: Vec<Binding> = bindings.iter().filter(|b| {
            let matches = format!("{} {} {}", b.name, b.owner, b.accels.iter().map(|a| kb::label(a)).collect::<Vec<_>>().join(" ")).to_lowercase().contains(&filter);
            let conflict = b.accels.iter().any(|a| kb::who(a, &bindings).len() > 1);
            matches && (!self.conflicts_only.is_active() || conflict)
        }).cloned().collect();
        self.count.set_label(&trf!("Registered shortcuts · {} of {}", shown.len(), bindings.len()));
        *self.shown.borrow_mut() = shown.clone();
        for b in &shown {
            let row = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8).build();
            row.append(&label(&b.name, "heading"));
            row.append(&label(&b.owner, "dim-label"));
            let keys = gtk::Box::new(gtk::Orientation::Vertical, 6);
            if b.accels.is_empty() { keys.append(&label(tr("Not set"), "dim-label")); }
            for accel in &b.accels {
                let combo = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                combo.append(&keycaps(accel));
                if kb::who(accel, &bindings).len() > 1 { let badge = label(tr("Conflict"), "status-chip");
                    badge.set_valign(gtk::Align::Center);
                    combo.append(&badge); }
                keys.append(&combo);
            }
            row.append(&keys);
            self.list.append(&row);
        }
        if shown.is_empty() {
            self.list.append(&label(tr("No shortcuts match these filters"), "dim-label"));
        }
        drop(bindings);
        let looked_up = self.looked_up.borrow().clone();
        if let Some(accel) = looked_up {
            if let Some(index) = shown.iter().position(|b| b.accels.contains(&accel)) {
                self.list.select_row(self.list.row_at_index(index as i32).as_ref());
                return;
            }
            self.recorded.set_label(&kb::label(&accel));
            self.inspect(Some(&accel), None);
        } else if !shown.is_empty() {
            let index = selected.and_then(|old| shown.iter().position(|b| b.schema == old.schema && b.key == old.key && b.path == old.path)).unwrap_or(0);
            self.list.select_row(self.list.row_at_index(index as i32).as_ref());
        } else {
            self.inspect(None, None);
        }
    }

    fn inspect(self: &Rc<Self>, accel: Option<&str>, binding: Option<&Binding>) {
        while let Some(child) = self.detail.first_child() { self.detail.remove(&child); }
        self.detail.append(&label(tr("Shortcut details"), "title-2"));
        let bindings = self.bindings.borrow();
        if let Some(accel) = accel {
            self.detail.append(&keycaps(accel));
            let users = kb::who(accel, &bindings);
            let message = match users.len() {
                0 => tr("Available as a system-wide shortcut. Apps may still use it within their windows.").to_owned(),
                1 => tr("1 action uses this shortcut").to_owned(),
                n => trf!("{} actions share this shortcut. Change an action to resolve the conflict.", n),
            };
            self.detail.append(&label(&message, if users.len() > 1 { "warning" } else { "dim-label" }));
            for b in users {
                let group = adw::PreferencesGroup::new();
                group.add(&self.binding_row(b, Some(accel)));
                self.detail.append(&group);
            }
            self.detail.append(&label(tr("Free combinations to try"), "heading"));
            let suggestions = gtk::FlowBox::builder().selection_mode(gtk::SelectionMode::None).column_spacing(6).row_spacing(6).max_children_per_line(2).build();
            for suggestion in kb::suggest(accel, &bindings, 4) {
                suggestions.insert(&keycaps(&suggestion), -1);
            }
            self.detail.append(&suggestions);
            self.detail.append(&label(tr("Choose Change to record a new combination, turn a shortcut off, or reset its default."), "dim-label"));
        } else if let Some(b) = binding {
            let group = adw::PreferencesGroup::new();
            group.add(&self.binding_row(b, None));
            self.detail.append(&group);
        } else {
            self.detail.append(&label(tr("Select a shortcut or record a key combination to see its owners and available alternatives."), "dim-label"));
        }
    }

    /// A shortcut with its keys and a Change button. `focus` is the
    /// combination this row was found by, which is the one that changes.
    fn binding_row(self: &Rc<Self>, b: &Binding, focus: Option<&str>) -> adw::ActionRow {
        let keys = if b.accels.is_empty() { tr("Not set").to_owned() } else { b.accels.iter().map(|a| kb::label(a)).collect::<Vec<_>>().join(tr("  or  ")) };
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&b.name))
            .subtitle(glib::markup_escape_text(&if focus.is_some() { format!("{} \u{00b7} {keys}", b.owner) } else { keys }))
            .build();
        if b.writable {
            let change = gtk::Button::builder().label(tr("Change…")).valign(gtk::Align::Center).build();
            let (weak, b, old) = (Rc::downgrade(self), b.clone(), focus.map(str::to_owned).or_else(|| b.accels.first().cloned()));
            change.connect_clicked(move |button| {
                if let Some(v) = weak.upgrade() {
                    v.change_dialog(button, b.clone(), old.clone());
                }
            });
            row.add_suffix(&change);
        } else {
            row.add_suffix(&gtk::Image::builder().icon_name("changes-prevent-symbolic").tooltip_text(tr("Locked by the system administrator")).build());
        }
        row
    }

    fn change_dialog(self: &Rc<Self>, parent: &gtk::Button, b: Binding, old: Option<String>) {
        let bindings = self.bindings.borrow().clone();
        let chosen: Rc<RefCell<Option<String>>> = Rc::default();

        let current = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&b.name))
            .subtitle(glib::markup_escape_text(&trf!("{} · now {}", b.owner, old.as_deref().map(kb::label).unwrap_or_else(|| tr("not set").into()))))
            .build();
        let press = gtk::Button::builder().label(tr("Press the New Shortcut…")).css_classes(["pill", "suggested-action"]).halign(gtk::Align::Center).build();
        let status = gtk::Label::builder().wrap(true).justify(gtk::Justification::Center).visible(false).build();
        let suggestions = gtk::FlowBox::builder().selection_mode(gtk::SelectionMode::None).max_children_per_line(3).column_spacing(6).row_spacing(6).homogeneous(true).build();
        let save = gtk::Button::builder().label(tr("Save")).css_classes(["suggested-action"]).sensitive(false).build();

        // Shows the chosen combination and whether it's free.
        let choose: Rc<dyn Fn(String)> = {
            let (chosen, status, save, bindings, b) = (chosen.clone(), status.clone(), save.clone(), bindings.clone(), b.clone());
            Rc::new(move |accel: String| {
                let others: Vec<String> = kb::who(&accel, &bindings).into_iter().filter(|o| o.schema != b.schema || o.key != b.key || o.path != b.path).map(|o| format!("{} ({})", o.name, o.owner)).collect();
                status.set_visible(true);
                if others.is_empty() {
                    status.set_label(&trf!("{} is free", kb::label(&accel)));
                    status.set_css_classes(&["success"]);
                    save.set_sensitive(true);
                } else {
                    status.set_label(&trf!("{} is already used by {}. Pick another, or change that one first.", kb::label(&accel), others.join(", ")));
                    status.set_css_classes(&["warning"]);
                    save.set_sensitive(false);
                }
                *chosen.borrow_mut() = Some(accel);
            })
        };
        for accel in kb::suggest(old.as_deref().unwrap_or("<Super><Shift>a"), &bindings, SUGGESTIONS) {
            let button = gtk::Button::builder().label(kb::label(&accel)).build();
            let choose = choose.clone();
            button.connect_clicked(move |_| choose(accel.clone()));
            suggestions.insert(&button, -1);
        }
        {
            let choose = choose.clone();
            press.connect_clicked(move |button| {
                let choose = choose.clone();
                button.set_label(tr("Press the keys now (Esc to cancel)"));
                let button2 = button.clone();
                capture(button, move |accel| {
                    button2.set_label(tr("Press the New Shortcut…"));
                    if let Some(accel) = accel {
                        choose(accel);
                    }
                });
            });
        }

        let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(18).margin_top(12).margin_bottom(24).margin_start(24).margin_end(24).build();
        let current_list = gtk::ListBox::builder().css_classes(["boxed-list"]).selection_mode(gtk::SelectionMode::None).build();
        current_list.append(&current);
        content.append(&current_list);
        content.append(&press);
        content.append(&status);
        content.append(&gtk::Label::builder().label(tr("Free combinations")).css_classes(["heading"]).xalign(0.0).build());
        content.append(&suggestions);

        let disable = gtk::Button::builder().label(tr("Turn Off")).css_classes(["destructive-action"]).build();
        let reset = gtk::Button::builder().label(tr("Reset to Default")).visible(b.path.is_none() && !b.is_default()).build();
        let actions = gtk::Box::builder().spacing(12).halign(gtk::Align::End).build();
        actions.append(&disable);
        actions.append(&reset);
        actions.append(&save);
        content.append(&actions);

        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&content));
        let dialog = adw::Dialog::builder().title(tr("Change Shortcut")).content_width(460).child(&view).build();

        let finish: Rc<Finish> = {
            let (weak, dialog) = (Rc::downgrade(self), dialog.clone());
            Rc::new(move |result, done| {
                let Some(v) = weak.upgrade() else { return };
                match result {
                    Ok(()) => {
                        toast(&v.toasts, done);
                        dialog.close();
                        v.reload();
                    }
                    Err(e) => toast(&v.toasts, trf!("Not changed: {}", e)),
                }
            })
        };
        {
            let (b, old, chosen, finish) = (b.clone(), old.clone(), chosen.clone(), finish.clone());
            save.connect_clicked(move |_| {
                let Some(new) = chosen.borrow().clone() else { return };
                finish(kb::change(&b, old.as_deref(), Some(&new)), &trf!("“{}” is now {}", b.name, kb::label(&new)));
            });
        }
        {
            let (b, old, finish) = (b.clone(), old.clone(), finish.clone());
            disable.connect_clicked(move |_| finish(kb::change(&b, old.as_deref(), None), &trf!("“{}” has no shortcut now", b.name)));
        }
        {
            let (b, finish) = (b.clone(), finish.clone());
            reset.connect_clicked(move |_| finish(kb::reset(&b), &trf!("“{}” is back to its default", b.name)));
        }
        dialog.present(Some(parent));
    }
}

/// Waits for the next key combination pressed in `widget`'s window and
/// passes it on (None if Esc was pressed). While waiting, the desktop's own
/// shortcuts are paused, so pressing one of them reaches Voltkit instead of
/// running; GNOME asks once for permission to do that.
fn capture(widget: &impl IsA<gtk::Widget>, done: impl Fn(Option<String>) + 'static) {
    let Some(root) = widget.root() else { return };
    let toplevel = root.native().and_then(|n| n.surface()).and_downcast::<gdk::Toplevel>();
    if let Some(t) = &toplevel {
        t.inhibit_system_shortcuts(None::<&gdk::Event>);
    }
    let keys = gtk::EventControllerKey::builder().propagation_phase(gtk::PropagationPhase::Capture).build();
    let done = Rc::new(RefCell::new(Some(done)));
    let finished = Rc::new(Cell::new(false));
    {
        let (root, toplevel, done, finished) = (root.clone(), toplevel.clone(), done.clone(), finished.clone());
        keys.connect_key_pressed(move |controller, key, _, state| {
            if finished.get() || kb::is_modifier(key) {
                return glib::Propagation::Stop;
            }
            let result = if key == gdk::Key::Escape && state.is_empty() {
                None
            } else {
                match kb::from_press(key, state) {
                    Some(c) => Some(kb::accel_name(c)),
                    // A plain letter: keep waiting for a real combination.
                    None => return glib::Propagation::Stop,
                }
            };
            finished.set(true);
            if let Some(t) = &toplevel {
                t.restore_system_shortcuts();
            }
            root.remove_controller(controller);
            if let Some(done) = done.borrow_mut().take() {
                done(result);
            }
            glib::Propagation::Stop
        });
    }
    root.add_controller(keys);
}
