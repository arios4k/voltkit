# QR Code

QR Code has two halves: **Scan** reads a QR code anywhere on your screen, and **Create** turns text, a link or Wi-Fi details into a code you can scan with your phone, copy or print.

![QR Code page with a generated code for a link](shot:qr)

## Scan from the screen

1. Press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>Q</kbd>, or click **Scan from Screen** on the page.
2. Drag a box around a QR code: in a video, a slide, a web page, a photo.
3. The text it holds is copied to your clipboard and shown under **Result**.

If the content is a web link, an **Open Link** button appears. It opens only `http://` and `https://` links, and only when you click it: Voltkit never opens anything by itself. **Copy** copies the text again.

If you cancel the capture, you see *No screenshot was taken*.

## Create a QR code

Under **Create**, choose the **Content** type:

### Text or link

Type or paste any text or a link. The code updates as you type. Very long text is refused with *That's too much text for a QR code*.

### Wi-Fi network

Fill in **Network name**, **Password** and **Security** (**WPA / WPA2 / WPA3** or **None (open network)**). Scanning the code with a phone offers to join the network. The code appears once the network name is filled in.

### Add your own logo (optional)

Click **Choose…** under **Logo** to put a small image (PNG, JPEG, WebP or SVG) in the middle of the code, and pick a **Logo size**: **Small**, **Medium** or **Large**. **Remove Logo** takes it away.

Voltkit makes sure the code still works:

- It uses the highest error correction when a logo covers part of the code. Even the largest logo covers only about 4 % of the code, far below the 30 % that error correction can recover.
- It reads the code back before showing it. You see **✓ Checked: scans correctly with the logo**. If the logo you chose breaks the code, Voltkit makes it smaller and says **✓ Scans correctly (logo made smaller so it fits)**. If it cannot be made to scan, it says *This code doesn't scan with a logo. Shorten the text or remove the logo.*

### Use the code

| Button | What it does |
|---|---|
| **Copy Image** | Copies the code as an image to paste into a document or chat |
| **Save as PNG…** | Saves a PNG file to print or share |

The code is drawn black on white with the quiet margin scanners need.

## From the terminal

```bash
voltkit qr scan                          # drag a box around a code on screen
voltkit qr read picture.png              # print every QR code found in an image file
voltkit qr make https://example.com      # show a QR code in the terminal
voltkit qr make "hello" --output code.png               # save a PNG instead
voltkit qr make https://example.com --output code.png --logo logo.svg   # with a logo (PNG output only)
voltkit trigger qr                       # the same as the shortcut
```

The terminal version draws the code with Unicode blocks. It scans normally on a dark terminal background.

## Troubleshooting

- **The code is not found.** Make the box tighter around the code and make sure the whole code, including its light margin, is inside it. Zoom in on the source first.
- **A logo is rejected.** Voltkit only accepts PNG, JPEG, WebP or SVG images of a reasonable size and shape.
- **The shortcut does nothing.** See [Global shortcuts and the background service](daemon).
