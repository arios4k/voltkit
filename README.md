<div align="center">

<img src="docs/icon.png" width="128" alt="Voltkit icon: a lightning bolt crossing a wrench">

# Voltkit

**English** · [Español](README.es.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [简体中文](README.zh-CN.md)

**Power tools for your Linux desktop.**
Screen OCR, a QR code scanner, a color picker, a screen ruler, an image &amp; video compressor, a text expander,
Smart Paste, Keep Awake, a batch file renamer, an image resizer, a zoom &amp; annotate tool, a hosts &amp; environment editor, a “what's using this?” finder, an app permissions
manager, a sleep troubleshooter, a system report, a shortcut detective and a GPU tuner, in one app.

[![Latest release](https://img.shields.io/github/v/release/arios4k/voltkit?label=release&color=7c3aed)](https://github.com/arios4k/voltkit/releases/latest)
[![CI](https://github.com/arios4k/voltkit/actions/workflows/ci.yml/badge.svg)](https://github.com/arios4k/voltkit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Install with apt](https://img.shields.io/badge/install-apt-orange)](#install-ubuntu-debian-and-derivatives)
[![Install on Fedora](https://img.shields.io/badge/install-Fedora-51a2da)](#install-fedora)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-dea584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Support on PayPal](https://img.shields.io/badge/PayPal-Support-00457C?logo=paypal&logoColor=white)](https://paypal.me/AlexisRios)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.7.0">
  <img src="docs/screenshots/color-light.png?v=0.7.0" alt="Voltkit's color picker showing #7C3AED with its HEX, RGB and HSL values and WCAG contrast ratings">
</picture>

</div>

## One app instead of eighteen

Getting these basics on Linux usually means hunting down a separate applet for each job,
each with its own look, settings and shortcuts. Voltkit puts them in one native GTK4 app with one set of
hotkeys:

| Instead of… | Voltkit gives you | Shortcut |
|---|---|---|
| nothing: there was no screen for it | **App Permissions**: change or forget the answers you gave apps asking for screenshots, the camera, your location and more | — |
| a separate image compressor, plus ffmpeg incantations | **Asset Compressor**: drop PNG, JPEG, WebP or MP4 files to strip metadata and shrink them | drag &amp; drop |
| a separate color picker | **Color Picker**: an eyedropper for any pixel, HEX/RGB/HSL, WCAG contrast check and history | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> |
| shell loops with `mv` or a separate renamer | **Batch Rename**: rename many files at once with search and replace, regular expressions and numbering, with a preview and undo | — |
| a separate image resizer | **Image Resizer**: resize many pictures at once, keeping their shape, without touching the originals | — |
| a separate GPU tuning tool, plus vendor utilities | **GPU Tuner**: monitor NVIDIA, AMD and Intel graphics cards and change power limits, fans and fan curves, temperature targets, clocks and voltage, with every change previewed and undone unless you confirm it | — |
| editing `/etc/hosts` and `~/.profile` in a terminal | **Hosts & Environment**: switch hosts entries on and off, and manage your session's environment variables | — |
| a shell extension to keep the screen on | **Keep Awake**: stop your computer from sleeping or locking for 30 minutes to 4 hours, or until you turn it off | — |
| a separate presentation pen | **Zoom & Annotate**: freeze the screen, zoom in and draw on it with a pen, arrows, boxes and text | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> |
| a QR scanner on your phone or a QR website | **QR Code**: read a QR code anywhere on screen, or turn text, links and Wi-Fi details into one | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd> |
| a separate screen ruler, or screenshots and an image editor | **Screen Ruler**: freeze the screen and measure anything in pixels | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> |
| guessing why a keyboard shortcut does nothing | **Shortcut Detective**: press any shortcut to see what owns it, find clashes, pick a free combination and change it | — |
| online converters for JSON, tables and letter case | **Smart Paste**: paste the clipboard converted: tidy JSON, a spreadsheet table as Markdown, a link without tracking, text from a PDF without broken lines, or the text in a copied image | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd> |
| a pile of terminal commands to answer “what hardware do I have?” | **System Report**: your hardware explained, with driver and drive-health checks and fixes, temperatures, firmware updates, memory upgrades, what changed since an update, and a report that's safe to share | — |
| a separate text expander | **Text Expansion**: a searchable snippet picker, plus optional typed triggers like `;date` | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> |
| a separate OCR app | **Text Extractor**: drag a box around any text on screen, even in videos and images, and it lands on your clipboard | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> |
| `lsof`, `fuser` and `ss` in a terminal | **What's Using This?**: see which apps keep a file, folder, drive or port busy, and close them | — |
| digging through system logs in a terminal | **Why Didn't It Sleep?**: see what keeps the computer awake, what woke it up, and how much battery it used asleep | — |

## Features

### 🛡️ App Permissions

When an app asks to take screenshots, use the camera or microphone, or see your location, the desktop
remembers your answer. App Permissions lists them by app: flip a switch to
change one, or forget it so the app asks again.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/permissions-dark.png?v=0.7.0">
  <img src="docs/screenshots/permissions-light.png?v=0.7.0" alt="App Permissions page listing apps with their camera and screenshot permissions">
</picture>

### 📦 Asset Compressor

Drop files on the window or run `voltkit compress`. Voltkit strips location data and other metadata,
compresses with the best tool for each format, and writes `name.min.ext` next to the original, which
it never touches. In our tests: JPEG −43%, PNG −23% (lossless), WebP −60%, MP4 −77%.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/compress-dark.png?v=0.7.0">
  <img src="docs/screenshots/compress-light.png?v=0.7.0" alt="Asset Compressor with four compressed files and their size savings">
</picture>

### ✏️ Batch Rename

Drop files or folders, say what to change, and watch the **preview** before anything happens: search and replace, regular expressions with `$1` groups, numbering with `{n:3}`, letter case, and the name or only the extension. Conflicts (two files with one name, a name that is taken) are named first, a batch is renamed **all or nothing**, and the last batch can be **undone**. The same works in a terminal: `voltkit rename *.jpg --find IMG --replace Holiday --apply`.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/rename-dark.png?v=0.11.0">
  <img src="docs/screenshots/rename-light.png?v=0.11.0" alt="Batch Rename with a preview of five photos being renamed">
</picture>

### 🖼️ Image Resizer

Drop pictures, pick a size (a longest side, a percent, or a width and height that fit, fill or stretch) and get resized copies beside the originals, which are never touched. Photos come out upright, WebP and JPEG quality is yours to set, and the size never grows unless you allow it. `voltkit resize photos/*.jpg --longest-side 1280`.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/resize-dark.png?v=0.11.0">
  <img src="docs/screenshots/resize-light.png?v=0.11.0" alt="Image Resizer with three pictures resized to 1920 pixels">
</picture>

### 🎨 Color Picker

Grab any pixel with <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> and the HEX code is already on your
clipboard. The color page converts between HEX, RGB and HSL, rates contrast against white and black by
WCAG standards (AA / AAA), and remembers your recent colors.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/color-dark.png?v=0.7.0">
  <img src="docs/screenshots/color-light.png?v=0.7.0" alt="Color Picker with a live preview, HEX/RGB/HSL formats, contrast ratings, and saved colors">
</picture>

### 🎛️ GPU Tuner

Watch your NVIDIA, AMD and Intel graphics cards live, with history, and change what the hardware allows: the power
limit, a fixed fan speed or a fan curve, the temperature a card aims for, clock offsets or limits, and voltage.
**What this card supports** lists every control the way the driver reports it, and says why one is missing and what
could enable it (a laptop GPU that hides its fan, or an AMD card that needs `amdgpu.ppfeaturemask`). Changes are
checked against the card's own limits, previewed, applied through a root helper, and **undone unless you confirm
them**: closing the app or a crash reverts them too. Saved settings are applied at startup by a background service,
which skips a profile that never finished starting, so a bad one can't lock you in a loop. `voltkit gpu` prints the same
report in a terminal.

Edit fan curves by dragging points on a graph, and choose which temperature they follow and which fans they drive. Save settings as **profiles** that switch by themselves: on battery, while an app runs, under a sustained load, or when the card runs hot. **Record a session** while a game or benchmark runs and compare two profiles side by side: temperature, power, energy and efficiency (active clock per watt, or frames per watt when frames are recorded). `voltkit gpu check` compares Voltkit's readings with `nvidia-smi` and measures what monitoring costs.

More for gamers and tinkerers: a profile can switch on while a **Steam or Proton game** runs (type `steam:<App ID>` as the program); the live readings say **why the clock is held back** (power limit or heat, NVIDIA); **Keep under a temperature** drives the fan to a target instead of a hand-drawn curve; sessions show **what they cost** at your electricity price; **Suggest a power limit** (`voltkit gpu advise`) finds the lowest limit that keeps almost all the speed from sessions you recorded; profiles can be **shared as one line of text** and imported with checks; and **Idle power check** (`voltkit gpu idle`) explains why the video memory stays at a high clock with nothing running, a common multi-monitor problem on Linux.

Tried it on your card? A [hardware report](https://github.com/arios4k/voltkit/issues/new?template=gpu-hardware-report.yml) of what worked and what didn't is the most useful help right now.

Applying changes is new and has so far been tested against simulated hardware only; see
[docs/GPU_TUNER.md](docs/GPU_TUNER.md) for what works, how it stays safe, and what isn't built yet.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-light.png?v=0.10.4" alt="GPU Tuner, Monitor tab: live readings with history, why the clock is held back, and what the card supports (simulated data)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-2-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-2-light.png?v=0.10.4" alt="GPU Tuner, Tune tab: power limit, fan curve editor and the other controls (simulated data)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-3-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-3-light.png?v=0.10.4" alt="GPU Tuner, Profiles tab: saved profiles, automatic rules, sharing and import (simulated data)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-4-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-4-light.png?v=0.10.4" alt="GPU Tuner, Sessions tab: recorded sessions, electricity price, power-limit suggestion and comparison (simulated data)">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/gpu-vf-dark.png?v=0.10.4">
  <img src="docs/screenshots/gpu-vf-light.png?v=0.10.4" alt="GPU Tuner voltage and frequency curve editor with draggable points (simulated data)">
</picture>

<sub>The screenshot uses simulated data from made-up cards.</sub>

### 📝 Hosts & Environment

Switch `/etc/hosts` entries on and off, add `myapp.test → 127.0.0.1` for local development, and set
environment variables like `EDITOR` or an extra `PATH` folder for your whole desktop session. System
entries are protected, and every hosts save keeps a backup.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/system-dark.png?v=0.7.0">
  <img src="docs/screenshots/system-light.png?v=0.7.0" alt="Hosts & Environment page with hosts entries and switches">
</picture>

### ☕ Keep Awake

Watching, presenting or downloading? Turn on Keep Awake for 30 minutes, a few hours or until you turn it
off, and your computer won't sleep or lock. It keeps going after you close Voltkit and switches itself off
when the time is up.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/awake-dark.png?v=0.7.0">
  <img src="docs/screenshots/awake-light.png?v=0.7.0" alt="Keep Awake with its current status and duration controls">
</picture>

### 📷 QR Code

Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd> and drag a box around a QR code in a video, a slide or a web
page: its text or link is on your clipboard. Going the other way, type a link, some text or your Wi-Fi
details and scan the code with your phone. Copy it as an image or save it as a PNG to print. Add your own small logo
to the middle if you like: Voltkit reads the code back to make sure it still scans.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/qr-dark.png?v=0.7.0">
  <img src="docs/screenshots/qr-light.png?v=0.7.0" alt="QR Code page with a generated code for a link">
</picture>

### 📏 Screen Ruler

<kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> freezes the screen. Drag to measure widths and heights in real
pixels (hold Shift for a straight line), and press Ctrl+C to copy the size.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/ruler-dark.png?v=0.7.0">
  <img src="docs/screenshots/ruler-light.png?v=0.7.0" alt="Screen Ruler with its launch action and measurement controls">
</picture>

### 🔍 Zoom & Annotate

<kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> freezes the screen. Scroll to zoom, drag to draw, hold Shift for a line, Ctrl for a rectangle, Tab for an ellipse, Ctrl+Shift for an arrow, press T to type, R/G/B/O/Y/P for color, W/K for a white or black board, and Ctrl+C or Ctrl+S to copy or save the picture.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/zoom-overlay-dark.png?v=0.11.0">
  <img src="docs/screenshots/zoom-overlay-light.png?v=0.11.0" alt="Zoom and Annotate: an arrow, a circle, text and a highlighter drawn on a frozen screen">
</picture>

### ⌨️ Shortcut Detective

Pressed a shortcut and nothing happened, or the wrong thing did? Press it in **Shortcut Detective** to see every
action that uses it: the window manager, GNOME Shell, system keys, your custom shortcuts, shell extensions and
Voltkit itself. Clashes are listed for you, each with free combinations to switch to, and any shortcut can be
changed, turned off or reset right there. Works on GNOME and the desktops built like it (Cinnamon, Budgie, MATE). A **cheat sheet** of every shortcut, grouped by owner, opens with <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>/</kbd>.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/shortcuts-dark.png?v=0.7.0">
  <img src="docs/screenshots/shortcuts-light.png?v=0.7.0" alt="Shortcut Detective showing that Super+V is used by two actions">
</picture>

### 📋 Smart Paste

Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd> instead of <kbd>Ctrl</kbd>+<kbd>V</kbd> and pick how to paste what you
copied: JSON tidied or compacted, a spreadsheet range as a Markdown table or CSV, a link without its tracking
parameters, a paragraph from a PDF without its broken lines, a list with bullets or numbers, or text in UPPERCASE,
Title Case, snake_case and more. Copied a screenshot? Paste the text in it. Only the conversions that fit what you
copied are offered, each with a preview, and your clipboard is left as it was. The same conversions work in
scripts: `wl-paste | voltkit convert markdown-table`.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/smartpaste-dark.png?v=0.7.0">
  <img src="docs/screenshots/smartpaste-light.png?v=0.7.0" alt="Smart Paste page turning a spreadsheet range into a Markdown table and CSV">
</picture>

### 🖥️ System Report

Not just a list of specs: **System Report** checks each piece of hardware and says what it means. Is the graphics
card on the right driver (and if not, the exact command to fix it)? Is the drive healthy, and how worn? Is the
processor running hot or slowing itself down? Is a BIOS or device firmware update waiting? How many memory slots
are free, and how much can this computer take?

Voltkit also keeps a daily snapshot of the hardware, drivers, kernel and firmware, so when something breaks after
an update, **What Changed** shows exactly what's different. When you need help, **Copy Report** gives you a tidy
summary for a forum or bug report, with no serial numbers, names or network addresses.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/sysreport-dark.png?v=0.7.0">
  <img src="docs/screenshots/sysreport-light.png?v=0.7.0" alt="System Report page with checks, recent changes and the computer's hardware">
</picture>

### ⌨️ Text Expansion

Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd>, search, press Enter, and the snippet is pasted into
whatever app you were using. Snippets can include the date in any format, your clipboard, and
fill-in-the-blank fields. Prefer typing? Turn on typed triggers and `;sig` becomes your signature in any
app. Add, edit and delete snippets right in the app: Insert buttons add the date, time, clipboard or a
fill-in field, and a live preview shows exactly what will be pasted.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/snippets-dark.png?v=0.7.0">
  <img src="docs/screenshots/snippets-light.png?v=0.7.0" alt="Text Expansion with searchable snippets and insertion controls">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/picker-dark.png?v=0.7.0">
  <img src="docs/screenshots/picker-light.png?v=0.7.0" alt="Snippet picker listing snippets with live previews">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/snippet-editor-dark.png?v=0.7.0">
  <img src="docs/screenshots/snippet-editor-light.png?v=0.7.0" alt="Snippet editor with a trigger, text with Insert buttons for date, time, clipboard and fill-in fields, and a live preview">
</picture>

### 🔍 Text Extractor

A Wi-Fi password on a conference slide, a command in a video tutorial, an error in a screenshot: press
<kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd>, drag a box, and paste. Powered by Tesseract, with support for
[100+ languages](https://tesseract-ocr.github.io/tessdoc/Data-Files-in-different-versions.html).

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/ocr-dark.png?v=0.7.0">
  <img src="docs/screenshots/ocr-light.png?v=0.7.0" alt="Text Extractor showing text read from a photo of a conference slide: a Wi-Fi password and a link">
</picture>

### 🔎 What's Using This?

“Target is busy” when you eject a USB stick, “address already in use” when you start a dev server: pick the
drive, file, folder or port and see which apps hold it, with what they have open. Close an app with one click
(it's asked to quit nicely first, with Force Quit only if it won't), or let **Eject** close them and eject for
you. Parts of the system itself are never offered for closing.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/using-dark.png?v=0.7.0">
  <img src="docs/screenshots/using-light.png?v=0.7.0" alt="What's Using This? page listing the apps that keep a USB drive busy">
</picture>

### 🌙 Why Didn't It Sleep?

Laptop warm in your bag, or battery empty in the morning? See what's keeping the computer awake right now (an app,
a video, Keep Awake, a power setting, an external monitor), every sleep of the last week with what started and ended
it and how much battery it used, and the USB devices allowed to wake it, with a switch to stop the one that keeps
waking it up.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/sleep-dark.png?v=0.7.0">
  <img src="docs/screenshots/sleep-light.png?v=0.7.0" alt="Why Didn't It Sleep? page listing what keeps the computer awake and recent sleeps">
</picture>

## Why Voltkit

- **Built for Wayland.** It uses the desktop's own portals for screenshots, the eyedropper and pasting, so
  there are no X11 hacks and no shell extensions.
- **Private by design.** Everything runs locally, with no network access and no telemetry. Typed
  triggers keep only the last few characters in memory and never log or store what you type.
- **Small and native.** One 4 MB binary (a 1.4 MB package), GTK4 + libadwaita, light and dark mode.
- **Scriptable.** Every feature is also a command (`voltkit ocr`, `voltkit color '#3584e4'`,
  `voltkit compress *.jpg`), with man pages for all of them.
- **Self-checking.** `voltkit doctor` tells you exactly what's missing and the command to install it.
- **Signed packages.** Install and update through apt from a GPG-signed repository.
- **In your language.** English, Spanish, German, French and Simplified Chinese, following your system language.

## FAQ

**Does it work on KDE, Sway or Hyprland?**
Voltkit is developed and tested on GNOME (Zorin OS 18). It uses standard XDG portals, so KDE Plasma
and other desktops should work, but they haven't been tested yet. Reports are welcome! On desktops
without the GlobalShortcuts portal, bind `voltkit trigger ocr|color|snippets` to keys yourself.

**Is it safe to turn on typed triggers?**
They're off by default. Wayland hides keystrokes between apps, so typed triggers need access to the
keyboard, which you allow once with your password when you turn them on. Voltkit keeps only the last
few characters in memory and never saves or sends what you type. The snippet picker works without any
of that.

**How do I change the language?**
Voltkit follows your session's language (English, Spanish, German, French or Simplified Chinese). To try another
one for Voltkit only, start it from a terminal with `LANGUAGE=fr voltkit` (or `es`, `de`, `zh_CN`, `en`).

<sub>Screenshots taken on Zorin OS 18 with Voltkit’s light and dark palettes. Regenerate them with
`docs/screenshots/capture.sh` (pass `es`, `de`, `fr` or `zh_CN` to capture the translated app).</sub>

## Install (Ubuntu, Debian and derivatives)

For Ubuntu 24.04+, Debian 13+, Linux Mint 22, Zorin OS 18, Pop!_OS 24.04 and other derivatives
on 64-bit Intel/AMD:

```bash
sudo curl -fsSLo /usr/share/keyrings/voltkit-archive-keyring.gpg \
  https://arios4k.github.io/voltkit/voltkit-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/voltkit-archive-keyring.gpg] https://arios4k.github.io/voltkit stable main" \
  | sudo tee /etc/apt/sources.list.d/voltkit.list
sudo apt update && sudo apt install voltkit
```

apt installs the required tools (Tesseract, wl-clipboard, the desktop portal) along with Voltkit, and
updates arrive with `sudo apt upgrade`.

Then open Voltkit from your apps. That's all: it starts its background service (which also starts at
every login from now on) and, on GNOME, adds its keyboard shortcuts. `voltkit doctor` checks that
everything is in place. Typing triggers like `;date` anywhere is optional: turn on **Typed
triggers** on the Text Expansion page (it asks for your password once and works right away).

## Install (Fedora)

For Fedora 43 and newer on 64-bit Intel/AMD, from the [COPR repository](https://copr.fedorainfracloud.org/coprs/arios/voltkit/):

```bash
sudo dnf copr enable arios/voltkit
sudo dnf install voltkit
```

Then open Voltkit from your apps; it sets itself up the same way. Updates arrive with
`sudo dnf upgrade`.

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
cargo install --path .     # installs to ~/.cargo/bin/voltkit
voltkit
```

Install the binary before you set up shortcuts or autostart: both record the binary's path.

To add Voltkit to your app grid with its icon:

```bash
install -Dm644 data/icons/io.github.arios4k.Voltkit.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.arios4k.Voltkit.svg
install -Dm644 data/icons/io.github.arios4k.Voltkit-symbolic.svg ~/.local/share/icons/hicolor/symbolic/apps/io.github.arios4k.Voltkit-symbolic.svg
sed "s|^Exec=voltkit|Exec=$HOME/.cargo/bin/voltkit|" data/io.github.arios4k.Voltkit.desktop > ~/.local/share/applications/io.github.arios4k.Voltkit.desktop
```

## Background service & global shortcuts

```bash
voltkit daemon                    # run in the foreground
voltkit daemon enable-autostart   # start at every login
voltkit daemon status             # running? which shortcut backend?
voltkit daemon stop
```

The daemon owns `io.github.arios4k.Voltkit.Daemon` on the session bus, so only one runs at a time.
It binds the hotkeys from your config in one of these ways:

| Desktop | How the hotkeys are bound |
|---|---|
| GNOME 48+, KDE Plasma 5.27+, Hyprland (xdg-desktop-portal ≥ 1.19) | **GlobalShortcuts portal**: the desktop asks you to approve them once |
| Older GNOME (e.g. 46) and GNOME-based distros | `voltkit shortcuts install` adds them to *Settings → Keyboard → Custom Shortcuts* (`--dry-run` previews; `shortcuts remove` undoes) |
| Sway, river, others | Bind `voltkit trigger ocr` / `voltkit trigger color` in your compositor config |

`voltkit trigger <action>` (`ocr`, `color` or `snippets`) sends the action to the running daemon, or runs it directly if
no daemon is running. Results and errors appear as desktop notifications.

## Text expansion

**Managing snippets:** on the Text Expansion page, use **Add Snippet**, click a snippet to edit it, or
delete it (with Undo). Changes are saved to `~/.config/voltkit/config.toml` and reach typed triggers
immediately. If you edit that file by hand, run `voltkit daemon reload`.

**Snippet picker** (no setup): press `Super+Shift+E`, type to search, press Enter. Snippets with
`{arg:NAME}` parameters show a small form. The result is pasted into the app you were using, and
your clipboard is put back afterwards. The first paste asks for remote-desktop permission (that's
how Wayland lets an app press Ctrl+V for you), and the permission is remembered.

**Typed triggers** (opt-in): type `;date` in any app and it turns into `2026-10-27`. Turn on **Typed
triggers** on the Text Expansion page. Wayland hides keystrokes from other apps, so the first time it
asks for your password to allow keyboard access, and then works right away: no terminal, no logging
out. Turning it off offers to remove the access again. From a terminal, the same is
`voltkit expander setup --apply` and `voltkit expander remove-access`.

Keyboard access lets programs run by the person at the computer read the keyboard, not only Voltkit.
Voltkit keeps only the last few characters in memory and never logs or stores what you type. See
`man voltkit-expander` for details.

For terminals, which paste with Ctrl+Shift+V, set `paste_shortcut` in the `[expander]` config section.

### Runtime helpers

Voltkit calls standard tools when they are installed, so it adds no heavy dependencies of its own.
Check what's missing, with the install command for your distro, at any time:

```bash
voltkit doctor
```

The control center shows a banner when a feature needs attention.

| Feature | Tools |
|---|---|
| OCR | `tesseract` (+ language packs such as `tesseract-ocr-eng`) |
| Region capture, eyedropper | the desktop's Screenshot portal; region capture falls back to `grim`+`slurp`, `gnome-screenshot`, `spectacle` or `maim` |
| Clipboard (daemon, CLI) | `wl-clipboard` (Wayland) or `xclip` (X11). Without it, the result is still shown in a notification |
| Compression | built in for PNG; `jpegoptim` (JPEG), `cwebp` (WebP), `ffmpeg` (MP4) |

## Command line

Every module also works headless, so you can bind it to a shortcut in any desktop:

```bash
voltkit                       # open the control center
voltkit ocr --lang eng        # select a region → text on clipboard
voltkit pick-color            # eyedropper → HEX on clipboard
voltkit color '#3584e4'       # HEX / RGB / HSL + contrast vs white/black
voltkit compress *.png --quality 75
voltkit expand 'Hi {arg:who}, {date:%A}' --arg who=Sam
voltkit qr make https://example.com  # QR code in the terminal (or --output code.png)
voltkit qr scan               # read a QR code on screen
voltkit ruler                 # measure on screen
voltkit awake 1h30m           # keep the computer awake (on / off / no argument for status)
voltkit using /media/$USER/USB  # what keeps this drive busy? (or a port: voltkit using 3000)
voltkit sleep                 # why it didn't sleep, and what woke it
voltkit report                # this computer, its checks and recent changes (--memory: slots and upgrades)
voltkit shortcuts who super+v # what a shortcut does (also: shortcuts list, shortcuts clashes)
voltkit smart-paste           # paste the clipboard converted
wl-paste | voltkit convert json  # Smart Paste's conversions in scripts (voltkit convert lists them)
```

Full documentation is in the man pages: `man voltkit`, `man voltkit-expand`, and so on.

### Man pages

The pages in [`man/`](man/) are generated from the CLI definitions, so they always match
`--help`; the translations are in `man/es/`, `man/de/`, `man/fr/` and `man/zh_CN/`. Install them next to
the binary, where `man` finds them automatically:

```bash
mkdir -p ~/.cargo/share/man/man1 && cp man/*.1 ~/.cargo/share/man/man1/
```

After changing a command or its help text, regenerate them; `cargo test` fails until you do:

```bash
cargo run -- manpages man
```

## Configuration

`~/.config/voltkit/config.toml`. Every key is optional; see
[`data/config.example.toml`](data/config.example.toml).

## Translations

Voltkit is translated into Spanish, German, French and Simplified Chinese. The text lives in ordinary gettext files in [`po/`](po/), which
are built into the binary, so there's nothing extra to install. To add a language or fix a translation, see
[`po/README.md`](po/README.md).

## Project layout

```
src/
  main.rs            CLI entry point (clap) → GUI when run with no subcommand
  i18n.rs            translations: built-in .po catalogs, the system language
  app.rs             GTK4 / libadwaita control center
  daemon.rs          background service: D-Bus API, portal shortcuts, autostart
  actions.rs         hotkey actions (OCR, eyedropper) with notifications
  shortcuts.rs       accelerator conversion + GNOME keybinding fallback
  picker.rs          snippet picker popup
  snippets_ui.rs     Text Expansion page: snippet list and editor
  qr_ui.rs           QR Code page
  ruler.rs           Screen Ruler (frozen-screenshot overlay)
  snapshot.rs        shared start-up of the tools that work on a frozen screenshot
  zoom.rs            Zoom & Annotate (zoom and draw on the frozen screen)
  rename_ui.rs       Batch Rename page: files, preview, rename and undo
  resize_ui.rs       Image Resizer page
  bulk_cli.rs        `voltkit rename` and `voltkit resize`
  guide_ui.rs        the shortcut cheat sheet window
  awake.rs           Keep Awake (Inhibit portal, held by the daemon)
  awake_ui.rs        Keep Awake page
  system_ui.rs       Hosts & Environment page
  usage_ui.rs        What's Using This? page
  permissions_ui.rs  App Permissions page
  sleep_ui.rs        Why Didn't It Sleep? page
  gpu_ui.rs          GPU Tuner page: readings, support report, controls
  gpu_cli.rs         `voltkit gpu`, the root helper and the background service
  sysreport_ui.rs    System Report page
  detective_ui.rs    Shortcut Detective page
  smartpaste_ui.rs   Smart Paste window and page
  expander.rs        typed triggers (evdev keyboard reading, uinput virtual keyboard)
  paste.rs           paste into the focused app, restoring the clipboard
  portal.rs          XDG portal calls (screenshot, PickColor) and notifications
  config.rs          TOML config with defaults
  system.rs          Wayland/X11 helpers: clipboard, tool discovery, process runner
  modules/
    ocr.rs           region capture + Tesseract
    color.rs         parsing, HSL, WCAG luminance & contrast
    compress.rs      format detection + per-format compression pipelines
    rename.rs        batch rename: plan, conflicts, two-step apply, undo journal
    resize.rs        image resizing with GdkPixbuf
    annotate.rs      the drawing model of Zoom & Annotate: marks, undo, zoom view
    guide.rs         the shortcut cheat sheet's groups and filter
    snippets.rs      snippet template engine
    qr.rs            QR code reading and making
    hosts.rs         /etc/hosts parsing that preserves everything it doesn't edit
    envvars.rs       session environment variables (environment.d)
    usage.rs         finds and closes the processes using a path or port (/proc)
    permissions.rs   the desktop portal's permission store
    sleep.rs         sleep blockers, sleep history (journal, UPower) and wake devices
    gpu.rs           GPU detection, sensors and what each card supports (NVIDIA, AMD, Intel)
    gpu_tune.rs      validated, transactional GPU changes, fan curves, startup guard
    nvml.rs          NVIDIA management library binding
    sysreport.rs     hardware, checks, drive health, sensors, firmware, snapshots, SMBIOS memory
    keybindings.rs   every GSettings shortcut: owners, clashes, free combinations, changes
    smartpaste.rs    Smart Paste's text conversions
  manpages.rs        man page generator (clap_mangen + hand-written sections)
  screenshots.rs     renders the README screenshots (hidden `screenshots` command)
po/                  translations (Spanish, German, French, Simplified Chinese) and update.py to refresh them
man/                 generated man pages (man/es, man/de, man/fr, man/zh_CN: translated)
packaging/apt/        APT repository builder and its landing page
docs/screenshots/    README screenshots and capture.sh to regenerate them
docs/ARCHITECTURE.md design notes and roadmap
```

## Contributing

```bash
cargo test
cargo clippy
python3 po/update.py --check   # translations up to date and complete
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the D-Bus API and the roadmap. A settings screen is planned. Bug reports and testing on other desktops are
especially welcome.

## Releasing

1. Bump `version` in `Cargo.toml`, run `cargo run -- manpages man` and `python3 po/update.py`, and commit.
2. Tag and push: `git tag v0.2.0 && git push origin v0.2.0`.

The [Release workflow](.github/workflows/release.yml) builds the `.deb`, publishes a GitHub Release
with it, and rebuilds the signed APT repository on GitHub Pages. It signs with the key in the
`APT_SIGNING_KEY` secret (fingerprint `65E8 4B8D 114C 6147 9CCA 4DDC EC0F 6B66 E480 2360`). To build
the repository locally: `APT_KEY_ID=<fingerprint> packaging/apt/build-repo.sh target/debian site`.

## Author

Created by **Alexis Rios-Negron**.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/screenshots/about-dark.png?v=0.7.0">
  <img src="docs/screenshots/about-light.png?v=0.7.0" width="640" alt="Voltkit's About window showing the app icon, version and author Alexis Rios-Negron">
</picture>

## Support Voltkit

If Voltkit has been useful to you, please consider [making a donation of any amount](https://paypal.me/AlexisRios) to help keep the project running and support its continued development. Every contribution is appreciated.

The intention is to keep Voltkit free and available to everyone. Thank you for using it and for any support you can offer!

## License

MIT © Alexis Rios-Negron
