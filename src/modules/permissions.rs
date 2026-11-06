//! App Permissions: the answers the desktop remembers when apps ask to take
//! screenshots, use the camera, see your location and so on.
//!
//! They live in xdg-desktop-portal's permission store, one table per kind of
//! request. Voltkit lets users review and change those stored answers.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use crate::i18n::tr;

const BUS_NAME: &str = "org.freedesktop.impl.portal.PermissionStore";
const OBJECT_PATH: &str = "/org/freedesktop/impl/portal/PermissionStore";
const INTERFACE: &str = "org.freedesktop.impl.portal.PermissionStore";

/// Tables that aren't permissions: file sharing grants and app chooser history.
const NOT_PERMISSIONS: &[&str] = &["documents", "desktop-used-apps"];

/// One remembered answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub table: String,
    pub id: String,
    /// The app's ID; empty for apps the desktop couldn't identify.
    pub app: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Allowed,
    Denied,
    /// Something other than a plain yes or no (a location accuracy, a saved
    /// screen-sharing choice): it can only be forgotten.
    Other,
}

impl Permission {
    pub fn state(&self) -> State {
        match self.values.first().map(String::as_str) {
            Some("yes") => State::Allowed,
            Some("no") => State::Denied,
            Some("NONE") if self.table == "location" => State::Denied,
            _ => State::Other,
        }
    }

    /// What the app asked to do, in plain words.
    pub fn title(&self) -> String {
        let known = match (self.table.as_str(), self.id.as_str()) {
            ("screenshot", _) => tr("Take screenshots"),
            ("devices", "camera") => tr("Use the camera"),
            ("devices", "microphone") => tr("Use the microphone"),
            ("devices", "speakers") => tr("Play sound"),
            ("location", _) => tr("See your location"),
            ("background", _) => tr("Run in the background"),
            ("notifications", _) => tr("Show notifications"),
            ("inhibit", _) => tr("Keep the computer awake"),
            ("remote-desktop", _) => tr("Control your computer remotely"),
            ("screencast", _) => tr("Record or share your screen"),
            ("wallpaper", _) => tr("Change the wallpaper"),
            ("realtime", _) => tr("Run with real-time priority"),
            ("gnome", "shell-introspect") => tr("Look at the windows on screen"),
            ("kde-authorized", _) => tr("Use desktop services"),
            _ => "",
        };
        if known.is_empty() { format!("{} ({})", self.table, self.id) } else { known.to_owned() }
    }

    pub fn describe_state(&self) -> String {
        match self.state() {
            State::Allowed => tr("Allowed").into(),
            State::Denied => tr("Denied").into(),
            State::Other if self.table == "location" => match self.values.first().map(String::as_str) {
                Some("EXACT") => tr("Allowed: exact location").into(),
                Some("CITY") | Some("NEIGHBORHOOD") | Some("STREET") => tr("Allowed: approximate location").into(),
                _ => tr("Remembered").into(),
            },
            State::Other => tr("Remembered").into(),
        }
    }
}

/// When set, permissions come from this file instead of the desktop (for the
/// README screenshots): one per line, `table<TAB>id<TAB>app<TAB>value…`.
fn demo_file() -> Option<PathBuf> {
    std::env::var_os("VOLTKIT_PERMISSIONS_FILE").map(PathBuf::from)
}

/// Tables in the permission store, read from its database folder.
fn tables() -> Vec<String> {
    let dir = dirs::data_dir().unwrap_or_default().join("flatpak/db");
    let mut tables: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .filter_map(|e| e.file_name().to_str().map(str::to_owned))
                .filter(|n| !n.contains('.') && !NOT_PERMISSIONS.contains(&n.as_str()))
                .collect()
        })
        .unwrap_or_default();
    tables.sort();
    tables
}

