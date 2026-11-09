//! Image Resizer: bulk-resize pictures with GdkPixbuf (which Voltkit already
//! uses), without touching the originals.
//!
//! The copy is written beside the original as `name (1920x1080).jpg`. The
//! picture's own rotation (EXIF orientation) is applied, so a photo taken
//! sideways comes out upright; other metadata is not copied. Resizing needs no
//! display, so it works in a terminal and in scripts.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use gtk::gdk_pixbuf::{InterpType, Pixbuf};

use crate::i18n::tr;

/// How the picture meets the size you asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Keeps the shape and fits inside the size. The default.
    #[default]
    Fit,
    /// Keeps the shape, fills the size, and trims what overflows from the middle.
    Fill,
    /// Becomes exactly that size, even if it distorts the picture.
    Stretch,
}

/// The size to aim for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    /// At most this wide and this high (either may be left open).
    Box { width: Option<u32>, height: Option<u32> },
    /// The longest side becomes this many pixels.
    LongestSide(u32),
    /// A share of the original size, in percent.
    Percent(u32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    /// The same format as the original.
    #[default]
    Same,
    Png,
    Jpeg,
    WebP,
}

impl Format {
    fn loader(self, source: &Path) -> Option<&'static str> {
        match self {
            Format::Png => Some("png"),
            Format::Jpeg => Some("jpeg"),
            Format::WebP => Some("webp"),
            Format::Same => match extension(source).as_str() {
                "png" => Some("png"),
                "jpg" | "jpeg" => Some("jpeg"),
                "webp" => Some("webp"),
                "tif" | "tiff" => Some("tiff"),
                "bmp" => Some("bmp"),
                _ => None,
            },
        }
    }

    fn extension(loader: &str) -> &'static str {
        match loader {
            "jpeg" => "jpg",
            "tiff" => "tif",
            "webp" => "webp",
            "bmp" => "bmp",
            _ => "png",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub size: Size,
    pub mode: Mode,
    /// Never make a picture bigger than it is.
    pub no_enlarge: bool,
    pub format: Format,
    /// JPEG and WebP quality, 1 to 100.
    pub quality: u8,
}

impl Default for Options {
    fn default() -> Self {
        Options { size: Size::LongestSide(1920), mode: Mode::Fit, no_enlarge: true, format: Format::Same, quality: 90 }
    }
}

fn extension(path: &Path) -> String {
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Whether the file looks like a picture Voltkit can resize.
pub fn is_picture(path: &Path) -> bool {
    matches!(extension(path).as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "tif" | "tiff")
}

fn scale(value: u32, num: u64, den: u64) -> u32 {
    (((u64::from(value) * num) + den / 2) / den.max(1)).max(1) as u32
}

/// The size of the resized picture, and for [`Mode::Fill`] the size it is first
/// scaled to before the overflow is trimmed away. Pure arithmetic.
pub fn target_size(w: u32, h: u32, o: &Options) -> (u32, u32) {
    let (w, h) = (w.max(1), h.max(1));
    let out = match o.size {
        Size::Percent(p) => (scale(w, u64::from(p.clamp(1, 1000)), 100), scale(h, u64::from(p.clamp(1, 1000)), 100)),
        Size::LongestSide(n) => {
            let n = n.max(1);
            if w >= h { (n, scale(h, u64::from(n), u64::from(w))) } else { (scale(w, u64::from(n), u64::from(h)), n) }
        }
        Size::Box { width, height } => match (width, height, o.mode) {
            (None, None, _) => (w, h),
            (Some(bw), None, _) => (bw.max(1), scale(h, u64::from(bw.max(1)), u64::from(w))),
            (None, Some(bh), _) => (scale(w, u64::from(bh.max(1)), u64::from(h)), bh.max(1)),
            (Some(bw), Some(bh), Mode::Stretch) => (bw.max(1), bh.max(1)),
            (Some(bw), Some(bh), mode) => {
                let (bw, bh) = (bw.max(1), bh.max(1));
                // Compare the shapes by cross-multiplying: no rounding.
                let wider = u64::from(w) * u64::from(bh) > u64::from(h) * u64::from(bw);
                let by_width = (mode == Mode::Fit) == wider;
                if by_width { (bw, scale(h, u64::from(bw), u64::from(w))) } else { (scale(w, u64::from(bh), u64::from(h)), bh) }
            }
        },
    };
    if o.no_enlarge && (out.0 > w || out.1 > h) && !matches!((o.size, o.mode), (Size::Box { width: Some(_), height: Some(_) }, Mode::Stretch | Mode::Fill)) {
        return (w, h);
    }
    out
}

/// The size of the file that is written: the box for Fill and Stretch, otherwise the scaled size.
pub fn final_size(w: u32, h: u32, o: &Options) -> (u32, u32) {
    match (o.size, o.mode) {
        (Size::Box { width: Some(bw), height: Some(bh) }, Mode::Fill | Mode::Stretch) if !(o.no_enlarge && (bw > w || bh > h)) => (bw.max(1), bh.max(1)),
        (Size::Box { width: Some(_), height: Some(_) }, Mode::Fill | Mode::Stretch) => (w, h),
        _ => target_size(w, h, o),
    }
}

/// Where the copy goes: `name (WxH).ext`, then `name (WxH) 2.ext` if that is taken.
pub fn output_path(source: &Path, w: u32, h: u32, loader: &str) -> PathBuf {
    let stem = source.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "image".into());
    let ext = Format::extension(loader);
    let dir = source.parent().unwrap_or(Path::new("."));
    let first = dir.join(format!("{stem} ({w}x{h}).{ext}"));
    if !first.exists() {
        return first;
    }
    (2..).map(|n| dir.join(format!("{stem} ({w}x{h}) {n}.{ext}"))).find(|p| !p.exists()).unwrap_or(first)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub output: PathBuf,
    pub from: (u32, u32),
    pub to: (u32, u32),
    pub bytes_before: u64,
    pub bytes_after: u64,
}

