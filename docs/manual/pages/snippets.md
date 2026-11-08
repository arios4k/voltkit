# Text Expansion

Text Expansion keeps your reusable text (signatures, replies, dates, code) as **snippets** and pastes them into any app. You can use them two ways: the **snippet picker** (a search window) and, optionally, **typed triggers** (type `;date` anywhere and it expands).

![Text Expansion with searchable snippets and insertion controls](shot:snippets)

## The snippet picker

Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> from any app (or click **Open Picker** on the page).

![The snippet picker listing snippets with live previews](shot:picker)

1. Type to search by trigger or text.
2. Move with <kbd>↑</kbd> <kbd>↓</kbd>, press <kbd>Enter</kbd> to insert, <kbd>Esc</kbd> to cancel.
3. If the snippet has fill-in fields, a small form asks for them first.
4. The expansion is **pasted into the app you were using**, and your clipboard is put back afterwards.

The first time, the desktop asks for permission to let Voltkit press <kbd>Ctrl</kbd>+<kbd>V</kbd> for you (the RemoteDesktop portal, which is how Wayland lets one app paste into another). The permission is remembered. If pasting is not possible, the picker copies the snippet and says *Press Ctrl+V to paste it*.

From a terminal: `voltkit picker`, or `voltkit picker --snippet ;sig` to open directly on one snippet.

## Manage your snippets

On the **Text Expansion** page, under **Your Snippets**:

| Action | How |
|---|---|
| Find a snippet | Type in **Find a snippet by trigger or content…**; the header shows *N of M* matches |
| Add | **Add Snippet** |
| Edit | Click a snippet, or its **Edit** button |
| Delete | **Delete**; an **Undo** toast lets you bring it back |

Changes are saved to `~/.config/voltkit/config.toml` and reach typed triggers immediately. If you edit that file by hand, run `voltkit daemon reload`.

### The snippet editor

![Snippet editor with a trigger, text with Insert buttons and a live preview](shot:snippet-editor)

| Field | Meaning |
|---|---|
| **Trigger** | A short code that names the snippet. You find it by this name in the picker, and type it when typed triggers are on. Start it with `;` to avoid clashes with normal words. It cannot contain spaces and must be unique. |
| **Text** | What gets pasted. |
| **Insert:** buttons | **Date**, **Time**, **Clipboard** (whatever you copied last) and **Fill-in Field** (a blank you fill in each time) add the right variable at the cursor. |
| **Preview** | Exactly what would be pasted right now. Fill-in fields show as ‹Fill-in› and the clipboard as ‹clipboard›. |

Voltkit warns you if a trigger is empty, has spaces, is already used, or would expand before you can finish typing a longer one.

## Variables

Write variables in braces inside the text:

| Variable | Expands to |
|---|---|
| `{date}` | Today's date, `2026-11-07` style (`%Y-%m-%d`) |
| `{date:FORMAT}` | The current date and time in any [strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) format, such as `{date:%A, %d %B %Y}` |
| `{time}` | The current time, `14:30` style (`%H:%M`) |
| `{time:FORMAT}` | Same as `{date:FORMAT}` |
| `{clipboard}` | The text currently on the clipboard |
| `{arg:NAME}` | A fill-in field called `NAME`, asked for when you use the snippet |
| `{{` and `}}` | A literal `{` or `}` |

An unknown variable, an unclosed brace or an invalid date format is reported instead of being pasted. Examples:

```toml
[[snippets]]
trigger = ";sig"
template = "Best regards,\n{arg:name}"

[[snippets]]
trigger = ";meet"
template = "Meeting notes {date:%Y-%m-%d}\n\nAttendees: {arg:who}"
```

You can test a template without pasting it:

```bash
voltkit expand 'Hi {arg:who}, today is {date:%A}' --arg who=Sam
```

## Typed triggers (optional)

With typed triggers on, typing `;date` in **any** app replaces it with the snippet. They are **off by default**.

### Turn them on

1. On the page, turn on **Typed triggers**.
2. Read the explanation in **Allow Typed Triggers?** and click **Allow**. The desktop asks for your administrator password **once**.
3. It works right away: no terminal, no logging out.

The row shows the state: *Off*, *Starting…*, *Typed triggers are on*, or a message explaining what is missing: *Needs keyboard access: turn it off and on again to allow it*, *On, but the background service isn't running: start it with the banner above*, or *On, but not working right now* (run `voltkit expander status` to see why).

### Turn them off

Turn the switch off. Voltkit asks **Also Remove Keyboard Access?**; choose **Remove** to take the access away again, or leave it to turn them back on later without a password.

### How they work and what they can see

- Wayland hides keystrokes between apps, so typed triggers need access to the keyboard. Voltkit reads key presses through the system's input devices and keeps **only the last 64 typed characters in memory**. It never logs, saves or sends what you type.
- When the characters end with a trigger, Voltkit erases the trigger with Backspace and pastes the expansion, restoring your clipboard afterwards.
- Snippets with fill-in fields open the picker's form instead of expanding at once.
- Setup installs one udev rule that gives the person at the computer access to keyboards and `/dev/uinput`, the way desktops grant devices to the logged-in user. Turning typed triggers off and removing access deletes `/etc/udev/rules.d/70-voltkit-typed-triggers.rules` and `/etc/modules-load.d/voltkit-uinput.conf`; access already granted ends at the next restart.

> [!WARNING] Keyboard access applies to every program the person at the computer runs, not only Voltkit. Turn typed triggers on only if you want that, and remove the access when you stop using them.

### Limits

- Keys are decoded with a US keyboard layout. Triggers made of letters, digits and `;` work on most layouts; punctuation may differ.
- A trigger that begins another trigger (`;d` and `;date`) fires first, so the longer one cannot be reached.
- Terminals paste with <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd>. Set `paste_shortcut` in the `[expander]` section of the [configuration file](config) (`"ctrl+v"`, `"ctrl+shift+v"` or `"shift+insert"`).

### From the terminal

```bash
voltkit expander status          # enabled? permissions ok?
voltkit expander setup           # print the setup commands to review
voltkit expander setup --apply   # run them (asks for the administrator password)
voltkit expander enable          # turn typed triggers on
voltkit expander disable         # turn them off
voltkit expander remove-access   # turn off and remove keyboard access
voltkit daemon reload            # re-read snippets after editing the config file
```

## Troubleshooting

- **Typed triggers do nothing.** `voltkit expander status` says what is wrong. The background service must be running and keyboard access allowed.
- **The picker will not paste.** Allow the RemoteDesktop permission when the desktop asks; otherwise use <kbd>Ctrl</kbd>+<kbd>V</kbd> after the picker copies the snippet.
- **The trigger expands in the wrong place or never.** Check for a shorter trigger that begins it, and for a non-US layout.