async fn call<R>(conn: &zbus::Connection, method: &str, body: &(impl serde::Serialize + zbus::zvariant::DynamicType)) -> Result<R>
where
    R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
{
    let reply = conn.call_method(Some(BUS_NAME), OBJECT_PATH, Some(INTERFACE), method, body).await?;
    Ok(reply.body().deserialize()?)
}

/// Every remembered answer, sorted by app and then by what was asked.
pub async fn load() -> Result<Vec<Permission>> {
    let mut all = if let Some(file) = demo_file() {
        parse_demo(&std::fs::read_to_string(file)?)
    } else {
        let conn = zbus::Connection::session().await.context(tr("no session bus"))?;
        let mut all = Vec::new();
        for table in tables() {
            let Ok((ids,)) = call::<(Vec<String>,)>(&conn, "List", &(table.as_str(),)).await else { continue };
            for id in ids {
                let Ok((apps, _data)) =
                    call::<(HashMap<String, Vec<String>>, zbus::zvariant::OwnedValue)>(&conn, "Lookup", &(table.as_str(), id.as_str())).await
                else {
                    continue;
                };
                for (app, values) in apps {
                    all.push(Permission { table: table.clone(), id: id.clone(), app, values });
                }
            }
        }
        all
    };
    all.sort_by(|a, b| (&a.app, a.title()).cmp(&(&b.app, b.title())));
    Ok(all)
}

fn parse_demo(text: &str) -> Vec<Permission> {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t');
            let (table, id, app) = (f.next()?, f.next()?, f.next()?);
            Some(Permission { table: table.into(), id: id.into(), app: app.into(), values: f.map(str::to_owned).collect() })
        })
        .collect()
}

/// Changes a yes/no answer.
pub async fn set_allowed(p: &Permission, allowed: bool) -> Result<()> {
    if demo_file().is_some() {
        return Ok(());
    }
    let conn = zbus::Connection::session().await?;
    let value = vec![if allowed { "yes" } else { "no" }];
    call::<()>(&conn, "SetPermission", &(p.table.as_str(), true, p.id.as_str(), p.app.as_str(), value)).await
}

/// Forgets the answer, so the app is asked again next time.
pub async fn forget(p: &Permission) -> Result<()> {
    if demo_file().is_some() {
        return Ok(());
    }
    let conn = zbus::Connection::session().await?;
    call::<()>(&conn, "DeletePermission", &(p.table.as_str(), p.id.as_str(), p.app.as_str())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perm(table: &str, id: &str, values: &[&str]) -> Permission {
        Permission { table: table.into(), id: id.into(), app: "org.example.App".into(), values: values.iter().map(|v| v.to_string()).collect() }
    }

    #[test]
    fn reads_states() {
        assert_eq!(perm("screenshot", "screenshot", &["yes"]).state(), State::Allowed);
        assert_eq!(perm("devices", "camera", &["no"]).state(), State::Denied);
        assert_eq!(perm("location", "location", &["NONE", "0"]).state(), State::Denied);
        assert_eq!(perm("location", "location", &["EXACT", "0"]).state(), State::Other);
        assert_eq!(perm("location", "location", &["EXACT", "0"]).describe_state(), "Allowed: exact location");
        assert_eq!(perm("screencast", "x", &[]).state(), State::Other);
    }

    #[test]
    fn names_permissions() {
        assert_eq!(perm("screenshot", "screenshot", &["yes"]).title(), "Take screenshots");
        assert_eq!(perm("devices", "camera", &["yes"]).title(), "Use the camera");
        assert_eq!(perm("mystery", "thing", &["yes"]).title(), "mystery (thing)");
    }

    #[test]
    fn reads_demo_files() {
        let list = parse_demo("# comment\nscreenshot\tscreenshot\torg.example.App\tyes\n\ndevices\tcamera\t\tno\n");
        assert_eq!(list.len(), 2);
        assert_eq!(list[1].app, "");
        assert_eq!(list[1].state(), State::Denied);
    }
}
