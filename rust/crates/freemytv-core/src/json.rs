//! JSON written byte for byte as Python's `json.dumps` writes it, so the Rust tools make the same files as the
//! Python ones (`config.js`, `i18n.js`, `apps.json`, `zones.js`). serde_json's own writer differs in spacing and in
//! which characters it escapes.

use serde_json::Value;
use std::fmt::Write;

/// How Python was asked to write: `ensure_ascii`, `separators` and `indent`.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    /// `ensure_ascii=True`: everything outside printable ASCII as `\uXXXX`.
    pub ascii: bool,
    /// `separators=(',', ':')`; otherwise `(', ', ': ')`, or `(',', ': ')` with an indent.
    pub compact: bool,
    /// `indent=N`.
    pub indent: Option<usize>,
}

impl Style {
    /// `json.dumps(x)`.
    pub const DEFAULT: Style = Style {
        ascii: true,
        compact: false,
        indent: None,
    };
    /// `json.dumps(x, ensure_ascii=False)`.
    pub const UNICODE: Style = Style {
        ascii: false,
        compact: false,
        indent: None,
    };
    /// `json.dumps(x, ensure_ascii=False, separators=(',', ':'))`.
    pub const UNICODE_COMPACT: Style = Style {
        ascii: false,
        compact: true,
        indent: None,
    };
}

/// `json.dumps(value, ...)` with the given style.
pub fn dumps(value: &Value, style: Style) -> String {
    let mut out = String::new();
    write_value(&mut out, value, style, 0);
    out
}

fn write_value(out: &mut String, value: &Value, style: Style, level: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_string(out, s, style.ascii),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                separator(out, style, level + 1, i == 0);
                write_value(out, item, style, level + 1);
            }
            close(out, style, level);
            out.push(']');
        }
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                separator(out, style, level + 1, i == 0);
                write_string(out, k, style.ascii);
                out.push_str(if style.compact { ":" } else { ": " });
                write_value(out, v, style, level + 1);
            }
            close(out, style, level);
            out.push('}');
        }
    }
}

fn separator(out: &mut String, style: Style, level: usize, first: bool) {
    match style.indent {
        Some(n) => {
            if !first {
                out.push(',');
            }
            out.push('\n');
            out.extend(std::iter::repeat_n(' ', n * level));
        }
        None if !first => out.push_str(if style.compact { "," } else { ", " }),
        None => {}
    }
}

fn close(out: &mut String, style: Style, level: usize) {
    if let Some(n) = style.indent {
        out.push('\n');
        out.extend(std::iter::repeat_n(' ', n * level));
    }
}

/// A JSON string as Python escapes it (lowercase hex, surrogate pairs above U+FFFF).
pub fn write_string(out: &mut String, s: &str, ascii: bool) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c if ascii && !(' '..='~').contains(&c) => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{:04x}", unit);
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_spacing() {
        assert_eq!(
            dumps(&json!({"a": [1, true, null], "b": "x"}), Style::DEFAULT),
            r#"{"a": [1, true, null], "b": "x"}"#
        );
    }

    #[test]
    fn compact() {
        assert_eq!(
            dumps(&json!({"a": [1, 2], "b": {}}), Style::UNICODE_COMPACT),
            r#"{"a":[1,2],"b":{}}"#
        );
    }

    #[test]
    fn indent_like_python() {
        let v = json!({"a": [1, {"b": []}], "c": "d"});
        let want = "{\n  \"a\": [\n    1,\n    {\n      \"b\": []\n    }\n  ],\n  \"c\": \"d\"\n}";
        assert_eq!(
            dumps(
                &v,
                Style {
                    ascii: true,
                    compact: false,
                    indent: Some(2)
                }
            ),
            want
        );
    }

    #[test]
    fn escapes() {
        let v = json!("São \"q\" \\ \n\u{1}\u{7f} 😀");
        assert_eq!(
            dumps(&v, Style::DEFAULT),
            r#""S\u00e3o \"q\" \\ \n\u0001\u007f \ud83d\ude00""#
        );
        assert_eq!(
            dumps(&v, Style::UNICODE),
            "\"São \\\"q\\\" \\\\ \\n\\u0001\u{7f} 😀\""
        );
    }

    #[test]
    fn keeps_key_order() {
        let v: Value = serde_json::from_str(r#"{"z": 1, "a": 2}"#).unwrap();
        assert_eq!(dumps(&v, Style::DEFAULT), r#"{"z": 1, "a": 2}"#);
    }
}
