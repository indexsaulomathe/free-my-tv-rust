//! The translation catalogues the web app needs: the keys `web/*.js` uses (`tr`, `trf`, `plural`, `formatDate`), and
//! each language's texts for them. A port of `web_calls`, `java_args`, `java_string` and `web_keys` in
//! `tools/translate.py`, and `catalogues` in `tools/webapp.py`.

use crate::Error;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const NAMES: [&str; 4] = ["trf", "tr", "plural", "formatDate"];

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Every translatable call in one web file: the value of each call's arguments (None when not a string literal).
pub fn web_calls(text: &str) -> Vec<Vec<Option<String>>> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let Some(name) = NAMES
            .iter()
            .find(|n| starts_with(&chars, i, n) && chars.get(i + n.len()) == Some(&'('))
        else {
            i += 1;
            continue;
        };
        let end = i + name.len() + 1;
        // (?<![\w.$]) or (?<=\bI18N\.)
        let free = i == 0 || !(is_word(chars[i - 1]) || chars[i - 1] == '.' || chars[i - 1] == '$');
        let on_i18n =
            i >= 5 && starts_with(&chars, i - 5, "I18N.") && (i == 5 || !is_word(chars[i - 6]));
        if !(free || on_i18n) {
            i += 1;
            continue;
        }
        let line_start = chars[..i]
            .iter()
            .rposition(|&c| c == '\n')
            .map_or(0, |p| p + 1);
        let prefix: String = chars[line_start..i]
            .iter()
            .collect::<String>()
            .trim_start()
            .to_string();
        if prefix.starts_with('*')
            || prefix.starts_with("//")
            || prefix.starts_with("/*")
            || prefix.ends_with("function ")
        {
            i = end;
            continue;
        }
        if let Some((args, _)) = call_args(&chars, end) {
            out.push(args.iter().map(|a| string_value(a)).collect());
        }
        i = end;
    }
    out
}

fn starts_with(chars: &[char], at: usize, s: &str) -> bool {
    s.chars()
        .enumerate()
        .all(|(k, c)| chars.get(at + k) == Some(&c))
}

/// The raw arguments of the call whose `(` ends at `start`, and the index of its `)` (`java_args`).
pub fn call_args(chars: &[char], start: usize) -> Option<(Vec<String>, usize)> {
    let (mut args, mut depth, mut i, mut cur) = (Vec::new(), 0usize, start, String::new());
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' || ch == '\'' {
            let mut j = i + 1;
            while *chars.get(j)? != ch {
                j += if chars[j] == '\\' { 2 } else { 1 };
            }
            cur.extend(&chars[i..=j]);
            i = j + 1;
            continue;
        }
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    args.push(cur.trim().to_string());
                    return Some((args, i));
                }
                depth -= 1;
            }
            ',' if depth == 0 => {
                args.push(cur.trim().to_string());
                cur.clear();
                i += 1;
                continue;
            }
            _ => {}
        }
        cur.push(ch);
        i += 1;
    }
    None
}

/// The value of an argument made only of string literals ('…' or "…") joined with `+`, else None (`java_string`).
pub fn string_value(arg: &str) -> Option<String> {
    let chars: Vec<char> = arg.trim().chars().collect();
    let mut raw = String::new();
    let mut i = 0;
    while i < chars.len() {
        let q = chars[i];
        if q != '"' && q != '\'' {
            return None;
        }
        let mut j = i + 1;
        loop {
            match chars.get(j) {
                None => return None,
                Some('\\') if j + 1 < chars.len() => j += 2,
                Some('\\') => return None,
                Some(&c) if c == q => break,
                Some(_) => j += 1,
            }
        }
        raw.extend(&chars[i + 1..j]);
        i = j + 1;
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i < chars.len() {
            if chars[i] != '+' {
                return None;
            }
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
        }
    }
    Some(unescape(&raw))
}

