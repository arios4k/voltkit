#[macro_use]
mod i18n;
mod about_ui;
mod workspace;
mod appearance;
mod actions;
mod app;
mod awake;
mod awake_ui;
mod config;
mod daemon;
mod detective_ui;
mod doctor;
mod expander;
mod manpages;
mod modules;
mod paste;
mod picker;
mod qr_ui;
mod ruler;
mod usage_ui;
mod permissions_ui;
mod sleep_ui;
mod sysreport_ui;
mod smartpaste_ui;
mod portal;
mod screenshots;
mod snippets_ui;
mod system_ui;
mod shortcuts;
mod system;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

use actions::Action;
use modules::{color, compress, keybindings, ocr, qr, sleep, smartpaste, snippets, sysreport, usage};
use crate::i18n::tr;

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
    /// Show which programs keep a file, folder, drive or network port busy
    ///
    /// TARGET is a path, or a port number such as 3000. Lists each program with its PID and how
    /// it uses the target. Only your own programs can be checked as a normal user; run it with
    /// sudo to include other users' programs and system services.
    Using {
        /// A file, folder or mount point, or a port number
        target: String,
        /// Tab-separated output, for the app's "Check with Password"
        #[arg(long, hide = true)]
        machine: bool,
        /// The app's language, which pkexec doesn't pass on
        #[arg(long, hide = true)]
        lang: Option<String>,
    },
    /// Paste the clipboard converted: tidy JSON, a table as Markdown, a cleaned link…
    ///
    /// Opens a small window listing the conversions that fit what's on the clipboard, with a
    /// preview of each. Enter pastes the chosen one into the app that had focus and restores the
    /// clipboard. An image on the clipboard can be pasted as its text (OCR).
    SmartPaste,
    /// Convert text from standard input with one of Smart Paste's conversions
    ///
    /// Without CONVERSION, lists them.
    Convert {
        /// A conversion such as json, markdown-table, clean-link or snake
        conversion: Option<String>,
    },
    /// Describe this computer: hardware, drivers, health and recent changes
    ///
    /// Prints a report that's safe to share in a forum or bug report (no serial numbers, names
    /// or network addresses), with checks and fixes for drivers, drive health, disk space,
    /// temperatures and firmware updates. Also saves today's snapshot, so later reports can
    /// show what changed.
    Report {
        /// Also show memory slots and upgrade options (asks for the administrator password)
        #[arg(long)]
        memory: bool,
        /// Memory slots as JSON, read as root (used by --memory and the app)
        #[arg(long, hide = true)]
        privileged: bool,
    },
    /// Explain why the computer doesn't sleep, or why it woke up
    ///
    /// Shows what keeps the computer awake right now (apps and settings), the sleeps of the
    /// last week with what started and ended each one and the battery used, and the USB devices
    /// allowed to wake it.
    Sleep,
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
    /// Print the command-line help text, for collecting it into the translations (po/update.py)
    #[command(hide = true)]
    I18nStrings,
    /// Render README screenshots of the GUI with demo content into DIR
    #[command(hide = true)]
    Screenshots {
        dir: PathBuf,
        /// Window width for checking adaptive layouts
        #[arg(long, value_parser = clap::value_parser!(i32).range(360..))]
        width: Option<i32>,
    },
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
    /// Grant keyboard access and turn typed triggers on, right away (no logout needed)
    Setup {
        /// Run the commands (asks for the administrator password) instead of only printing them
        #[arg(long)]
        apply: bool,
    },
    /// Turn typed triggers on
    Enable,
    /// Turn typed triggers off
    Disable,
    /// Turn typed triggers off and remove Voltkit's keyboard access
    RemoveAccess,
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
    /// List every system-wide shortcut, grouped by who owns it
    List,
    /// Show what a shortcut does, such as `voltkit shortcuts who super+shift+s`
    Who {
        /// The keys, like super+shift+s, ctrl+alt+t or print
        keys: Vec<String>,
    },
    /// Show shortcuts used by more than one action
    Clashes,
}

