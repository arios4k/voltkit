# Snap Store version

Voltkit is available in three ways: from its own **apt** repository (Ubuntu, Debian and derivatives), from **Fedora** COPR, and from the **Snap Store**. This page explains what is different about the snap: why it exists, what works in it, what to do after installing it, what it can't do and why, and how to switch between versions.

## Why there is a snap version

Mostly **so people can find Voltkit**. The Snap Store is available on every Linux distribution that supports snaps, and it is built into app stores such as Ubuntu's App Center, where people browse and install apps without a terminal. Publishing there makes the tool easier to discover and to install in one step.

**Most of Voltkit works the same in the snap.** 16 of its 18 tools work there, most of them fully, and only a handful of features are left out. Those are limits of the snap sandbox, explained below, not of Voltkit. If you need them, install with apt or Fedora instead: that version has every feature.

## Which version should I install?

| | apt (Ubuntu, Debian…) | Fedora (COPR) | Snap Store |
|---|---|---|---|
| Where | Ubuntu 24.04+, Debian 13+, Mint 22, Zorin OS 18, Pop!_OS 24.04 and derivatives | Fedora 43+ | Any distribution with snapd, and Ubuntu's App Center |
| Features | All | All | All except those listed in [What works only with apt or Fedora](#what-works-only-with-apt-or-fedora) |
| Updates | `sudo apt upgrade` | `sudo dnf upgrade` | Automatic |
| Setup after installing | None | None | Connect a few permissions once ([below](#connect-the-permissions)) |
| Runs in a sandbox | No | No | Yes (strict confinement) |

**Install only one of them.** See [Installing only one version](#installing-only-one-version).

## What works in the snap

| Tool | In the snap |
|---|---|
| [Text Extractor](ocr) | Works fully (English text recognition is included) |
| [Color Picker](color) | Works fully |
| [QR Code](qr) | Works fully |
| [Screen Ruler](ruler) | Works fully |
| [Zoom & Annotate](zoom) | Works fully |
| [Smart Paste](smartpaste) | Works fully |
| [Keep Awake](awake) | Works fully |
| [Shortcut Detective](shortcuts) | Works fully |
| [Asset Compressor](compress) | Works fully for files in your home folder; USB drives after connecting `removable-media` |
| [Batch Rename](rename) | Same as the compressor |
| [Image Resizer](resize) | Same as the compressor |
| [Text Expansion](snippets) | The snippet picker works; **typed triggers don't** |
| [What's Using This?](using) | Your own apps, after connecting `system-observe`, `mount-observe` and `process-control`; **not other users' or the system's apps** |
| [Why Didn't It Sleep?](sleep) | After connecting `log-observe` and `login-session-observe`; **can't stop a USB device from waking the computer** |
| [System Report](sysreport) | After connecting `hardware-observe`, `system-observe` and `mount-observe`; **no drive health, memory slots or firmware checks** |
| [GPU Tuner](gpu) | **Monitoring only**, after connecting `hardware-observe`; can't change settings |
| [Hosts & Environment](system) | **Not available** (shown read-only) |
| [App Permissions](permissions) | **Not available** (shown read-only) |

## After installing the snap

### Open Voltkit once

Open **Voltkit** from your apps. It starts its background service, sets it to start at every login, and adds its keyboard shortcuts. The shortcuts run the snap's command, `/snap/bin/voltkit`.

### Connect the permissions

Snaps get access to the system one permission (*interface*) at a time. The ones that are safe for every app are connected automatically: the desktop, the screen, sound, your home folder, settings and the desktop's portals for screenshots, the eyedropper and pasting. The ones below give access to system information, so Snap leaves them for you to allow. Copy these commands into a terminal and press Enter (they ask for your password once):

```bash
sudo snap connect voltkit:removable-media
sudo snap connect voltkit:system-observe
sudo snap connect voltkit:mount-observe
sudo snap connect voltkit:process-control
sudo snap connect voltkit:log-observe
sudo snap connect voltkit:login-session-observe
sudo snap connect voltkit:hardware-observe
```

If your app store shows a **Permissions** section for Voltkit (GNOME Software does), you can switch them on there instead. You only need the ones for the tools you use:

| Permission | Lets Voltkit | Used by |
|---|---|---|
| `removable-media` | use files on USB drives and other external disks | Asset Compressor, Batch Rename, Image Resizer |
| `system-observe` | see which apps have files and ports open, and read drivers and the system's state | What's Using This?, System Report |
| `mount-observe` | list the drives that are connected | What's Using This?, System Report |
| `process-control` | close the apps that keep something busy | What's Using This? |
| `log-observe` | read the sleep history from the system log | Why Didn't It Sleep? |
| `login-session-observe` | see what is keeping the computer awake | Why Didn't It Sleep? |
| `hardware-observe` | read hardware details, temperatures and fans | System Report, GPU Tuner |

### Check

```bash
voltkit doctor
```

Its *Snap Store version* section lists every permission with ✓ (connected) or – (not connected) and the command for each one that is missing, followed by the features that need apt or Fedora. You can also list the connections with:

```bash
snap connections voltkit
```

To take a permission back, disconnect it, for example:

```bash
sudo snap disconnect voltkit:process-control
```

## What works only with apt or Fedora

**This is a limitation of snaps, not of Voltkit.** Every app from the Snap Store runs in a sandbox (*strict confinement*) that keeps it away from the rest of the system. A snap:

- **can't ask for your administrator password.** Voltkit uses `pkexec` for the few things only the administrator may do; inside a snap it isn't allowed;
- **can't change system files**, such as `/etc/hosts` or the files in `/sys` that control devices;
- **can't write to your hidden settings folders** (`~/.config` and the like): it gets its own private copy instead;
- **can only talk to the system services Snap allows**, which leaves out the desktop's permission store and the disk service;
- **can't get access to the keyboard** unless the Snap Store reviews the app by hand.

That protects you from apps you haven't checked, but a few Voltkit features need exactly those things:

| Tool | What the snap can't do | Why |
|---|---|---|
| [GPU Tuner](gpu) | Change settings (power limit, fans, clocks, voltage) and apply profiles at startup. **Monitoring works.** | Settings are written to the graphics driver by a helper that runs as administrator after asking for your password (`pkexec`). A snap can't ask for it, and applying a profile at startup needs a system service, which a snap can't install. |
| [Hosts & Environment](system) | The whole tool. | `/etc/hosts` is a system file only the administrator can change. Environment variables are saved in your hidden settings folder (`~/.config/environment.d`), which a snap can't write to: it only sees its own private copy, so changes would have no effect. |
| [App Permissions](permissions) | The whole tool. | The answers you gave apps are kept by the desktop's permission store (`xdg-permission-store`), a service snaps aren't allowed to talk to. |
| [Text Expansion](snippets) | Typed triggers such as `;date`. **The snippet picker** (<kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd>) **works.** | Typed triggers read the keyboard and type for you, which needs a one-time setup by the administrator (a rule in `/etc/udev`). A snap can't make that change, and the Snap Store reserves keyboard access for apps it reviews by hand. |
| [System Report](sysreport) | Drive health, memory slots and firmware update checks. **Everything else works** once its permissions are connected. | Drive health comes from the system's disk service (UDisks), which snaps aren't allowed to query; memory slot details are in a table (SMBIOS) only the administrator can read; firmware checks use the `fwupdmgr` tool, which isn't inside the snap. |
| [What's Using This?](using) | See and close apps that belong to other users or to the system. **Your own apps work** once its permissions are connected. | Other users' and the system's apps are visible only to the administrator, and closing them needs your password. |
| [Why Didn't It Sleep?](sleep) | Stop a USB device from waking the computer. **Everything else works** once its permissions are connected. | Which devices may wake the computer is a system setting; changing it, and keeping it after a restart, needs the administrator password. |

None of this can be fixed with `snap connect`: the Snap Store doesn't offer these permissions to apps like Voltkit. If you need them, [switch to apt or Fedora](#from-the-snap-to-apt-or-fedora).

## How the app tells you

In the snap, every affected tool says so at the top of its page, so you know right away:

- **"Connect 2 permissions to use everything here"** (or similar): some permissions above aren't connected yet. **Details** lists what each one is for and the exact `sudo snap connect` commands, with a **Copy Commands** button. The banner disappears once they are connected (it checks again whenever you open the page).
- **"Part of this tool works only in the APT and Fedora versions"**: the tool works, but one feature is left out. **Details** says which, and why.
- **"This tool works only in the APT and Fedora versions of Voltkit"**: the whole tool is unavailable. Its page is shown read-only so nothing is saved to the wrong place.

**Details** also has **How to Install the Full Version**, which opens the [Install](install) page.

## Installing only one version

Install Voltkit **one way only**. With both the snap and the apt or Fedora package installed:

- your apps show **two Voltkit entries**;
- **two background services** start at login, but only one can own the keyboard shortcuts, so shortcuts may run the wrong copy or seem to do nothing;
- each copy keeps **its own settings and snippets**, so changes in one don't show in the other.

### From the snap to apt or Fedora

**1.** Optional: keep your snippets and settings. The snap keeps them in its own folder; copy them to where the apt or Fedora version looks:

```bash
mkdir -p ~/.config/voltkit && cp ~/snap/voltkit/current/.config/voltkit/config.toml ~/.config/voltkit/
```

**2.** Remove the snap:

```bash
sudo snap remove voltkit
```

**3.** Install with [apt](install#ubuntu-debian-and-derivatives) or [Fedora](install#fedora) and open Voltkit once. It sets its shortcuts to the new copy.

### From apt or Fedora to the snap

**1.** Optional: keep your snippets and settings. Save a copy where you can find it:

```bash
cp ~/.config/voltkit/config.toml ~/voltkit-config-backup.toml
```

**2.** If you used typed triggers, remove their keyboard access while the apt or Fedora version is still installed:

```bash
voltkit expander remove-access
```

**3.** Remove the apt or Fedora version. On Ubuntu and Debian:

```bash
sudo apt remove voltkit
```

On Fedora:

```bash
sudo dnf remove voltkit
```

**4.** Install the snap and open Voltkit once, so its shortcuts and start-at-login are set up:

```bash
sudo snap install voltkit
```

**5.** If you saved your settings in step 1, put them where the snap looks, then reload them:

```bash
mkdir -p ~/snap/voltkit/current/.config/voltkit && cp ~/voltkit-config-backup.toml ~/snap/voltkit/current/.config/voltkit/config.toml && voltkit daemon reload
```

**6.** [Connect the permissions](#connect-the-permissions).

## Updates and removal

Snaps update themselves in the background, usually within a few hours of a release. To update now, or to see the installed and published versions:

```bash
sudo snap refresh voltkit
```

```bash
snap info voltkit
```

To remove the snap:

```bash
sudo snap remove voltkit
```

This also removes the snap's settings and snippets (Snap keeps a snapshot for a while, see `snap saved`). Copy `~/snap/voltkit/current/.config/voltkit/config.toml` first if you want to keep them.

## Troubleshooting the snap

| Problem | What to do |
|---|---|
| A banner says to connect permissions, but I did | Open another tool and come back: the page checks again when it is shown. `snap connections voltkit` shows what is really connected; check the command was typed as `voltkit:` followed by the permission name. |
| Keyboard shortcuts do nothing or open the wrong copy | Check that only one Voltkit is installed (`snap list voltkit` and `apt list --installed voltkit`). If both are, remove one as in [Installing only one version](#installing-only-one-version), then open the remaining one once. |
| `voltkit doctor` says the background service isn't running | Open Voltkit once, or log out and in again: the snap starts the service at login. |
| A tool's page is greyed out | That tool needs apt or Fedora: see [What works only with apt or Fedora](#what-works-only-with-apt-or-fedora). |
| Files on a USB drive can't be opened | Connect `removable-media` (`sudo snap connect voltkit:removable-media`). |
| Text Extractor doesn't read my language | The snap includes English only. For other languages, install with apt or Fedora and add the Tesseract language packs. |
