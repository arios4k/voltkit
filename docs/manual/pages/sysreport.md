# System Report

**System Report** answers "what is in this computer, and is it healthy?" It is not just a list of specifications: it checks each piece of hardware and tells you what the result means, with the command to fix a problem where one is safe. It also remembers how your computer looked on earlier days, so you can see exactly what changed after an update.

![System Report page with checks, recent changes and the computer's hardware](shot:sysreport)

The report loads when the page opens (*Checking this computer…*). Click **Check Again** to read everything again.

## Overview: This Computer

A table of what the computer is, one row per kind of hardware that applies: **Computer** (the model, for example a Lenovo ThinkPad), **System** (the distribution), **Kernel**, **Desktop**, **Processor** (cores, threads and top speed), **Memory** (how much is usable), **Graphics**, **Storage** (each drive with its size), **Network**, **Audio**, **Bluetooth**, **Camera** and the **BIOS** version. Rows the computer does not have are left out.

## Checks

Each check has a level and a plain-language explanation:

| Level | Meaning |
|---|---|
| **OK** | Fine. For example *Intel UHD Graphics works* with its driver and version |
| **Info** | Worth knowing, nothing is wrong. For example *Restart to finish installing updates* or *A newer Linux kernel is installed* |
| **Warning** | Something to plan for. For example a drive that is *N % worn* |
| **Problem** | Needs fixing. For example *has no driver* or a drive reporting failure |

### What it checks

- **Graphics, network and audio drivers.** Is each device on a working driver, a basic fallback, or no driver at all? For a missing or fallback driver, Voltkit shows the fix: on Ubuntu-like systems the command `sudo ubuntu-drivers install`, and on Fedora and Arch with NVIDIA a link to the right guide. Other devices without a driver are flagged too.
- **Drive health.** Using the drive's own self-assessment (through UDisks2, no password): failing drives, damaged sectors, SSD wear and the hours it has run.
- **Disk space.** *The system drive* and *Your home folder* are flagged **getting full** and **almost full**, because apps and updates can fail on a full disk.
- **Temperatures.** Sensors that run hot get a warning with advice to check the air vents. The page also shows **Temperatures** with each sensor labelled **normal**, **warm** or **hot**, the fans, and how many times the processor **slowed down because of heat** since the computer started.
- **Updates waiting for a restart** and a **newer kernel installed**.
- **Firmware updates** from the makers (see below).

For each check with a command, **Copy the command, to paste in a terminal** puts it on your clipboard (*Command copied: paste it in a terminal*). For problems with no single safe command, **Open Guide** opens the distribution's guide. Voltkit never runs fixes for you.

## Firmware Updates

Updates for the BIOS, drives, docks and other devices, from the makers through the **Linux Vendor Firmware Service** (fwupd). They often fix battery, sleep and stability problems.

- **Up to date** *as of the last check*. **Check Online** downloads the latest list (this is the only network use on the page, and only when you click it).
- When updates exist, each shows the device, the current and the available version and a summary. **Install…** opens your desktop's software updates (GNOME Software), where firmware updates are installed. If that is not available it copies `fwupdmgr update` for you to run in a terminal.
- If fwupd is not installed: *Firmware updates aren't available. Install fwupd to get firmware updates from the makers of your hardware.*

## Memory and Upgrades

Memory slots, what is in them and how much the computer can take. Reading them needs the administrator password, so the page has a **Show…** button: it asks once through polkit and reads the SMBIOS tables. You see:

- **Installed**: total memory, type and speed.
- **Slots**: how many exist and how many are used. It says if memory is soldered to the board and cannot be upgraded.
- Each slot: size and type, *soldered*, or *empty*.
- **Maximum**: *Up to N GB (as the computer reports it)*, with plain advice about upgrading.

From the terminal: `voltkit report --memory`.

## What Changed

Voltkit saves a snapshot of your hardware, drivers, kernel and firmware **each day it runs** and keeps the last 90. **What Changed** compares consecutive days and lists the differences, newest first, each with the date it was first seen. It is handy when something stops working after an update: *Hardware, drivers, kernel and firmware, compared day by day*.

On the first day the page says *Nothing to compare yet*: changes show up from tomorrow on.

## Copy Report

**Copy Report** shows exactly what will be copied before you copy it (*Copy System Report*), then puts a tidy summary on the clipboard to paste into a forum post or bug report. The report has **no serial numbers, user or computer names, or network addresses**. Only the notable checks (warnings and problems) are listed, so the report stays short.

## From the terminal

```bash
voltkit report            # the report as text, and saves today's snapshot
voltkit report --memory   # also memory slots and upgrade options (asks for the administrator password)
```

## Troubleshooting

- **No drive health.** The drive may not report it (some USB enclosures). Voltkit shows what it can read.
- **Firmware updates are not available.** Install `fwupd` for your distribution.
- **What Changed is empty.** It needs at least two days of snapshots.
