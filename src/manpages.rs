//! Man pages, generated from the clap definitions plus hand-written sections.
//!
//! Regenerate the committed copies with `cargo run -- manpages man`.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::CommandFactory;
use clap_mangen::Man;
use roff::{Roff, bold, roman};

use crate::Cli;

enum Block {
    Para(&'static str),
    /// A term (rendered bold) and its description.
    Item(&'static str, &'static str),
    /// A description followed by an indented command line.
    Example(&'static str, &'static str),
}

use Block::{Example, Item, Para};
use crate::i18n::tr;

struct Section {
    heading: &'static str,
    blocks: &'static [Block],
}

const FILES: Section = Section {
    heading: N_!("FILES"),
    blocks: &[
        Item("~/.config/voltkit/config.toml", N_!("Configuration: hotkeys, OCR language, compression quality and snippets. Every key is optional.")),
        Item("~/.local/share/voltkit/colors.txt", N_!("Color history, newest first, shared by pick-color, the daemon and the control center.")),
        Item("~/.config/autostart/io.github.arios4k.Voltkit.Daemon.desktop", N_!("Created by voltkit daemon enable-autostart.")),
    ],
};

/// Extra sections for each page, keyed by page name.
fn extra_sections(page: &str) -> Vec<Section> {
    match page {
        "voltkit" => vec![
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Open the control center:"), "voltkit"),
                    Example(N_!("Copy text from part of the screen:"), "voltkit ocr"),
                    Example(N_!("Check a color's contrast:"), "voltkit color '#3584e4'"),
                    Example(N_!("Start the background service at every login:"), "voltkit daemon enable-autostart"),
                ],
            },
            FILES,
            Section {
                heading: N_!("ENVIRONMENT"),
                blocks: &[
                    Item("WAYLAND_DISPLAY", N_!("When set, the clipboard is accessed with wl-copy and wl-paste; otherwise with xclip.")),
                    Item("XDG_CONFIG_HOME, XDG_DATA_HOME", N_!("Override the base directories of the files listed above.")),
                ],
            },
            Section {
                heading: N_!("EXIT STATUS"),
                blocks: &[Para(N_!("0 on success, 1 on any error. Errors are printed to standard error."))],
            },
        ],
        "voltkit-ocr" => vec![
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Recognize English text:"), "voltkit ocr"),
                    Example(N_!("Recognize English and German:"), "voltkit ocr --lang eng+deu"),
                    Example(N_!("Save the text instead of only copying it:"), "voltkit ocr > notes.txt"),
                ],
            },
            Section {
                heading: N_!("NOTES"),
                blocks: &[Para(
                    N_!("Each language needs its Tesseract data package, for example tesseract-ocr-deu on Debian and Ubuntu. \
                     List the installed ones with tesseract --list-langs. Leave a small margin around the text when \
                     selecting: letters cut by the selection edge are misread."),
                )],
            },
        ],
        "voltkit-pick-color" => vec![Section {
            heading: N_!("EXAMPLES"),
            blocks: &[
                Example(N_!("Pick a color and show all its formats:"), "voltkit color \"$(voltkit pick-color)\""),
            ],
        }],
        "voltkit-color" => vec![
            Section {
                heading: N_!("CONTRAST LEVELS"),
                blocks: &[
                    Para(N_!("Ratings are for normal-size text under WCAG 2.x:")),
                    Item("AAA", N_!("7:1 or higher.")),
                    Item("AA", N_!("4.5:1 or higher.")),
                    Item(N_!("AA Large"), N_!("3:1 or higher; passes only for large text (18pt, or 14pt bold).")),
                    Item(N_!("Fail"), N_!("Below 3:1.")),
                ],
            },
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Hex with or without #, short or long form:"), "voltkit color f80"),
                    Example(N_!("Functional notation:"), "voltkit color 'rgb(51, 130, 202)'"),
                ],
            },
        ],
        "voltkit-compress" => vec![
            Section {
                heading: N_!("TOOLS"),
                blocks: &[
                    Para(N_!("Compression runs these programs, which must be installed:")),
                    Item("PNG", N_!("built in (oxipng), lossless, all metadata stripped.")),
                    Item("JPEG", N_!("jpegoptim, recompressed down to --quality, all metadata stripped.")),
                    Item("WebP", N_!("cwebp, at --quality, metadata dropped.")),
                    Item("MP4", N_!("ffmpeg, H.264 with a CRF derived from --quality, AAC audio, metadata removed, fast-start enabled.")),
                ],
            },
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Compress every PNG in a folder:"), "voltkit compress ~/Pictures/*.png"),
                    Example(N_!("Shrink a video harder:"), "voltkit compress clip.mp4 --quality 50"),
                ],
            },
        ],
        "voltkit-expand" => vec![
            Section {
                heading: N_!("VARIABLES"),
                blocks: &[
                    Item("{date}", N_!("Today's date as YYYY-MM-DD.")),
                    Item("{date:FORMAT}", N_!("The current time in any strftime(3) FORMAT, for example {date:%A %d %B}.")),
                    Item("{time}", N_!("The current time as HH:MM.")),
                    Item("{clipboard}", N_!("The clipboard's text, or nothing if it is empty.")),
                    Item("{arg:NAME}", N_!("The value given with --arg NAME=VALUE.")),
                    Item("{{ and }}", N_!("A literal { or }.")),
                ],
            },
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Fill in parameters:"), "voltkit expand 'Hi {arg:name}, re: {arg:topic}' --arg name=Sam --arg topic=Invoices"),
                    Example(N_!("Values with spaces need quotes:"), "voltkit expand 'Hi {arg:name}' --arg 'name=Sam Lee'"),
                    Example(N_!("Quote the clipboard with a timestamp:"), "voltkit expand '> {clipboard} ({date} {time})'"),
                ],
            },
        ],
        "voltkit-daemon" => vec![
            Section {
                heading: N_!("SHORTCUT BACKENDS"),
                blocks: &[
                    Item("portal", N_!("The GlobalShortcuts portal (GNOME 48+, KDE Plasma 5.27+, Hyprland). The desktop asks once for approval.")),
                    Item("gnome-keybindings", N_!("Custom shortcuts added by voltkit shortcuts install.")),
                    Item("none", N_!("Bind voltkit trigger ACTION in the desktop or compositor settings.")),
                ],
            },
            Section {
                heading: N_!("D-BUS INTERFACE"),
                blocks: &[
                    Para(N_!("Bus name io.github.arios4k.Voltkit.Daemon, object /io/github/arios4k/Voltkit/Daemon, interface io.github.arios4k.Voltkit.Daemon1:")),
                    Item("Trigger(s action)", N_!("Start \"ocr\", \"color\" or \"snippets\" and return immediately. Ignored while another one is open.")),
                    Item("Status() → (sss)", N_!("The version, the shortcut backend, and whether typed triggers are on, off or unavailable.")),
                    Item("KeepAwake(u minutes)", N_!("Keep the computer from sleeping or locking, for MINUTES or, with 0, until stopped.")),
                    Item("StopAwake()", N_!("End Keep Awake.")),
                    Item("AwakeStatus() → (bx)", N_!("Whether Keep Awake is on, and when it ends as a Unix time (0: until stopped).")),
                    Item("Reload()", N_!("Re-read snippets from the configuration file, so typed triggers use edits right away. The control center calls this after every change.")),
                    Item("Quit()", N_!("Stop the daemon.")),
                ],
            },
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Check whether it is running:"), "voltkit daemon status"),
                    Example(N_!("Trigger OCR from another program:"), "busctl --user call io.github.arios4k.Voltkit.Daemon /io/github/arios4k/Voltkit/Daemon io.github.arios4k.Voltkit.Daemon1 Trigger s ocr"),
                ],
            },
            FILES,
        ],
        "voltkit-picker" => vec![
            Section {
                heading: N_!("KEYS"),
                blocks: &[
                    Item(N_!("Up, Down"), N_!("Choose a snippet while typing in the search box.")),
                    Item(N_!("Enter"), N_!("Insert the snippet, or move to the next parameter field.")),
                    Item(N_!("Escape"), N_!("Go back from the parameter form, or close the picker.")),
                ],
            },
            Section {
                heading: N_!("NOTES"),
                blocks: &[Para(
                    N_!("The first paste asks for remote-desktop permission; the grant is remembered in \
                     ~/.local/state/voltkit/remote-desktop-token. If pasting is refused or unavailable, the \
                     expansion stays on the clipboard and a notification says to press Ctrl+V. Apps that paste \
                     with another shortcut, such as terminals, need paste_shortcut set in the [expander] section \
                     of the configuration."),
                )],
            },
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[Example(N_!("Open the picker on one snippet's form:"), "voltkit picker --snippet ';quote'")],
            },
        ],
        "voltkit-expander" => vec![
            Section {
                heading: N_!("HOW IT WORKS"),
                blocks: &[
                    Para(
                        N_!("The daemon reads key presses from /dev/input and keeps the last 64 typed characters in \
                         memory. When they end with a snippet's trigger, it erases the trigger with Backspace \
                         through a uinput virtual keyboard and pastes the expansion, restoring the clipboard \
                         afterwards. Snippets with {arg:NAME} parameters open the picker's form instead."),
                    ),
                    Para(
                        N_!("Setup installs a udev rule that tags keyboards and /dev/uinput with uaccess, the way \
                         desktops grant devices to the person at the computer, and applies it at once: no \
                         group to join and no need to log out. The daemon starts and stops typed triggers as \
                         they are turned on and off, and picks up keyboards plugged in later."),
                    ),
                    Para(
                        N_!("Keys are decoded with a US layout, so triggers made of letters, digits and ; work on \
                         most layouts while punctuation may differ. A trigger that is a prefix of another (;d \
                         and ;date) fires first, so the longer one is unreachable."),
                    ),
                ],
            },
            Section {
                heading: N_!("SECURITY"),
                blocks: &[Para(
                    N_!("Keyboard access applies to every program the person at the computer runs, not only \
                     Voltkit. Typed text is never logged, written to disk or sent anywhere. `voltkit expander \
                     remove-access` (or turning typed triggers off in the app and choosing to remove access) \
                     deletes /etc/udev/rules.d/70-voltkit-typed-triggers.rules and \
                     /etc/modules-load.d/voltkit-uinput.conf; the access already granted ends at the next \
                     restart. Setups from before version 0.6 also added you to the input group, which \
                     `gpasswd -d $USER input` undoes."),
                )],
            },
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Review the setup commands, then run them:"), "voltkit expander setup\nvoltkit expander setup --apply"),
                    Example(N_!("Turn typed triggers off, and remove the keyboard access:"), "voltkit expander remove-access"),
                ],
            },
        ],
        "voltkit-doctor" => vec![
            Section {
                heading: N_!("OUTPUT"),
                blocks: &[
                    Item("✓", N_!("In place.")),
                    Item("✗", N_!("Missing, and the feature does not work without it.")),
                    Item("–", N_!("Missing or off, but the feature still works in a limited way.")),
                    Para(N_!("Install commands use the package manager of the running distribution (apt, dnf, pacman or zypper), detected from /etc/os-release.")),
                ],
            },
            Section {
                heading: N_!("EXIT STATUS"),
                blocks: &[Para(N_!("0 when nothing required is missing, 1 otherwise, so it can be used in scripts."))],
            },
        ],
        "voltkit-awake" => vec![
            Section {
                heading: N_!("NOTES"),
                blocks: &[Para(
                    N_!("The background service holds the request through the desktop's Inhibit portal, so it \
                     needs `voltkit daemon` running. When the time is up, Keep Awake ends by itself and shows a \
                     notification."),
                )],
            },
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Stay awake for a 90-minute movie:"), "voltkit awake 1h30m"),
                    Example(N_!("Stay awake until told otherwise:"), "voltkit awake on"),
                    Example(N_!("Check, then stop:"), "voltkit awake\nvoltkit awake off"),
                ],
            },
        ],
        "voltkit-qr" => vec![Section {
            heading: N_!("EXAMPLES"),
            blocks: &[
                Example(N_!("Copy the link from a QR code on screen:"), "voltkit qr scan"),
                Example(N_!("Read the codes in a photo:"), "voltkit qr read photo.jpg"),
                Example(N_!("Show a link as a QR code in the terminal:"), "voltkit qr make https://example.com"),
                Example(N_!("Save a Wi-Fi code to print:"), "voltkit qr make 'WIFI:T:WPA;S:Home;P:secret;;' --output wifi.png"),
            ],
        }],
        "voltkit-using" => vec![Section {
            heading: N_!("EXAMPLES"),
            blocks: &[
                Example(N_!("Find what keeps a USB drive busy:"), "voltkit using /media/$USER/USB"),
                Example(N_!("Find the program on port 3000:"), "voltkit using 3000"),
                Example(N_!("Include system services:"), "sudo voltkit using 80"),
                Example(N_!("Ask a program to quit, then force it if it won't:"), "kill PID\nkill -KILL PID"),
            ],
        }],
        "voltkit-sleep" => vec![Section {
            heading: N_!("NOTES"),
            blocks: &[Para(
                N_!("The sleep history comes from the system journal, which members of the adm or systemd-journal \
                 groups can read. Linux records what woke the computer only when it was the lid or the power \
                 button; other wakes are described by their likely causes. Which USB devices may wake the \
                 computer can be changed in the app (with the administrator password); the choice is kept in \
                 /etc/udev/rules.d/90-voltkit-wakeup.rules."),
            )],
        }],
        "voltkit-report" => vec![
            Section {
                heading: N_!("NOTES"),
                blocks: &[Para(
                    N_!("The report leaves out serial numbers, user and computer names, and network addresses, so it \
                     can be posted in a forum or bug report. Drive health comes from UDisks2 and firmware updates \
                     from fwupd. Each run (and the background service, once a day) saves a snapshot of the \
                     hardware, drivers, kernel and firmware in ~/.local/share/voltkit/snapshots, which later \
                     reports compare to show what changed. --memory reads the memory slots from the SMBIOS \
                     tables, which needs the administrator password."),
                )],
            },
            Section {
                heading: N_!("EXAMPLES"),
                blocks: &[
                    Example(N_!("Copy the report to paste in a forum post:"), "voltkit report | wl-copy"),
                    Example(N_!("Include memory slots and upgrade options:"), "voltkit report --memory"),
                ],
            },
        ],
        "voltkit-convert" => vec![Section {
            heading: N_!("EXAMPLES"),
            blocks: &[
                Example(N_!("List the conversions:"), "voltkit convert"),
                Example(N_!("Tidy JSON from a file:"), "voltkit convert json < data.json"),
                Example(N_!("Turn a copied spreadsheet range into a Markdown table:"), "wl-paste | voltkit convert markdown-table"),
                Example(N_!("Remove tracking from a link:"), "echo 'https://example.com/?utm_source=x' | voltkit convert clean-link"),
            ],
        }],
        "voltkit-trigger" => vec![Section {
            heading: N_!("EXAMPLES"),
            blocks: &[Example(N_!("A Sway or i3 binding:"), "bindsym $mod+Shift+t exec voltkit trigger ocr")],
        }],
        "voltkit-shortcuts" => vec![Section {
            heading: N_!("EXAMPLES"),
            blocks: &[
                Example(N_!("What does Super+V do?"), "voltkit shortcuts who super+v"),
                Example(N_!("Find shortcuts used by more than one action:"), "voltkit shortcuts clashes"),
                Example(N_!("Preview the gsettings commands:"), "voltkit shortcuts install --dry-run"),
                Example(N_!("Remove the shortcuts again:"), "voltkit shortcuts remove"),
            ],
        }],
        _ => Vec::new(),
    }
}

fn render_section(roff: &mut Roff, section: &Section) {
    roff.control("SH", [tr(section.heading)]);
    for block in section.blocks {
        match block {
            Para(text) => {
                roff.control("PP", []);
                roff.text([roman(tr(text))]);
            }
            Item(term, text) => {
                roff.control("TP", []);
                roff.text([bold(tr(term))]);
                roff.text([roman(tr(text))]);
            }
            Example(text, command) => {
                roff.control("PP", []);
                roff.text([roman(tr(text))]);
                roff.control("RS", ["4"]);
                roff.control("nf", []);
                roff.text([bold(*command)]);
                roff.control("fi", []);
                roff.control("RE", []);
            }
        }
    }
}

fn see_also(page: &str, siblings: &[String]) -> Roff {
    let mut refs: Vec<String> = std::iter::once("voltkit".to_owned())
        .chain(siblings.iter().cloned())
        .filter(|p| p != page)
        .map(|p| format!("{p}(1)"))
        .collect();
    let external = match page {
        "voltkit-ocr" => &["tesseract(1)"][..],
        "voltkit-compress" => &["jpegoptim(1)", "cwebp(1)", "ffmpeg(1)"],
        "voltkit-expand" => &["strftime(3)"],
        "voltkit-shortcuts" => &["gsettings(1)"],
        "voltkit-expander" => &["udev(7)", "usermod(8)"],
        "voltkit-daemon" => &["busctl(1)"],
        _ => &[],
    };
    refs.extend(external.iter().map(|s| s.to_string()));

    let mut roff = Roff::new();
    roff.control("SH", [tr("SEE ALSO")]);
    let mut inlines = Vec::new();
    for (i, r) in refs.iter().enumerate() {
        if i > 0 {
            inlines.push(roman(", "));
        }
        let (name, sect) = r.split_once('(').unwrap();
        inlines.push(bold(name));
        inlines.push(roman(format!("({sect}")));
    }
    roff.text(inlines);
    roff
}

/// Lists nested subcommands (`daemon status`, ...) inline, since they don't get pages of their own.
fn nested_commands(cmd: &clap::Command) -> Roff {
    let mut roff = Roff::new();
    roff.control("SH", [tr("COMMANDS")]);
    let parent = cmd.get_display_name().unwrap_or(cmd.get_name()).replace('-', " ");
    for sub in cmd.get_subcommands().filter(|s| !s.is_hide_set()) {
        let mut usage = format!("{parent} {}", sub.get_name());
        for arg in sub.get_arguments().filter(|a| !a.is_hide_set() && a.get_long() != Some("help")) {
            if let Some(long) = arg.get_long() {
                usage.push_str(&format!(" [--{long}]"));
            }
        }
        roff.control("TP", []);
        roff.text([bold(usage)]);
        let about = sub.get_long_about().or(sub.get_about()).map(|a| a.to_string()).unwrap_or_default();
        roff.text([roman(about)]);
        for arg in sub.get_arguments().filter(|a| !a.is_hide_set() && a.get_long() != Some("help")) {
            if let (Some(long), Some(help)) = (arg.get_long(), arg.get_help()) {
                roff.control("RS", []);
                roff.text([bold(format!("--{long}")), roman(format!(": {help}"))]);
                roff.control("RE", []);
            }
        }
    }
    roff
}

fn render_page(cmd: clap::Command, siblings: &[String]) -> Result<(String, Vec<u8>)> {
    let page = cmd.get_display_name().unwrap_or(cmd.get_name()).to_owned();
    let is_root = page == "voltkit";
    let has_args = cmd.get_arguments().any(|a| !a.is_hide_set());
    let has_subcommands = cmd.get_subcommands().any(|s| !s.is_hide_set());
    let nested = (!is_root && has_subcommands).then(|| nested_commands(&cmd));
    let man = Man::new(cmd)
        .manual(tr("Voltkit Manual"))
        .source(format!("Voltkit {}", env!("CARGO_PKG_VERSION")));

    let mut out = Vec::new();
    man.render_title(&mut out)?;
    let mut generated = Vec::new();
    man.render_name_section(&mut generated)?;
    man.render_synopsis_section(&mut generated)?;
    man.render_description_section(&mut generated)?;
    if has_args {
        man.render_options_section(&mut generated)?;
    }
    if nested.is_none() && has_subcommands {
        man.render_subcommands_section(&mut generated)?;
    }
    out.extend(translate_headings(&String::from_utf8(generated)?).into_bytes());
    if let Some(roff) = nested {
        roff.to_writer(&mut out)?;
    }
    let mut roff = Roff::new();
    for section in extra_sections(&page) {
        render_section(&mut roff, &section);
    }
    roff.to_writer(&mut out)?;
    see_also(&page, siblings).to_writer(&mut out)?;
    let mut author = Roff::new();
    author.control("SH", [tr("AUTHOR")]);
    author.text([roman(env!("CARGO_PKG_AUTHORS").replace(':', ", "))]);
    author.to_writer(&mut out)?;
    Ok((format!("{page}.1"), out))
}

/// The section headings clap_mangen writes, in the language in use.
fn translate_headings(roff: &str) -> String {
    const HEADINGS: [&str; 5] = [N_!("NAME"), N_!("SYNOPSIS"), N_!("DESCRIPTION"), N_!("OPTIONS"), N_!("SUBCOMMANDS")];
    roff.lines()
        .map(|line| match line.strip_prefix(".SH ").filter(|h| HEADINGS.contains(h)) {
            Some(heading) => match tr(heading) {
                t if t.contains(' ') => format!(".SH \"{t}\"\n"),
                t => format!(".SH {t}\n"),
            },
            None => format!("{line}\n"),
        })
        .collect()
}

/// Every page as (file name, contents): `voltkit.1` plus one per visible subcommand.
/// Nested subcommands such as `daemon status` are documented on their parent's page.
pub fn pages() -> Result<Vec<(String, Vec<u8>)>> {
    let mut root = Cli::command();
    if crate::i18n::language() != "en" {
        root = crate::i18n::localize_command(root, false);
    }
    let mut root = root.disable_help_subcommand(true);
    root.build();
    let subcommands: Vec<clap::Command> = root.get_subcommands().filter(|s| !s.is_hide_set()).cloned().collect();
    let names: Vec<String> = subcommands
        .iter()
        .map(|c| c.get_display_name().unwrap_or(c.get_name()).to_owned())
        .collect();

    let mut pages = vec![render_page(root.clone(), &names)?];
    for cmd in subcommands {
        pages.push(render_page(cmd, &[])?);
    }
    Ok(pages)
}

/// Every page in every language as (path under the man directory, contents):
/// English at the top, each translation in a folder named after its language,
/// the way they're installed (/usr/share/man/man1, /usr/share/man/es/man1…).
pub fn all_pages() -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut out: Vec<(PathBuf, Vec<u8>)> = crate::i18n::with_language("en", pages)?.into_iter().map(|(n, b)| (PathBuf::from(n), b)).collect();
    for lang in crate::i18n::languages() {
        let pages = crate::i18n::with_language(lang, pages)?;
        out.extend(pages.into_iter().map(|(n, b)| (Path::new(lang).join(n), b)));
    }
    Ok(out)
}

pub fn generate(dir: &Path) -> Result<Vec<PathBuf>> {
    all_pages()?
        .into_iter()
        .map(|(name, bytes)| {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap_or(dir))?;
            std::fs::write(&path, bytes)?;
            Ok(path)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_page_per_visible_command() {
        let names: Vec<String> = pages().unwrap().into_iter().map(|(n, _)| n).collect();
        assert!(names.contains(&"voltkit.1".to_owned()));
        assert!(names.contains(&"voltkit-expand.1".to_owned()));
        assert!(!names.iter().any(|n| n.contains("manpages") || n.contains("help")));
    }

    #[test]
    fn committed_pages_are_current() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("man");
        let expected = all_pages().unwrap();
        for (name, bytes) in &expected {
            let committed = std::fs::read(dir.join(name)).unwrap_or_default();
            assert!(committed == *bytes, "man/{} is stale; run `cargo run -- manpages man`", name.display());
        }
        let mut on_disk = 0;
        for lang in std::iter::once(".").chain(crate::i18n::languages()) {
            on_disk += std::fs::read_dir(dir.join(lang)).unwrap().flatten().filter(|e| e.path().is_file()).count();
        }
        assert_eq!(on_disk, expected.len(), "man/ has pages for commands that no longer exist");
    }
}
