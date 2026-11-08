# Tune: changing GPU settings

The **Tune** tab changes the card's settings. Every control is shown, but only the ones the selected card supports are enabled; the rest are disabled with the reason (and the charts for fan and voltage curves are shown greyed out so you can see what they would look like).

![GPU Tuner, Tune tab: power limit and the fan curve editor](shot:gpu-2)

## How a change works

1. **Turn on** the controls you want to change with their switches. A control that is off is not touched.
2. Set the values.
3. Click **Preview** to see *What will change*: a list of steps, each with the value it will restore. If you turned nothing on, you see *Nothing to change*. If something cannot be applied, the dialog says why (*This can't be applied*).
4. Click **Apply**. The desktop asks for your administrator password (the change is made by a small root helper started through polkit).
5. A dialog asks **Keep these settings?** with a countdown (15 seconds by default): *If you don't confirm, the card goes back to how it was in N seconds.* Click **Keep** to keep the change or **Revert** to undo it.

**Nothing stays unless you confirm it.** A click on Revert, the countdown ending, closing the app or the app crashing all undo the change. After you confirm: *Settings applied and kept.*

If you cancel the password prompt: *Cancelled, nothing was changed.* If `pkexec` is not installed: *pkexec is needed to change GPU settings.*

### Keep after restart

The **Keep after restart** switch saves these settings and applies them every time the computer starts, through Voltkit's GPU service (see [Safety](gpu-safety) for how a bad setting cannot lock you in a loop). **Fan curves that Voltkit itself follows need this switch**: *This card has no built-in fan curve, so Voltkit's background service follows it. Turn on Keep after restart.*

## Power limit

Turn on **Power limit** and set **Watts** within the range the card reports (*Between N W and M W*). A lower limit makes the card cooler, quieter and more efficient at a small cost in speed; a higher one lets it draw more. See [Sessions](gpu-sessions) to measure the trade-off and get a suggestion.

## Fan

Turn on **Fan** and choose a **Mode**:

| Mode | What it does |
|---|---|
| **The card's own control** | The default automatic fan control |
| **Fixed speed** | A constant **Speed (%)**. It cannot go below 20 % |
| **Curve** | A fan speed that follows the temperature along a curve you draw |
| **Keep under a temperature** | You choose **Keep the card under (°C)** (50 to 90) and Voltkit builds a curve for you: quiet while the card is cooler, ramping up as it nears the target, and at full speed once it gets there |

For **Curve** and **Keep under a temperature** you can also choose:

- **Temperature source**: which temperature the fan follows: the hottest sensor (default), the GPU, the hotspot or the memory temperature, when the card reports more than one.
- **Fans**: which fans to drive (NVIDIA cards with several fans). All by default.

> [!NOTE] The card's built-in curve can only follow its own sensor and drive every fan, so it is used only with the default choices. Otherwise Voltkit's GPU service follows the curve in software, which needs **Keep after restart**.

### The fan curve editor

Under **Curve**, a chart shows fan speed against temperature. The shaded zone is where no safe curve may go.

| Action | How |
|---|---|
| Move a point | Drag it |
| Select several points | Rubber-band around them, or Shift-click |
| Nudge a point | Arrow keys; hold Shift for 5 steps |
| Add a point | Double-click the line, or **Add point** |
| Remove a point | Right-click it or press Delete, or **Remove** in its row |
| Undo / redo | **Undo** and **Redo** buttons |
| Copy / paste a curve | **Copy curve** and **Paste curve** |
| Type exact values | **Curve points**: each **Point N** has a temperature (°C) and a fan speed (%) |

The chart keeps the curve valid while you drag: temperatures at least a degree apart, speeds that never drop as it gets hotter, everything inside the chart. A dot marks where the card is right now.

**Rules every curve must follow:** it needs at least two points; temperatures go up; a hotter point cannot have a slower fan than a cooler one; and the fan must reach **at least 60 % by 85 °C** so the card stays safe.

**Smooth fans:** Voltkit's software curve follows the temperature up quickly (up to 15 % per 2 seconds) and down slowly (3 %), and only lets the speed fall after the temperature has dropped 3 °C below what set it, so the fans do not hunt up and down.

## Temperature target

Turn on **Temperature target** to set the temperature the card's own fan control aims for, between 40 and 100 °C. On AMD this is the driver's fan target temperature (RDNA 3 and newer); on NVIDIA it is the acoustic temperature threshold.

## Clock offsets

Raise or lower the clocks from their defaults, in MHz: **Core (MHz)** from −500 to +500 and **Memory (MHz)** from −1000 to +1000. Negative core offsets are the usual way to run cooler; positive ones to overclock.

## Clock limits (Intel)

Keep the clock between a **Minimum (MHz)** and a **Maximum (MHz)**, within the card's own range. They are written in a safe order.

## Voltage

Two forms, depending on what the card exposes:

- **Voltage offset**: **Millivolts (mV)** from −200 to 0. A lower voltage runs cooler but can crash the card if too low.
![GPU Tuner voltage and frequency curve editor with draggable points](shot:gpu-vf)

- **Voltage and frequency curve** (AMD cards with a full table): set the voltage for every clock the card runs at. Drag the points on the chart or type numbers in **Point N** rows (MHz and mV); the two stay in step, and each point stays inside the box the card allows. A card's curve has a fixed number of points, so none can be added or removed. Only the points you changed are written, each with its old value to undo.

## Validation

Values outside what the card reports are refused before anything is written: the power limit must be inside the card's range, a fixed fan speed cannot go below 20 %, and fan curves must follow the rules above.

## Troubleshooting

- **A control is greyed out.** Open the Monitor tab: *What this card supports* gives the exact reason.
- **The password prompt fails.** Settings are changed through `pkexec`; install polkit for your distribution.
- **I applied something and the screen froze.** Restart. Nothing was confirmed, so nothing is saved. If you used **Keep after restart** and the computer froze before it finished starting, the next start skips the profile: see [Safety](gpu-safety).
