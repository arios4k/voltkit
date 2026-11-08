# GPU Tuner

**GPU Tuner** monitors NVIDIA, AMD and Intel graphics cards and, where the card and its driver allow it, changes their power limit, fans, fan curves, temperature target, clock offsets or limits and voltage. It works from what each driver reports, and tells you plainly when a control is not available, why, and what (if anything) would enable it.

![GPU Tuner, Monitor tab: live readings with history and what the card supports](shot:gpu)

> [!IMPORTANT] Applying changes is new and has so far been tested against simulated hardware only, plus read-only monitoring on real cards. The page marks the controls **Experimental**. Everything you do not confirm is undone, but a wrong voltage can still crash the computer. Start with small steps, one setting at a time. See [Safety](gpu-safety).

## The four tabs

The page is organized into tabs so nothing is crammed:

| Tab | What is in it |
|---|---|
| [**Monitor**](gpu#the-monitor-tab) | Live readings with history, and what this card supports |
| [**Tune**](gpu-tune) | Power limit, fan, temperature target, clock offsets or limits, voltage |
| [**Profiles**](gpu-profiles) | Named settings that switch by themselves, sharing and importing |
| [**Sessions**](gpu-sessions) | Record, compare, electricity cost and the power-limit suggestion |

## Choose a card

With more than one graphics card, a **Graphics card** selector at the top picks which one the tabs show (for example an integrated Intel card and a discrete NVIDIA card). Voltkit opens on the first NVIDIA or AMD card, not on an integrated one. With a single card the selector is not shown.

## The Monitor tab

### Right now

Live readings, refreshed every couple of seconds, each with a small history graph of the last minute and a half:

| Reading | Meaning |
|---|---|
| **Temperature** | The GPU temperature in °C |
| **Utilization** | How busy the GPU is, in % |
| **Power** | Electrical power drawn, in watts |
| **Core clock** | The graphics clock, in MHz |
| **Memory clock** | The video memory clock, in MHz |
| **Fan** | Fan speed in % and, where reported, RPM |
| **Held back by** | Why the card is holding its clock back right now: **the power limit**, **heat**, **another limit**, or **Nothing: full speed**. NVIDIA cards report this. Other cards show a dash |
| **Video memory** | Used and total video memory in MB |
| **Idle power check** | See below |

A reading the card does not report shows as a dash (—): that is normal, for example integrated Intel graphics report no temperature or fan.

### Held back by

Graphics cards slow themselves down for a few reasons. This row tells you which one is active, which is the first thing to know when performance drops:

- **The power limit**: the card is using all the power it is allowed. Raising the power limit may help.
- **Heat**: the card is too hot and is cutting its clock to cool down (including the hardware's own emergency slowdown). Better cooling or a stronger fan curve helps.
- **Another limit**: for example a voltage or sync limit.
- **Nothing: full speed**: nothing is holding it back.

The same information is recorded in [sessions](gpu-sessions) so you can compare how often each profile was held back.

### Idle power check

A common Linux problem: with several monitors, mixed refresh rates or a very fast panel, the driver cannot lower the **video memory clock** when nothing is running, and the card draws tens of watts for nothing. The row **Idle power check** watches for it.

- It only gives a verdict when the card is **idle** (under 10 % busy). While you use the card it keeps the last verdict.
- Memory at or above **900 MHz** with nothing running counts as *stuck high*. Otherwise: *The memory idles at its low clock. Nothing to fix.*
- When stuck, Voltkit names the likely causes from your displays: **several displays** are on (the driver may not find a gap long enough between refreshes), the displays refresh at **different rates**, or a display runs at **119 Hz or more**.
- The fix it suggests: give every display the same refresh rate (60 Hz is the easiest), or lower the fastest one, then check again.
- The refresh rates are each monitor's *preferred* mode (read from its EDID), which may differ from the mode in use, so Voltkit calls them likely causes, not facts. If no display setup that usually causes this is found, a program may be using the card (an overlay, a video, or a browser with hardware acceleration).

From the terminal: `voltkit gpu idle`.

### What this card supports

Next to the readings, a list of every control with a check mark (**supported**) or an information mark (**not available**) and the exact reason, in the driver's terms. For example: *The driver doesn't allow changing the fan on this card. This is common on laptop GPUs.* Where something would enable a control, it says so (for example an AMD card that needs the `amdgpu.ppfeaturemask` kernel option).

The controls are: Monitoring, Fan speed, Fan curve, Power limit, Temperature target, Clock offsets, Clock limits, and Voltage and frequency curve.

## What each vendor allows

| Control | AMD (amdgpu) | NVIDIA | Intel |
|---|---|---|---|
| Monitoring | temperature, hotspot, power, clocks, fan, utilization, memory | the same through the NVIDIA management library (`nvidia-smi` if it is missing) | clocks and utilization; temperature and fan on discrete Arc cards |
| Power limit | within the card's range | within the reported range | discrete Arc cards; integrated graphics share the processor's budget and are read-only |
| Fixed fan speed | yes | yes, every fan | no: firmware controls the fans |
| Fan curve | the driver's own curve when it has one, otherwise followed by Voltkit's service | followed by the service | no |
| Temperature target | RDNA 3 and newer | the acoustic temperature threshold | no |
| Clock offsets | maximum clock or offset style | core and memory offsets | no; clock limits instead |
| Clock limits | no | no | minimum and maximum clock, within the card's range |
| Voltage | voltage offset; a full voltage and frequency curve where the card has one | no: NVIDIA exposes offsets, not a curve | no |

Cards from other makers are listed but not tuned. Laptop GPUs often lock most controls; Voltkit shows them disabled with the reason and never implies software can go past hardware, firmware or driver limits.

## From the terminal

```bash
voltkit gpu            # each card, its readings and which controls it supports (with the reason for any it doesn't)
```

See [GPU Tuner on the command line](gpu-terminal) for every command.
