//! tools/make_symbols.py in Rust: each icon drawn 8x larger without smoothing (fontmode "1"), its gaps closed with a
//! 9x9 max then min filter, cropped with 16 px around it, scaled down 8x (Lanczos) and saved as white with alpha.
use crate::{resize, text};
use image::{Rgba, RgbaImage};
use std::collections::HashMap;
use std::path::Path;

/// Name in the app -> (Material Symbols name, filled?), in make_symbols.py's order.
const MAP: [(&str, &str, i32); 31] = [
    ("sunny", "sunny", 1),
    ("clear_night", "clear_night", 1),
    ("partly_cloudy_day", "partly_cloudy_day", 1),
    ("partly_cloudy_night", "partly_cloudy_night", 1),
    ("cloud", "cloud", 1),
    ("foggy", "foggy", 1),
    ("rainy", "rainy", 1),
    ("thunderstorm", "thunderstorm", 1),
    ("snowflake", "ac_unit", 0),
    ("tv", "tv", 0),
    ("gamepad", "sports_esports", 1),
    ("hdmi", "settings_input_hdmi", 1),
    ("apps", "apps", 1),
    ("settings", "settings", 1),
    ("speaker", "speaker", 1),
    ("sparkles", "auto_awesome", 1),
    ("bedtime", "bedtime", 1),
    ("home", "home", 1),
    ("chevron_right", "chevron_right", 0),
    ("bell", "notifications", 1),
    ("sync", "sync", 0),
    ("check", "check", 0),
    ("play", "play_arrow", 1),
    ("tv_settings", "display_settings", 1),
    ("chevron_down", "keyboard_arrow_down", 0),
    ("sunrise", "wb_twilight", 1),
    ("sunset", "wb_twilight", 1),
    ("live_tv", "live_tv", 1),
    ("bitcoin", "currency_bitcoin", 0),
    ("keystone", "settings_overscan", 1),
    ("cast", "cast", 1),
];

/// MaxFilter / MinFilter (size 9) with Pillow's edge-replicating border: separable, rows then columns.
fn rank(px: &[u8], w: usize, h: usize, max: bool) -> Vec<u8> {
    let pick = |a: u8, b: u8| if max { a.max(b) } else { a.min(b) };
    let mut tmp = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut v = px[y * w + x];
            for d in 1..=4i64 {
                let l = (x as i64 - d).max(0) as usize;
                let r = (x as i64 + d).min(w as i64 - 1) as usize;
                v = pick(v, pick(px[y * w + l], px[y * w + r]));
            }
            tmp[y * w + x] = v;
        }
    }
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut v = tmp[y * w + x];
            for d in 1..=4i64 {
                let u = (y as i64 - d).max(0) as usize;
                let b = (y as i64 + d).min(h as i64 - 1) as usize;
                v = pick(v, pick(tmp[u * w + x], tmp[b * w + x]));
            }
            out[y * w + x] = v;
        }
    }
    out
}

fn shift_div255(a: u32) -> u32 {
    ((a >> 8) + a) >> 8
}

/// Writes each icon to `out/<name>.png`, as `python3 -I tools/make_symbols.py FONT CODEPOINTS OUT [EM]` does;
/// returns how many.
pub fn make_symbols(
    font_path: &Path,
    codepoints: &Path,
    out: &Path,
    em: usize,
) -> Result<usize, String> {
    let list = std::fs::read_to_string(codepoints)
        .map_err(|e| format!("{}: {e}", codepoints.display()))?;
    let codes: HashMap<&str, u32> = list
        .lines()
        .filter_map(|l| match l.split_whitespace().collect::<Vec<_>>()[..] {
            [name, hex] => u32::from_str_radix(hex, 16).ok().map(|c| (name, c)),
            _ => None,
        })
        .collect();
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut font = text::Font::new(font_path, (em * 8) as i64, &[])?;
    font.mono = true;
    let side = em * 16;
    for (name, material, fill) in MAP {
        // Axes Fill, Grade, Optical Size, Weight.
        font.set_axes(&[fill as f64, 0.0, 48.0, 600.0])?;
        let ch = codes
            .get(material)
            .and_then(|&c| char::from_u32(c))
            .ok_or_else(|| format!("{}: no {material}", codepoints.display()))?
            .to_string();
        let mut big = vec![0u8; side * side];
        let mask = font.render(&ch)?;
        let (mw, mh) = (mask.width, mask.height);
        let (dx, dy) = (
            (em * 4) as i64 + mask.x_offset,
            (em * 4) as i64 + mask.y_offset,
        );
        for my in 0..mh as i64 {
            for mx in 0..mw as i64 {
                let (x, y) = (mx + dx, my + dy);
                if x < 0 || y < 0 || x >= side as i64 || y >= side as i64 {
                    continue;
                }
                let m = mask.px[(my * mw as i64 + mx) as usize] as u32;
                let o = &mut big[y as usize * side + x as usize];
                *o = shift_div255(*o as u32 * (255 - m) + 255 * m + 128) as u8; // BLEND(mask, out, 255)
            }
        }
        let big = rank(&rank(&big, side, side, true), side, side, false);
        // getbbox: the non-zero pixels.
        let (mut x0, mut y0, mut x1, mut y1) = (side, side, 0, 0);
        for y in 0..side {
            for x in 0..side {
                if big[y * side + x] != 0 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x + 1);
                    y1 = y1.max(y + 1);
                }
            }
        }
        let pad = 16;
        let (cx0, cy0, cw, chh) = (x0 - pad, y0 - pad, x1 - x0 + 2 * pad, y1 - y0 + 2 * pad);
        let mut crop = vec![0u8; cw * chh];
        for y in 0..chh {
            crop[y * cw..(y + 1) * cw]
                .copy_from_slice(&big[(y + cy0) * side + cx0..(y + cy0) * side + cx0 + cw]);
        }
        let w = ((cw as f64 / 8.0).round_ties_even() as usize).max(1);
        let h = ((chh as f64 / 8.0).round_ties_even() as usize).max(1);
        let small = resize::resample(&crop, 1, cw, chh, w, h);
        let img = RgbaImage::from_fn(w as u32, h as u32, |x, y| {
            Rgba([255, 255, 255, small[y as usize * w + x as usize]])
        });
        let path = out.join(format!("{name}.png"));
        img.save(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(MAP.len())
}
