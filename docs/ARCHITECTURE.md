# Architecture

## Principles

1. **One binary.** The GUI, the CLI and (later) the background daemon are the same executable,
   chosen by subcommand.
2. **No desktop lock-in.** No GNOME Shell or KDE extensions. Desktop integration goes through
   XDG desktop portals where they exist, and falls back to widely packaged tools.
3. **Logic is UI-free.** Everything in `src/modules/` is plain Rust with unit tests. `app.rs` is
   only wiring, so the same code drives the GUI, the CLI and the daemon.
4. **Non-destructive by default.** The compressor writes `name.min.ext` and never touches the
   original.

## Wayland realities

| Need | Approach |
|---|---|
| Global hotkeys | `org.freedesktop.portal.GlobalShortcuts` (GNOME 48+, KDE 5.27+, Hyprland). Fallbacks: GNOME custom keybindings via `gsettings` (`voltkit shortcuts install`), or compositor bindings to `voltkit trigger`. |
| Region screenshot | `org.freedesktop.portal.Screenshot` (interactive), falling back to grim/slurp, gnome-screenshot, spectacle or maim |
| Eyedropper | `org.freedesktop.portal.Screenshot.PickColor` |
| Notifications | `org.freedesktop.Notifications` directly. The notification portal needs an app ID that unsandboxed apps on older portals can't provide. |
| Text expansion | Wayland has no global key snooping by design, so there are two modes. The **picker** needs no setup: it pastes through the RemoteDesktop portal (permission asked once, kept with a restore token). **Typed triggers** are opt-in: evdev reads keyboards and a uinput virtual keyboard sends Backspace and the paste key, which needs the `input` group and a udev rule (`voltkit expander setup`). An IBus engine was ruled out: components install only system-wide, and users would have to switch input sources. |
| Inserting text | Always through the clipboard: copy, press the paste shortcut, restore the previous clipboard. That handles any Unicode text regardless of keyboard layout. |

## Daemon

`voltkit daemon` owns `io.github.arios4k.Voltkit.Daemon` and requests it with `DoNotQueue`, so a
second instance exits instead of waiting. Object `/io/github/arios4k/Voltkit/Daemon`, interface
`io.github.arios4k.Voltkit.Daemon1`:

| Method | Signature | Purpose |
|---|---|---|
| `Trigger` | `s → ()` | start `"ocr"`, `"color"` or `"snippets"`; returns immediately; ignored while another one is open |
| `Status` | `() → (sss)` | version; shortcut backend: `portal`, `gnome-keybindings` or `none`; typed triggers: `on`, `off` or `unavailable` |
| `Quit` | `() → ()` | stop the daemon |

```bash
busctl --user call io.github.arios4k.Voltkit.Daemon /io/github/arios4k/Voltkit/Daemon \
  io.github.arios4k.Voltkit.Daemon1 Trigger s ocr
```

Picked colors go to `~/.local/share/voltkit/colors.txt`, which the GUI's history shows.

## Roadmap

- [x] `voltkit daemon` with portal global shortcuts, GNOME fallback, autostart and a D-Bus API
- [x] Portal-based capture and PickColor via [`ashpd`](https://crates.io/crates/ashpd)
- [x] Color picker: persistent history
- [ ] Color picker: palette generation (complementary, triadic, shades), export to CSS/GIMP palette
- [x] Snippet picker and typed triggers
- [ ] Typed triggers: decode keys with the user's xkb layout instead of US
- [ ] KDE fallback via KGlobalAccel for Plasma versions without the portal
- [ ] OCR: language picker, QR/barcode detection, optional in-process `leptess` backend
- [ ] Compressor: Nautilus/Dolphin/Nemo "Compress with Voltkit" context-menu entries, AVIF,
      resize presets
- [ ] Snippet editor in the GUI (currently edited in TOML)
- [ ] Flatpak manifest and Flathub submission
- [x] App icon (full-color and symbolic, `data/icons/`)
- [ ] Screenshots, AppStream metainfo