fn parse_kv(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .ok_or_else(|| trf!("expected name=value, got {}", format!("{:?}", s)))
}

fn expander_command(command: ExpanderCommand, mut config: config::Config) -> Result<()> {
    match command {
        ExpanderCommand::Status => {
            println!("{}", if config.expander.typed_triggers { tr("typed triggers: enabled") } else { tr("typed triggers: disabled") });
            let missing = expander::missing_permissions();
            if missing.is_empty() {
                println!("{}", tr("permissions: ok"));
            } else {
                for m in missing {
                    println!("{}", trf!("permissions: {}", m));
                }
                println!("{}", trf!("next step: {}", expander::next_step()));
            }
        }
        ExpanderCommand::Setup { apply } => {
            if apply {
                expander::grant_access()?;
                config.expander.typed_triggers = true;
                config.save()?;
                let _ = daemon::reload();
                println!("{}", tr("Done: typed triggers can read the keyboard now, no need to log out."));
            } else {
                println!("{}", tr("# Typed triggers need these commands to run as root. They let the person at the"));
                println!("{}", tr("# computer read keyboards and use a virtual keyboard, right away. Run"));
                println!("{}", tr("# `voltkit expander setup --apply` to run them (it asks for your password).\n"));
                print!("{}", expander::grant_script());
            }
        }
        ExpanderCommand::RemoveAccess => {
            config.expander.typed_triggers = false;
            config.save()?;
            let _ = daemon::reload();
            expander::revoke_access()?;
            println!("{}", tr("Typed triggers are off, and Voltkit's keyboard access is removed."));
        }
        ExpanderCommand::Enable | ExpanderCommand::Disable => {
            let enable = matches!(command, ExpanderCommand::Enable);
            config.expander.typed_triggers = enable;
            config.save()?;
            println!("{}", if enable { tr("Typed triggers on") } else { tr("Typed triggers off") });
            if enable && !expander::missing_permissions().is_empty() {
                println!("{}", trf!("They need keyboard access first: {}", expander::next_step()));
            }
            if daemon::reload().is_err() {
                println!("{}", tr("Takes effect when the background service starts"));
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}: {e:#}", tr("Error"));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = if i18n::language() == "en" {
        Cli::parse()
    } else {
        let command = i18n::localize_command(Cli::command(), true);
        let matches = command.clone().get_matches();
        // The translated `help` subcommand stands in for clap's own, at any level.
        let mut path = vec!["voltkit".to_owned()];
        let mut level = &matches;
        while let Some((name, sub)) = level.subcommand() {
            if name == "help" {
                // Asking for `--help` on the named command prints it with the full usage line.
                path.extend(sub.get_many::<String>("command").into_iter().flatten().cloned());
                path.push("--help".to_owned());
                command.get_matches_from(path);
                return Ok(());
            }
            path.push(name.to_owned());
            level = sub;
        }
        Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit())
    };
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
            for (name, bg) in [(tr("on white"), color::Rgb::WHITE), (tr("on black"), color::Rgb::BLACK)] {
                let ratio = c.contrast_ratio(bg);
                println!("{name:<12} {ratio:>5.2}:1  {}", color::wcag_rating(ratio));
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
                    println!("{}", trf!("running (v{}), shortcuts: {}, typed triggers: {}", version, backend, daemon::describe_typed(&typed)))
                }
                None => println!("{}", tr("not running")),
            },
            DaemonCommand::Reload => match daemon::reload() {
                Ok(()) => println!("{}", tr("Reloaded snippets")),
                Err(_) => println!("{}", tr("not running")),
            },
            DaemonCommand::Stop => {
                if daemon::stop().is_err() {
                    println!("{}", tr("not running"));
                }
            }
            DaemonCommand::EnableAutostart => {
                let path = daemon::set_autostart(true)?;
                if path.starts_with("/etc") {
                    println!("{}", trf!("Starts at login through {}, installed with Voltkit", path.display()));
                } else {
                    println!("{}", trf!("Created {}", path.display()));
                }
            }
            DaemonCommand::DisableAutostart => {
                println!("{}", trf!("Removed {}", daemon::set_autostart(false)?.display()))
            }
        },
        // The ruler opens its own window: running it here keeps the launcher's
        // activation token, so the window gets focus (GNOME only shows the
        // screenshot permission prompt to the focused app).
        Some(Command::Trigger { action: Action::Ruler }) => ruler::run(),
        // Smart Paste reads the clipboard, which needs a focused window: same reason.
        Some(Command::Trigger { action: Action::SmartPaste }) => smartpaste_ui::run(config),
        Some(Command::Trigger { action }) => daemon::trigger(action, &config)?,
        Some(Command::Qr { command }) => match command {
            QrCommand::Scan => {
                let text = portal::block_on(qr::scan(None))?.join("\n");
                println!("{text}");
                system::copy_to_clipboard(&text)?;
            }
            QrCommand::Read { file } => {
                let codes = qr::decode_file(&file)?;
                anyhow::ensure!(!codes.is_empty(), "{}", trf!("no QR code found in {}", file.display()));
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
                            .context(tr("the code doesn't scan with this logo; shorten the text"))?;
                        qr::to_pixbuf_with_logo(&m, 12, &logo, size)
                    }
                    None => qr::to_pixbuf(&qr::encode(&text)?, 12),
                };
                image.savev(&path, "png", &[])?;
                println!("{}", trf!("Saved {}", path.display()));
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
        Some(Command::SmartPaste) => smartpaste_ui::run(config),
        Some(Command::Convert { conversion: None }) => {
            for c in smartpaste::ALL {
                println!("{:<16} {}", c.id, tr(c.title));
            }
        }
        Some(Command::Convert { conversion: Some(id) }) => {
            let c = smartpaste::find(&id).with_context(|| trf!("no conversion called {}; run `voltkit convert` for the list", id))?;
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
            let out = (c.apply)(&input).with_context(|| trf!("{} doesn't fit this text", tr(c.title)))?;
            println!("{out}");
        }
        Some(Command::Ruler) => ruler::run(),
        Some(Command::Report { privileged: true, .. }) => println!("{}", serde_json::to_string(&sysreport::memory_info()?)?),
        Some(Command::Report { memory, .. }) => {
            let report = sysreport::collect();
            let _ = sysreport::save_snapshot(&report.facts);
            print!("{}", sysreport::share_text(&report, &sysreport::changes()));
            let fixes: Vec<_> = report.checks.iter().filter(|c| c.command.is_some() || c.link.is_some()).collect();
            if !fixes.is_empty() {
                println!("\n{}", tr("Fixes:"));
                for c in fixes {
                    println!("  {}: {}", c.title, c.command.as_deref().or(c.link.as_deref()).unwrap_or(""));
                }
            }
            if memory {
                println!("\n{}", tr("Memory:"));
                for (label, value) in sysreport::memory_advice(&sysreport::memory_info_as_root()?) {
                    println!("  {label}: {value}");
                }
            }
        }
        Some(Command::Sleep) => print!("{}", sleep::render_text(&portal::block_on(sleep::report()))),
        Some(Command::Using { target, machine, lang }) => {
            if let Some(lang) = lang {
                i18n::set_language(&lang);
            }
            let target = usage::Target::parse(&target)?;
            let scan = usage::scan(&target);
            if machine {
                print!("{}", usage::to_machine(&scan));
                return Ok(());
            }
            if scan.holders.is_empty() {
                println!("{}", trf!("Nothing is using {}", target.describe()));
            }
            for h in &scan.holders {
                let app = h.app_id.as_deref().map(|a| format!(" ({a})")).unwrap_or_default();
                println!("{:>7}  {:<10} {}{app}", h.pid, usage::user_name(h.uid), h.command);
                for u in &h.uses {
                    println!("{:>7}  {:<10}   {u}", "", "");
                }
                if let Some(what) = h.protected() {
                    println!("{:>7}  {:<10}   ({})", "", "", trf!("this is {}", what));
                }
            }
            if scan.unreadable > 0 {
                eprintln!(
                    "{}",
                    trnf!(
                        "{} program of other users or the system wasn't checked; run with sudo to include it",
                        "{} programs of other users or the system weren't checked; run with sudo to include them",
                        scan.unreadable
                    )
                );
            }
        }
        Some(Command::Awake { duration: None }) => match daemon::awake_status() {
            Some((on, until)) => println!("{}", trf!("Keep Awake: {}", awake::describe(on, until))),
            None => println!("{}", tr("Keep Awake: off (the background service isn't running)")),
        },
        Some(Command::Awake { duration: Some(d) }) => match awake::parse_duration(&d)? {
            None => {
                daemon::stop_awake()?;
                println!("{}", tr("Keep Awake: Off"));
            }
            Some(minutes) => {
                daemon::keep_awake(minutes)?;
                let (on, until) = daemon::awake_status().unwrap_or((true, 0));
                println!("{}", trf!("Keep Awake: {}", awake::describe(on, until)));
            }
        },
        Some(Command::Expander { command }) => expander_command(command, config)?,
        Some(Command::Screenshots { dir, width }) => screenshots::run(config, dir, width)?,
        Some(Command::I18nStrings) => {
            for s in i18n::command_strings(&Cli::command()) {
                print!("{s}\0");
            }
        }
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
                    println!("{}", tr("Added to GNOME keyboard shortcuts"));
                }
            }
            ShortcutsCommand::Remove { dry_run } => shortcuts::gnome_remove(dry_run)?,
            ShortcutsCommand::List => {
                let mut owner = String::new();
                for b in keybindings::all() {
                    if b.owner != owner {
                        owner = b.owner.clone();
                        println!("\n{owner}");
                    }
                    let keys: Vec<String> = b.accels.iter().map(|a| keybindings::label(a)).collect();
                    println!("  {:<28} {}", if keys.is_empty() { "-".to_owned() } else { keys.join(", ") }, b.name);
                }
            }
            ShortcutsCommand::Who { keys } => {
                let typed = keys.join(" ");
                let combo = keybindings::parse_typed(&typed).with_context(|| trf!("{} isn't a shortcut", typed))?;
                let accel = keybindings::accel_name(combo);
                let all = keybindings::all();
                let users = keybindings::who(&accel, &all);
                if users.is_empty() {
                    println!("{}", trf!("{} is free. Nearby free combinations:", keybindings::label(&accel)));
                } else {
                    for b in users {
                        println!("{}: {} ({})", keybindings::label(&accel), b.name, b.owner);
                    }
                    println!("{}", tr("Free combinations nearby:"));
                }
                for s in keybindings::suggest(&accel, &all, 6) {
                    println!("  {}", keybindings::label(&s));
                }
            }
            ShortcutsCommand::Clashes => {
                let all = keybindings::all();
                let clashes = keybindings::conflicts(&all);
                if clashes.is_empty() {
                    println!("{}", tr("No clashes: no two actions share a shortcut."));
                }
                for (combo, users) in clashes {
                    println!("{}", keybindings::label(&keybindings::accel_name(combo)));
                    for b in users {
                        println!("  {} ({})", b.name, b.owner);
                    }
                }
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod packaging_tests {
    /// The Fedora package names the version it builds; a release must update
    /// it together with Cargo.toml.
    #[test]
    fn package_versions_match() {
        let version = env!("CARGO_PKG_VERSION");
        let read = |p: &str| std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/").to_owned() + p).unwrap();
        assert!(read("packaging/rpm/voltkit.spec").contains(&format!("\nVersion:        {version}\n")), "voltkit.spec: set Version: {version}");
    }
}
