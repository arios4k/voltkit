//! XDG desktop portal and freedesktop D-Bus helpers.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use ashpd::desktop::screenshot::Screenshot;
use ashpd::desktop::{Color, ResponseError};
use zbus::zvariant::Value;

use crate::modules::color::Rgb;

pub const APP_ID: &str = "io.github.arios4k.Voltkit";

/// Runs a future to completion from synchronous code (CLI, GUI worker threads).
/// Must not be called from inside the daemon's runtime.
///
/// All calls share one runtime that lives as long as the process. ashpd caches
/// its D-Bus connection, and the connection's tasks run on the runtime that
/// created it; with a runtime per call, the GUI's second portal request (a
/// second color pick or OCR capture) hung forever on the dead connection.
pub fn block_on<F: Future>(future: F) -> F::Output {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("failed to start async runtime")
        })
        .block_on(future)
}

/// Error raised when the user dismisses a portal dialog.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

fn map_portal_err(e: ashpd::Error) -> anyhow::Error {
    match e {
        ashpd::Error::Response(ResponseError::Cancelled) => Cancelled.into(),
        other => anyhow::Error::new(other),
    }
}

/// Opens the desktop's interactive screenshot UI and returns the saved image.
pub async fn screenshot_interactive() -> Result<PathBuf> {
    let response = Screenshot::request()
        .interactive(true)
        .modal(true)
        .send()
        .await
        .map_err(map_portal_err)?
        .response()
        .map_err(map_portal_err)?;
    file_uri_to_path(response.uri().as_str())
}

/// Lets the user click anywhere on screen and returns that pixel's color.
pub async fn pick_color() -> Result<Rgb> {
    let c = Color::pick()
        .send()
        .await
        .map_err(map_portal_err)?
        .response()
        .map_err(map_portal_err)?;
    let to_u8 = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Ok(Rgb { r: to_u8(c.red()), g: to_u8(c.green()), b: to_u8(c.blue()) })
}

/// Shows a desktop notification through `org.freedesktop.Notifications`,
/// which, unlike the notification portal, works for unsandboxed apps everywhere.
pub async fn notify(summary: &str, body: &str) -> Result<()> {
    let conn = zbus::Connection::session().await?;
    let hints: HashMap<&str, Value> = HashMap::from([("desktop-entry", Value::from(APP_ID))]);
    conn.call_method(
        Some("org.freedesktop.Notifications"),
        "/org/freedesktop/Notifications",
        Some("org.freedesktop.Notifications"),
        "Notify",
        &("Voltkit", 0u32, APP_ID, summary, escape_markup(body), Vec::<&str>::new(), hints, 4000i32),
    )
    .await
    .context("failed to show notification")?;
    Ok(())
}

fn escape_markup(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn file_uri_to_path(uri: &str) -> Result<PathBuf> {
    let Some(rest) = uri.strip_prefix("file://") else {
        bail!("portal returned a non-file URI: {uri}");
    };
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok());
        if bytes[i] == b'%'
            && let Some(b) = hex.and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    use std::os::unix::ffi::OsStringExt;
    Ok(PathBuf::from(std::ffi::OsString::from_vec(out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_file_uris() {
        assert_eq!(
            file_uri_to_path("file:///home/a/Screenshot%20from%202026.png").unwrap(),
            PathBuf::from("/home/a/Screenshot from 2026.png")
        );
        assert_eq!(file_uri_to_path("file:///tmp/100%").unwrap(), PathBuf::from("/tmp/100%"));
        assert!(file_uri_to_path("https://example.com/x.png").is_err());
    }

    #[test]
    fn escapes_notification_markup() {
        assert_eq!(escape_markup("a<b> & c"), "a&lt;b&gt; &amp; c");
    }
}

#[cfg(test)]
mod session_bus_tests {
    #[test]
    #[ignore = "needs a desktop session bus"]
    fn portal_calls_survive_repeated_block_on() {
        for i in 0..3 {
            let r = std::thread::spawn(|| {
                super::block_on(async {
                    tokio::time::timeout(std::time::Duration::from_secs(3), async {
                        ashpd::desktop::remote_desktop::RemoteDesktop::new().await?.available_device_types().await
                    })
                    .await
                })
            })
            .join()
            .unwrap();
            assert!(r.is_ok(), "portal call {} hung", i + 1);
        }
    }
}
