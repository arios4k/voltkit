//! Keep Awake: stops the computer from sleeping or locking its screen.
//!
//! The daemon holds the request through the Inhibit portal, so it lasts after
//! the control center closes. The inhibition ends when the portal request is
//! closed, by the timer or by turning it off.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, bail};
use ashpd::desktop::inhibit::{InhibitFlags, InhibitOptions, InhibitProxy};
use chrono::{DateTime, Local, TimeZone};
use tokio::sync::Notify;

use crate::portal;

struct Session {
    id: u64,
    until: Option<DateTime<Local>>,
    stop: Arc<Notify>,
}

#[derive(Default)]
pub struct KeepAwake {
    session: Mutex<Option<Session>>,
    next_id: AtomicU64,
}

impl KeepAwake {
    /// Keeps the computer awake for `minutes`, or until turned off when 0.
    /// Replaces any earlier request.
    pub async fn start(self: &Arc<Self>, minutes: u32) -> Result<()> {
        self.stop();
        let request = InhibitProxy::new()
            .await?
            .inhibit(
                None,
                InhibitFlags::Idle | InhibitFlags::Suspend,
                InhibitOptions::default().set_reason("Keep Awake is on in Voltkit"),
            )
            .await?;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let stop = Arc::new(Notify::new());
        let until = (minutes > 0).then(|| Local::now() + chrono::Duration::minutes(minutes as i64));
        *self.session.lock().unwrap() = Some(Session { id, until, stop: stop.clone() });

        let this = self.clone();
        tokio::spawn(async move {
            let timed_out = if minutes == 0 {
                stop.notified().await;
                false
            } else {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(minutes as u64 * 60)) => true,
                    _ = stop.notified() => false,
                }
            };
            let _ = request.close().await;
            if timed_out {
                {
                    let mut session = this.session.lock().unwrap();
                    if session.as_ref().is_some_and(|s| s.id == id) {
                        *session = None;
                    }
                }
                let _ = portal::notify("Keep Awake ended", "Your computer can sleep again.").await;
            }
        });
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(session) = self.session.lock().unwrap().take() {
            // A permit is stored even if the task isn't waiting yet.
            session.stop.notify_one();
        }
    }

    /// (on, end time as Unix seconds, or 0 for "until turned off").
    pub fn status(&self) -> (bool, i64) {
        match self.session.lock().unwrap().as_ref() {
            Some(s) => (true, s.until.map_or(0, |t| t.timestamp())),
            None => (false, 0),
        }
    }
}

/// Parses `30m`, `1h`, `1h30m`, `90` (minutes), `on` (until turned off, 0) or
/// `off` (None).
pub fn parse_duration(text: &str) -> Result<Option<u32>> {
    let t = text.trim().to_ascii_lowercase();
    match t.as_str() {
        "off" | "stop" => return Ok(None),
        "on" | "forever" | "indefinitely" => return Ok(Some(0)),
        _ => {}
    }
    if let Ok(minutes) = t.parse::<u32>() {
        return positive(minutes, text);
    }
    let mut minutes = 0u32;
    let mut number = String::new();
    for c in t.chars() {
        match c {
            '0'..='9' => number.push(c),
            'h' | 'm' if !number.is_empty() => {
                let n: u32 = number.parse()?;
                minutes += if c == 'h' { n * 60 } else { n };
                number.clear();
            }
            _ => bail!("can't read the duration {text:?}; try 30m, 1h, 1h30m, on or off"),
        }
    }
    if !number.is_empty() {
        bail!("can't read the duration {text:?}; add m or h, like 30m or 2h");
    }
    positive(minutes, text)
}

fn positive(minutes: u32, text: &str) -> Result<Option<u32>> {
    if minutes == 0 {
        bail!("the duration {text:?} is zero; use off to turn Keep Awake off");
    }
    Ok(Some(minutes))
}

/// "Off", "On until turned off" or "On until 15:30".
pub fn describe(on: bool, until: i64) -> String {
    match (on, Local.timestamp_opt(until, 0).single()) {
        (false, _) => "Off".into(),
        (true, _) if until == 0 => "On until turned off".into(),
        (true, Some(t)) => format!("On until {}", t.format("%H:%M")),
        (true, None) => "On".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("30m").unwrap(), Some(30));
        assert_eq!(parse_duration("1h").unwrap(), Some(60));
        assert_eq!(parse_duration("1h30m").unwrap(), Some(90));
        assert_eq!(parse_duration("90").unwrap(), Some(90));
        assert_eq!(parse_duration(" ON ").unwrap(), Some(0));
        assert_eq!(parse_duration("off").unwrap(), None);
        assert!(parse_duration("0").is_err());
        assert!(parse_duration("2").is_ok());
        assert!(parse_duration("1h30").is_err());
        assert!(parse_duration("soon").is_err());
    }

    #[test]
    fn describes_status() {
        assert_eq!(describe(false, 0), "Off");
        assert_eq!(describe(true, 0), "On until turned off");
        let at = Local.with_ymd_and_hms(2026, 10, 29, 15, 30, 0).unwrap().timestamp();
        assert_eq!(describe(true, at), "On until 15:30");
    }
}
