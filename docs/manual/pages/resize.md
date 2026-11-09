# Image Resizer

Image Resizer makes smaller (or bigger) copies of many pictures at once: for email, a website, a phone, or just to save space. It writes the copy next to each original and **never changes the original**.

![Image Resizer with three pictures resized to 1920 pixels](shot:resize)

## Use it, step by step

1. Open **Image Resizer**.
2. **Add the pictures.** Drop them on **Drop Pictures Here**, or click **Choose Pictures…** and select them (several at once). Each one appears in the **Pictures** list with its size in pixels and its file size. Add more at any time. The trash button on a row takes that picture out of the list, and **Clear** empties the list.
3. **Choose the size** in the panels on the right (see below), or click one of the ideas under **Which size should I use?** to set it for you.
4. **Click Resize.** The button says how many pictures it will do (*Resize 3 Pictures*). Each row shows its progress and then its result: the old and new size in pixels, the file size before and after, and the name of the new file.
5. Find the copies beside the originals, named `name (WxH).ext`. `beach.jpg` resized to 1920 × 1440 becomes `beach (1920x1440).jpg`. A copy never replaces a file: if the name is taken it becomes `beach (1920x1440) 2.jpg`.

Nothing is resized until you click **Resize**, so you can add pictures and change the settings in any order.

**Want a different size too?** Change the size and click **Resize Again**: it makes new copies of everything in the list with the current settings. The originals are never touched, and the earlier copies stay.

Supported pictures: PNG, JPEG, WebP, GIF, BMP and TIFF. Dropped files of other kinds are ignored.

## Which size should I use?

Click an idea to set the controls, then click **Resize**.

| Idea | What it sets | Good for |
|---|---|---|
| **Email or chat** | Medium: 1280 px on the longest side | Pictures small enough to send that still look sharp on a screen |
| **Website or blog** | Large: 1920 px on the longest side | A full-width page |
| **A post for social media** | Width 1080 px, height open (0) | Feeds that scale to 1080 px wide |
| **A square thumbnail** | 256 × 256 px, **Fill the size and trim the edges** | Avatars and small previews |
| **A phone wallpaper** | 1080 × 2400 px, **Fill the size and trim the edges** | A tall phone screen |
| **A 4 × 6 inch print** | 1800 × 1200 px (300 dots per inch), **Fit inside the size** | Photo prints |
| **Half the size** | 50 percent | Any picture you just want smaller |

Not sure? **Large: 1920 px** is a good default for sharing, and **Medium: 1280 px** for email.

## Size

**Make the pictures** offers these choices (or use an idea above):

| Choice | Result |
|---|---|
| **Small: 640 px on the longest side** | The longest side becomes 640 px, keeping the shape |
| **Medium: 1280 px…** | 1280 px |
| **Large: 1920 px…** (the default) | 1920 px |
| **Very large: 3840 px…** | 3840 px |
| **A share of their size** | **Percent of the original**, from 1 to 500 |
| **A width and height** | **Width (px)** and **Height (px)**; `0` leaves one open and the other follows the shape. Choose **When the shape is different**: **Fit inside the size**, **Fill the size and trim the edges** (the middle is kept), or **Stretch to exactly that size** |

**Allow enlarging** is off by default: a picture smaller than the size you asked for is left at its own size. Turn it on to make small pictures bigger (they get blurry).

## Output

| Control | What it does |
|---|---|
| **Format** | **Same as the original**, **PNG**, **JPEG** or **WebP**. WebP needs the WebP pixbuf loader on your system; if it is missing, Voltkit tells you which package to install. |
| **Quality** | 1 to 100 for JPEG and WebP (default 90). PNG is always lossless. |

## What happens to the picture

- **Rotation is applied.** A photo taken sideways (its EXIF orientation) comes out upright.
- **Other metadata is not copied** into the new file, so details such as where a photo was taken are left out.
- The original is never opened for writing.

## From the terminal

```bash
voltkit resize photos/*.jpg --longest-side 1280
voltkit resize *.png --percent 50
voltkit resize banner.jpg --width 1200 --height 400 --mode fill
voltkit resize *.png --longest-side 800 --format webp --quality 85
```

Choose the size with `--longest-side`, `--percent`, or `--width` and/or `--height` (default: `--longest-side 1920`). Other options: `--mode fit|fill|stretch`, `--enlarge`, `--format same|png|jpeg|webp`, `--quality N`. The command prints each result with its sizes and file sizes, and exits with an error if any picture failed.

## Troubleshooting

- **A format fails.** *This computer can't write WebP pictures* means the WebP pixbuf loader is missing (package `webp-pixbuf-loader`).
- **Nothing happens when I add pictures.** That is expected: add them, then click **Resize**.
- **The picture did not get smaller.** It may already be smaller than the size you asked for: turn on **Allow enlarging**, or choose a smaller size.
- **A file is not accepted.** Only PNG, JPEG, WebP, GIF, BMP and TIFF pictures are resized.
