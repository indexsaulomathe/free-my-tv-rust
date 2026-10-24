//! Pillow's GaussianBlur for one-channel images, ported from libImaging/BoxBlur.c (Pillow 13.0.0): three box blur
//! passes per direction with 24-bit fixed-point weights, in u32 arithmetic that wraps as the C does.

/// _gaussian_blur_radius (float arithmetic as in the C).
fn box_radius(radius: f32, passes: i32) -> f32 {
    let sigma2: f32 = radius * radius / passes as f32;
    let big_l: f32 = (12.0f64 * sigma2 as f64 + 1.0).sqrt() as f32;
    let l: f32 = ((big_l as f64 - 1.0) / 2.0).floor() as f32;
    let mut a: f32 = (2.0 * l + 1.0) * (l * (l + 1.0) - 3.0 * sigma2);
    a /= 6.0 * (sigma2 - (l + 1.0) * (l + 1.0));
    l + a
}

/// ImagingLineBoxBlur8
fn line(out: &mut [u8], inp: &[u8], radius: i32, edge_a: i32, edge_b: i32, ww: u32, fw: u32) {
    let lastx = inp.len() as i32 - 1;
    let px = |i: i32| inp[i as usize] as u32;
    let mut acc: u32 = px(0).wrapping_mul((radius + 1) as u32);
    for x in 0..edge_a - 1 {
        acc = acc.wrapping_add(px(x));
    }
    acc = acc.wrapping_add(px(lastx).wrapping_mul((radius - edge_a + 1) as u32));
    let mut step = |acc: &mut u32, x: i32, sub: i32, add: i32, left: i32, right: i32| {
        *acc = acc.wrapping_add(px(add)).wrapping_sub(px(sub));
        let bulk = acc
            .wrapping_mul(ww)
            .wrapping_add((px(left) + px(right)).wrapping_mul(fw));
        out[x as usize] = (bulk.wrapping_add(1 << 23) >> 24) as u8;
    };
    if edge_a <= edge_b {
        for x in 0..edge_a {
            step(&mut acc, x, 0, x + radius, 0, x + radius + 1);
        }
        for x in edge_a..edge_b {
            step(
                &mut acc,
                x,
                x - radius - 1,
                x + radius,
                x - radius - 1,
                x + radius + 1,
            );
        }
        for x in edge_b..=lastx {
            step(&mut acc, x, x - radius - 1, lastx, x - radius - 1, lastx);
        }
    } else {
        for x in 0..edge_b {
            step(&mut acc, x, 0, x + radius, 0, x + radius + 1);
        }
        for x in edge_b..edge_a {
            step(&mut acc, x, 0, lastx, 0, lastx);
        }
        for x in edge_a..=lastx {
            step(&mut acc, x, x - radius - 1, lastx, x - radius - 1, lastx);
        }
    }
}

/// ImagingHorizontalBoxBlur, in place.
fn horizontal(px: &mut [u8], w: usize, h: usize, float_radius: f32) {
    let radius = float_radius as i32;
    let ww = ((1u32 << 24) as f32 / (float_radius * 2.0 + 1.0)) as u32;
    let fw = ((1u32 << 24).wrapping_sub(((radius * 2 + 1) as u32).wrapping_mul(ww))) / 2;
    let edge_a = (radius + 1).min(w as i32);
    let edge_b = (w as i32 - radius - 1).max(0);
    let mut out = vec![0u8; w];
    for y in 0..h {
        let row = &mut px[y * w..(y + 1) * w];
        line(&mut out, row, radius, edge_a, edge_b, ww, fw);
        row.copy_from_slice(&out);
    }
}

fn transpose(px: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut t = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            t[x * h + y] = px[y * w + x];
        }
    }
    t
}

/// ImageFilter.GaussianBlur(radius) on an L image (3 passes).
pub fn gaussian_blur(px: &[u8], w: usize, h: usize, radius: f32) -> Vec<u8> {
    let r = box_radius(radius, 3);
    let mut out = px.to_vec();
    if r != 0.0 {
        for _ in 0..3 {
            horizontal(&mut out, w, h, r);
        }
        let mut t = transpose(&out, w, h);
        for _ in 0..3 {
            horizontal(&mut t, h, w, r);
        }
        out = transpose(&t, h, w);
    }
    out
}
