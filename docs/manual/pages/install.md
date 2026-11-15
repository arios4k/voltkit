# Install

Voltkit runs on 64-bit Intel/AMD Linux. It is developed and tested on GNOME (Zorin OS 18) and uses standard XDG desktop portals, so KDE Plasma and other desktops should work too.

## Ubuntu, Debian and derivatives

For Ubuntu 24.04 and newer, Debian 13 and newer, Linux Mint 22, Zorin OS 18, Pop!_OS 24.04 and similar:

```bash
sudo curl -fsSLo /usr/share/keyrings/voltkit-archive-keyring.gpg \
  https://arios4k.github.io/voltkit/voltkit-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/voltkit-archive-keyring.gpg] https://arios4k.github.io/voltkit stable main" \
  | sudo tee /etc/apt/sources.list.d/voltkit.list
sudo apt update && sudo apt install voltkit
```

The repository is signed with a GPG key, and apt installs the tools Voltkit needs (Tesseract, wl-clipboard, the desktop portal) together with it. Updates arrive with `sudo apt upgrade`.

## Fedora

For Fedora 43 and newer, from the [COPR repository](https://copr.fedorainfracloud.org/coprs/arios/voltkit/):

```bash
sudo dnf copr enable arios/voltkit
sudo dnf install voltkit
```

Updates arrive with `sudo dnf upgrade`.

## Snap Store

Voltkit is also in the Snap Store, so you can find it in Ubuntu's App Center:

```bash
sudo snap install voltkit
```

Snap updates arrive automatically.

> [!IMPORTANT] Apps from the Snap Store run in a sandbox that can't ask for your administrator password or change system files. Some features need one of them, so they work only when Voltkit is installed with apt (Ubuntu, Debian) or DNF (Fedora). Each tool affected shows a banner at the top of its page that says what is missing and why.

### After installing the snap

1. **Open Voltkit once** from your apps. It starts its background service, sets it to start at every login, and adds its keyboard shortcuts.
2. **Allow the permissions the snap doesn't get automatically.** Run these once in a terminal, or switch them on in your app store under **Voltkit → Permissions**:

   ```bash
   sudo snap connect voltkit:removable-media
   sudo snap connect voltkit:system-observe
   sudo snap connect voltkit:mount-observe
   sudo snap connect voltkit:process-control
   sudo snap connect voltkit:log-observe
   sudo snap connect voltkit:login-session-observe
   sudo snap connect voltkit:hardware-observe
   ```

   | Permission | Lets Voltkit |
   |---|---|
   | `removable-media` | use files on USB drives and other external disks (Compressor, Batch Rename, Image Resizer) |
   | `system-observe` | see which apps have files and ports open, and read drivers and the system's state (What's Using This?, System Report) |
   | `mount-observe` | list the drives that are connected (What's Using This?, System Report) |
   | `process-control` | close the apps that keep something busy (What's Using This?) |
   | `log-observe` | read the sleep history (Why Didn't It Sleep?) |
   | `login-session-observe` | see what is keeping the computer awake (Why Didn't It Sleep?) |
   | `hardware-observe` | read hardware details, temperatures and fans (System Report, GPU Tuner) |

   You can connect only the ones for the tools you use. Nothing else needs connecting: screenshots, the eyedropper, the clipboard and pasting go through the desktop's portals, which the snap may use without asking.
3. **Check** with `voltkit doctor`: its *Snap Store version* section lists each permission and whether it is connected.

### What works only with apt or Fedora

| Tool | What the snap can't do |
|---|---|
| [GPU Tuner](gpu) | Changing settings (power limit, fans, clocks, voltage) and applying profiles at startup. Monitoring works. |
| [Hosts & Environment](system) | The whole tool: editing `/etc/hosts` and your session's environment variables. |
| [Text Expansion](snippets) | Typed triggers such as `;date`. The snippet picker (<kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd>) works. |
| [System Report](sysreport) | Drive health, memory slots and firmware update checks. |
| [What's Using This?](using) | Seeing and closing apps that belong to other users or to the system. |
| [Why Didn't It Sleep?](sleep) | Stopping a USB device from waking the computer. |

These need the administrator password (Voltkit asks through `pkexec`) or write system files, and a snap is not allowed either. To use them, remove the snap (`sudo snap remove voltkit`) and install with [apt](#ubuntu-debian-and-derivatives) or [Fedora](#fedora) instead.

## From source

You need Rust 1.85 or newer, GTK 4.14 or newer and libadwaita 1.5 or newer.

```bash
# Debian/Ubuntu
sudo apt install libgtk-4-dev libadwaita-1-dev
# Fedora
sudo dnf install gtk4-devel libadwaita-devel
# Arch
sudo pacman -S gtk4 libadwaita

git clone https://github.com/arios4k/voltkit
cd voltkit
cargo install --path .
voltkit
```

> [!NOTE] Install the binary before you set up shortcuts or autostart: both record the binary's path.

To add Voltkit to your app grid with its icon when installing from source, copy the icons and desktop file as described in the [README](https://github.com/arios4k/voltkit#install-from-source).

## First run

Open **Voltkit** from your apps. That is all: it starts its background service (which also starts at every login from now on) and, on GNOME, adds its keyboard shortcuts. If something a feature needs is missing, a banner at the top of the window says so and offers the command to fix it. See [Troubleshooting](troubleshooting).

Typing triggers such as `;date` in any app is optional: turn on **Typed triggers** on the [Text Expansion](snippets) page.

## Helper programs

Voltkit calls standard tools when they are installed, so it adds no heavy dependencies of its own.

| Feature | Needs |
|---|---|
| Text Extractor (OCR) | `tesseract` plus a language pack such as `tesseract-ocr-eng` |
| Region capture and the eyedropper | the desktop's Screenshot portal; region capture falls back to `grim`+`slurp`, `gnome-screenshot`, `spectacle` or `maim` |
| Clipboard in the background service and the command line | `wl-clipboard` (Wayland) or `xclip` (X11). Without it the result is still shown in a notification |
| Asset Compressor | built in for PNG; `jpegoptim` for JPEG, `cwebp` for WebP, `ffmpeg` for MP4 |
| Pasting into other apps | the RemoteDesktop portal (the desktop asks once) |

Run `voltkit doctor` at any time to see what is missing and the exact install command for your distribution.

## Updating and removing

- **apt:** `sudo apt upgrade` updates, `sudo apt remove voltkit` removes.
- **Fedora:** `sudo dnf upgrade`, `sudo dnf remove voltkit`.
- **Snap:** updates arrive automatically; `sudo snap remove voltkit` removes it, together with the snap's own copy of your settings.
- Your settings stay in `~/.config/voltkit/` and your data in `~/.local/share/voltkit/` until you delete them. See [Configuration file](config).
