//! The web app's pure logic, from `web/data.js` and `web/strings.js`, with the same results as the JavaScript.
//!
//! Compiled to WebAssembly (`freemytv make-logic` → `web/logic_wasm.js`) and called through `web/logic.js` on TVs
//! that run it (Chromium 57+: webOS 5+, Tizen 5+). Older TVs, and every TV while the module loads, use the JavaScript.
//! The calls (`ffi`) take and give UTF-8 strings in the module's memory; a failure gives no answer, and the
//! JavaScript does that call instead.

use serde_json::Value;

pub mod ffi;

/// "2026-10-20T18:31" at the location (Open-Meteo's local time) → epoch ms, as `Date.UTC(...) - offset * 1000`.
pub fn local_time(iso: &str, offset_seconds: f64) -> f64 {
    let mut nums = [0f64; 5];
    let b = iso.as_bytes();
    let mut i = 0;
    for (k, sep) in [b'-', b'-', b'T', b':', 0].iter().enumerate() {
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return 0.0;
        }
        nums[k] = iso[start..i].parse::<f64>().unwrap_or(f64::NAN);
        if *sep != 0 {
            if b.get(i) != Some(sep) {
                return 0.0;
            }
            i += 1;
        }
    }
    date_utc(nums[0], nums[1] - 1.0, nums[2], nums[3], nums[4]) - offset_seconds * 1000.0
}

