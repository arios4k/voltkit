//! Translations.
//!
//! The catalogs are ordinary gettext `.po` files in `po/`, compiled into the
//! binary, so translations work from the source tree, the .deb, the RPM and a
//! plain `cargo install` alike, with no files to install. The language comes
//! from the usual environment variables (LANGUAGE, LC_ALL, LC_MESSAGES, LANG);
//! anything without a catalog falls back to English.
//!
//! Mark text with [`tr`] (or [`trf!`] when it has `{}` placeholders, or [`trn`]
//! when it depends on a count). Text kept in tables and constants is marked with
//! [`N_!`] where it is written, and passed through [`tr`] where it is shown.
//! `po/update.py` collects every marked string into `po/voltkit.pot` and merges
//! it into each language's catalog.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// A language Voltkit has a catalog for, with its file.
struct Language {
    code: &'static str,
    po: &'static str,
}

const LANGUAGES: &[Language] = &[
    Language { code: "es", po: include_str!("../po/es.po") },
    Language { code: "de", po: include_str!("../po/de.po") },
];

/// One translated message: a single form, or the singular and plural forms.
type Catalog = HashMap<String, Vec<String>>;

/// The parsed catalogs, in the order of [`LANGUAGES`].
fn catalogs() -> &'static [Catalog] {
    static CATALOGS: OnceLock<Vec<Catalog>> = OnceLock::new();
    CATALOGS.get_or_init(|| LANGUAGES.iter().map(|l| parse_po(l.po)).collect())
}

/// Index into [`LANGUAGES`] plus one; 0 is English. `UNSET` until first use.
static CURRENT: AtomicU8 = AtomicU8::new(UNSET);
const UNSET: u8 = u8::MAX;

fn current_index() -> u8 {
    if let Some(index) = OVERRIDE.with(|o| o.get()) {
        return index;
    }
    let mut index = CURRENT.load(Ordering::Relaxed);
    if index == UNSET {
        index = detect();
        CURRENT.store(index, Ordering::Relaxed);
    }
    index
}

fn current() -> Option<&'static Catalog> {
    current_index().checked_sub(1).map(|i| &catalogs()[i as usize])
}

fn index_of(code: &str) -> Option<u8> {
    match LANGUAGES.iter().position(|l| l.code == code) {
        Some(i) => Some(i as u8 + 1),
        None if code == "en" => Some(0),
        None => None,
    }
}

/// Picks the language from the environment the way gettext does: LANGUAGE (a
/// colon-separated preference list) when the locale isn't C, otherwise the
/// first of LC_ALL, LC_MESSAGES and LANG that is set.
fn detect() -> u8 {
    // Tests check English text, whatever the machine running them uses.
    if cfg!(test) {
        return 0;
    }
    let get = |name| std::env::var(name).ok().filter(|v| !v.is_empty());
    let locale = get("LC_ALL").or_else(|| get("LC_MESSAGES")).or_else(|| get("LANG"));
    let is_c = |l: &str| l == "C" || l == "POSIX" || l.starts_with("C.");
    if locale.as_deref().is_some_and(is_c) {
        return 0;
    }
    let mut wanted: Vec<String> = get("LANGUAGE").map(|l| l.split(':').map(str::to_owned).collect()).unwrap_or_default();
    wanted.extend(locale);
    for w in &wanted {
        if w.starts_with("en") || is_c(w) {
            return 0;
        }
        let base = w.split(['_', '.', '@', '-']).next().unwrap_or("");
        if let Some(i) = LANGUAGES.iter().position(|l| l.code == base) {
            return i as u8 + 1;
        }
    }
    0
}

/// Uses `code` ("es", "de", or "en" for the original English) from now on,
/// whatever the environment says. Returns false if there's no such catalog.
pub fn set_language(code: &str) -> bool {
    let Some(index) = index_of(code) else { return false };
    CURRENT.store(index, Ordering::Relaxed);
    true
}

