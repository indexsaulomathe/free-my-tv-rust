#!/usr/bin/env bash
# Runs every test in the fork: the Python tools, the web app (Node), and the Rust crates.
#   tests/run.sh            # everything
#   tests/run.sh python     # only one suite: python, web or rust
set -euo pipefail
cd "$(dirname "$0")/.."
what="${1:-all}"

if [ "$what" = all ] || [ "$what" = python ]; then
  echo "== Python (tests/python)"
  python3 -m pytest -q tests/python
fi
if [ "$what" = all ] || [ "$what" = web ]; then
  echo "== Web app (tests/web)"
  node --test tests/web/*.test.js
fi
if [ "$what" = all ] || [ "$what" = rust ]; then
  echo "== Rust (rust/)"
  (cd rust && cargo test --quiet)
fi
echo "All tests passed."
