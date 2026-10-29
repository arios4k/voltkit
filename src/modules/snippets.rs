//! Snippet template engine.
//!
//! Variables are written in braces:
//!
//! | Variable          | Expands to                                  |
//! |-------------------|---------------------------------------------|
//! | `{date}`          | today's date, `%Y-%m-%d`                    |
//! | `{date:FMT}`      | the current time in any `strftime` format   |
//! | `{time}`          | the current time, `%H:%M`                   |
//! | `{clipboard}`     | current clipboard text                      |
//! | `{arg:NAME}`      | a macro parameter supplied at expansion     |
//!
//! `{{` and `}}` produce literal braces.

use std::collections::HashMap;
use std::fmt::Write;

use anyhow::{Result, bail};
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct Context {
    pub now: DateTime<Local>,
    pub clipboard: Option<String>,
    pub args: HashMap<String, String>,
}

impl Default for Context {
    fn default() -> Self {
        Self { now: Local::now(), clipboard: None, args: HashMap::new() }
    }
}

pub fn expand(template: &str, ctx: &Context) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push_str(&tail[..1]);
            rest = &tail[2..];
            continue;
        }
        if tail.starts_with('}') {
            bail!("unmatched '}}' at byte {}", template.len() - tail.len());
        }
        let Some(end) = tail.find('}') else {
            bail!("unclosed '{{' at byte {}", template.len() - tail.len());
        };
        expand_var(&tail[1..end], ctx, &mut out)?;
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

fn expand_var(var: &str, ctx: &Context, out: &mut String) -> Result<()> {
    let (name, param) = match var.split_once(':') {
        Some((n, p)) => (n.trim(), Some(p)),
        None => (var.trim(), None),
    };
    match (name, param) {
        ("date", None) => write!(out, "{}", ctx.now.format("%Y-%m-%d"))?,
        ("time", None) => write!(out, "{}", ctx.now.format("%H:%M"))?,
        ("date" | "time", Some(fmt)) => {
            // chrono panics on Display for invalid formats; validate first.
            let items: Vec<_> = chrono::format::StrftimeItems::new(fmt).collect();
            if items.iter().any(|i| matches!(i, chrono::format::Item::Error)) {
                bail!("invalid date format {fmt:?}");
            }
            write!(out, "{}", ctx.now.format_with_items(items.into_iter()))?
        }
        ("clipboard", None) => out.push_str(ctx.clipboard.as_deref().unwrap_or("")),
        ("arg", Some(key)) => match ctx.args.get(key.trim()) {
            Some(v) => out.push_str(v),
            None => bail!("missing parameter {:?}", key.trim()),
        },
        _ => bail!("unknown variable {{{var}}}"),
    }
    Ok(())
}

/// The template as it would be pasted, with fill-in fields and the clipboard
/// shown as ‹placeholders›.
pub fn preview(template: &str) -> Result<String> {
    let ctx = Context {
        clipboard: Some("‹clipboard›".into()),
        args: required_args(template).into_iter().map(|a| (a.clone(), format!("‹{a}›"))).collect(),
        ..Default::default()
    };
    expand(template, &ctx)
}

/// Names of the `{arg:NAME}` parameters a template needs, in order of first use.
pub fn required_args(template: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = template;
    while let Some(i) = rest.find("{arg:") {
        let tail = &rest[i + 5..];
        let Some(end) = tail.find('}') else { break };
        let name = tail[..end].trim().to_owned();
        if !names.contains(&name) {
            names.push(name);
        }
        rest = &tail[end..];
    }
    names
}

/// Scores how well `query` matches `text` as a case-insensitive subsequence.
/// Higher is better; `None` means no match. Consecutive and word-start matches score more.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let mut score = 0;
    let mut pos = 0;
    let mut prev_match: Option<usize> = None;
    for q in query.to_lowercase().chars().filter(|c| !c.is_whitespace()) {
        let i = pos + text[pos..].iter().position(|&c| c == q)?;
        score += 1;
        if prev_match.is_some_and(|p| p + 1 == i) {
            score += 5;
        }
        if i == 0 || !text[i - 1].is_alphanumeric() {
            score += 3;
        }
        score -= (i - pos).min(5) as i32;
        prev_match = Some(i);
        pos = i + 1;
    }
    Some(score)
}

