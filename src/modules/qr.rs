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
    if text.is_empty() {
        bail!("nothing to encode");
    }
    let code = qrcode::QrCode::new(text.as_bytes()).context("text is too long for a QR code")?;
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
