# Sessions: record, compare, cost and suggestions

The **Sessions** tab answers "which settings are actually better?". You record how the card behaves while a game or benchmark runs, then compare two recordings, see what a session cost in electricity, and ask for the power limit that saves the most for almost the same speed.

![GPU Tuner, Sessions tab: recorded sessions, electricity price and the power-limit suggestion](shot:gpu-4)

## Record a session

1. Start your game or benchmark.
2. In **Sessions**, type a **Label** for what you are trying (for example the profile name).
3. Click **Start recording**. The button changes to **Stop and save**.
4. Click **Stop and save** when you are done: *Session saved.*

A session is a CSV file you can open in a spreadsheet, with a row per second: temperature, hotspot, utilization, clocks, power, fan, video memory, the power limit and why the card was held back, plus an optional `fps` column. Files are saved in `~/.local/share/voltkit/gpu-sessions/`. The page lists the most recent ones with the date, length, average power and, if you set a price, the cost.

From the terminal: `voltkit gpu record --seconds 120 --label Quiet`.

## Compare two sessions

Tick the check boxes of two sessions and click **Compare the two ticked**. A larger window lists the measures **by category**, each naming **one better session** (or **Same**). The older session is "before" and the newer one "after".

![The session comparison dialog](shot:gpu-compare)

| Category | Measures compared |
|---|---|
| **Temperature** | average and highest temperature; time held back by heat |
| **Power and energy** | average and highest power; energy used (Wh); time held back by the power limit |
| **Fan** | average fan speed |
| **Clock speed** | average core clock; active clock |
| **Efficiency** | active clock per watt |
| **Frames** | average frames per second; frames per second per watt |

A category is won by the session that is ahead on more of its measures; on a tie, by its first measure with a clear winner. Differences under half a percent count as a tie, so noise never decides. Categories a session cannot measure (a card that reports no power, no `fps` column) are left out.

> [!NOTE] **Efficiency is a proxy.** *Active MHz per watt* is the core clock weighted by how busy the card was, divided by the power it drew. It says how much clock-work each watt bought, not how many frames. Add an `fps` column to a session file (for example from MangoHud) and frames per second and frames per watt are reported too.

From the terminal: `voltkit gpu sessions` lists them and `voltkit gpu compare 1 2` compares two by number.

## What a session cost

Type your electricity price in **Electricity price per kWh**, for example `0.15`, `0,15 €` or `$0.15`, and press the apply button. Voltkit saves the number and the currency sign in the [configuration file](config). Each session then shows its energy cost, so you can see what a profile saves over a month. Leave the field empty to turn it off.

## Suggest a power limit

Click **Suggest a power limit**. Voltkit reads your sessions recorded under **different power limits** and finds the lowest limit that keeps almost all the speed (within 3 % of the fastest):

1. Record the same game or benchmark under at least two different power limits (once per limit).
2. Click **Suggest a power limit**. You get a table of every limit you tried with its performance and power, and a suggestion such as *Suggestion: run at about 220 W. It draws 16 % less power for 1.3 % less speed.*

How it decides:

- Sessions under 30 seconds, or on a mostly idle card (under 50 % busy), say nothing about a limit and are ignored.
- Limits within 5 W count as the same; several runs at one limit are averaged.
- Performance is **frames per second** when every session has frames, otherwise **active megahertz**.
- It only suggests limits **you actually measured**. If no lower limit stays within the tolerance: *No lower limit you tried stays within 3 % of the fastest. Record sessions at more limits to look further.*

Then set that limit on the [Tune tab](gpu-tune) or save it as a [profile](gpu-profiles). From the terminal:

```bash
voltkit gpu advise                 # default tolerance 3 %
voltkit gpu advise --tolerance 5   # accept up to 5 % less speed
voltkit gpu advise --gpu 0000:03:00.0
```
