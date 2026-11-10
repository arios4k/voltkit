# What's new

This page lists what changed in every Voltkit release, newest first, with links to the full explanation of each feature. Your version is shown in **☰ → About Voltkit** and by `voltkit --version`.

> [!TIP] Update with `sudo apt upgrade` (Ubuntu, Debian and derivatives) or `sudo dnf upgrade` (Fedora). Your settings and snippets are kept.

## Version 0.11.1

*November 9, 2026*

- **Keyboard keys in Shortcut Detective.** Make Caps Lock an Escape or Ctrl key, swap Alt and Super, or turn on a Compose key, with one click. The change takes effect at once and is undone by choosing the first entry. See [Shortcut Detective](shortcuts#keyboard-keys).
- **A guide to typing accents.** The new manual page [Typing accents and symbols](accents) shows how to type é, ñ, ü, ç, € and more with the Compose key, and other ways to type special characters.

## Version 0.11.0

*November 9, 2026*

**Three new tools, and a shortcut cheat sheet.** Voltkit now has eighteen tools.

| Feature | How it works | Read more |
|---|---|---|
| Batch Rename | Search and replace, regular expressions, numbering with `{n:3}`, letter case, a preview that names every conflict, all-or-nothing renaming and undo. `voltkit rename` | [Batch Rename](rename) |
| Image Resizer | Add pictures, choose a size (or click one of the ideas, such as "Email or chat"), then click **Resize**. Copies go beside the originals. `voltkit resize` | [Image Resizer](resize) |
| Zoom & Annotate | Freeze the screen, zoom in and draw on it with a pen, lines, boxes, arrows and text, then copy or save the picture. <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd>. `voltkit zoom` | [Zoom & Annotate](zoom) |
| Shortcut cheat sheet | A **Cheat Sheet** button in Shortcut Detective and <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>/</kbd> open a searchable sheet of every shortcut, grouped by owner. `voltkit shortcut-guide` | [Shortcut Detective](shortcuts#the-cheat-sheet) |

- **Choose where annotated pictures are saved.** Zoom & Annotate saves in `Pictures/Voltkit` by default; **Change…** on its page picks another folder, and <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd> saves one picture anywhere. See [Zoom & Annotate](zoom#where-pictures-are-saved).
- **The author's other apps** are in their own "More from the same author" group in the About box.
- The words in the Screen Ruler page's diagram are centered.

## Version 0.10.5

*November 7, 2026*

- **A detailed manual.** This manual covers every tool, control, shortcut and command, with light and dark screenshots, search and a page for each tool. It is published at arios4k.github.io/voltkit/manual.
- **A Help button.** The **?** button in the header, **Help & Manual** in the main menu and <kbd>F1</kbd> open the page of this manual for the tool you are using. See [A tour of the app](tour#the-header-buttons).

## Version 0.10.4

*November 7, 2026*

**GPU Tuner is organized into tabs and gets new tools.**

| Feature | How it works | Read more |
|---|---|---|
| Monitor, Tune, Profiles and Sessions tabs | The long page is split into four tabs, each scrolling by itself | [GPU Tuner](gpu) |
| Why the clock is held back | A live **Held back by** row names the cause: the power limit, heat or another limit (NVIDIA). Sessions record it, and comparisons include it | [Monitor tab](gpu#held-back-by) |
| Idle power check | Explains why the video memory clock stays high with nothing running (several monitors, mixed or fast refresh rates). `voltkit gpu idle` | [Idle power check](gpu#idle-power-check) |
| Session cost | Type your electricity price and every session shows what it cost | [Sessions](gpu-sessions#what-a-session-cost) |
| Suggest a power limit | Finds the lowest limit that keeps almost all the speed, from sessions you recorded. `voltkit gpu advise` | [Sessions](gpu-sessions#suggest-a-power-limit) |
| Steam and Proton game rules | A profile can switch on while a game runs: type `steam:<App ID>` | [Profiles](gpu-profiles#choose-when-a-profile-switches-in) |
| Keep under a temperature | A fan mode where you pick a target and Voltkit builds the curve | [Tune](gpu-tune#fan) |
| Share a profile | Copy a profile as one line of text and import it with checks | [Profiles](gpu-profiles#share-a-profile) |

- **About** now fits without scrolling.

## Version 0.10.3

*November 7, 2026*

- **Other apps by the same author.** The About box links to ThermalPrint and Battery Health Center, with their official icons.

## Version 0.10.2

*November 6, 2026*

- **A clearer session comparison.** The comparison window is larger, has a **Close** button, and names **one better session in each category** instead of marking both. See [Sessions](gpu-sessions#compare-two-sessions).
- **Everything stays visible.** Controls and charts a card does not support stay in GPU Tuner, disabled, with the reason, instead of disappearing. See [Tune](gpu-tune).
- **Accessibility.** Names for switches, check boxes, the color swatch and the ruler canvas, so screen readers announce them.

## Version 0.10.1

*November 6, 2026*

- **A new graphics-card icon** for GPU Tuner.
- **Voltage and frequency curve chart.** On AMD cards with a full curve, drag points on a chart instead of typing numbers. See [Tune](gpu-tune#voltage).

## Version 0.10.0

*November 6, 2026*

- **Drag-to-edit fan curves.** A graph with draggable points, a choice of temperature source and of which fans to drive. See [Tune](gpu-tune#the-fan-curve-editor).
- **Profiles that switch by themselves** by power source, running program, load or temperature. See [Profiles](gpu-profiles).
- **Sessions.** Record how a profile behaves and compare two side by side, with energy and efficiency. See [Sessions](gpu-sessions).
- **Voltage and frequency curves** on AMD cards that have them.
- **`voltkit gpu check`** compares Voltkit's readings with `nvidia-smi` and measures what monitoring costs. It also fixed video memory reading too high on NVIDIA. See [GPU on the command line](gpu-terminal#check-voltkit-s-readings).

## Version 0.9.2

*November 6, 2026*

- **New tool: GPU Tuner.** Monitor NVIDIA, AMD and Intel graphics cards and change power limits, fans and fan curves, temperature targets, clocks and voltage, with every change previewed and undone unless you confirm it. See [GPU Tuner](gpu).
- Fixed unreadable number fields in the light theme.

## Version 0.9.1

*November 5, 2026*

- **Translated guidance for busy items.** [What's Using This?](using) explains in your language how to inspect busy files, drives and network ports.
- Refreshed documentation and project funding links.

## Version 0.9.0

*November 4, 2026*

- **French and Simplified Chinese.** The app, the command line, `--help` and the man pages are translated. See [A tour of the app](tour#language).

## Version 0.8.0

*November 3, 2026*

- **Spanish and German.** The app, the command line, `--help` and the man pages are translated.
- **Follows the system language,** falling back to English.

## Version 0.7.0

*November 2, 2026*

- **A redesign of every tool** with adaptive card layouts and consistent light and dark styling. See [A tour of the app](tour).
- **Searchable lists** of snippets ([Text Expansion](snippets)) and of apps and permissions ([App Permissions](permissions)), and a shortcut details workspace in [Shortcut Detective](shortcuts).
- **A branded About dialog,** a larger default window and compact tool navigation.

## Version 0.6.0

*November 2, 2026*

- **Typed triggers turn on with one switch:** one password, no logout or restart. See [Text Expansion](snippets#typed-triggers-optional).
- **PNG compression is built in,** so `oxipng` is no longer needed. See [Asset Compressor](compress).
- **Quit** in the main menu (<kbd>Ctrl</kbd>+<kbd>Q</kbd>) and more room beside header buttons.

## Version 0.5.0

*November 2, 2026*

- **New tool: System Report.** Your hardware explained, with checks for drivers, drive health, temperatures, firmware and memory, plus what changed since earlier days. See [System Report](sysreport).
- **New tool: Shortcut Detective.** See who owns a shortcut, find clashes and change them. See [Shortcut Detective](shortcuts).
- The menu lists the tools alphabetically.

## Version 0.4.3

*November 1, 2026*

- The download page links the Fedora COPR repository. See [Install](install#fedora).

## Version 0.4.2

*November 1, 2026*

- **Hotkey results show up in the open app:** text, QR codes and colors captured by a shortcut appear on the tool's page when the window is open.
- The [Color Picker](color) page names its hotkey.

## Version 0.4.1

*November 1, 2026*

- **The first Fedora package.** It sets itself up on first launch (background service and hotkeys). See [Install](install#fedora).

## Version 0.4.0

*November 1, 2026*

- **New tool: Smart Paste.** Paste the clipboard converted: JSON, tables, clean links, letter case. See [Smart Paste](smartpaste).
- **New tool: Why Didn't It Sleep?** See what keeps the computer awake and what woke it. See [Why Didn't It Sleep?](sleep).

## Version 0.3.0

*November 1, 2026*

- **New tool: What's Using This?** See which apps keep a file, folder, drive or port busy, and close them. See [What's Using This?](using).
- **New tool: App Permissions.** Review and forget the answers you gave apps. See [App Permissions](permissions).

## Version 0.2.2

*October 29, 2026*

- **A logo in QR codes you create,** with a read-back check that the code still scans. See [QR Code](qr#add-your-own-logo-optional).

## Version 0.2.1

*October 29, 2026*

- Documentation fixes.

## Version 0.2.0

*October 29, 2026*

- **New tool: QR Code.** Scan a code on screen, or create one. See [QR Code](qr).
- **New tool: Screen Ruler.** Measure anything on screen in pixels. See [Screen Ruler](ruler).
- **New tool: Keep Awake.** See [Keep Awake](awake).
- **New tool: Hosts & Environment.** Edit `/etc/hosts` and your session's environment variables. See [Hosts & Environment](system).
- Portal dialogs attach to the app window, and tools that open their own window now come to the front.

## Version 0.1.3

*October 29, 2026*

- **Manage snippets in the app** with an editor, replacing the earlier playground. See [Text Expansion](snippets#manage-your-snippets).

## Version 0.1.2

*October 28, 2026*

- Fixed portal requests that hung after the first one in the app.
- A clearer color picker button and an About window.

## Version 0.1.1

*October 27, 2026*

- Added screenshots and a feature tour to the project page.

## Version 0.1.0

*October 27, 2026*

The first release.

- **Text Extractor** (OCR) and **Color Picker** with global shortcuts through a background service. See [Text Extractor](ocr) and [Color Picker](color).
- **Text Expansion** with a snippet picker and optional typed triggers. See [Text Expansion](snippets).
- **Asset Compressor.** See [Asset Compressor](compress).
- Man pages for the command-line tools, `voltkit doctor` with an in-app banner for anything missing, and a signed apt repository. See [Troubleshooting](troubleshooting).
