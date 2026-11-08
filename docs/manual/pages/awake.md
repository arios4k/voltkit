# Keep Awake

Watching a film, presenting, or waiting for a long download? Keep Awake stops your computer from going to sleep or locking its screen for a while.

![Keep Awake with its current status and duration controls](shot:awake)

## Use it

1. Open **Keep Awake**.
2. Choose how long in the **For** list:

   | Choice | Length |
   |---|---|
   | 30 minutes | 30 minutes |
   | 1 hour | 60 minutes |
   | 2 hours | 120 minutes |
   | 4 hours | 240 minutes |
   | Until turned off | no time limit |

3. Turn the switch on. The status changes to **Keep Awake is on**, and when a timer is running it shows the time left.
4. Turn the switch off whenever you are done: the status says **Keep Awake is off**.

Choose a duration *before* turning it on. Turning it on again with a different duration replaces the earlier request.

## It keeps going after you close the window

Keep Awake is held by Voltkit's background service through the desktop's **Inhibit portal**, not by the window. You can close Voltkit and it continues, and it ends by itself when the timer runs out. If the background service is not running, the page tells you: *Needs the background service: start it with the banner above*. Click **Start** on the banner at the top of the window.

## From the terminal

```bash
voltkit awake            # show whether Keep Awake is on
voltkit awake 30m        # 30 minutes
voltkit awake 1h30m      # 1 hour 30 minutes
voltkit awake 90         # a plain number is minutes
voltkit awake on         # until you turn it off
voltkit awake off        # stop
```

## What it does and does not do

- It asks the desktop not to **suspend** and not to **lock the screen** because of inactivity.
- It does not change your power settings and does not stop you from sleeping the computer yourself (closing the lid or choosing Suspend still works as the desktop defines).
- To find out what else is keeping the computer awake, or why it woke up, see [Why Didn't It Sleep?](sleep).
