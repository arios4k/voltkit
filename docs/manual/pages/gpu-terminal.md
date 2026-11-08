# GPU Tuner on the command line

Everything GPU Tuner does is also a command: `voltkit gpu` and its subcommands. Changing settings needs administrator rights, and anything you do not confirm is undone.

## Commands

| Command | What it does |
|---|---|
| `voltkit gpu` / `voltkit gpu show` | Each card with its readings and which controls it supports, with the reason for any it does not |
| `voltkit gpu apply FILE` | Apply settings from a JSON file, undoing them unless you confirm |
| `voltkit gpu forget ADDRESS` | Remove a card's saved settings and give its fans back to the card |
| `voltkit gpu service` | Run the background service that applies saved settings and follows fan curves |
| `voltkit gpu record` | Record a card's readings to compare profiles later |
| `voltkit gpu sessions` | List recorded sessions, newest first |
| `voltkit gpu compare A B` | Compare two sessions (a file, or a number from `sessions`) |
| `voltkit gpu advise` | Suggest the power limit that saves the most for almost the same speed |
| `voltkit gpu idle` | Find out why the video memory stays at a high clock while idle |
| `voltkit gpu check` | Check Voltkit's readings against `nvidia-smi` and measure what monitoring costs |
| `voltkit gpu profiles list` | List saved profiles and the rules that switch them |
| `voltkit gpu profiles active` | Show which profile each card is using now |
| `voltkit gpu profiles install FILE` | Install profiles and rules from a JSON file |

## Apply settings: `voltkit gpu apply`

```bash
sudo voltkit gpu apply settings.json            # asks to keep, undoes after 15 s
sudo voltkit gpu apply settings.json --yes      # keep without asking
sudo voltkit gpu apply settings.json --confirm 30   # wait 30 seconds for confirmation
sudo voltkit gpu apply settings.json --save     # keep after restarts; run fan curves in the service
cat settings.json | sudo voltkit gpu apply -    # read from standard input
```

The file holds the settings for one card. Anything you leave out is not touched:

```json
{
  "gpu": "0000:03:00.0",
  "power_limit_w": 220,
  "fan": { "mode": "curve", "points": [
      { "temp_c": 30, "speed_pct": 25 },
      { "temp_c": 60, "speed_pct": 50 },
      { "temp_c": 85, "speed_pct": 100 } ] },
  "temperature_target_c": 75,
  "core_offset_mhz": -100,
  "mem_offset_mhz": 200,
  "voltage_offset_mv": -50
}
```

| Key | Meaning |
|---|---|
| `gpu` | The card's PCI address, as `voltkit gpu` shows it |
| `power_limit_w` | Power limit in watts, within the card's range |
| `fan` | `{"mode":"auto"}`, `{"mode":"fixed","speed_pct":50}` or `{"mode":"curve","points":[…],"sensor":…,"fans":[…]}` |
| `temperature_target_c` | 40 to 100 |
| `core_offset_mhz`, `mem_offset_mhz` | Clock offsets |
| `voltage_offset_mv` | Voltage offset |
| `voltage_curve` | AMD cards with a full curve: every point as `{"mhz":1450,"mv":800}`, all of them |
| `core_clock_min_mhz`, `core_clock_max_mhz` | Intel clock limits |

Values the card does not allow are refused.

## Profiles from a file: `voltkit gpu profiles install`

The file is a list with one entry per card:

```json
[
  {
    "gpu": "0000:03:00.0",
    "profiles": [
      { "name": "Quiet", "settings": { "gpu": "0000:03:00.0", "power_limit_w": 180 } },
      { "name": "Performance", "settings": { "gpu": "0000:03:00.0", "power_limit_w": 260 } }
    ],
    "rules": [
      { "profile": "Quiet", "conditions": [ { "when": "power_source", "on_battery": true } ] },
      { "profile": "Performance", "conditions": [ { "when": "app", "names": ["steam:1245620", "blender"] } ] }
    ],
    "default": "Quiet"
  }
]
```

Condition types:

| `when` | Fields |
|---|---|
| `power_source` | `on_battery`: true or false |
| `app` | `names`: program names, or `steam:<App ID>` |
| `workload` | `util_above_pct`, `for_secs` |
| `temperature` | `above_c` |

A rule can have several conditions; all must hold. Rules are tried in order. `install` replaces the saved profiles, turns the service on and prints *Profiles installed.*

## Record and compare

```bash
voltkit gpu record --seconds 120 --label Stock       # while a game or benchmark runs
voltkit gpu record --seconds 120 --label Undervolt --gpu 0000:03:00.0
voltkit gpu sessions                                 # numbered list
voltkit gpu compare 2 1                              # two sessions, by number or file
voltkit gpu advise                                   # which power limit to use
```

`record` prints a summary: duration, temperature, power, **energy used**, **active clock per watt**, the energy cost when an electricity price is set, and how often the card was held back by the power limit and by heat.

## The service

`voltkit gpu service` is what `voltkit-gpu.service` runs (installed by the package, enabled by **Apply profiles** or `profiles install`). It applies the active profile, follows the rules, runs software fan curves and returns the fans to the card's own control when it stops.

```bash
systemctl status voltkit-gpu.service
sudo systemctl disable --now voltkit-gpu.service
```

## Check Voltkit's readings

`voltkit gpu check --seconds 10` compares what Voltkit reads (through the NVIDIA management library) with `nvidia-smi` taken at the same moments and reports how far apart they are, plus the CPU time and memory monitoring uses. Cards from other makers have no second source on most computers; the command says so.
