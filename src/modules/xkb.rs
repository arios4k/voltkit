//! Keyboard keys: the few key changes almost everyone wants (Caps Lock as Esc or
//! Ctrl, Alt and Super swapped, a Compose key), made through the desktop's own
//! keyboard options. They are a plain setting, so there is nothing to install, no
//! background program reads the keyboard, and each change is undone by choosing
//! "As it comes", which can't lock anyone out of their keyboard.

use gtk::gio;
use gtk::prelude::*;

use crate::i18n::tr;

const SCHEMA: &str = "org.gnome.desktop.input-sources";
const KEY: &str = "xkb-options";

/// One thing to choose, with the option each choice puts on (empty: nothing, as it comes).
pub struct Group {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    /// (xkb option, what it is called)
    pub choices: &'static [(&'static str, &'static str)],
}

pub const GROUPS: &[Group] = &[
    Group {
        id: "caps",
        title: N_!("Caps Lock"),
        subtitle: N_!("Make the key you rarely use into one you do."),
        choices: &[
            ("", N_!("As it comes")),
            ("caps:escape", N_!("Escape")),
            ("ctrl:nocaps", N_!("Ctrl")),
            ("caps:swapescape", N_!("Swap with Escape")),
            ("caps:backspace", N_!("Backspace")),
        ],
    },
    Group {
        id: "altwin",
        title: N_!("Alt and Super"),
        subtitle: N_!("For keyboards laid out like a Mac's."),
        choices: &[("", N_!("As they come")), ("altwin:swap_alt_win", N_!("Swap Alt and Super"))],
    },
    Group {
        id: "compose",
        title: N_!("Compose key"),
        subtitle: N_!("Press it, then two keys, to type accents and symbols: Compose, ' and e give é."),
        choices: &[
            ("", N_!("Off")),
            ("compose:ralt", N_!("Right Alt")),
            ("compose:menu", N_!("Menu key")),
            ("compose:rctrl", N_!("Right Ctrl")),
            ("compose:rwin", N_!("Right Super")),
            ("compose:caps", N_!("Caps Lock")),
        ],
    },
];

pub fn group(id: &str) -> Option<&'static Group> {
    GROUPS.iter().find(|g| g.id == id)
}

/// The index of the choice a group has in `options` (0, "as it comes", when none).
pub fn selected(options: &[String], group: &Group) -> usize {
    group.choices.iter().position(|(opt, _)| !opt.is_empty() && options.iter().any(|o| o == opt)).unwrap_or(0)
}

/// `options` with the group's choice put on. Other options (for example the way you switch
/// layouts) are kept as they are. Two things can't both own Caps Lock, so choosing it for the
/// Compose key lets go of Caps Lock's other jobs, and the other way round.
pub fn with_choice(options: &[String], group_id: &str, choice: usize) -> Vec<String> {
    let Some(g) = group(group_id) else { return options.to_vec() };
    let wanted = g.choices.get(choice).map_or("", |c| c.0);
    let mut out: Vec<String> = options.iter().filter(|o| !g.choices.iter().any(|(opt, _)| opt == o)).cloned().collect();
    if wanted == "compose:caps" {
        let caps = group("caps").map_or(&[][..], |c| c.choices);
        out.retain(|o| !caps.iter().any(|(opt, _)| opt == o));
    }
    if group_id == "caps" && !wanted.is_empty() {
        out.retain(|o| o != "compose:caps");
    }
    if !wanted.is_empty() {
        out.push(wanted.to_owned());
    }
    out
}

pub fn title(group: &Group) -> &'static str {
    tr(group.title)
}

// ---------------------------------------------------------------- The desktop

/// Whether this desktop keeps its keyboard options where Voltkit can change them.
pub fn supported() -> bool {
    gio::SettingsSchemaSource::default().and_then(|s| s.lookup(SCHEMA, true)).is_some_and(|s| s.has_key(KEY))
}

pub fn read() -> Vec<String> {
    if !supported() {
        return Vec::new();
    }
    gio::Settings::new(SCHEMA).strv(KEY).iter().map(|s| s.to_string()).collect()
}

pub fn write(options: &[String]) -> anyhow::Result<()> {
    let refs: Vec<&str> = options.iter().map(String::as_str).collect();
    gio::Settings::new(SCHEMA).set_strv(KEY, refs.as_slice())?;
    gio::Settings::sync();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_choice_is_put_on_and_other_options_are_kept() {
        let layouts = opts(&["grp:alt_shift_toggle", "lv3:ralt_switch"]);
        let after = with_choice(&layouts, "caps", 1);
        assert_eq!(after, opts(&["grp:alt_shift_toggle", "lv3:ralt_switch", "caps:escape"]));
        // Choosing another one replaces it; choosing "as it comes" removes it.
        let ctrl = with_choice(&after, "caps", 2);
        assert_eq!(ctrl, opts(&["grp:alt_shift_toggle", "lv3:ralt_switch", "ctrl:nocaps"]));
        assert_eq!(with_choice(&ctrl, "caps", 0), layouts);
    }

    #[test]
    fn the_selection_is_read_back_from_the_options() {
        let caps = group("caps").unwrap();
        assert_eq!(selected(&opts(&[]), caps), 0);
        assert_eq!(selected(&opts(&["ctrl:nocaps"]), caps), 2);
        assert_eq!(selected(&opts(&["compose:ralt", "caps:swapescape"]), caps), 3);
        assert_eq!(selected(&opts(&["compose:ralt"]), group("compose").unwrap()), 1);
        assert_eq!(selected(&opts(&["unrelated:thing"]), caps), 0);
    }

    #[test]
    fn groups_do_not_disturb_each_other() {
        let start = opts(&["caps:escape", "altwin:swap_alt_win"]);
        let with_compose = with_choice(&start, "compose", 1);
        assert_eq!(with_compose, opts(&["caps:escape", "altwin:swap_alt_win", "compose:ralt"]));
        let no_swap = with_choice(&with_compose, "altwin", 0);
        assert_eq!(no_swap, opts(&["caps:escape", "compose:ralt"]));
    }

    #[test]
    fn caps_lock_cannot_be_two_things_at_once() {
        // Compose on Caps Lock lets go of Caps Lock as Escape.
        let a = with_choice(&opts(&["caps:escape"]), "compose", 5);
        assert_eq!(a, opts(&["compose:caps"]));
        // And choosing a Caps Lock job drops Compose on Caps Lock.
        let b = with_choice(&a, "caps", 2);
        assert_eq!(b, opts(&["ctrl:nocaps"]));
        // But "as it comes" for Caps Lock leaves Compose alone.
        assert_eq!(with_choice(&a, "caps", 0), a);
    }

    #[test]
    fn nonsense_changes_nothing() {
        let o = opts(&["caps:escape"]);
        assert_eq!(with_choice(&o, "nope", 1), o);
        assert_eq!(with_choice(&o, "caps", 99), Vec::<String>::new(), "an unknown choice means as it comes");
    }

    #[test]
    fn every_choice_is_a_real_xkb_option_shape() {
        for g in GROUPS {
            assert_eq!(g.choices[0].0, "", "{} starts with as-it-comes", g.id);
            for (opt, _) in &g.choices[1..] {
                assert!(opt.contains(':') && !opt.contains(' '), "{opt}");
            }
        }
    }
}
