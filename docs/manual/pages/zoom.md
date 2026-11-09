# Zoom & Annotate

Zoom & Annotate freezes your screen, lets you **zoom into any part of it and draw on it**, and copies or saves the result. It is for presentations, tutorials, bug reports, or pointing something out to a friend.

![Zoom & Annotate: an arrow, a circle, text and a highlighter drawn on a frozen screen](shot:zoom-overlay)

## Use it

1. Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd>, or click **Open Zoom & Annotate** on the page. Voltkit minimizes its own window, takes a snapshot of your screen and shows it full screen.
2. Zoom with the mouse wheel, draw with the left button.
3. Press <kbd>Ctrl</kbd>+<kbd>C</kbd> to copy the picture, <kbd>Ctrl</kbd>+<kbd>S</kbd> to save it (in the folder shown on the page, `Pictures/Voltkit` by default), and <kbd>Esc</kbd> to close.

**Nothing is saved automatically.** If you close with <kbd>Esc</kbd> without copying or saving, the drawing is gone.

The first time, your desktop asks whether Voltkit may take screenshots; choose **Allow**.

> [!NOTE] Wayland does not let apps draw over other windows, so Zoom & Annotate works on a **snapshot**, like the [Screen Ruler](ruler). What you draw on is frozen at the moment you opened it.

## Zoom and move

| Action | Result |
|---|---|
| Scroll | Zoom in and out around the pointer (100 % to 800 %) |
| <kbd>+</kbd> / <kbd>-</kbd> | Zoom in and out around the middle of the screen |
| <kbd>0</kbd> | Back to 100 % |
| Middle-drag, or the arrow keys | Move the picture while zoomed in |

## Draw

| Action | Result |
|---|---|
| Drag | Draw freehand with the pen |
| <kbd>Shift</kbd> + drag | A straight line |
| <kbd>Ctrl</kbd> + drag | A rectangle |
| <kbd>Tab</kbd> held + drag | An ellipse |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd> + drag | An arrow |
| <kbd>T</kbd>, then click | Type text where you clicked. <kbd>Enter</kbd> finishes, <kbd>Shift</kbd>+<kbd>Enter</kbd> starts a new line, <kbd>Esc</kbd> cancels. |
| <kbd>H</kbd> | Highlighter on or off: wide and translucent |
| <kbd>R</kbd> <kbd>G</kbd> <kbd>B</kbd> <kbd>O</kbd> <kbd>Y</kbd> <kbd>P</kbd> | Red, green, blue, orange, yellow or pink |
| <kbd>Ctrl</kbd>+scroll, or <kbd>[</kbd> and <kbd>]</kbd> | A thinner or thicker pen (1 to 40) |
| <kbd>W</kbd> / <kbd>K</kbd> | A white or a black board instead of the screen. Press again to go back. |

A ring at the pointer shows the pen's size and color. Everything you draw stays in place when you zoom, because it is attached to the picture, not to the screen.

## Fix and finish

| Action | Result |
|---|---|
| Right-click, or <kbd>Ctrl</kbd>+<kbd>Z</kbd> | Undo the last mark |
| <kbd>Ctrl</kbd>+<kbd>Y</kbd>, or <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Z</kbd> | Redo. Drawing something new forgets what could be redone. |
| <kbd>E</kbd> | Erase everything. This cannot be undone. |
| <kbd>Ctrl</kbd>+<kbd>C</kbd> | Copy the picture to the clipboard, to paste into a document or chat |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Save a PNG in your save folder as `annotation-DATE-TIME.png` (see below) |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd> | Save this one picture somewhere else: a file chooser opens in your save folder |
| <kbd>F1</kbd> or <kbd>?</kbd> | Show every key on the screen |
| <kbd>Esc</kbd> | Close (or leave text mode or help first) |

Copying and saving give the picture at the screen's **full resolution**, with the marks on it and without the zoom, the pen ring or the status line. A status line at the bottom shows the tool, color, pen size and zoom: *Pen · Red · 4 px · 100 %*.

## Where pictures are saved

By default **<kbd>Ctrl</kbd>+<kbd>S</kbd>** saves in your **Pictures** folder, in a folder called `Voltkit` (for example `/home/you/Pictures/Voltkit`), as `annotation-DATE-TIME.png`, for example `annotation-20261109-142530.png`. Each save makes a new file, so nothing is overwritten. The folder is created if it does not exist.

You always see where it goes:

- The **Zoom & Annotate** page has a **Saving pictures** section: **Pictures are saved in** shows the current folder.
- In the overlay, press <kbd>F1</kbd>: the help shows *Ctrl+S saves in /home/you/Pictures/Voltkit*.
- After saving, the status line says **Saved** with the full path.

![The Zoom & Annotate page, with the Saving pictures section showing the folder and its Change button](shot:zoom)

### Change the folder

1. Open **Zoom & Annotate** in Voltkit.
2. Under **Saving pictures**, click **Change…**.
3. Choose a folder and confirm. A message says *Pictures will be saved in …*.
4. To go back to the default, click the **undo** button next to **Change…** (it appears when you have chosen your own folder): *Pictures are saved in Pictures/Voltkit again*.

The choice is kept in the [configuration file](config) as `save_dir` in the `[zoom]` section, and the next time you open Zoom & Annotate it uses it. To save **one** picture somewhere else without changing the folder, press <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd> and pick the place.

## From the terminal

```bash
voltkit zoom           # open Zoom & Annotate
voltkit trigger zoom   # the same as the shortcut, through the background service
```

## Troubleshooting

- **The shortcut does nothing.** See [Global shortcuts and the background service](daemon). The shortcut is `zoom` in the [configuration file](config).
- **Ctrl+C copies nothing.** Install `wl-clipboard` (Wayland) or `xclip` (X11). Run `voltkit doctor`.
- **"Screenshots weren't allowed."** Choose **Try Again** and **Allow** when your desktop asks. If you chose Deny before, forget that answer in [App Permissions](permissions).
