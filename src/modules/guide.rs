//! The Shortcut Guide: a cheat sheet of the system-wide shortcuts, grouped by who
//! owns them. It is a read-only view of what Shortcut Detective already knows,
//! plus a few shortcuts almost every app shares.

use super::keybindings::Binding;
use crate::i18n::tr;

/// One shortcut in the sheet: what it does and the keys that do it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub action: String,
    /// GSettings form (`<Super>s`), shown with [`super::keybindings::label`].
    pub keys: Vec<String>,
}

/// A group of shortcuts under one heading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub title: String,
    pub rows: Vec<Row>,
}

/// Shortcuts that work in most apps, in the same form as the system ones.
fn common() -> Section {
    let row = |action: &str, key: &str| Row { action: tr(action).to_owned(), keys: vec![key.to_owned()] };
    Section {
        title: tr("In most apps").to_owned(),
        rows: vec![
            row(N_!("Copy"), "<Control>c"),
            row(N_!("Cut"), "<Control>x"),
            row(N_!("Paste"), "<Control>v"),
            row(N_!("Undo"), "<Control>z"),
            row(N_!("Redo"), "<Control><Shift>z"),
            row(N_!("Select all"), "<Control>a"),
            row(N_!("Find"), "<Control>f"),
            row(N_!("Save"), "<Control>s"),
            row(N_!("Print"), "<Control>p"),
            row(N_!("Close the window or tab"), "<Control>w"),
            row(N_!("New tab"), "<Control>t"),
            row(N_!("Quit the app"), "<Control>q"),
        ],
    }
}

/// How the groups are ordered: Voltkit's own first, then the window manager and the
/// shell, then everything else alphabetically.
fn rank(owner: &str) -> (u8, String) {
    let first = match owner {
        "Voltkit" => 0,
        "Windows" => 1,
        "GNOME Shell" => 2,
        _ => 3,
    };
    (first, owner.to_lowercase())
}

/// The sheet for `bindings`, narrowed to what matches `filter` (an action, a group or keys).
/// Actions without a shortcut are left out; the "In most apps" group goes last.
pub fn sections(bindings: &[Binding], filter: &str) -> Vec<Section> {
    let needle = filter.trim().to_lowercase();
    let matches = |title: &str, row: &Row| {
        needle.is_empty()
            || title.to_lowercase().contains(&needle)
            || row.action.to_lowercase().contains(&needle)
            || row.keys.iter().any(|k| super::keybindings::label(k).to_lowercase().contains(&needle) || k.to_lowercase().contains(&needle))
    };
    let mut owners: Vec<&str> = bindings.iter().filter(|b| !b.accels.is_empty()).map(|b| b.owner.as_str()).collect();
    owners.sort_by_key(|o| rank(o));
    owners.dedup();
    let mut out = Vec::new();
    for owner in owners {
        let mut rows: Vec<Row> = bindings
            .iter()
            .filter(|b| b.owner == owner && !b.accels.is_empty())
            .map(|b| Row { action: b.name.clone(), keys: b.accels.clone() })
            .filter(|r| matches(owner, r))
            .collect();
        rows.sort_by_key(|r| r.action.to_lowercase());
        rows.dedup();
        if !rows.is_empty() {
            out.push(Section { title: owner.to_owned(), rows });
        }
    }
    let mut extra = common();
    extra.rows.retain(|r| matches(&extra.title, r));
    if !extra.rows.is_empty() {
        out.push(extra);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(owner: &str, name: &str, accels: &[&str]) -> Binding {
        Binding {
            schema: "x".into(),
            path: None,
            schema_dir: None,
            key: name.into(),
            name: name.into(),
            owner: owner.into(),
            accels: accels.iter().map(|s| (*s).to_owned()).collect(),
            defaults: Vec::new(),
            list: true,
            writable: true,
        }
    }

    fn sample() -> Vec<Binding> {
        vec![
            b("Zeta extension", "Do the thing", &["<Super>z"]),
            b("GNOME Shell", "Open the overview", &["<Super>"]),
            b("Windows", "Close window", &["<Alt>F4"]),
            b("Windows", "Switch windows", &["<Alt>Tab", "<Super>Tab"]),
            b("Voltkit", "Measure on screen", &["<Super><Shift>r"]),
            b("Windows", "Not set", &[]),
        ]
    }

    #[test]
    fn groups_are_ordered_and_unset_shortcuts_are_left_out() {
        let titles: Vec<String> = sections(&sample(), "").into_iter().map(|s| s.title).collect();
        assert_eq!(titles[..4], ["Voltkit", "Windows", "GNOME Shell", "Zeta extension"]);
        assert_eq!(titles.last().map(String::as_str), Some(tr("In most apps")));
        let windows = &sections(&sample(), "")[1];
        assert_eq!(windows.rows.len(), 2, "the unset one is not listed");
        assert_eq!(windows.rows[0].action, "Close window", "rows are alphabetical");
        assert_eq!(windows.rows[1].keys, ["<Alt>Tab", "<Super>Tab"]);
    }

    #[test]
    fn the_filter_matches_actions_groups_and_keys() {
        let by_action = sections(&sample(), "overview");
        assert_eq!(by_action.len(), 1);
        assert_eq!(by_action[0].title, "GNOME Shell");
        let by_group = sections(&sample(), "windows");
        assert!(by_group.iter().any(|s| s.title == "Windows" && s.rows.len() == 2));
        let by_keys = sections(&sample(), "alt+f4");
        assert_eq!(by_keys.len(), 1);
        assert_eq!(by_keys[0].rows[0].action, "Close window");
        assert!(sections(&sample(), "zzzzzz").is_empty());
    }

    #[test]
    fn the_common_shortcuts_always_exist_even_with_nothing_else() {
        let only = sections(&[], "");
        assert_eq!(only.len(), 1);
        assert!(only[0].rows.iter().any(|r| r.keys == ["<Control>c"]));
        assert_eq!(sections(&[], "paste")[0].rows.len(), 1);
    }
}
