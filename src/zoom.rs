//! `voltkit zoom`: Zoom & Annotate.
//!
//! Like the Screen Ruler, it takes a screenshot and shows it full screen, frozen
//! (Wayland doesn't let apps draw over other windows). You zoom into it and draw on
//! it with the pen, lines, rectangles, ellipses, arrows and text, on the screenshot
//! or on a white or black board, then copy or save the result as a picture.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{cairo, gdk, glib};

use crate::i18n::tr;
use crate::modules::annotate::{self, Canvas, Mark, Shape, Tool, View};
use crate::snapshot::{self, Tool as OverlayTool};
use crate::system;

pub fn run() {
    snapshot::run(&OverlayTool {
        title: N_!("Zoom & Annotate"),
        allow_title: N_!("Allow Zoom & Annotate to See Your Screen"),
        allow_text: N_!(
            "Zoom & Annotate draws on a snapshot of your screen. Your desktop will ask once whether \
             Voltkit may take screenshots; choose Allow. This window closes before the snapshot."
        ),
        denied_text: N_!(
            "Screenshots weren't allowed, so Zoom & Annotate can't open. Choose Try Again, then \
             Allow when your desktop asks. If you chose Deny before, forget that answer in \
             Voltkit's App Permissions page first."
        ),
        show,
    });
}

const HELP: &str = N_!(
    "Scroll: zoom  ·  Middle-drag or arrow keys: move  ·  0: reset zoom\n\
     Drag: pen  ·  Shift: line  ·  Ctrl: rectangle  ·  Tab: ellipse  ·  Ctrl+Shift: arrow\n\
     R G B O Y P: color  ·  H: highlighter  ·  Ctrl+scroll or [ ]: pen size\n\
     T: type text (Enter finishes)  ·  W / K: white / black board\n\
     Right-click or Ctrl+Z: undo  ·  Ctrl+Y: redo  ·  E: erase all\n\
     Ctrl+C: copy the picture  ·  Ctrl+S: save it  ·  F1: this help  ·  Esc: close"
);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Board {
    Screen,
    White,
    Black,
}

struct Drag {
    tool: Tool,
    /// Where it started and where it is, in the picture's pixels.
    start: (f64, f64),
    end: (f64, f64),
    path: Vec<(f64, f64)>,
}

struct State {
    screenshot: cairo::ImageSurface,
    canvas: Canvas,
    view: View,
    /// Picture pixels to unzoomed screen pixels.
    k: f64,
    color: (f64, f64, f64),
    /// The pen's width on screen, in logical pixels.
    pen: f64,
    highlight: bool,
    board: Board,
    drag: Option<Drag>,
    /// Typing: where, and what has been typed.
    text: Option<((f64, f64), String)>,
    awaiting_text: bool,
    tab_down: bool,
    cursor: (f64, f64),
    help: bool,
    note: Option<String>,
    /// Where Ctrl+S puts the picture.
    folder: std::path::PathBuf,
}

impl State {
    fn to_picture(&self, screen: (f64, f64)) -> (f64, f64) {
        let p = self.view.to_picture(screen);
        (p.0 / self.k, p.1 / self.k)
    }

    /// A mark with the pen as it is now, in the picture's pixels.
    fn mark(&self, shape: Shape) -> Mark {
        Mark { shape, color: self.color, width: self.pen / self.k, highlight: self.highlight }
    }

    fn status(&self) -> String {
        let tool = if self.awaiting_text || self.text.is_some() {
            tr("Text").to_owned()
        } else if self.highlight {
            tr("Highlighter").to_owned()
        } else {
            tr("Pen").to_owned()
        };
        let board = match self.board {
            Board::Screen => "",
            Board::White => "  ·  ⬜",
            Board::Black => "  ·  ⬛",
        };
        let base = format!("{tool}  ·  {}  ·  {:.0} px  ·  {:.0} %{board}  ·  {}", tr(annotate::color_name(self.color)), self.pen, self.view.zoom * 100.0, tr("F1: help"));
        match &self.note {
            Some(note) => format!("{note}  ·  {base}"),
            None => base,
        }
    }
}

