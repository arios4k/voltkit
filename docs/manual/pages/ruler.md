# Screen Ruler

Screen Ruler freezes your screen and lets you measure anything on it in real pixels: a button, an image, the gap between two elements.

![Screen Ruler page with its launch button and measurement controls](shot:ruler)

## Use it

1. Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>R</kbd>, or click **Open Ruler** on the page. The screen freezes and fills the display.
2. **Drag** to measure a rectangle. Its width and height in pixels are shown as you drag.
3. Hold <kbd>Shift</kbd> while dragging to measure a **straight line** instead.
4. Press <kbd>Ctrl</kbd>+<kbd>C</kbd> to copy the measured size to the clipboard.
5. **Right-click** to clear the measurement and measure again.
6. Press <kbd>Esc</kbd> to close the ruler.

## Controls

| Action | Result |
|---|---|
| Drag | Measure a rectangle (width × height) |
| <kbd>Shift</kbd> + drag | Measure a straight line |
| <kbd>Ctrl</kbd>+<kbd>C</kbd> | Copy the measured size |
| Right-click | Clear the measurement |
| <kbd>Esc</kbd> | Close the ruler |

## Accuracy

Measurements use screenshot pixels, including on high-density displays, so you measure what is really drawn, not scaled units.

## From the terminal

```bash
voltkit ruler          # open the ruler
voltkit trigger ruler  # the same as the shortcut, through the background service
```

## Troubleshooting

- **The ruler opens behind other windows.** Open it from the shortcut or the button; Voltkit asks the desktop to bring the new window to the front.
- **The shortcut does nothing.** See [Global shortcuts and the background service](daemon).
