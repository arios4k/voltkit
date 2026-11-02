//! The Shortcut Detective page: look up a shortcut by pressing it, see
//! clashes, browse every system-wide shortcut, and change them.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, glib};

use crate::app::toast;
use crate::modules::keybindings::{self as kb, Binding};

const SUGGESTIONS: usize = 6;

/// Reports a change's result and closes the dialog when it worked.
type Finish = dyn Fn(anyhow::Result<()>, &str);

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = adw::PreferencesPage::new();
    if !kb::supported() {
        let group = adw::PreferencesGroup::builder()
            .title("Shortcut Detective")
            .description(
                "This desktop keeps its keyboard shortcuts somewhere Voltkit can't read yet. Shortcut Detective \
                 works on GNOME, Cinnamon, Budgie and MATE; on KDE Plasma, use System Settings \u{2192} Shortcuts.",
            )
            .build();
        page.add(&group);
        return page.upcast();
    }
    let view = View::new(toasts);
    for g in [&view.intro, &view.lookup, &view.conflicts, &view.all] {
        page.add(g);
    }
    if std::env::var_os("VOLTKIT_SHORTCUTS_DEMO").is_some() {
        *view.looked_up.borrow_mut() = Some("<Super>v".into());
    }
    view.reload();
    page.connect_destroy(move |_| {
        let _ = &view;
    });
    page.upcast()
}

struct View {
    intro: adw::PreferencesGroup,
    search: gtk::SearchEntry,
    lookup: adw::PreferencesGroup,
    conflicts: adw::PreferencesGroup,
    all: adw::PreferencesGroup,
    rows: RefCell<Vec<(adw::PreferencesGroup, gtk::Widget)>>,
    bindings: RefCell<Vec<Binding>>,
    /// The combination being looked up, as GSettings writes it.
    looked_up: RefCell<Option<String>>,
    toasts: adw::ToastOverlay,
}

impl View {
    fn new(toasts: &adw::ToastOverlay) -> Rc<Self> {
        let press = gtk::Button::builder()
            .child(&adw::ButtonContent::builder().icon_name("input-keyboard-symbolic").label("Press a Shortcut…").build())
            .valign(gtk::Align::Center)
            .css_classes(["suggested-action"])
            .build();
        let intro = adw::PreferencesGroup::builder()
            .title("Shortcut Detective")
            .description(
                "Find out what a keyboard shortcut does, fix shortcuts that clash, and change them. Covers the \
                 shortcuts that work everywhere (not the ones inside each app, like Ctrl+C).",
            )
            .header_suffix(&crate::app::spaced(&press))
            .build();
        let search = gtk::SearchEntry::builder().placeholder_text("Search by name, or type a shortcut like super+shift+s").build();
        intro.add(&search);
        let lookup = adw::PreferencesGroup::builder().visible(false).build();
        let conflicts = adw::PreferencesGroup::builder().title("Clashes").build();
        let all = adw::PreferencesGroup::builder().title("All Shortcuts").build();
        let view = Rc::new(Self {
            intro,
            search: search.clone(),
            lookup,
            conflicts,
            all,
            rows: RefCell::default(),
            bindings: RefCell::default(),
            looked_up: RefCell::default(),
            toasts: toasts.clone(),
        });
        let weak = Rc::downgrade(&view);
        press.connect_clicked(move |b| {
            let Some(v) = weak.upgrade() else { return };
            let weak = Rc::downgrade(&v);
            toast(&v.toasts, "Press the shortcut to look up (Esc to cancel)");
            capture(b, move |accel| {
                if let (Some(v), Some(accel)) = (weak.upgrade(), accel) {
                    v.search.set_text("");
                    *v.looked_up.borrow_mut() = Some(accel);
                    v.show();
                }
            });
        });
        let weak = Rc::downgrade(&view);
        search.connect_search_changed(move |s| {
            if let Some(v) = weak.upgrade() {
                *v.looked_up.borrow_mut() = kb::parse_typed(&s.text()).map(kb::accel_name);
                v.show();
            }
        });
        view
    }

    fn reload(self: &Rc<Self>) {
        *self.bindings.borrow_mut() = kb::all();
        self.show();
    }

    fn add(&self, group: &adw::PreferencesGroup, row: impl IsA<gtk::Widget>) {
        group.add(&row);
        self.rows.borrow_mut().push((group.clone(), row.upcast()));
    }