fn surface_from(texture: &gdk::Texture) -> Option<cairo::ImageSurface> {
    let (w, h) = (texture.width(), texture.height());
    let stride = w as usize * 4;
    let mut data = vec![0u8; stride * h as usize];
    texture.download(&mut data, stride);
    cairo::ImageSurface::create_for_data(data, cairo::Format::ARgb32, w, h, stride as i32).ok()
}

fn show(app: &adw::Application, texture: gdk::Texture) {
    if let Some((window, area, _)) = build(app, texture) {
        window.fullscreen();
        window.present();
        area.grab_focus();
    }
}

/// For the manual's screenshots: the window on `texture`, with a few marks already drawn.
pub fn demo(app: &adw::Application, texture: gdk::Texture) -> Option<gtk::ApplicationWindow> {
    let (window, _, state) = build(app, texture)?;
    let mut s = state.borrow_mut();
    let red = annotate::color_for_key('r')?;
    let blue = annotate::color_for_key('b')?;
    let yellow = annotate::color_for_key('y')?;
    let k = f64::from(s.screenshot.width()) / 1240.0;
    let mark = |shape, color, width: f64, highlight| Mark { shape, color, width: width * k, highlight };
    s.canvas.push(mark(Shape::Ellipse((470.0 * k, 250.0 * k), (700.0 * k, 360.0 * k)), red, 4.0, false));
    s.canvas.push(mark(Shape::Arrow((300.0 * k, 160.0 * k), (480.0 * k, 260.0 * k)), red, 4.0, false));
    s.canvas.push(mark(Shape::Text { at: (240.0 * k, 90.0 * k), text: "Look at this".into() }, blue, 4.0, false));
    s.canvas.push(mark(Shape::Line((520.0 * k, 400.0 * k), (880.0 * k, 400.0 * k)), yellow, 4.0, true));
    s.help = false;
    drop(s);
    Some(window)
}