/// Snippets matching `query`, best first. Trigger matches outrank template matches.
pub fn search<'a>(snippets: &'a [crate::config::Snippet], query: &str) -> Vec<&'a crate::config::Snippet> {
    let mut hits: Vec<(i32, usize)> = snippets
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let by_trigger = fuzzy_score(query, &s.trigger).map(|v| v * 2 + 10);
            let by_template = fuzzy_score(query, &s.template);
            by_trigger.max(by_template).map(|score| (score, i))
        })
        .collect();
    // Stable sort keeps config order for ties (and for an empty query).
    hits.sort_by_key(|&(score, _)| std::cmp::Reverse(score));
    hits.into_iter().map(|(_, i)| &snippets[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ctx() -> Context {
        Context {
            now: Local.with_ymd_and_hms(2026, 3, 14, 9, 26, 0).unwrap(),
            clipboard: Some("copied".into()),
            args: HashMap::from([("name".into(), "Ada".into())]),
        }
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(expand("hello", &ctx()).unwrap(), "hello");
    }

    #[test]
    fn expands_builtin_variables() {
        let out = expand("{date} {time} [{clipboard}]", &ctx()).unwrap();
        assert_eq!(out, "2026-03-14 09:26 [copied]");
    }

    #[test]
    fn custom_date_format() {
        assert_eq!(expand("{date:%A %d %B}", &ctx()).unwrap(), "Saturday 14 March");
        assert!(expand("{date:%Q}", &ctx()).is_err());
    }

    #[test]
    fn macro_parameters() {
        assert_eq!(expand("Hi {arg:name}!", &ctx()).unwrap(), "Hi Ada!");
        assert!(expand("{arg:missing}", &ctx()).is_err());
    }

    #[test]
    fn escaped_braces() {
        assert_eq!(expand("fn() {{ {arg:name} }}", &ctx()).unwrap(), "fn() { Ada }");
    }

    #[test]
    fn reports_malformed_templates() {
        assert!(expand("{date", &ctx()).is_err());
        assert!(expand("oops}", &ctx()).is_err());
        assert!(expand("{nope}", &ctx()).is_err());
    }

    #[test]
    fn empty_clipboard_expands_to_nothing() {
        let c = Context { clipboard: None, ..ctx() };
        assert_eq!(expand("[{clipboard}]", &c).unwrap(), "[]");
    }

    #[test]
    fn fuzzy_matching() {
        assert!(fuzzy_score("dt", ";date").is_some());
        assert!(fuzzy_score("xyz", ";date").is_none());
        assert!(fuzzy_score("dat", ";date") > fuzzy_score("dat", "a d b a c t"));
        assert_eq!(fuzzy_score("", "anything"), Some(0));
    }

    #[test]
    fn search_ranks_triggers_first() {
        use crate::config::Snippet;
        let s = |t: &str, b: &str| Snippet { trigger: t.into(), template: b.into() };
        let list = vec![s(";sig", "Best regards"), s(";date", "{date}"), s(";now", "{date} {time}")];
        let found: Vec<_> = search(&list, "date").iter().map(|s| s.trigger.as_str()).collect();
        assert_eq!(found, vec![";date", ";now"]);
        assert_eq!(search(&list, "").len(), 3);
        assert_eq!(search(&list, "regards")[0].trigger, ";sig");
    }

    #[test]
    fn preview_shows_placeholders() {
        assert_eq!(preview("> {clipboard} — {arg:author}").unwrap(), "> ‹clipboard› — ‹author›");
        assert!(preview("{nope}").is_err());
    }

    #[test]
    fn lists_required_args() {
        assert_eq!(required_args("{arg:a} {arg: b } {arg:a}"), vec!["a", "b"]);
    }
}
