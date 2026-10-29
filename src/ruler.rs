//! `voltkit ruler`: the Screen Ruler.
//!
//! Wayland doesn't let apps draw over other windows, so the ruler takes a
//! screenshot and shows it full screen, frozen; measurements are made on that
//! image, in the screenshot's real pixels.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{cairo, gdk, gio, glib};

use crate::portal::{self, APP_ID, Cancelled};
use crate::system;

const HINT: &str = "Drag to measure  ·  Shift: straight line  ·  Ctrl+C: copy size  ·  Right-click: clear  ·  Esc: close";

pub fn run() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(|app| {
        // Hold the app open while the screenshot is taken, before any window exists.
        let (app, hold) = (app.clone(), app.hold());
        glib::spawn_future_local(async move {
            let shot = gio::spawn_blocking(|| portal::block_on(portal::screenshot_full())).await;
            match shot {
                Ok(Ok(path)) => {
                    let texture = gdk::Texture::from_filename(&path);
                    let _ = std::fs::remove_file(&path);
                    match texture {
                        Ok(texture) => show(&app, texture),
                        Err(e) => fail(&e.to_string()).await,
                    }
                }
                Ok(Err(e)) if e.is::<Cancelled>() => {}
                Ok(Err(e)) => fail(&format!("{e:#}")).await,
                Err(_) => fail("screenshot worker crashed").await,
            }
            drop(hold);
        });
    });
    app.run_with_args::<&str>(&[]);
}

async fn fail(message: &str) {
    let message = message.to_owned();
    let _ = gio::spawn_blocking(move || portal::block_on(portal::notify("Screen Ruler", &message))).await;
}

#[derive(Default)]
struct Measure {
    start: Option<(f64, f64)>,
    end: Option<(f64, f64)>,
    cursor: Option<(f64, f64)>,
}

impl Measure {
    /// Size of the current measurement in screenshot pixels.
    fn size(&self, ratio: f64) -> Option<(i64, i64)> {
        let ((x0, y0), (x1, y1)) = (self.start?, self.end?);
        Some((((x1 - x0).abs() * ratio).round() as i64, ((y1 - y0).abs() * ratio).round() as i64))
    }
}

/// "120 × 48 px", or just the length for a straight line.
fn size_label(w: i64, h: i64) -> String {
    match (w, h) {
        (w, 0) => format!("{w} px"),
        (0, h) => format!("{h} px"),
        (w, h) => format!("{w} × {h} px"),
    }
}