/// Builds the window; the caller shows it.
fn build(app: &adw::Application, texture: gdk::Texture) -> Option<(gtk::ApplicationWindow, gtk::DrawingArea, Rc<RefCell<State>>)> {
    let screenshot = surface_from(&texture)?;
    let area = gtk::DrawingArea::builder().hexpand(true).vexpand(true).focusable(true).build();
    area.set_cursor_from_name(Some("crosshair"));
    area.update_property(&[gtk::accessible::Property::Label(tr("Drawing area. Press F1 for the keys."))]);
    let status = gtk::Label::builder().halign(gtk::Align::Center).valign(gtk::Align::End).margin_bottom(16).css_classes(["osd", "toolbar"]).build();
    let help = gtk::Label::builder()
        .label(tr(HELP))
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .justify(gtk::Justification::Left)
        .css_classes(["osd", "toolbar"])
        .visible(false)
        .build();
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&area));
    overlay.add_overlay(&status);
    overlay.add_overlay(&help);
    // The labels must not swallow clicks meant for the drawing.
    status.set_can_target(false);
    help.set_can_target(false);

    let window = gtk::ApplicationWindow::builder().application(app).title(tr("Zoom & Annotate")).decorated(false).child(&overlay).build();

    let state = Rc::new(RefCell::new(State {
        screenshot,
        canvas: Canvas::default(),
        view: View::default(),
        k: 1.0,
        color: annotate::color_for_key('r').unwrap_or((1.0, 0.0, 0.0)),
        pen: 4.0,
        highlight: false,
        board: Board::Screen,
        drag: None,
        text: None,
        awaiting_text: false,
        tab_down: false,
        cursor: (0.0, 0.0),
        help: false,
        note: None,
        folder: crate::config::Config::load().map(|c| c.zoom.folder()).unwrap_or_else(|_| crate::config::ZoomConfig::default_folder()),
    }));
    let tex_w = f64::from(texture.width());

    // Whatever changes, update the status line and redraw.
    let refresh: Rc<dyn Fn()> = {
        let (state, area, status, help) = (state.clone(), area.clone(), status.clone(), help.clone());
        Rc::new(move || {
            let s = state.borrow();
            status.set_label(&s.status());
            help.set_label(&format!("{}\n\n{}", tr(HELP), crate::i18n::fill(tr("Ctrl+S saves in {}  ·  Ctrl+Shift+S: choose where, once"), &[&s.folder.display()])));
            help.set_visible(s.help);
            area.queue_draw();
        })
    };

    {
        let state = state.clone();
        area.set_draw_func(move |area, cr, w, h| {
            let mut s = state.borrow_mut();
            s.k = f64::from(area.width().max(1)) / tex_w.max(1.0);
            draw(cr, f64::from(w), f64::from(h), &s);
        });
    }

    // Drawing with the left button.
    let drag = gtk::GestureDrag::builder().button(1).build();
    {
        let (state, refresh, area) = (state.clone(), refresh.clone(), area.clone());
        drag.connect_drag_begin(move |gesture, x, y| {
            area.grab_focus();
            let mut s = state.borrow_mut();
            let at = s.to_picture((x, y));
            if s.awaiting_text {
                s.awaiting_text = false;
                s.text = Some((at, String::new()));
            } else {
                let mods = gesture.current_event_state();
                let tool = Tool::from_keys(mods.contains(gdk::ModifierType::SHIFT_MASK), mods.contains(gdk::ModifierType::CONTROL_MASK), s.tab_down);
                s.drag = Some(Drag { tool, start: at, end: at, path: vec![at] });
            }
            drop(s);
            refresh();
        });
    }
    {
        let (state, refresh) = (state.clone(), refresh.clone());
        drag.connect_drag_update(move |gesture, dx, dy| {
            let Some((sx, sy)) = gesture.start_point() else { return };
            let mut s = state.borrow_mut();
            let at = s.to_picture((sx + dx, sy + dy));
            if let Some(d) = s.drag.as_mut() {
                d.end = at;
                if d.tool == Tool::Pen {
                    d.path.push(at);
                }
            }
            drop(s);
            refresh();
        });
    }
    {
        let (state, refresh) = (state.clone(), refresh.clone());
        drag.connect_drag_end(move |_, _, _| {
            let mut s = state.borrow_mut();
            if let Some(d) = s.drag.take() {
                let moved = (d.end.0 - d.start.0).abs() + (d.end.1 - d.start.1).abs() > 1.0;
                if moved || d.path.len() > 2 {
                    let mark = s.mark(annotate::shape_for(d.tool, d.start, d.end, &d.path));
                    s.canvas.push(mark);
                }
            }
            drop(s);
            refresh();
        });
    }
    area.add_controller(drag);

    // Moving the zoomed picture with the middle button.
    let pan = gtk::GestureDrag::builder().button(2).build();
    let last: Rc<RefCell<(f64, f64)>> = Rc::default();
    {
        let last = last.clone();
        pan.connect_drag_begin(move |_, _, _| *last.borrow_mut() = (0.0, 0.0));
    }
    {
        let (state, refresh, area, last) = (state.clone(), refresh.clone(), area.clone(), last.clone());
        pan.connect_drag_update(move |_, dx, dy| {
            let prev = std::mem::replace(&mut *last.borrow_mut(), (dx, dy));
            state.borrow_mut().view.pan((dx - prev.0, dy - prev.1), f64::from(area.width()), f64::from(area.height()));
            refresh();
        });
    }
    area.add_controller(pan);

    // Right-click undoes.
    let right = gtk::GestureClick::builder().button(3).build();
    {
        let (state, refresh) = (state.clone(), refresh.clone());
        right.connect_pressed(move |_, _, _, _| {
            state.borrow_mut().canvas.undo();
            refresh();
        });
    }
    area.add_controller(right);

    let motion = gtk::EventControllerMotion::new();
    {
        let (state, area) = (state.clone(), area.clone());
        motion.connect_motion(move |_, x, y| {
            state.borrow_mut().cursor = (x, y);
            area.queue_draw();
        });
    }
    area.add_controller(motion);

    // The wheel zooms; with Ctrl it changes the pen's size.
    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    {
        let (state, refresh, area) = (state.clone(), refresh.clone(), area.clone());
        scroll.connect_scroll(move |ctrl, _, dy| {
            let mut s = state.borrow_mut();
            if ctrl.current_event_state().contains(gdk::ModifierType::CONTROL_MASK) {
                s.pen = (s.pen - dy).clamp(annotate::MIN_WIDTH, annotate::MAX_WIDTH);
            } else {
                let at = s.cursor;
                s.view.zoom_at(at, 1.15_f64.powf(-dy), f64::from(area.width()), f64::from(area.height()));
            }
            drop(s);
            refresh();
            glib::Propagation::Stop
        });
    }
    area.add_controller(scroll);

    // The keys.
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let (state, refresh, window, area) = (state.clone(), refresh.clone(), window.clone(), area.clone());
        keys.connect_key_pressed(move |_, key, _, mods| {
            let handled = on_key(&state, &window, &area, &refresh, key, mods);
            refresh();
            if handled { glib::Propagation::Stop } else { glib::Propagation::Proceed }
        });
    }
    {
        let state = state.clone();
        keys.connect_key_released(move |_, key, _, _| {
            if key == gdk::Key::Tab {
                state.borrow_mut().tab_down = false;
            }
        });
    }
    window.add_controller(keys);

    refresh();
    Some((window, area, state))
}

