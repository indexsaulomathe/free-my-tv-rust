"""The Rust CLI (rust/target/release/freemytv) makes the same files as the Python scripts.

Skipped when the binary isn't built: `cd rust && cargo build --release`.
"""
import os
import subprocess
import sys
import zipfile

import pytest

from conftest import ROOT, load, read_ar, read_tar_gz

BIN = os.path.join(ROOT, 'rust', 'target', 'release', 'freemytv')
pytestmark = pytest.mark.skipif(not os.path.exists(BIN), reason='Rust CLI not built')

webos_build = load('webos/build.py', 'webos_build_parity')
tizen_build = load('tizen/build.py', 'tizen_build_parity')
homebrew = load('tools/homebrew_repo.py', 'homebrew_parity')
zones = load('tools/make_zones.py', 'zones_parity')
import webapp  # noqa: E402

CONFIG = '\n'.join(['tmdb_key="k3y"', 'tmdb_region=BR', 'latitude=-23.55', 'longitude=-46.63', 'city=São Paulo',
                    'language=pt', 'currency=BRL', 'temperature=C', 'show_crypto=false', 'hide_apps=a, b']) + '\n'


def rust(*args):
    return subprocess.run([BIN, *args, '--root', ROOT], check=True, capture_output=True, text=True).stdout


def ipk_contents(data):
    members = read_ar(data)
    control = {k: v[1] for k, v in read_tar_gz(members[1][1]).items()}
    files = [(k, v[1], v[0].mode, v[0].type) for k, v in read_tar_gz(members[2][1]).items()]
    return [n for n, _ in members], control, files


@pytest.mark.parametrize('generic', [True, False])
def test_ipk(tmp_path, monkeypatch, generic):
    cfg = tmp_path / 'config.properties'
    cfg.write_text(CONFIG, encoding='utf-8')
    args = ['--generic'] if generic else ['--config', str(cfg)]
    monkeypatch.setattr(webos_build, 'IPK', str(tmp_path / 'py.ipk'))
    webos_build.build(False, webapp.config_from_args(args))
    rust('webos-build', *args, '--out', str(tmp_path / 'rs.ipk'))
    assert ipk_contents((tmp_path / 'py.ipk').read_bytes()) == ipk_contents((tmp_path / 'rs.ipk').read_bytes())


def test_wgt(tmp_path, monkeypatch):
    monkeypatch.setattr(tizen_build, 'WGT', str(tmp_path / 'py.wgt'))
    tizen_build.build(False, webapp.config_from_args(['--generic']), None)
    rust('tizen-build', '--generic', '--out', str(tmp_path / 'rs.wgt'))
    with zipfile.ZipFile(tmp_path / 'py.wgt') as a, zipfile.ZipFile(tmp_path / 'rs.wgt') as b:
        assert a.namelist() == b.namelist()
        assert all(a.read(n) == b.read(n) for n in a.namelist())


def test_homebrew_repo(tmp_path, monkeypatch):
    ipk = tmp_path / 'x.ipk'
    ipk.write_bytes(os.urandom(5000))
    monkeypatch.setattr(homebrew, 'OUT', str(tmp_path / 'py.json'))
    monkeypatch.setattr('sys.argv', ['homebrew_repo.py', str(ipk)])
    homebrew.main()
    rust('homebrew-repo', str(ipk), '--out', str(tmp_path / 'rs.json'))
    assert (tmp_path / 'py.json').read_bytes() == (tmp_path / 'rs.json').read_bytes()


@pytest.mark.skipif(not os.path.exists('/usr/share/zoneinfo/tzdata.zi'), reason='no tz database')
def test_zones(tmp_path, monkeypatch):
    (tmp_path / 'web').mkdir()
    monkeypatch.setattr(zones, 'ROOT', str(tmp_path))
    monkeypatch.setattr('sys.argv', ['make_zones.py'])
    zones.main()
    rust('make-zones', '--out', str(tmp_path / 'rs.js'))
    assert (tmp_path / 'web' / 'zones.js').read_bytes() == (tmp_path / 'rs.js').read_bytes()


