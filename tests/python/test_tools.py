"""tools/homebrew_repo.py, tools/make_zones.py and tools/translate.py."""
import hashlib
import json

import pytest

from conftest import load

homebrew = load('tools/homebrew_repo.py', 'homebrew_repo')
zones = load('tools/make_zones.py', 'make_zones')
import translate  # noqa: E402


# MARK: homebrew_repo

def test_homebrew_repo(tmp_path, monkeypatch, root):
    out = tmp_path / 'apps.json'
    monkeypatch.setattr(homebrew, 'OUT', str(out))
    homebrew.write('ab' * 32, 1234)
    repo = json.loads(out.read_text())
    version = json.load(open(root + '/webos/appinfo.json'))['version']
    m = repo['packages'][0]['manifest']
    assert repo['paging']['itemsTotal'] == 1
    assert m['ipkHash'] == {'sha256': 'ab' * 32} and m['ipkSize'] == 1234 and m['rootRequired'] is False
    assert m['ipkUrl'].endswith('/releases/download/v%s/FreeMyTV-webos.ipk' % version)
    assert out.read_text().endswith('}\n')


def test_homebrew_main(tmp_path, monkeypatch):
    out = tmp_path / 'apps.json'
    monkeypatch.setattr(homebrew, 'OUT', str(out))
    ipk = tmp_path / 'x.ipk'
    ipk.write_bytes(b'hello')
    monkeypatch.setattr('sys.argv', ['homebrew_repo.py', str(ipk)])
    assert homebrew.main() == 0
    m = json.loads(out.read_text())['packages'][0]['manifest']
    assert m['ipkHash']['sha256'] == hashlib.sha256(b'hello').hexdigest() and m['ipkSize'] == 5
    monkeypatch.setattr('sys.argv', ['homebrew_repo.py', 'not-an-ipk.zip'])
    assert homebrew.main() == 2


# MARK: make_zones

@pytest.mark.parametrize('text,width,want', [
    ('+4230', 2, 42.5), ('-2332', 2, -23.53), ('-0740023', 3, -74.01), ('+0000', 2, 0), ('-033200', 2, -3.53),
    ('+1393357', 3, 139.57),
])
def test_degrees(text, width, want):
    assert zones.degrees(text, width) == want


def test_make_zones(tmp_path, monkeypatch):
    base = tmp_path / 'zoneinfo'
    base.mkdir()
    (base / 'zone.tab').write_text('# comment\n\nBR\t-2332-04637\tAmerica/Sao_Paulo\tS & SE Brazil\n'
                                   'US\t+404251-0740023\tAmerica/New_York\n', encoding='utf-8')
    (base / 'tzdata.zi').write_text('L America/Sao_Paulo Brazil/East\nL America/New_York America/New_York\n'
                                    'L Nowhere/Zone Alias/X\n', encoding='utf-8')
    (tmp_path / 'web').mkdir()
    monkeypatch.setattr(zones, 'ROOT', str(tmp_path))
    monkeypatch.setattr('sys.argv', ['make_zones.py', str(base)])
    zones.main()
    js = (tmp_path / 'web' / 'zones.js').read_text(encoding='utf-8')
    data = json.loads(js.split('var ZONES = ', 1)[1].rstrip(';\n'))
    assert data == {'America/Sao_Paulo': 'BR,-23.53,-46.62,São Paulo', 'America/New_York': 'US,40.71,-74.01',
                    'Brazil/East': '=America/Sao_Paulo'}


def test_committed_zones_match_the_format(root):
    js = open(root + '/web/zones.js', encoding='utf-8').read()
    data = json.loads(js.split('var ZONES = ', 1)[1].rstrip(';\n'))
    assert len(data) > 300
    for zone, value in data.items():
        if value.startswith('='):
            assert value[1:] in data, zone
        else:
            parts = value.split(',')
            assert len(parts) in (3, 4) and len(parts[0]) == 2, zone
            float(parts[1]), float(parts[2])


# MARK: translate

def test_java_args():
    text = 'Ui.trf("{n} min, ago", "n", f(a, b)) + rest'
    args, end = translate.java_args(text, text.index('(') + 1)
    assert args == ['"{n} min, ago"', '"n"', 'f(a, b)'] and text[end] == ')'


def test_java_args_unbalanced():
    with pytest.raises((ValueError, IndexError)):
        translate.java_args('f("a", (b', 2)


def test_placeholders():
    assert translate.placeholders('{n} of {total}, {n}') == ['{n}', '{total}']
    assert translate.placeholders('no {1} {_ok}') == ['{_ok}']


@pytest.mark.parametrize('fmt,ok', [('h:mm a', True), ("HH'h'mm", True), ("d 'de' MMMM", True), ("h 'o''clock'", True), ("it''s h", False),
                                    ('h:mm q', False), ("'open", False), ('XXXX', False), ('XXX', True)])
def test_valid_format(fmt, ok):
    assert translate.valid_format(fmt) is ok


def test_fits():
    assert translate.fits('{n} min ago', '{n} min ago', 'há {n} min')
    assert not translate.fits('{n} min ago', '{n} min ago', 'há {m} min')
    assert not translate.fits('x', 'x', None)
    key = 'plural|one:{n} season|other:{n} seasons'
    assert translate.fits(key, 'one:{n} season|other:{n} seasons', 'one:{n} temporada|other:{n} temporadas')
    assert not translate.fits(key, 'one:{n} season|other:{n} seasons', 'one:{n} temporada')       # no other
    assert not translate.fits(key, 'one:{n} season|other:{n} seasons', 'one:uma|other:{n} temporadas')
    assert not translate.fits('format|h:mm', 'h:mm', 'h:mm q')


def test_problems():
    en = {'a': 'a', 'b': '{n} b', 'c': 'c'}
    cat = {'a': 'A', 'b': 'B', 'z': 'Z'}
    assert translate.problems(en, cat) == (['c'], ['z'], ['b'])


def test_repository_catalogues_are_valid():
    en = translate.extract()
    assert len(en) > 100
    assert translate.check(en) == 0