/// `Date.UTC(year, month, day, hours, minutes)`: months overflow into years, years 0-99 are 1900-1999.
pub fn date_utc(year: f64, month: f64, day: f64, hours: f64, minutes: f64) -> f64 {
    if ![year, month, day, hours, minutes]
        .iter()
        .all(|v| v.is_finite())
    {
        return f64::NAN;
    }
    let mut y = year.trunc();
    if (0.0..=99.0).contains(&y) {
        y += 1900.0;
    }
    let m = month.trunc();
    let y = y + (m / 12.0).floor();
    let m = m.rem_euclid(12.0);
    let days = days_from_civil(y as i64, m as i64 + 1, 1) as f64 + day.trunc() - 1.0;
    let ms = days * 86_400_000.0 + hours.trunc() * 3_600_000.0 + minutes.trunc() * 60_000.0;
    if ms.abs() > 8.64e15 {
        f64::NAN
    } else {
        ms
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// JavaScript's `localTime(iso || '', off)` takes anything: only a string can match.
fn local_time_of(v: Option<&Value>, off: f64) -> f64 {
    match v {
        Some(Value::String(s)) => local_time(s, off),
        _ => 0.0,
    }
}

/// `parseWeather`: Open-Meteo's answer → {temp, code, isDay, max, min, sunrise, sunset, hours}, or null.
/// None where the JavaScript would throw (a missing daily or hourly list): the caller runs the JavaScript then.
pub fn parse_weather(o: &Value, now: f64) -> Option<Value> {
    let Some(current) = o.get("current").filter(|c| truthy(c)) else {
        return Some(Value::Null);
    };
    let off = match o.get("utc_offset_seconds") {
        None | Some(Value::Null) => 0.0,
        Some(Value::Number(n)) => n.as_f64()?,
        Some(v) if !truthy(v) => 0.0,
        Some(_) => return None, // a string offset: coerced by JavaScript; leave it to it
    };
    let daily = o.get("daily")?;
    // A list's first item; Some(None) when the list is empty (undefined in JavaScript, left out of the JSON).
    let first = |name: &str| -> Option<Option<Value>> {
        match daily.get(name)? {
            Value::Array(a) => Some(a.first().cloned()),
            _ => None,
        }
    };
    let mut w = serde_json::Map::new();
    let mut put = |k: &str, v: Option<Value>| {
        if let Some(v) = v {
            w.insert(k.into(), v);
        }
    };
    put("temp", current.get("temperature_2m").cloned());
    put("code", current.get("weather_code").cloned());
    put("isDay", Some(Value::Bool(!is_zero(current.get("is_day")))));
    put("max", first("temperature_2m_max")?);
    put("min", first("temperature_2m_min")?);
    put(
        "sunrise",
        Some(num(local_time_of(first("sunrise")?.as_ref(), off))),
    );
    put(
        "sunset",
        Some(num(local_time_of(first("sunset")?.as_ref(), off))),
    );
    let mut hours = Vec::new();
    if let Some(hr) = o.get("hourly").filter(|h| truthy(h)) {
        let times = hr.get("time")?.as_array()?;
        let list = |name: &str| hr.get(name).and_then(Value::as_array);
        let temps = list("temperature_2m")?;
        let codes = list("weather_code")?;
        let is_day = hr.get("is_day").filter(|d| truthy(d));
        for (i, t) in times.iter().enumerate() {
            if hours.len() >= 6 {
                break;
            }
            let at = local_time_of(Some(t), off);
            if at <= now {
                continue;
            }
            let day = match is_day {
                None => true,
                Some(Value::Array(a)) => !is_zero(a.get(i)),
                Some(_) => true, // a non-list is_day: every hour reads as day, as `hr.is_day[i] !== 0` does
            };
            let mut h = serde_json::Map::new();
            h.insert("at".into(), num(at));
            if let Some(t) = temps.get(i) {
                h.insert("temp".into(), t.clone());
            }
            if let Some(c) = codes.get(i) {
                h.insert("code".into(), c.clone());
            }
            h.insert("day".into(), Value::Bool(day));
            hours.push(Value::Object(h));
        }
    }
    w.insert("hours".into(), Value::Array(hours));
    Some(Value::Object(w))
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// `x !== 0`'s opposite: only the number zero.
fn is_zero(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Number(n)) if n.as_f64() == Some(0.0))
}

fn num(f: f64) -> Value {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 9.0e15 {
        Value::from(f as i64)
    } else {
        serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

/// The English key of a WMO weather code's words (`condition`); the JavaScript translates it. Empty: none.
pub fn condition(code: f64, day: bool) -> &'static str {
    // NaN goes on like JavaScript's comparisons do, down to "Rain".
    if code < 0.0 {
        return "";
    }
    let c = code;
    let is = |v: f64| c == v;
    if is(0.0) {
        return if day { "Clear" } else { "night|Clear" };
    }
    if is(1.0) {
        return if day { "Mostly Sunny" } else { "Mostly Clear" };
    }
    if is(2.0) {
        return "Partly Cloudy";
    }
    if is(3.0) {
        return "Cloudy";
    }
    if is(45.0) || is(48.0) {
        return "Fog";
    }
    if (51.0..=57.0).contains(&c) {
        return "Drizzle";
    }
    if is(65.0) || is(67.0) || is(82.0) {
        return "Heavy Rain";
    }
    if (71.0..=77.0).contains(&c) || is(85.0) || is(86.0) {
        return "Snow";
    }
    if c >= 95.0 {
        return "Thunderstorms";
    }
    "Rain"
}

/// The symbol's name for a WMO weather code (`symbol`).
pub fn symbol(code: f64, day: bool) -> &'static str {
    let c = code;
    if c <= 1.0 {
        return if day { "sunny" } else { "clear_night" };
    }
    if c == 2.0 {
        return if day {
            "partly_cloudy_day"
        } else {
            "partly_cloudy_night"
        };
    }
    if c == 3.0 {
        return "cloud";
    }
    if c == 45.0 || c == 48.0 {
        return "foggy";
    }
    if (71.0..=77.0).contains(&c) || c == 85.0 || c == 86.0 {
        return "snowflake";
    }
    if c >= 95.0 {
        return "thunderstorm";
    }
    "rainy"
}

/// A title's franchise, so one hour doesn't show two of the same: lowercased, before ":" or " - " (`franchise`).
pub fn franchise(title: &str) -> String {
    let t = js_lower(title);
    let cut = t.find(':').or_else(|| t.find(" - "));
    let head = match cut {
        Some(c) if c > 0 => &t[..c],
        _ => &t[..],
    };
    head.trim_matches(js_space).to_string()
}

/// `toLowerCase` as JavaScript does it (Rust's agrees except for a final sigma, which JavaScript also handles).
fn js_lower(s: &str) -> String {
    s.to_lowercase()
}

/// `\s` in JavaScript: Unicode white space and the BOM.
fn js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// The streaming services as TMDB names them (part of the name, lowercase) and our keys (`SERVICES`).
pub const SERVICES: [(&str, &str); 11] = [
    ("netflix", "netflix"),
    ("prime video", "prime"),
    ("disney", "disney"),
    ("hulu", "hulu"),
    ("paramount", "paramount"),
    ("peacock", "peacock"),
    ("globoplay", "globoplay"),
    ("max", "max"),
    ("apple tv", "appletv"),
    ("hotstar", "hotstar"),
    ("crunchyroll", "crunchyroll"),
];

/// The index in SERVICES of a TMDB provider, or None (`service`): "… Amazon Channel" is sold inside another app,
/// and Apple TV is only the streaming service (350; 2 is the store).
pub fn service(provider_name: &str, provider_id: f64) -> Option<usize> {
    let n = js_lower(provider_name);
    if n.contains(" channel") {
        return None;
    }
    SERVICES.iter().position(|(name, key)| {
        if *key == "appletv" && provider_id != 350.0 {
            return false;
        }
        if *name == "max" {
            has_word(&n, "max")
        } else {
            n.contains(name)
        }
    })
}

/// `/\bword\b/` with JavaScript's ASCII word characters.
fn has_word(text: &str, word: &str) -> bool {
    let w = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    text.match_indices(word).any(|(i, _)| {
        !w(text[..i].chars().next_back()) && !w(text[i + word.len()..].chars().next())
    })
}

/// The best backdrop: no text over it, the largest, then the best rated (`bestBackdrop`). None: no answer.
pub fn best_backdrop(images: &Value) -> Option<Value> {
    let list = images.get("backdrops").filter(|l| truthy(l));
    let Some(list) = list else {
        return Some(Value::Null);
    };
    let list = list.as_array()?;
    let (mut best, mut score) = (Value::Null, -1.0f64);
    for b in list {
        let path = b.get("file_path").cloned().unwrap_or(Value::Null);
        if !truthy(&path) {
            continue;
        }
        let number = |name: &str| -> Option<f64> {
            match b.get(name) {
                None | Some(Value::Null) => Some(0.0),
                Some(Value::Number(n)) => n.as_f64(),
                Some(v) if !truthy(v) => Some(0.0),
                Some(_) => None,
            }
        };
        let no_text = matches!(b.get("iso_639_1"), Some(Value::Null));
        let s = if no_text { 1e7 } else { 0.0 }
            + number("width")?.min(3840.0) * 100.0
            + number("vote_average")?;
        if s > score {
            score = s;
            best = path;
        }
    }
    Some(best)
}

// MARK: - strings.js

/// The CLDR plural category of a whole number where the TV has no Intl.PluralRules (`PLURALS`, `slavic`).
pub fn plural_category(lang: &str, signed: f64) -> &'static str {
    let n = signed.abs();
    let t = n % 10.0;
    let h = n % 100.0;
    match lang {
        "ru" | "uk" => {
            if t == 1.0 && h != 11.0 {
                "one"
            } else if (2.0..=4.0).contains(&t) && !(12.0..=14.0).contains(&h) {
                "few"
            } else {
                "many"
            }
        }
        "pl" => {
            if n == 1.0 {
                "one"
            } else if (2.0..=4.0).contains(&t) && !(12.0..=14.0).contains(&h) {
                "few"
            } else {
                "many"
            }
        }
        "cs" => {
            if n == 1.0 {
                "one"
            } else if (2.0..=4.0).contains(&n) {
                "few"
            } else {
                "other"
            }
        }
        "ro" => {
            if n == 1.0 {
                "one"
            } else if n == 0.0 || (1.0..=19.0).contains(&h) {
                "few"
            } else {
                "other"
            }
        }
        // The rest: one for 1, else other (the number as given, as `n === 1 ? 'one' : 'other'`).
        _ => {
            if signed == 1.0 {
                "one"
            } else {
                "other"
            }
        }
    }
}