/// The codes of the translated languages.
pub fn languages() -> impl Iterator<Item = &'static str> {
    LANGUAGES.iter().map(|l| l.code)
}

/// The code of the language in use, "en" for English.
pub fn language() -> &'static str {
    current_index().checked_sub(1).map_or("en", |i| LANGUAGES[i as usize].code)
}

thread_local! {
    /// A language for this thread only, set by [`with_language`] and [`in_english`].
    static OVERRIDE: std::cell::Cell<Option<u8>> = const { std::cell::Cell::new(None) };
}

fn with_index<T>(index: u8, f: impl FnOnce() -> T) -> T {
    let was = OVERRIDE.with(|o| o.replace(Some(index)));
    let out = f();
    OVERRIDE.with(|o| o.set(was));
    out
}

/// Runs `f` with translations off on this thread, for text that is saved and
/// compared later, so that it doesn't depend on the language in use.
pub fn in_english<T>(f: impl FnOnce() -> T) -> T {
    with_index(0, f)
}

/// Runs `f` in language `code` ("en" for English) on this thread only, such
/// as to write each language's man pages.
pub fn with_language<T>(code: &str, f: impl FnOnce() -> T) -> T {
    with_index(index_of(code).unwrap_or(0), f)
}

/// Translates English text built from `templates`, where each `{}` stands for
/// a number, such as a value saved by [`in_english`]. Text that matches no
/// template is returned as it is.
pub fn translate_english(text: &str, templates: &[&str]) -> String {
    // Longest templates first, so "{} cores, {} threads" wins over "{} cores".
    let mut templates: Vec<&str> = templates.to_vec();
    templates.sort_by_key(|t| std::cmp::Reverse(t.len()));
    let mut out = text.to_owned();
    for template in templates {
        let parts: Vec<&str> = template.split("{}").collect();
        let mut from = 0;
        while let Some((start, end, numbers)) = find_template(&out, from, &parts) {
            let args: Vec<&dyn std::fmt::Display> = numbers.iter().map(|n| n as &dyn std::fmt::Display).collect();
            let replacement = fill(tr(template), &args);
            out.replace_range(start..end, &replacement);
            from = start + replacement.len();
        }
    }
    out
}

/// The first place at or after `from` where `text` matches `parts` (a template
/// split at its `{}`), with a number for each `{}`: (start, end, numbers).
fn find_template(text: &str, from: usize, parts: &[&str]) -> Option<(usize, usize, Vec<String>)> {
    let number_len = |s: &str| s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(s.len());
    let starts = (from..=text.len()).filter(|&i| text.is_char_boundary(i));
    for start in starts {
        let mut pos = start;
        if !text[pos..].starts_with(parts[0]) {
            continue;
        }
        // A number must start a word, not sit inside one like the 7840 of "7840HS".
        if parts[0].is_empty() && text[..start].chars().next_back().is_some_and(char::is_alphanumeric) {
            continue;
        }
        pos += parts[0].len();
        let mut numbers = Vec::new();
        let mut matched = true;
        for part in &parts[1..] {
            let rest = &text[pos..];
            // Trailing commas and dots belong to the text, not the number.
            let len = number_len(rest);
            let len = rest[..len].trim_end_matches([',', '.']).len();
            if len == 0 || !rest[len..].starts_with(part) {
                matched = false;
                break;
            }
            numbers.push(rest[..len].to_owned());
            pos += len + part.len();
        }
        if matched && pos > start {
            return Some((start, pos, numbers));
        }
    }
    None
}

/// `text` in the user's language, or `text` itself when it has no translation.
pub fn tr(text: &str) -> &str {
    match current().and_then(|c| c.get(text)).and_then(|forms| forms.first()) {
        Some(t) if !t.is_empty() => t,
        _ => text,
    }
}

