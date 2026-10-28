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
    fn lists_required_args() {
        assert_eq!(required_args("{arg:a} {arg: b } {arg:a}"), vec!["a", "b"]);
    }
}
