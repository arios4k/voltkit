mod app;
mod config;
mod modules;
mod system;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use modules::{color, compress, ocr, snippets};

/// PowerToys-style micro-utilities for Linux, in one binary.
#[derive(Parser)]
#[command(name = "powerlinux", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Select a screen region and copy the recognized text to the clipboard
    Ocr {
        /// Tesseract language code(s), e.g. "eng" or "eng+deu"
        #[arg(long, default_value = "eng")]
        lang: String,
    },
    /// Convert a color (#hex or rgb(...)) and report WCAG contrast
    Color { value: String },
    /// Strip metadata and compress PNG, JPEG, WebP or MP4 files
    Compress {
        files: Vec<PathBuf>,
        /// Quality from 1 (smallest) to 100 (best)
        #[arg(long, default_value_t = 80)]
        quality: u8,
    },
    /// Expand a snippet template, e.g. "Sent {date:%A}: {clipboard}"
    Expand {
        template: String,
        /// Macro parameters as name=value
        #[arg(long = "arg", value_parser = parse_kv)]
        args: Vec<(String, String)>,
    },
}

fn parse_kv(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .ok_or_else(|| format!("expected name=value, got {s:?}"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::Config::load()?;

    match cli.command {
        None => app::run(config),
        Some(Command::Ocr { lang }) => {
            let text = ocr::capture_and_recognize(&lang)?;
            system::copy_to_clipboard(&text)?;
            println!("{text}");
        }
        Some(Command::Color { value }) => {
            let c = color::Rgb::parse(&value)?;
            println!("HEX  {}", c.to_hex());
            println!("RGB  {}", c.to_rgb_string());
            println!("HSL  {}", c.to_hsl_string());
            for (name, bg) in [("white", color::Rgb::WHITE), ("black", color::Rgb::BLACK)] {
                let ratio = c.contrast_ratio(bg);
                println!("on {name:<5} {ratio:>5.2}:1  {}", color::wcag_rating(ratio));
            }
        }
        Some(Command::Compress { files, quality }) => {
            for file in files {
                let r = compress::compress(&file, quality)?;
                println!(
                    "{} -> {} ({} -> {}, {:+.1}%)",
                    file.display(),
                    r.output.display(),
                    r.input_size,
                    r.output_size,
                    r.change_percent()
                );
            }
        }
        Some(Command::Expand { template, args }) => {
            let ctx = snippets::Context {
                clipboard: system::read_clipboard().ok(),
                args: args.into_iter().collect(),
                ..Default::default()
            };
            println!("{}", snippets::expand(&template, &ctx)?);
        }
    }
    Ok(())
}
