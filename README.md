<img src="docs/icon.png" width="128" alt="PowerLinux icon: a lightning bolt crossing a wrench">

# PowerLinux

**A unified suite of micro-utilities for Linux, in one small binary.**

Linux users usually install five to ten separate applets to get basics like screen OCR, a color
picker, an image compressor and text expansion. PowerLinux puts them in one GTK4/libadwaita control
center. It needs no terminal tweaking and no desktop-specific extensions.

| Module | Shortcut | Status |
|---|---|---|
| **Text Extractor (OCR)**: drag a box around any on-screen text and it's copied to the clipboard | `Super+Shift+T` | ✅ GUI, CLI, hotkey |
| **Color Picker**: screen eyedropper, HEX/RGB/HSL conversion, WCAG contrast check, saved history | `Super+Shift+C` | ✅ GUI, CLI, hotkey |
| **Asset Compressor**: drop PNG/JPEG/WebP/MP4 files to strip metadata and shrink them | drag & drop | ✅ GUI + CLI |
| **Text Expansion**: snippets with `{date}`, `{clipboard}`, `{arg:name}` variables, from a searchable picker or typed triggers like `;date` | `Super+Shift+E` | ✅ picker · ✅ typed triggers (opt-in) |

## Install (from source)

Requirements: Rust 1.85+, GTK ≥ 4.14, libadwaita ≥ 1.5.

```bash
# Debian/Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev
# Fedora
sudo dnf install gtk4-devel libadwaita-devel
# Arch
sudo pacman -S gtk4 libadwaita
```

```bash
cargo install --path .     # installs to ~/.cargo/bin/powerlinux
powerlinux
```

Install the binary before you set up shortcuts or autostart: both record the binary's path.

To add PowerLinux to your app grid with its icon:

```bash
install -Dm644 data/icons/io.github.powerlinux.PowerLinux.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.powerlinux.PowerLinux.svg
install -Dm644 data/icons/io.github.powerlinux.PowerLinux-symbolic.svg ~/.local/share/icons/hicolor/symbolic/apps/io.github.powerlinux.PowerLinux-symbolic.svg
sed "s|^Exec=powerlinux|Exec=$HOME/.cargo/bin/powerlinux|" data/io.github.powerlinux.PowerLinux.desktop > ~/.local/share/applications/io.github.powerlinux.PowerLinux.desktop
```

## Background service & global shortcuts

```bash
powerlinux daemon                    # run in the foreground
powerlinux daemon enable-autostart   # start at every login
powerlinux daemon status             # running? which shortcut backend?
powerlinux daemon stop
```

The daemon owns `io.github.powerlinux.Daemon` on the session bus, so only one runs at a time.
It binds the hotkeys from your config in one of these ways:

| Desktop | How the hotkeys are bound |
|---|---|
| GNOME 48+, KDE Plasma 5.27+, Hyprland (xdg-desktop-portal ≥ 1.19) | **GlobalShortcuts portal**: the desktop asks you to approve them once |
| Older GNOME (e.g. 46) and GNOME-based distros | `powerlinux shortcuts install` adds them to *Settings → Keyboard → Custom Shortcuts* (`--dry-run` previews; `shortcuts remove` undoes) |
| Sway, river, others | Bind `powerlinux trigger ocr` / `powerlinux trigger color` in your compositor config |

`powerlinux trigger <action>` (`ocr`, `color` or `snippets`) sends the action to the running daemon, or runs it directly if
no daemon is running. Results and errors appear as desktop notifications.

## Text expansion

**Snippet picker** (no setup): press `Super+Shift+E`, type to search, press Enter. Snippets with
`{arg:NAME}` parameters show a small form. The result is pasted into the app you were using, and
your clipboard is put back afterwards. The first paste asks for remote-desktop permission (that's
how Wayland lets an app press Ctrl+V for you), and the permission is remembered.

