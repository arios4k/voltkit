mod actions;
mod app;
mod awake;
mod awake_ui;
mod config;
mod daemon;
mod doctor;
mod expander;
mod manpages;
mod modules;
mod paste;
mod picker;
mod qr_ui;
mod ruler;
mod portal;
mod screenshots;
mod snippets_ui;
mod system_ui;
mod shortcuts;
mod system;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use actions::Action;
use modules::{color, compress, ocr, qr, snippets};

/// Everyday desktop micro-utilities for Linux, in one binary.
///
/// Run without a subcommand to open the GTK control center. Each subcommand runs one
/// utility from the terminal, a script or a keyboard shortcut.
#[derive(Parser)]
#[command(name = "voltkit", version, author, about)]
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
    /// Open the snippet picker and paste the chosen snippet into the focused app
    ///
    /// Search with the keyboard, press Enter, and fill in any {arg:NAME} parameters. The
    /// expansion is pasted into the app that had focus, through the RemoteDesktop portal (the
    /// desktop asks for permission once), and the previous clipboard is restored.
    Picker {
        /// Open directly on the snippet with this trigger
        #[arg(long, value_name = "TRIGGER")]
        snippet: Option<String>,
    },
    /// Set up and toggle typed triggers (type ;date anywhere to expand it)
    ///
    /// Typed triggers read keyboard input through evdev and paste through a uinput virtual
    /// keyboard, which needs a one-time setup with root rights. Only the last few typed
    /// characters are kept in memory; nothing is logged or stored.
    Expander {
        #[command(subcommand)]
        command: ExpanderCommand,
    },
    /// Measure distances on screen in pixels
    ///
    /// Freezes the screen and shows it full screen. Drag to measure a width and height; hold
    /// Shift for a straight line. Ctrl+C copies the size, right-click clears, Esc closes.
    Ruler,
    /// Keep the computer from sleeping or locking its screen
    ///
    /// Held by the background service, so it continues after this command or the window
    /// closes. Without DURATION, shows whether Keep Awake is on.
    Awake {
        /// 30m, 1h, 1h30m or a number of minutes; `on` until turned off; `off` to stop
        duration: Option<String>,
    },
    /// Read QR codes from the screen or image files, or make them from text
    Qr {
        #[command(subcommand)]
        command: QrCommand,
    },
    /// Run the background service that listens for global shortcuts
    ///
    /// Without a subcommand, runs the service in the foreground until interrupted. It owns
    /// io.github.arios4k.Voltkit.Daemon on the session bus, so only one instance runs, and binds the
    /// configured hotkeys through the XDG GlobalShortcuts portal when the desktop provides it.
    Daemon {
        #[command(subcommand)]
        command: Option<DaemonCommand>,
    },
    /// Check that everything Voltkit needs is installed and set up
    ///
    /// Checks the helper programs, OCR language data, desktop portals, clipboard tools, the
    /// background service and hotkeys, then prints the install command for anything missing on
    /// this distro.
    Doctor,
    /// Run a hotkey action via the daemon, or directly if it isn't running
    ///
    /// Meant to be bound to a key in the desktop's keyboard settings or a compositor config.
    /// Results and errors are shown as desktop notifications.
    Trigger { action: Action },
    /// Manage GNOME keybindings for desktops without the GlobalShortcuts portal
    ///
    /// Adds or removes custom shortcuts in GNOME Settings (via gsettings) that run
    /// `voltkit trigger`. Other custom shortcuts are left untouched.
    Shortcuts {
        #[command(subcommand)]
        command: ShortcutsCommand,
    },
    /// Write man pages for voltkit and its subcommands into DIR
    #[command(hide = true)]
    Manpages { dir: PathBuf },
    /// Render README screenshots of the GUI with demo content into DIR
    #[command(hide = true)]
    Screenshots { dir: PathBuf },
}

