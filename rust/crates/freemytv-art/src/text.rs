//! Pillow's FreeTypeFont text, ported from src/_imagingft.c (Pillow 13.0.0) with raqm's layout: the same FreeType
//! and HarfBuzz (hb-ft) calls, so the same glyph positions and coverage. Only what the art needs: horizontal text,
//! anchor "la", mode "L" or "1" (`mono`), no stroke.
#![allow(non_camel_case_types)]
use freetype_sys::*;
use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::path::Path;

#[repr(C)]
struct hb_glyph_info_t {
    codepoint: u32,
    mask: u32,
    cluster: u32,
    var1: u32,
    var2: u32,
}

#[repr(C)]
struct hb_glyph_position_t {
    x_advance: i32,
    y_advance: i32,
    x_offset: i32,
    y_offset: i32,
    var: u32,
}

extern "C" {
    fn hb_ft_font_create_referenced(face: FT_Face) -> *mut c_void;
    fn hb_font_destroy(font: *mut c_void);
    fn hb_buffer_create() -> *mut c_void;
    fn hb_buffer_destroy(buf: *mut c_void);
    fn hb_buffer_add_utf8(
        buf: *mut c_void,
        text: *const c_char,
        len: c_int,
        offset: c_uint,
        item_len: c_int,
    );
    fn hb_buffer_set_direction(buf: *mut c_void, dir: c_uint);
    fn hb_buffer_set_script(buf: *mut c_void, script: u32);
    fn hb_buffer_guess_segment_properties(buf: *mut c_void);
    fn hb_shape(font: *mut c_void, buf: *mut c_void, features: *const c_void, n: c_uint);
    fn hb_buffer_get_glyph_infos(buf: *mut c_void, len: *mut c_uint) -> *mut hb_glyph_info_t;
    fn hb_buffer_get_glyph_positions(
        buf: *mut c_void,
        len: *mut c_uint,
    ) -> *mut hb_glyph_position_t;
    fn FT_Set_Var_Design_Coordinates(
        face: FT_Face,
        n: FT_UInt,
        coords: *const FT_Fixed,
    ) -> FT_Error;
    fn FT_Library_Version(
        lib: FT_Library,
        major: *mut FT_Int,
        minor: *mut FT_Int,
        patch: *mut FT_Int,
    );
}

const HB_DIRECTION_LTR: c_uint = 4;
const HB_SCRIPT_LATIN: u32 = u32::from_be_bytes(*b"Latn");

/// PIXEL(x): 26.6 to pixels, rounding as _imagingft.c does.
fn pixel(x: i64) -> i64 {
    ((x + 32) & -64) >> 6
}

struct Glyph {
    index: u32,
    x_offset: i64,
    x_advance: i64,
    y_offset: i64,
}

/// getsize()/getbbox(): the text's size and where its top left is, relative to the anchor point.
pub struct BBox {
    pub width: i64,
    pub height: i64,
    pub x_offset: i64,
    pub y_offset: i64,
}

/// render(): the text's coverage, and where its top left goes relative to the anchor point.
pub struct Mask {
    pub width: u32,
    pub height: u32,
    pub px: Vec<u8>,
    pub x_offset: i64,
    pub y_offset: i64,
}

pub struct Font {
    lib: FT_Library,
    face: FT_Face,
    /// fontmode "1": FT_LOAD_TARGET_MONO, coverage 0 or 255.
    pub mono: bool,
}

impl Font {
    /// set_variation_by_axes(coords), as font_setvaraxes does.
    pub fn set_axes(&mut self, coords: &[f64]) -> Result<(), String> {
        let fixed: Vec<FT_Fixed> = coords.iter().map(|c| (c * 65536.0) as FT_Fixed).collect();
        match unsafe {
            FT_Set_Var_Design_Coordinates(self.face, fixed.len() as FT_UInt, fixed.as_ptr())
        } {
            0 => Ok(()),
            e => Err(format!("the font refused its axes (FreeType error {e})")),
        }
    }

    fn flags(&self) -> FT_Int32 {
        if self.mono {
            FT_LOAD_DEFAULT | FT_LOAD_TARGET_MONO
        } else {
            FT_LOAD_DEFAULT
        }
    }

