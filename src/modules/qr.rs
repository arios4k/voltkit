//! QR codes: read them from screenshots or image files, and make them from text.

use std::path::Path;

use anyhow::{Context, Result, bail};
use gtk::gdk_pixbuf::{Colorspace, Pixbuf};

use super::ocr;

/// Reads every QR code in an image file.
pub fn decode_file(path: &Path) -> Result<Vec<String>> {
    let pixbuf = Pixbuf::from_file(path).with_context(|| format!("cannot open {}", path.display()))?;
    Ok(decode_pixbuf(&pixbuf))
}

pub fn decode_pixbuf(pixbuf: &Pixbuf) -> Vec<String> {
    let (w, h) = (pixbuf.width() as usize, pixbuf.height() as usize);
    let channels = pixbuf.n_channels() as usize;
    let stride = pixbuf.rowstride() as usize;
    let bytes = pixbuf.read_pixel_bytes();
    let luma = |x: usize, y: usize| {
        let i = y * stride + x * channels;
        let (r, g, b) = (bytes[i] as u32, bytes[i + 1] as u32, bytes[i + 2] as u32);
        ((r * 299 + g * 587 + b * 114) / 1000) as u8
    };
    let mut image = rqrr::PreparedImage::prepare_from_greyscale(w, h, luma);
    image.detect_grids().into_iter().filter_map(|grid| grid.decode().ok().map(|(_, text)| text)).collect()
}

/// Lets the user select a screen region and reads the QR codes in it.
pub async fn scan(parent: Option<&str>) -> Result<Vec<String>> {
    let image = ocr::capture_region(parent).await?;
    let path = image.clone();
    let codes = tokio::task::spawn_blocking(move || decode_file(&path)).await?;
    let _ = std::fs::remove_file(&image);
    let codes = codes?;
    if codes.is_empty() {
        bail!("no QR code found in the selected area");
    }
    Ok(codes)
}

/// A QR code as a square grid of modules, `true` for dark.
pub struct Matrix {
    pub width: usize,
    dark: Vec<bool>,
}

impl Matrix {
    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.width && self.dark[y * self.width + x]
    }
}

pub fn encode(text: &str) -> Result<Matrix> {
    encode_for(text, false)
}

/// Encodes `text`, with the highest error correction when a logo will cover
/// part of the code: up to 30% of it can then be lost and still read.
pub fn encode_for(text: &str, with_logo: bool) -> Result<Matrix> {
    if text.is_empty() {
        bail!("nothing to encode");
    }
    let level = if with_logo { qrcode::EcLevel::H } else { qrcode::EcLevel::M };
    let code = qrcode::QrCode::with_error_correction_level(text.as_bytes(), level)
        .context("text is too long for a QR code")?;
    let dark = code.to_colors().into_iter().map(|c| c == qrcode::Color::Dark).collect();
    Ok(Matrix { width: code.width(), dark })
}

/// Modules of light margin required around a QR code for scanners to find it.
const QUIET_ZONE: usize = 4;

/// Renders a QR code as black on white, `scale` pixels per module.
pub fn to_pixbuf(matrix: &Matrix, scale: usize) -> Pixbuf {
    let size = (matrix.width + 2 * QUIET_ZONE) * scale;
    let pixbuf = Pixbuf::new(Colorspace::Rgb, false, 8, size as i32, size as i32).expect("pixbuf allocation");
    pixbuf.fill(0xffffffff);
    for y in 0..matrix.width {
        for x in 0..matrix.width {
            if matrix.is_dark(x, y) {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let (px, py) = ((x + QUIET_ZONE) * scale + dx, (y + QUIET_ZONE) * scale + dy);
                        pixbuf.put_pixel(px as u32, py as u32, 0, 0, 0, 255);
                    }
                }
            }
        }
    }
    pixbuf
}

