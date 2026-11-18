# Storage Map

Choose **Scan Folder…** or drop a folder onto the scan controls. Storage Map scans in the background and shows how much disk space the folder and its contents occupy.

- The treemap gives larger items more space. Click a folder to drill into it or a file to show its location in your file manager.
- The list below the map works with the keyboard. Open a row with Enter. Choose **Largest files** to see the biggest files throughout the current folder and its subfolders.
- Use the up arrow to return to the parent, or **Rescan** to refresh the original selection.

Sizes use the blocks allocated by the filesystem. Sparse files can use less space than their file size suggests. Hard links count once per scan, and shared blocks such as reflinks can still be counted more than once. Symbolic links are skipped, and the summary reports unreadable entries. Scanning makes no changes to your files.

The Snap can scan folders it can access. To scan external drives, connect the removable-media interface as described in [Snap Store version](snap).

## From the terminal

```bash
voltkit storage-map ~/Downloads
voltkit storage-map ~/Downloads --largest-files --limit 10
```

The default lists up to 20 immediate children ordered by disk usage. `--largest-files` searches the full tree for files. `--limit` sets how many entries to print. See `man voltkit-storage-map`.
