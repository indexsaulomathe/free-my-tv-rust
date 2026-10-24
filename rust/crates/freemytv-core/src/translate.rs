//! `python3 tools/translate.py --check`: the keys from the code (Java and web) against every catalogue in
//! `app/assets/i18n/`. Same checks and the same messages (keys written as Python's `repr`), so either tool can
//! run in CI. `--fill`, `--prune` and `--new` stay in Python for now.

use crate::i18n::{self, call_args, literal_value};
use crate::Error;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

static CALL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bUi\.(trf|tr|plural|datePattern)\(").unwrap());
static PLACEHOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{[a-zA-Z_][a-zA-Z_0-9]*\}").unwrap());
static PLURAL_FORM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(zero|one|two|few|many|other):").unwrap());

/// One translatable call: where it is, which function, and its arguments' values (None when not a literal).
pub struct Call {
    pub path: String,
    pub line: usize,
    pub kind: String,
    pub args: Vec<Option<String>>,
}

fn line_of(text: &str, byte: usize) -> usize {
    text[..byte].matches('\n').count() + 1
}

/// `Ui.tr`, `Ui.trf`, `Ui.plural` and `Ui.datePattern` calls in one Java file (`java_calls`).
pub fn java_calls_in(path: &str, text: &str) -> Vec<Call> {
    let chars: Vec<char> = text.chars().collect();
    let starts: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
    let mut out = Vec::new();
    for m in CALL.captures_iter(text) {
        let whole = m.get(0).unwrap();
        let line_start = text[..whole.start()].rfind('\n').map_or(0, |p| p + 1);
        let prefix = text[line_start..whole.start()].trim_start();
        if prefix.starts_with('*') || prefix.starts_with("//") || prefix.starts_with("/*") {
            continue; // an example in a comment
        }
        let at = starts.binary_search(&whole.end()).unwrap_or(chars.len());
        let Some((args, _)) = call_args(&chars, at) else {
            continue;
        };
        out.push(Call {
            path: path.to_string(),
            line: line_of(text, whole.start()),
            kind: m[1].to_string(),
            args: args.iter().map(|a| literal_value(a, &['"'])).collect(),
        });
    }
    out
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<Call>) -> Result<(), Error> {
    for name in crate::sorted_names(dir)? {
        let path = dir.join(&name);
        if path.is_dir() {
            walk(&path, root, out)?;
        } else if name.ends_with(".java") {
            let text =
                fs::read_to_string(&path).map_err(|e| Error(format!("{}: {e}", path.display())))?;
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            out.extend(java_calls_in(&rel, &text));
        }
    }
    Ok(())
}

/// Every call in `app/src` and `web/` (`calls`).
pub fn calls(root: &Path) -> Result<Vec<Call>, Error> {
    let mut out = Vec::new();
    let src = root.join("app").join("src");
    if src.is_dir() {
        walk(&src, root, &mut out)?;
    }
    let web = root.join("web");
    for name in crate::sorted_names(&web)? {
        if !name.ends_with(".js") || name == "strings.js" {
            continue;
        }
        let text =
            fs::read_to_string(web.join(&name)).map_err(|e| Error(format!("{name}: {e}")))?;
        for (line, kind, args) in i18n::web_calls_at(&text) {
            out.push(Call {
                path: format!("web/{name}"),
                line,
                kind,
                args,
            });
        }
    }
    Ok(out)
}

pub fn english_of(key: &str) -> &str {
    key.split_once('|').map_or(key, |(_, t)| t)
}

/// {key: English} from the code, and a warning for each call whose text isn't a literal (`extract`).
pub fn extract(root: &Path) -> Result<(BTreeMap<String, String>, Vec<String>), Error> {
    let mut keys = BTreeMap::new();
    let mut warnings = Vec::new();
    for c in calls(root)? {
        match c.args.into_iter().next().flatten() {
            Some(key) => {
                let english = english_of(&key).to_string();
                keys.insert(key, english);
            }
            None => warnings.push(format!(
                "warning: {}:{}: {} with text that isn't a string literal",
                c.path, c.line, c.kind
            )),
        }
    }
    Ok((keys, warnings))
}

pub fn placeholders(text: &str) -> BTreeSet<String> {
    PLACEHOLDER
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// SimpleDateFormat letters outside quotes; doubled quotes are literal quotes (`valid_format`).
pub fn valid_format(text: &str) -> bool {
    let c: Vec<char> = text.chars().collect();
    let (mut quoted, mut i) = (false, 0);
    while i < c.len() {
        if c[i] == '\'' {
            if c.get(i + 1) == Some(&'\'') {
                i += 2;
                continue;
            }
            quoted = !quoted;
        } else if !quoted && c[i].is_ascii_alphabetic() {
            if !"GyYMLwWDdFEuaHkKhmsSzZX".contains(c[i]) {
                return false;
            }
            if c[i] == 'X' && c[i..].starts_with(&['X', 'X', 'X', 'X']) {
                return false;
            }
        }
        i += 1;
    }
    !quoted
}

/// The translation keeps the English placeholders; a plural is forms like "one:…|other:…", each with them (`fits`).
pub fn fits(key: &str, english: &str, text: &str) -> bool {
    if key.starts_with("format|") && !valid_format(text) {
        return false;
    }
    if !key.starts_with("plural|") {
        return placeholders(text) == placeholders(english);
    }
    let want = placeholders(english);
    let forms: Vec<&str> = text.split('|').collect();
    forms
        .iter()
        .all(|f| PLURAL_FORM.is_match(f) && placeholders(f) == want)
        && forms.iter().all(|f| f.contains("{n}"))
        && forms.iter().any(|f| f.starts_with("other:"))
}

/// A string written as Python's `repr` writes it, so the messages match the Python's.
pub fn py_repr(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(q);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == q => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            c if c.is_control() || c == '\u{2028}' || c == '\u{2029}' || c == '\u{ad}' => {
                let n = c as u32;
                if n <= 0xff {
                    out.push_str(&format!("\\x{n:02x}"));
                } else if n <= 0xffff {
                    out.push_str(&format!("\\u{n:04x}"));
                } else {
                    out.push_str(&format!("\\U{n:08x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push(q);
    out
}

/// Every problem in every catalogue, in the Python's order (`check`).
pub fn check(i18n_dir: &Path, en: &BTreeMap<String, String>) -> Result<Vec<String>, Error> {
    let mut errors = Vec::new();
    let mut langs = vec!["en".to_string()];
    langs.extend(i18n::languages(i18n_dir)?);
    for lang in langs {
        let path = i18n_dir.join(format!("{lang}.json"));
        let parsed: Result<Value, String> = fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str(&t).map_err(|e| e.to_string()));
        let cat: Map<String, Value> = match parsed {
            Err(e) => {
                errors.push(format!("{lang}: invalid JSON: {e}"));
                continue;
            }
            Ok(Value::Object(m)) => m,
            Ok(_) => {
                errors.push(format!("{lang}: catalogue must be a JSON object"));
                continue;
            }
        };
        for key in en.keys().filter(|k| !cat.contains_key(*k)) {
            errors.push(format!("{lang}: missing key: {}", py_repr(key)));
        }
        for key in cat.keys().filter(|k| !en.contains_key(*k)) {
            errors.push(format!("{lang}: obsolete key: {}", py_repr(key)));
        }
        for (key, value) in &cat {
            let r = py_repr(key);
            match value {
                Value::String(v) => {
                    if key.starts_with("format|") && !valid_format(v) {
                        errors.push(format!("{lang}: invalid format: {r}: {}", py_repr(v)));
                    } else if key.starts_with("plural|") && !fits(key, english_of(key), v) {
                        errors.push(format!("{lang}: invalid plural forms: {r}"));
                    } else if en
                        .get(key)
                        .is_some_and(|e| placeholders(v) != placeholders(e))
                    {
                        errors.push(format!("{lang}: placeholder mismatch: {r}"));
                    }
                    if lang == "en" && en.get(key).is_some_and(|e| e != v) {
                        errors.push(format!("en: out-of-date value: {r}"));
                    }
                }
                _ => {
                    errors.push(format!("{lang}: non-string value: {r}"));
                    // Python compares the value with en[key] even when it isn't a string.
                    if lang == "en" && en.contains_key(key) {
                        errors.push(format!("en: out-of-date value: {r}"));
                    }
                }
            }
        }
    }
    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java() {
        let java = "class A {\n  String a = Ui.tr(\"Live TV\");\n  // Ui.tr(\"comment\")\n  \
                    String b = Ui.trf(\"{n} min \" + \"ago\", \"n\", 1);\n  String c = Ui.tr(name);\n  \
                    String d = Ui.plural(\"plural|one:{n} a|other:{n} b\", n); String e = MyUi.tr(\"no\");\n}";
        let calls = java_calls_in("A.java", java);
        let keys: Vec<_> = calls
            .iter()
            .map(|c| (c.line, c.kind.as_str(), c.args[0].clone()))
            .collect();
        assert_eq!(
            keys,
            vec![
                (2, "tr", Some("Live TV".into())),
                (4, "trf", Some("{n} min ago".into())),
                (5, "tr", None),
                (6, "plural", Some("plural|one:{n} a|other:{n} b".into())),
            ]
        );
    }

    #[test]
    fn formats() {
        assert!(valid_format("h:mm a"));
        assert!(valid_format("HH'h'mm"));
        assert!(valid_format("d 'de' MMMM"));
        assert!(valid_format("h 'o''clock'"));
        assert!(!valid_format("h:mm 'a"));
        assert!(!valid_format("bb"));
        assert!(!valid_format("XXXX"));
        assert!(valid_format("XXX"));
        assert!(valid_format("H時mm分"));
    }

    #[test]
    fn plurals_and_placeholders() {
        let key = "plural|one:{n} season|other:{n} seasons";
        assert!(fits(
            key,
            english_of(key),
            "one:{n} сезон|few:{n} сезона|many:{n} сезонов|other:{n} сезона"
        ));
        assert!(fits(key, english_of(key), "other:{n}シーズン"));
        assert!(!fits(key, english_of(key), "one:{n} temporada"));
        assert!(!fits(
            key,
            english_of(key),
            "one:temporada|other:{n} temporadas"
        ));
        assert!(!fits(key, english_of(key), "un:{n} a|other:{n} b"));
        assert!(fits("{n} min ago", "{n} min ago", "há {n} min"));
        assert!(!fits("{n} min ago", "{n} min ago", "há {m} min"));
    }

    #[test]
    fn repr_like_python() {
        assert_eq!(py_repr("Live TV"), "'Live TV'");
        assert_eq!(py_repr("it's"), "\"it's\"");
        assert_eq!(py_repr("it's \"x\""), "'it\\'s \"x\"'");
        assert_eq!(py_repr("a\nb\\"), "'a\\nb\\\\'");
        assert_eq!(py_repr("São Paulo · 東京"), "'São Paulo · 東京'");
    }
}
