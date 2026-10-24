//! Pillow's resize(..., Image.LANCZOS) for RGBA, ported from libImaging/Resample.c and Convert.c (Pillow 13.0.0):
//! premultiplied alpha (RGBA -> RGBa with MULDIV255), 22-bit fixed-point Lanczos coefficients, horizontal pass then
//! vertical, and back to straight alpha.
use image::{Rgba, RgbaImage};

const PRECISION_BITS: u32 = 32 - 8 - 2;

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        return 1.0;
    }
    let x = x * std::f64::consts::PI;
    x.sin() / x
}

fn lanczos(x: f64) -> f64 {
    if (-3.0..3.0).contains(&x) {
        sinc(x) * sinc(x / 3.0)
    } else {
        0.0
    }
}

/// precompute_coeffs + normalize_coeffs_8bpc: per output pixel, (first input pixel, count, fixed-point weights).
fn coeffs(in_size: i32, in0: f32, in1: f32, out_size: i32) -> Vec<(i32, i32, Vec<i32>)> {
    let scale = (in1 - in0) as f64 / out_size as f64;
    let filterscale = scale.max(1.0);
    let support = 3.0 * filterscale;
    let inv = 1.0 / filterscale;
    (0..out_size)
        .map(|xx| {
            let center = in0 as f64 + (xx as f64 + 0.5) * scale;
            let xmin = ((center - support + 0.5) as i32).max(0);
            let xmax = ((center + support + 0.5) as i32).min(in_size) - xmin;
            let mut k: Vec<f64> = (0..xmax)
                .map(|x| lanczos((x as f64 + xmin as f64 - center + 0.5) * inv))
                .collect();
            let ww: f64 = k.iter().sum();
            if ww != 0.0 {
                k.iter_mut().for_each(|w| *w /= ww);
            }
            let fixed = k
                .iter()
                .map(|&w| {
                    let v = w * (1u32 << PRECISION_BITS) as f64;
                    if w < 0.0 {
                        (-0.5 + v) as i32
                    } else {
                        (0.5 + v) as i32
                    }
                })
                .collect();
            (xmin, xmax, fixed)
        })
        .collect()
}

fn clip8(ss: i32) -> u8 {
    (ss >> PRECISION_BITS).clamp(0, 255) as u8
}

fn muldiv255(a: u32, b: u32) -> u8 {
    let t = a * b + 128;
    (((t >> 8) + t) >> 8) as u8
}

/// ImagingResampleInner for 8-bit images of `ch` channels, Lanczos, box = the whole image.
pub fn resample(px: &[u8], ch: usize, iw: usize, ih: usize, w: usize, h: usize) -> Vec<u8> {
    let need_h = w != iw;
    let need_v = h != ih;
    let horizontal_first =
        !((ih as i64 - h as i64) > 0 && (ih as i64 - h as i64) > (iw as i64 - w as i64) * 2);
    let mut vert = if (need_h && horizontal_first) || need_v {
        coeffs(ih as i32, 0.0, ih as f32, h as i32)
    } else {
        vec![]
    };
    let mut first = 0usize;
    let mut last = ih;
    if need_h && horizontal_first {
        first = vert[0].0 as usize;
        last = (vert[h - 1].0 + vert[h - 1].1) as usize;
        for v in vert.iter_mut() {
            v.0 -= first as i32;
        }
    }
    let horiz = if need_h {
        coeffs(iw as i32, 0.0, iw as f32, w as i32)
    } else {
        vec![]
    };
    let hpass = |src: &[u8], sw: usize, rows: usize, offset: usize| -> Vec<u8> {
        let mut out = vec![0u8; w * rows * ch];
        for yy in 0..rows {
            let line = &src[(yy + offset) * sw * ch..(yy + offset + 1) * sw * ch];
            for (xx, (xmin, xmax, k)) in horiz.iter().enumerate() {
                for c in 0..ch {
                    let mut ss = 1i32 << (PRECISION_BITS - 1);
                    for x in 0..*xmax as usize {
                        ss = ss.wrapping_add(line[(x + *xmin as usize) * ch + c] as i32 * k[x]);
                    }
                    out[(yy * w + xx) * ch + c] = clip8(ss);
                }
            }
        }
        out
    };
    let vpass = |src: &[u8], sw: usize| -> Vec<u8> {
        let mut out = vec![0u8; sw * h * ch];
        for (yy, (ymin, ymax, k)) in vert.iter().enumerate() {
            for xx in 0..sw {
                for c in 0..ch {
                    let mut ss = 1i32 << (PRECISION_BITS - 1);
                    for y in 0..*ymax as usize {
                        ss = ss.wrapping_add(
                            src[((y + *ymin as usize) * sw + xx) * ch + c] as i32 * k[y],
                        );
                    }
                    out[(yy * sw + xx) * ch + c] = clip8(ss);
                }
            }
        }
        out
    };
    let mut cur = px.to_vec();
    let mut cw = iw;
    if horizontal_first {
        if need_h {
            cur = hpass(&cur, cw, last - first, first);
            cw = w;
        }
        if need_v {
            cur = vpass(&cur, cw);
        }
    } else {
        if need_v {
            cur = vpass(&cur, cw);
        }
        if need_h {
            cur = hpass(&cur, cw, h, 0);
        }
    }
    cur
}

pub fn resize(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let pre: Vec<u8> = img
        .pixels()
        .flat_map(|p| {
            [
                muldiv255(p[0] as u32, p[3] as u32),
                muldiv255(p[1] as u32, p[3] as u32),
                muldiv255(p[2] as u32, p[3] as u32),
                p[3],
            ]
        })
        .collect();
    let out = resample(
        &pre,
        4,
        img.width() as usize,
        img.height() as usize,
        w as usize,
        h as usize,
    );
    RgbaImage::from_fn(w, h, |x, y| {
        let i = (y * w + x) as usize * 4;
        let p = [out[i], out[i + 1], out[i + 2], out[i + 3]];
        let a = p[3] as u32;
        if a == 255 || a == 0 {
            Rgba(p)
        } else {
            let un = |c: u8| (255 * c as u32 / a).min(255) as u8;
            Rgba([un(p[0]), un(p[1]), un(p[2]), p[3]])
        }
    })
}
