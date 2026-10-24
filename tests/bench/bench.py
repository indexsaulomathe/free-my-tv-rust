#!/usr/bin/env python3
"""Python vs Rust: runs each tool N times as a fresh process and reports time and peak memory.

    python3 tests/bench/bench.py            # 20 runs each
    python3 tests/bench/bench.py --runs 50 --markdown

Measures what a person waits for: the whole command, interpreter start included. Needs the Rust CLI built
(`cd rust && cargo build --release && cargo build --release --example peak_rss`). Files the Python scripts write in the repository are restored afterwards.
"""
import argparse
import os
import platform
import statistics
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BIN = os.path.join(ROOT, 'rust', 'target', 'release', 'freemytv')


PEAK = os.path.join(ROOT, 'rust', 'target', 'release', 'examples', 'peak_rss')


def run(cmd):
    """Wall time (s) of one run."""
    start = time.perf_counter()
    if subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode != 0:
        sys.exit('failed: ' + ' '.join(cmd))
    return time.perf_counter() - start


def peak_mb(cmd):
    """Peak RSS (MB), measured by a small Rust helper: a child of Python would count Python's own memory."""
    if not os.path.exists(PEAK):
        return float('nan')
    return int(subprocess.run([PEAK, *cmd], capture_output=True, text=True, check=True).stdout) / 1024


def measure(cmd, runs):
    run(cmd)   # warm the file cache
    times = [run(cmd) for _ in range(runs)]
    return statistics.median(times), statistics.stdev(times) if runs > 1 else 0, max(peak_mb(cmd) for _ in range(3))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--runs', type=int, default=20)
    ap.add_argument('--markdown', action='store_true')
    args = ap.parse_args()
    os.chdir(ROOT)
    if not os.path.exists(BIN):
        sys.exit('Build the Rust CLI first: cd rust && cargo build --release')
    tmp = tempfile.mkdtemp()
    py = sys.executable
    ipk = os.path.join(tmp, 'bench.ipk')
    subprocess.run([BIN, 'webos-build', '--generic', '--out', ipk], check=True, capture_output=True)
    cases = [
        ('LG .ipk (webos/build.py --generic)', [py, 'webos/build.py', '--generic'],
         [BIN, 'webos-build', '--generic', '--out', os.path.join(tmp, 'r.ipk')]),
        ('Samsung .wgt (tizen/build.py --generic)', [py, 'tizen/build.py', '--generic'],
         [BIN, 'tizen-build', '--generic', '--out', os.path.join(tmp, 'r.wgt')]),
        ('Homebrew apps.json (tools/homebrew_repo.py)', [py, 'tools/homebrew_repo.py', ipk],
         [BIN, 'homebrew-repo', ipk, '--out', os.path.join(tmp, 'r.json')]),
        ('Time zones (tools/make_zones.py)', [py, 'tools/make_zones.py'],
         [BIN, 'make-zones', '--out', os.path.join(tmp, 'r.js')]),
        ('Translation check (tools/translate.py --check)', [py, 'tools/translate.py', '--check'],
         [BIN, 'translate-check']),
    ]
    rows = []
    try:
        for name, pcmd, rcmd in cases:
            p, r = measure(pcmd, args.runs), measure(rcmd, args.runs)
            rows.append((name, p, r))
    finally:
        subprocess.run(['git', 'checkout', '-q', '--', 'docs/webos/apps.json', 'web/zones.js'], cwd=ROOT)
    env = '%s, %s, Python %s, %d CPUs' % (platform.system(), platform.machine(), platform.python_version(), os.cpu_count())
    if args.markdown:
        print('Ambiente: %s. %d execuções por comando, mediana.\n' % (env, args.runs))
        print('| Ferramenta | Python | Rust | Ganho de tempo | Memória Python | Memória Rust |')
        print('|---|---:|---:|---:|---:|---:|')
        for name, p, r in rows:
            print('| %s | %.1f ms ± %.1f | %.1f ms ± %.1f | %.1f× | %.1f MB | %.1f MB |'
                  % (name, p[0] * 1000, p[1] * 1000, r[0] * 1000, r[1] * 1000, p[0] / r[0], p[2], r[2]))
    else:
        print(env)
        for name, p, r in rows:
            print('%-45s python %7.1f ms  rust %7.1f ms  %5.1fx  mem %4.0f / %4.0f MB'
                  % (name, p[0] * 1000, r[0] * 1000, p[0] / r[0], p[2], r[2]))


if __name__ == '__main__':
    main()
