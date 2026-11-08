# Why Didn't It Sleep?

A laptop that is warm in your bag, or a battery that is empty in the morning, usually has a reason. **Why Didn't It Sleep?** shows what is keeping the computer awake right now, every sleep of the last week with what started and ended it and how much battery it used, and which USB devices are allowed to wake it.

![Why Didn't It Sleep? page listing what keeps the computer awake and recent sleeps](shot:sleep)

## Right now

Under **Right Now**, Voltkit lists the apps and settings that keep the computer awake at the moment. Each item says what it prevents and why.

- **Apps** that asked the system not to sleep (a video player, a download manager, Voltkit's own [Keep Awake](awake)). Voltkit's own Keep Awake has a **Turn Off** button right on the row.
- **Power settings**: automatic sleep turned off in the desktop's power settings, and what closing the lid does. The **Settings** button opens the power settings.
- Each item is marked by what it does: the computer **can't sleep at all**, the **screen stays on and automatic sleep doesn't happen**, or **closing the lid doesn't put it to sleep**.

If nothing is holding it awake you see **Nothing is keeping it awake**: *It sleeps when you close the lid, choose Suspend, or leave it idle.*

**Sleep Mode** explains the kind of sleep this computer uses when it matters (for example a lighter sleep that uses more battery).

Click **Check Again** after you change something.

## Recent sleeps

**Recent Sleeps** covers the last 7 days, newest first, up to 12 entries. Each sleep shows when it started and ended and how long it lasted, and under it:

- **Went to sleep because…** what started it (the lid, the power button, a menu choice, being idle).
- **Woken by…** what ended it. Sleeps ended by something other than the lid or power button are the interesting ones.
- **Battery** before and after, and the drain in percent per hour. A drain above 1.5 % an hour is flagged as *more than usual*, a hint that something is waking or stressing the computer during sleep.

A **lid closed but stayed awake** entry appears when the lid was closed for over two hours without the computer sleeping. It is usually because a monitor or dock was connected, it was plugged in, or an app kept it awake.

If the history cannot be read, the page says *The sleep history isn't available* with the reason; if there were none, *No sleeps in the last week*.

## Devices that can wake it

**Devices That Can Wake It** lists USB devices (mice, keyboards, hubs, receivers) that are allowed to wake the computer, with how many times each woke it since it started (*Woke the computer once since it started*, *Woke the computer N times since it started*).

Each device has a switch (**Can wake the computer** / **Can't wake the computer**). Turn off the one that keeps waking your laptop in the bag. Voltkit applies it now and **keeps it after restarts** with a small udev rule (`/etc/udev/rules.d/90-voltkit-wakeup.rules`). It asks for the administrator password, because changing wake-up devices is a system setting. If none are connected you see *No USB devices that could wake it are connected*.

## What it reads

Everything comes from the system's own records: the logind and GNOME session inhibitors, the power settings, the system journal (systemd-sleep and logind messages), UPower's battery history and the USB devices in sysfs. Only changing which devices may wake the computer needs administrator rights. Nothing is sent anywhere.

## From the terminal

```bash
voltkit sleep    # the same report as text
```

## Troubleshooting

- **The history is empty.** The journal must be readable by your user and keep at least a week of logs.
- **A device switch asks for a password and fails.** Changing wake-up devices needs `pkexec` (polkit).
