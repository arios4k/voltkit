//! The QR Code page: read codes from the screen, and create codes from text,
//! links or Wi-Fi details.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::{gdk, gio, glib};

use crate::app::{shortcut_label, toast};
use crate::config::Config;
use crate::modules::qr::{self, WifiSecurity};
use crate::portal;

/// On-screen size of the generated code, in logical pixels.
const PREVIEW_SIZE: usize = 260;
/// Pixels per module in saved PNGs: large enough to print.
const SAVE_SCALE: usize = 12;

pub fn page(config: &Config, toasts: &adw::ToastOverlay) -> gtk::Widget {
    let page = adw::PreferencesPage::new();
    page.add(&scan_group(config, toasts));
    page.add(&create_group(toasts));
    page.upcast()
}

fn scan_group(config: &Config, toasts: &adw::ToastOverlay) -> adw::PreferencesGroup {
    let scan = gtk::Button::builder()
        .child(&adw::ButtonContent::builder().icon_name("camera-photo-symbolic").label("Scan from Screen").build())
        .valign(gtk::Align::Center)
        .css_classes(["suggested-action"])
        .build();
    let group = adw::PreferencesGroup::builder()
        .title("Scan")
        .description(format!(
            "Drag a box around a QR code anywhere on your screen: a video, a slide, a web page. Shortcut: {}",
            shortcut_label(&config.hotkeys.qr)
        ))
        .header_suffix(&scan)
        .build();

    let result = adw::ActionRow::builder()
        .title("Result")
        .subtitle_selectable(true)
        .subtitle_lines(4)
        .visible(false)
        .build();
    let copy = flat_button("edit-copy-symbolic", "Copy");
    let open = flat_button("web-browser-symbolic", "Open Link");
    result.add_suffix(&open);
    result.add_suffix(&copy);
    group.add(&result);

    let text: Rc<RefCell<String>> = Rc::default();
    {
        let (text, toasts) = (text.clone(), toasts.clone());
        copy.connect_clicked(move |b| {
            b.clipboard().set_text(&text.borrow());
            toast(&toasts, "Copied");
        });
    }
    {
        let text = text.clone();
        open.connect_clicked(move |b| {
            let window = b.root().and_downcast::<gtk::Window>();
            gtk::UriLauncher::new(text.borrow().trim()).launch(window.as_ref(), gio::Cancellable::NONE, |_| {});
        });
    }

    let toasts = toasts.clone();
    scan.connect_clicked(move |button| {
        let (button, result, open, text, toasts) =
            (button.clone(), result.clone(), open.clone(), text.clone(), toasts.clone());
        // The window stays put: GNOME won't bring back a minimized window on its
        // own, so the result would land in a hidden window.
        button.set_sensitive(false);
        glib::spawn_future_local(async move {
            // Attach the screenshot dialogs to this window (the guard keeps it exported).
            let (parent, _guard) = portal::parent_handle(&button.native().expect("button in a window")).await;
            let scanned = gio::spawn_blocking(move || portal::block_on(qr::scan(parent.as_deref()))).await;
            button.set_sensitive(true);
            match scanned {
                Ok(Ok(codes)) => {
                    let found = codes.join("\n");
                    result.set_subtitle(&glib::markup_escape_text(&found));
                    result.set_visible(true);
                    open.set_visible(qr::is_link(&found));
                    button.clipboard().set_text(&found);
                    *text.borrow_mut() = found;
                    toast(&toasts, "QR code copied to clipboard");
                }
                Ok(Err(e)) if e.is::<portal::Cancelled>() => toast(&toasts, "No screenshot was taken"),
                Ok(Err(e)) => toast(&toasts, e.to_string()),
                Err(_) => toast(&toasts, "QR scanner crashed"),
            }
        });
    });
    group
}