/// Handles one key press. Returns whether it was used.
fn on_key(state: &Rc<RefCell<State>>, window: &gtk::ApplicationWindow, area: &gtk::DrawingArea, refresh: &Rc<dyn Fn()>, key: gdk::Key, mods: gdk::ModifierType) -> bool {
    let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
    let shift = mods.contains(gdk::ModifierType::SHIFT_MASK);
    let mut s = state.borrow_mut();
    s.note = None;
    let (w, h) = (f64::from(area.width()), f64::from(area.height()));

    // While typing, keys are letters, not commands.
    if s.text.is_some() {
        match key {
            gdk::Key::Escape => s.text = None,
            gdk::Key::Return | gdk::Key::KP_Enter if shift => {
                if let Some((_, t)) = s.text.as_mut() {
                    t.push('\n');
                }
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                if let Some((at, text)) = s.text.take()
                    && !text.trim().is_empty()
                {
                    let mark = s.mark(Shape::Text { at, text });
                    s.canvas.push(mark);
                }
            }
            gdk::Key::BackSpace => {
                if let Some((_, t)) = s.text.as_mut() {
                    t.pop();
                }
            }
            _ => {
                if !ctrl && let Some(c) = key.to_unicode().filter(|c| !c.is_control())
                    && let Some((_, t)) = s.text.as_mut() {
                        t.push(c);
                    }
            }
        }
        return true;
    }

    match key {
        gdk::Key::Escape => {
            if s.awaiting_text || s.help {
                s.awaiting_text = false;
                s.help = false;
            } else {
                drop(s);
                window.close();
            }
            return true;
        }
        gdk::Key::Tab | gdk::Key::ISO_Left_Tab => {
            s.tab_down = true;
            return true;
        }
        gdk::Key::F1 | gdk::Key::question => s.help = !s.help,
        gdk::Key::Left => s.view.pan((60.0, 0.0), w, h),
        gdk::Key::Right => s.view.pan((-60.0, 0.0), w, h),
        gdk::Key::Up => s.view.pan((0.0, 60.0), w, h),
        gdk::Key::Down => s.view.pan((0.0, -60.0), w, h),
        gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => s.view.zoom_at((w / 2.0, h / 2.0), 1.25, w, h),
        gdk::Key::minus | gdk::Key::KP_Subtract => s.view.zoom_at((w / 2.0, h / 2.0), 0.8, w, h),
        gdk::Key::_0 | gdk::Key::KP_0 => s.view.reset(),
        gdk::Key::bracketleft => s.pen = (s.pen - 1.0).max(annotate::MIN_WIDTH),
        gdk::Key::bracketright => s.pen = (s.pen + 1.0).min(annotate::MAX_WIDTH),
        _ => {
            let Some(c) = key.to_unicode() else { return false };
            if ctrl {
                match c.to_ascii_lowercase() {
                    'z' if shift => {
                        s.canvas.redo();
                    }
                    'z' => {
                        s.canvas.undo();
                    }
                    'y' => {
                        s.canvas.redo();
                    }
                    'c' => {
                        let message = match export_png(&s).and_then(|png| system::copy_png_to_clipboard(&png)) {
                            Ok(()) => tr("Copied the picture").to_owned(),
                            Err(e) => crate::i18n::fill(tr("Could not copy: {}"), &[&format!("{e:#}")]),
                        };
                        s.note = Some(message);
                    }
                    's' if shift => {
                        // Save As: choose the file once, without changing the usual folder.
                        match export_png(&s) {
                            Ok(png) => {
                                let (state, refresh, folder) = (state.clone(), refresh.clone(), s.folder.clone());
                                drop(s);
                                save_as(window, png, folder, state, refresh);
                                return true;
                            }
                            Err(e) => s.note = Some(crate::i18n::fill(tr("Could not save: {}"), &[&format!("{e:#}")])),
                        }
                    }
                    's' => {
                        let message = match save_png(&s) {
                            Ok(path) => crate::i18n::fill(tr("Saved {}"), &[&path.display()]),
                            Err(e) => crate::i18n::fill(tr("Could not save: {}"), &[&format!("{e:#}")]),
                        };
                        s.note = Some(message);
                    }
                    _ => return false,
                }
            } else if let Some(color) = annotate::color_for_key(c) {
                s.color = color;
            } else {
                match c.to_ascii_lowercase() {
                    'h' => s.highlight = !s.highlight,
                    'w' => s.board = if s.board == Board::White { Board::Screen } else { Board::White },
                    'k' => s.board = if s.board == Board::Black { Board::Screen } else { Board::Black },
                    't' => s.awaiting_text = true,
                    'e' => s.canvas.clear(),
                    _ => return false,
                }
            }
        }
    }
    true
}