/// The form for a category in "one:{n} season|other:{n} seasons": that one, else "other", else the last, with {n}
/// filled (`plural`). `count` is the number as JavaScript writes it (`String(n)`).
pub fn plural_form(forms: &str, category: &str, count: &str) -> String {
    let (mut chosen, mut other, mut last) = (None, None, "");
    for form in forms.split('|') {
        let (name, text) = match form.find(':') {
            Some(c) => (&form[..c], &form[c + 1..]),
            None => ("", form),
        };
        last = text;
        if name == category {
            chosen = Some(text);
        }
        if name == "other" {
            other = Some(text);
        }
    }
    chosen.or(other).unwrap_or(last).replace("{n}", count)
}

/// The language without its country ("zh" for "zh-CN"), with Java's old codes and Norwegian (`base`).
pub fn base(code: &str) -> String {
    let l = code.split('-').next().unwrap_or("").to_lowercase();
    match l.as_str() {
        "in" => "id".into(),
        "iw" => "he".into(),
        "no" | "nn" => "nb".into(),
        _ => l,
    }
}

const REGIONS: [(&str, &str); 23] = [
    ("pt", "BR"),
    ("es", "ES"),
    ("de", "DE"),
    ("fr", "FR"),
    ("it", "IT"),
    ("nl", "NL"),
    ("pl", "PL"),
    ("sv", "SE"),
    ("da", "DK"),
    ("nb", "NO"),
    ("fi", "FI"),
    ("cs", "CZ"),
    ("ro", "RO"),
    ("tr", "TR"),
    ("ru", "RU"),
    ("uk", "UA"),
    ("el", "GR"),
    ("ja", "JP"),
    ("ko", "KR"),
    ("hi", "IN"),
    ("id", "ID"),
    ("vi", "VN"),
    ("th", "TH"),
];

