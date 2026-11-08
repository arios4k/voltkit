# Voltkit Manual

**Voltkit** is a native GTK4/libadwaita app for Linux that puts fifteen everyday desktop tools in one window, with one set of keyboard shortcuts, and every one of them also works from the terminal. This manual explains every feature of the app, page by page, with the exact controls, shortcuts and commands.

![Voltkit's Color Picker, one of the fifteen tools](shot:color)

## How to read this manual

- **New here?** Start with [Install](install) and [A tour of the app](tour).
- **Looking for one tool?** Pick it in the left menu. Each tool page follows the same pattern: what it is for, how to open it, every control on its page, the keyboard shortcut and terminal command, and troubleshooting.
- **Looking for a command or a setting?** See the [Command-line reference](cli), [Global shortcuts and the background service](daemon) and [Configuration file](config).
- **Something is not working?** See [Troubleshooting and `voltkit doctor`](troubleshooting).
- **Updated recently?** See [What's new](whats-new) for the changes in every release.
- Press <kbd>/</kbd> anywhere in the manual to search it.

## The fifteen tools

| Tool | What it does | Shortcut |
|---|---|---|
| [App Permissions](permissions) | Review, change or forget the answers you gave apps that asked for screenshots, the camera, your location and more | — |
| [Asset Compressor](compress) | Strip metadata from and shrink PNG, JPEG, WebP and MP4 files | drag and drop |
| [Color Picker](color) | Eyedropper for any pixel, HEX/RGB/HSL conversion, WCAG contrast and history | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> |
| [GPU Tuner](gpu) | Monitor NVIDIA, AMD and Intel graphics cards and tune power, fans, clocks and voltage safely | — |
| [Hosts & Environment](system) | Switch `/etc/hosts` entries on and off and manage session environment variables | — |
| [Keep Awake](awake) | Stop the computer from sleeping or locking for a while | — |
| [QR Code](qr) | Read a QR code on screen, or make one from text, a link or Wi-Fi details | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd> |
| [Screen Ruler](ruler) | Freeze the screen and measure anything in pixels | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> |
| [Shortcut Detective](shortcuts) | Find who owns a keyboard shortcut, find clashes, change shortcuts | — |
| [Smart Paste](smartpaste) | Paste the clipboard converted: JSON, tables, clean links, letter case | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd> |
| [System Report](sysreport) | Your hardware explained, with driver, drive and temperature checks and a shareable report | — |
| [Text Expansion](snippets) | Searchable snippets and optional typed triggers such as `;date` | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> |
| [Text Extractor](ocr) | OCR: drag a box around any text on screen and copy it | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> |
| [What's Using This?](using) | See which apps keep a file, folder, drive or port busy, and close them | — |
| [Why Didn't It Sleep?](sleep) | What keeps the computer awake, what woke it, and how much battery sleep used | — |

## Principles

- **Wayland first.** Voltkit uses the desktop's own portals for screenshots, the eyedropper and pasting. There are no X11 hacks and no shell extensions.
- **Private by design.** Everything runs on your computer. Voltkit makes no network requests and sends no telemetry. The only things that leave your computer are the links you click yourself, such as this manual.
- **Safe by default.** Originals are never modified, system entries are protected, every hosts save keeps a backup, and every GPU change is undone unless you confirm it.
- **Scriptable.** Every tool is also a command, with a man page, so you can bind it to a key or use it in a script.
- **In your language.** The app follows your system language: English, Spanish, German, French and Simplified Chinese. This manual is in English.

## Getting help

- In the app, press <kbd>F1</kbd> or click the **?** button in the header bar to open the page of this manual for the tool you are using.
- In a terminal: `voltkit --help`, `voltkit <command> --help` and `man voltkit`.
- Found a bug or have an idea? [Open an issue](https://github.com/arios4k/voltkit/issues).
