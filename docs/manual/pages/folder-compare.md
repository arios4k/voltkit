# Folder Compare

Use Folder Compare to find changes between two folders or check that a backup matches the original.

1. Open **Folder Compare** in the sidebar.
2. Choose or drop an **Original** folder and a **Backup or comparison folder**.
3. Click **Quick Compare** to check file sizes and modification times, or **Verify Contents** to compare file contents byte for byte.
4. Filter the results by status. Open a changed UTF-8 text file to preview both versions, or click **Export Report…** to save the results.

Comparison is read-only. Voltkit checks symbolic link targets without following the links and reports entries it cannot verify. It does not check ownership, permissions, or extended attributes.

For external drives in the Snap, connect the removable-media interface. See [Snap Store version](snap).

## From the terminal

```bash
voltkit folder-compare ~/Documents /media/user/Backup/Documents
voltkit folder-compare ~/Documents /media/user/Backup/Documents --verify --all
voltkit folder-compare ~/Documents /media/user/Backup/Documents --check
```

The default checks file sizes and modification times. Add `--verify` for a byte-for-byte comparison and `--all` to include matching entries. `--check` verifies contents and exits with status 1 if anything differs or cannot be verified. Redirect output to a file to save the report. See `man voltkit-folder-compare`.