/// `\uXXXX`, `\n`, `\t`, and `\x` → x.
fn unescape(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' || i + 1 >= chars.len() {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let c = chars[i + 1];
        if c == 'u' && i + 6 <= chars.len() {
            let hex: String = chars[i + 2..i + 6].iter().collect();
            if hex.chars().all(|h| h.is_ascii_hexdigit()) {
                if let Some(ch) = char::from_u32(u32::from_str_radix(&hex, 16).unwrap()) {
                    out.push(ch);
                }
                i += 6;
                continue;
            }
        }
        out.push(match c {
            'n' => '\n',
            't' => '\t',
            other => other,
        });
        i += 2;
    }
    out
}

/// The keys `web/` uses, plus the plain text of each "context|Text" key (`web_keys`). strings.js is skipped.
pub fn web_keys(web: &Path) -> Result<BTreeSet<String>, Error> {
    let mut keys = BTreeSet::new();
    for name in crate::sorted_names(web)? {
        if !name.ends_with(".js") || name == "strings.js" {
            continue;
        }
        let text =
            fs::read_to_string(web.join(&name)).map_err(|e| Error(format!("{name}: {e}")))?;
        for args in web_calls(&text) {
            if let Some(Some(key)) = args.first() {
                if let Some((_, plain)) = key.split_once('|') {
                    keys.insert(plain.to_string());
                }
                keys.insert(key.clone());
            }
        }
    }
    Ok(keys)
}

/// The languages with a catalogue, without English (`languages`).
pub fn languages(i18n: &Path) -> Result<Vec<String>, Error> {
    Ok(crate::sorted_names(i18n)?
        .into_iter()
        .filter(|n| n.ends_with(".json") && n != "en.json")
        .map(|n| n[..n.len() - 5].to_string())
        .collect())
}

/// {lang: {key: text}} with only the keys the web app uses, sorted (`catalogues` in webapp.py).
pub fn catalogues(i18n: &Path, web: &Path) -> Result<Value, Error> {
    let keys = web_keys(web)?;
    let mut out = Map::new();
    for lang in languages(i18n)? {
        let text = fs::read_to_string(i18n.join(format!("{lang}.json")))
            .map_err(|e| Error(format!("{lang}.json: {e}")))?;
        let cat: Map<String, Value> =
            serde_json::from_str(&text).map_err(|e| Error(format!("{lang}.json: {e}")))?;
        let mut pairs: Vec<(String, Value)> =
            cat.into_iter().filter(|(k, _)| keys.contains(k)).collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        out.insert(lang, Value::Object(pairs.into_iter().collect()));
    }
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(js: &str) -> Vec<Option<String>> {
        web_calls(js)
            .into_iter()
            .map(|a| a.into_iter().next().flatten())
            .collect()
    }

    #[test]
    fn finds_calls() {
        let js = "a = tr('Live TV'); b = I18N.trf(\"{n} min\", {n: 1}); c = x.tr('no'); d = $tr('no');\n\
                  // tr('comment')\n  * tr('doc')\nfunction tr(english) {}\ne = plural('plural|one:{n} a|other:{n} b', n);\n\
                  f = formatDate('format|h:mm a', at); g = tr(name); h = mytr('no');";
        assert_eq!(
            keys(js),
            vec![
                Some("Live TV".into()),
                Some("{n} min".into()),
                Some("plural|one:{n} a|other:{n} b".into()),
                Some("format|h:mm a".into()),
                None,
            ]
        );
    }

    #[test]
    fn literals() {
        assert_eq!(string_value("'a' + \"b\""), Some("ab".into()));
        assert_eq!(string_value("'it\\'s'"), Some("it's".into()));
        assert_eq!(string_value("'\\u00e3\\n'"), Some("ã\n".into()));
        assert_eq!(string_value("'a' + b"), None);
        assert_eq!(string_value("x"), None);
    }

    #[test]
    fn args() {
        let chars: Vec<char> = "f(\"a, b\", g(1, 2), [3]) rest".chars().collect();
        let (args, end) = call_args(&chars, 2).unwrap();
        assert_eq!(args, vec!["\"a, b\"", "g(1, 2)", "[3]"]);
        assert_eq!(chars[end], ')');
        let open: Vec<char> = "f('a', (b".chars().collect();
        assert!(call_args(&open, 2).is_none());
    }
}
