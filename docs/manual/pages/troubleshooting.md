# Troubleshooting and `voltkit doctor`

## `voltkit doctor`

Run it any time something does not work:

```bash
voltkit doctor
```

It checks the helper programs, OCR language data, desktop portals, clipboard tools, the background service and hotkeys, and prints the install command for anything missing on your distribution. A healthy system looks like this:

```
Text Extractor (OCR)
  ✓ tesseract
  ✓ language data: eng
  ✓ region capture: Screenshot portal
Color Picker
  ✓ screen eyedropper: Screenshot portal
Clipboard (hotkeys and command line)
  ✓ xclip
Asset Compressor
  ✓ jpegoptim (JPEG)
  ✓ cwebp (WebP)
  ✓ ffmpeg (MP4)
Text Expansion
  ✓ picker pastes for you: RemoteDesktop portal
  ✓ typed triggers: keyboard access
Keep Awake
  ✓ stop sleeping and locking: Inhibit portal
Hosts & Environment
  ✓ saving /etc/hosts: pkexec
Background Service
  ✓ daemon running
  ✓ hotkeys: GNOME custom shortcuts

Everything is in place.
```

The same findings appear in the app as the **N features need attention** banner, with **Copy Command** for the fix.

## Common problems

| Problem | What to do |
|---|---|
| A keyboard shortcut does nothing | `voltkit daemon status`. Start the service with the **Start** banner or `voltkit daemon`. Check the key is not used by something else in [Shortcut Detective](shortcuts). See [Global shortcuts](daemon). |
| Text Extractor copies nothing or garbles text | `voltkit doctor`: Tesseract or the language pack may be missing. Set the language in the [configuration file](config). |
| The color picker or region capture does not appear | The desktop's Screenshot portal is missing. Install `xdg-desktop-portal` and the portal for your desktop (`xdg-desktop-portal-gnome`, `-kde`, `-wlr`…). |
| Pasting from the picker or Smart Paste does not happen | Allow the RemoteDesktop permission when the desktop asks. Otherwise press <kbd>Ctrl</kbd>+<kbd>V</kbd> after it copies the result. Manage it in [App Permissions](permissions). |
| Compressing a format fails | Install the helper: `jpegoptim` (JPEG), `webp` (WebP), `ffmpeg` (MP4). |
| Saving the hosts file or changing GPU settings asks for a password and fails | Install polkit (`pkexec`). |
| Typed triggers do nothing | `voltkit expander status` says why. |
| The clipboard is not set from a shortcut | Install `wl-clipboard` (Wayland) or `xclip` (X11). The result still appears in a notification. |
| A GPU control is greyed out | Open the GPU Tuner's Monitor tab: *What this card supports* gives the reason. |
| Voltkit does not follow my language | Start it with `LANGUAGE=es voltkit` to test; set your session language otherwise. |

## Reporting a problem

[Open an issue](https://github.com/arios4k/voltkit/issues) with:

- the output of `voltkit --version` and `voltkit doctor`,
- your distribution and desktop (GNOME, KDE…) and whether it runs Wayland or X11,
- what you did and what happened.

For GPU Tuner, use the [hardware report form](https://github.com/arios4k/voltkit/issues/new?template=gpu-hardware-report.yml) and include the output of `voltkit gpu`. System Report's **Copy Report** produces a summary that is safe to paste: no serial numbers, user or computer names, or network addresses.