/// Which formats this system can write.
pub fn can_write(loader: &str) -> bool {
    Pixbuf::formats().iter().any(|f| f.is_writable() && f.name().is_some_and(|n| n.as_str() == loader))
}

/// Resizes one picture. The original is never changed.
pub fn resize(source: &Path, o: &Options) -> Result<Report> {
    let loader = o.format.loader(source).with_context(|| format!("{}: {}", source.display(), tr("this kind of picture can't be written")))?;
    if !can_write(loader) {
        bail!("{}", crate::i18n::fill(tr("This computer can't write {} pictures. Install its pixbuf loader (for WebP: webp-pixbuf-loader)."), &[&loader.to_uppercase()]));
    }
    let loaded = Pixbuf::from_file(source).with_context(|| format!("{}: {}", source.display(), tr("can't read the picture")))?;
    let pixbuf = loaded.apply_embedded_orientation().unwrap_or(loaded);
    let (w, h) = (pixbuf.width() as u32, pixbuf.height() as u32);
    let (tw, th) = target_size(w, h, o);
    let (fw, fh) = final_size(w, h, o);
    let scaled = if (tw, th) == (w, h) {
        pixbuf.clone()
    } else {
        pixbuf.scale_simple(tw as i32, th as i32, InterpType::Hyper).context(tr("the picture is too large to resize"))?
    };
    let result = if (fw, fh) == (tw, th) {
        scaled
    } else {
        // Fill: trim the overflow from the middle.
        let (x, y) = (((tw.saturating_sub(fw)) / 2) as i32, ((th.saturating_sub(fh)) / 2) as i32);
        scaled.new_subpixbuf(x, y, fw.min(tw) as i32, fh.min(th) as i32).copy().context(tr("the picture is too large to resize"))?
    };
    let output = output_path(source, result.width() as u32, result.height() as u32, loader);
    let quality = o.quality.clamp(1, 100).to_string();
    match loader {
        "jpeg" | "webp" => result.savev(&output, loader, &[("quality", quality.as_str())]),
        _ => result.savev(&output, loader, &[]),
    }
    .with_context(|| format!("{}: {}", output.display(), tr("can't save the picture")))?;
    Ok(Report {
        output: output.clone(),
        from: (w, h),
        to: (result.width() as u32, result.height() as u32),
        bytes_before: std::fs::metadata(source).map(|m| m.len()).unwrap_or(0),
        bytes_after: std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(size: Size, mode: Mode, no_enlarge: bool) -> Options {
        Options { size, mode, no_enlarge, ..Options::default() }
    }

    #[test]
    fn longest_side_keeps_the_shape() {
        let opts = o(Size::LongestSide(1000), Mode::Fit, false);
        assert_eq!(target_size(4000, 3000, &opts), (1000, 750));
        assert_eq!(target_size(3000, 4000, &opts), (750, 1000));
        assert_eq!(target_size(500, 500, &opts), (1000, 1000));
        assert_eq!(target_size(500, 400, &o(Size::LongestSide(1000), Mode::Fit, true)), (500, 400), "never enlarged");
    }

    #[test]
    fn percent_scales_both_sides() {
        assert_eq!(target_size(1000, 600, &o(Size::Percent(50), Mode::Fit, false)), (500, 300));
        assert_eq!(target_size(1000, 600, &o(Size::Percent(200), Mode::Fit, true)), (1000, 600));
        assert_eq!(target_size(3, 3, &o(Size::Percent(10), Mode::Fit, false)), (1, 1), "never zero");
    }

    #[test]
    fn a_box_fits_fills_or_stretches() {
        let b = Size::Box { width: Some(800), height: Some(600) };
        // 4:3 into 16:9-ish box: fit is limited by height, fill by width.
        assert_eq!(target_size(4000, 3000, &o(b, Mode::Fit, false)), (800, 600));
        let wide = Size::Box { width: Some(1000), height: Some(500) };
        assert_eq!(target_size(4000, 3000, &o(wide, Mode::Fit, false)), (667, 500));
        assert_eq!(target_size(4000, 3000, &o(wide, Mode::Fill, false)), (1000, 750));
        assert_eq!(final_size(4000, 3000, &o(wide, Mode::Fill, false)), (1000, 500));
        assert_eq!(target_size(4000, 3000, &o(wide, Mode::Stretch, false)), (1000, 500));
        assert_eq!(final_size(4000, 3000, &o(wide, Mode::Stretch, false)), (1000, 500));
    }

    #[test]
    fn one_open_side_follows_the_shape() {
        let only_w = Size::Box { width: Some(600), height: None };
        assert_eq!(target_size(1200, 800, &o(only_w, Mode::Fit, false)), (600, 400));
        let only_h = Size::Box { width: None, height: Some(400) };
        assert_eq!(target_size(1200, 800, &o(only_h, Mode::Fit, false)), (600, 400));
        assert_eq!(target_size(1200, 800, &o(Size::Box { width: None, height: None }, Mode::Fit, false)), (1200, 800));
    }

    #[test]
    fn names_never_overwrite() {
        let dir = std::env::temp_dir().join(format!("voltkit-resize-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("photo.jpg");
        assert_eq!(output_path(&src, 800, 600, "jpeg"), dir.join("photo (800x600).jpg"));
        std::fs::write(dir.join("photo (800x600).jpg"), "x").unwrap();
        assert_eq!(output_path(&src, 800, 600, "jpeg"), dir.join("photo (800x600) 2.jpg"));
    }

    fn picture(dir: &Path, name: &str, w: i32, h: i32) -> PathBuf {
        let pb = Pixbuf::new(gtk::gdk_pixbuf::Colorspace::Rgb, false, 8, w, h).unwrap();
        pb.fill(0x3366ccff);
        let path = dir.join(name);
        pb.savev(&path, if name.ends_with("png") { "png" } else { "jpeg" }, &[]).unwrap();
        path
    }

    #[test]
    fn it_resizes_a_real_picture_and_leaves_the_original() {
        let dir = std::env::temp_dir().join(format!("voltkit-resize-real-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = picture(&dir, "big.png", 400, 300);
        let before = std::fs::read(&src).unwrap();
        let report = resize(&src, &o(Size::LongestSide(100), Mode::Fit, true)).unwrap();
        assert_eq!(report.from, (400, 300));
        assert_eq!(report.to, (100, 75));
        assert_eq!(report.output, dir.join("big (100x75).png"));
        let out = Pixbuf::from_file(&report.output).unwrap();
        assert_eq!((out.width(), out.height()), (100, 75));
        assert_eq!(std::fs::read(&src).unwrap(), before, "the original is untouched");
        // Converting to JPEG, filling a box.
        let jpg = Options { size: Size::Box { width: Some(80), height: Some(80) }, mode: Mode::Fill, format: Format::Jpeg, ..Options::default() };
        let r = resize(&src, &jpg).unwrap();
        assert_eq!(r.to, (80, 80));
        assert_eq!(r.output.extension().unwrap(), "jpg");
    }

    #[test]
    fn it_rejects_things_that_are_not_pictures() {
        let dir = std::env::temp_dir().join(format!("voltkit-resize-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let text = dir.join("notes.txt");
        std::fs::write(&text, "hello").unwrap();
        assert!(resize(&text, &Options::default()).is_err());
        let fake = dir.join("fake.png");
        std::fs::write(&fake, "not a png").unwrap();
        assert!(resize(&fake, &Options::default()).is_err());
        assert!(is_picture(&fake) && !is_picture(&text));
    }
}
