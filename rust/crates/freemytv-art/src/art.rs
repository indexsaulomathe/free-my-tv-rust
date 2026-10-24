//! tools/make_art.py: the mark (a TV with a sparkle on the gradient squircle), the TV banner and the site's images.

use crate::{blur, draw, resize, text};
use image::{GrayImage, Luma, Rgba, RgbaImage};
use std::path::{Path, PathBuf};

/// Sunrise -> pink -> dusk.
const STOPS: [(f64, [f64; 3]); 3] = [
    (0.0, [255., 194., 122.]),
    (0.5, [255., 122., 158.]),
    (1.0, [154., 140., 255.]),
];

/// gradient(): diagonal three-color gradient, rounded as Python's round() (halves to even).
fn gradient(w: u32, h: u32, angle: f64) -> RgbaImage {
    let a = angle.to_radians();
    let (dx, dy) = (a.cos(), a.sin());
    let c = [
        0.0,
        w as f64 * dx,
        h as f64 * dy,
        w as f64 * dx + h as f64 * dy,
    ];
    let lo = c.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = c.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let t = ((x as f64 * dx + y as f64 * dy) - lo) / (hi - lo);
            for i in 0..STOPS.len() - 1 {
                let (t0, c0) = STOPS[i];
                let (t1, c1) = STOPS[i + 1];
                if t <= t1 {
                    let f = (t - t0) / (t1 - t0);
                    let px = |k: usize| (c0[k] + (c1[k] - c0[k]) * f).round_ties_even() as u8;
                    img.put_pixel(x, y, Rgba([px(0), px(1), px(2), 255]));
                    break;
                }
            }
        }
    }
    img
}

fn shift_div255(a: u32) -> u32 {
    ((a >> 8) + a) >> 8
}

/// Image.alpha_composite(src, dest=(dx, dy)): libImaging/AlphaComposite.c, with 7 bits of extra precision.
fn alpha_composite(dst: &mut RgbaImage, src: &RgbaImage, dx: i64, dy: i64) {
    const P: u32 = 7;
    for sy in 0..src.height() as i64 {
        for sx in 0..src.width() as i64 {
            let (x, y) = (sx + dx, sy + dy);
            if x < 0 || y < 0 || x >= dst.width() as i64 || y >= dst.height() as i64 {
                continue;
            }
            let s = *src.get_pixel(sx as u32, sy as u32);
            if s[3] == 0 {
                continue;
            }
            let d = dst.get_pixel_mut(x as u32, y as u32);
            let (sa, da) = (s[3] as u32, d[3] as u32);
            let outa255 = sa * 255 + da * (255 - sa);
            let coef1 = sa * 255 * 255 * (1 << P) / outa255;
            let coef2 = 255 * (1 << P) - coef1;
            for c in 0..3 {
                let t = s[c] as u32 * coef1 + d[c] as u32 * coef2;
                d[c] = (shift_div255(t + (0x80 << P)) >> P) as u8;
            }
            d[3] = shift_div255(outa255 + 0x80) as u8;
        }
    }
}

/// One color with this alpha, as the RGBA layers make_art composites.
fn colored(color: [u8; 3], alpha: &GrayImage) -> RgbaImage {
    RgbaImage::from_fn(alpha.width(), alpha.height(), |x, y| {
        match alpha.get_pixel(x, y)[0] {
            0 => Rgba([0, 0, 0, 0]),
            a => Rgba([color[0], color[1], color[2], a]),
        }
    })
}

/// ImageDraw.text's draw_bitmap: libImaging/Paste.c's fill_mask_L on an RGBA image.
fn fill_mask(dst: &mut RgbaImage, ink: [u8; 4], mask: &text::Mask, dx: i64, dy: i64) {
    for my in 0..mask.height as i64 {
        for mx in 0..mask.width as i64 {
            let (x, y) = (mx + dx, my + dy);
            if x < 0 || y < 0 || x >= dst.width() as i64 || y >= dst.height() as i64 {
                continue;
            }
            let m = mask.px[(my * mask.width as i64 + mx) as usize] as u32;
            let out = dst.get_pixel_mut(x as u32, y as u32);
            let a = out[3] as u32;
            for i in 0..4 {
                let mut cm = m;
                if i != 3 && cm != 0 {
                    cm = 255 - (255 - cm) * (1 - (255 - a) / 255);
                }
                out[i] = shift_div255(out[i] as u32 * (255 - cm) + ink[i] as u32 * cm + 128) as u8;
            }
        }
    }
}

