# PowerLinux

**A unified suite of micro-utilities for Linux, in one small binary.**

Linux users usually install five to ten separate applets to get basics like screen OCR, a color
picker, an image compressor and text expansion. PowerLinux puts them in one GTK4/libadwaita control
center. It needs no terminal tweaking and no desktop-specific extensions.

| Module | Shortcut | Status |
|---|---|---|
| **Text Extractor (OCR)**: drag a box around any on-screen text and it's copied to the clipboard | `Super+Shift+T` | ✅ GUI + CLI |
| **Color Picker**: HEX/RGB/HSL conversion, WCAG contrast check, history | `Super+Shift+C` | ✅ converter, contrast, history · 🚧 screen eyedropper |
| **Asset Compressor**: drop PNG/JPEG/WebP/MP4 files to strip metadata and shrink them | drag & drop | ✅ GUI + CLI |
| **Text Expansion**: snippets with `{date}`, `{clipboard}`, `{arg:name}` variables | typed triggers | ✅ template engine · 🚧 background expander |

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
cargo build --release
./target/release/powerlinux
```

### Runtime helpers

PowerLinux calls standard tools when they are installed, so it adds no heavy dependencies of its own:

| Feature | Tools |
|---|---|
| OCR | `tesseract` (+ language packs such as `tesseract-ocr-eng`) |
| Region capture | Wayland: `grim`+`slurp` (wlroots), `gnome-screenshot`, or `spectacle` · X11: `maim` |
| Clipboard (CLI) | `wl-clipboard` (Wayland) or `xclip` (X11) |
| Compression | `oxipng` (PNG), `jpegoptim` (JPEG), `cwebp` (WebP), `ffmpeg` (MP4) |

## Command line

Every module also works headless, so you can bind it to a shortcut in any desktop:

```bash
powerlinux                       # open the control center
powerlinux ocr --lang eng        # select a region → text on clipboard
powerlinux color '#3584e4'       # HEX / RGB / HSL + contrast vs white/black
powerlinux compress *.png --quality 75
powerlinux expand 'Hi {arg:who}, {date:%A}' --arg who=Sam
```

## Configuration

`~/.config/powerlinux/config.toml`. Every key is optional; see
[`data/config.example.toml`](data/config.example.toml).

## Project layout

```
src/
  main.rs            CLI entry point (clap) → GUI when run with no subcommand
  app.rs             GTK4 / libadwaita control center
  config.rs          TOML config with defaults
  system.rs          Wayland/X11 helpers: clipboard, tool discovery, process runner
  modules/
    ocr.rs           region capture + Tesseract
    color.rs         parsing, HSL, WCAG luminance & contrast
    compress.rs      format detection + per-format compression pipelines
    snippets.rs      snippet template engine
docs/ARCHITECTURE.md design notes and roadmap
```

## Contributing

```bash
cargo test
cargo clippy
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the roadmap. Global shortcuts through the XDG
portal and the background snippet expander are the next big pieces.

## License

MIT
