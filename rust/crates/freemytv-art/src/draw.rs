//! Pillow's ImageDraw for one-channel images, ported from libImaging/Draw.c and PIL/ImageDraw.py (Pillow 13.0.0):
//! the same integer and float steps, so the same pixels. Only what tools/make_art.py uses: filled polygons,
//! rectangles, pieslices, arcs and rounded rectangles.

pub struct Canvas {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

impl Canvas {
    pub fn new(w: u32, h: u32) -> Canvas {
        Canvas {
            w: w as i32,
            h: h as i32,
            px: vec![0; (w * h) as usize],
        }
    }

    /// hline8
    fn hline(&mut self, mut x0: i32, y0: i32, mut x1: i32, ink: u8) {
        if y0 < 0 || y0 >= self.h {
            return;
        }
        if x0 < 0 {
            x0 = 0;
        } else if x0 >= self.w {
            return;
        }
        if x1 < 0 {
            return;
        } else if x1 >= self.w {
            x1 = self.w - 1;
        }
        if x0 <= x1 {
            let row = (y0 * self.w) as usize;
            self.px[row + x0 as usize..=row + x1 as usize].fill(ink);
        }
    }

    /// ImagingDrawRectangle, filled.
    pub fn rectangle(&mut self, x0: i32, mut y0: i32, x1: i32, mut y1: i32, ink: u8) {
        if y0 > y1 {
            std::mem::swap(&mut y0, &mut y1);
        }
        if y0 < 0 {
            y0 = 0;
        } else if y0 >= self.h {
            return;
        }
        if y1 < 0 {
            return;
        } else if y1 > self.h {
            y1 = self.h;
        }
        for y in y0..=y1 {
            self.hline(x0, y, x1, ink);
        }
    }

    /// ImagingDrawPolygon, filled; `xy` already truncated to int as _draw_polygon does.
    pub fn polygon(&mut self, xy: &[(i32, i32)], ink: u8) {
        let count = xy.len();
        if count == 0 {
            return;
        }
        let mut e: Vec<Edge> = Vec::new();
        let mut i = 0;
        while i + 1 < count {
            let (x0, y0) = xy[i];
            let (x1, y1) = xy[i + 1];
            if y0 == y1 && i != 0 && y0 == xy[i - 1].1 {
                let prev_x = xy[i - 1].0;
                let last = e.last_mut().unwrap();
                if x1 > x0 && x0 > prev_x {
                    last.xmax = x1;
                    i += 1;
                    continue;
                } else if x1 < x0 && x0 < prev_x {
                    last.xmin = x1;
                    i += 1;
                    continue;
                }
            }
            e.push(Edge::new(x0, y0, x1, y1));
            i += 1;
        }
        if xy[i] != xy[0] {
            e.push(Edge::new(xy[i].0, xy[i].1, xy[0].0, xy[0].1));
        }
        self.polygon_generic(&e, ink);
    }

