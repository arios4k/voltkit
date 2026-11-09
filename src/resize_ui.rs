//! The Image Resizer page: drop pictures, choose a size, get resized copies beside the originals.

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

    // ---- Drop zone and results
    let choose = gtk::Button::builder().label(tr("Choose Pictures…")).halign(gtk::Align::Center).css_classes(["pill", "suggested-action"]).margin_top(6).margin_bottom(24).build();
    let drop_zone = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(12).css_classes(["card"]).build();
    drop_zone.append(&gtk::Image::builder().icon_name("image-x-generic-symbolic").pixel_size(64).margin_top(24).css_classes(["dim-label"]).accessible_role(gtk::AccessibleRole::Presentation).build());
    drop_zone.append(&gtk::Label::builder().label(tr("Drop Pictures Here")).css_classes(["title-2"]).build());
    drop_zone.append(
        &gtk::Label::builder()
            .label(tr("PNG, JPEG, WebP, GIF, BMP and TIFF. The copies are saved beside the originals, which are never changed."))
            .wrap(true)
            .justify(gtk::Justification::Center)
            .margin_start(12)
            .margin_end(12)
            .css_classes(["dim-label"])
            .build(),
    );
    drop_zone.append(&choose);

    let results = adw::PreferencesGroup::builder().title(tr("Results")).description(tr("Choose or drop pictures to see the new sizes here.")).name("resize-results").build();

    let process: Rc<dyn Fn(PathBuf)> = {
        let (options, results, toasts) = (options.clone(), results.clone(), toasts.clone());
        Rc::new(move |path: PathBuf| {
            let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let row = adw::ActionRow::builder().title(glib::markup_escape_text(&name)).subtitle(tr("Resizing…")).subtitle_lines(2).build();
            let spinner = gtk::Spinner::builder().spinning(true).build();
            row.add_suffix(&spinner);
            results.add(&row);
            let opts = options();
            let toasts = toasts.clone();
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || resize::resize(&path, &opts).map_err(|e| format!("{e:#}"))).await;
                row.remove(&spinner);
                match result {
                    Ok(Ok(r)) => {
                        let out = r.output.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        row.set_subtitle(&glib::markup_escape_text(&format!("{}×{} → {}×{} · {} → {}\n{out}", r.from.0, r.from.1, r.to.0, r.to.1, size_text(r.bytes_before), size_text(r.bytes_after))));
                        row.add_prefix(&gtk::Image::builder().icon_name("emblem-ok-symbolic").css_classes(["success"]).accessible_role(gtk::AccessibleRole::Presentation).build());
                    }
                    Ok(Err(e)) => {
                        row.set_subtitle(tr("Failed"));
                        toast(&toasts, format!("{name}: {e}"));
                    }
                    Err(_) => row.set_subtitle(tr("Failed")),
                }
            });
        })
    };

    let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
    {
        let process = process.clone();
        target.connect_drop(move |_, value, _, _| {
            let Ok(list) = value.get::<gdk::FileList>() else { return false };
            for path in list.files().iter().filter_map(|f| f.path()).filter(|p| resize::is_picture(p)) {
                process(path);
            }
            true
        });
    }
    drop_zone.add_controller(target);
    // Made-up pictures for the manual's screenshots.
    if std::env::var_os("VOLTKIT_RESIZE_DEMO").is_some() {
        let dir = std::env::temp_dir().join(format!("voltkit-resize-demo-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        for (name, w, h, color) in [("beach.jpg", 3200, 2400, 0x4f9fd8ffu32), ("mountain.png", 2400, 1600, 0x5b8a5bff), ("portrait.jpg", 1800, 2700, 0xd8884fff)] {
            if let Some(pb) = gtk::gdk_pixbuf::Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, false, 8, w, h) {
                pb.fill(color);
                let path = dir.join(name);
                let _ = pb.savev(&path, if name.ends_with("png") { "png" } else { "jpeg" }, &[]);
                process(path);
            }
        }
    }
    choose.connect_clicked(move |b| {
        let process = process.clone();
        let parent = b.root().and_downcast::<gtk::Window>();
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(tr("Pictures")));
        for pattern in ["*.png", "*.jpg", "*.jpeg", "*.webp", "*.gif", "*.bmp", "*.tif", "*.tiff"] {
            filter.add_pattern(pattern);
        }
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        gtk::FileDialog::builder().title(tr("Resize Pictures")).filters(&filters).build().open_multiple(parent.as_ref(), gio::Cancellable::NONE, move |res| {
            let Ok(model) = res else { return };
            for i in 0..model.n_items() {
                if let Some(path) = model.item(i).and_downcast::<gio::File>().and_then(|f| f.path()) {
                    process(path);
                }
            }
        });
    });

    let page = crate::workspace::Workspace::new();
    page.add(&drop_zone);
    page.add(&results);
    page.aside(&size_group);
    page.aside(&output);
    page.aside(&crate::workspace::note(
        tr("Your originals stay intact"),
        tr("Each copy is saved beside its original as name (WxH).ext. A picture's own rotation is applied; other metadata, such as the place a photo was taken, is not copied."),
        "document-save-symbolic",
    ));
    page.upcast()
}
