# App Permissions

When an app asks to take screenshots, use the camera or microphone, or see your location, the desktop remembers your answer. **App Permissions** lists those answers by app so you can change one or make the app ask again.

![App Permissions page listing apps with their camera and screenshot permissions](shot:permissions)

## What it shows

Voltkit reads the **permission store** of the desktop portal (xdg-desktop-portal). An app appears here **after it has asked** for something. It is a history of your answers, not a list of everything an app is technically able to do: apps outside a sandbox can have capabilities that never go through the portal.

Each application is a row you expand to see its remembered permissions. The kinds of permission include:

- Take screenshots
- Use the camera
- Use the microphone
- Play sound
- See your location (the answer can be **Allowed: exact location** or **Allowed: approximate location**)
- Run in the background
- Show notifications
- Keep the computer awake
- Control your computer remotely
- Record or share your screen
- Change the wallpaper
- Run with real-time priority
- Look at the windows on screen
- Use desktop services

## Use it

1. Open **App Permissions**. The list loads when the page opens; click **Refresh** to read it again.
2. Use **Find an app or permission…** to narrow the list. Search by an app name or a permission such as *camera*. *No apps match this search* means nothing fits.
3. Expand an app to see its answers.
4. **Change an answer** with the switch next to it: **Allowed** or **Denied**.
5. **Forget, so the app asks again** removes the saved answer. The next time the app needs that permission, the desktop asks you again, and a toast confirms: *{app} will ask again*.

Answers that are more than a plain yes or no (a location accuracy, a saved screen-sharing choice) are marked **Remembered**. They can only be forgotten, not switched.

Apps the desktop could not identify are grouped under **Apps without an ID**.

## Empty page

*No app has asked for anything yet* means the store is empty. Answers appear after an app asks, for example to take a screenshot.

## Notes

- Changes are made in the portal's permission store and apply the next time the app asks. A running app may keep what it already received until it restarts.
- This page does not read or change Flatpak overrides; it works with the answers the desktop portal stored.
- Nothing is sent anywhere. The page only talks to the portal on your computer.

## Troubleshooting

- **The list is empty or shows an error.** It needs the session's D-Bus and the permission store of the desktop portal. *Could not read permissions* shows the reason.
- **I changed an answer and the app still has access.** Restart the app.
