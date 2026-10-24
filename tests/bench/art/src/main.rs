//! Prototype for #13 (not part of the build; see run.sh): tools/make_art.py in Rust (tiny-skia for shapes, image for blur/resize/PNG, ab_glyph for text).
use ab_glyph::{Font, FontRef, PxScale, ScaleFont, VariableFont};
use image::{imageops, GrayImage, Rgba, RgbaImage};
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Transform};

const STOPS: [(f64, [f64; 3]); 3] = [(0.0, [255., 194., 122.]), (0.5, [255., 122., 158.]), (1.0, [154., 140., 255.])];

fn gradient(w: u32, h: u32, angle: f64) -> RgbaImage {
    let a = angle.to_radians();
    let (dx, dy) = (a.cos(), a.sin());
    let c = [0.0, w as f64 * dx, h as f64 * dy, w as f64 * dx + h as f64 * dy];
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

/// Pillow's rounded rectangle covers (x0, y0) to (x1, y1) inclusive.
fn rrect(pb: &mut PathBuilder, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) {
    let (x1, y1) = (x1 + 1.0, y1 + 1.0);
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0);
    let k = 0.552_284_8 * r;
    pb.move_to(x0 + r, y0);
    pb.line_to(x1 - r, y0);
    pb.cubic_to(x1 - r + k, y0, x1, y0 + r - k, x1, y0 + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.cubic_to(x0 + r - k, y1, x0, y1 - r + k, x0, y1 - r);
    pb.line_to(x0, y0 + r);
    pb.cubic_to(x0, y0 + r - k, x0 + r - k, y0, x0 + r, y0);
    pb.close();
}

fn fill(pm: &mut Pixmap, path: &Path, rule: FillRule) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = false; // ImageDraw draws without smoothing; the 4x supersampling smooths it
    pm.fill_path(path, &paint, rule, Transform::identity(), None);
}

fn sparkle(pb: &mut PathBuilder, cx: f64, cy: f64, r: f64) {
    for i in 0..8 {
        let ang = std::f64::consts::FRAC_PI_2 * (i / 2) as f64 + if i % 2 == 0 { 0.0 } else { std::f64::consts::FRAC_PI_4 };
        let rad = if i % 2 == 0 { r } else { r * 0.26 };
        let (x, y) = (cx + rad * (ang - std::f64::consts::FRAC_PI_2).cos(), cy + rad * (ang - std::f64::consts::FRAC_PI_2).sin());
        if i == 0 { pb.move_to(x as f32, y as f32) } else { pb.line_to(x as f32, y as f32) }
    }
    pb.close();
}

fn alpha_of(pm: &Pixmap) -> GrayImage {
    GrayImage::from_fn(pm.width(), pm.height(), |x, y| image::Luma([pm.pixel(x, y).unwrap().alpha()]))
}

/// Pillow's alpha_composite (straight alpha), of a single color with this alpha, at (dx, dy).
fn composite_color(dst: &mut RgbaImage, color: [u8; 3], alpha: &GrayImage, dx: i64, dy: i64) {
    for y in 0..dst.height() as i64 {
        for x in 0..dst.width() as i64 {
            let (sx, sy) = (x - dx, y - dy);
            if sx < 0 || sy < 0 || sx >= alpha.width() as i64 || sy >= alpha.height() as i64 { continue; }
            let sa = alpha.get_pixel(sx as u32, sy as u32)[0] as f64 / 255.0;
            if sa == 0.0 { continue; }
            let d = dst.get_pixel_mut(x as u32, y as u32);
            let da = d[3] as f64 / 255.0;
            let oa = sa + da * (1.0 - sa);
            for k in 0..3 {
                d[k] = ((color[k] as f64 * sa + d[k] as f64 * da * (1.0 - sa)) / oa).round() as u8;
            }
            d[3] = (oa * 255.0).round() as u8;
        }
    }
}

/// Lanczos resize with premultiplied alpha, as Pillow does for RGBA.
fn resize(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let pre = image::ImageBuffer::<Rgba<f32>, Vec<f32>>::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        let a = p[3] as f32 / 255.0; // image's f32 pixels are 0..1
        Rgba([p[0] as f32 / 255.0 * a, p[1] as f32 / 255.0 * a, p[2] as f32 / 255.0 * a, a])
    });
    let small = imageops::resize(&pre, w, h, imageops::FilterType::Lanczos3);
    RgbaImage::from_fn(w, h, |x, y| {
        let p = small.get_pixel(x, y);
        let a = p[3].clamp(0.0, 1.0);
        let un = |c: f32| if a > 0.0 { (c / a * 255.0).round().clamp(0.0, 255.0) as u8 } else { 0 };
        Rgba([un(p[0]), un(p[1]), un(p[2]), (a * 255.0).round() as u8])
    })
}

