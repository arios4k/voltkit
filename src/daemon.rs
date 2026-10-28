//! `voltkit daemon`: the background service.
//!
//! It owns `io.github.arios4k.Voltkit.Daemon` on the session bus, so only one
//! instance runs. It binds global shortcuts through the portal when the desktop
//! offers one, and runs hotkey actions for `voltkit trigger`, keybindings and the GUI.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;
use tokio::sync::Notify;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

use crate::actions::{self, Action};
use crate::expander;
use crate::config::Config;
use crate::portal::{self, APP_ID};
use crate::shortcuts;

pub const BUS_NAME: &str = "io.github.arios4k.Voltkit.Daemon";
const OBJECT_PATH: &str = "/io/github/arios4k/Voltkit/Daemon";
const INTERFACE: &str = "io.github.arios4k.Voltkit.Daemon1";

#[derive(Clone)]
struct State {
    config: Arc<Config>,
    /// Only one capture UI at a time; repeated key presses are ignored.
    busy: Arc<AtomicBool>,
    quit: Arc<Notify>,
    backend: Arc<std::sync::Mutex<String>>,
    expander: Arc<std::sync::Mutex<String>>,
}

impl State {
    fn spawn(&self, action: Action) {
        if self.busy.swap(true, Ordering::SeqCst) {
            println!("{}: skipped, another action is still open", action.id());
            return;
        }
        let state = self.clone();
        tokio::spawn(async move {
            // Released on drop, so a failing or panicking action can't block later hotkeys.
            let _busy = BusyGuard(state.busy.clone());
            println!("{}: started", action.id());
            actions::run(action, &state.config).await;
            println!("{}: finished", action.id());
        });
    }
}

struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

struct Service(State);

#[zbus::interface(name = "io.github.arios4k.Voltkit.Daemon1")]
impl Service {
    /// Starts an action ("ocr" or "color") and returns immediately.
    fn trigger(&self, action: &str) -> zbus::fdo::Result<()> {
        let action = Action::from_id(action)
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs(format!("unknown action {action:?}")))?;
        self.0.spawn(action);
        Ok(())
    }

    /// Returns (version, shortcut backend, typed-trigger state).
    fn status(&self) -> (String, String, String) {
        (
            env!("CARGO_PKG_VERSION").to_owned(),
            self.0.backend.lock().unwrap().clone(),
            self.0.expander.lock().unwrap().clone(),
        )
    }

    fn quit(&self) {
        self.0.quit.notify_one();
    }
}

pub fn run(config: Config) -> Result<()> {
    // Several threads, so one slow action (a stuck helper program, a portal
    // dialog) can never stop the daemon from answering D-Bus.
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(serve(config))
}

async fn serve(config: Config) -> Result<()> {
    let state = State {
        config: Arc::new(config),
        busy: Arc::default(),
        quit: Arc::default(),
        backend: Arc::new(std::sync::Mutex::new("none".into())),
        expander: Arc::new(std::sync::Mutex::new("off".into())),
    };

    let conn = zbus::connection::Builder::session()?
        .serve_at(OBJECT_PATH, Service(state.clone()))?
        .build()
        .await
        .context("cannot connect to the session bus")?;
    // Without DoNotQueue a second instance would silently wait in line for the name.
    let reply = conn.request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into()).await;
    if !matches!(reply, Ok(RequestNameReply::PrimaryOwner)) {
        bail!("the Voltkit daemon is already running");
    }
    println!("Voltkit daemon {} started", env!("CARGO_PKG_VERSION"));

    // Best effort: lets xdg-desktop-portal ≥ 1.19 attribute us to our .desktop file.
    if let Ok(id) = APP_ID.parse() {
        let _ = ashpd::register_host_app(id).await;
    }

    let portal_task = {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = portal_shortcuts(&state).await {
                let fallback = if shortcuts::gnome_installed() { "gnome-keybindings" } else { "none" };
                *state.backend.lock().unwrap() = fallback.into();
                eprintln!("Global shortcuts portal unavailable: {e:#}");
                if fallback == "none" {
                    eprintln!(
                        "Bind `voltkit trigger ocr` / `voltkit trigger color` in your desktop's \
                         keyboard settings, or on GNOME run `voltkit shortcuts install`."
                    );
                } else {
                    println!("Using GNOME custom keybindings");
                }
            }
        })
    };

    let expander_task = state.config.expander.typed_triggers.then(|| {
        let state = state.clone();
        tokio::spawn(async move {
            *state.expander.lock().unwrap() = "on".into();
            if let Err(e) = expander::run(state.config.clone()).await {
                *state.expander.lock().unwrap() = "unavailable".into();
                eprintln!("Typed triggers unavailable: {e:#}");
                let _ = portal::notify("Typed triggers unavailable", &format!("{e:#}")).await;
            }
        })
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = state.quit.notified() => {}
    }
    portal_task.abort();
    if let Some(task) = expander_task {
        task.abort();
    }
    println!("Voltkit daemon stopped");
    Ok(())
}