/// sparkle(): a four-point star; the C side truncates each point to int.
fn sparkle(c: &mut draw::Canvas, cx: f64, cy: f64, r: f64) {
    use std::f64::consts::PI;
    let pts: Vec<(i32, i32)> = (0..8)
        .map(|i| {
            let ang = PI / 2.0 * (i / 2) as f64 + if i % 2 == 0 { 0.0 } else { PI / 4.0 };
            let rad = if i % 2 == 0 { r } else { r * 0.26 };
            (
                (cx + rad * (ang - PI / 2.0).cos()) as i32,
                (cy + rad * (ang - PI / 2.0).sin()) as i32,
            )
        })
        .collect();
    c.polygon(&pts, 255);
}

fn gray(c: &draw::Canvas) -> GrayImage {
    GrayImage::from_raw(c.w as u32, c.h as u32, c.px.clone()).expect("canvas size")
}

/// mark(): a TV with a sparkle, on the gradient squircle (bg) or alone (white), drawn 4x and scaled down.
fn mark(size: u32, bg: bool) -> RgbaImage {
    let s = size * 4;
    let sf = s as f64;
    let mut img = if bg {
        gradient(s, s, 35.0)
    } else {
        RgbaImage::new(s, s)
    };
    if bg {
        let mut mask = draw::Canvas::new(s, s);
        mask.rounded_rectangle(
            [0.0, 0.0, sf - 1.0, sf - 1.0],
            (sf * 0.23).floor(),
            Some(255),
            None,
        );
        for (x, y, p) in img.enumerate_pixels_mut() {
            p[3] = mask.px[(y * s + x) as usize];
        }
    }
    let mut layer = draw::Canvas::new(s, s);
    let outline = (255, (sf * 0.055) as i32);
    layer.rounded_rectangle(
        [sf * 0.2, sf * 0.27, sf * 0.8, sf * 0.66],
        (sf * 0.07).floor(),
        None,
        Some(outline),
    );
    layer.rounded_rectangle(
        [sf * 0.36, sf * 0.715, sf * 0.64, sf * 0.765],
        (sf * 0.025).floor(),
        Some(255),
        None,
    );
    sparkle(&mut layer, sf * 0.5, sf * 0.465, sf * 0.12);
    sparkle(&mut layer, sf * 0.655, sf * 0.355, sf * 0.045);
    let white = gray(&layer);
    if bg {
        let shadow =
            blur::gaussian_blur(white.as_raw(), s as usize, s as usize, (sf * 0.02) as f32);
        let tint = GrayImage::from_fn(s, s, |x, y| {
            Luma([(shadow[(y * s + x) as usize] as f64 * 0.22) as u8])
        });
        alpha_composite(
            &mut img,
            &colored([120, 40, 80], &tint),
            0,
            (sf * 0.012) as i64,
        );
    }
    alpha_composite(&mut img, &colored([255, 255, 255], &white), 0, 0);
    resize::resize(&img, size, size)
}

/// banner(): the mark and "Free My TV" in Inter (weight 650) on the gradient, drawn 4x and scaled down.
fn banner(font_path: &Path, w: u32, h: u32) -> Result<RgbaImage, String> {
    let s = 4u32;
    let mut img = gradient(w * s, h * s, 20.0);
    let m = mark((h as f64 * 0.5 * s as f64) as u32, false);
    // Inter's axes: optical size (32 here) and weight.
    let font = text::Font::new(
        font_path,
        (h as f64 * 0.15 * s as f64) as i64,
        &[32.0, 650.0],
    )?;
    let words = "Free My TV";
    let b = font.bbox(words)?;
    let gap = (w as f64 * 0.02 * s as f64) as i64;
    let total = m.width() as i64 + gap + b.width;
    let x = ((w * s) as i64 - total).div_euclid(2);
    alpha_composite(
        &mut img,
        &m,
        x,
        (((h * s) - m.height()) as f64 / 2.0) as i64,
    );
    let y = ((h * s) as i64 - b.height).div_euclid(2) - b.y_offset;
    let mask = font.render(words)?;
    let tx = x + m.width() as i64 + gap - b.x_offset + mask.x_offset;
    fill_mask(&mut img, [255, 255, 255, 255], &mask, tx, y + mask.y_offset);
    Ok(resize::resize(&img, w, h))
}

/// Writes the app's icon and banner and the site's images, as `python3 tools/make_art.py` does.
pub fn make_art(root: &Path) -> Result<Vec<PathBuf>, String> {
    let res = root.join("app/res/drawable-nodpi");
    let site = root.join("docs/assets");
    for dir in [&res, &site] {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let font = root.join("app/assets/fonts/InterVariable.ttf");
    let images = [
        (res.join("icon.png"), mark(192, true)),
        (res.join("banner.png"), banner(&font, 640, 360)?),
        (site.join("icon-512.png"), mark(512, true)),
        (site.join("apple-touch-icon.png"), mark(180, true)),
        (site.join("favicon-64.png"), mark(64, true)),
    ];
    let mut written = Vec::new();
    for (path, img) in images {
        img.save(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}
