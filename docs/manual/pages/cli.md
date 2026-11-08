# Command-line reference

Every feature also works from a terminal, a script or a keyboard shortcut. Run `voltkit` with no command to open the app.

```bash
voltkit --help               # every command
voltkit COMMAND --help       # one command in detail
voltkit --version
man voltkit                  # the manual page, and man voltkit-COMMAND for each command
```

## Commands

| Command | What it does | Tool page |
|---|---|---|
| `voltkit ocr [--lang LANG]` | Select a screen region and copy its text | [Text Extractor](ocr) |
| `voltkit pick-color` | Pick a color from the screen and copy its HEX value | [Color Picker](color) |
| `voltkit color VALUE` | Convert a color and report WCAG contrast | [Color Picker](color) |
| `voltkit compress FILE… [--quality N]` | Strip metadata and compress PNG, JPEG, WebP or MP4 | [Asset Compressor](compress) |
| `voltkit expand TEMPLATE [--arg NAME=VALUE]…` | Expand a snippet template | [Text Expansion](snippets) |
| `voltkit picker [--snippet TRIGGER]` | Open the snippet picker | [Text Expansion](snippets) |
| `voltkit expander …` | Set up and toggle typed triggers | [Text Expansion](snippets) |
| `voltkit ruler` | Measure distances on screen in pixels | [Screen Ruler](ruler) |
| `voltkit awake [DURATION]` | Keep the computer awake | [Keep Awake](awake) |
| `voltkit using TARGET` | Which programs keep a file, folder, drive or port busy | [What's Using This?](using) |
| `voltkit smart-paste` | Paste the clipboard converted | [Smart Paste](smartpaste) |
| `voltkit convert [CONVERSION]` | Smart Paste's conversions on standard input | [Smart Paste](smartpaste) |
| `voltkit report [--memory]` | Describe this computer: hardware, drivers, health, changes | [System Report](sysreport) |
| `voltkit sleep` | Why the computer did not sleep, or why it woke | [Why Didn't It Sleep?](sleep) |
| `voltkit gpu …` | Monitor and tune graphics cards | [GPU Tuner](gpu-terminal) |
| `voltkit qr scan\|read\|make` | Read or make QR codes | [QR Code](qr) |
| `voltkit shortcuts …` | List, inspect and manage keyboard shortcuts | [Shortcut Detective](shortcuts) |
| `voltkit daemon …` | The background service | [Global shortcuts](daemon) |
| `voltkit trigger ACTION` | Run a hotkey action | [Global shortcuts](daemon) |
| `voltkit doctor` | Check that everything Voltkit needs is installed | [Troubleshooting](troubleshooting) |

## Examples

```bash
voltkit ocr --lang eng+deu              # text on screen → clipboard
voltkit pick-color                      # eyedropper → HEX on the clipboard
voltkit color '#3584e4'                 # HEX / RGB / HSL + contrast vs white and black
voltkit compress *.png --quality 75
voltkit expand 'Hi {arg:who}, {date:%A}' --arg who=Sam
voltkit qr make https://example.com     # QR code in the terminal (or --output code.png)
voltkit qr read photo.png               # QR codes in an image file
voltkit awake 1h30m                     # keep awake (on / off / no argument for status)
voltkit using /media/$USER/USB          # what keeps this drive busy? (or a port: voltkit using 3000)
voltkit sleep
voltkit report --memory
voltkit shortcuts who super+v
wl-paste | voltkit convert json         # Smart Paste conversions in scripts
voltkit gpu                             # graphics cards, readings, supported controls
```

## Binding a command to a key

If your desktop has no GlobalShortcuts portal, bind `voltkit trigger ocr`, `trigger color`, `trigger snippets`, `trigger qr`, `trigger ruler` or `trigger smart-paste` to keys in your desktop's keyboard settings or compositor config. `trigger` sends the action to the running background service, or runs it directly if the service is not running. Results and errors appear as desktop notifications.

## Man pages

The manual pages are generated from the command definitions, so they always match `--help`. Translations are provided in Spanish, German, French and Simplified Chinese.

```bash
man voltkit
man voltkit-gpu
man -L es voltkit-compress    # Spanish, if installed
```
