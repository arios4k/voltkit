# GPU Tuner

GPU Tuner monitors NVIDIA, AMD and Intel graphics cards and changes their power
limit, fans and fan curves, temperature target, clock offsets or limits and
voltage offset. It
works from what each card's driver reports, and says so when a control isn't
available, why, and what (if anything) enables it.

The goal is to be the most trustworthy GPU tuning tool on Linux. That is a goal,
not a claim: it will be judged by the measurements below, and "best on Linux"
stays out of the product copy until comparative testing supports it. GPU Tuner
never implies that software can get past hardware, firmware or driver limits.

## What works today

| Control | AMD (amdgpu sysfs) | NVIDIA (NVML) | Intel (i915 / xe sysfs) |
|---|---|---|---|
| Monitoring | temperature, hotspot, power, clocks, fan, utilization, memory | the same, through NVML (`nvidia-smi` if NVML is missing) | clocks, utilization (from idle time); temperature and fan on discrete Arc cards |
| Power limit | `hwmon/power1_cap`, within the card's range | `nvmlDeviceSetPowerManagementLimit`, within the reported range | `hwmon/power1_max` on discrete Arc cards; integrated graphics share the CPU budget and are read-only |
| Fixed fan speed | `pwm1` with `pwm1_enable` | `nvmlDeviceSetFanSpeed_v2`, every fan | not offered: fans are firmware-controlled |
| Fan curve | the driver's `gpu_od/fan_ctrl/fan_curve` when it exists, otherwise followed by the service | followed by the service | not offered |
| Temperature target | `gpu_od/fan_ctrl/fan_target_temperature` (RDNA 3 and newer) | the acoustic temperature threshold | not offered |
| Clock offsets | `pp_od_clk_voltage` (maximum clock or offset style) | `nvmlDeviceSet{Gpc,Mem}ClkVfOffset` | not offered; clock limits instead |
| Voltage | voltage offset (`vo`) where the table has `OD_VDDGFX_OFFSET`; full curve tables are read but not yet edited | not offered: NVIDIA's driver exposes offsets, not a curve | not offered |