// ---------------------------------------------------------------- Drawing

fn draw(cr: &cairo::Context, w: f64, h: f64, s: &State) {
    match s.board {
        Board::White => cr.set_source_rgb(1.0, 1.0, 1.0),
        Board::Black => cr.set_source_rgb(0.0, 0.0, 0.0),
        Board::Screen => cr.set_source_rgb(0.1, 0.1, 0.1),
    }
    let _ = cr.paint();
    cr.save().ok();
    cr.translate(s.view.offset.0, s.view.offset.1);
    cr.scale(s.k * s.view.zoom, s.k * s.view.zoom);
    if s.board == Board::Screen {
        let _ = cr.set_source_surface(&s.screenshot, 0.0, 0.0);
        cr.source().set_filter(if s.view.zoom > 2.0 { cairo::Filter::Nearest } else { cairo::Filter::Good });
        let _ = cr.paint();
    }
    paint_marks(cr, s.canvas.marks());
    if let Some(d) = &s.drag {
        paint_marks(cr, &[s.mark(annotate::shape_for(d.tool, d.start, d.end, &d.path))]);
    }
    if let Some((at, text)) = &s.text {
        // A bar after the text shows where typing goes.
        paint_marks(cr, &[s.mark(Shape::Text { at: *at, text: format!("{text}▏") })]);
    }
    cr.restore().ok();

    // A ring the size of the pen, at the pointer.
    if s.drag.is_none() && s.text.is_none() {
        let (x, y) = s.cursor;
        cr.arc(x, y, (s.pen * s.view.zoom / 2.0).max(2.0), 0.0, std::f64::consts::TAU);
        cr.set_source_rgba(s.color.0, s.color.1, s.color.2, 0.9);
        cr.set_line_width(1.5);
        let _ = cr.stroke();
    }
    let _ = (w, h);
}