/// The BCP 47 tag for dates and numbers: the TV's own country when it is in this language (`localeOf`).
pub fn locale_of(code: &str, tv: &str) -> String {
    if code == "en" {
        return "en-US".into();
    }
    if code == "pt" {
        return "pt-BR".into();
    }
    if code.find('-').is_some_and(|i| i > 0) {
        return code.into();
    }
    if base(tv) == code && tv.find('-').is_some_and(|i| i > 0) {
        return tv.into();
    }
    match REGIONS.iter().find(|(l, _)| *l == code) {
        Some((_, r)) => format!("{code}-{r}"),
        None => code.into(),
    }
}

/// The catalogue for a setting ("auto", "fr", "pt-BR"…), or for the TV's language with "auto"; else "en" (`resolve`).
/// `languages`: "en" and every language with a catalogue, sorted.
pub fn resolve(setting: &str, tv: &str, languages: &[&str]) -> String {
    if !setting.is_empty() && setting != "auto" {
        if let Some(code) = best_match(&setting.replacen('_', "-", 1), languages) {
            return code;
        }
    }
    let tv = tv.replacen('_', "-", 1);
    // Simplified Chinese only where it is the written form (not Taiwan, Hong Kong or Macau).
    if base(&tv) == "zh" && traditional(&tv) {
        return "en".into();
    }
    best_match(&base(&tv), languages).unwrap_or_else(|| "en".into())
}

