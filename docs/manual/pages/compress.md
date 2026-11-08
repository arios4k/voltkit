# Asset Compressor

Asset Compressor shrinks images and videos and removes the hidden metadata inside them (location, camera, editing history). It never touches your originals: it writes a new file next to each one.

![Asset Compressor with four compressed files and their size savings](shot:compress)

## Use it

1. Open **Asset Compressor** and drop files on the window, or click **Choose Files…** and pick them (you can select several).
2. Each file appears under **Results** with its progress and, when done, how much smaller it became.
3. Find the result beside the original, named `name.min.ext`. A file called `photo.jpg` produces `photo.min.jpg`.

You can also drag files from your file manager onto the **Drop Files Here** area.

## Formats

| Format | Extensions | How it is compressed | Needs |
|---|---|---|---|
| PNG | `.png` | Lossless, with every metadata chunk removed. Built into Voltkit. | nothing |
| JPEG | `.jpg`, `.jpeg` | Metadata stripped, lossy at the chosen quality | `jpegoptim` |
| WebP | `.webp` | Metadata removed, lossy at the chosen quality | `cwebp` (package `webp`) |
| MP4 video | `.mp4`, `.m4v`, `.mov` | Re-encoded with H.264 and AAC audio (128 kbps), metadata removed, with the index at the start for fast web playback | `ffmpeg` |

In Voltkit's own tests the typical savings were: JPEG −43 %, PNG −23 % (lossless), WebP −60 %, MP4 −77 %. Your results depend on the files.

## Compression settings

**Quality** goes from 1 (smallest file) to 100 (best quality) and defaults to 80. It controls JPEG, WebP and video. **PNG is always lossless**, so quality does not apply to it. Change the default for every run in the [configuration file](config):

```toml
[compress]
quality = 80
```

## Safety

- Originals are **never modified or deleted**.
- Metadata is removed from the copy only.
- If a helper program is missing, the file fails with a message naming it, and the banner at the top of the window and `voltkit doctor` tell you what to install.

## From the terminal

```bash
voltkit compress photo.jpg                 # writes photo.min.jpg
voltkit compress *.png                     # several files
voltkit compress clip.mp4 --quality 70     # smaller video
```

The command prints each result and the size saved.

## Troubleshooting

- **A format fails.** Install its helper: `jpegoptim` for JPEG, `webp` for WebP, `ffmpeg` for MP4. Run `voltkit doctor` for the command for your distribution.
- **The file did not get smaller.** Already-optimized files may not shrink. Lower the quality for lossy formats.