fn mark(size: u32, bg: bool) -> RgbaImage {
    let s = size * 4;
    let sf = s as f64;
    let mut img = if bg { gradient(s, s, 35.0) } else { RgbaImage::new(s, s) };
    if bg {
        let mut mask = Pixmap::new(s, s).unwrap();
        let mut pb = PathBuilder::new();
        rrect(&mut pb, 0.0, 0.0, (s - 1) as f32, (s - 1) as f32, (sf * 0.23).floor() as f32);
        fill(&mut mask, &pb.finish().unwrap(), FillRule::Winding);
        for (x, y, p) in img.enumerate_pixels_mut() {
            p[3] = mask.pixel(x, y).unwrap().alpha();
        }
    }
    let mut layer = Pixmap::new(s, s).unwrap();
    let (x0, y0, x1, y1) = (sf * 0.2, sf * 0.27, sf * 0.8, sf * 0.66);
    let (r, wd) = ((sf * 0.07).floor(), (sf * 0.055).floor());
    let mut pb = PathBuilder::new();
    rrect(&mut pb, x0 as f32, y0 as f32, x1 as f32, y1 as f32, r as f32);
    rrect(&mut pb, (x0 + wd) as f32, (y0 + wd) as f32, (x1 - wd) as f32, (y1 - wd) as f32, (r - wd).max(0.0) as f32);
    fill(&mut layer, &pb.finish().unwrap(), FillRule::EvenOdd);
    let mut pb = PathBuilder::new();
    rrect(&mut pb, (sf * 0.36) as f32, (sf * 0.715) as f32, (sf * 0.64) as f32, (sf * 0.765) as f32, (sf * 0.025).floor() as f32);
    sparkle(&mut pb, sf * 0.5, sf * 0.465, sf * 0.12);
    sparkle(&mut pb, sf * 0.655, sf * 0.355, sf * 0.045);
    fill(&mut layer, &pb.finish().unwrap(), FillRule::Winding);
    let white = alpha_of(&layer);
    if bg {
        let shadow = imageops::blur(&white, (sf * 0.02) as f32);
        let tint = GrayImage::from_fn(s, s, |x, y| image::Luma([(shadow.get_pixel(x, y)[0] as f64 * 0.22) as u8]));
        composite_color(&mut img, [120, 40, 80], &tint, 0, (sf * 0.012) as i64);
    }
    composite_color(&mut img, [255, 255, 255], &white, 0, 0);
    resize(&img, size, size)
}

fn banner(font_data: &[u8], w: u32, h: u32) -> RgbaImage {
    let s = 4;
    let mut img = gradient(w * s, h * s, 20.0);
    let m = mark((h as f64 * 0.5 * s as f64) as u32, false);
    let mut font = FontRef::try_from_slice(font_data).unwrap();
    font.set_variation(b"wght", 650.0);
    font.set_variation(b"opsz", 32.0);
    let px = (h as f64 * 0.15 * s as f64) as f32;
    let scale = PxScale::from(px * font.height_unscaled() / font.units_per_em().unwrap());
    let sf = font.as_scaled(scale);
    // Lay out "Free My TV" and take its ink box.
    let (mut caret, mut glyphs, mut prev) = (0.0f32, Vec::new(), None);
    for ch in "Free My TV".chars() {
        let id = sf.glyph_id(ch);
        if let Some(p) = prev { caret += sf.kern(p, id); }
        glyphs.push(id.with_scale_and_position(scale, ab_glyph::point(caret, sf.ascent())));
        caret += sf.h_advance(id);
        prev = Some(id);
    }
    let outlined: Vec<_> = glyphs.into_iter().filter_map(|g| font.outline_glyph(g)).collect();
    let (mut bx0, mut by0, mut bx1, mut by1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for o in &outlined {
        let b = o.px_bounds();
        bx0 = bx0.min(b.min.x); by0 = by0.min(b.min.y); bx1 = bx1.max(b.max.x); by1 = by1.max(b.max.y);
    }
    let gap = (w as f64 * 0.02 * s as f64) as i64;
    let total = m.width() as i64 + gap + (bx1 - bx0).round() as i64;
    let x = (img.width() as i64 - total) / 2;
    let my = ((img.height() - m.height()) as f64 / 2.0) as i64;
    let malpha = GrayImage::from_fn(m.width(), m.height(), |a, b| image::Luma([m.get_pixel(a, b)[3]]));
    composite_color(&mut img, [255, 255, 255], &malpha, x, my);
    let mut text = GrayImage::new(img.width(), img.height());
    let (ox, oy) = ((x + m.width() as i64 + gap) as f32 - bx0, ((img.height() as f32 - (by1 - by0)) / 2.0).floor() - by0);
    for o in &outlined {
        let b = o.px_bounds();
        o.draw(|gx, gy, c| {
            let (px, py) = ((b.min.x + ox) as i64 + gx as i64, (b.min.y + oy) as i64 + gy as i64);
            if px >= 0 && py >= 0 && (px as u32) < text.width() && (py as u32) < text.height() {
                let v = text.get_pixel_mut(px as u32, py as u32);
                v[0] = v[0].max((c.min(1.0) * 255.0).round() as u8);
            }
        });
    }
    composite_color(&mut img, [255, 255, 255], &text, 0, 0);
    resize(&img, w, h)
}

fn main() {
    let root = std::env::args().nth(1).expect("ROOT");
    let out = std::env::args().nth(2).expect("OUT");
    let font = std::fs::read(format!("{root}/app/assets/fonts/InterVariable.ttf")).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    mark(192, true).save(format!("{out}/icon.png")).unwrap();
    banner(&font, 640, 360).save(format!("{out}/banner.png")).unwrap();
    mark(512, true).save(format!("{out}/icon-512.png")).unwrap();
    mark(180, true).save(format!("{out}/apple-touch-icon.png")).unwrap();
    mark(64, true).save(format!("{out}/favicon-64.png")).unwrap();
    println!("ok");
}
