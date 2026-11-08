# Text Extractor

Text Extractor reads text from anywhere on your screen: a Wi-Fi password on a conference slide, a command in a video tutorial, an error message in a screenshot. You drag a box, and the text lands on your clipboard.

![Text Extractor showing text read from a photo of a conference slide](shot:ocr)

## Use it

1. Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>T</kbd> from any app, or open **Text Extractor** in Voltkit and click **Capture Region**.
2. The desktop shows its screenshot tool. Drag a box around the text.
3. Voltkit recognizes the text on your computer and copies it to the clipboard. A notification confirms it.
4. Paste it anywhere with <kbd>Ctrl</kbd>+<kbd>V</kbd>.

If you cancel the screenshot, Voltkit shows *No screenshot was taken* and nothing is copied.

## The page

| Control | What it does |
|---|---|
| **Capture Region** | Starts a capture, exactly like the keyboard shortcut. |
| **Extracted text** | Shows the last text captured, so you can read or select part of it. Text from the keyboard shortcut also appears here when the window is open. |
| **Copy Text** | Copies the extracted text again. It is disabled until something has been captured. |
| Character count | *No text captured yet*, then *N characters* once you have a result. |
| Description line | Shows the recognition language and the current shortcut. |

Text is copied to the clipboard automatically after every capture. **Copy Text** is for copying it again after you have copied something else.

## Languages

Recognition uses [Tesseract](https://github.com/tesseract-ocr/tesseract) with English by default. To read another language:

1. Install its language pack, for example `sudo apt install tesseract-ocr-deu` for German or `tesseract-ocr-spa` for Spanish. More than 100 languages exist.
2. Set the language in the [configuration file](config):

```toml
[ocr]
lang = "eng+deu"   # one or several languages joined with +
```

From the terminal you can choose per run: `voltkit ocr --lang deu`.

## From the terminal

```bash
voltkit ocr                  # select a region, text goes to the clipboard and is printed
voltkit ocr --lang eng+deu   # several languages at once
voltkit trigger ocr          # the same as the shortcut, through the background service
```

## How region capture works

Voltkit asks the desktop's **Screenshot portal** for an interactive region capture, which works on Wayland without extra permissions. If the portal is not available it tries, in this order, `grim` with `slurp`, `gnome-screenshot`, `spectacle` and `maim`. If none exist, it tells you which to install.

## Tips

- Zoom the text in the source (a video, a PDF) before capturing: bigger text reads better.
- Capture one paragraph at a time. Tesseract is most accurate on plain printed text with good contrast.
- Everything happens locally. The image is not uploaded anywhere.

## Troubleshooting

- **Nothing is copied.** Run `voltkit doctor`. Tesseract or a language pack may be missing.
- **Garbled text.** Install the language pack for the text you are reading and set `lang` accordingly.
- **The shortcut does nothing.** The background service may be off. See [Global shortcuts and the background service](daemon).
