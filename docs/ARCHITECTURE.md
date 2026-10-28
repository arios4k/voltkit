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
| Global hotkeys | `org.freedesktop.portal.GlobalShortcuts` (GNOME 48+, KDE 5.27+, Hyprland). Until that lands here, users bind `powerlinux ocr` etc. in their DE's shortcut settings. |
| Region screenshot | `org.freedesktop.portal.Screenshot` (interactive), currently via grim/slurp, gnome-screenshot, spectacle or maim |
| Eyedropper | `org.freedesktop.portal.Screenshot.PickColor` |
| Text expansion | Wayland has no global key snooping by design. Options: (a) an IBus/Fcitx input-method engine, which is the cleanest route and works in every toolkit; (b) evdev read + uinput write, which needs the user in the `input` group. Plan: ship (a), and offer (b) as an opt-in. |

## Roadmap

- [ ] `powerlinux daemon`: tray-less background service. It registers portal global shortcuts
      and owns the snippet engine. The GUI talks to it over D-Bus (`io.github.powerlinux.PowerLinux`).
- [ ] Portal-based capture and PickColor via the [`ashpd`](https://crates.io/crates/ashpd) crate
- [ ] Color picker: persistent history, palette generation (complementary, triadic, shades),
      export to CSS/GIMP palette
- [ ] OCR: language picker, QR/barcode detection, optional in-process `leptess` backend
- [ ] Compressor: Nautilus/Dolphin/Nemo "Compress with PowerLinux" context-menu entries, AVIF,
      resize presets
- [ ] Snippet editor in the GUI (currently edited in TOML), plus a picker popup with fuzzy search
- [ ] Flatpak manifest and Flathub submission
- [ ] App icon, screenshots, AppStream metainfo