/// The form of `singular`/`plural` that fits `n`, in the user's language.
/// Spanish and German, like English, use the singular for exactly one.
pub fn trn<'a>(singular: &'a str, plural: &'a str, n: u64) -> &'a str {
    let index = usize::from(n != 1);
    match current().and_then(|c| c.get(singular)).and_then(|forms| forms.get(index)) {
        Some(t) if !t.is_empty() => t,
        _ if n == 1 => singular,
        _ => plural,
    }
}

/// Fills `{}` placeholders in order, or `{0}`, `{1}`… by position (a translation
/// may need to move them). `{{` and `}}` are literal braces.
pub fn fill(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16 * args.len());
    let mut next = 0;
    let mut chars = template.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '{' if chars.peek().map(|(_, c)| *c) == Some('{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek().map(|(_, c)| *c) == Some('}') => {
                chars.next();
                out.push('}');
            }
            '{' => {
                let rest = &template[i + 1..];
                let Some(end) = rest.find('}') else {
                    out.push('{');
                    continue;
                };
                let inside = &rest[..end];
                let index = if inside.is_empty() {
                    next += 1;
                    Some(next - 1)
                } else {
                    inside.parse::<usize>().ok()
                };
                match index.and_then(|i| args.get(i)) {
                    Some(arg) => {
                        use std::fmt::Write;
                        let _ = write!(out, "{arg}");
                        for _ in 0..=end {
                            chars.next();
                        }
                    }
                    None => out.push('{'),
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// Translates a message with `{}` placeholders and fills them in:
/// `trf!("Saved {}", path.display())`.
macro_rules! trf {
    ($text:literal $(, $arg:expr)* $(,)?) => {
        $crate::i18n::fill($crate::i18n::tr($text), &[$(&$arg as &dyn ::std::fmt::Display),*])
    };
}

/// Like [`trf!`] for a message that depends on a count `n`, which is also the
/// first value filled in: `trnf!("{} file", "{} files", n)`.
macro_rules! trnf {
    ($singular:literal, $plural:literal, $n:expr $(, $arg:expr)* $(,)?) => {{
        let n = $n;
        $crate::i18n::fill($crate::i18n::trn($singular, $plural, n as u64), &[&n as &dyn ::std::fmt::Display $(, &$arg as &dyn ::std::fmt::Display)*])
    }};
}

/// Marks text for translation where it's stored, in a table or a constant,
/// without translating it there. Pass it through [`tr`] where it's shown.
macro_rules! N_ {
    ($text:literal) => {
        $text
    };
}

/// Reads the messages of a `.po` file. Fuzzy and untranslated entries are left
/// out, so they show in English rather than wrong.
fn parse_po(po: &str) -> Catalog {
    #[derive(Default)]
    struct Entry {
        id: String,
        forms: Vec<String>,
        fuzzy: bool,
    }
    let mut catalog = Catalog::new();
    let mut entry = Entry::default();
    // Which string a continuation line ("…") belongs to.
    enum Field {
        None,
        Id,
        Plural,
        Str(usize),
    }
    let mut field = Field::None;
    let mut finish = |entry: &mut Entry| {
        let e = std::mem::take(entry);
        if !e.id.is_empty() && !e.fuzzy && e.forms.iter().all(|f| !f.is_empty()) && !e.forms.is_empty() {
            catalog.insert(e.id, e.forms);
        }
    };
    for line in po.lines().map(str::trim) {
        if line.is_empty() {
            finish(&mut entry);
            field = Field::None;
        } else if let Some(flags) = line.strip_prefix("#,") {
            entry.fuzzy |= flags.split(',').any(|f| f.trim() == "fuzzy");
        } else if line.starts_with('#') {
        } else if let Some(rest) = line.strip_prefix("msgid_plural ") {
            let _ = unquote(rest);
            field = Field::Plural;
        } else if let Some(rest) = line.strip_prefix("msgid ") {
            if !entry.forms.is_empty() || !entry.id.is_empty() {
                finish(&mut entry);
            }
            entry.id = unquote(rest);
            field = Field::Id;
        } else if let Some(rest) = line.strip_prefix("msgstr[") {
            let (n, rest) = rest.split_once("] ").unwrap_or(("0", rest));
            let n: usize = n.parse().unwrap_or(0);
            if entry.forms.len() <= n {
                entry.forms.resize(n + 1, String::new());
            }
            entry.forms[n] = unquote(rest);
            field = Field::Str(n);
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            entry.forms = vec![unquote(rest)];
            field = Field::Str(0);
        } else if line.starts_with('"') {
            let more = unquote(line);
            match field {
                Field::Id => entry.id.push_str(&more),
                Field::Str(n) => entry.forms[n].push_str(&more),
                Field::Plural | Field::None => {}
            }
        }
    }
    finish(&mut entry);
    catalog
}

/// The text of a quoted `.po` string, with C escapes resolved.
fn unquote(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s);
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// Gives clap's help text in the user's language: the about and help of every
/// command and argument, and the headings and built-in flags clap adds.
pub fn localize_command(cmd: clap::Command, help_subcommand: bool) -> clap::Command {
    use clap::builder::StyledStr;
    let t = |s: Option<&StyledStr>| s.map(|s| tr(&s.to_string()).to_owned());
    let about = t(cmd.get_about());
    let long_about = t(cmd.get_long_about());
    let mut cmd = cmd
        .subcommand_help_heading(tr("Commands"))
        .help_template(format!(
            "{{about-with-newline}}\n{}: {{usage}}\n\n{{all-args}}{{after-help}}",
            tr("Usage")
        ));
    if let Some(a) = about {
        cmd = cmd.about(a);
    }
    if let Some(a) = long_about {
        cmd = cmd.long_about(a);
    }
    let options: &'static str = tr("Options");
    let arguments: &'static str = tr("Arguments");
    let cmd = cmd.mut_args(|arg| {
        let help = t(arg.get_help());
        let long_help = t(arg.get_long_help());
        let heading = if arg.is_positional() { arguments } else { options };
        let mut arg = arg.help_heading(heading);
        if let Some(h) = help {
            arg = arg.help(h);
        }
        if let Some(h) = long_help {
            arg = arg.long_help(h);
        }
        arg
    });
    // clap's own wording: a pointer to the other form of help when there's more to see.
    let has_long = cmd.get_long_about().is_some() || cmd.get_arguments().any(|a| a.get_long_help().is_some());
    let help = clap::Arg::new("help").short('h').long("help").action(clap::ArgAction::Help).help_heading(options);
    let help = if has_long {
        help.help(tr("Print help (see more with '--help')")).long_help(tr("Print help (see a summary with '-h')"))
    } else {
        help.help(tr("Print help"))
    };
    let cmd = cmd.disable_help_flag(true).arg(help);
    let cmd = if cmd.get_version().is_some() {
        cmd.disable_version_flag(true).arg(
            clap::Arg::new("version")
                .short('V')
                .long("version")
                .action(clap::ArgAction::Version)
                .help(tr("Print version"))
                .help_heading(options),
        )
    } else {
        cmd
    };
    let names: Vec<String> = cmd.get_subcommands().map(|s| s.get_name().to_owned()).collect();
    let mut cmd = cmd;
    for name in names {
        cmd = cmd.mut_subcommand(name, |c| localize_command(c, help_subcommand));
    }
    if help_subcommand && cmd.has_subcommands() {
        cmd = cmd.disable_help_subcommand(true).subcommand(
            clap::Command::new("help")
                .about(tr("Print this message or the help of the given subcommand(s)"))
                .arg(clap::Arg::new("command").num_args(0..).action(clap::ArgAction::Append))
                .disable_help_flag(true)
                .hide(false),
        );
    }
    cmd
}

/// Every piece of help text in `cmd` and its subcommands, for collecting into
/// the catalog (see `po/update.py`) and for checking it's all translated.
pub fn command_strings(cmd: &clap::Command) -> Vec<String> {
    let mut out = Vec::new();
    let mut push = |s: Option<&clap::builder::StyledStr>| {
        if let Some(s) = s.map(|s| s.to_string()).filter(|s| !s.is_empty()) {
            out.push(s);
        }
    };
    push(cmd.get_about());
    push(cmd.get_long_about());
    for arg in cmd.get_arguments().filter(|a| !a.is_hide_set()) {
        push(arg.get_help());
        push(arg.get_long_help());
    }
    for sub in cmd.get_subcommands().filter(|s| !s.is_hide_set()) {
        out.extend(command_strings(sub));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_placeholders_in_order_and_by_position() {
        assert_eq!(fill("{} of {}", &[&1, &2]), "1 of 2");
        assert_eq!(fill("{1} de {0}", &[&"a", &"b"]), "b de a");
        assert_eq!(fill("{{literal}} {}", &[&"x"]), "{literal} x");
        assert_eq!(fill("{missing}", &[]), "{missing}");
    }

    #[test]
    fn finds_numbers_in_saved_text() {
        let parts = ["", " cores, ", " threads"];
        assert_eq!(
            find_template("AMD Ryzen 7 7840HS · 8 cores, 16 threads, up to 5.1 GHz", 0, &parts),
            Some((22, 41, vec!["8".to_owned(), "16".to_owned()]))
        );
        assert_eq!(find_template("up to 5.1 GHz", 0, &["up to ", " GHz"]), Some((0, 13, vec!["5.1".to_owned()])));
        assert_eq!(find_template("no driver here", 0, &["no driver"]), Some((0, 9, vec![])));
        assert_eq!(find_template("7840HS cores", 0, &["", " cores"]), None);
        // In English, translating is a no-op.
        let text = "AMD Ryzen 7 · 8 cores, 16 threads, up to 5.1 GHz";
        assert_eq!(translate_english(text, &["{} cores, {} threads", "{} cores", "up to {} GHz"]), text);
    }

    #[test]
    fn reads_po_entries() {
        let c = parse_po(
            "msgid \"\"\nmsgstr \"Content-Type: text/plain\\n\"\n\n\
             msgid \"Hello\"\nmsgstr \"Hola\"\n\n\
             #, fuzzy\nmsgid \"Fuzzy\"\nmsgstr \"Dudoso\"\n\n\
             msgid \"Empty\"\nmsgstr \"\"\n\n\
             msgid \"\"\n\"Two \"\n\"lines\"\nmsgstr \"\"\n\"Dos \"\n\"líneas\"\n\n\
             msgid \"{} file\"\nmsgid_plural \"{} files\"\nmsgstr[0] \"{} archivo\"\nmsgstr[1] \"{} archivos\"\n",
        );
        assert_eq!(c["Hello"], ["Hola"]);
        assert_eq!(c["Two lines"], ["Dos líneas"]);
        assert_eq!(c["{} file"], ["{} archivo", "{} archivos"]);
        assert!(!c.contains_key("Fuzzy") && !c.contains_key("Empty"));
    }

    /// Placeholders in a translation must match the original, or text would
    /// show with a value missing or a stray `{}`.
    #[test]
    fn translations_keep_their_placeholders() {
        fn holes(s: &str) -> usize {
            let s = s.replace("{{", "").replace("}}", "");
            let mut n = 0;
            let mut rest = s.as_str();
            while let Some(i) = rest.find('{') {
                let after = &rest[i + 1..];
                match after.find('}') {
                    Some(j) if after[..j].is_empty() || after[..j].parse::<usize>().is_ok() => {
                        n += 1;
                        rest = &after[j + 1..];
                    }
                    _ => rest = after,
                }
            }
            n
        }
        let mut bad = Vec::new();
        for (lang, catalog) in LANGUAGES.iter().zip(catalogs()) {
            for (id, forms) in catalog {
                for f in forms {
                    if holes(f) != holes(id) {
                        bad.push(format!("{}: {id:?} → {f:?}", lang.code));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "placeholders differ:\n{}", bad.join("\n"));
    }
}
