#!/usr/bin/env bash
# Issue #13 prototype: tools/make_art.py in Rust, timed and compared with the committed images. Needs cargo and
# Pillow (python3 -m pip install pillow). Not part of the build.
set -euo pipefail
cd "$(dirname "$0")"
root="$(cd ../../.. && pwd)"
cargo build -q --release
time ./target/release/art-prototype "$root" out
python3 compare.py "$root" out
