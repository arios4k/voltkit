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

struct Section {
    heading: &'static str,
    blocks: &'static [Block],
}

const FILES: Section = Section {
    heading: "FILES",
    blocks: &[
        Item("~/.config/voltkit/config.toml", "Configuration: hotkeys, OCR language, compression quality and snippets. Every key is optional."),
        Item("~/.local/share/voltkit/colors.txt", "Color history, newest first, shared by pick-color, the daemon and the control center."),
        Item("~/.config/autostart/io.github.arios4k.Voltkit.Daemon.desktop", "Created by voltkit daemon enable-autostart."),
    ],
};

/// Extra sections for each page, keyed by page name.
fn extra_sections(page: &str) -> Vec<Section> {
    match page {
        "voltkit" => vec![
            Section {
                heading: "EXAMPLES",
                blocks: &[
                    Example("Open the control center:", "voltkit"),
                    Example("Copy text from part of the screen:", "voltkit ocr"),
                    Example("Check a color's contrast:", "voltkit color '#3584e4'"),
                    Example("Start the background service at every login:", "voltkit daemon enable-autostart"),
                ],
            },
            FILES,
            Section {
                heading: "ENVIRONMENT",
                blocks: &[
                    Item("WAYLAND_DISPLAY", "When set, the clipboard is accessed with wl-copy and wl-paste; otherwise with xclip."),
                    Item("XDG_CONFIG_HOME, XDG_DATA_HOME", "Override the base directories of the files listed above."),
                ],
            },
            Section {
                heading: "EXIT STATUS",
                blocks: &[Para("0 on success, 1 on any error. Errors are printed to standard error.")],
            },
        ],
        "voltkit-ocr" => vec![
            Section {
                heading: "EXAMPLES",
                blocks: &[
                    Example("Recognize English text:", "voltkit ocr"),
                    Example("Recognize English and German:", "voltkit ocr --lang eng+deu"),
                    Example("Save the text instead of only copying it:", "voltkit ocr > notes.txt"),
                ],
            },
            Section {
                heading: "NOTES",
                blocks: &[Para(
                    "Each language needs its Tesseract data package, for example tesseract-ocr-deu on Debian and Ubuntu. \
                     List the installed ones with tesseract --list-langs. Leave a small margin around the text when \
                     selecting: letters cut by the selection edge are misread.",
                )],
            },
        ],
        "voltkit-pick-color" => vec![Section {
            heading: "EXAMPLES",
            blocks: &[
                Example("Pick a color and show all its formats:", "voltkit color \"$(voltkit pick-color)\""),
            ],
        }],
        "voltkit-color" => vec![
            Section {
                heading: "CONTRAST LEVELS",
                blocks: &[
                    Para("Ratings are for normal-size text under WCAG 2.x:"),
                    Item("AAA", "7:1 or higher."),
                    Item("AA", "4.5:1 or higher."),
                    Item("AA Large", "3:1 or higher; passes only for large text (18pt, or 14pt bold)."),
                    Item("Fail", "Below 3:1."),
                ],
            },
            Section {
                heading: "EXAMPLES",
                blocks: &[
                    Example("Hex with or without #, short or long form:", "voltkit color f80"),
                    Example("Functional notation:", "voltkit color 'rgb(51, 130, 202)'"),
                ],
            },
        ],
        "voltkit-compress" => vec![
            Section {
                heading: "TOOLS",
                blocks: &[
                    Para("Compression runs these programs, which must be installed:"),
                    Item("PNG", "oxipng, lossless, all metadata stripped."),
                    Item("JPEG", "jpegoptim, recompressed down to --quality, all metadata stripped."),
                    Item("WebP", "cwebp, at --quality, metadata dropped."),
                    Item("MP4", "ffmpeg, H.264 with a CRF derived from --quality, AAC audio, metadata removed, fast-start enabled."),
                ],
            },
            Section {
                heading: "EXAMPLES",
                blocks: &[
                    Example("Compress every PNG in a folder:", "voltkit compress ~/Pictures/*.png"),
                    Example("Shrink a video harder:", "voltkit compress clip.mp4 --quality 50"),
                ],
            },
        ],
        "voltkit-expand" => vec![
            Section {
                heading: "VARIABLES",
                blocks: &[
                    Item("{date}", "Today's date as YYYY-MM-DD."),
                    Item("{date:FORMAT}", "The current time in any strftime(3) FORMAT, for example {date:%A %d %B}."),
                    Item("{time}", "The current time as HH:MM."),
                    Item("{clipboard}", "The clipboard's text, or nothing if it is empty."),
                    Item("{arg:NAME}", "The value given with --arg NAME=VALUE."),
                    Item("{{ and }}", "A literal { or }."),
                ],
            },
            Section {
                heading: "EXAMPLES",
                blocks: &[
                    Example("Fill in parameters:", "voltkit expand 'Hi {arg:name}, re: {arg:topic}' --arg name=Sam --arg topic=Invoices"),
                    Example("Values with spaces need quotes:", "voltkit expand 'Hi {arg:name}' --arg 'name=Sam Lee'"),
                    Example("Quote the clipboard with a timestamp:", "voltkit expand '> {clipboard} ({date} {time})'"),
                ],
            },
        ],
        "voltkit-daemon" => vec![
            Section {
                heading: "SHORTCUT BACKENDS",
                blocks: &[
                    Item("portal", "The GlobalShortcuts portal (GNOME 48+, KDE Plasma 5.27+, Hyprland). The desktop asks once for approval."),
                    Item("gnome-keybindings", "Custom shortcuts added by voltkit shortcuts install."),
                    Item("none", "Bind voltkit trigger ACTION in the desktop or compositor settings."),
                ],
            },
            Section {
                heading: "D-BUS INTERFACE",
                blocks: &[
                    Para("Bus name io.github.arios4k.Voltkit.Daemon, object /io/github/arios4k/Voltkit/Daemon, interface io.github.arios4k.Voltkit.Daemon1:"),
                    Item("Trigger(s action)", "Start \"ocr\", \"color\" or \"snippets\" and return immediately. Ignored while another one is open."),
                    Item("Status() → (sss)", "The version, the shortcut backend, and whether typed triggers are on, off or unavailable."),
                    Item("Reload()", "Re-read snippets from the configuration file, so typed triggers use edits right away. The control center calls this after every change."),
                    Item("Quit()", "Stop the daemon."),
                ],
            },
            Section {
                heading: "EXAMPLES",
                blocks: &[
                    Example("Check whether it is running:", "voltkit daemon status"),
                    Example("Trigger OCR from another program:", "busctl --user call io.github.arios4k.Voltkit.Daemon /io/github/arios4k/Voltkit/Daemon io.github.arios4k.Voltkit.Daemon1 Trigger s ocr"),
                ],
            },
            FILES,
        ],
        "voltkit-picker" => vec![
            Section {
                heading: "KEYS",
                blocks: &[
                    Item("Up, Down", "Choose a snippet while typing in the search box."),
                    Item("Enter", "Insert the snippet, or move to the next parameter field."),
                    Item("Escape", "Go back from the parameter form, or close the picker."),
                ],
            },
            Section {
                heading: "NOTES",
                blocks: &[Para(
                    "The first paste asks for remote-desktop permission; the grant is remembered in \
                     ~/.local/state/voltkit/remote-desktop-token. If pasting is refused or unavailable, the \
                     expansion stays on the clipboard and a notification says to press Ctrl+V. Apps that paste \
                     with another shortcut, such as terminals, need paste_shortcut set in the [expander] section \
                     of the configuration.",
                )],
            },
            Section {
                heading: "EXAMPLES",
                blocks: &[Example("Open the picker on one snippet's form:", "voltkit picker --snippet ';quote'")],
            },
        ],
        "voltkit-expander" => vec![
            Section {
                heading: "HOW IT WORKS",
                blocks: &[
                    Para(
                        "The daemon reads key presses from /dev/input and keeps the last 64 typed characters in \
                         memory. When they end with a snippet's trigger, it erases the trigger with Backspace \
                         through a uinput virtual keyboard and pastes the expansion, restoring the clipboard \
                         afterwards. Snippets with {arg:NAME} parameters open the picker's form instead.",
                    ),
                    Para(
                        "After setup, start a new session so it includes the input group: log out and back \
                         in, or restart the computer if lingering is enabled (loginctl show-user $USER \
                         --property=Linger), since then your user services outlive a log out.",
                    ),
                    Para(
                        "Keys are decoded with a US layout, so triggers made of letters, digits and ; work on \
                         most layouts while punctuation may differ. A trigger that is a prefix of another (;d \
                         and ;date) fires first, so the longer one is unreachable.",
                    ),
                ],
            },
            Section {
                heading: "SECURITY",
                blocks: &[Para(
                    "Setup adds you to the input group, which lets every program you run read keyboard input, \
                     not only Voltkit. Typed text is never logged, written to disk or sent anywhere. Undo \
                     the setup by removing you from the group (gpasswd -d $USER input) and deleting \
                     /etc/udev/rules.d/99-voltkit-uinput.rules and /etc/modules-load.d/voltkit-uinput.conf.",
                )],
            },
            Section {
                heading: "EXAMPLES",
                blocks: &[
                    Example("Review the setup commands, then run them:", "voltkit expander setup\nvoltkit expander setup --apply"),
                    Example("After logging out and back in:", "voltkit expander enable"),
                ],
            },
        ],
        "voltkit-doctor" => vec![
            Section {
                heading: "OUTPUT",
                blocks: &[
                    Item("✓", "In place."),
                    Item("✗", "Missing, and the feature does not work without it."),
                    Item("–", "Missing or off, but the feature still works in a limited way."),
                    Para("Install commands use the package manager of the running distribution (apt, dnf, pacman or zypper), detected from /etc/os-release."),
                ],
            },
            Section {
                heading: "EXIT STATUS",
                blocks: &[Para("0 when nothing required is missing, 1 otherwise, so it can be used in scripts.")],
            },
        ],
        "voltkit-qr" => vec![Section {
            heading: "EXAMPLES",
            blocks: &[
                Example("Copy the link from a QR code on screen:", "voltkit qr scan"),
                Example("Read the codes in a photo:", "voltkit qr read photo.jpg"),
                Example("Show a link as a QR code in the terminal:", "voltkit qr make https://example.com"),
                Example("Save a Wi-Fi code to print:", "voltkit qr make 'WIFI:T:WPA;S:Home;P:secret;;' --output wifi.png"),
            ],
        }],
        "voltkit-trigger" => vec![Section {
            heading: "EXAMPLES",
            blocks: &[Example("A Sway or i3 binding:", "bindsym $mod+Shift+t exec voltkit trigger ocr")],
        }],
        "voltkit-shortcuts" => vec![Section {
            heading: "EXAMPLES",
            blocks: &[
                Example("Preview the gsettings commands:", "voltkit shortcuts install --dry-run"),
                Example("Remove the shortcuts again:", "voltkit shortcuts remove"),
            ],
        }],
        _ => Vec::new(),
    }
}

