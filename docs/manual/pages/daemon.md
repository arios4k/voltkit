# Global shortcuts and the background service

Keyboard shortcuts that work from any app, Keep Awake that survives closing the window, and typed triggers all need Voltkit's **background service** (the daemon). It starts by itself the first time you open the app and at every login after that.

## What the service does

- Listens for the global keyboard shortcuts and runs the action.
- Holds [Keep Awake](awake) so it lasts after the window closes.
- Runs [typed triggers](snippets) when they are on.
- Keeps a hardware snapshot for [System Report](sysreport)'s *What Changed*.

It owns the name `io.github.arios4k.Voltkit.Daemon` on the session bus, so only one runs at a time.

## Control it

```bash
voltkit daemon                    # run in the foreground
voltkit daemon status             # running? which shortcut backend?
voltkit daemon stop
voltkit daemon reload             # re-read snippets after editing the config file
voltkit daemon enable-autostart   # start at every login
voltkit daemon disable-autostart
```

`voltkit daemon status` prints something like `running (v0.10.4), shortcuts: gnome-keybindings, typed triggers: on`. In the app, the **Start** banner appears whenever the service is off.

## How the shortcuts are bound

| Desktop | How |
|---|---|
| GNOME 48+, KDE Plasma 5.27+, Hyprland (xdg-desktop-portal 1.19 or newer) | The **GlobalShortcuts portal**: the desktop asks you to approve them once |
| Older GNOME (for example 46) and GNOME-based distributions | Custom shortcuts in *Settings → Keyboard → Custom Shortcuts*, added automatically (or with `voltkit shortcuts install`) |
| Sway, river and others | Bind `voltkit trigger ocr`, `trigger color` and the others in your compositor config |

```bash
voltkit shortcuts install --dry-run   # preview what would be added
voltkit shortcuts install             # add them to GNOME Settings
voltkit shortcuts remove              # remove only the ones Voltkit added
```

Other custom shortcuts are left untouched.

## The default keys

| Action | Default | Config key |
|---|---|---|
| Text Extractor | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> | `ocr` |
| Color Picker | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | `color_picker` |
| Snippet picker | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> | `snippets` |
| QR scan | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd> | `qr` |
| Screen Ruler | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd> | `ruler` |
| Smart Paste | <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd> | `smart_paste` |

Change them in `~/.config/voltkit/config.toml` (see [Configuration file](config)) and run `voltkit shortcuts install` again, or use [Shortcut Detective](shortcuts) to find a free combination and change it.

## Troubleshooting

- **A shortcut does nothing.** Run `voltkit daemon status`. If it is not running, `voltkit daemon` or the **Start** banner starts it. Then check the shortcut is not taken by something else with Shortcut Detective.
- **The desktop asked about a shortcut and I said no.** Remove and re-add it in your desktop's keyboard settings, or run `voltkit shortcuts install`.
- **I do not want a background service.** Run `voltkit daemon disable-autostart` and `voltkit daemon stop`. The app still works, but shortcuts, Keep Awake after closing and typed triggers will not.