fn create_group(toasts: &adw::ToastOverlay) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title("Create")
        .description("Type text or a link, or describe a Wi-Fi network, then scan the code with your phone.")
        .build();

    let kind = adw::ComboRow::builder()
        .title("Content")
        .model(&gtk::StringList::new(&["Text or link", "Wi-Fi network"]))
        .build();
    let text = adw::EntryRow::builder().title("Text or link").build();
    let ssid = adw::EntryRow::builder().title("Network name").visible(false).build();
    let password = adw::PasswordEntryRow::builder().title("Password").visible(false).build();
    let security = adw::ComboRow::builder()
        .title("Security")
        .model(&gtk::StringList::new(&["WPA / WPA2 / WPA3", "WEP", "None (open network)"]))
        .visible(false)
        .build();
    for row in [kind.upcast_ref::<gtk::Widget>(), text.upcast_ref(), ssid.upcast_ref(), password.upcast_ref(), security.upcast_ref()] {
        group.add(row);
    }

    // The code, with its actions underneath
    let picture = gtk::Picture::builder()
        .can_shrink(false)
        .halign(gtk::Align::Center)
        .visible(false)
        .build();
    let placeholder = gtk::Label::builder()
        .label("Your QR code appears here")
        .css_classes(["dim-label"])
        .height_request(PREVIEW_SIZE as i32)
        .build();
    let copy = gtk::Button::builder()
        .child(&adw::ButtonContent::builder().icon_name("edit-copy-symbolic").label("Copy Image").build())
        .sensitive(false)
        .build();
    let save = gtk::Button::builder()
        .child(&adw::ButtonContent::builder().icon_name("document-save-symbolic").label("Save as PNG…").build())
        .sensitive(false)
        .build();
    let buttons = gtk::Box::builder().spacing(12).halign(gtk::Align::Center).margin_bottom(18).build();
    buttons.append(&copy);
    buttons.append(&save);
    let card = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(12)
        .css_classes(["card"])
        .build();
    picture.set_margin_top(18);
    card.append(&picture);
    card.append(&placeholder);
    card.append(&buttons);
    group.add(&card);

    let current: Rc<RefCell<Option<String>>> = Rc::default();
    let update = {
        let (kind, text, ssid, password, security) = (kind.clone(), text.clone(), ssid.clone(), password.clone(), security.clone());
        let (picture, placeholder, copy, save, current) =
            (picture.clone(), placeholder.clone(), copy.clone(), save.clone(), current.clone());
        move || {
            let wifi = kind.selected() == 1;
            text.set_visible(!wifi);
            for row in [ssid.upcast_ref::<gtk::Widget>(), password.upcast_ref(), security.upcast_ref()] {
                row.set_visible(wifi);
            }
            let open = security.selected() == 2;
            password.set_sensitive(!open);

            let payload = if wifi {
                let sec = match security.selected() {
                    0 => WifiSecurity::Wpa,
                    1 => WifiSecurity::Wep,
                    _ => WifiSecurity::Open,
                };
                (!ssid.text().is_empty()).then(|| qr::wifi_payload(&ssid.text(), &password.text(), sec))
            } else {
                (!text.text().is_empty()).then(|| text.text().to_string())
            };
            let matrix = payload.as_deref().map(qr::encode);
            match matrix {
                Some(Ok(m)) => {
                    // Whole pixels per module keep the code crisp on screen.
                    let scale = (PREVIEW_SIZE / (m.width + 8)).max(1);
                    picture.set_paintable(Some(&gdk::Texture::for_pixbuf(&qr::to_pixbuf(&m, scale))));
                    picture.set_visible(true);
                    placeholder.set_visible(false);
                    *current.borrow_mut() = payload;
                }
                other => {
                    picture.set_visible(false);
                    placeholder.set_visible(true);
                    placeholder.set_label(match other {
                        Some(Err(_)) => "That's too much text for a QR code.",
                        _ if wifi => "Enter the network name to create a code",
                        _ => "Your QR code appears here",
                    });
                    *current.borrow_mut() = None;
                }
            }
            let ready = current.borrow().is_some();
            copy.set_sensitive(ready);
            save.set_sensitive(ready);
        }
    };
    let update = Rc::new(update);
    for entry in [text.upcast_ref::<gtk::Editable>(), ssid.upcast_ref(), password.upcast_ref()] {
        let update = update.clone();
        entry.connect_changed(move |_| update());
    }
    for combo in [&kind, &security] {
        let update = update.clone();
        combo.connect_selected_notify(move |_| update());
    }
    update();

    let print_ready = move |current: &Rc<RefCell<Option<String>>>| -> Option<Pixbuf> {
        let payload = current.borrow().clone()?;
        qr::encode(&payload).ok().map(|m| qr::to_pixbuf(&m, SAVE_SCALE))
    };
    {
        let (current, toasts) = (current.clone(), toasts.clone());
        copy.connect_clicked(move |b| {
            if let Some(pixbuf) = print_ready(&current) {
                b.clipboard().set_texture(&gdk::Texture::for_pixbuf(&pixbuf));
                toast(&toasts, "QR code image copied");
            }
        });
    }
    {
        let (current, toasts) = (current.clone(), toasts.clone());
        save.connect_clicked(move |b| {
            let Some(pixbuf) = print_ready(&current) else { return };
            let toasts = toasts.clone();
            let window = b.root().and_downcast::<gtk::Window>();
            gtk::FileDialog::builder().initial_name("qr-code.png").build().save(
                window.as_ref(),
                gio::Cancellable::NONE,
                move |res| {
                    let Some(path) = res.ok().and_then(|f| f.path()) else { return };
                    match pixbuf.savev(&path, "png", &[]) {
                        Ok(()) => toast(&toasts, format!("Saved {}", path.display())),
                        Err(e) => toast(&toasts, format!("Could not save: {e}")),
                    }
                },
            );
        });
    }
    group
}

fn flat_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
}