    fn show(self: &Rc<Self>) {
        for (group, row) in self.rows.take() {
            group.remove(&row);
        }
        let bindings = self.bindings.borrow().clone();

        // Lookup: who uses this combination?
        let looked_up = self.looked_up.borrow().clone();
        self.lookup.set_visible(looked_up.is_some());
        if let Some(accel) = &looked_up {
            let users = kb::who(accel, &bindings);
            self.lookup.set_title(&glib::markup_escape_text(&format!("Who Uses {}", kb::label(accel))));
            if users.is_empty() {
                let row = adw::ActionRow::builder().title("Nobody: it's free").subtitle("No system-wide shortcut uses this combination. Apps may still use it inside their own windows.").subtitle_lines(2).build();
                let icon = gtk::Image::from_icon_name("emblem-ok-symbolic");
                icon.add_css_class("success");
                row.add_prefix(&icon);
                self.add(&self.lookup, row);
            }
            for b in users {
                let row = self.binding_row(b, Some(accel));
                self.add(&self.lookup, row);
            }
        }

        // Clashes.
        let clashes = kb::conflicts(&bindings);
        self.conflicts.set_description(Some(if clashes.is_empty() {
            "No two actions share a shortcut."
        } else {
            "These combinations do more than one thing, so only one of them works. Change one of each pair."
        }));
        if clashes.is_empty() {
            let row = adw::ActionRow::builder().title("No clashes").build();
            let icon = gtk::Image::from_icon_name("emblem-ok-symbolic");
            icon.add_css_class("success");
            row.add_prefix(&icon);
            self.add(&self.conflicts, row);
        }
        for (combo, users) in &clashes {
            let accel = kb::accel_name(*combo);
            let names: Vec<String> = users.iter().map(|b| format!("{} ({})", b.name, b.owner)).collect();
            let expander = adw::ExpanderRow::builder()
                .title(glib::markup_escape_text(&kb::label(&accel)))
                .subtitle(glib::markup_escape_text(&names.join(", ")))
                .subtitle_lines(2)
                .build();
            let icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
            icon.add_css_class("warning");
            expander.add_prefix(&icon);
            for b in users {
                expander.add_row(&self.binding_row(b, Some(&accel)));
            }
            self.add(&self.conflicts, expander);
        }

        // Everything, grouped by owner, filtered by the search.
        let filter = self.search.text().to_lowercase();
        let filter = if looked_up.is_some() { String::new() } else { filter };
        let mut owners: Vec<(String, Vec<&Binding>)> = Vec::new();
        for b in &bindings {
            let labels: String = b.accels.iter().map(|a| kb::label(a)).collect::<Vec<_>>().join(" ").to_lowercase();
            if !filter.is_empty() && !b.name.to_lowercase().contains(&filter) && !b.owner.to_lowercase().contains(&filter) && !labels.contains(&filter) {
                continue;
            }
            match owners.iter_mut().find(|(o, _)| *o == b.owner) {
                Some((_, list)) => list.push(b),
                None => owners.push((b.owner.clone(), vec![b])),
            }
        }
        if owners.is_empty() {
            self.add(&self.all, adw::ActionRow::builder().title("No shortcuts match").build());
        }
        for (owner, list) in owners {
            let set = list.iter().filter(|b| !b.accels.is_empty()).count();
            let expander = adw::ExpanderRow::builder()
                .title(glib::markup_escape_text(&owner))
                .subtitle(format!("{set} set, {} not set", list.len() - set))
                .expanded(!filter.is_empty())
                .build();
            for b in list {
                expander.add_row(&self.binding_row(b, None));
            }
            self.add(&self.all, expander);
        }
    }

