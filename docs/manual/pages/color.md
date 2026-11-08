# Color Picker

Color Picker takes the color of any pixel on your screen, converts it between HEX, RGB and HSL, rates how readable text in that color is on white and on black, and remembers the colors you used.

![Color Picker with a live preview, formats, contrast ratings and history](shot:color)

## Pick a color from the screen

1. Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> from any app, or click **Pick from Screen** on the page.
2. The desktop's eyedropper appears. Click the pixel you want.
3. The HEX code (for example `#7C3AED`) is copied to your clipboard and saved to the history. A notification or toast says *Copied #7C3AED*.

## The page

### Color

Type a color in the **HEX or rgb()** field. It accepts `#rgb`, `#rrggbb` (the `#` is optional) and `rgb(r, g, b)`. The large swatch updates as you type; the **Live color preview** shows the color at a larger size. Press <kbd>Enter</kbd> to save the color to the history.

### Formats

The color in three notations, each with a **Copy** button:

| Format | Example |
|---|---|
| HEX | `#7C3AED` |
| RGB | `rgb(124, 58, 237)` |
| HSL | `hsl(262, 83%, 58%)` |

### WCAG Contrast

The WCAG 2.x contrast ratio of text in this color on a white background (**Text on white**) and on black (**Text on black**), with the rating for normal-size text:

| Ratio | Rating |
|---|---|
| 7 or more | **AAA** (best) |
| 4.5 to 7 | **AA** |
| 3 to 4.5 | **AA Large**: only large text (18 pt, or 14 pt bold, and up) passes |
| under 3 | **Fail** |

Use it to check that a text color on your design's background stays readable. To check two arbitrary colors, pick or type one and read its ratio against white and black.

### History

The eight most recent colors appear as rows. Select one to put it back in the field. Voltkit keeps the last 32 colors, newest first, and the page, the keyboard shortcut and the command line share the same history.

## From the terminal

```bash
voltkit pick-color          # eyedropper, HEX on the clipboard
voltkit color '#3584e4'     # HEX, RGB, HSL and contrast vs white and black
voltkit color 'rgb(53,132,228)'
voltkit trigger color       # the same as the shortcut, through the background service
```

## Troubleshooting

- **The eyedropper does not appear.** It needs the desktop's Screenshot portal. Run `voltkit doctor`.
- **The shortcut does nothing.** See [Global shortcuts and the background service](daemon).
