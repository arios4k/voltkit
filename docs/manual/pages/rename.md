# Batch Rename

Batch Rename renames many files at once. You describe the change (search and replace, a regular expression, numbering, or a new letter case), look at the **preview** that names every result and every problem, and only then rename. A batch is renamed **all or nothing**, and the last batch can be undone.

![Batch Rename with a preview of five photos being renamed](shot:rename)

## Use it

1. Open **Batch Rename** and drop files or folders on the page, or use **Add Files…** and **Add Folders…**. Add more at any time; **Clear** starts again.
2. Set what to change under **Search and replace**.
3. Read the **Preview**: each item shows its old name and its new name, a green check when it is ready, a warning when it has a problem, and *Unchanged* when its name stays the same. The line under **Preview** counts them: *5 to rename, 1 unchanged, 0 with problems*.
4. Click **Rename**. It is available once something will change and nothing has a problem.
5. Changed your mind? **Undo Last Rename** puts the old names back.

Items are numbered in the order they appear in the list. Dropping a folder renames the folder itself; to rename the files inside it, drop those files.

## Search and replace

| Control | What it does |
|---|---|
| **Search for** | The text to find. Leave it **empty** to match the whole name, so a replacement renames everything. |
| **Replace with** | The text to put instead. `{n}` is the item's number and `{n:3}` pads it to 3 digits (`007`). |
| **Regular expression** | Treats the search as a regular expression. In the replacement, `$1`, `$2`… are the groups you marked with ( ), and `${name}` is a named group. Off: the search and the replacement are plain text and `$` means a dollar sign. |
| **Match case** | On: `img` does not match `IMG`. |
| **Replace every match** | On (the default): every match in the name. Off: only the first. |
| **Change** | **The name, not the extension** (the default: `photo` in `photo.jpg`), **Only the extension** (`jpg`), or **The whole name**. A leading dot belongs to the name, so `.bashrc` has no extension, and a folder never has one. |
| **Letter case** | After the replacement: **Keep as it is**, **UPPERCASE**, **lowercase** or **Title Case**. |
| **Numbering** | **Start at** and **Step**: where `{n}` begins and how much each next item adds. |

### Example: number a folder of videos

To rename `lecture one.mp4`, `intro.mp4`, `final cut.mp4`… to `1.mp4`, `2.mp4`, `3.mp4`:

1. Drop the `.mp4` files on the page.
2. Leave **Search for** empty.
3. Type `{n}` in **Replace with** (or `{n:2}` for `01.mp4`, `02.mp4`).
4. Keep **Change** on *The name, not the extension*, so `.mp4` stays.
5. Check the **Preview**, then click **Rename**.

The numbers follow the order of the list. From a terminal, `voltkit rename *.mp4 --replace '{n}' --apply` numbers them in alphabetical order.

### Examples

| Goal | Search | Replace with | Options |
|---|---|---|---|
| `IMG_0412.jpg` → `Holiday_0412.jpg` | `IMG` | `Holiday` | |
| Number everything: `shot-001.png`, `shot-002.png` | *(empty)* | `shot-{n:3}` | |
| `report 2026-11-07.pdf` → `report 07.11.2026.pdf` | `(\d{4})-(\d{2})-(\d{2})` | `$3.$2.$1` | Regular expression |
| `.jpeg` → `.jpg` | `jpeg` | `jpg` | Change: Only the extension |
| Replace spaces with dashes | ` ` (a space) | `-` | |
| `HELLO WORLD.TXT` → `hello world.TXT` | *(empty)* | *(empty)* | Letter case: lowercase, Change: The name |

## Problems are found before anything happens

An item is marked with a warning, and **Rename stays off**, when its new name:

- is **empty**, is `.` or `..`, has a **slash**, or is over 255 bytes long;
- would be the **same as another item's** new name ("Another item gets the same name");
- is **already taken** by a file that is not being renamed ("A file with that name already exists").

A name that is free only because its owner is renamed away is fine, so shifting `c1 → c2 → c3` works, and so does swapping two names.

An invalid regular expression is explained in the preview instead of the list.

## All or nothing, and undo

Renaming goes through temporary names, so swaps and chains work. If anything fails half-way, **everything done so far is put back** and you see why. After a successful rename, Voltkit remembers the batch (only the last one) in `~/.local/share/voltkit/rename-undo.json`.

**Undo Last Rename** reverses it. It refuses, changing nothing, if one of the renamed items has moved or deleted, or if an old name has been taken in the meantime. After a rename or an undo, the list follows the items under their new names, so you can rename them again.

## From the terminal

```bash
voltkit rename *.jpg --find IMG --replace Holiday          # preview only
voltkit rename *.jpg --find IMG --replace Holiday --apply  # really rename
voltkit rename *.txt --regex --find '(\d{4})-(\d{2})' --replace '$2.$1'
voltkit rename *.png --replace 'shot-{n:3}' --apply        # number them
voltkit rename --undo                                       # undo the last batch
```

Options: `--regex`, `--ignore-case`, `--first` (only the first match), `--scope name|extension|full`, `--case keep|upper|lower|title`, `--start N`, `--step N`. Without `--apply` the command only prints the preview, and it refuses to rename anything while any item has a problem.

## Troubleshooting

- **Rename is greyed out.** Read the warnings in the preview, or there is nothing to change.
- **Undo says the rename can't be undone.** A renamed file was moved or deleted, or its old name is now used by something else.
- **My regular expression does nothing.** Turn on **Regular expression**; otherwise the search is plain text.
