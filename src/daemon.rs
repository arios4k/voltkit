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
use crate::awake::KeepAwake;
use crate::expander;
use crate::config::Config;
use crate::portal::{self, APP_ID};
use crate::shortcuts;
use crate::i18n::tr;

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
    /// Typed triggers: "off", "on", "needs keyboard access" or "unavailable".
    expander: Arc<std::sync::Mutex<String>>,
    expander_task: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    keep_awake: Arc<KeepAwake>,
    /// Snippets for typed triggers; `Reload` replaces them after edits.
    snippets: Arc<std::sync::RwLock<Vec<crate::config::Snippet>>>,
}

impl State {
    fn spawn(&self, action: Action) {
        if self.busy.swap(true, Ordering::SeqCst) {
            println!("{}", trf!("{}: skipped, another action is still open", action.id()));
            return;
        }
        let state = self.clone();
        tokio::spawn(async move {
            // Released on drop, so a failing or panicking action can't block later hotkeys.
            let _busy = BusyGuard(state.busy.clone());
            println!("{}", trf!("{}: started", action.id()));
            actions::run(action, &state.config).await;
            println!("{}", trf!("{}: finished", action.id()));
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
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs(trf!("unknown action {}", format!("{:?}", action))))?;
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

    /// Keeps the computer from sleeping or locking for `minutes` (0: until stopped).
    async fn keep_awake(&self, minutes: u32) -> zbus::fdo::Result<()> {
        self.0.keep_awake.start(minutes).await.map_err(|e| zbus::fdo::Error::Failed(format!("{e:#}")))
    }

    fn stop_awake(&self) {
        self.0.keep_awake.stop();
    }

    /// (on, end as Unix seconds or 0 for "until turned off").
    fn awake_status(&self) -> (bool, i64) {
        self.0.keep_awake.status()
    }

    /// Re-reads the config file: snippet edits apply right away, and typed
    /// triggers start or stop when they're turned on or off.
    fn reload(&self) -> zbus::fdo::Result<()> {
        let config = Config::load().map_err(|e| zbus::fdo::Error::Failed(format!("{e:#}")))?;
        *self.0.snippets.write().unwrap() = config.snippets.clone();
        sync_expander(&self.0, Arc::new(config));
        Ok(())
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
        snippets: Arc::new(std::sync::RwLock::new(config.snippets.clone())),
        keep_awake: Arc::default(),
        config: Arc::new(config),
        busy: Arc::default(),
        quit: Arc::default(),
        backend: Arc::new(std::sync::Mutex::new("none".into())),
        expander: Arc::new(std::sync::Mutex::new("off".into())),
        expander_task: Arc::default(),
    };

    let snap_file = snap_address_file();
    if snap_file.is_some() && call::<(String, String, String)>("Status", &()).await.is_ok() {
        bail!("{}", tr("the Voltkit daemon is already running"));
    }
    let conn = zbus::connection::Builder::session()?
        .serve_at(OBJECT_PATH, Service(state.clone()))?
        .build()
        .await
        .context(tr("cannot connect to the session bus"))?;
    if let Some(file) = &snap_file {
        // The snap may not own a well-known name without a reviewed D-Bus
        // slot, so it tells the snap's other processes its unique name.
        let unique = conn.unique_name().context(tr("cannot connect to the session bus"))?.to_string();
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(file, unique)?;
    } else {
        // Without DoNotQueue a second instance would silently wait in line for the name.
        let reply = conn.request_name_with_flags(BUS_NAME, RequestNameFlags::DoNotQueue.into()).await;
        if !matches!(reply, Ok(RequestNameReply::PrimaryOwner)) {
            bail!("{}", tr("the Voltkit daemon is already running"));
        }
    }
    println!("{}", trf!("Voltkit daemon {} started", env!("CARGO_PKG_VERSION")));

    // Best effort: lets xdg-desktop-portal ≥ 1.19 attribute us to our .desktop file.
    if let Ok(id) = APP_ID.parse() {
        let _ = ashpd::register_host_app(id).await;
    }

    let portal_task = {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = portal_shortcuts(&state).await {
                // First run on GNOME without the portal: add the shortcuts.
                if shortcuts::gnome_first_run() {
                    match shortcuts::gnome_install(&state.config, false) {
                        Ok(()) => println!("{}", tr("Added Voltkit's keyboard shortcuts to GNOME")),
                        Err(e) => eprintln!("{}", trf!("Could not add the keyboard shortcuts: {}", format!("{:#}", e))),
                    }
                }
                let fallback = if shortcuts::gnome_installed() { "gnome-keybindings" } else { "none" };
                *state.backend.lock().unwrap() = fallback.into();
                eprintln!("{}", trf!("Global shortcuts portal unavailable: {}", format!("{:#}", e)));
                if fallback == "none" {
                    eprintln!("{}", tr("Bind `voltkit trigger ocr` / `voltkit trigger color` in your desktop's \
                         keyboard settings, or on GNOME run `voltkit shortcuts install`."));
                } else {
                    println!("{}", tr("Using GNOME custom keybindings"));
                    // After an update that adds tools, bind their shortcuts too.
                    if shortcuts::gnome_incomplete() {
                        match shortcuts::gnome_install(&state.config, false) {
                            Ok(()) => println!("{}", tr("Added the shortcuts of new tools")),
                            Err(e) => eprintln!("{}", trf!("Could not add the new shortcuts: {}", format!("{:#}", e))),
                        }
                    }
                }
            }
        })
    };

