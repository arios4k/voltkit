# FAQ, privacy and safety

## Privacy

**Does Voltkit send anything over the network?** No. Everything runs locally, with no network access of its own and no telemetry. The only exceptions are things you ask for: **Check Online** in System Report downloads the latest firmware list, and links you click (such as this manual or a QR code link) open in your browser.

**What does it store?** Your settings and snippets, the last 32 colors you used, daily hardware snapshots (for *What Changed*), and the GPU sessions you record. All stay on your computer, in the folders listed in [Configuration file](config).

**Are typed triggers a keylogger?** No. They are off by default. Voltkit keeps only the last 64 typed characters **in memory** to see whether they end with a trigger, and never logs, saves or sends them. They do need keyboard access, which applies to every program you run until you remove it. See [Text Expansion](snippets).

**Does System Report share my serial numbers?** No. **Copy Report** leaves out serial numbers, user and computer names, and network addresses.

## Safety

**Does it change my original files?** No. [Asset Compressor](compress) writes `name.min.ext` beside each file and never modifies or deletes the original.

**Can it break `/etc/hosts`?** Every save writes a new file next to the old one, backs up the old one to `/etc/hosts.voltkit-backup`, and then swaps it in. System entries are protected unless you allow editing them.

**Can GPU Tuner damage my card?** Every change is validated against the card's limits, previewed and undone unless you confirm it, and a startup guard prevents boot loops. But changing voltage and clocks always carries risk, and the write paths are not yet tested on real AMD hardware. See [GPU Tuner safety](gpu-safety).

**Can it close important programs?** [What's Using This?](using) never offers the desktop, the display server, your login session and other critical parts of the system for closing, and asks politely before forcing anything.

## General

**Which desktops are supported?** Voltkit is developed on GNOME (Zorin OS 18). It uses standard XDG portals, so KDE Plasma, Hyprland and others should work, but are less tested. Shortcut Detective supports GNOME, Cinnamon, Budgie and MATE. On desktops without the GlobalShortcuts portal, bind `voltkit trigger …` to keys yourself.

**Does it work on X11?** The tools use portals and work on Wayland and X11 sessions. The clipboard helper is `wl-clipboard` on Wayland and `xclip` on X11.

**Do I need to keep the window open?** No. Shortcuts, Keep Awake and typed triggers are handled by the background service. Closing the window leaves it running.

**How do I change the language?** Voltkit follows your session language (English, Spanish, German, French and Simplified Chinese). To test another: `LANGUAGE=de voltkit`. This manual is in English.

**How do I uninstall it?** `sudo apt remove voltkit` or `sudo dnf remove voltkit`. Your settings stay in `~/.config/voltkit/` until you delete them. If you turned on typed triggers, run `voltkit expander remove-access` first.

**Is it free?** Yes. Voltkit is open source under the MIT license: [github.com/arios4k/voltkit](https://github.com/arios4k/voltkit). If you find it useful, you can [support the author on PayPal](https://paypal.me/AlexisRios).
