//! Color parsing, conversion and WCAG 2.x contrast checking.

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const WHITE: Rgb = Rgb { r: 255, g: 255, b: 255 };
    pub const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };

    /// Parses `#rgb`, `#rrggbb` (with or without `#`) or `rgb(r, g, b)`.
    pub fn parse(input: &str) -> Result<Self> {
        let s = input.trim().to_ascii_lowercase();
        if let Some(inner) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
            let parts: Vec<_> = inner.split(',').map(|p| p.trim().parse::<u8>()).collect();
            return match parts.as_slice() {
                [Ok(r), Ok(g), Ok(b)] => Ok(Rgb { r: *r, g: *g, b: *b }),
                _ => bail!("expected rgb(0-255, 0-255, 0-255), got {input:?}"),
            };
        }
        let hex = s.strip_prefix('#').unwrap_or(&s);
        let expanded: String = match hex.len() {
            3 => hex.chars().flat_map(|c| [c, c]).collect(),
            6 => hex.to_owned(),
            _ => bail!("unrecognized color {input:?}"),
        };
        let v = u32::from_str_radix(&expanded, 16)
            .map_err(|_| anyhow::anyhow!("invalid hex color {input:?}"))?;
        Ok(Rgb { r: (v >> 16) as u8, g: (v >> 8) as u8, b: v as u8 })
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn to_rgb_string(self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    /// Returns (hue 0-360, saturation 0-100, lightness 0-100).
    pub fn to_hsl(self) -> (f64, f64, f64) {
        let [r, g, b] = [self.r, self.g, self.b].map(|c| c as f64 / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        if d == 0.0 {
            return (0.0, 0.0, l * 100.0);
        }
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let h = if max == r {
            ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        } * 60.0;
        (h, s * 100.0, l * 100.0)
    }

    pub fn to_hsl_string(self) -> String {
        let (h, s, l) = self.to_hsl();
        format!("hsl({:.0}, {:.0}%, {:.0}%)", h, s, l)
    }

    /// WCAG relative luminance in 0.0..=1.0.
    pub fn luminance(self) -> f64 {
        let lin = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// WCAG contrast ratio between 1.0 and 21.0.
    pub fn contrast_ratio(self, other: Rgb) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

/// Human-readable WCAG conformance level for normal-size text.
pub fn wcag_rating(ratio: f64) -> &'static str {
    match ratio {
        r if r >= 7.0 => "AAA",
        r if r >= 4.5 => "AA",
        r if r >= 3.0 => "AA Large",
        _ => "Fail",
    }
}

/// Recently used colors, newest first, shared by the GUI and the daemon.
pub mod history {
    use std::path::PathBuf;

    use anyhow::Result;

    use super::Rgb;

    const MAX: usize = 32;

    fn path() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("voltkit").join("colors.txt"))
    }

    pub fn load() -> Vec<Rgb> {
        let Some(text) = path().and_then(|p| std::fs::read_to_string(p).ok()) else {
            return Vec::new();
        };
        text.lines().filter_map(|l| Rgb::parse(l).ok()).take(MAX).collect()
    }

    pub fn push(color: Rgb) -> Result<()> {
        let Some(path) = path() else { return Ok(()) };
        let mut colors = load();
        insert(&mut colors, color);
        std::fs::create_dir_all(path.parent().unwrap())?;
        let text: String = colors.iter().map(|c| c.to_hex() + "\n").collect();
        std::fs::write(path, text)?;
        Ok(())
    }

    /// Moves `color` to the front, dropping duplicates and old entries.
    pub(super) fn insert(colors: &mut Vec<Rgb>, color: Rgb) {
        colors.retain(|c| *c != color);
        colors.insert(0, color);
        colors.truncate(MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_dedupes_and_caps() {
        let mut list = vec![Rgb::WHITE, Rgb::BLACK];
        history::insert(&mut list, Rgb::BLACK);
        assert_eq!(list, vec![Rgb::BLACK, Rgb::WHITE]);
        for i in 0..40 {
            history::insert(&mut list, Rgb { r: i, g: 0, b: 0 });
        }
        assert_eq!(list.len(), 32);
        assert_eq!(list[0], Rgb { r: 39, g: 0, b: 0 });
    }

    #[test]
    fn parses_hex_forms() {
        assert_eq!(Rgb::parse("#ff8800").unwrap(), Rgb { r: 255, g: 136, b: 0 });
        assert_eq!(Rgb::parse("F80").unwrap(), Rgb { r: 255, g: 136, b: 0 });
        assert_eq!(Rgb::parse(" #FFF ").unwrap(), Rgb::WHITE);
    }

    #[test]
    fn parses_rgb_function() {
        assert_eq!(Rgb::parse("rgb(10, 20, 30)").unwrap(), Rgb { r: 10, g: 20, b: 30 });
        assert!(Rgb::parse("rgb(300, 0, 0)").is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(Rgb::parse("#12345").is_err());
        assert!(Rgb::parse("#zzzzzz").is_err());
    }

    #[test]
    fn formats() {
        let c = Rgb::parse("#3584e4").unwrap();
        assert_eq!(c.to_hex(), "#3584E4");
        assert_eq!(c.to_rgb_string(), "rgb(53, 132, 228)");
        assert_eq!(c.to_hsl_string(), "hsl(213, 76%, 55%)");
    }

    #[test]
    fn hsl_of_grays_and_primaries() {
        assert_eq!(Rgb::BLACK.to_hsl(), (0.0, 0.0, 0.0));
        let (h, s, l) = Rgb::parse("#00ff00").unwrap().to_hsl();
        assert_eq!((h.round(), s.round(), l.round()), (120.0, 100.0, 50.0));
    }

    #[test]
    fn contrast_matches_wcag() {
        let ratio = Rgb::BLACK.contrast_ratio(Rgb::WHITE);
        assert!((ratio - 21.0).abs() < 1e-9);
        let gray = Rgb::parse("#767676").unwrap();
        assert_eq!(wcag_rating(gray.contrast_ratio(Rgb::WHITE)), "AA");
        assert_eq!(wcag_rating(1.5), "Fail");
    }
}