/// Logo sizes offered in the app (small, medium, large), as a share of the
/// code's width. Even the largest covers about 4% of the code, far below the
/// 30% that the highest error correction can recover.
pub const LOGO_SIZES: [f64; 3] = [0.12, 0.16, 0.20];
/// The smallest logo tried when a larger one makes the code unreadable.
const MIN_LOGO: f64 = 0.08;
/// Pixels per module for the read-back check, small and large, so the code
/// is known to read at different sizes.
const CHECK_SCALES: [usize; 2] = [4, 8];

/// Image formats accepted as a logo, as named by GdkPixbuf.
pub const LOGO_FORMATS: [&str; 4] = ["png", "jpeg", "webp", "svg"];
const LOGO_MAX_BYTES: u64 = 5 * 1024 * 1024;
const LOGO_MIN_SIDE: i32 = 16;
const LOGO_MAX_SIDE: i32 = 8192;
/// Logos are loaded at this size, enough for the largest saved code.
const LOGO_PIXELS: i32 = 512;

/// Loads an image to use as a logo, refusing anything that isn't a PNG, JPEG,
/// WebP or SVG of reasonable size and shape.
pub fn load_logo(path: &Path) -> Result<Pixbuf> {
    let bytes = std::fs::metadata(path).with_context(|| format!("cannot open {}", path.display()))?.len();
    if bytes > LOGO_MAX_BYTES {
        bail!("the image is too large ({} MB); use one under 5 MB", bytes / (1024 * 1024));
    }
    let Some((format, w, h)) = Pixbuf::file_info(path) else {
        bail!("not an image Voltkit can read; use a PNG, JPEG, WebP or SVG");
    };
    let name = format.name().map(|n| n.to_string()).unwrap_or_default();
    if !LOGO_FORMATS.contains(&name.as_str()) {
        bail!("{} images can't be used as a logo; use a PNG, JPEG, WebP or SVG", name.to_uppercase());
    }
    if name != "svg" && (w < LOGO_MIN_SIDE || h < LOGO_MIN_SIDE) {
        bail!("the image is too small ({w}×{h}); use one at least {LOGO_MIN_SIDE}×{LOGO_MIN_SIDE} pixels");
    }
    if w > LOGO_MAX_SIDE || h > LOGO_MAX_SIDE {
        bail!("the image is too large ({w}×{h}); use one under {LOGO_MAX_SIDE}×{LOGO_MAX_SIDE} pixels");
    }
    if w.max(h) > 2 * w.min(h).max(1) {
        bail!("the image is too wide or tall ({w}×{h}); use a square-ish logo");
    }
    Pixbuf::from_file_at_scale(path, LOGO_PIXELS, LOGO_PIXELS, true)
        .with_context(|| format!("cannot read {}", path.display()))
}

/// Renders a QR code with `logo` in the middle, on a white square that
/// covers about `fraction` of the code's width.
pub fn to_pixbuf_with_logo(matrix: &Matrix, scale: usize, logo: &Pixbuf, fraction: f64) -> Pixbuf {
    let pixbuf = to_pixbuf(matrix, scale);
    let n = matrix.width;
    // Whole modules, centered: same parity as the code's width.
    let mut box_modules = ((n as f64 * fraction).round() as usize).max(3);
    if box_modules % 2 != n % 2 {
        box_modules += 1;
    }
    let start = (QUIET_ZONE + (n - box_modules) / 2) * scale;
    let side = box_modules * scale;
    pixbuf.new_subpixbuf(start as i32, start as i32, side as i32, side as i32).fill(0xffffffff);

    // The logo keeps its proportions inside the square, with a module of white around it.
    let inner = (box_modules - 2) * scale;
    let ratio = (inner as f64 / logo.width() as f64).min(inner as f64 / logo.height() as f64);
    let (w, h) = (((logo.width() as f64 * ratio).round() as i32).max(1), ((logo.height() as f64 * ratio).round() as i32).max(1));
    let (x, y) = (start as i32 + (side as i32 - w) / 2, start as i32 + (side as i32 - h) / 2);
    logo.composite(&pixbuf, x, y, w, h, x as f64, y as f64, ratio, ratio, gtk::gdk_pixbuf::InterpType::Bilinear, 255);
    pixbuf
}