    // A daily snapshot of the hardware, drivers and firmware, so System
    // Report can show what changed. Its own thread: collecting it blocks.
    std::thread::spawn(|| {
        loop {
            if let Err(e) = crate::modules::sysreport::snapshot_if_due() {
                eprintln!("{}", trf!("system snapshot: {}", format!("{:#}", e)));
            }
            std::thread::sleep(std::time::Duration::from_secs(3 * 3600));
        }
    });

    sync_expander(&state, state.config.clone());

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = state.quit.notified() => {}
    }
    portal_task.abort();
    if let Some(task) = state.expander_task.lock().unwrap().take() {
        task.abort();
    }
    println!("{}", tr("Voltkit daemon stopped"));
    Ok(())
}

/// Starts typed triggers if they're on and not running, or stops them if off.
fn sync_expander(state: &State, config: Arc<Config>) {
    let mut task = state.expander_task.lock().unwrap();
    if config.expander.typed_triggers {
        if task.as_ref().is_none_or(|t| t.is_finished()) {
            *task = Some(tokio::spawn(run_expander(state.clone(), config)));
        }
    } else {
        if let Some(t) = task.take() {
            t.abort();
            println!("{}", tr("Typed triggers off"));
        }
        *state.expander.lock().unwrap() = "off".into();
    }
}

