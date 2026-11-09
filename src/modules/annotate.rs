//! The drawing model of Zoom & Annotate: marks on a picture, a zoomable view, and
//! the geometry the screen overlay needs. Everything here is plain numbers, so it is
//! tested without a screen. Marks are stored in the picture's own pixels, so zooming
//! never moves them and a saved image has them exactly where you drew them.

pub type Point = (f64, f64);

/// What a mark looks like.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A free line through these points.
    Pen(Vec<Point>),
    Line(Point, Point),
    Rect(Point, Point),
    Ellipse(Point, Point),
    Arrow(Point, Point),
    Text { at: Point, text: String },
}

/// One thing drawn on the picture.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub shape: Shape,
    pub color: (f64, f64, f64),
    /// Line width, or the text size is derived from it.
    pub width: f64,
    /// Translucent and wide, like a highlighter pen.
    pub highlight: bool,
}

/// The kind of mark a drag makes, from the keys held when it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Line,
    Rect,
    Ellipse,
    Arrow,
}

impl Tool {
    /// Shift draws a straight line, Ctrl a rectangle, Ctrl+Shift an arrow, and the Tab key
    /// held an ellipse. Nothing held is the pen.
    pub fn from_keys(shift: bool, ctrl: bool, tab: bool) -> Tool {
        match (ctrl, shift, tab) {
            (true, true, _) => Tool::Arrow,
            (true, false, _) => Tool::Rect,
            (false, true, _) => Tool::Line,
            (false, false, true) => Tool::Ellipse,
            (false, false, false) => Tool::Pen,
        }
    }
}

/// Builds the shape for a drag from `start` to `end` (the pen follows `path`).
pub fn shape_for(tool: Tool, start: Point, end: Point, path: &[Point]) -> Shape {
    match tool {
        Tool::Pen => Shape::Pen(path.to_vec()),
        Tool::Line => Shape::Line(start, end),
        Tool::Rect => Shape::Rect(start, end),
        Tool::Ellipse => Shape::Ellipse(start, end),
        Tool::Arrow => Shape::Arrow(start, end),
    }
}

/// The colors on the keys: R, G, B, O, Y and P (red, green, blue, orange, yellow, pink).
pub fn color_for_key(c: char) -> Option<(f64, f64, f64)> {
    Some(match c.to_ascii_lowercase() {
        'r' => (0.93, 0.18, 0.18),
        'g' => (0.18, 0.72, 0.25),
        'b' => (0.16, 0.42, 0.95),
        'o' => (0.98, 0.55, 0.08),
        'y' => (0.98, 0.84, 0.10),
        'p' => (0.95, 0.35, 0.70),
        _ => return None,
    })
}

pub fn color_name(color: (f64, f64, f64)) -> &'static str {
    ['r', 'g', 'b', 'o', 'y', 'p']
        .into_iter()
        .find(|c| color_for_key(*c) == Some(color))
        .map_or(N_!("Custom"), |c| match c {
            'r' => N_!("Red"),
            'g' => N_!("Green"),
            'b' => N_!("Blue"),
            'o' => N_!("Orange"),
            'y' => N_!("Yellow"),
            _ => N_!("Pink"),
        })
}

pub const MIN_WIDTH: f64 = 1.0;
pub const MAX_WIDTH: f64 = 40.0;

/// The marks on the picture, with undo and redo.
#[derive(Default, Debug)]
pub struct Canvas {
    marks: Vec<Mark>,
    undone: Vec<Mark>,
}

impl Canvas {
    pub fn marks(&self) -> &[Mark] {
        &self.marks
    }

    pub fn push(&mut self, mark: Mark) {
        self.marks.push(mark);
        // Drawing something new ends the chance to redo what was undone.
        self.undone.clear();
    }

    pub fn undo(&mut self) -> bool {
        match self.marks.pop() {
            Some(m) => {
                self.undone.push(m);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.undone.pop() {
            Some(m) => {
                self.marks.push(m);
                true
            }
            None => false,
        }
    }

    /// Erases everything. This cannot be undone.
    pub fn clear(&mut self) {
        self.marks.clear();
        self.undone.clear();
    }
}

/// How far the picture is zoomed and which part shows. A point of the picture
/// (in the overlay's own pixels at no zoom) is at `point * zoom + offset` on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub zoom: f64,
    pub offset: Point,
}

pub const MAX_ZOOM: f64 = 8.0;

impl Default for View {
    fn default() -> Self {
        View { zoom: 1.0, offset: (0.0, 0.0) }
    }
}

impl View {
    /// Keeps the picture covering the whole `w`×`h` screen.
    fn clamp(&mut self, w: f64, h: f64) {
        self.zoom = self.zoom.clamp(1.0, MAX_ZOOM);
        self.offset.0 = self.offset.0.clamp(w - w * self.zoom, 0.0);
        self.offset.1 = self.offset.1.clamp(h - h * self.zoom, 0.0);
    }

    /// Zooms by `factor` around the screen point `at`, which stays where it is.
    pub fn zoom_at(&mut self, at: Point, factor: f64, w: f64, h: f64) {
        let before = self.to_picture(at);
        self.zoom = (self.zoom * factor).clamp(1.0, MAX_ZOOM);
        self.offset = (at.0 - before.0 * self.zoom, at.1 - before.1 * self.zoom);
        self.clamp(w, h);
    }