/// The largest logo size, up to `fraction`, at which the code still reads
/// back as `text`. `None` if even the smallest logo breaks it.
pub fn checked_logo_size(matrix: &Matrix, text: &str, logo: &Pixbuf, fraction: f64) -> Option<f64> {
    let mut f = fraction;
    while f >= MIN_LOGO - 1e-9 {
        let reads = CHECK_SCALES.iter().all(|&scale| {
            let image = to_pixbuf_with_logo(matrix, scale, logo, f);
            decode_pixbuf(&image).iter().any(|t| t == text)
        });
        if reads {
            return Some(f);
        }
        f -= 0.02;
    }
    None
}

/// Renders a QR code with Unicode half blocks, two module rows per text line.
/// Light modules are drawn as blocks, which scans as normal on the usual dark
/// terminal background (most phone scanners also read the inverted version).
pub fn to_terminal(matrix: &Matrix) -> String {
    let span = matrix.width + 2 * QUIET_ZONE;
    let light = |x: usize, y: usize| {
        let (x, y) = (x as isize - QUIET_ZONE as isize, y as isize - QUIET_ZONE as isize);
        x < 0 || y < 0 || !matrix.is_dark(x as usize, y as usize)
    };
    let mut out = String::new();
    for y in (0..span).step_by(2) {
        for x in 0..span {
            let top = light(x, y);
            let bottom = y + 1 < span && light(x, y + 1);
            out.push(match (top, bottom) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        out.push('\n');
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiSecurity {
    Wpa,
    Wep,
    Open,
}

/// The text phones expect in a QR code to join a Wi-Fi network.
pub fn wifi_payload(ssid: &str, password: &str, security: WifiSecurity) -> String {
    let escape = |s: &str| {
        s.chars().fold(String::new(), |mut out, c| {
            if matches!(c, '\\' | ';' | ',' | ':' | '"') {
                out.push('\\');
            }
            out.push(c);
            out
        })
    };
    match security {
        WifiSecurity::Open => format!("WIFI:T:nopass;S:{};;", escape(ssid)),
        WifiSecurity::Wpa => format!("WIFI:T:WPA;S:{};P:{};;", escape(ssid), escape(password)),
        WifiSecurity::Wep => format!("WIFI:T:WEP;S:{};P:{};;", escape(ssid), escape(password)),
    }
}

pub fn is_link(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("https://") || t.starts_with("http://")) && !t.contains(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_codes_decode_again() {
        for text in ["https://github.com/arios4k/voltkit", "Hello, Linux! ⚡", "WIFI:T:WPA;S:Home;P:secret;;"] {
            let pixbuf = to_pixbuf(&encode(text).unwrap(), 6);
            assert_eq!(decode_pixbuf(&pixbuf), vec![text.to_owned()]);
        }
    }

    /// A colorful logo with transparent corners, like a typical app icon.
    fn sample_logo() -> Pixbuf {
        let logo = Pixbuf::new(Colorspace::Rgb, true, 8, 64, 64).unwrap();
        logo.fill(0x00000000);
        for y in 0..64u32 {
            for x in 0..64u32 {
                let (dx, dy) = (x as i32 - 32, y as i32 - 32);
                if dx * dx + dy * dy < 30 * 30 {
                    logo.put_pixel(x, y, (x * 4) as u8, 40, (y * 4) as u8, 255);
                }
            }
        }
        logo
    }

    #[test]
    fn codes_with_a_logo_still_decode() {
        let logo = sample_logo();
        let long = "https://example.com/".to_owned() + &"a".repeat(150);
        for text in ["https://github.com/arios4k/voltkit", "WIFI:T:WPA;S:Home;P:secret;;", "hi", long.as_str()] {
            let m = encode_for(text, true).unwrap();
            for size in LOGO_SIZES {
                let used = checked_logo_size(&m, text, &logo, size).expect("a logo size that scans");
                assert!(used >= MIN_LOGO - 1e-9 && used <= size + 1e-9);
                let image = to_pixbuf_with_logo(&m, 8, &logo, used);
                assert_eq!(decode_pixbuf(&image), vec![text.to_owned()], "{text} at {used}");
            }
        }
    }

    #[test]
    fn logo_stays_small() {
        // The square under the largest logo is a small share of the code.
        for text in ["hi", "https://github.com/arios4k/voltkit"] {
            let m = encode_for(text, true).unwrap();
            let image = to_pixbuf_with_logo(&m, 1, &sample_logo(), LOGO_SIZES[2]);
            let code = (m.width * m.width) as f64;
            let white_box = {
                let mut side = ((m.width as f64 * LOGO_SIZES[2]).round() as usize).max(3);
                if side % 2 != m.width % 2 {
                    side += 1;
                }
                (side * side) as f64
            };
            assert!(white_box / code < 0.08, "{text}: {}", white_box / code);
            assert!(image.width() > 0);
        }
    }

    #[test]
    fn rejects_unsuitable_logos() {
        let dir = std::env::temp_dir().join(format!("voltkit-logo-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let save = |name: &str, w: i32, h: i32, format: &str| {
            let path = dir.join(name);
            let p = Pixbuf::new(Colorspace::Rgb, false, 8, w, h).unwrap();
            p.fill(0x3366ccff);
            p.savev(&path, format, &[]).unwrap();
            path
        };
        assert!(load_logo(&save("ok.png", 64, 64, "png")).is_ok());
        assert!(load_logo(&save("ok.jpg", 100, 60, "jpeg")).is_ok());
        assert!(load_logo(&save("tiny.png", 8, 8, "png")).is_err());
        assert!(load_logo(&save("banner.png", 300, 50, "png")).is_err());
        assert!(load_logo(&save("bitmap.bmp", 64, 64, "bmp")).is_err());
        let text = dir.join("notes.png");
        std::fs::write(&text, "not an image").unwrap();
        assert!(load_logo(&text).is_err());
        assert!(load_logo(&dir.join("missing.png")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn logo_covers_the_middle() {
        let m = encode_for("voltkit", true).unwrap();
        let image = to_pixbuf_with_logo(&m, 4, &sample_logo(), LOGO_SIZES[2]);
        let c = image.width() as usize / 2;
        let bytes = image.read_pixel_bytes();
        let i = c * image.rowstride() as usize + c * image.n_channels() as usize;
        // The logo's center color, not black or white.
        assert_eq!(bytes[i + 1], 40);
    }

    #[test]
    fn blank_image_has_no_codes() {
        let pixbuf = Pixbuf::new(Colorspace::Rgb, false, 8, 200, 200).unwrap();
        pixbuf.fill(0xffffffff);
        assert!(decode_pixbuf(&pixbuf).is_empty());
    }

    #[test]
    fn rejects_empty_and_oversized_text() {
        assert!(encode("").is_err());
        assert!(encode(&"x".repeat(5000)).is_err());
    }

    #[test]
    fn wifi_payload_escapes_special_characters() {
        assert_eq!(wifi_payload("Café;Net", "p:a\"ss", WifiSecurity::Wpa), r#"WIFI:T:WPA;S:Café\;Net;P:p\:a\"ss;;"#);
        assert_eq!(wifi_payload("Guest", "ignored", WifiSecurity::Open), "WIFI:T:nopass;S:Guest;;");
    }

    #[test]
    fn terminal_rendering_is_square() {
        let m = encode("voltkit").unwrap();
        let out = to_terminal(&m);
        let span = m.width + 2 * QUIET_ZONE;
        assert_eq!(out.lines().count(), span.div_ceil(2));
        assert!(out.lines().all(|l| l.chars().count() == span));
    }

    #[test]
    fn detects_links() {
        assert!(is_link("https://example.com/a?b=c"));
        assert!(!is_link("visit https://example.com"));
        assert!(!is_link("ftp://example.com"));
    }
}
