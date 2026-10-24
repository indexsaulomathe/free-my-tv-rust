//! The app's art in Rust: `tools/make_art.py` (icon, TV banner, site images) and `tools/make_symbols.py` (the
//! icons from Material Symbols).
//!
//! The pixels match Pillow 13.0.0's: its drawing ([`draw`], from libImaging/Draw.c), blur (BoxBlur.c), resize
//! ([`resize`], Resample.c) and compositing (AlphaComposite.c, Paste.c) are ported step for step, and text goes
//! through the same libraries Pillow uses, FreeType and HarfBuzz ([`text`]). Same input, same pixels (the PNG bytes
//! differ: another encoder).

mod art;
mod blur;
pub mod draw;
pub mod resize;
pub mod symbols;
pub mod text;

pub use art::make_art;
pub use symbols::make_symbols;