    /// Moves the picture by `delta` screen pixels.
    pub fn pan(&mut self, delta: Point, w: f64, h: f64) {
        self.offset = (self.offset.0 + delta.0, self.offset.1 + delta.1);
        self.clamp(w, h);
    }

    pub fn reset(&mut self) {
        *self = View::default();
    }

    /// A screen point as a point of the picture.
    pub fn to_picture(self, screen: Point) -> Point {
        ((screen.0 - self.offset.0) / self.zoom, (screen.1 - self.offset.1) / self.zoom)
    }
}

/// The two points that make the wings of an arrow head at `to`, for a shaft from `from`.
pub fn arrow_head(from: Point, to: Point, size: f64) -> [Point; 2] {
    let angle = (to.1 - from.1).atan2(to.0 - from.0);
    let spread = 0.45; // about 26 degrees either side
    let wing = |a: f64| (to.0 - size * (angle + a).cos(), to.1 - size * (angle + a).sin());
    [wing(spread), wing(-spread)]
}

/// The font size of a text mark made with a pen of `width`.
pub fn text_size(width: f64) -> f64 {
    16.0 + width * 4.0
}

/// A corner-to-corner box as (x, y, width, height), whichever way it was dragged.
pub fn bounds(a: Point, b: Point) -> (f64, f64, f64, f64) {
    (a.0.min(b.0), a.1.min(b.1), (a.0 - b.0).abs(), (a.1 - b.1).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(x: f64) -> Mark {
        Mark { shape: Shape::Line((0.0, 0.0), (x, 0.0)), color: (1.0, 0.0, 0.0), width: 4.0, highlight: false }
    }

    #[test]
    fn modifier_keys_choose_the_tool() {
        assert_eq!(Tool::from_keys(false, false, false), Tool::Pen);
        assert_eq!(Tool::from_keys(true, false, false), Tool::Line);
        assert_eq!(Tool::from_keys(false, true, false), Tool::Rect);
        assert_eq!(Tool::from_keys(true, true, false), Tool::Arrow);
        assert_eq!(Tool::from_keys(false, false, true), Tool::Ellipse);
    }

    #[test]
    fn undo_and_redo_walk_the_marks_and_new_drawing_forgets_the_redo() {
        let mut c = Canvas::default();
        assert!(!c.undo() && !c.redo());
        c.push(mark(1.0));
        c.push(mark(2.0));
        assert!(c.undo());
        assert_eq!(c.marks().len(), 1);
        assert!(c.redo());
        assert_eq!(c.marks().len(), 2);
        assert!(c.undo());
        c.push(mark(3.0));
        assert!(!c.redo(), "drawing again ends the redo");
        c.clear();
        assert!(c.marks().is_empty() && !c.undo());
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer_still() {
        let (w, h) = (1000.0, 800.0);
        let mut v = View::default();
        let at = (700.0, 300.0);
        let picture_point = v.to_picture(at);
        v.zoom_at(at, 2.0, w, h);
        assert_eq!(v.zoom, 2.0);
        let after = v.to_picture(at);
        assert!((after.0 - picture_point.0).abs() < 1e-9 && (after.1 - picture_point.1).abs() < 1e-9);
    }

    #[test]
    fn the_view_never_leaves_the_picture_or_zooms_out_past_one() {
        let (w, h) = (1000.0, 800.0);
        let mut v = View::default();
        v.zoom_at((0.0, 0.0), 0.5, w, h);
        assert_eq!(v, View::default(), "no zooming out past the whole picture");
        v.zoom_at((500.0, 400.0), 100.0, w, h);
        assert_eq!(v.zoom, MAX_ZOOM);
        v.pan((5000.0, 5000.0), w, h);
        assert_eq!(v.offset, (0.0, 0.0));
        v.pan((-50000.0, -50000.0), w, h);
        assert_eq!(v.offset, (w - w * MAX_ZOOM, h - h * MAX_ZOOM));
        v.reset();
        assert_eq!(v, View::default());
    }

    #[test]
    fn screen_points_map_back_to_the_picture() {
        let v = View { zoom: 2.0, offset: (-100.0, -50.0) };
        assert_eq!(v.to_picture((100.0, 150.0)), (100.0, 100.0));
        assert_eq!(View::default().to_picture((12.0, 34.0)), (12.0, 34.0));
    }

    #[test]
    fn an_arrow_head_points_back_along_the_shaft() {
        let [a, b] = arrow_head((0.0, 0.0), (100.0, 0.0), 20.0);
        assert!(a.0 < 100.0 && b.0 < 100.0, "the wings are behind the tip");
        assert!((a.1 + b.1).abs() < 1e-9, "and mirror each other");
        assert!(a.1 != b.1);
    }

    #[test]
    fn keys_pick_colors_and_unknown_keys_do_not() {
        assert!(color_for_key('r').is_some() && color_for_key('R') == color_for_key('r'));
        assert_eq!(color_for_key('x'), None);
        assert_eq!(color_name(color_for_key('b').unwrap()), "Blue");
        assert_eq!(color_name((0.1, 0.2, 0.3)), "Custom");
    }

    #[test]
    fn bounds_work_in_any_direction() {
        assert_eq!(bounds((10.0, 20.0), (4.0, 50.0)), (4.0, 20.0, 6.0, 30.0));
        assert_eq!(shape_for(Tool::Line, (0.0, 0.0), (1.0, 1.0), &[]), Shape::Line((0.0, 0.0), (1.0, 1.0)));
        assert_eq!(text_size(4.0), 32.0);
    }
}