def copy_for_translate(dst):
    import shutil
    for d in ('app/src', 'app/assets/i18n', 'web'):
        shutil.copytree(os.path.join(ROOT, d), dst / d)
    (dst / 'tools').mkdir()
    shutil.copy(os.path.join(ROOT, 'tools', 'translate.py'), dst / 'tools')


def both_check(root):
    py = subprocess.run(['python3', str(root / 'tools' / 'translate.py'), '--check'], capture_output=True, text=True)
    rs = subprocess.run([BIN, 'translate-check', '--root', str(root)], capture_output=True, text=True)
    return (py.returncode, py.stdout, py.stderr), (rs.returncode, rs.stdout, rs.stderr)


def test_translate_check_clean(tmp_path):
    copy_for_translate(tmp_path)
    py, rs = both_check(tmp_path)
    assert py == rs
    assert rs[0] == 0


def test_translate_check_problems(tmp_path):
    import json
    copy_for_translate(tmp_path)
    i18n = tmp_path / 'app' / 'assets' / 'i18n'
    pt = json.loads((i18n / 'pt.json').read_text(encoding='utf-8'))
    keys = list(pt)
    plural = next(k for k in keys if k.startswith('plural|'))
    fmt = next(k for k in keys if k.startswith('format|'))
    with_ph = next(k for k in keys if '{' in k and not k.startswith('plural|'))
    del pt[keys[0]]                                  # missing
    pt["it's \"obsolete\"\n"] = 'x'                  # obsolete, with quotes and a newline in its repr
    pt[plural] = 'one:{n} só'                        # no "other"
    pt[fmt] = "h:mm 'a"                              # unclosed quote
    pt[with_ph] = 'sem placeholder'                  # placeholder mismatch
    pt[keys[5]] = 3                                  # not a string
    (i18n / 'pt.json').write_text(json.dumps(pt, ensure_ascii=False), encoding='utf-8')
    en = json.loads((i18n / 'en.json').read_text(encoding='utf-8'))
    en[keys[7]] = en[keys[7]] + ' (old)'             # out of date
    (i18n / 'en.json').write_text(json.dumps(en, ensure_ascii=False), encoding='utf-8')
    (i18n / 'es.json').write_text('[1, 2]', encoding='utf-8')
    (tmp_path / 'web' / 'extra.js').write_text("var a = tr(name);\nvar b = tr('Brand new key');\n", encoding='utf-8')
    py, rs = both_check(tmp_path)
    assert py == rs
    assert rs[0] == 1 and rs[1].count('\n') > 8


FAKE_ADB = r'''#!/usr/bin/env python3
import os, sys
args = sys.argv[1:]
mode = os.environ['FAKE_MODE']
size = len(sys.stdin.buffer.read()) if 'exec-in' in args else None
with open(os.environ['FAKE_LOG'], 'a') as f:
    f.write(repr((args, size)) + '\n')
rest = args[2:] if args[:1] == ['-s'] else args
if rest[0] == 'connect':
    print('failed to connect' if mode == 'noconnect' else 'connected to ' + rest[1])
elif rest[:4] == ['shell', 'pm', 'list', 'packages']:
    print('package:io.freemytvx' if mode == 'noapp' else 'package:io.freemytv')
elif rest[:2] == ['shell', 'mkdir']:
    sys.exit(1 if mode in ('inbox', 'old') else 0)
elif rest[:3] == ['shell', 'content', 'query']:
    print('Error: Could not find provider' if mode == 'old' else 'No result found.')
'''


