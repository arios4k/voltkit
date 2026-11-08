# What's Using This?

"Target is busy" when you eject a USB stick, "address already in use" when you start a development server, a document that will not delete: **What's Using This?** shows which apps keep the file, folder, drive or network port busy, and lets you close them.

![What's Using This? page listing the apps that keep a USB drive busy](shot:using)

## Choose what to inspect

| Choice | How |
|---|---|
| **Drives** | USB sticks, memory cards and other disks you can eject are listed. Select one. If no removable drive is connected the page says *No drives to eject are connected*. |
| **File…** or **Folder…** | Choose a file or a folder (for example one reported as busy). You can also **drop it on the window**. |
| **Network port** | Type a port number from 1 to 65535, such as `3000`, and press <kbd>Enter</kbd> or click the checkmark. |

Example: if your development server says *address already in use* on `localhost:3000`, enter `3000`.

## Results

The result is titled *Using "{target}"* (or *Port 3000*). Each app that uses it is a row with its name, its process number (**PID**), and how it uses it (for example that it has a file open or listens on the port). If an app has several processes, the row counts them (*and N more*).

- If nothing is using it: **Nothing is using it**, and for a drive *You can eject it now*.
- Click **Check Again** after you close something.
- Review the results before closing anything, and save your work first.

## Close an app

- **Close** asks the app to quit nicely, like closing its window. *Unsaved work in it may be lost.* The row shows *Closing…* and then *{app} closed*.
- If an app will not quit, **Force Quit** stops it immediately, **without a chance to save**. Voltkit asks you to confirm first.
- Parts of the system itself are **never offered for closing**: the desktop shell and window manager, the display server, your login session, the service manager, the message bus, the login screen, remote login, the permission service, the drive manager, the file system service, desktop portals, the sound and video services, the network service, and Voltkit itself. Their row says *This is {what}, so Voltkit won't close it*.

Voltkit closes a process only if it is still the same process it listed (it checks the process start time), so a reused process number is never closed by mistake.

## Eject a drive

For a drive that will not eject, select it under **Drives** and click **Eject**. If apps are using it, Voltkit lists them: *Close {N} and Eject?* **Close and Eject** asks them to quit and then ejects. If some do not close, use **Force Quit** on them and eject again. When it works you see *{drive} ejected: you can unplug it*.

## Servers and other users' programs

As a normal user Voltkit can only inspect **your own** programs. Servers and system services often run as another user. Click **Check with Password** to scan as the administrator (the desktop asks for your password through polkit); the results then include every user's programs. Closing another user's program also asks for the password.

## From the terminal

```bash
voltkit using /media/$USER/USB    # what keeps this drive busy?
voltkit using ~/Documents/report.odt
voltkit using 3000                # what uses port 3000?
sudo voltkit using 3000           # include other users' programs and system services
```

The command prints each program with its PID and how it uses the target.

## Troubleshooting

- **A program is missing from the list.** It may belong to another user: use **Check with Password**, or run the command with `sudo`.
- **A drive still will not eject.** The system may be using it. Close file managers and terminals open on it, then **Check Again**.