Intel also has **clock limits** (minimum and maximum clock, within the card's RPn to RP0 range), written
in a safe order. Cards from other vendors are listed but not tuned.

## Safety model

- **Ask the driver first.** A control is offered only when its read-back works
  and the numbers are plausible. A card that answers with nonsense (a laptop GPU
  reporting a 5 kW power range, seen in testing) is treated as unsupported.
- **Validate.** Values outside the card's reported range are refused. Fixed fan
  speeds can't go below 20 %. A fan curve must keep speeds from dropping as the
  card gets hotter and must reach 60 % by 85 °C.
- **Preview.** Every change is shown as a list of steps, each with the value it
  will restore.
- **Transactional.** Steps run in order. If one fails, the earlier ones are
  undone in reverse. If an undo fails too, the user is told exactly which
  setting is stuck.
- **Confirm or revert.** The app runs a root helper through `pkexec` that applies
  the change and waits for "keep". Nothing else keeps it: a click on Revert, a
  15-second timeout, the app closing or the app crashing all undo the change.
- **Startup guard.** A saved profile is applied by `voltkit-gpu.service`. The
  service leaves a marker before applying and removes it after the computer has
  stayed up for a minute. If the marker is still there at the next start, the
  profile is skipped and disabled until the user turns it back on, so a profile
  that freezes the machine can't do it again and again.
- **Fans go back to the card.** When the service stops, fans return to the card's
  own control. With no temperature reading, a software curve runs the fan at full
  speed.
- **Smooth fans.** The software curve follows the temperature up quickly (up to
  15 % per 2 seconds) and down slowly (3 %), and only lets the speed fall after
  the temperature has dropped 3 °C below what set it.

## Testing

Detection, planning, rollback, the confirm protocol, the startup guard and the fan
governor are covered by unit tests against fake sysfs trees and a fake hardware
layer that can fail on any step. **The write paths have not been run on real
AMD hardware, and NVIDIA writes have not run on a card that supports them**: the
development machine has an Intel GPU and a laptop Quadro P520, which supports
none of the controls. Treat each control as untested on hardware until the
checklist below has been run on a card that offers it.

## Measurable targets

These are what to measure before claiming more:

- **Coverage:** controls detected and applied correctly across GPU generations
  (RDNA 1/2/3, Polaris/Vega, Pascal through Ada), recorded per card.
- **Monitoring accuracy:** readings against `nvidia-smi` / `amdgpu_top` within 1 °C,
  2 % utilization and 2 W.
- **Responsiveness:** page interaction under 50 ms while monitoring.
- **Resource use:** CPU and memory of the app and the service while idle and
  monitoring (the target is a few tenths of a percent of one core).
- **Recovery:** every injected failure (a failed write, a killed helper, a frozen
  boot) ends with the card back in its previous state.

## Fan curves, profiles and sessions

- **Curve chart.** Drag points on a temperature and fan-speed graph; rubber-band or
  shift-click to select several; arrow keys nudge (Shift for 5); double-click the
  line to add a point; right-click or Delete removes. The rules live in
  `gpu_curve.rs` and keep the curve valid while you drag (temperatures at least a
  degree apart, speeds that never drop, inside the chart). The graph shades the
  zone no safe curve may enter and marks where the card is right now.
- **Sensor and fans.** A curve can follow the hottest sensor, the GPU, the hotspot
  or the memory temperature, and drive all fans or only some (NVIDIA). The
  driver's own curve is used only when the defaults are chosen, because it
  can't follow a chosen sensor.
- **Profiles.** Per-card named profiles, with rules tried in order: on battery or
  mains, while a program runs, under a sustained load, or when hot. A profile
  stays at least 20 seconds, a load must last 30 seconds, and a heat rule lets go
  only after the card has cooled 5 °C. Heat rules go first. "While a program runs"
  matches process names (`steam`, `blender`), not windows. The service applies
  them and writes the active one to `/run/voltkit/gpu-active.json`.
- **Sessions.** `voltkit gpu record` (or the Sessions section) writes a CSV of the
  readings. `voltkit gpu compare` or the page puts two sessions side by side with
  the better one marked. **Efficiency is a proxy**: active MHz per watt (core clock
  weighted by utilization, divided by power). It says how much clock-work each
  watt bought, not frames. Add an `fps` column (for example from MangoHud) and
  frames per second per watt are reported too. Cards that report no power (the
  development machine's laptop Quadro, for one) show no efficiency.
- **Voltage and frequency curve.** For AMD cards whose table has `OD_VDDC_CURVE`,
  every point can be set within the ranges the card reports; only changed points
  are written, each with its old value to undo. Edited as numbers for now, not on
  the chart.

## Measured so far

`voltkit gpu check` compares Voltkit's readings with a second source and measures
the cost of monitoring. On the development machine (Quadro P520, NVIDIA 580):

| Measure | Result | Target |
|---|---|---|
| Temperature, utilization, clocks, Voltkit (NVML) against `nvidia-smi`, 8 paired readings | 0 difference | 1 °C, 5 %, 15 MHz |
| Video memory, same comparison | 0.2 MB (53 MB before the first check found and fixed Voltkit counting the driver's reserved memory as used) | 8 MB |
| CPU per reading of every card | about 0.8 ms, or 0.04 % of one core at a reading every 2 seconds | a few tenths of a percent |
| Memory of the monitoring process | 52 MB (the whole command-line process) | |
| `nvidia-smi` fallback | about 21 ms per reading, in a separate process | |

This is a check of Voltkit against NVIDIA's own tool on one card, not a
comparison with other tuning tools. Accuracy has not been measured on AMD or
Intel, where no second source is installed here. Power, which the Quadro doesn't
report, is untested.

## Reporting hardware results

Real results are the most useful contribution right now. Open a
[GPU Tuner hardware report](https://github.com/arios4k/voltkit/issues/new?template=gpu-hardware-report.yml)
with the output of `voltkit gpu`, your distribution, kernel and driver, what you
tried and what happened. Try one setting at a time and start with small steps; the
page marks the controls **Experimental** until they are confirmed on real hardware.

## Not yet built

Comparative testing against other tools (LACT, CoreCtrl, amdgpu_top, radeontop) on
AMD and Intel hardware, which needs those cards; a separate curve per fan or
sensor; matching programs by window rather than process name; dragging points on
the voltage and frequency curve; and running the write paths on real AMD, Arc and
supported NVIDIA cards, which remains the biggest gap.