fn show(app: &adw::Application, texture: gdk::Texture) {
    let picture = gtk::Picture::builder()
        .paintable(&texture)
        .content_fit(gtk::ContentFit::Fill)
        .can_shrink(true)
        .build();
    let area = gtk::DrawingArea::builder().hexpand(true).vexpand(true).build();
    area.set_cursor_from_name(Some("crosshair"));
    let hint = gtk::Label::builder()
        .label(HINT)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Start)
        .margin_top(16)
        .css_classes(["osd", "toolbar"])
        .build();
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&picture));
    overlay.add_overlay(&area);
    overlay.add_overlay(&hint);
    // The hint must not swallow clicks meant for the ruler.
    hint.set_can_target(false);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Screen Ruler")
        .decorated(false)
        .child(&overlay)
        .build();

    let state: Rc<RefCell<Measure>> = Rc::default();
    // Screenshot pixels per logical pixel of the window.
    let ratio = {
        let (area, texture) = (area.clone(), texture.clone());
        move || texture.width() as f64 / area.width().max(1) as f64
    };
    let ratio = Rc::new(ratio);

    {
        let (state, ratio) = (state.clone(), ratio.clone());
        area.set_draw_func(move |_, cr, w, h| draw(cr, w as f64, h as f64, &state.borrow(), ratio()));
    }

    let drag = gtk::GestureDrag::new();
    {
        let (state, area) = (state.clone(), area.clone());
        drag.connect_drag_begin(move |_, x, y| {
            let mut s = state.borrow_mut();
            s.start = Some((x, y));
            s.end = Some((x, y));
            area.queue_draw();
        });
    }
    {
        let (state, area) = (state.clone(), area.clone());
        drag.connect_drag_update(move |gesture, dx, dy| {
            let straight = gesture.current_event_state().contains(gdk::ModifierType::SHIFT_MASK);
            let (dx, dy) = if straight { if dx.abs() >= dy.abs() { (dx, 0.0) } else { (0.0, dy) } } else { (dx, dy) };
            let mut s = state.borrow_mut();
            if let Some((x, y)) = s.start {
                s.end = Some((x + dx, y + dy));
                s.cursor = s.end;
            }
            area.queue_draw();
        });
    }
    area.add_controller(drag);

    let motion = gtk::EventControllerMotion::new();
    {
        let (state, area) = (state.clone(), area.clone());
        motion.connect_motion(move |_, x, y| {
            state.borrow_mut().cursor = Some((x, y));
            area.queue_draw();
        });
    }
    area.add_controller(motion);

    let right_click = gtk::GestureClick::builder().button(3).build();
    {
        let (state, area) = (state.clone(), area.clone());
        right_click.connect_pressed(move |_, _, _, _| {
            let mut s = state.borrow_mut();
            s.start = None;
            s.end = None;
            area.queue_draw();
        });
    }
    area.add_controller(right_click);

    let keys = gtk::EventControllerKey::new();
    {
        let (state, window, hint, ratio) = (state.clone(), window.clone(), hint.clone(), ratio.clone());
        keys.connect_key_pressed(move |_, key, _, modifiers| {
            if key == gdk::Key::Escape {
                window.close();
                return glib::Propagation::Stop;
            }
            if modifiers.contains(gdk::ModifierType::CONTROL_MASK) && matches!(key, gdk::Key::c | gdk::Key::C) {
                if let Some((w, h)) = state.borrow().size(ratio()) {
                    let text = match (w, h) {
                        (w, 0) => w.to_string(),
                        (0, h) => h.to_string(),
                        (w, h) => format!("{w}×{h}"),
                    };
                    // wl-copy keeps serving the clipboard after this window closes.
                    let message = match system::copy_to_clipboard(&text) {
                        Ok(()) => format!("Copied {text}  ·  Esc: close"),
                        Err(e) => format!("Could not copy: {e}"),
                    };
                    hint.set_label(&message);
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
    }
    window.add_controller(keys);

    window.fullscreen();
    window.present();
}

fn draw(cr: &cairo::Context, w: f64, h: f64, s: &Measure, ratio: f64) {
    // Every line is drawn twice, dark and wide under bright and thin, to stay
    // visible on any background.
    let stroke = |cr: &cairo::Context| {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.6);
        cr.set_line_width(3.0);
        let _ = cr.stroke_preserve();
        cr.set_source_rgb(1.0, 0.85, 0.1);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
    };

    if let Some((cx, cy)) = s.cursor {
        cr.set_dash(&[6.0, 4.0], 0.0);
        cr.move_to(0.0, cy.floor() + 0.5);
        cr.line_to(w, cy.floor() + 0.5);
        cr.move_to(cx.floor() + 0.5, 0.0);
        cr.line_to(cx.floor() + 0.5, h);
        stroke(cr);
        cr.set_dash(&[], 0.0);
    }

    if let (Some((x0, y0)), Some((x1, y1)), Some((mw, mh))) = (s.start, s.end, s.size(ratio)) {
        let (x, y, rw, rh) = (x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs());
        cr.set_source_rgba(1.0, 0.85, 0.1, 0.15);
        cr.rectangle(x, y, rw, rh);
        let _ = cr.fill();
        cr.rectangle(x.floor() + 0.5, y.floor() + 0.5, rw.round(), rh.round());
        stroke(cr);
        label(cr, x1 + 14.0, y1 + 14.0, w, h, &size_label(mw, mh));
    } else if let Some((cx, cy)) = s.cursor {
        let text = format!("{}, {}", (cx * ratio).round(), (cy * ratio).round());
        label(cr, cx + 14.0, cy + 14.0, w, h, &text);
    }
}

/// Draws a text badge near (x, y), kept inside the screen.
fn label(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, text: &str) {
    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
    cr.set_font_size(15.0);
    let Ok(ext) = cr.text_extents(text) else { return };
    let (pad, bw, bh) = (8.0, ext.width() + 16.0, ext.height() + 14.0);
    let bx = x.min(w - bw - 4.0).max(4.0);
    let by = y.min(h - bh - 4.0).max(4.0);
    cr.set_source_rgba(0.08, 0.08, 0.1, 0.85);
    rounded(cr, bx, by, bw, bh, 6.0);
    let _ = cr.fill();
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.move_to(bx + pad - ext.x_bearing(), by + 7.0 - ext.y_bearing());
    let _ = cr.show_text(text);
}

fn rounded(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::PI;
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 1.5 * PI);
    cr.close_path();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_in_screenshot_pixels() {
        let m = Measure { start: Some((10.0, 20.0)), end: Some((70.0, 50.0)), cursor: None };
        assert_eq!(m.size(1.0), Some((60, 30)));
        // On a 2× display a logical pixel is two screenshot pixels.
        assert_eq!(m.size(2.0), Some((120, 60)));
        // Dragging up or left measures the same.
        let back = Measure { start: Some((70.0, 50.0)), end: Some((10.0, 20.0)), cursor: None };
        assert_eq!(back.size(1.0), Some((60, 30)));
        assert_eq!(Measure::default().size(1.0), None);
    }

    #[test]
    fn labels_lines_and_boxes() {
        assert_eq!(size_label(120, 48), "120 × 48 px");
        assert_eq!(size_label(300, 0), "300 px");
        assert_eq!(size_label(0, 7), "7 px");
    }
}