#[derive(Subcommand)]
enum QrCommand {
    /// Select a screen region and copy the text of the QR code in it
    Scan,
    /// Print the text of every QR code in an image file
    Read { file: PathBuf },
    /// Show TEXT as a QR code in the terminal, or save it as a PNG image
    Make {
        text: String,
        /// Save a PNG image instead of printing to the terminal
        #[arg(long, value_name = "FILE")]
        output: Option<PathBuf>,
        /// Put a small logo (PNG, JPEG, WebP or SVG) in the middle of the code (PNG
        /// output only). The code is read back to make sure it still scans.
        #[arg(long, value_name = "IMAGE", requires = "output")]
        logo: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum DaemonCommand {
    /// Show whether the daemon is running and how shortcuts are bound
    Status,
    /// Stop the running daemon
    Stop,
    /// Make the running daemon re-read snippets from the config file
    Reload,
    /// Start the daemon automatically at login
    EnableAutostart,
    /// Stop starting the daemon at login
    DisableAutostart,
}

#[derive(Subcommand)]
enum ExpanderCommand {
    /// Show whether typed triggers are enabled and can run
    Status,
    /// Grant keyboard access: add you to the input group and allow /dev/uinput
    Setup {
        /// Run the commands with sudo instead of only printing them
        #[arg(long)]
        apply: bool,
    },
    /// Turn typed triggers on (restarts the daemon if it is running)
    Enable,
    /// Turn typed triggers off (restarts the daemon if it is running)
    Disable,
}

#[derive(Subcommand)]
enum ShortcutsCommand {
    /// Add Super+Shift+T, C and E (or your configured keys) to GNOME Settings
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

fn expander_command(command: ExpanderCommand, mut config: config::Config) -> Result<()> {
    match command {
        ExpanderCommand::Status => {
            println!("typed triggers: {}", if config.expander.typed_triggers { "enabled" } else { "disabled" });
            let missing = expander::missing_permissions();
            if missing.is_empty() {
                println!("permissions: ok");
            } else {
                for m in missing {
                    println!("permissions: {m}");
                }
                println!("next step: {}", expander::next_step());
            }
        }
        ExpanderCommand::Setup { apply } => {
            let user = std::env::var("USER")?;
            let script = expander::setup_script(&user);
            if apply {
                let status = std::process::Command::new("sudo").args(["sh", "-c", &script]).status()?;
                anyhow::ensure!(status.success(), "setup failed");
                println!("Done. Log out and back in, then run `voltkit expander enable`.");
            } else {
                println!("# Typed triggers need these commands to run as root. They add you to the");
                println!("# `input` group, which lets programs you run read keyboard input, and allow");
                println!("# /dev/uinput for the virtual keyboard. Run `voltkit expander setup --apply`");
                println!("# to run them with sudo, then log out and back in.\n");
                print!("{script}");
            }
        }
        ExpanderCommand::Enable | ExpanderCommand::Disable => {
            let enable = matches!(command, ExpanderCommand::Enable);
            config.expander.typed_triggers = enable;
            config.save()?;
            println!("Typed triggers {}", if enable { "enabled" } else { "disabled" });
            if enable {
                let missing = expander::missing_permissions();
                if !missing.is_empty() {
                    println!("They won't work yet: {}", expander::next_step());
                }
            }
            if daemon::restart_if_running()? {
                println!("Restarted the daemon");
            } else {
                println!("Takes effect the next time the daemon starts");
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::Config::load()?;

    match cli.command {
        None => app::run(config),
        Some(Command::Ocr { lang }) => {
            let text = portal::block_on(ocr::extract_text(&lang, None))?;
            println!("{text}");
            system::copy_to_clipboard(&text)?;
        }
        Some(Command::PickColor) => {
            let c = portal::block_on(portal::pick_color(None))?;
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
                Some((version, backend, typed)) => {
                    println!("running (v{version}), shortcuts: {backend}, typed triggers: {typed}")
                }
                None => println!("not running"),
            },
            DaemonCommand::Reload => match daemon::reload() {
                Ok(()) => println!("Reloaded snippets"),
                Err(_) => println!("not running"),
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
        // The ruler opens its own window: running it here keeps the launcher's
        // activation token, so the window gets focus (GNOME only shows the
        // screenshot permission prompt to the focused app).
        Some(Command::Trigger { action: Action::Ruler }) => ruler::run(),
        Some(Command::Trigger { action }) => daemon::trigger(action, &config)?,
        Some(Command::Qr { command }) => match command {
            QrCommand::Scan => {
                let text = portal::block_on(qr::scan(None))?.join("\n");
                println!("{text}");
                system::copy_to_clipboard(&text)?;
            }
            QrCommand::Read { file } => {
                let codes = qr::decode_file(&file)?;
                anyhow::ensure!(!codes.is_empty(), "no QR code found in {}", file.display());
                for code in codes {
                    println!("{code}");
                }
            }
            QrCommand::Make { text, output: Some(path), logo } => {
                let image = match logo {
                    Some(file) => {
                        let logo = qr::load_logo(&file)?;
                        let m = qr::encode_for(&text, true)?;
                        let size = qr::checked_logo_size(&m, &text, &logo, qr::LOGO_SIZES[1])
                            .context("the code doesn't scan with this logo; shorten the text")?;
                        qr::to_pixbuf_with_logo(&m, 12, &logo, size)
                    }
                    None => qr::to_pixbuf(&qr::encode(&text)?, 12),
                };
                image.savev(&path, "png", &[])?;
                println!("Saved {}", path.display());
            }
            QrCommand::Make { text, output: None, .. } => print!("{}", qr::to_terminal(&qr::encode(&text)?)),
        },
        Some(Command::Doctor) => {
            let report = doctor::run(&config);
            print!("{}", report.render());
            if report.missing(doctor::Need::Required).next().is_some() {
                std::process::exit(1);
            }
        }
        Some(Command::Picker { snippet }) => picker::run(config, snippet),
        Some(Command::Ruler) => ruler::run(),
        Some(Command::Awake { duration: None }) => match daemon::awake_status() {
            Some((on, until)) => println!("Keep Awake: {}", awake::describe(on, until)),
            None => println!("Keep Awake: off (the background service isn't running)"),
        },
        Some(Command::Awake { duration: Some(d) }) => match awake::parse_duration(&d)? {
            None => {
                daemon::stop_awake()?;
                println!("Keep Awake: Off");
            }
            Some(minutes) => {
                daemon::keep_awake(minutes)?;
                let (on, until) = daemon::awake_status().unwrap_or((true, 0));
                println!("Keep Awake: {}", awake::describe(on, until));
            }
        },
        Some(Command::Expander { command }) => expander_command(command, config)?,
        Some(Command::Screenshots { dir }) => screenshots::run(config, dir)?,
        Some(Command::Manpages { dir }) => {
            for path in manpages::generate(&dir)? {
                println!("{}", path.display());
            }
        }
        Some(Command::Shortcuts { command }) => match command {
            ShortcutsCommand::Install { dry_run } => {
                shortcuts::gnome_install(&config, dry_run)?;
                if !dry_run {
                    for action in Action::ALL {
                        println!("{:<18} {}", action.accelerator(&config), action.description());
                    }
                    println!("Added to GNOME keyboard shortcuts");
                }
            }
            ShortcutsCommand::Remove { dry_run } => shortcuts::gnome_remove(dry_run)?,
        },
    }
    Ok(())
}