fn render_section(roff: &mut Roff, section: &Section) {
    roff.control("SH", [section.heading]);
    for block in section.blocks {
        match block {
            Para(text) => {
                roff.control("PP", []);
                roff.text([roman(*text)]);
            }
            Item(term, text) => {
                roff.control("TP", []);
                roff.text([bold(*term)]);
                roff.text([roman(*text)]);
            }
            Example(text, command) => {
                roff.control("PP", []);
                roff.text([roman(*text)]);
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
        "voltkit-compress" => &["oxipng(1)", "jpegoptim(1)", "cwebp(1)", "ffmpeg(1)"],
        "voltkit-expand" => &["strftime(3)"],
        "voltkit-shortcuts" => &["gsettings(1)"],
        "voltkit-expander" => &["udev(7)", "usermod(8)"],
        "voltkit-daemon" => &["busctl(1)"],
        _ => &[],
    };
    refs.extend(external.iter().map(|s| s.to_string()));

    let mut roff = Roff::new();
    roff.control("SH", ["SEE ALSO"]);
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
    roff.control("SH", ["COMMANDS"]);
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
        .manual("Voltkit Manual")
        .source(format!("Voltkit {}", env!("CARGO_PKG_VERSION")));

    let mut out = Vec::new();
    man.render_title(&mut out)?;
    man.render_name_section(&mut out)?;
    man.render_synopsis_section(&mut out)?;
    man.render_description_section(&mut out)?;
    if has_args {
        man.render_options_section(&mut out)?;
    }
    match nested {
        Some(roff) => roff.to_writer(&mut out)?,
        None if has_subcommands => man.render_subcommands_section(&mut out)?,
        None => {}
    }
    let mut roff = Roff::new();
    for section in extra_sections(&page) {
        render_section(&mut roff, &section);
    }
    roff.to_writer(&mut out)?;
    see_also(&page, siblings).to_writer(&mut out)?;
    let mut author = Roff::new();
    author.control("SH", ["AUTHOR"]);
    author.text([roman(env!("CARGO_PKG_AUTHORS").replace(':', ", "))]);
    author.to_writer(&mut out)?;
    Ok((format!("{page}.1"), out))
}

/// Every page as (file name, contents): `voltkit.1` plus one per visible subcommand.
/// Nested subcommands such as `daemon status` are documented on their parent's page.
pub fn pages() -> Result<Vec<(String, Vec<u8>)>> {
    let mut root = Cli::command().disable_help_subcommand(true);
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

pub fn generate(dir: &Path) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    pages()?
        .into_iter()
        .map(|(name, bytes)| {
            let path = dir.join(name);
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
        let expected = pages().unwrap();
        for (name, bytes) in &expected {
            let committed = std::fs::read(dir.join(name)).unwrap_or_default();
            assert!(committed == *bytes, "man/{name} is stale; run `cargo run -- manpages man`");
        }
        let on_disk = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(on_disk, expected.len(), "man/ has pages for commands that no longer exist");
    }
}
