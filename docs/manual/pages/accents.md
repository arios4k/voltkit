# Typing accents and symbols

Need é, ñ, ü, ç, €, © or « »? Linux has a built-in way to type them in **every app**: the **Compose key**. You press it, then two keys, and the character appears. No popup, no menu, and it works everywhere you can type.

## Turn it on

1. Open **Shortcut Detective** in Voltkit and scroll to **Keyboard keys**.
2. Under **Compose key**, choose a key: **Right Alt** is the usual choice (the Menu key and Right Ctrl are good on a keyboard that has no spare Right Alt).
3. That is all. It works at once, in every app.

> [!TIP] Not using Voltkit or not on GNOME? The Compose key is a standard setting: in GNOME Tweaks it is under *Keyboard → Compose Key*, and in KDE under *System Settings → Keyboard → Advanced*.

## Use it

1. **Tap** the Compose key and let go. It does not need to be held.
2. Press the **two keys** of the sequence, one after the other.

For example, for **é**: tap Compose, then <kbd>'</kbd>, then <kbd>e</kbd>. For a capital **É**, hold <kbd>Shift</kbd> for the letter: Compose, <kbd>'</kbd>, <kbd>Shift</kbd>+<kbd>E</kbd>.

## Common sequences

| To type | Press (after Compose) | Examples |
|---|---|---|
| **Acute accent** | <kbd>'</kbd> and a letter | `'e` → é, `'a` → á, `'o` → ó, `'u` → ú, `'i` → í |
| **Grave accent** | <kbd>`</kbd> and a letter | `` `e `` → è, `` `a `` → à, `` `u `` → ù |
| **Circumflex** | <kbd>^</kbd> and a letter | `^e` → ê, `^o` → ô, `^i` → î |
| **Diaeresis (umlaut)** | <kbd>"</kbd> and a letter | `"u` → ü, `"o` → ö, `"a` → ä, `"e` → ë |
| **Tilde** | <kbd>~</kbd> and a letter | `~n` → ñ, `~a` → ã, `~o` → õ |
| **Cedilla** | <kbd>,</kbd> then <kbd>c</kbd> | `,c` → ç |
| **Ring** | <kbd>o</kbd> then <kbd>a</kbd> | `oa` → å |
| **German sharp s** | <kbd>s</kbd> then <kbd>s</kbd> | `ss` → ß |
| **Ligatures and slashed o** | | `ae` → æ, `oe` → œ, `o/` → ø |
| **Spanish marks** | | `??` → ¿, `!!` → ¡ |
| **Money** | | `=e` → €, `L-` → £, `Y=` → ¥ |
| **Symbols** | | `oc` → ©, `or` → ®, `tm` → ™, `oo` → °, `+-` → ±, `xx` → × |
| **Quotes and dashes** | | `<<` → «, `>>` → », `--.` → – (en dash), `---` → — (em dash), `..` → … |
| **Fractions** | | `12` → ½, `14` → ¼, `34` → ¾ |

The sequences come from your system's Compose table, so a few differ slightly with your language and locale. If one does not work, try it the other way round (`e=` for €).

## Other ways to type special characters

- **A different keyboard layout.** *English (US, international, with dead keys)* turns <kbd>'</kbd>, <kbd>`</kbd>, <kbd>"</kbd> and <kbd>~</kbd> into accent keys, and your language's own layout may already have the letters you need. Add layouts in your desktop's keyboard settings.
- **By name.** The **Characters** app (GNOME) or the emoji picker finds any character by what it is called.
- **By code.** In most GTK apps, press <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>U</kbd>, type the character's Unicode code (`e9` for é), and press <kbd>Space</kbd> or <kbd>Enter</kbd>.
- **Often needed?** A [Text Expansion](snippets) snippet can type any text for you, such as a trigger `;eur` that types €.

## Troubleshooting

- **Nothing happens.** Check that a Compose key is chosen under **Keyboard keys**, and tap and release it before the two keys (do not hold it).
- **The wrong character appears.** Your system's table may use a different sequence: try the keys in the other order.
- **Some apps ignore it.** A few apps that handle keyboard input themselves (some games and remote-desktop windows) do not use the Compose table.