@pytest.mark.parametrize('mode', ['direct', 'inbox', 'old', 'noapp', 'noconnect'])
def test_photos(tmp_path, mode):
    bin_dir = tmp_path / 'bin'
    bin_dir.mkdir()
    (bin_dir / 'adb').write_text(FAKE_ADB)
    (bin_dir / 'adb').chmod(0o755)
    pics = tmp_path / 'Férias'
    for p in ['b praia.jpg', 'a#1?.PNG', '.oculta.jpg', 'nota.txt', 'sub/c.webp']:
        (pics / p).parent.mkdir(parents=True, exist_ok=True)
        (pics / p).write_bytes(os.urandom(100 + len(p)))
    args = ['192.168.1.50', str(pics) + '/', str(pics / 'nota.txt')]
    out = {}
    for who, cmd in (('py', ['python3', os.path.join(ROOT, 'tools', 'photos.py')]), ('rs', [BIN, 'photos'])):
        log = tmp_path / (who + '.log')
        env = dict(os.environ, PATH=str(bin_dir) + os.pathsep + os.environ['PATH'], FAKE_MODE=mode, FAKE_LOG=str(log))
        r = subprocess.run(cmd + args, capture_output=True, text=True, env=env)
        out[who] = (r.returncode, r.stdout, log.read_text())
    assert out['py'] == out['rs']


def art_built():
    if not os.path.exists(BIN):
        return False
    r = subprocess.run([BIN, 'make-art', '--root', '/nonexistent'], capture_output=True, text=True)
    return 'built without the art' not in r.stderr


def same_pixels(a, b):
    from PIL import Image
    x, y = Image.open(a), Image.open(b)
    return x.size == y.size and x.convert('RGBA').tobytes() == y.convert('RGBA').tobytes()


@pytest.mark.skipif(not art_built(), reason='freemytv built without --features art')
def test_make_art(tmp_path):
    pytest.importorskip('PIL')
    import shutil
    for who in ('py', 'rs'):
        (tmp_path / who / 'tools').mkdir(parents=True)
        shutil.copy(os.path.join(ROOT, 'tools', 'make_art.py'), tmp_path / who / 'tools')
        shutil.copytree(os.path.join(ROOT, 'app', 'assets', 'fonts'), tmp_path / who / 'app' / 'assets' / 'fonts')
    subprocess.run([sys.executable, str(tmp_path / 'py' / 'tools' / 'make_art.py')], check=True, capture_output=True)
    subprocess.run([BIN, 'make-art', '--root', str(tmp_path / 'rs')], check=True, capture_output=True)
    for rel in ('app/res/drawable-nodpi/icon.png', 'app/res/drawable-nodpi/banner.png', 'docs/assets/icon-512.png',
                'docs/assets/apple-touch-icon.png', 'docs/assets/favicon-64.png'):
        assert same_pixels(tmp_path / 'py' / rel, tmp_path / 'rs' / rel), rel


SYMBOLS_FONT = os.environ.get('FMTV_SYMBOLS_FONT')   # MaterialSymbolsRounded[FILL,GRAD,opsz,wght].ttf
SYMBOLS_CODEPOINTS = os.environ.get('FMTV_SYMBOLS_CODEPOINTS')


@pytest.mark.skipif(not (art_built() and SYMBOLS_FONT and SYMBOLS_CODEPOINTS),
                    reason='needs --features art and FMTV_SYMBOLS_FONT / FMTV_SYMBOLS_CODEPOINTS (the Python takes ~45 s)')
def test_make_symbols(tmp_path):
    pytest.importorskip('PIL')
    subprocess.run([sys.executable, '-I', os.path.join(ROOT, 'tools', 'make_symbols.py'), SYMBOLS_FONT,
                    SYMBOLS_CODEPOINTS, str(tmp_path / 'py')], check=True, capture_output=True)
    subprocess.run([BIN, 'make-symbols', SYMBOLS_FONT, SYMBOLS_CODEPOINTS, str(tmp_path / 'rs')], check=True,
                   capture_output=True)
    names = sorted(os.listdir(tmp_path / 'py'))
    assert names == sorted(os.listdir(tmp_path / 'rs'))
    for name in names:
        assert same_pixels(tmp_path / 'py' / name, tmp_path / 'rs' / name), name
