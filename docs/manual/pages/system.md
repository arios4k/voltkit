# Hosts & Environment

Two small editors you would otherwise use a terminal for: your computer's **hosts file** (which names map to which addresses) and the **environment variables** that apps in your desktop session receive.

![Hosts & Environment page with hosts entries and switches](shot:system)

## Hosts file

`/etc/hosts` maps names to IP addresses before DNS is asked. It is how you make `myapp.test` open your local development server, or block a site.

### Entries

Each entry shows its IP address, its host names and an optional note. A **switch** turns the entry on or off without deleting it (an off entry is kept as a commented line). Buttons let you **Edit** or **Delete** it.

### Add or edit an entry

Click **Add Entry** and fill in:

| Field | Rules |
|---|---|
| **IP address** | An IPv4 or IPv6 address such as `127.0.0.1` or `::1` |
| **Host names, separated by spaces** | At least one name, with letters, digits, dots and dashes, such as `myapp.test` |
| **Note (optional)** | A comment saved with the entry |

Voltkit explains what is wrong if a field is not valid.

### Save or discard

Changes are collected until you click **Save Changes** (or **Discard** to drop them). Saving writes `/etc/hosts` as the administrator through polkit, so you see **one password prompt**. The new file is written beside the old one, the old one is backed up as `/etc/hosts.voltkit-backup`, and only then is the new one moved into place, so a failure never leaves a half-written file. The toast tells you where the previous version is.

### System entries are protected

Entries the system relies on (`localhost`, the IPv6 basics and this computer's own name) are shown as *System entry, kept as it is* and cannot be changed. If you really need to, turn on **Allow editing system entries**. Changing `localhost` or your computer's name can break networking and `sudo`; a backup is kept either way.

Comments, blank lines and entries you did not touch are written back exactly as they were read.

## Environment variables

Environment variables such as `EDITOR` or an extra folder in `PATH` apply to every app in your desktop session. Voltkit keeps yours in `~/.config/environment.d/90-voltkit.conf`, a file systemd reads at login, so they reach every app started in the session.

### Your variables

- **Add Variable** creates one. **Name** uses letters, digits and underscores and does not start with a digit. **Value** must fit on one line.
- **Filter variables** searches the list.
- Edit or delete a variable from its row. Deleting shows an **Undo** toast.
- After saving, the toast reminds you: *Log out and back in to apply it*. Running apps keep the environment they started with.

### Already in your session

This list shows variables the session already has. You can **Override** one to give it a different value from now on. Variables that wire the running session together (bus addresses, display sockets, session IDs) are not offered, because overriding them breaks the session.

> [!TIP] Examples: `EDITOR=nano`, or add a folder to `PATH` for tools you install in your home directory.

## Troubleshooting

- **The password prompt does not appear.** Saving the hosts file needs `pkexec` (polkit). Install `polkitd`/`policykit-1` for your distribution.
- **A variable has no effect.** Log out and back in; apps read the environment when they start.
- **I broke my hosts file.** Restore the backup: `sudo cp /etc/hosts.voltkit-backup /etc/hosts`.