/// Runs typed triggers, waiting for keyboard access when it's missing: once
/// it's granted (in the app, with the password) they start by themselves.
async fn run_expander(state: State, config: Arc<Config>) {
    let mut said_waiting = false;
    loop {
        let missing = tokio::task::spawn_blocking(expander::missing_permissions).await.unwrap_or_default();
        if !missing.is_empty() {
            *state.expander.lock().unwrap() = "needs keyboard access".into();
            if !said_waiting {
                println!("{}", trf!("Typed triggers are waiting for keyboard access: {}", missing.join("; ")));
                said_waiting = true;
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            continue;
        }
        *state.expander.lock().unwrap() = "on".into();
        if let Err(e) = expander::run(config.clone(), state.snippets.clone()).await {
            *state.expander.lock().unwrap() = "unavailable".into();
            eprintln!("{}", trf!("Typed triggers unavailable: {}", format!("{:#}", e)));
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    }
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
        println!("{}", trf!("Shortcut {} {}", format!("{:<6}", s.id()), s.trigger_description()));
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

/// In the snap, the file holding the running daemon's unique bus name, in
/// the snap's private runtime folder. Outside a snap the daemon owns
/// `BUS_NAME` instead, so there is no file.
fn snap_address_file() -> Option<PathBuf> {
    crate::sandbox::snap().then(|| {
        std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("voltkit-daemon-address")
    })
}

/// The bus name to reach the daemon at; empty when the snap's daemon has
/// never started.
pub fn destination() -> String {
    match snap_address_file() {
        Some(file) => std::fs::read_to_string(file).map(|s| s.trim().to_owned()).unwrap_or_default(),
        None => BUS_NAME.to_owned(),
    }
}

async fn call<R>(method: &str, body: &(impl serde::Serialize + zbus::zvariant::DynamicType)) -> Result<R>
where
    R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
{
    let destination = destination();
    if destination.is_empty() {
        bail!("{}", tr(NOT_RUNNING));
    }
    let conn = zbus::Connection::session().await?;
    let reply = conn.call_method(Some(destination.as_str()), OBJECT_PATH, Some(INTERFACE), method, body).await?;
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

/// The typed-trigger state from [`status`] in words for the user.
pub fn describe_typed(state: &str) -> &str {
    match state {
        "off" => tr("off"),
        "on" => tr("on"),
        "needs keyboard access" => tr("needs keyboard access"),
        "unavailable" => tr("unavailable"),
        other => other,
    }
}

/// Returns (version, shortcut backend, typed-trigger state) of the running daemon, if any.
pub fn status() -> Option<(String, String, String)> {
    portal::block_on(call::<(String, String, String)>("Status", &())).ok()
}


const NOT_RUNNING: &str = N_!("the background service isn't running; start it with `voltkit daemon`");

/// Keeps the computer awake through the daemon for `minutes` (0: until stopped).
pub fn keep_awake(minutes: u32) -> Result<()> {
    if status().is_none() {
        bail!("{}", tr(NOT_RUNNING));
    }
    portal::block_on(call::<()>("KeepAwake", &(minutes,)))
}

pub fn stop_awake() -> Result<()> {
    portal::block_on(call::<()>("StopAwake", &())).context(tr(NOT_RUNNING))
}

/// (on, end as Unix seconds or 0), or None if the daemon isn't running.
pub fn awake_status() -> Option<(bool, i64)> {
    portal::block_on(call::<(bool, i64)>("AwakeStatus", &())).ok()
}

/// Asks a running daemon to re-read the snippets. Fails if it isn't running.
pub fn reload() -> Result<()> {
    portal::block_on(call::<()>("Reload", &())).context(tr("the daemon is not running"))
}

pub fn stop() -> Result<()> {
    portal::block_on(call::<()>("Quit", &())).context(tr("the daemon is not running"))
}

// ------------------------------------------------------------ autostart

fn autostart_path() -> Result<PathBuf> {
    Ok(dirs::config_dir().context(tr("no config directory"))?.join("autostart").join(format!("{BUS_NAME}.desktop")))
}

fn autostart_entry(exe: &str) -> String {
    let exe = if exe.contains(' ') { format!("\"{exe}\"") } else { exe.to_owned() };
    format!(
        "[Desktop Entry]\nType=Application\nName=Voltkit Background Service\n\
         Comment=Global shortcuts for Voltkit\nExec={exe} daemon\nIcon={APP_ID}\n\
         NoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
    )
}

/// The login entry installed by the packages, which starts the service for
/// every user. A user entry with the same name overrides it.
const SYSTEM_AUTOSTART: &str = "/etc/xdg/autostart/io.github.arios4k.Voltkit.Daemon.desktop";
/// The binary the system entry runs.
const SYSTEM_EXE: &str = "/usr/bin/voltkit";

/// In the snap: turns on starting at login the first time, unless the user
/// already chose (an entry exists, on or off).
pub fn ensure_snap_autostart() -> Result<()> {
    if !autostart_path()?.exists() {
        set_autostart(true)?;
    }
    Ok(())
}

pub fn set_autostart(enabled: bool) -> Result<PathBuf> {
    let path = autostart_path()?;
    let system = std::path::Path::new(SYSTEM_AUTOSTART).exists();
    // In the snap, snapd starts the entry found in the snap's own config folder.
    let exe = crate::sandbox::host_exe()?;
    if enabled && system && exe == std::path::Path::new(SYSTEM_EXE) {
        // The package's entry already starts it: drop any override.
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(PathBuf::from(SYSTEM_AUTOSTART));
    }
    if enabled {
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, autostart_entry(&exe.to_string_lossy()))?;
    } else if system || crate::sandbox::snap() {
        // Turn the package's entry off for this user only. In the snap a
        // hidden entry also records the choice, so it isn't turned back on.
        let exe = if system { PathBuf::from(SYSTEM_EXE) } else { exe };
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, format!("{}Hidden=true\n", autostart_entry(&exe.to_string_lossy())))?;
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
