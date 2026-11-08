# Profiles: settings that switch by themselves

A **profile** is a named set of GPU settings (for example *Quiet*, *Balanced*, *Performance*). Voltkit's GPU service can **switch between profiles automatically**: on battery, while a game runs, under a sustained load, or when the card runs hot.

![GPU Tuner, Profiles tab: saved profiles with their automatic rules](shot:gpu-3)

Profiles belong to one card. Each row shows the profile's name and a summary of what it sets (for example *Power limit 220 W · Fan 35 %*).

## Make a profile

1. On the [Tune tab](gpu-tune), turn on and set the controls you want.
2. On the **Profiles** tab, click **Save current settings as a profile…**.
3. Type a name (*Name, for example Quiet*) and click **Save**. Saving under an existing name replaces that profile.

The settings turned on in Tune are stored under that name. Nothing is applied yet.

## Choose when a profile switches in

Expand a profile and pick an option under **Switch to it**:

| Option | The profile is used… |
|---|---|
| **Not automatic** | only when you apply it yourself |
| **On battery** | while the computer runs on battery |
| **On mains power** | while it is plugged in |
| **While an app runs** | while any program in your list is running |
| **When the GPU is busy** | when the GPU is busier than a percentage for a while |
| **When the GPU runs hot** | when the card is hotter than a temperature |

Options come with their own field:

- **Programs or Steam games (steam:<App ID>), separated by commas** for *While an app runs*. Names match the program's process name (`blender`, `steam`), not windows. For a **Steam or Proton game**, type `steam:` and the game's App ID, for example `steam:1245620`: Voltkit recognizes games started by Steam (including under Proton) by their App ID, so it works whatever the game's executable is called. Find a game's App ID in its store page address.
- **Busier than (%)** for *When the GPU is busy*: the load must last 30 seconds.
- **Hotter than (°C)** for *When the GPU runs hot*.

Also on each profile:

- **Use when no rule applies**: the default profile for when nothing else matches. Only one profile can be the default.
- **Copy a share code**: see below.
- **Delete this profile** removes it and every rule that pointed to it.

### How rules are chosen

Rules are tried in order and the first that matches wins. **Heat rules always go first**: a card that is too hot must cool down whatever else is going on. To keep a profile from flapping, a profile stays at least **20 seconds** before another replaces it, a load rule needs the load to last **30 seconds**, and a heat rule lets go only after the card has cooled **5 °C** below its trigger.

## Turn the profiles on

Click **Apply profiles**. It installs the profiles in Voltkit's background GPU service, which switches between them (the desktop asks for your administrator password). The service applies the active profile at startup and follows the rules while the computer runs. It also runs fan curves that the card cannot follow itself.

The service's activity can be seen with `voltkit gpu profiles active`.

## Share a profile

Profiles can be shared as **one line of text**:

- **Copy a share code** (inside a profile) copies a line starting with `voltkit-profile:1`. It holds the profile's name, the card it was made on (as a hint) and its settings. It contains **no PCI address and no personal data**.
- **Import a shared profile…** asks you to paste a code, checks it and adds it as a **new** profile. A profile with the same name is never replaced: it becomes *Name (2)*. Nothing is applied until you choose to.

Voltkit refuses nonsense before it gets near a card: a missing or invalid code, a name that is empty or over 40 characters, a power limit outside 10 to 1500 W, a fan speed under 20 %, an invalid fan curve, or offsets outside normal ranges. After importing, the toast reminds you that *every chip behaves a little differently, so try it before you rely on it*. When you apply it, the normal checks against your own card's limits and the confirm-or-revert step still apply.

## Where profiles are stored

- Your edits: `~/.config/voltkit/gpu-profiles.json`.
- What the service runs: `/etc/voltkit/gpu-profiles.json`, written by **Apply profiles**.
- The active profile: `/run/voltkit/gpu-active.json`.

## If a profile freezes the computer

A saved profile is applied by the GPU service with a **startup guard**: it leaves a marker before applying and removes it after the computer has stayed up for a minute. If the marker is still there at the next start, the profile is skipped and disabled until you turn it back on, so a profile that freezes the machine cannot do it again and again. See [Safety](gpu-safety).
