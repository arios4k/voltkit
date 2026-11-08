# GPU Tuner: safety, limits and results

GPU Tuner is built to be trustworthy: it asks the driver first, validates, previews, applies in a way that can be undone, and never leaves you stuck.

## The safety model

- **Ask the driver first.** A control is offered only when reading its value back works and the numbers are plausible. A card that answers with nonsense (a laptop GPU reporting a 5 kW power range, seen in testing) is treated as unsupported.
- **Validate.** Values outside the card's reported range are refused. Fixed fan speeds cannot go below 20 %. A fan curve must keep speeds from dropping as the card gets hotter and must reach 60 % by 85 °C.
- **Preview.** Every change is shown as a list of steps, each with the value it will restore.
- **Transactional.** Steps run in order. If one fails, the earlier ones are undone in reverse. If an undo fails too, you are told exactly which setting is stuck.
- **Confirm or revert.** The change is applied by a root helper that waits for your **Keep**. Nothing else keeps it: a click on Revert, the countdown ending, the app closing, or the app crashing all undo the change.
- **Startup guard.** A saved profile is applied by `voltkit-gpu.service`. It leaves a marker before applying and removes it after the computer has stayed up for a minute. If the marker is still there at the next start, the profile is skipped and disabled until you turn it back on, so a profile that freezes the machine cannot do it again and again. Open GPU Tuner to turn it back on.
- **Fans go back to the card.** When the service stops, fans return to the card's own control. With no temperature reading, a software curve runs the fan at full speed.

## What is tested

Detection, planning, rollback, the confirm protocol, the startup guard and the fan governor are covered by automated tests against fake hardware that can fail at any step. **The write paths have not been run on real AMD hardware, and NVIDIA writes have not run on a card that supports them**: the development machine has an Intel GPU and a laptop Quadro P520, which supports none of the controls. Treat each control as untested on hardware until you have tried it on your own card, and start with small changes.

Monitoring was checked against `nvidia-smi` on one NVIDIA card: temperature, utilization and clocks matched, and monitoring uses about 0.04 % of one processor core at a reading every 2 seconds.

## Help improve it

Real results are the most useful contribution right now. Open a [GPU Tuner hardware report](https://github.com/arios4k/voltkit/issues/new?template=gpu-hardware-report.yml) with the output of `voltkit gpu`, your distribution, kernel and driver, what you tried and what happened.

## Not built yet

- A **stability scanner** that raises clocks under load and backs off on errors: it needs a load generator and real crashes to test safely.
- Comparative testing against other tools on AMD and Intel hardware.
- A separate fan curve per fan or sensor, and matching programs by window instead of process name.
