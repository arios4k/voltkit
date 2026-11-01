//! Smart Paste: ways to convert clipboard text before pasting it. Each
//! conversion says whether it fits the text, so only useful ones are offered.

pub struct Conversion {
    pub id: &'static str,
    pub title: &'static str,
    /// The converted text, or `None` when this conversion doesn't fit.
    pub apply: fn(&str) -> Option<String>,
}

/// Specific conversions (for JSON, tables, links) come before general ones.
pub const ALL: &[Conversion] = &[
    Conversion { id: "plain", title: "Plain text", apply: plain },
    Conversion { id: "json", title: "Formatted JSON", apply: json_pretty },
    Conversion { id: "json-compact", title: "Compact JSON", apply: json_compact },
    Conversion { id: "markdown-table", title: "Markdown table", apply: markdown_table },
    Conversion { id: "csv", title: "CSV", apply: csv },
    Conversion { id: "clean-link", title: "Link without tracking", apply: clean_link },
    Conversion { id: "decode-link", title: "Readable link", apply: decode_link },
    Conversion { id: "base64-decode", title: "Decoded Base64", apply: base64_decode },
    Conversion { id: "join-lines", title: "Join broken lines", apply: join_lines },
    Conversion { id: "one-line", title: "One line", apply: one_line },
    Conversion { id: "tidy-spaces", title: "Tidy spaces", apply: tidy_spaces },
    Conversion { id: "bullets", title: "Bulleted list", apply: bullets },
    Conversion { id: "numbered", title: "Numbered list", apply: numbered },
    Conversion { id: "quote", title: "Quote", apply: quote },
    Conversion { id: "upper", title: "UPPERCASE", apply: upper },
    Conversion { id: "lower", title: "lowercase", apply: lower },
    Conversion { id: "title", title: "Title Case", apply: title_case },
    Conversion { id: "sentence", title: "Sentence case", apply: sentence_case },
    Conversion { id: "snake", title: "snake_case", apply: snake_case },
    Conversion { id: "kebab", title: "kebab-case", apply: kebab_case },
    Conversion { id: "camel", title: "camelCase", apply: camel_case },
];

/// The conversions that fit `text`, with their results, in display order.
/// "Plain text" always comes first; others only when they change something.
pub fn available(text: &str) -> Vec<(&'static Conversion, String)> {
    ALL.iter()
        .filter_map(|c| {
            let out = (c.apply)(text)?;
            (c.id == "plain" || out != text).then_some((c, out))
        })
        .collect()
}

pub fn find(id: &str) -> Option<&'static Conversion> {
    ALL.iter().find(|c| c.id == id)
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

