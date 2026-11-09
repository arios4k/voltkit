# Shortcut Detective

You pressed a shortcut and nothing happened, or the wrong thing did. **Shortcut Detective** shows every system-wide keyboard shortcut, who owns it, which ones **clash**, and free combinations you can switch to. You can change, turn off or reset any shortcut right there.

![Shortcut Detective showing that Super+V is used by two actions](shot:shortcuts)

It works on **GNOME** and the desktops built like it (**Cinnamon, Budgie, MATE**), which keep their shortcuts in GSettings. On other desktops, such as KDE Plasma, the page says so and points you to *System Settings → Shortcuts*.

## The cheat sheet

Click **Cheat Sheet** (next to **Record Shortcut**), or press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>/</kbd> from any app, to open a **cheat sheet** of your shortcuts: a window you read at a glance and close with <kbd>Esc</kbd>.

![The shortcut cheat sheet, grouped by who owns each shortcut](shot:guide)

- Shortcuts are grouped by who owns them: **Voltkit** first, then **Windows**, **GNOME Shell**, then the rest alphabetically (system, extensions…). Actions without a shortcut are left out.
- Each shortcut shows its keys as key caps; when an action has several, they are shown side by side with *or*.
- A last group, **In most apps**, lists the editing shortcuts almost every app shares (copy, paste, undo, find, save and so on).
- **Type to find a shortcut…**: filter by action, group or keys (`overview`, `alt+f4`, `windows`). *No shortcuts match* means nothing fits.
- It is read-only. To change a shortcut, use Shortcut Detective's **Change…**.

The cheat sheet shows the **system-wide** shortcuts. Apps can have more of their own, and Wayland does not let Voltkit tell which app is in front, so it cannot show the shortcuts of the app you are using. On a desktop whose shortcuts Voltkit cannot read, only the **In most apps** group is shown.

The hotkey is `shortcut_guide` in the [configuration file](config). From the terminal: `voltkit shortcut-guide`.

## What it covers

Voltkit reads the shortcuts of: the window manager, GNOME Shell, system and media keys, your custom shortcuts, enabled shell extensions, and Voltkit's own hotkeys. Each action shows what it does in plain words and who it belongs to (*Windows*, *GNOME Shell*, *Voltkit*, an extension name…).

## Press a shortcut to investigate

1. Click **Record Shortcut**.
2. Press the key combination when the page says *Press the keys now*. <kbd>Esc</kbd> cancels (*Recording cancelled. Choose Record Shortcut to try again.*).
3. The details show every action that uses it:
   - **1 action uses this shortcut**: *Available as a system-wide shortcut. Apps may still use it within their windows.*
   - **N actions share this shortcut**: a clash. Change an action to resolve it.
4. Under **Free combinations to try**, Voltkit suggests alternatives: the same key with other modifiers first, then nearby keys. It leaves out combinations apps use for themselves (like <kbd>Ctrl</kbd>+<kbd>S</kbd>), so a suggestion never steals an app's shortcut.

A bare letter or number cannot be a shortcut because it types text; a combination needs a modifier.

## Browse and find

- **All Shortcuts** lists everything: *Registered shortcuts · N of M*. Type in **Filter shortcuts by action, owner, or keys…** to narrow it. *No shortcuts match these filters* means nothing fits.
- **Conflicts** lists only the combinations used by more than one action: *Conflicts · N*.
- **Refresh shortcuts** reads them again, for example after you changed something in Settings.
- Actions without a shortcut show **Not set**; clashing ones show a **Conflict** badge.

## Change a shortcut

Select an action, then:

| Button | What it does |
|---|---|
| **Change…** (**Change Shortcut**) | Opens a recorder. Press the new combination. Voltkit tells you *{keys} is free*, or *{keys} is already used by {action}. Pick another, or change that one first.* Click **Save**. |
| **Turn Off** | Removes the shortcut so the action has none |
| **Reset to Default** | Puts it back how the desktop came |

You can also click one of the **Free combinations** to use it. A toast confirms: *"{action}" is now {keys}*, *has no shortcut now* or *is back to its default*. An action can have several shortcuts; changing one keeps the others. Shortcuts locked by the system administrator say **Locked by the system administrator** and cannot be changed.

Voltkit's own hotkeys also live in its [configuration file](config); changing them here keeps the two in step.

## Related commands

```bash
voltkit shortcuts list                 # every system-wide shortcut, grouped by owner
voltkit shortcuts who super+v          # what a shortcut does
voltkit shortcuts clashes              # combinations used by more than one action
voltkit shortcuts install              # add Voltkit's hotkeys to GNOME Settings (--dry-run to preview)
voltkit shortcuts remove               # remove the ones install added
```

`shortcuts install` and `remove` are for desktops that do not offer the GlobalShortcuts portal. See [Global shortcuts and the background service](daemon).

## Troubleshooting

- **"Shortcuts aren't available on this desktop."** Your desktop does not keep shortcuts in GSettings. Use its own settings.
- **A shortcut works in the window but not globally.** Apps can use keys inside their own windows; this page only lists system-wide shortcuts.
