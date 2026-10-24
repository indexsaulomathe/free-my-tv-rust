"""The Rust CLI (rust/target/release/freemytv) makes the same files as the Python scripts.

Skipped when the binary isn't built: `cd rust && cargo build --release`.
"""
import os
import subprocess
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
