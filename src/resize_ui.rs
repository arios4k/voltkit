//! The Image Resizer page: drop pictures, choose a size, get resized copies beside the originals.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::app::toast;
use crate::bulk_cli::size_text;
use crate::i18n::tr;
use crate::modules::resize::{self, Format, Mode, Options, Size};

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    // ---- Size
    let size_group = adw::PreferencesGroup::builder().title(tr("Size")).build();
    let kind = adw::ComboRow::builder()
        .title(tr("Make the pictures"))
        .model(&gtk::StringList::new(&[
            tr("Small: 640 px on the longest side"),
            tr("Medium: 1280 px on the longest side"),
            tr("Large: 1920 px on the longest side"),
            tr("Very large: 3840 px on the longest side"),
            tr("A share of their size"),
            tr("A width and height"),
        ]))
        .selected(2)
        .build();
    let percent = adw::SpinRow::with_range(1.0, 500.0, 1.0);
    percent.set_title(tr("Percent of the original"));
    percent.set_value(50.0);
    let width = adw::SpinRow::with_range(0.0, 20000.0, 10.0);
    width.set_title(tr("Width (px)"));
    width.set_subtitle(tr("0 leaves the width open"));
    width.set_value(1280.0);
    let height = adw::SpinRow::with_range(0.0, 20000.0, 10.0);
    height.set_title(tr("Height (px)"));
    height.set_subtitle(tr("0 leaves the height open"));
    height.set_value(720.0);
    let mode = adw::ComboRow::builder()
        .title(tr("When the shape is different"))
        .model(&gtk::StringList::new(&[tr("Fit inside the size"), tr("Fill the size and trim the edges"), tr("Stretch to exactly that size")]))
        .build();
    let enlarge = adw::SwitchRow::builder().title(tr("Allow enlarging")).subtitle(tr("Off: a picture smaller than the size is left at its size")).build();
    for w in [kind.upcast_ref::<gtk::Widget>(), percent.upcast_ref(), width.upcast_ref(), height.upcast_ref(), mode.upcast_ref(), enlarge.upcast_ref()] {
        size_group.add(w);
    }

    // ---- File type
    let output = adw::PreferencesGroup::builder().title(tr("Output")).build();
    let format = adw::ComboRow::builder()
        .title(tr("Format"))
        .model(&gtk::StringList::new(&[tr("Same as the original"), "PNG", "JPEG", "WebP"]))
        .build();
    let quality = adw::SpinRow::with_range(1.0, 100.0, 1.0);
    quality.set_title(tr("Quality"));
    quality.set_subtitle(tr("For JPEG and WebP. PNG is always lossless."));
    quality.set_value(90.0);
    output.add(&format);
    output.add(&quality);

    // Only the controls the choice needs.
    let sync = {
        let (kind, percent, width, height, mode) = (kind.clone(), percent.clone(), width.clone(), height.clone(), mode.clone());
        Rc::new(move || {
            let k = kind.selected();
            percent.set_visible(k == 4);
            width.set_visible(k == 5);
            height.set_visible(k == 5);
            mode.set_visible(k == 5);
        })
    };
    sync();
    {
        let sync = sync.clone();
        kind.connect_selected_notify(move |_| sync());
    }

    let options = {
        let (kind, percent, width, height, mode, enlarge, format, quality) =
            (kind.clone(), percent.clone(), width.clone(), height.clone(), mode.clone(), enlarge.clone(), format.clone(), quality.clone());
        Rc::new(move || Options {
            size: match kind.selected() {
                0 => Size::LongestSide(640),
                1 => Size::LongestSide(1280),
                3 => Size::LongestSide(3840),
                4 => Size::Percent(percent.value() as u32),
                5 => Size::Box { width: Some(width.value() as u32).filter(|w| *w > 0), height: Some(height.value() as u32).filter(|h| *h > 0) },
                _ => Size::LongestSide(1920),
            },
            mode: match mode.selected() {
                1 => Mode::Fill,
                2 => Mode::Stretch,
                _ => Mode::Fit,
            },
            no_enlarge: !enlarge.is_active(),
            format: match format.selected() {
                1 => Format::Png,
                2 => Format::Jpeg,
                3 => Format::WebP,
                _ => Format::Same,
            },
            quality: quality.value() as u8,
        })
    };

    // ---- Pictures, and the button that resizes them
    let choose = gtk::Button::builder().label(tr("Choose Pictures…")).halign(gtk::Align::Center).css_classes(["pill"]).margin_top(6).margin_bottom(24).build();
    let drop_zone = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).css_classes(["card"]).build();
    drop_zone.append(&gtk::Image::builder().icon_name("image-x-generic-symbolic").pixel_size(64).margin_top(24).css_classes(["dim-label"]).accessible_role(gtk::AccessibleRole::Presentation).build());
    drop_zone.append(&gtk::Label::builder().label(tr("Drop Pictures Here")).css_classes(["title-2"]).build());
    drop_zone.append(
        &gtk::Label::builder()
            .label(tr("PNG, JPEG, WebP, GIF, BMP and TIFF. Add the pictures, choose the size, then click Resize. The copies are saved beside the originals, which are never changed."))
            .wrap(true)
            .justify(gtk::Justification::Center)
            .margin_start(12)
            .margin_end(12)
            .css_classes(["dim-label"])
            .build(),
    );
    drop_zone.append(&choose);

    let list = adw::PreferencesGroup::builder().title(tr("Pictures")).description(tr("Nothing added yet. Drop pictures above or choose them.")).name("resize-results").build();
    let resize_button = gtk::Button::builder().label(tr("Resize")).css_classes(["suggested-action", "pill"]).sensitive(false).halign(gtk::Align::End).build();
    let clear = gtk::Button::builder().label(tr("Clear")).sensitive(false).build();
    let actions = gtk::Box::builder().spacing(8).halign(gtk::Align::End).margin_top(8).build();
    actions.append(&clear);
    actions.append(&resize_button);

    struct Item {
        path: PathBuf,
        row: adw::ActionRow,
        done: bool,
    }
    let items: Rc<RefCell<Vec<Item>>> = Rc::default();
    let running = Rc::new(Cell::new(false));

    // The button says what it will do, and the list says what is in it.
    let refresh: Rc<dyn Fn()> = {
        let (items, list, resize_button, clear, running) = (items.clone(), list.clone(), resize_button.clone(), clear.clone(), running.clone());
        Rc::new(move || {
            let items = items.borrow();
            let pending = items.iter().filter(|i| !i.done).count();
            clear.set_sensitive(!items.is_empty() && !running.get());
            resize_button.set_sensitive(!items.is_empty() && !running.get());
            if items.is_empty() {
                list.set_description(Some(tr("Nothing added yet. Drop pictures above or choose them.")));
                resize_button.set_label(tr("Resize"));
                resize_button.set_tooltip_text(None);
            } else if pending > 0 {
                list.set_description(Some(tr("Choose the size on the right, then click Resize.")));
                resize_button.set_label(&trnf!("Resize {} Picture", "Resize {} Pictures", pending));
                resize_button.set_tooltip_text(None);
            } else {
                list.set_description(Some(tr("Done. Change the size and click Resize Again for new copies.")));
                resize_button.set_label(tr("Resize Again"));
                resize_button.set_tooltip_text(Some(tr("Makes new copies with the current settings")));
            }
        })
    };

    let add: Rc<dyn Fn(Vec<PathBuf>)> = {
        let (items, list, refresh) = (items.clone(), list.clone(), refresh.clone());
        Rc::new(move |paths: Vec<PathBuf>| {
            for path in paths.into_iter().filter(|p| resize::is_picture(p)) {
                if items.borrow().iter().any(|i| i.path == path) {
                    continue;
                }
                let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let size = gtk::gdk_pixbuf::Pixbuf::file_info(&path).map(|(_, w, h)| format!("{w}×{h}")).unwrap_or_default();
                let bytes = std::fs::metadata(&path).map(|m| size_text(m.len())).unwrap_or_default();
                let row = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&name))
                    .subtitle(glib::markup_escape_text(&format!("{size} · {bytes}")))
                    .subtitle_lines(2)
                    .tooltip_text(path.display().to_string())
                    .build();
                row.add_prefix(&gtk::Image::builder().icon_name("image-x-generic-symbolic").accessible_role(gtk::AccessibleRole::Presentation).build());
                let remove = gtk::Button::builder().icon_name("user-trash-symbolic").valign(gtk::Align::Center).css_classes(["flat"]).tooltip_text(tr("Remove")).build();
                remove.update_property(&[gtk::accessible::Property::Label(&crate::i18n::fill(tr("Remove {}"), &[&name]))]);
                row.add_suffix(&remove);
                list.add(&row);
                {
                    let (items, list, refresh, row, path) = (items.clone(), list.clone(), refresh.clone(), row.clone(), path.clone());
                    remove.connect_clicked(move |_| {
                        list.remove(&row);
                        items.borrow_mut().retain(|i| i.path != path);
                        refresh();
                    });
                }
                items.borrow_mut().push(Item { path, row, done: false });
            }
            refresh();
        })
    };

    {
        let (items, list, refresh) = (items.clone(), list.clone(), refresh.clone());
        clear.connect_clicked(move |_| {
            for item in items.borrow_mut().drain(..) {
                list.remove(&item.row);
            }
            refresh();
        });
    }

    // Resize everything that has not been done, or everything again when all is done.
    {
        let (items, options, running, refresh, toasts, resize_button) = (items.clone(), options.clone(), running.clone(), refresh.clone(), toasts.clone(), resize_button.clone());
        resize_button.connect_clicked(move |_| {
            if items.borrow().iter().all(|i| i.done) {
                for i in items.borrow_mut().iter_mut() {
                    i.done = false;
                }
            }
            let jobs: Vec<(PathBuf, adw::ActionRow)> = items.borrow().iter().filter(|i| !i.done).map(|i| (i.path.clone(), i.row.clone())).collect();
            if jobs.is_empty() {
                return;
            }
            running.set(true);
            refresh();
            let opts = options();
            let (items, running, refresh, toasts) = (items.clone(), running.clone(), refresh.clone(), toasts.clone());
            glib::spawn_future_local(async move {
                let (mut worked, mut failed) = (0usize, 0usize);
                for (path, row) in jobs {
                    let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                    row.set_subtitle(tr("Resizing…"));
                    let spinner = gtk::Spinner::builder().spinning(true).build();
                    row.add_suffix(&spinner);
                    let source = path.clone();
                    let result = gio::spawn_blocking(move || resize::resize(&source, &opts).map_err(|e| format!("{e:#}"))).await;
                    row.remove(&spinner);
                    match result {
                        Ok(Ok(r)) => {
                            worked += 1;
                            let out = r.output.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            row.set_subtitle(&glib::markup_escape_text(&format!("{}×{} → {}×{} · {} → {}\n{out}", r.from.0, r.from.1, r.to.0, r.to.1, size_text(r.bytes_before), size_text(r.bytes_after))));
                            row.set_tooltip_text(Some(&r.output.display().to_string()));
                            if let Some(item) = items.borrow_mut().iter_mut().find(|i| i.path == path) {
                                item.done = true;
                            }
                        }
                        Ok(Err(e)) => {
                            failed += 1;
                            row.set_subtitle(&glib::markup_escape_text(&format!("{}: {e}", tr("Failed"))));
                        }
                        Err(_) => {
                            failed += 1;
                            row.set_subtitle(tr("Failed"));
                        }
                    }
                    let _ = name;
                }
                running.set(false);
                refresh();
                let message = if failed == 0 { trnf!("Resized {} picture. The copy is beside the original.", "Resized {} pictures. The copies are beside the originals.", worked) } else { trnf!("{} picture could not be resized", "{} pictures could not be resized", failed) };
                toast(&toasts, message);
            });
        });
    }

    let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    {
        let add = add.clone();
        target.connect_drop(move |_, value, _, _| {
            let Ok(files) = value.get::<gdk::FileList>() else { return false };
            add(files.files().iter().filter_map(|f| f.path()).collect());
            true
        });
    }
    drop_zone.add_controller(target);
    // Made-up pictures for the manual's screenshots, resized as if the button was clicked.
    if std::env::var_os("VOLTKIT_RESIZE_DEMO").is_some() {
        let dir = std::env::temp_dir().join(format!("voltkit-resize-demo-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut made = Vec::new();
        for (name, w, h, color) in [("beach.jpg", 3200, 2400, 0x4f9fd8ffu32), ("mountain.png", 2400, 1600, 0x5b8a5bff), ("portrait.jpg", 1800, 2700, 0xd8884fff)] {
            if let Some(pb) = gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, false, 8, w, h) {
                pb.fill(color);
                let path = dir.join(name);
                let _ = pb.savev(&path, if name.ends_with("png") { "png" } else { "jpeg" }, &[]);
                made.push(path);
            }
        }
        add(made);
        if std::env::var("VOLTKIT_RESIZE_DEMO").as_deref() == Ok("done") {
            let button = resize_button.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || button.emit_clicked());
        }
    }
    {
        let add = add.clone();
        choose.connect_clicked(move |b| {
            let add = add.clone();
            let parent = b.root().and_downcast::<gtk::Window>();
            let filter = gtk::FileFilter::new();
            filter.set_name(Some(tr("Pictures")));
            for pattern in ["*.png", "*.jpg", "*.jpeg", "*.webp", "*.gif", "*.bmp", "*.tif", "*.tiff"] {
                filter.add_pattern(pattern);
            }
            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&filter);
            gtk::FileDialog::builder().title(tr("Choose Pictures")).filters(&filters).build().open_multiple(parent.as_ref(), gio::Cancellable::NONE, move |res| {
                let Ok(model) = res else { return };
                add((0..model.n_items()).filter_map(|i| model.item(i).and_downcast::<gio::File>().and_then(|f| f.path())).collect());
            });
        });
    }

    // Ideas for what to choose, one click to try each.
    let ideas = adw::PreferencesGroup::builder()
        .title(tr("Which size should I use?"))
        .description(tr("Click an idea to set the size for you."))
        .build();
    type Apply = Box<dyn Fn()>;
    let set_box = {
        let (kind, width, height, mode) = (kind.clone(), width.clone(), height.clone(), mode.clone());
        move |w: f64, h: f64, m: u32| {
            kind.set_selected(5);
            width.set_value(w);
            height.set_value(h);
            mode.set_selected(m);
        }
    };
    let rows: Vec<(String, String, Apply)> = vec![
        (tr("Email or chat").to_owned(), tr("Medium: 1280 px on the longest side. Small enough to send, sharp on a screen.").to_owned(), Box::new({ let kind = kind.clone(); move || kind.set_selected(1) })),
        (tr("Website or blog").to_owned(), tr("Large: 1920 px on the longest side. Fills a full-width page.").to_owned(), Box::new({ let kind = kind.clone(); move || kind.set_selected(2) })),
        (tr("A post for social media").to_owned(), tr("A width of 1080 px, with the height left open (0).").to_owned(), Box::new({ let s = set_box.clone(); move || s(1080.0, 0.0, 0) })),
        (tr("A square thumbnail").to_owned(), tr("256 × 256 px, filling the square and trimming the edges.").to_owned(), Box::new({ let s = set_box.clone(); move || s(256.0, 256.0, 1) })),
        (tr("A phone wallpaper").to_owned(), tr("1080 × 2400 px, filling the screen and trimming the edges.").to_owned(), Box::new({ let s = set_box.clone(); move || s(1080.0, 2400.0, 1) })),
        (tr("A 4 × 6 inch print").to_owned(), tr("1800 × 1200 px (300 dots per inch), fitting inside.").to_owned(), Box::new({ let s = set_box.clone(); move || s(1800.0, 1200.0, 0) })),
        (tr("Half the size").to_owned(), tr("50 percent of the original, whatever its size.").to_owned(), Box::new({ let (kind, percent) = (kind.clone(), percent.clone()); move || { kind.set_selected(4); percent.set_value(50.0); } })),
    ];
    for (title, subtitle, apply) in rows {
        let row = adw::ActionRow::builder().title(title).subtitle(subtitle).subtitle_lines(2).activatable(true).build();
        row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        row.connect_activated(move |_| apply());
        ideas.add(&row);
    }

    let page = crate::workspace::Workspace::new();
    page.add(&drop_zone);
    list.add(&actions);
    page.add(&list);
    page.aside(&size_group);
    page.aside(&output);
    page.aside(&ideas);
    page.aside(&crate::workspace::note(
        tr("Your originals stay intact"),
        tr("Each copy is saved beside its original as name (WxH).ext. A picture's own rotation is applied; other metadata, such as the place a photo was taken, is not copied."),
        "document-save-symbolic",
    ));
    page.upcast()
}
