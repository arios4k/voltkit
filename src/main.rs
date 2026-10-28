mod actions;
mod app;
mod config;
mod daemon;
mod manpages;
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
///
/// Run without a subcommand to open the GTK control center. Each subcommand runs one
/// utility from the terminal, a script or a keyboard shortcut.
#[derive(Parser)]
#[command(name = "powerlinux", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Select a screen region and copy the recognized text to the clipboard
    ///
    /// Opens the desktop's interactive screenshot tool through the XDG Screenshot portal, then
    /// runs Tesseract on the selected region. Without the portal it falls back to grim+slurp,
    /// gnome-screenshot, spectacle or maim. The text is printed and copied to the clipboard.
    Ocr {
        /// Tesseract language code(s), e.g. "eng" or "eng+deu"
        #[arg(long, default_value = "eng")]
        lang: String,
    },
    /// Pick a color from the screen and copy its HEX value
    ///
    /// Uses the XDG Screenshot portal's color picker. The value is printed, copied to the
    /// clipboard and added to the color history shown in the control center.
    PickColor,
    /// Convert a color (#hex or rgb(...)) and report WCAG contrast
    ///
    /// Prints the color as HEX, RGB and HSL, and its WCAG 2.x contrast ratio as text on white
    /// and on black.
    Color {
        /// #rgb, #rrggbb (the # is optional) or rgb(r, g, b)
        value: String,
    },
    /// Strip metadata and compress PNG, JPEG, WebP or MP4 files
    ///
    /// Each FILE is written to NAME.min.EXT beside it; originals are never modified.
    Compress {
        /// Files to compress (.png, .jpg/.jpeg, .webp, .mp4/.m4v/.mov)
        #[arg(required = true, value_name = "FILE")]
        files: Vec<PathBuf>,
        /// Quality from 1 (smallest) to 100 (best); ignored for PNG, which is always lossless
        #[arg(long, default_value_t = 80)]
        quality: u8,
    },
    /// Expand a snippet template, e.g. "Sent {date:%A}: {clipboard}"
    ///
    /// Prints TEMPLATE with its {variables} replaced. Fails if a variable is unknown or an
    /// {arg:NAME} has no matching --arg.
    Expand {
        /// Text with {variables} to expand; quote it to keep the braces from the shell
        template: String,
        /// A value for {arg:NAME}; repeat for several parameters
        #[arg(long = "arg", value_name = "NAME=VALUE", value_parser = parse_kv)]
        args: Vec<(String, String)>,
    },
    /// Run the background service that listens for global shortcuts
    ///
    /// Without a subcommand, runs the service in the foreground until interrupted. It owns
    /// io.github.powerlinux.Daemon on the session bus, so only one instance runs, and binds the
    /// configured hotkeys through the XDG GlobalShortcuts portal when the desktop provides it.
    Daemon {
        #[command(subcommand)]
        command: Option<DaemonCommand>,
    },
    /// Run a hotkey action via the daemon, or directly if it isn't running
    ///
    /// Meant to be bound to a key in the desktop's keyboard settings or a compositor config.
    /// Results and errors are shown as desktop notifications.
    Trigger { action: Action },
    /// Manage GNOME keybindings for desktops without the GlobalShortcuts portal
    ///
    /// Adds or removes custom shortcuts in GNOME Settings (via gsettings) that run
    /// `powerlinux trigger`. Other custom shortcuts are left untouched.
    Shortcuts {
        #[command(subcommand)]
        command: ShortcutsCommand,
    },
    /// Write man pages for powerlinux and its subcommands into DIR
    #[command(hide = true)]
    Manpages { dir: PathBuf },
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
        /// Print the gsettings commands instead of running them
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
        Some(Command::Manpages { dir }) => {
            for path in manpages::generate(&dir)? {
                println!("{}", path.display());
            }
        }
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
