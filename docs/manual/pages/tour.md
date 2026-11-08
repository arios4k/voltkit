# A tour of the app

Open **Voltkit** from your apps. This page explains the window, the menu, the banners and the habits shared by every tool.

![The About dialog](shot:about)

## The window

- **The sidebar** on the left lists the fifteen tools, in alphabetical order for your language, each with an icon. Select one to open its page. Hover over a tool to see a one-line description.
- **The page** shows the tool's name and a short description at the top, then its controls. Pages use two columns on a wide window and one column on a narrow one; every page scrolls on its own.
- **On a narrow window** (under about 600 pixels) the sidebar becomes a list you go back to with the window's back button, like on a phone.
- The window follows your system's **light or dark** appearance.

## The header buttons

| Button | What it does |
|---|---|
| **?** (Help) | Opens this manual in your web browser, on the page for the tool you are using. The shortcut is <kbd>F1</kbd>. |
| **☰** (Main Menu) | **Help & Manual**, **About Voltkit** and **Quit** |

### Help & Manual

Opens this manual's home page, `https://arios4k.github.io/voltkit/manual/`.

### About Voltkit

Shows the version (click it to copy the version information), links to the project and to *Report an issue*, links to other apps by the same author, the author's name, and the **MIT License**, which is free to use, modify and share.

### Quit

<kbd>Ctrl</kbd>+<kbd>Q</kbd> closes the window. Voltkit's **background service keeps running**, so the keyboard shortcuts, Keep Awake and typed triggers keep working. To stop the service too, run `voltkit daemon stop`.

## Banners

A banner may appear at the top of a page:

- **The background service is off, so keyboard shortcuts won't work** with a **Start** button. Click it to start the service. See [Global shortcuts and the background service](daemon).
- **N feature(s) need attention** with a **Details** button. It lists what is missing, for example a helper program or a language pack, and offers **Copy Command**: the exact install command for your distribution, ready to paste in a terminal. The full report is `voltkit doctor`. See [Troubleshooting](troubleshooting).

## Notifications and toasts

- Results of shortcuts (copied text, a picked color) appear as **desktop notifications**, because the window may be closed.
- Actions in the window show a short **toast** at the bottom (*Copied #7C3AED*, *Saved*…). Some toasts have an **Undo** button.

## Keyboard shortcuts

Global shortcuts work from any app while the background service runs:

| Shortcut | Action |
|---|---|
| <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> | [Text Extractor](ocr): drag a box around text |
| <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | [Color Picker](color): pick a color |
| <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> | [Text Expansion](snippets): open the snippet picker |
| <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd> | [QR Code](qr): scan a code on screen |
| <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> | [Screen Ruler](ruler): measure on screen |
| <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd> | [Smart Paste](smartpaste): paste converted |

You can change them in the [configuration file](config) or with [Shortcut Detective](shortcuts). Inside the app: <kbd>F1</kbd> opens help and <kbd>Ctrl</kbd>+<kbd>Q</kbd> quits.

## Language

Voltkit follows your session's language: **English, Spanish, German, French and Simplified Chinese**. To try another one for Voltkit only, start it from a terminal:

```bash
LANGUAGE=fr voltkit    # or es, de, zh_CN, en
```

## Accessibility

Controls have text labels for screen readers, decorative icons and charts are hidden from them (charts also have a description), disabled controls say why they are disabled, and everything can be reached with the keyboard. If you find something that does not work with your assistive technology, please [open an issue](https://github.com/arios4k/voltkit/issues).

## Where Voltkit keeps things

| What | Where |
|---|---|
| Settings and snippets | `~/.config/voltkit/config.toml` |
| GPU profiles you edit | `~/.config/voltkit/gpu-profiles.json` |
| Environment variables you set | `~/.config/environment.d/90-voltkit.conf` |
| Color history, hardware snapshots, GPU sessions | `~/.local/share/voltkit/` |
| Hosts file backup | `/etc/hosts.voltkit-backup` |
| GPU service settings | `/etc/voltkit/` |

See [Configuration file](config).