/// Draws marks in the picture's pixels (the context is already scaled to them).
fn paint_marks(cr: &cairo::Context, marks: &[Mark]) {
    for m in marks {
        let (r, g, b) = m.color;
        cr.set_source_rgba(r, g, b, if m.highlight { 0.35 } else { 1.0 });
        cr.set_line_cap(if m.highlight { cairo::LineCap::Square } else { cairo::LineCap::Round });
        cr.set_line_join(cairo::LineJoin::Round);
        cr.set_line_width(if m.highlight { m.width * 4.0 } else { m.width });
        match &m.shape {
            Shape::Pen(points) => {
                if let [(x, y)] = points.as_slice() {
                    cr.arc(*x, *y, m.width / 2.0, 0.0, std::f64::consts::TAU);
                    let _ = cr.fill();
                } else if let Some(((x0, y0), rest)) = points.split_first() {
                    cr.move_to(*x0, *y0);
                    for (x, y) in rest {
                        cr.line_to(*x, *y);
                    }
                    let _ = cr.stroke();
                }
            }
            Shape::Line(a, b) => {
                cr.move_to(a.0, a.1);
                cr.line_to(b.0, b.1);
                let _ = cr.stroke();
            }
            Shape::Rect(a, b) => {
                let (x, y, w, h) = annotate::bounds(*a, *b);
                cr.rectangle(x, y, w, h);
                let _ = cr.stroke();
            }
            Shape::Ellipse(a, b) => {
                let (x, y, w, h) = annotate::bounds(*a, *b);
                if w > 0.0 && h > 0.0 {
                    cr.save().ok();
                    cr.translate(x + w / 2.0, y + h / 2.0);
                    cr.scale(w / 2.0, h / 2.0);
                    cr.arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::TAU);
                    cr.restore().ok();
                    let _ = cr.stroke();
                }
            }
            Shape::Arrow(a, b) => {
                cr.move_to(a.0, a.1);
                cr.line_to(b.0, b.1);
                let _ = cr.stroke();
                let [l, r] = annotate::arrow_head(*a, *b, (m.width * 4.0).max(14.0));
                cr.move_to(b.0, b.1);
                cr.line_to(l.0, l.1);
                cr.line_to(r.0, r.1);
                cr.close_path();
                let _ = cr.fill();
            }
            Shape::Text { at, text } => {
                cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
                let size = annotate::text_size(m.width);
                cr.set_font_size(size);
                for (i, line) in text.split('\n').enumerate() {
                    cr.move_to(at.0, at.1 + size * (i as f64 + 1.0));
                    let _ = cr.show_text(line);
                }
            }
        }
    }
}

/// The picture with everything drawn on it, at the screenshot's own size.
fn render(s: &State) -> anyhow::Result<cairo::ImageSurface> {
    let (w, h) = (s.screenshot.width(), s.screenshot.height());
    let out = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h)?;
    let cr = cairo::Context::new(&out)?;
    match s.board {
        Board::White => {
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint()?;
        }
        Board::Black => {
            cr.set_source_rgb(0.0, 0.0, 0.0);
            cr.paint()?;
        }
        Board::Screen => {
            cr.set_source_surface(&s.screenshot, 0.0, 0.0)?;
            cr.paint()?;
        }
    }
    paint_marks(&cr, s.canvas.marks());
    drop(cr);
    Ok(out)
}

fn export_png(s: &State) -> anyhow::Result<Vec<u8>> {
    let mut surface = render(s)?;
    surface.flush();
    let (w, h, stride) = (surface.width(), surface.height(), surface.stride() as usize);
    let bytes = glib::Bytes::from(&surface.data()?[..]);
    // Cairo's pixels are premultiplied, blue first, on a little-endian computer.
    let texture = gdk::MemoryTexture::new(w, h, gdk::MemoryFormat::B8g8r8a8Premultiplied, &bytes, stride);
    Ok(texture.save_to_png_bytes().to_vec())
}

