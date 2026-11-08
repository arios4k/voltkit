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
- Your settings stay in `~/.config/voltkit/` and your data in `~/.local/share/voltkit/` until you delete them. See [Configuration file](config).
