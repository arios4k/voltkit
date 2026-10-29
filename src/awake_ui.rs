//! The Keep Awake page. The background service holds the request, so this page
//! only turns it on and off and shows its state.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::{gio, glib};

use crate::app::toast;
use crate::{awake, daemon};

/// Choices in the duration list, in minutes; 0 means until turned off.
const DURATIONS: &[(u32, &str)] =
    &[(30, "30 minutes"), (60, "1 hour"), (120, "2 hours"), (240, "4 hours"), (0, "Until turned off")];
const DEFAULT_DURATION: u32 = 1;
const REFRESH: Duration = Duration::from_secs(15);

pub fn page(toasts: &adw::ToastOverlay) -> gtk::Widget {
    let switch = adw::SwitchRow::builder().title("Keep Awake").subtitle("Off").build();
    let duration = adw::ComboRow::builder()
        .title("For")
        .model(&gtk::StringList::new(&DURATIONS.iter().map(|(_, label)| *label).collect::<Vec<_>>()))
        .selected(DEFAULT_DURATION)
        .build();
    let group = adw::PreferencesGroup::builder()
        .title("Keep Awake")
        .description(
            "Stops your computer from sleeping or locking the screen while you watch, present, read or \
             download. It keeps going after you close this window, and switches itself off when the time is up.",
        )
        .build();
    group.add(&switch);
    group.add(&duration);

    // Set while the switch is moved to match the service, so that isn't taken as a click.
    let syncing = Rc::new(Cell::new(false));

    let refresh = {
        let (switch, duration, syncing) = (switch.clone(), duration.clone(), syncing.clone());
        move || {
            let (switch, duration, syncing) = (switch.clone(), duration.clone(), syncing.clone());
            glib::spawn_future_local(async move {
                let status = gio::spawn_blocking(daemon::awake_status).await.ok().flatten();
                syncing.set(true);
                match status {
                    Some((on, until)) => {
                        switch.set_active(on);
                        switch.set_subtitle(&awake::describe(on, until));
                        switch.set_sensitive(true);
                    }
                    None => {
                        switch.set_active(false);
                        switch.set_subtitle("Needs the background service: start it with the banner above");
                        switch.set_sensitive(false);
                    }
                }
                duration.set_sensitive(!switch.is_active());
                syncing.set(false);
            });
        }
    };
    let refresh = Rc::new(refresh);
    refresh();
    {
        // Keep the "until" time and timer expiry current while the app is open.
        let (refresh, switch) = (refresh.clone(), switch.downgrade());
        glib::timeout_add_local(REFRESH, move || {
            if switch.upgrade().is_none() {
                return glib::ControlFlow::Break;
            }
            refresh();
            glib::ControlFlow::Continue
        });
    }

    {
        let (duration, toasts, refresh, syncing) = (duration.clone(), toasts.clone(), refresh.clone(), syncing.clone());
        switch.connect_active_notify(move |switch| {
            if syncing.get() {
                return;
            }
            let on = switch.is_active();
            let minutes = DURATIONS.get(duration.selected() as usize).map_or(60, |(m, _)| *m);
            let (toasts, refresh) = (toasts.clone(), refresh.clone());
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || {
                    if on { daemon::keep_awake(minutes) } else { daemon::stop_awake() }
                })
                .await;
                if let Ok(Err(e)) = result {
                    toast(&toasts, format!("Keep Awake: {e:#}"));
                }
                refresh();
            });
        });
    }

    let page = adw::PreferencesPage::new();
    page.add(&group);
    page.upcast()
}
