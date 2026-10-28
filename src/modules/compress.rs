//! Metadata stripping and lossy/lossless compression for common asset types.
//!
//! Originals are never modified: output goes to `<name>.min.<ext>` beside the input.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::system;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
    WebP,
    Mp4,
}

impl Format {
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "png" => Format::Png,
            "jpg" | "jpeg" => Format::Jpeg,
            "webp" => Format::WebP,
            "mp4" | "m4v" | "mov" => Format::Mp4,
            _ => return None,
        })
    }
}

#[derive(Debug)]
pub struct Report {
    pub output: PathBuf,
    pub input_size: u64,
    pub output_size: u64,
}

impl Report {
    pub fn change_percent(&self) -> f64 {
        if self.input_size == 0 {
            return 0.0;
        }
        (self.output_size as f64 / self.input_size as f64 - 1.0) * 100.0
    }
}

pub fn output_path(input: &Path) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let ext = input.extension().map(|e| e.to_string_lossy()).unwrap_or_default();
    input.with_file_name(format!("{stem}.min.{ext}"))
}

/// Maps quality 1-100 to an x264 CRF (lower CRF = better quality).
fn quality_to_crf(quality: u8) -> u8 {
    let q = quality.clamp(1, 100) as u32;
    (51 - (q * 33 / 100)) as u8 // 100 -> 18, 50 -> 35, 1 -> 51
}

/// Builds the external command that writes a compressed copy of `input` to `output`.
/// Tools that only work in place are pointed at `output`, which holds a copy.
pub fn build_command(format: Format, input: &Path, output: &Path, quality: u8) -> Command {
    let quality = quality.clamp(1, 100);
    match format {
        Format::Png => {
            let mut c = Command::new("oxipng");
            c.args(["-o", "3", "--strip", "all", "--force", "--quiet"]).arg(output);
            c
        }
        Format::Jpeg => {
            let mut c = Command::new("jpegoptim");
            c.args(["--strip-all", "--quiet", &format!("--max={quality}")]).arg(output);
            c
        }
        Format::WebP => {
            let mut c = Command::new("cwebp");
            c.args(["-quiet", "-metadata", "none", "-q", &quality.to_string()])
                .arg(input)
                .arg("-o")
                .arg(output);
            c
        }
        Format::Mp4 => {
            let mut c = Command::new("ffmpeg");
            c.args(["-y", "-loglevel", "error", "-i"])
                .arg(input)
                .args(["-map_metadata", "-1", "-c:v", "libx264", "-preset", "medium"])
                .args(["-crf", &quality_to_crf(quality).to_string()])
                .args(["-c:a", "aac", "-b:a", "128k", "-movflags", "+faststart"])
                .arg(output);
            c
        }
    }
}

fn works_in_place(format: Format) -> bool {
    matches!(format, Format::Png | Format::Jpeg)
}

pub fn compress(input: &Path, quality: u8) -> Result<Report> {
    let Some(format) = Format::from_path(input) else {
        bail!("unsupported file type: {}", input.display());
    };
    let input_size = std::fs::metadata(input)
        .with_context(|| format!("cannot read {}", input.display()))?
        .len();
    let output = output_path(input);
    if works_in_place(format) {
        std::fs::copy(input, &output)?;
    }
    if let Err(e) = system::run(&mut build_command(format, input, &output, quality)) {
        let _ = std::fs::remove_file(&output);
        return Err(e);
    }
    let output_size = std::fs::metadata(&output)?.len();
    Ok(Report { output, input_size, output_size })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(c: &Command) -> Vec<String> {
        c.get_args().map(|a| a.to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn detects_formats_case_insensitively() {
        assert_eq!(Format::from_path(Path::new("a/B.JPG")), Some(Format::Jpeg));
        assert_eq!(Format::from_path(Path::new("clip.mov")), Some(Format::Mp4));
        assert_eq!(Format::from_path(Path::new("doc.pdf")), None);
        assert_eq!(Format::from_path(Path::new("noext")), None);
    }

    #[test]
    fn output_sits_next_to_input() {
        assert_eq!(output_path(Path::new("/x/photo.jpeg")), Path::new("/x/photo.min.jpeg"));
    }

    #[test]
    fn crf_mapping_is_monotonic_and_bounded() {
        assert_eq!(quality_to_crf(100), 18);
        assert_eq!(quality_to_crf(0), quality_to_crf(1));
        assert!(quality_to_crf(80) < quality_to_crf(40));
    }

    #[test]
    fn commands_strip_metadata() {
        let (i, o) = (Path::new("in.x"), Path::new("out.x"));
        let png = build_command(Format::Png, i, o, 80);
        assert_eq!(png.get_program(), "oxipng");
        assert!(args(&png).contains(&"all".into()));
        assert_eq!(args(&png).last().unwrap(), "out.x");

        let jpg = build_command(Format::Jpeg, i, o, 70);
        assert!(args(&jpg).contains(&"--max=70".into()));

        let webp = build_command(Format::WebP, i, o, 150);
        assert!(args(&webp).windows(2).any(|w| w == ["-q", "100"]));

        let mp4 = build_command(Format::Mp4, i, o, 80);
        assert!(args(&mp4).windows(2).any(|w| w == ["-map_metadata", "-1"]));
    }
}
