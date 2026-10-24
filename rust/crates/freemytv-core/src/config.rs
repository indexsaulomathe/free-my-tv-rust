//! `config.properties`: the person's settings, as `tools/webapp.py` reads them (`read_config`, `GENERIC`).

use crate::Error;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// The settings the LG and Samsung builds put in `config.js`, in Python's key order.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub tmdb_key: String,
    pub tmdb_region: String,
    pub latitude: String,
    pub longitude: String,
    pub city: String,
    pub language: String,
    pub currency: String,
    pub temperature: String,
    pub show_crypto: bool,
    pub hide_animation: bool,
    pub hide_apps: Vec<String>,
}

impl Config {
    /// `--generic`: no key and no city; the TV's time zone picks the city, the currency and the region.
    pub fn generic() -> Config {
        Config {
            tmdb_key: String::new(),
            tmdb_region: String::new(),
            latitude: String::new(),
            longitude: String::new(),
            city: String::new(),
            language: "auto".into(),
            currency: String::new(),
            temperature: "auto".into(),
            show_crypto: true,
            hide_animation: false,
            hide_apps: Vec::new(),
        }
    }

    /// Parses the text of a `config.properties`.
    pub fn parse(text: &str) -> Result<Config, Error> {
        let values = properties(text);
        let get = |k: &str| values.get(k).map(String::as_str).unwrap_or("");
        let or = |k: &str, d: &str| -> String {
            let v = get(k);
            if v.is_empty() {
                d.to_string()
            } else {
                v.to_string()
            }
        };
        let number = |k: &str, d: &str| -> Result<String, Error> {
            let v = or(k, d);
            if is_number(&v) {
                Ok(v)
            } else {
                Err(Error(format!(
                    "config.properties: {k} must be a number (got \"{v}\")"
                )))
            }
        };
        Ok(Config {
            tmdb_key: get("tmdb_key").to_string(),
            tmdb_region: or("tmdb_region", "US"),
            latitude: number("latitude", "40.7128")?,
            longitude: number("longitude", "-74.0060")?,
            city: get("city").to_string(),
            language: or("language", "auto"),
            currency: or("currency", "USD"),
            temperature: or("temperature", "auto"),
            show_crypto: values
                .get("show_crypto")
                .map_or("true", |s| s.as_str())
                .to_lowercase()
                != "false",
            hide_animation: values
                .get("hide_animation")
                .map_or("false", |s| s.as_str())
                .to_lowercase()
                == "true",
            hide_apps: get("hide_apps")
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
        })
    }

    /// Reads `path`, creating it from `config.example.properties` (next to it in the repository) when missing.
    pub fn read(path: &Path, example: &Path) -> Result<Config, Error> {
        if !path.exists() {
            fs::copy(example, path)
                .map_err(|e| Error(format!("can't create {}: {e}", path.display())))?;
            eprintln!("Created config.properties: add your TMDB key and your city there (optional), then build again.");
        }
        let text = fs::read_to_string(path)
            .map_err(|e| Error(format!("can't read {}: {e}", path.display())))?;
        Config::parse(&text)
    }

    /// The settings as `window.CONFIG` (the same keys and order as Python's dict).
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("tmdbKey".into(), json!(self.tmdb_key));
        m.insert("tmdbRegion".into(), json!(self.tmdb_region));
        m.insert("latitude".into(), json!(self.latitude));
        m.insert("longitude".into(), json!(self.longitude));
        m.insert("city".into(), json!(self.city));
        m.insert("language".into(), json!(self.language));
        m.insert("currency".into(), json!(self.currency));
        m.insert("temperature".into(), json!(self.temperature));
        m.insert("showCrypto".into(), json!(self.show_crypto));
        m.insert("hideAnimation".into(), json!(self.hide_animation));
        m.insert("hideApps".into(), json!(self.hide_apps));
        Value::Object(m)
    }
}

/// `key=value` lines; `#` comments and lines without `=` are skipped; values lose surrounding spaces and quotes.
pub fn properties(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    out
}

/// `^-?\d+(\.\d+)?$`
fn is_number(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s, None),
    };
    let digits = |p: &str| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit());
    digits(int) && frac.is_none_or(digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = Config::parse("").unwrap();
        assert_eq!(c.tmdb_region, "US");
        assert_eq!(
            (c.latitude.as_str(), c.longitude.as_str()),
            ("40.7128", "-74.0060")
        );
        assert!(c.show_crypto && !c.hide_animation && c.hide_apps.is_empty());
    }

    #[test]
    fn values() {
        let c = Config::parse("# x\nnope\ntmdb_key = \"abc\"\ncity=São Paulo\nshow_crypto=FALSE\nhide_animation=True\nhide_apps= a , ,b\nurl=a=b\n").unwrap();
        assert_eq!(c.tmdb_key, "abc");
        assert_eq!(c.city, "São Paulo");
        assert!(!c.show_crypto && c.hide_animation);
        assert_eq!(c.hide_apps, vec!["a", "b"]);
    }

    #[test]
    fn numbers() {
        for ok in ["1", "-1", "1.5", "-23.55"] {
            assert!(is_number(ok), "{ok}");
        }
        for bad in ["", "abc", "1,5", "--1", "1.", ".5", "-"] {
            assert!(!is_number(bad), "{bad}");
        }
        assert!(Config::parse("latitude=abc").is_err());
    }

    #[test]
    fn json_order() {
        let keys: Vec<String> = match Config::generic().to_json() {
            Value::Object(m) => m.keys().cloned().collect(),
            _ => unreachable!(),
        };
        assert_eq!(keys[0], "tmdbKey");
        assert_eq!(keys[10], "hideApps");
    }
}
