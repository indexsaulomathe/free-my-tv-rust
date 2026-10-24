// HarfBuzz (with hb-ft) from the system, as Pillow's raqm uses it; FreeType comes through freetype-sys.
fn main() {
    pkg_config::probe_library("harfbuzz")
        .expect("HarfBuzz with hb-ft is needed (harfbuzz development package)");
}
