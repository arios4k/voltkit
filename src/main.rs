mod actions;
mod app;
mod config;
mod daemon;
mod modules;
mod portal;
mod shortcuts;
mod system;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use actions::Action;
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
    /// Pick a color from the screen and copy its HEX value
    PickColor,
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
    /// Run the background service that listens for global shortcuts
    Daemon {
        #[command(subcommand)]
        command: Option<DaemonCommand>,
    },
    /// Run a hotkey action via the daemon (or directly if it isn't running);
    /// bind this in your desktop's keyboard settings
    Trigger { action: Action },
    /// Manage GNOME keybindings for desktops without the GlobalShortcuts portal
    Shortcuts {
        #[command(subcommand)]
        command: ShortcutsCommand,
    },
}

#[derive(Subcommand)]
enum DaemonCommand {
    /// Show whether the daemon is running and how shortcuts are bound
    Status,
    /// Stop the running daemon
    Stop,
    /// Start the daemon automatically at login
    EnableAutostart,
    /// Stop starting the daemon at login
    DisableAutostart,
}

#[derive(Subcommand)]
enum ShortcutsCommand {
    /// Add Super+Shift+T / Super+Shift+C (or your configured keys) to GNOME Settings
    Install {
        /// Print the gsettings commands instead of running them
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove the keybindings added by `install`
    Remove {
        #[arg(long)]
        dry_run: bool,
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
            let text = portal::block_on(ocr::extract_text(&lang))?;
            println!("{text}");
            system::copy_to_clipboard(&text)?;
        }
        Some(Command::PickColor) => {
            let c = portal::block_on(portal::pick_color())?;
            color::history::push(c)?;
            println!("{}", c.to_hex());
            system::copy_to_clipboard(&c.to_hex())?;
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
        Some(Command::Daemon { command: None }) => daemon::run(config)?,
        Some(Command::Daemon { command: Some(cmd) }) => match cmd {
            DaemonCommand::Status => match daemon::status() {
                Some((version, backend)) => println!("running (v{version}), shortcuts: {backend}"),
                None => println!("not running"),
            },
            DaemonCommand::Stop => {
                if daemon::stop().is_err() {
                    println!("not running");
                }
            }
            DaemonCommand::EnableAutostart => {
                println!("Created {}", daemon::set_autostart(true)?.display())
            }
            DaemonCommand::DisableAutostart => {
                println!("Removed {}", daemon::set_autostart(false)?.display())
            }
        },
        Some(Command::Trigger { action }) => daemon::trigger(action, &config)?,
        Some(Command::Shortcuts { command }) => match command {
            ShortcutsCommand::Install { dry_run } => {
                shortcuts::gnome_install(&config, dry_run)?;
                if !dry_run {
                    println!(
                        "Added {} and {} to GNOME keyboard shortcuts",
                        config.hotkeys.ocr, config.hotkeys.color_picker
                    );
                }
            }
            ShortcutsCommand::Remove { dry_run } => shortcuts::gnome_remove(dry_run)?,
        },
    }
    Ok(())
}