/// Binds the configured hotkeys via the GlobalShortcuts portal and dispatches
/// activations until the daemon stops.
async fn portal_shortcuts(state: &State) -> Result<()> {
    let portal = GlobalShortcuts::new().await?;
    let session = portal.create_session(Default::default()).await?;

    let triggers: Vec<(Action, String)> = Action::ALL
        .into_iter()
        .map(|a| Ok((a, shortcuts::accel_to_portal(a.accelerator(&state.config))?)))
        .collect::<Result<_>>()?;
    let requested: Vec<NewShortcut> = triggers
        .iter()
        .map(|(a, t)| NewShortcut::new(a.id(), a.description()).preferred_trigger(t.as_str()))
        .collect();
    let bound = portal.bind_shortcuts(&session, &requested, None, Default::default()).await?.response()?;

    *state.backend.lock().unwrap() = "portal".into();
    for s in bound.shortcuts() {
        println!("Shortcut {:<6} {}", s.id(), s.trigger_description());
    }

    let mut activations = portal.receive_activated().await?;
    while let Some(event) = activations.next().await {
        if let Some(action) = Action::from_id(event.shortcut_id()) {
            state.spawn(action);
        }
    }
    Ok(())
}

// ------------------------------------------------------------ client side

async fn call<R>(method: &str, body: &(impl serde::Serialize + zbus::zvariant::DynamicType)) -> Result<R>
where
    R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
{
    let conn = zbus::Connection::session().await?;
    let reply = conn.call_method(Some(BUS_NAME), OBJECT_PATH, Some(INTERFACE), method, body).await?;
    Ok(reply.body().deserialize()?)
}

/// Asks a running daemon to perform `action`, or performs it in this process.
pub fn trigger(action: Action, config: &Config) -> Result<()> {
    portal::block_on(async {
        if call::<()>("Trigger", &(action.id(),)).await.is_err() {
            actions::run(action, config).await;
        }
        Ok(())
    })
}

/// Returns (version, shortcut backend, typed-trigger state) of the running daemon, if any.
pub fn status() -> Option<(String, String, String)> {
    portal::block_on(call::<(String, String, String)>("Status", &())).ok()
}

/// Restarts the daemon so it picks up a new config. Does nothing if it isn't running.
pub fn restart_if_running() -> Result<bool> {
    if status().is_none() {
        return Ok(false);
    }
    stop()?;
    // Wait for the old instance to release the bus name.
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if status().is_none() {
            break;
        }
    }
    use std::os::unix::process::CommandExt;
    std::process::Command::new(std::env::current_exe()?)
        .arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()?;
    Ok(true)
}

pub fn stop() -> Result<()> {
    portal::block_on(call::<()>("Quit", &())).context("the daemon is not running")
}

// ------------------------------------------------------------ autostart

fn autostart_path() -> Result<PathBuf> {
    Ok(dirs::config_dir().context("no config directory")?.join("autostart").join(format!("{BUS_NAME}.desktop")))
}

fn autostart_entry(exe: &str) -> String {
    let exe = if exe.contains(' ') { format!("\"{exe}\"") } else { exe.to_owned() };
    format!(
        "[Desktop Entry]\nType=Application\nName=Voltkit Background Service\n\
         Comment=Global shortcuts for Voltkit\nExec={exe} daemon\nIcon={APP_ID}\n\
         NoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
    )
}

pub fn set_autostart(enabled: bool) -> Result<PathBuf> {
    let path = autostart_path()?;
    if enabled {
        let exe = std::env::current_exe()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, autostart_entry(&exe.to_string_lossy()))?;
    } else if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_entry_runs_daemon() {
        let entry = autostart_entry("/usr/bin/voltkit");
        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("\nExec=/usr/bin/voltkit daemon\n"));
        assert!(autostart_entry("/opt/a b/voltkit").contains("Exec=\"/opt/a b/voltkit\" daemon"));
    }
}