    /// A shortcut with its keys and a Change button. `focus` is the
    /// combination this row was found by, which is the one that changes.
    fn binding_row(self: &Rc<Self>, b: &Binding, focus: Option<&str>) -> adw::ActionRow {
        let keys = if b.accels.is_empty() { "Not set".to_owned() } else { b.accels.iter().map(|a| kb::label(a)).collect::<Vec<_>>().join("  or  ") };
        let row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&b.name))
            .subtitle(glib::markup_escape_text(&if focus.is_some() { format!("{} \u{00b7} {keys}", b.owner) } else { keys }))
            .build();
        if b.writable {
            let change = gtk::Button::builder().label("Change…").valign(gtk::Align::Center).build();
            let (weak, b, old) = (Rc::downgrade(self), b.clone(), focus.map(str::to_owned).or_else(|| b.accels.first().cloned()));
            change.connect_clicked(move |button| {
                if let Some(v) = weak.upgrade() {
                    v.change_dialog(button, b.clone(), old.clone());
                }
            });
            row.add_suffix(&change);
        } else {
            row.add_suffix(&gtk::Image::builder().icon_name("changes-prevent-symbolic").tooltip_text("Locked by the system administrator").build());
        }
        row
    }

    fn change_dialog(self: &Rc<Self>, parent: &gtk::Button, b: Binding, old: Option<String>) {
        let bindings = self.bindings.borrow().clone();
        let chosen: Rc<RefCell<Option<String>>> = Rc::default();

        let current = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&b.name))
            .subtitle(glib::markup_escape_text(&format!(
                "{} \u{00b7} now {}",
                b.owner,
                old.as_deref().map(kb::label).unwrap_or_else(|| "not set".into())
            )))
            .build();
        let press = gtk::Button::builder().label("Press the New Shortcut…").css_classes(["pill", "suggested-action"]).halign(gtk::Align::Center).build();
        let status = gtk::Label::builder().wrap(true).justify(gtk::Justification::Center).visible(false).build();
        let suggestions = gtk::FlowBox::builder().selection_mode(gtk::SelectionMode::None).max_children_per_line(3).column_spacing(6).row_spacing(6).homogeneous(true).build();
        let save = gtk::Button::builder().label("Save").css_classes(["suggested-action"]).sensitive(false).build();

        // Shows the chosen combination and whether it's free.
        let choose: Rc<dyn Fn(String)> = {
            let (chosen, status, save, bindings, b) = (chosen.clone(), status.clone(), save.clone(), bindings.clone(), b.clone());
            Rc::new(move |accel: String| {
                let others: Vec<String> = kb::who(&accel, &bindings).into_iter().filter(|o| o.schema != b.schema || o.key != b.key || o.path != b.path).map(|o| format!("{} ({})", o.name, o.owner)).collect();
                status.set_visible(true);
                if others.is_empty() {
                    status.set_label(&format!("{} is free", kb::label(&accel)));
                    status.set_css_classes(&["success"]);
                    save.set_sensitive(true);
                } else {
                    status.set_label(&format!("{} is already used by {}. Pick another, or change that one first.", kb::label(&accel), others.join(", ")));
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
                button.set_label("Press the keys now (Esc to cancel)");
                let button2 = button.clone();
                capture(button, move |accel| {
                    button2.set_label("Press the New Shortcut…");
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
        content.append(&gtk::Label::builder().label("Free combinations").css_classes(["heading"]).xalign(0.0).build());
        content.append(&suggestions);

        let disable = gtk::Button::builder().label("Turn Off").css_classes(["destructive-action"]).build();
        let reset = gtk::Button::builder().label("Reset to Default").visible(b.path.is_none() && !b.is_default()).build();
        let actions = gtk::Box::builder().spacing(12).halign(gtk::Align::End).build();
        actions.append(&disable);
        actions.append(&reset);
        actions.append(&save);
        content.append(&actions);

        let view = adw::ToolbarView::new();
        view.add_top_bar(&adw::HeaderBar::new());
        view.set_content(Some(&content));
        let dialog = adw::Dialog::builder().title("Change Shortcut").content_width(460).child(&view).build();

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
                    Err(e) => toast(&v.toasts, format!("Not changed: {e}")),
                }
            })
        };
        {
            let (b, old, chosen, finish) = (b.clone(), old.clone(), chosen.clone(), finish.clone());
            save.connect_clicked(move |_| {
                let Some(new) = chosen.borrow().clone() else { return };
                finish(kb::change(&b, old.as_deref(), Some(&new)), &format!("“{}” is now {}", b.name, kb::label(&new)));
            });
        }
        {
            let (b, old, finish) = (b.clone(), old.clone(), finish.clone());
            disable.connect_clicked(move |_| finish(kb::change(&b, old.as_deref(), None), &format!("“{}” has no shortcut now", b.name)));
        }
        {
            let (b, finish) = (b.clone(), finish.clone());
            reset.connect_clicked(move |_| finish(kb::reset(&b), &format!("“{}” is back to its default", b.name)));
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
