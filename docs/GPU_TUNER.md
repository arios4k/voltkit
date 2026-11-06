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

## Not yet built

Per-GPU and per-application profiles with automatic switching (workload,
temperature, power source), recorded sessions and side-by-side profile
comparison with performance per watt, mouse-drag editing of curves on a graph,
voltage and frequency curve editing for cards with full curve tables, multiple
fan and sensor selection for software curves, and comparative testing against
other tools.