    /// ImageFont.truetype(path, size), then set_variation_by_axes(coords) when there are any.
    pub fn new(path: &Path, size: i64, coords: &[f64]) -> Result<Font, String> {
        unsafe {
            let mut lib: FT_Library = std::ptr::null_mut();
            if FT_Init_FreeType(&mut lib) != 0 {
                return Err("FreeType didn't start".into());
            }
            let (mut major, mut minor, mut patch) = (0, 0, 0);
            FT_Library_Version(lib, &mut major, &mut minor, &mut patch);
            if (major, minor, patch) != (2, 14, 3) {
                eprintln!(
                    "Note: FreeType {major}.{minor}.{patch} here; Pillow 13.0.0 ships 2.14.3, so text may differ from the Python's."
                );
            }
            let mut face: FT_Face = std::ptr::null_mut();
            let c = CString::new(path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
            if FT_New_Face(lib, c.as_ptr(), 0, &mut face) != 0 {
                FT_Done_FreeType(lib);
                return Err(format!("can't open the font {}", path.display()));
            }
            let mut font = Font {
                lib,
                face,
                mono: false,
            };
            let mut req = FT_Size_RequestRec {
                size_request_type: FT_SIZE_REQUEST_TYPE_NOMINAL,
                width: size * 64,
                height: size * 64,
                horiResolution: 0,
                vertResolution: 0,
            };
            if FT_Request_Size(face, &mut req) != 0 {
                return Err(format!("{}: can't use size {size}", path.display()));
            }
            if !coords.is_empty() {
                font.set_axes(coords)?;
            }
            Ok(font)
        }
    }

    /// text_layout_raqm: HarfBuzz over the FreeType face (hb-ft), left to right, Latin.
    fn layout(&self, text: &str) -> Result<Vec<Glyph>, String> {
        let c = CString::new(text).map_err(|_| "text with a NUL character".to_string())?;
        unsafe {
            let font = hb_ft_font_create_referenced(self.face);
            let buf = hb_buffer_create();
            hb_buffer_add_utf8(buf, c.as_ptr(), -1, 0, -1);
            hb_buffer_set_direction(buf, HB_DIRECTION_LTR);
            hb_buffer_set_script(buf, HB_SCRIPT_LATIN);
            hb_buffer_guess_segment_properties(buf);
            hb_shape(font, buf, std::ptr::null(), 0);
            let mut n = 0;
            let infos =
                std::slice::from_raw_parts(hb_buffer_get_glyph_infos(buf, &mut n), n as usize);
            let pos =
                std::slice::from_raw_parts(hb_buffer_get_glyph_positions(buf, &mut n), n as usize);
            let out = infos
                .iter()
                .zip(pos)
                .map(|(i, p)| Glyph {
                    index: i.codepoint,
                    x_offset: p.x_offset as i64,
                    x_advance: p.x_advance as i64,
                    y_offset: p.y_offset as i64,
                })
                .collect();
            hb_buffer_destroy(buf);
            hb_font_destroy(font);
            Ok(out)
        }
    }

    /// bounding_box_and_anchors, horizontal, anchor "la".
    pub fn bbox(&self, text: &str) -> Result<BBox, String> {
        let glyphs = self.layout(text)?;
        let (mut position, mut x_min, mut x_max, mut y_min, mut y_max) =
            (0i64, 0i64, 0i64, 0i64, 0i64);
        unsafe {
            for g in &glyphs {
                let px = pixel(position + g.x_offset);
                let py = pixel(g.y_offset);
                position += g.x_advance;
                x_max = x_max.max(pixel(position));
                load(FT_Load_Glyph(self.face, g.index, self.flags()))?;
                let mut glyph: FT_Glyph = std::ptr::null_mut();
                load(FT_Get_Glyph((*self.face).glyph, &mut glyph))?;
                let mut b = FT_BBox {
                    xMin: 0,
                    yMin: 0,
                    xMax: 0,
                    yMax: 0,
                };
                FT_Glyph_Get_CBox(glyph, FT_GLYPH_BBOX_PIXELS, &mut b);
                x_max = x_max.max(b.xMax + px);
                x_min = x_min.min(b.xMin + px);
                y_max = y_max.max(b.yMax + py);
                y_min = y_min.min(b.yMin + py);
                FT_Done_Glyph(glyph);
            }
            let y_anchor = if glyphs.is_empty() {
                0
            } else {
                pixel((*(*self.face).size).metrics.ascender)
            };
            Ok(BBox {
                width: x_max - x_min,
                height: y_max - y_min,
                x_offset: x_min,
                y_offset: y_anchor - y_max,
            })
        }
    }

    /// font_render: the coverage mask and where its top left goes, relative to the anchor point.
    pub fn render(&self, text: &str) -> Result<Mask, String> {
        let BBox {
            width,
            height,
            x_offset,
            y_offset,
        } = self.bbox(text)?;
        let glyphs = self.layout(text)?;
        let (w, h) = (width as usize, height as usize);
        let mut im = vec![0u8; w * h];
        unsafe {
            let (mut x, mut x_min, mut y_max) = (0i64, 0i64, 0i64);
            for g in &glyphs {
                let px = pixel(x + g.x_offset);
                let py = pixel(g.y_offset);
                load(FT_Load_Glyph(
                    self.face,
                    g.index,
                    self.flags() | FT_LOAD_RENDER,
                ))?;
                let slot = (*self.face).glyph;
                y_max = y_max.max((*slot).bitmap_top as i64 + py);
                x_min = x_min.min((*slot).bitmap_left as i64 + px);
                x += g.x_advance;
            }
            let mut x = (-x_min as f64 * 64.0).round() as i64;
            let y = (-y_max as f64 * 64.0).round() as i64;
            for g in &glyphs {
                let px = pixel(x + g.x_offset);
                let py = pixel(y + g.y_offset);
                load(FT_Load_Glyph(
                    self.face,
                    g.index,
                    self.flags() | FT_LOAD_RENDER,
                ))?;
                let slot = (*self.face).glyph;
                let bm = &(*slot).bitmap;
                let mono = bm.pixel_mode as u32 == FT_PIXEL_MODE_MONO;
                if !mono && bm.pixel_mode as u32 != FT_PIXEL_MODE_GRAY {
                    return Err("unsupported bitmap pixel mode".into());
                }
                let xx = px + (*slot).bitmap_left as i64;
                let top = -(py + (*slot).bitmap_top as i64);
                let mut x0 = 0i64;
                let mut x1 = bm.width as i64;
                if xx < 0 {
                    x0 = -xx;
                }
                if xx + x1 > w as i64 {
                    x1 = w as i64 - xx;
                }
                // An empty glyph (a space) has no buffer: nothing to blend.
                let rows = if bm.buffer.is_null() {
                    0
                } else {
                    bm.rows as i64
                };
                for row in 0..rows {
                    let yy = top + row;
                    let source = bm.buffer.offset((row * bm.pitch as i64) as isize);
                    if yy >= 0 && yy < h as i64 {
                        for k in x0..x1 {
                            // FT_Bitmap_Convert of a mono bitmap gives 0 or 1, times convert_scale 255.
                            let src = if mono {
                                ((*source.offset((k / 8) as isize) >> (7 - k % 8)) & 1) as u32 * 255
                            } else {
                                *source.offset(k as isize) as u32
                            };
                            if src > 0 {
                                let t = &mut im[yy as usize * w + (xx + k) as usize];
                                *t = if *t > 0 {
                                    (src + muldiv255(*t as u32, 255 - src) as u32).min(255) as u8
                                } else {
                                    src as u8
                                };
                            }
                        }
                    }
                }
                x += g.x_advance;
            }
        }
        Ok(Mask {
            width: width as u32,
            height: height as u32,
            px: im,
            x_offset,
            y_offset,
        })
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            FT_Done_Face(self.face);
            FT_Done_FreeType(self.lib);
        }
    }
}

/// A FreeType call's result, as Pillow's geterror() reports it.
fn load(error: FT_Error) -> Result<(), String> {
    match error {
        0 => Ok(()),
        e => Err(format!("FreeType error {e}")),
    }
}

fn muldiv255(a: u32, b: u32) -> u8 {
    let t = a * b + 128;
    (((t >> 8) + t) >> 8) as u8
}
