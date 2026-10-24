"""webos/build.py (.ipk) and tizen/build.py (.wgt): the packages the TVs install."""
import json
import zipfile

import pytest

from conftest import load, read_ar, read_tar_gz

webos_build = load('webos/build.py', 'webos_build')
tizen_build = load('tizen/build.py', 'tizen_build')


def test_ar_layout():
    data = webos_build.ar([('a', b'odd'), ('bb', b'even')])
    assert read_ar(data) == [('a', b'odd'), ('bb', b'even')]
    assert len(data) == 8 + 60 + 3 + 1 + 60 + 4     # odd members get a padding byte


def test_tar_gz_is_owned_by_root():
    members = read_tar_gz(webos_build.tar_gz([('d', None), ('d/f.txt', b'hi')]))
    info, body = members['d/f.txt']
    assert body == b'hi' and info.uid == 0 and info.gid == 0 and info.mode == 0o644
    assert members['d'][0].isdir() and members['d'][0].mode == 0o755


@pytest.fixture
def ipk(tmp_path, generic, monkeypatch):
    monkeypatch.setattr(webos_build, 'IPK', str(tmp_path / 'app.ipk'))
    webos_build.build(False, generic)
    return (tmp_path / 'app.ipk').read_bytes()


def test_ipk_members(ipk):
    names = [n for n, _ in read_ar(ipk)]
    assert names == ['debian-binary', 'control.tar.gz', 'data.tar.gz']
    assert read_ar(ipk)[0][1] == b'2.0\n'


def test_ipk_control(ipk, root):
    info = json.load(open(root + '/webos/appinfo.json'))
    control = read_tar_gz(read_ar(ipk)[1][1])['control'][1].decode()
    fields = dict(line.split(': ', 1) for line in control.strip().split('\n'))
    assert fields['Package'] == info['id'] and fields['Version'] == info['version']
    assert fields['Architecture'] == 'all' and int(fields['Installed-Size']) > 100_000


def test_ipk_data(ipk, root):
    data = read_tar_gz(read_ar(ipk)[2][1])
    base = 'usr/palm/applications/io.freemytv/'
    for name in ('appinfo.json', 'index.html', 'config.js', 'i18n.js', 'home.js', 'icon.png'):
        assert base + name in data, name
    package = json.loads(data['usr/palm/packages/io.freemytv/packageinfo.json'][1])
    assert package == {'id': 'io.freemytv', 'version': json.load(open(root + '/webos/appinfo.json'))['version'],
                       'app': 'io.freemytv'}
    # every file's folder is listed before it, as ares-package does
    seen = set()
    for name, (info, _) in data.items():
        parent = name.rsplit('/', 1)[0]
        assert '/' not in name or parent in seen, name
        seen.add(name)


def test_wgt(tmp_path, generic, monkeypatch):
    monkeypatch.setattr(tizen_build, 'WGT', str(tmp_path / 'app.wgt'))
    tizen_build.build(False, generic, None)
    with zipfile.ZipFile(tmp_path / 'app.wgt') as z:
        names = z.namelist()
        assert names == sorted(names)
        assert {'config.xml', 'icon.png', 'index.html', 'config.js'} <= set(names)
        assert b'tizen:application' in z.read('config.xml')


def test_tizen_sign_without_cli_exits(monkeypatch):
    monkeypatch.setattr(tizen_build, 'tizen_cli', lambda: None)
    with pytest.raises(SystemExit):
        tizen_build.sign('profile')
