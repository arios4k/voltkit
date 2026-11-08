# Configuration file

Voltkit's settings are in `~/.config/voltkit/config.toml`. **Every key is optional**: Voltkit works without the file, and missing keys use the defaults. Snippets you add in the app are saved here, and after editing by hand you can run `voltkit daemon reload` for the service to re-read it.

## A complete example

```toml
# ~/.config/voltkit/config.toml

[hotkeys]
ocr = "<Super><Shift>t"
color_picker = "<Super><Shift>c"
snippets = "<Super><Shift>e"      # opens the snippet picker
qr = "<Super><Shift>q"            # scans a QR code on screen
ruler = "<Super><Shift>r"         # opens the screen ruler
smart_paste = "<Super><Shift>v"   # pastes the clipboard converted

[ocr]
lang = "eng"            # e.g. "eng+deu"; needs matching tesseract language packs

[compress]
quality = 80            # 1–100

[expander]
typed_triggers = false        # expand ;date etc. as you type; run `voltkit expander setup` first
paste_shortcut = "ctrl+v"     # or "ctrl+shift+v" / "shift+insert"

[gpu]
price_per_kwh = 0.15          # your electricity price, to show what a GPU session cost
currency = "€"

[[snippets]]
trigger = ";date"
template = "{date:%Y-%m-%d}"

[[snippets]]
trigger = ";sig"
template = "Best regards,\n{arg:name}"
```

## Sections

| Section | Key | Meaning | Default |
|---|---|---|---|
| `[hotkeys]` | `ocr`, `color_picker`, `snippets`, `qr`, `ruler`, `smart_paste` | The global shortcuts, in the form `<Super><Shift>t` | see [Global shortcuts](daemon) |
| `[ocr]` | `lang` | Tesseract language code(s); join several with `+` | `eng` |
| `[compress]` | `quality` | Default quality for JPEG, WebP and video, 1 to 100 | `80` |
| `[expander]` | `typed_triggers` | Whether typed triggers are on (use the page switch or `voltkit expander setup` to set it up) | `false` |
| `[expander]` | `paste_shortcut` | What the picker presses to paste: `ctrl+v`, `ctrl+shift+v` (terminals) or `shift+insert` | `ctrl+v` |
| `[gpu]` | `price_per_kwh`, `currency` | Electricity price and currency sign for session costs | not set |
| `[[snippets]]` | `trigger`, `template` | One snippet each; see [Text Expansion](snippets) | three examples |

## Other files

| Path | What |
|---|---|
| `~/.config/voltkit/gpu-profiles.json` | GPU profiles you edit in the app |
| `~/.config/environment.d/90-voltkit.conf` | Environment variables set in [Hosts & Environment](system) |
| `~/.local/share/voltkit/` | Color history, daily hardware snapshots, and `gpu-sessions/` |
| `/etc/voltkit/gpu.json`, `/etc/voltkit/gpu-profiles.json` | What the GPU service runs |
| `/etc/hosts.voltkit-backup` | The backup of the hosts file |
| `/etc/udev/rules.d/70-voltkit-typed-triggers.rules` | Keyboard access for typed triggers (only if you turned them on) |
| `/etc/udev/rules.d/90-voltkit-wakeup.rules` | USB wake-up choices from [Why Didn't It Sleep?](sleep) |

## Environment variables

| Variable | Effect |
|---|---|
| `LANGUAGE`, `LANG` | The language Voltkit uses |
| `VOLTKIT_GPU_DEMO`, `VOLTKIT_REPORT_DEMO`, `VOLTKIT_SLEEP_DEMO`… | Show made-up data (used to make this manual's screenshots) |