    /// polygon_generic, for an image without alpha drawing (hline8).
    fn polygon_generic(&mut self, e: &[Edge], ink: u8) {
        if e.is_empty() {
            return;
        }
        let mut ymin = self.h - 1;
        let mut ymax = 0;
        let mut table: Vec<&Edge> = Vec::new();
        for edge in e {
            ymin = ymin.min(edge.ymin);
            ymax = ymax.max(edge.ymax);
            if edge.ymin == edge.ymax {
                self.hline(edge.xmin, edge.ymin, edge.xmax, ink);
                continue;
            }
            table.push(edge);
        }
        if ymin < 0 {
            ymin = 0;
        }
        if ymax > self.h {
            ymax = self.h;
        }
        let at = |edge: &Edge, y: i32| -> f32 { (y - edge.y0) as f32 * edge.dx + edge.x0 as f32 };
        let mut xx: Vec<f32> = Vec::with_capacity(table.len() * 2);
        let mut y = ymin;
        while y <= ymax {
            xx.clear();
            for (i, current) in table.iter().enumerate() {
                if y >= current.ymin && y <= current.ymax {
                    xx.push(at(current, y));
                    if y == current.ymax && y < ymax {
                        let last = *xx.last().unwrap();
                        xx.push(last);
                    } else if (y == current.ymin || y == current.ymax) && current.dx != 0.0 {
                        for other in &table[..i] {
                            if (y != other.ymin && y != other.ymax) || other.dx == 0.0 {
                                continue;
                            }
                            let j = xx.len() - 1;
                            if xx[j].round() == at(other, y).round() {
                                let offset = if y == current.ymax { -1 } else { 1 };
                                let adjacent = at(current, y + offset);
                                if y + offset >= other.ymin && y + offset <= other.ymax {
                                    let adjacent_other = at(other, y + offset);
                                    if xx[j] > adjacent + 1.0 && xx[j] > adjacent_other + 1.0 {
                                        xx[j] = ((adjacent as f64).max(adjacent_other as f64)
                                            as f32)
                                            .round()
                                            + 1.0;
                                    } else if xx[j] < adjacent - 1.0 && xx[j] < adjacent_other - 1.0
                                    {
                                        xx[j] = ((adjacent as f64).min(adjacent_other as f64)
                                            as f32)
                                            .round()
                                            - 1.0;
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            xx.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut i = 1;
            while i < xx.len() {
                self.hline(round_up(xx[i - 1]), y, round_down(xx[i]), ink);
                i += 2;
            }
            y += 1;
        }
    }

    /// ellipseNew
    pub fn ellipse(&mut self, [x0, y0, x1, y1]: [i32; 4], ink: u8, fill: bool, mut width: i32) {
        let (a, b) = (x1 - x0, y1 - y0);
        if a < 0 || b < 0 {
            return;
        }
        if fill {
            width = a + b;
        }
        let mut st = EllipseState::new(a, b, width);
        while let Some((lx, ly, rx)) = st.next() {
            self.hline(x0 + (lx + a) / 2, y0 + (ly + b) / 2, x0 + (rx + a) / 2, ink);
        }
    }

    fn clip_ellipse(&mut self, [x0, y0, x1, y1]: [i32; 4], mut st: ClipEllipse, ink: u8) {
        let (a, b) = (x1 - x0, y1 - y0);
        while let Some((lx, ly, rx)) = st.next() {
            self.hline(x0 + (lx + a) / 2, y0 + (ly + b) / 2, x0 + (rx + a) / 2, ink);
        }
    }

    /// ImagingDrawPieslice, filled.
    pub fn pieslice(&mut self, xy: [i32; 4], start: f32, end: f32, ink: u8) {
        let [x0, y0, x1, y1] = xy;
        let (start, end) = normalize_angles(start, end);
        if start + 360.0 == end {
            return self.ellipse(xy, ink, true, 0);
        }
        if start == end {
            return;
        }
        let (a, b) = (x1 - x0, y1 - y0);
        if a < 0 || b < 0 {
            return;
        }
        let st = ClipEllipse::pie(a, b, x1 + y1 - x0 - y0, start, end);
        self.clip_ellipse(xy, st, ink);
    }

    /// ImagingDrawArc
    pub fn arc(&mut self, xy: [i32; 4], start: f32, end: f32, ink: u8, width: i32) {
        let [x0, y0, x1, y1] = xy;
        let (start, end) = normalize_angles(start, end);
        if start + 360.0 == end {
            return self.ellipse(xy, ink, false, width);
        }
        if start == end {
            return;
        }
        let (a, b) = (x1 - x0, y1 - y0);
        if a < 0 || b < 0 {
            return;
        }
        let st = ClipEllipse::arc(a, b, width, start, end);
        self.clip_ellipse(xy, st, ink);
    }

    /// ImageDraw.rounded_rectangle, all four corners, as PIL/ImageDraw.py draws it.
    pub fn rounded_rectangle(
        &mut self,
        xy: [f64; 4],
        radius: f64,
        fill: Option<u8>,
        outline: Option<(u8, i32)>,
    ) {
        let [fx0, fy0, fx1, fy1] = xy;
        let mut d = (fx1 - fx0).min(fy1 - fy0).min(radius * 2.0);
        let (x0, y0, x1, y1) = (py_round(fx0), py_round(fy0), py_round(fx1), py_round(fy1));
        let full_x = d >= (x1 - x0 - 1) as f64;
        if full_x {
            d = (x1 - x0) as f64;
        }
        let full_y = d >= (y1 - y0 - 1) as f64;
        if full_y {
            d = (y1 - y0) as f64;
        }
        // The C side takes (int) of each coordinate.
        let t = |p: [f64; 4]| [p[0] as i32, p[1] as i32, p[2] as i32, p[3] as i32];
        if full_x && full_y {
            // All corners joined: ImageDraw.ellipse.
            if let Some(ink) = fill {
                self.ellipse(t(xy), ink, true, 1);
            }
            if let Some((ink, width)) = outline {
                if fill != Some(ink) && width != 0 {
                    self.ellipse(t(xy), ink, false, width);
                }
            }
            return;
        }
        if d == 0.0 {
            // No curve: ImageDraw.rectangle (filled; make_art outlines none of these).
            if let Some(ink) = fill {
                let [a, b, c, e] = t(xy);
                self.rectangle(a, b, c, e, ink);
            }
            return;
        }
        let r = (d / 2.0).floor() as i32;
        let (x0f, y0f, x1f, y1f) = (x0 as f64, y0 as f64, x1 as f64, y1 as f64);
        let parts: Vec<([f64; 4], f32, f32)> = if full_x {
            vec![
                ([x0f, y0f, x0f + d, y0f + d], 180.0, 360.0),
                ([x0f, y1f - d, x0f + d, y1f], 0.0, 180.0),
            ]
        } else if full_y {
            vec![
                ([x0f, y0f, x0f + d, y0f + d], 90.0, 270.0),
                ([x1f - d, y0f, x1f, y0f + d], 270.0, 90.0),
            ]
        } else {
            vec![
                ([x0f, y0f, x0f + d, y0f + d], 180.0, 270.0),
                ([x1f - d, y0f, x1f, y0f + d], 270.0, 360.0),
                ([x1f - d, y1f - d, x1f, y1f], 0.0, 90.0),
                ([x0f, y1f - d, x0f + d, y1f], 90.0, 180.0),
            ]
        };
        if let Some(ink) = fill {
            for (p, s, e) in &parts {
                self.pieslice(t(*p), *s, *e, ink);
            }
            if full_x {
                self.rectangle(x0, y0 + r + 1, x1, y1 - r - 1, ink);
            } else if x1 - r > x0 + r + 1 {
                self.rectangle(x0 + r + 1, y0, x1 - r - 1, y1, ink);
            }
            if !full_x && !full_y {
                self.rectangle(x0, y0 + r + 1, x0 + r, y1 - (r + 1), ink);
                self.rectangle(x1 - r, y0 + r + 1, x1, y1 - (r + 1), ink);
            }
        }
        if let Some((ink, width)) = outline {
            if fill != Some(ink) && width != 0 {
                for (p, s, e) in &parts {
                    self.arc(t(*p), *s, *e, ink, width);
                }
                if !full_x {
                    self.rectangle(x0 + r + 1, y0, x1 - (r + 1), y0 + width - 1, ink);
                    self.rectangle(x0 + r + 1, y1 - width + 1, x1 - (r + 1), y1, ink);
                }
                if !full_y {
                    self.rectangle(x0, y0 + r + 1, x0 + width - 1, y1 - (r + 1), ink);
                    self.rectangle(x1 - width + 1, y0 + r + 1, x1, y1 - (r + 1), ink);
                }
            }
        }
    }
}

/// Python's round(): halves to even.
pub fn py_round(v: f64) -> i32 {
    v.round_ties_even() as i32
}

fn round_up(f: f32) -> i32 {
    if f >= 0.0 {
        ((f + 0.5) as f64).floor() as i32
    } else {
        -((f.abs() + 0.5) as f64).floor() as i32
    }
}

fn round_down(f: f32) -> i32 {
    if f >= 0.0 {
        ((f - 0.5) as f64).ceil() as i32
    } else {
        -((f.abs() - 0.5) as f64).ceil() as i32
    }
}

struct Edge {
    x0: i32,
    y0: i32,
    xmin: i32,
    ymin: i32,
    xmax: i32,
    ymax: i32,
    dx: f32,
}

impl Edge {
    fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Edge {
        let dx = if y0 == y1 {
            0.0
        } else {
            (x1 - x0) as f32 / (y1 - y0) as f32
        };
        Edge {
            x0,
            y0,
            xmin: x0.min(x1),
            xmax: x0.max(x1),
            ymin: y0.min(y1),
            ymax: y0.max(y1),
            dx,
        }
    }
}

/// fmod as C does it (sign of the dividend).
fn fmod(a: f64, b: f64) -> f64 {
    a % b
}

/// normalize_angles: 0 <= al < 360, al <= ar <= al + 360 (floats, computed in double and stored in float).
fn normalize_angles(al: f32, ar: f32) -> (f32, f32) {
    if ar - al >= 360.0 {
        return (0.0, 360.0);
    }
    let (alf, arf) = (al as f64, ar as f64);
    let nal = fmod(
        if alf < 0.0 {
            360.0 - fmod(-alf, 360.0)
        } else {
            alf
        },
        360.0,
    ) as f32;
    let nalf = nal as f64;
    let nar = (nalf
        + fmod(
            if arf < nalf {
                360.0 - fmod(nalf - arf, 360.0)
            } else {
                arf - nalf
            },
            360.0,
        )) as f32;
    (nal, nar)
}

struct Quarter {
    cx: i32,
    cy: i32,
    ex: i32,
    ey: i32,
    a2: i64,
    b2: i64,
    a2b2: i64,
    finished: bool,
}

impl Quarter {
    fn new(a: i32, b: i32) -> Quarter {
        if a < 0 || b < 0 {
            return Quarter {
                cx: 0,
                cy: 0,
                ex: 0,
                ey: 0,
                a2: 0,
                b2: 0,
                a2b2: 0,
                finished: true,
            };
        }
        let (a2, b2) = (a as i64 * a as i64, b as i64 * b as i64);
        Quarter {
            cx: a,
            cy: b % 2,
            ex: a % 2,
            ey: b,
            a2,
            b2,
            a2b2: a2 * b2,
            finished: false,
        }
    }

    fn delta(&self, x: i64, y: i64) -> i64 {
        (self.a2 * y * y + self.b2 * x * x - self.a2b2).abs()
    }

    fn next(&mut self) -> Option<(i32, i32)> {
        if self.finished {
            return None;
        }
        let ret = (self.cx, self.cy);
        if self.cx == self.ex && self.cy == self.ey {
            self.finished = true;
        } else {
            let (mut nx, mut ny) = (self.cx, self.cy + 2);
            let mut ndelta = self.delta(nx as i64, ny as i64);
            if nx > 1 {
                let d = self.delta((self.cx - 2) as i64, (self.cy + 2) as i64);
                if ndelta > d {
                    nx = self.cx - 2;
                    ny = self.cy + 2;
                    ndelta = d;
                }
                let d = self.delta((self.cx - 2) as i64, self.cy as i64);
                if ndelta > d {
                    nx = self.cx - 2;
                    ny = self.cy;
                }
            }
            self.cx = nx;
            self.cy = ny;
        }
        Some(ret)
    }
}

struct EllipseState {
    st_o: Quarter,
    st_i: Quarter,
    py: i32,
    pl: i32,
    pr: i32,
    buf: Vec<(i32, i32, i32)>,
    finished: bool,
    leftmost: i32,
}

impl EllipseState {
    fn new(a: i32, b: i32, w: i32) -> EllipseState {
        let mut st_o = Quarter::new(a, b);
        let leftmost = a % 2;
        let first = if w < 1 { None } else { st_o.next() };
        match first {
            None => EllipseState {
                st_o,
                st_i: Quarter::new(-1, -1),
                py: 0,
                pl: 0,
                pr: 0,
                buf: Vec::new(),
                finished: true,
                leftmost,
            },
            Some((pr, py)) => EllipseState {
                st_o,
                st_i: Quarter::new(a - 2 * (w - 1), b - 2 * (w - 1)),
                py,
                pl: leftmost,
                pr,
                buf: Vec::new(),
                finished: false,
                leftmost,
            },
        }
    }

    fn next(&mut self) -> Option<(i32, i32, i32)> {
        if self.buf.is_empty() {
            if self.finished {
                return None;
            }
            let (y, mut l, r) = (self.py, self.pl, self.pr);
            let last = loop {
                match self.st_o.next() {
                    Some((_, cy)) if cy <= y => {}
                    other => break other,
                }
            };
            match last {
                None => self.finished = true,
                Some((cx, cy)) => {
                    self.pr = cx;
                    self.py = cy;
                }
            }
            let inner = loop {
                match self.st_i.next() {
                    Some((cx, cy)) if cy <= y => l = cx,
                    other => break other,
                }
            };
            self.pl = match inner {
                None => self.leftmost,
                Some((cx, _)) => cx,
            };
            if (l > 0 || l < r) && y > 0 {
                self.buf.push((if l == 0 { 2 } else { l }, y, r));
            }
            if y > 0 {
                self.buf.push((-r, y, -l));
            }
            if l > 0 || l < r {
                self.buf.push((if l == 0 { 2 } else { l }, -y, r));
            }
            self.buf.push((-r, -y, -l));
        }
        self.buf.pop()
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    And,
    Or,
    Clip,
}

struct Node {
    kind: Kind,
    a: f64,
    b: f64,
    c: f64,
    l: Option<Box<Node>>,
    r: Option<Box<Node>>,
}

fn clip(a: f64, b: f64, c: f64) -> Box<Node> {
    Box::new(Node {
        kind: Kind::Clip,
        a,
        b,
        c,
        l: None,
        r: None,
    })
}

fn join(kind: Kind, l: Box<Node>, r: Box<Node>) -> Box<Node> {
    Box::new(Node {
        kind,
        a: 0.0,
        b: 0.0,
        c: 0.0,
        l: Some(l),
        r: Some(r),
    })
}

fn transpose(n: &mut Option<Box<Node>>) {
    if let Some(node) = n {
        if node.kind == Kind::Clip {
            std::mem::swap(&mut node.a, &mut node.b);
        }
        transpose(&mut node.l);
        transpose(&mut node.r);
    }
}

/// clip_tree_do_clip: the clipped segment's ends as (x, +1 open / -1 close).
fn do_clip(root: &Option<Box<Node>>, mut x0: i32, y: i32, mut x1: i32) -> Vec<(i32, i8)> {
    let Some(node) = root else {
        return vec![(x0, 1), (x1, -1)];
    };
    match node.kind {
        Kind::Clip => {
            let eps = 1e-9;
            let (a, b, c) = (node.a, node.b, node.c);
            let yf = y as f64;
            if a.abs() < eps {
                if b * yf + c < -eps {
                    x0 = 1;
                    x1 = 0;
                }
            } else {
                let ix = -(b * yf + c) / a;
                if a * x0 as f64 + b * yf + c < eps {
                    x0 = (x0 as f64).max(ix).round() as i32;
                }
                if a * x1 as f64 + b * yf + c < eps {
                    x1 = (x1 as f64).min(ix).round() as i32;
                }
            }
            if x0 <= x1 {
                vec![(x0, 1), (x1, -1)]
            } else {
                vec![]
            }
        }
        Kind::And | Kind::Or => {
            let l1 = do_clip(&node.l, x0, y, x1);
            let l2 = do_clip(&node.r, x0, y, x1);
            let (mut i, mut j) = (0, 0);
            let (mut k1, mut k2) = (0i32, 0i32);
            let mut out: Vec<(i32, i8)> = Vec::new();
            while i < l1.len() || j < l2.len() {
                let t;
                if j >= l2.len()
                    || (i < l1.len()
                        && (l1[i].0 < l2[j].0 || (l1[i].0 == l2[j].0 && l1[i].1 > l2[j].1)))
                {
                    t = l1[i];
                    k1 += t.1 as i32;
                    i += 1;
                } else {
                    t = l2[j];
                    k2 += t.1 as i32;
                    j += 1;
                }
                let tail = out.last().map(|e| e.1);
                let keep = match node.kind {
                    Kind::Or => (t.1 == 1 && tail != Some(1)) || (t.1 == -1 && k1 == 0 && k2 == 0),
                    _ => {
                        (t.1 == 1 && tail != Some(1) && k1 > 0 && k2 > 0)
                            || (t.1 == -1 && tail == Some(1) && (k1 == 0 || k2 == 0))
                    }
                };
                if keep {
                    out.push(t);
                }
            }
            out
        }
    }
}

struct ClipEllipse {
    st: EllipseState,
    root: Option<Box<Node>>,
    head: std::collections::VecDeque<(i32, i8)>,
    y: i32,
}

impl ClipEllipse {
    /// arc_init
    fn arc(a: i32, b: i32, w: i32, al: f32, ar: f32) -> ClipEllipse {
        if a < b {
            let mut s = ClipEllipse::arc(b, a, w, 90.0 - ar, 90.0 - al);
            s.st = EllipseState::new(a, b, w);
            transpose(&mut s.root);
            return s;
        }
        let st = EllipseState::new(a, b, w);
        let (al, ar) = normalize_angles(al, ar);
        let root = if ar == al + 360.0 {
            None
        } else {
            let (af, bf) = (a as f64, b as f64);
            let (alr, arr) = (
                al as f64 * std::f64::consts::PI / 180.0,
                ar as f64 * std::f64::consts::PI / 180.0,
            );
            let lc = || {
                clip(
                    -af * alr.sin(),
                    bf * alr.cos(),
                    (af * af - bf * bf) * (al as f64 * std::f64::consts::PI / 90.0).sin() / 2.0,
                )
            };
            let rc = || {
                clip(
                    af * arr.sin(),
                    -bf * arr.cos(),
                    (bf * bf - af * af) * (ar as f64 * std::f64::consts::PI / 90.0).sin() / 2.0,
                )
            };
            let small = ar - al < 180.0;
            if fmod(al as f64, 180.0) == 0.0 || fmod(ar as f64, 180.0) == 0.0 {
                Some(join(if small { Kind::And } else { Kind::Or }, lc(), rc()))
            } else if ((al / 180.0) as i32 + (ar / 180.0) as i32) % 2 == 1 {
                let lb = if (al / 180.0) as i32 % 2 == 0 {
                    1.0
                } else {
                    -1.0
                };
                let rb = if (ar / 180.0) as i32 % 2 == 0 {
                    1.0
                } else {
                    -1.0
                };
                Some(join(
                    Kind::Or,
                    join(Kind::And, clip(0.0, lb, 0.0), lc()),
                    join(Kind::And, clip(0.0, rb, 0.0), rc()),
                ))
            } else {
                let kind = if small { Kind::And } else { Kind::Or };
                let side = if ar < 180.0 || ar > 540.0 { 1.0 } else { -1.0 };
                Some(join(kind, join(kind, lc(), rc()), clip(0.0, side, 0.0)))
            }
        };
        ClipEllipse {
            st,
            root,
            head: Default::default(),
            y: 0,
        }
    }

    /// pie_init
    fn pie(a: i32, b: i32, w: i32, al: f32, ar: f32) -> ClipEllipse {
        let st = EllipseState::new(a, b, w);
        let (af, bf) = (a as f64, b as f64);
        let (alr, arr) = (
            al as f64 * std::f64::consts::PI / 180.0,
            ar as f64 * std::f64::consts::PI / 180.0,
        );
        let (xl, xr, yl, yr) = (
            af * alr.cos(),
            af * arr.cos(),
            bf * alr.sin(),
            bf * arr.sin(),
        );
        let mut root = join(
            if ar - al < 180.0 { Kind::And } else { Kind::Or },
            clip(-yl, xl, 0.0),
            clip(yr, -xr, 0.0),
        );
        if ar - al < 90.0 {
            root = join(Kind::And, root, clip((xl + xr) / 2.0, (yl + yr) / 2.0, 0.0));
        }
        ClipEllipse {
            st,
            root: Some(root),
            head: Default::default(),
            y: 0,
        }
    }

    fn next(&mut self) -> Option<(i32, i32, i32)> {
        while self.head.is_empty() {
            let (x0, y, x1) = self.st.next()?;
            self.head = do_clip(&self.root, x0, y, x1).into();
            self.y = y;
        }
        let (l, _) = self.head.pop_front()?;
        let (r, _) = self.head.pop_front()?;
        Some((l, self.y, r))
    }
}