/// The text without formatting, with Unix line endings.
fn plain(text: &str) -> Option<String> {
    Some(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Text copied from a PDF or email breaks every line; this joins the lines of
/// each paragraph (keeping blank lines between them) and rejoins hyphenated words.
fn join_lines(text: &str) -> Option<String> {
    // Tables, lists and code have short or structured lines; prose wraps long ones.
    let long = text.lines().filter(|l| !l.trim().is_empty()).map(|l| l.trim().chars().count()).sum::<usize>()
        >= 25 * text.lines().filter(|l| !l.trim().is_empty()).count().max(1);
    if !long || table(text).is_some() || json_value(text).is_some() {
        return None;
    }
    let text = plain(text)?;
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    if !paragraphs.iter().any(|p| p.trim().contains('\n')) {
        return None;
    }
    let joined: Vec<String> = paragraphs
        .iter()
        .map(|p| {
            let mut out = String::new();
            for line in p.lines().map(str::trim) {
                if line.is_empty() {
                    continue;
                }
                if out.is_empty() {
                    out.push_str(line);
                } else if out.ends_with('-') && out.chars().rev().nth(1).is_some_and(char::is_alphabetic)
                    && line.chars().next().is_some_and(char::is_lowercase)
                {
                    out.pop();
                    out.push_str(line);
                } else {
                    out.push(' ');
                    out.push_str(line);
                }
            }
            out
        })
        .collect();
    Some(joined.join("\n\n"))
}

fn one_line(text: &str) -> Option<String> {
    (text.trim().contains('\n') && table(text).is_none()).then(|| text.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Removes trailing spaces, repeated spaces and tabs, and blank lines at the ends.
fn tidy_spaces(text: &str) -> Option<String> {
    // Tabs separate the cells of a copied table: keep them.
    if text.contains('\t') && table(text).is_some() {
        return None;
    }
    let tidy: Vec<String> = plain(text)?.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect();
    let mut out = tidy.join("\n");
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    Some(out.trim().to_owned())
}

fn json_value(text: &str) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(text.trim()).ok()?;
    (value.is_object() || value.is_array()).then_some(value)
}

fn json_pretty(text: &str) -> Option<String> {
    serde_json::to_string_pretty(&json_value(text)?).ok()
}

fn json_compact(text: &str) -> Option<String> {
    serde_json::to_string(&json_value(text)?).ok()
}

/// Rows of a table copied from a spreadsheet (tabs) or a CSV file.
fn table(text: &str) -> Option<Vec<Vec<String>>> {
    let rows: Vec<&str> = lines(text.trim_end()).into_iter().filter(|l| !l.trim().is_empty()).collect();
    if rows.len() < 2 {
        return None;
    }
    let tabs = rows[0].matches('\t').count();
    if tabs > 0 && rows.iter().all(|r| r.matches('\t').count() == tabs) {
        return Some(rows.iter().map(|r| r.split('\t').map(|c| c.trim().to_owned()).collect()).collect());
    }
    let parsed: Vec<Vec<String>> = rows.iter().map(|r| parse_csv_row(r)).collect();
    let width = parsed[0].len();
    (width > 1 && parsed.iter().all(|r| r.len() == width)).then_some(parsed)
}

fn parse_csv_row(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = row.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => cells.push(std::mem::take(&mut cell).trim().to_owned()),
            c => cell.push(c),
        }
    }
    cells.push(cell.trim().to_owned());
    cells
}

fn markdown_table(text: &str) -> Option<String> {
    let rows = table(text)?;
    let width = rows[0].len();
    let cell = |s: &str| s.replace('|', "\\|");
    let mut out = format!("| {} |\n", rows[0].iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | "));
    out.push_str(&format!("|{}\n", " --- |".repeat(width)));
    for row in &rows[1..] {
        out.push_str(&format!("| {} |\n", row.iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | ")));
    }
    Some(out.trim_end().to_owned())
}

/// Tab-separated (spreadsheet) text as CSV.
fn csv(text: &str) -> Option<String> {
    if !text.contains('\t') {
        return None;
    }
    let rows = table(text)?;
    let quote = |c: &str| {
        if c.contains([',', '"', '\n']) { format!("\"{}\"", c.replace('"', "\"\"")) } else { c.to_owned() }
    };
    Some(rows.iter().map(|r| r.iter().map(|c| quote(c)).collect::<Vec<_>>().join(",")).collect::<Vec<_>>().join("\n"))
}

fn single_link(text: &str) -> Option<&str> {
    let t = text.trim();
    ((t.starts_with("https://") || t.starts_with("http://")) && !t.contains(char::is_whitespace)).then_some(t)
}

/// Removes tracking parameters (utm_*, fbclid, gclid, …) from a link.
fn clean_link(text: &str) -> Option<String> {
    const TRACKING: &[&str] = &[
        "fbclid", "gclid", "dclid", "gbraid", "wbraid", "msclkid", "mc_cid", "mc_eid", "igshid", "yclid", "_hsenc", "_hsmi",
        "mkt_tok", "si", "ref_src", "ref_url", "spm", "share_id",
    ];
    let link = single_link(text)?;
    let (base, fragment) = link.split_once('#').map_or((link, None), |(b, f)| (b, Some(f)));
    let (path, query) = base.split_once('?')?;
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| {
            let key = p.split('=').next().unwrap_or("");
            !(key.starts_with("utm_") || TRACKING.contains(&key))
        })
        .collect();
    let mut out = path.to_owned();
    if !kept.is_empty() {
        out.push('?');
        out.push_str(&kept.join("&"));
    }
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(f);
    }
    Some(out)
}

/// Percent-decodes a link (%20 → space) when the result is valid text.
fn decode_link(text: &str) -> Option<String> {
    let link = single_link(text)?;
    if !link.contains('%') {
        return None;
    }
    let bytes = link.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(b) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?, 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).ok()
}

fn list_lines(text: &str) -> Option<Vec<&str>> {
    let items: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    (items.len() >= 2).then_some(items)
}

/// Strips an existing bullet or number so lists can be converted both ways.
fn strip_marker(line: &str) -> &str {
    let l = line.trim_start_matches(['-', '*', '•', '·']).trim_start();
    let digits = l.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && matches!(l[digits..].chars().next(), Some('.') | Some(')')) {
        l[digits + 1..].trim_start()
    } else {
        l
    }
}

fn bullets(text: &str) -> Option<String> {
    Some(list_lines(text)?.iter().map(|l| format!("- {}", strip_marker(l))).collect::<Vec<_>>().join("\n"))
}

fn numbered(text: &str) -> Option<String> {
    Some(list_lines(text)?.iter().enumerate().map(|(i, l)| format!("{}. {}", i + 1, strip_marker(l))).collect::<Vec<_>>().join("\n"))
}

fn quote(text: &str) -> Option<String> {
    let text = text.trim_end();
    (!text.is_empty()).then(|| text.lines().map(|l| if l.is_empty() { ">".to_owned() } else { format!("> {l}") }).collect::<Vec<_>>().join("\n"))
}

/// Ordinary words, worth changing the case of: not a link or JSON.
fn has_letters(text: &str) -> bool {
    text.chars().any(char::is_alphabetic) && single_link(text).is_none() && json_value(text).is_none()
}

fn upper(text: &str) -> Option<String> {
    has_letters(text).then(|| text.to_uppercase())
}

fn lower(text: &str) -> Option<String> {
    has_letters(text).then(|| text.to_lowercase())
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |c| c.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect())
}

