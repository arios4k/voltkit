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
| Global hotkeys | `org.freedesktop.portal.GlobalShortcuts` (GNOME 48+, KDE 5.27+, Hyprland). Fallbacks: GNOME custom keybindings via `gsettings` (`powerlinux shortcuts install`), or compositor bindings to `powerlinux trigger`. |
| Region screenshot | `org.freedesktop.portal.Screenshot` (interactive), falling back to grim/slurp, gnome-screenshot, spectacle or maim |
| Eyedropper | `org.freedesktop.portal.Screenshot.PickColor` |
| Notifications | `org.freedesktop.Notifications` directly. The notification portal needs an app ID that unsandboxed apps on older portals can't provide. |
| Text expansion | Wayland has no global key snooping by design. Options: (a) an IBus/Fcitx input-method engine, which is the cleanest route and works in every toolkit; (b) evdev read + uinput write, which needs the user in the `input` group. Plan: ship (a), and offer (b) as an opt-in. |

## Daemon

`powerlinux daemon` owns `io.github.powerlinux.Daemon` and requests it with `DoNotQueue`, so a
second instance exits instead of waiting. Object `/io/github/powerlinux/Daemon`, interface
`io.github.powerlinux.Daemon1`:

| Method | Signature | Purpose |
|---|---|---|
| `Trigger` | `s → ()` | start `"ocr"` or `"color"`; returns immediately; ignored while another capture is open |
| `Status` | `() → (ss)` | version and shortcut backend: `portal`, `gnome-keybindings` or `none` |
| `Quit` | `() → ()` | stop the daemon |

```bash
busctl --user call io.github.powerlinux.Daemon /io/github/powerlinux/Daemon \
  io.github.powerlinux.Daemon1 Trigger s ocr
```

Picked colors go to `~/.local/share/powerlinux/colors.txt`, which the GUI's history shows.

## Roadmap

- [x] `powerlinux daemon` with portal global shortcuts, GNOME fallback, autostart and a D-Bus API
- [x] Portal-based capture and PickColor via [`ashpd`](https://crates.io/crates/ashpd)
- [x] Color picker: persistent history
- [ ] Color picker: palette generation (complementary, triadic, shades), export to CSS/GIMP palette
- [ ] Daemon owns the snippet engine (see Text expansion above)
- [ ] KDE fallback via KGlobalAccel for Plasma versions without the portal
- [ ] OCR: language picker, QR/barcode detection, optional in-process `leptess` backend
- [ ] Compressor: Nautilus/Dolphin/Nemo "Compress with PowerLinux" context-menu entries, AVIF,
      resize presets
- [ ] Snippet editor in the GUI (currently edited in TOML), plus a picker popup with fuzzy search
- [ ] Flatpak manifest and Flathub submission
- [ ] App icon, screenshots, AppStream metainfo