fn save_png(s: &State) -> anyhow::Result<std::path::PathBuf> {
    let dir = s.folder.clone();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("annotation-{}.png", chrono::Local::now().format("%Y%m%d-%H%M%S")));
    std::fs::write(&path, export_png(s)?)?;
    Ok(path)
}

/// Asks where to save this one picture, starting in the usual folder.
fn save_as(window: &gtk::ApplicationWindow, png: Vec<u8>, folder: std::path::PathBuf, state: Rc<RefCell<State>>, refresh: Rc<dyn Fn()>) {
    let _ = std::fs::create_dir_all(&folder);
    let dialog = gtk::FileDialog::builder()
        .title(tr("Save the Picture"))
        .initial_name(format!("annotation-{}.png", chrono::Local::now().format("%Y%m%d-%H%M%S")))
        .initial_folder(&gtk::gio::File::for_path(&folder))
        .build();
    dialog.save(Some(window), gtk::gio::Cancellable::NONE, move |result| {
        let Ok(file) = result else { return };
        let Some(path) = file.path() else { return };
        let message = match std::fs::write(&path, &png) {
            Ok(()) => crate::i18n::fill(tr("Saved {}"), &[&path.display()]),
            Err(e) => crate::i18n::fill(tr("Could not save: {}"), &[&e]),
        };
        state.borrow_mut().note = Some(message);
        refresh();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(w: i32, h: i32) -> State {
        let screenshot = cairo::ImageSurface::create(cairo::Format::ARgb32, w, h).unwrap();
        State {
            screenshot,
            canvas: Canvas::default(),
            view: View::default(),
            k: 1.0,
            color: (1.0, 0.0, 0.0),
            pen: 4.0,
            highlight: false,
            board: Board::Screen,
            drag: None,
            text: None,
            awaiting_text: false,
            tab_down: false,
            cursor: (0.0, 0.0),
            help: false,
            note: None,
            folder: std::env::temp_dir(),
        }
    }

    #[test]
    fn the_saved_picture_has_the_screenshots_size_and_the_marks_on_it() {
        let mut s = state(120, 80);
        s.board = Board::White;
        let mark = s.mark(Shape::Line((0.0, 40.0), (120.0, 40.0)));
        s.canvas.push(mark);
        let png = export_png(&s).unwrap();
        assert_eq!(&png[..4], b"\x89PNG");
        let surface = render(&s).unwrap();
        assert_eq!((surface.width(), surface.height()), (120, 80));
        let mut surface = surface;
        surface.flush();
        let data = surface.data().unwrap();
        let stride = 120 * 4;
        // A pixel on the red line, and one well away from it (white board).
        let on = &data[40 * stride + 60 * 4..][..4];
        let off = &data[5 * stride + 60 * 4..][..4];
        assert_eq!(off, [255, 255, 255, 255]);
        assert!(on[2] > 200 && on[1] < 80, "the line is red (BGRA): {on:?}");
    }

    #[test]
    fn marks_are_made_with_the_pen_in_picture_pixels() {
        let mut s = state(100, 100);
        s.k = 0.5; // a display where the picture has twice the pixels of the window
        s.pen = 4.0;
        let m = s.mark(Shape::Line((0.0, 0.0), (1.0, 1.0)));
        assert_eq!(m.width, 8.0, "a 4 px pen on screen is 8 picture pixels");
        assert_eq!(s.to_picture((50.0, 50.0)), (100.0, 100.0));
    }

    #[test]
    fn the_status_line_tells_the_pen_and_zoom() {
        let mut s = state(10, 10);
        s.color = annotate::color_for_key('r').unwrap();
        s.highlight = true;
        s.view.zoom = 2.0;
        let text = s.status();
        assert!(text.contains("Highlighter") && text.contains("Red") && text.contains("200 %"), "{text}");
    }
}