**Typed triggers** (opt-in): type `;date` in any app and it turns into `2026-10-27`. Wayland hides
keystrokes from other apps, so this needs a one-time root setup:

```bash
powerlinux expander setup          # shows the commands and what they change
powerlinux expander setup --apply  # runs them with sudo; then log out and back in
powerlinux expander enable
```

Setup adds you to the `input` group, which lets *any* program you run read keyboard input. PowerLinux
keeps only the last few characters in memory and never logs or stores what you type. See
`man powerlinux-expander` for details and how to undo it.

For terminals, which paste with Ctrl+Shift+V, set `paste_shortcut` in the `[expander]` config section.

### Runtime helpers

PowerLinux calls standard tools when they are installed, so it adds no heavy dependencies of its own.
Check what's missing, with the install command for your distro, at any time:

```bash
powerlinux doctor
```

The control center shows a banner when a feature needs attention.

| Feature | Tools |
|---|---|
| OCR | `tesseract` (+ language packs such as `tesseract-ocr-eng`) |
| Region capture, eyedropper | the desktop's Screenshot portal; region capture falls back to `grim`+`slurp`, `gnome-screenshot`, `spectacle` or `maim` |
| Clipboard (daemon, CLI) | `wl-clipboard` (Wayland) or `xclip` (X11). Without it, the result is still shown in a notification |
| Compression | `oxipng` (PNG), `jpegoptim` (JPEG), `cwebp` (WebP), `ffmpeg` (MP4) |

## Command line

Every module also works headless, so you can bind it to a shortcut in any desktop:

```bash
powerlinux                       # open the control center
powerlinux ocr --lang eng        # select a region → text on clipboard
powerlinux pick-color            # eyedropper → HEX on clipboard
powerlinux color '#3584e4'       # HEX / RGB / HSL + contrast vs white/black
powerlinux compress *.png --quality 75
powerlinux expand 'Hi {arg:who}, {date:%A}' --arg who=Sam
```

Full documentation is in the man pages: `man powerlinux`, `man powerlinux-expand`, and so on.

### Man pages

The pages in [`man/`](man/) are generated from the CLI definitions, so they always match
`--help`. Install them next to the binary, where `man` finds them automatically:

```bash
mkdir -p ~/.cargo/share/man/man1 && cp man/*.1 ~/.cargo/share/man/man1/
```

After changing a command or its help text, regenerate them; `cargo test` fails until you do:

```bash
cargo run -- manpages man
```

## Configuration

`~/.config/powerlinux/config.toml`. Every key is optional; see
[`data/config.example.toml`](data/config.example.toml).

## Project layout

```
src/
  main.rs            CLI entry point (clap) → GUI when run with no subcommand
  app.rs             GTK4 / libadwaita control center
  daemon.rs          background service: D-Bus API, portal shortcuts, autostart
  actions.rs         hotkey actions (OCR, eyedropper) with notifications
  shortcuts.rs       accelerator conversion + GNOME keybinding fallback
  picker.rs          snippet picker popup
  expander.rs        typed triggers (evdev keyboard reading, uinput virtual keyboard)
  paste.rs           paste into the focused app, restoring the clipboard
  portal.rs          XDG portal calls (screenshot, PickColor) and notifications
  config.rs          TOML config with defaults
  system.rs          Wayland/X11 helpers: clipboard, tool discovery, process runner
  modules/
    ocr.rs           region capture + Tesseract
    color.rs         parsing, HSL, WCAG luminance & contrast
    compress.rs      format detection + per-format compression pipelines
    snippets.rs      snippet template engine
  manpages.rs        man page generator (clap_mangen + hand-written sections)
man/                 generated man pages
docs/ARCHITECTURE.md design notes and roadmap
```

## Contributing

```bash
cargo test
cargo clippy
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the D-Bus API and the roadmap. The
next big pieces are packaging (Flatpak, .deb) and a settings screen.

## Author

Created by **Alexis Rios-Negron**.

## License

MIT © Alexis Rios-Negron