fn title_case(text: &str) -> Option<String> {
    const SMALL: &[&str] = &["a", "an", "and", "as", "at", "but", "by", "for", "in", "of", "on", "or", "the", "to", "via"];
    if !has_letters(text) {
        return None;
    }
    let out = text
        .lines()
        .map(|line| {
            line.split(' ')
                .enumerate()
                .map(|(i, w)| if i > 0 && SMALL.contains(&w.to_lowercase().as_str()) { w.to_lowercase() } else { capitalize(w) })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(out)
}

fn sentence_case(text: &str) -> Option<String> {
    if !has_letters(text) {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut start = true;
    for c in text.chars() {
        if start && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            start = false;
        } else {
            out.extend(c.to_lowercase());
        }
        if matches!(c, '.' | '!' | '?' | '\n') {
            start = true;
        }
    }
    Some(out)
}

/// Words of a short phrase or identifier ("Hello world", "helloWorld",
/// "hello-world"), for the code-style cases. Long text doesn't fit.
fn words(text: &str) -> Option<Vec<String>> {
    let t = text.trim();
    if t.is_empty() || t.len() > 100 || t.contains('\n') || !has_letters(t) {
        return None;
    }
    let mut words = Vec::new();
    let mut word = String::new();
    let mut prev_lower = false;
    for c in t.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && prev_lower && !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            prev_lower = c.is_lowercase() || c.is_ascii_digit();
            word.extend(c.to_lowercase());
        } else {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            prev_lower = false;
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    (words.len() <= 12).then_some(words)
}

fn snake_case(text: &str) -> Option<String> {
    Some(words(text)?.join("_"))
}

fn kebab_case(text: &str) -> Option<String> {
    Some(words(text)?.join("-"))
}

fn camel_case(text: &str) -> Option<String> {
    let words = words(text)?;
    Some(words.iter().enumerate().map(|(i, w)| if i == 0 { w.clone() } else { capitalize(w) }).collect())
}

/// Decodes Base64 when the text looks like it and decodes to readable text.
fn base64_decode(text: &str) -> Option<String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let t = text.trim();
    if t.len() < 8 || !t.len().is_multiple_of(4) || t.contains(char::is_whitespace) {
        return None;
    }
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = Vec::new();
    for &b in t.trim_end_matches('=').as_bytes() {
        let v = ALPHABET.iter().position(|&a| a == b)? as u32;
        bits = (bits << 6) | v;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    let s = String::from_utf8(out).ok()?;
    s.chars().all(|c| !c.is_control() || c.is_whitespace()).then_some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Vec<&'static str> {
        available(text).into_iter().map(|(c, _)| c.id).collect()
    }

    #[test]
    fn joins_lines_from_pdfs() {
        let pdf = "Linux desktops have come a long way in the last ten\nyears, and the tools around them keep getting exam-\nples of good design worth sharing with others.\n\nThe second paragraph also wraps where the PDF\nhad its line breaks, in the middle of a sentence.";
        assert_eq!(
            join_lines(pdf).unwrap(),
            "Linux desktops have come a long way in the last ten years, and the tools around them keep getting \
             examples of good design worth sharing with others.\n\nThe second paragraph also wraps where the PDF had \
             its line breaks, in the middle of a sentence."
        );
        assert_eq!(join_lines("one paragraph only"), None);
        // Short lines (a list) aren't broken prose.
        assert_eq!(join_lines("milk\neggs\nbread"), None);
        // A dash between words isn't a hyphenation.
        assert_eq!(
            join_lines("a well-designed tool that is, well -\nknown among the people who use Linux every day").unwrap(),
            "a well-designed tool that is, well - known among the people who use Linux every day"
        );
    }

    #[test]
    fn formats_json() {
        assert_eq!(json_pretty(r#"{"b":1,"a":[1,2]}"#).unwrap(), "{\n  \"b\": 1,\n  \"a\": [\n    1,\n    2\n  ]\n}");
        assert_eq!(json_compact("{ \"a\" : 1 }").unwrap(), r#"{"a":1}"#);
        assert_eq!(json_pretty("42"), None);
        assert_eq!(json_pretty("not json"), None);
    }

    #[test]
    fn converts_spreadsheet_tables() {
        let tsv = "Name\tAge\nAna\t31\nLuis, Jr.\t40\n";
        assert_eq!(markdown_table(tsv).unwrap(), "| Name | Age |\n| --- | --- |\n| Ana | 31 |\n| Luis, Jr. | 40 |");
        assert_eq!(csv(tsv).unwrap(), "Name,Age\nAna,31\n\"Luis, Jr.\",40");
        let from_csv = "a,\"b, c\"\n1,2";
        assert_eq!(markdown_table(from_csv).unwrap(), "| a | b, c |\n| --- | --- |\n| 1 | 2 |");
        assert_eq!(csv(from_csv), None);
        assert_eq!(markdown_table("just\ntwo lines"), None);
    }

    #[test]
    fn cleans_links() {
        assert_eq!(
            clean_link("https://example.com/a?utm_source=x&id=7&fbclid=abc#top").unwrap(),
            "https://example.com/a?id=7#top"
        );
        assert_eq!(clean_link("https://example.com/a?utm_medium=y").unwrap(), "https://example.com/a");
        assert_eq!(clean_link("see https://example.com"), None);
        assert_eq!(decode_link("https://example.com/caf%C3%A9%20menu").unwrap(), "https://example.com/café menu");
    }

    #[test]
    fn changes_case() {
        assert_eq!(title_case("the lord of the rings").unwrap(), "The Lord of the Rings");
        assert_eq!(sentence_case("HELLO THERE. HOW ARE YOU?").unwrap(), "Hello there. How are you?");
        assert_eq!(snake_case("Hello World").unwrap(), "hello_world");
        assert_eq!(snake_case("parseHTTPResponse2").unwrap(), "parse_httpresponse2");
        assert_eq!(kebab_case("user_name").unwrap(), "user-name");
        assert_eq!(camel_case("user name id").unwrap(), "userNameId");
        assert_eq!(snake_case("a\nb"), None);
    }

    #[test]
    fn makes_lists() {
        assert_eq!(bullets("milk\neggs\n\n2. bread").unwrap(), "- milk\n- eggs\n- bread");
        assert_eq!(numbered("- milk\n- eggs").unwrap(), "1. milk\n2. eggs");
        assert_eq!(quote("hi\n\nthere").unwrap(), "> hi\n>\n> there");
    }

    #[test]
    fn decodes_base64() {
        assert_eq!(base64_decode("SGVsbG8sIExpbnV4IQ==").unwrap(), "Hello, Linux!");
        assert_eq!(base64_decode("not base64 at all"), None);
        assert_eq!(base64_decode("AAAAAAAA"), None);
    }

    #[test]
    fn offers_table_conversions_for_tables() {
        let ids = ids("Name\tAge\nAna\t31\nLuis\t40");
        assert_eq!(&ids[..3], &["plain", "markdown-table", "csv"]);
        assert!(!ids.contains(&"join-lines") && !ids.contains(&"one-line") && !ids.contains(&"tidy-spaces"));
    }

    #[test]
    fn offers_only_fitting_conversions() {
        let json = ids(r#"{"a":1}"#);
        assert_eq!(json[0], "plain");
        assert!(json.contains(&"json") && !json.contains(&"markdown-table"));
        let link = ids("https://example.com/?utm_source=x");
        assert!(link.contains(&"clean-link") && !link.contains(&"snake"));
        // Lowercase text isn't offered as lowercase again.
        assert!(!ids("already lower").contains(&"lower"));
    }
}
