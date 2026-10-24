//! Prototype for #15: Recs.franchise and HomeData.parseWeather in Rust, called from Java over JNI.
use jni::objects::{JClass, JString};
use jni::sys::{jdoubleArray, jstring};
use jni::JNIEnv;

fn franchise(title: &str) -> String {
    let t = title.to_lowercase();
    let cut = t.find(':').or_else(|| t.find(" - "));
    match cut { Some(c) if c > 0 => t[..c].trim().to_string(), _ => t.trim().to_string() }
}

#[no_mangle]
pub extern "system" fn Java_Bench_franchiseRust<'a>(mut env: JNIEnv<'a>, _: JClass<'a>, title: JString<'a>) -> jstring {
    let s: String = env.get_string(&title).map(Into::into).unwrap_or_default();
    env.new_string(franchise(&s)).map(|j| j.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// "2026-10-20T18:31" at a fixed offset → epoch ms (prototype: the Java uses the zone's rules).
fn local(iso: &str, off: i64) -> f64 {
    let n: Vec<i64> = iso.split(|c| c == '-' || c == 'T' || c == ':').filter_map(|p| p.parse().ok()).collect();
    if n.len() < 5 { return 0.0; }
    let (y, m, d) = (n[0] - if n[1] <= 2 { 1 } else { 0 }, n[1], n[2]);
    let era = y.div_euclid(400); let yoe = y - era * 400; let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1; let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    ((days * 86400 + n[3] * 3600 + n[4] * 60 - off) * 1000) as f64
}

/// [temp, code, isDay, max, min, sunrise, sunset, then at, temp, code, day for each hour]
#[no_mangle]
pub extern "system" fn Java_Bench_weatherRust<'a>(mut env: JNIEnv<'a>, _: JClass<'a>, body: JString<'a>, now: f64) -> jdoubleArray {
    let s: String = env.get_string(&body).map(Into::into).unwrap_or_default();
    let o: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
    let off = o["utc_offset_seconds"].as_i64().unwrap_or(0);
    let (c, d, h) = (&o["current"], &o["daily"], &o["hourly"]);
    let f = |v: &serde_json::Value| v.as_f64().unwrap_or(f64::NAN);
    let mut out = vec![f(&c["temperature_2m"]), f(&c["weather_code"]), (c["is_day"].as_i64().unwrap_or(1) == 1) as i32 as f64,
        f(&d["temperature_2m_max"][0]), f(&d["temperature_2m_min"][0]),
        local(d["sunrise"][0].as_str().unwrap_or(""), off), local(d["sunset"][0].as_str().unwrap_or(""), off)];
    let times = h["time"].as_array().cloned().unwrap_or_default();
    let mut n = 0;
    for (i, t) in times.iter().enumerate() {
        if n >= 6 { break; }
        let at = local(t.as_str().unwrap_or(""), off);
        if at <= now { continue; }
        n += 1;
        out.extend([at, f(&h["temperature_2m"][i]), f(&h["weather_code"][i]), (h["is_day"][i].as_i64().unwrap_or(1) == 1) as i32 as f64]);
    }
    let arr = env.new_double_array(out.len() as i32).unwrap();
    env.set_double_array_region(&arr, 0, &out).unwrap();
    arr.into_raw()
}