/// `/-(Hant|TW|HK|MO)\b/i`.
fn traditional(tag: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    ["-hant", "-tw", "-hk", "-mo"].iter().any(|p| {
        lower.match_indices(p).any(|(i, _)| {
            !lower[i + p.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    })
}

fn best_match(tag: &str, languages: &[&str]) -> Option<String> {
    let lower = tag.to_lowercase();
    let first = base(&lower);
    languages
        .iter()
        .find(|l| l.to_lowercase() == lower)
        .or_else(|| languages.iter().find(|l| base(l) == first))
        .map(|l| l.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn times() {
        // Date.UTC(2026, 9, 20, 18, 31) = 1792521060000
        assert_eq!(local_time("2026-10-20T18:31", 0.0), 1_792_521_060_000.0);
        assert_eq!(
            local_time("2026-10-20T18:31", -10800.0),
            1_792_531_860_000.0
        );
        assert_eq!(local_time("2026-10-20", 0.0), 0.0);
        assert_eq!(local_time("x2026-10-20T18:31", 0.0), 0.0);
        assert_eq!(local_time("2026-10-20T18:31:59Z", 0.0), 1_792_521_060_000.0);
        // Month 13 is January of the next year; year 99 is 1999.
        assert_eq!(
            local_time("2026-13-01T00:00", 0.0),
            date_utc(2027.0, 0.0, 1.0, 0.0, 0.0)
        );
        assert_eq!(local_time("99-01-01T00:00", 0.0), 915_148_800_000.0);
    }

    #[test]
    fn weather() {
        let o = json!({
            "utc_offset_seconds": 0,
            "current": {"temperature_2m": 21.5, "weather_code": 3, "is_day": 0},
            "daily": {"temperature_2m_max": [25], "temperature_2m_min": [15],
                      "sunrise": ["2026-10-20T05:30"], "sunset": ["2026-10-20T18:00"]},
            "hourly": {"time": ["2026-10-20T10:00", "2026-10-20T11:00", "2026-10-20T12:00"],
                       "temperature_2m": [20, 21], "weather_code": [1, 2, 3], "is_day": [1, 0, 1]}
        });
        let now = local_time("2026-10-20T10:00", 0.0);
        let w = parse_weather(&o, now).unwrap();
        assert_eq!(w["isDay"], false);
        assert_eq!(w["sunrise"], json!(1_792_474_200_000i64));
        assert_eq!(w["hours"].as_array().unwrap().len(), 2);
        assert_eq!(w["hours"][0]["day"], false);
        assert!(w["hours"][1].get("temp").is_none());
        assert_eq!(parse_weather(&json!({}), 0.0), Some(Value::Null));
        assert_eq!(parse_weather(&json!({"current": {}}), 0.0), None); // JavaScript throws here
    }

    #[test]
    fn words() {
        assert_eq!(condition(0.0, false), "night|Clear");
        assert_eq!(condition(f64::NAN, true), "Rain");
        assert_eq!(condition(-1.0, true), "");
        assert_eq!(condition(56.5, true), "Drizzle");
        assert_eq!(condition(60.0, true), "Rain");
        assert_eq!(symbol(-1.0, true), "sunny");
        assert_eq!(symbol(99.0, true), "thunderstorm");
        assert_eq!(franchise("  Star Wars: Andor"), "star wars");
        assert_eq!(franchise("Dune - Part Two"), "dune");
        assert_eq!(franchise(":x"), ":x");
        assert_eq!(service("HBO Max", 1899.0), Some(7));
        assert_eq!(service("Maxdome", 1.0), None);
        assert_eq!(service("Apple TV", 2.0), None);
        assert_eq!(service("Apple TV+", 350.0), Some(8));
        assert_eq!(service("Netflix Amazon Channel", 1.0), None);
    }

    #[test]
    fn backdrops() {
        let images = json!({"backdrops": [
            {"file_path": "/a.jpg", "iso_639_1": "en", "width": 3840, "vote_average": 9},
            {"file_path": "/b.jpg", "iso_639_1": null, "width": 1280, "vote_average": 5},
            {"file_path": "", "iso_639_1": null, "width": 9999}
        ]});
        assert_eq!(best_backdrop(&images), Some(json!("/b.jpg")));
        assert_eq!(best_backdrop(&json!({})), Some(Value::Null));
    }

    #[test]
    fn plurals() {
        assert_eq!(plural_category("ru", 21.0), "one");
        assert_eq!(plural_category("ru", 11.0), "many");
        assert_eq!(plural_category("uk", 22.0), "few");
        assert_eq!(plural_category("pl", 12.0), "many");
        assert_eq!(plural_category("cs", 3.0), "few");
        assert_eq!(plural_category("ro", 0.0), "few");
        assert_eq!(plural_category("ro", 20.0), "other");
        assert_eq!(plural_category("pt", 1.0), "one");
        assert_eq!(plural_category("pt", -1.0), "other");
        assert_eq!(plural_category("ru", -21.0), "one");
        let forms = "one:{n} сезон|few:{n} сезона|many:{n} сезонов|other:{n} сезона";
        assert_eq!(plural_form(forms, "many", "5"), "5 сезонов");
        assert_eq!(plural_form("one:{n} a|other:{n} b", "few", "3"), "3 b");
        assert_eq!(plural_form("{n} x", "one", "1"), "1 x");
    }

    #[test]
    fn languages() {
        let all = ["de", "en", "es", "nb", "pt", "zh-CN"];
        assert_eq!(resolve("auto", "pt-BR", &all), "pt");
        assert_eq!(resolve("pt_BR", "en-US", &all), "pt");
        assert_eq!(resolve("auto", "zh-TW", &all), "en");
        assert_eq!(resolve("auto", "zh-Hans-CN", &all), "zh-CN");
        assert_eq!(resolve("auto", "no-NO", &all), "nb");
        assert_eq!(resolve("fr", "fr-FR", &all), "en");
        assert_eq!(locale_of("es", "es-MX"), "es-MX");
        assert_eq!(locale_of("es", "en-US"), "es-ES");
        assert_eq!(locale_of("zh-CN", ""), "zh-CN");
        assert_eq!(locale_of("xx", ""), "xx");
        assert_eq!(base("in-ID"), "id");
    }
}
