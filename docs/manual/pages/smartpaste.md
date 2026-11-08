# Smart Paste

Smart Paste lets you **paste what you copied, converted**. Instead of <kbd>Ctrl</kbd>+<kbd>V</kbd>, press <kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd> and choose how it should land: tidy JSON, a spreadsheet range as a Markdown table, a link without its tracking, a PDF paragraph without broken lines, UPPERCASE, `snake_case` and more.

![Smart Paste page turning a spreadsheet range into a Markdown table and CSV](shot:smartpaste)

## Use it in any app

1. **Copy something**: text, cells from a spreadsheet, JSON, a link, or a screenshot.
2. **Click where it should go**: an email, a document, a chat, a terminal.
3. **Press the Smart Paste shortcut** (<kbd>Super</kbd>+<kbd>Shift</kbd>+<kbd>V</kbd>). A small window lists the ways the clipboard can be pasted, each with a **preview** of exactly what would be pasted.
4. **Choose one and press <kbd>Enter</kbd>.** Use <kbd>↑</kbd> <kbd>↓</kbd> to choose and <kbd>Esc</kbd> to cancel.

Only the conversions that **fit what you copied** are offered, and "Plain text" is always first. Your clipboard is left as it was after pasting. If pasting is not possible, Smart Paste copies the result and says *Press Ctrl+V to paste it*. As with the [snippet picker](snippets), the desktop asks once for permission to paste for you.

If nothing useful is on the clipboard you see **Nothing to Paste**: copy some text or an image first.

## Copied an image?

Smart Paste reads the text in a copied screenshot (OCR, locally) and offers to paste it. You see *Reading the Text in the Image…*, then the text. If the image has no text Voltkit can read, it says **No Text in the Image**. Recognition uses the language set in [Text Extractor](ocr).

## All conversions

| Conversion | When it is offered | What it does |
|---|---|---|
| **Plain text** | always | The text without formatting, with Unix line endings |
| **Formatted JSON** | the text is valid JSON | Indents it readably |
| **Compact JSON** | the text is valid JSON | Removes all whitespace |
| **Markdown table** | tab-separated cells (a spreadsheet range) or a CSV | Builds a Markdown table |
| **CSV** | tab-separated cells | Converts to comma-separated values, quoting where needed |
| **Link without tracking** | a single link with tracking parameters | Removes `utm_*` and well-known trackers such as `fbclid`, `gclid`, `msclkid`, `mc_cid`, `igshid` and others |
| **Readable link** | a link with percent codes | Turns `%20` and similar into real characters when the result is valid text |
| **Decoded Base64** | text that is Base64 and decodes to readable text | Shows the decoded text |
| **Join broken lines** | text copied from a PDF or email with a line break in every line | Joins each paragraph into one line, keeps blank lines between paragraphs, rejoins hyphenated words |
| **One line** | multi-line text | Puts everything on one line |
| **Tidy spaces** | extra spaces, tabs or blank lines | Removes trailing and repeated spaces and blank lines at the ends |
| **Bulleted list** | several lines | Adds bullets (an existing bullet or number is replaced) |
| **Numbered list** | several lines | Adds numbers (an existing bullet or number is replaced) |
| **Quote** | text | Prefixes each line with `> ` |
| **UPPERCASE**, **lowercase** | ordinary words | Changes the letter case |
| **Title Case**, **Sentence case** | ordinary words | Capitalizes words, or only the first letter of each sentence |
| **snake_case**, **kebab-case**, **camelCase** | a short phrase or identifier | Converts to the code-style naming |

A conversion that would change nothing, or that does not fit, is not shown. Case conversions are not offered for links or JSON.

## The page

The **Smart Paste** page in the app shows the same ideas without leaving the window:

- **Use it in any app**: the four steps above, with your current shortcut.
- **Source text**: type or paste text here to try the conversions.
- **Conversions**: one row per conversion that fits, with a **Copy** button (it changes to **Copied**). Rows that do not fit say **Only plain text fits this**.

## From the terminal

The same conversions work in scripts, reading from standard input:

```bash
voltkit convert                         # list the conversions
wl-paste | voltkit convert markdown-table
cat data.json | voltkit convert json-compact
echo "Hello world" | voltkit convert snake    # hello_world
voltkit smart-paste                      # open Smart Paste for the current clipboard
voltkit trigger smart-paste              # the same as the shortcut
```

Conversion names for `voltkit convert`: `plain`, `json`, `json-compact`, `markdown-table`, `csv`, `clean-link`, `decode-link`, `base64-decode`, `join-lines`, `one-line`, `tidy-spaces`, `bullets`, `numbered`, `quote`, `upper`, `lower`, `title`, `sentence`, `snake`, `kebab`, `camel`.

## Troubleshooting

- **The shortcut does nothing.** See [Global shortcuts and the background service](daemon).
- **A conversion is missing.** It is only offered when it fits what you copied. Check the source text.
- **Nothing is pasted.** Allow the RemoteDesktop permission when the desktop asks, or press <kbd>Ctrl</kbd>+<kbd>V</kbd> after Smart Paste copies the result.
