"""webos/install.py, without a TV: the key's path, the Key Server download, and the commands it sends."""
import json
import os
import subprocess

import pytest

from conftest import load

install = load('webos/install.py', 'webos_install')


def test_key_path(monkeypatch, tmp_path):
    monkeypatch.setenv('HOME', str(tmp_path))
    assert install.key_path('192.168.1.50') == os.path.join(str(tmp_path), '.ssh', 'freemytv_webos_192_168_1_50')


def test_ssh_options_pin_the_port_and_user(monkeypatch):
    calls = []
    monkeypatch.setattr(subprocess, 'run', lambda args, **kw: calls.append(args) or subprocess.CompletedProcess(args, 0, '', ''))
    install.ssh('10.0.0.2', '/k', 'echo hi')
    args = calls[0]
    assert args[0] == 'ssh' and args[-2] == 'prisoner@10.0.0.2' and args[-1] == 'echo hi'
    assert args[args.index('-p') + 1] == '9922' and args[args.index('-i') + 1] == '/k'


def test_known_risk_host_key_is_not_checked():
    """Documents finding #4 of documentacao/seguranca.md; the Rust CLI fixes it."""
    assert 'StrictHostKeyChecking=no' in install.SSH_OPTS


def test_luna_sends_json(monkeypatch):
    sent = []

    def fake_ssh(ip, key, command, stdin=None, tty=False):
        sent.append(command)
        return subprocess.CompletedProcess([], 0, '{"returnValue":true}', '')
    monkeypatch.setattr(install, 'ssh', fake_ssh)
    out = install.luna('1.2.3.4', '/k', 'luna://com.webos.applicationManager/launch', {'id': 'io.freemytv'})
    assert '"returnValue":true' in out
    assert 'luna-send-pub' in sent[0] and 'luna://com.webos.applicationManager/launch' in sent[0]
    assert json.dumps({'id': 'io.freemytv'}).replace(' ', '') in sent[0].replace(' ', '').replace("\\", '')


def test_existing_key_is_reused(monkeypatch, tmp_path):
    monkeypatch.setenv('HOME', str(tmp_path))
    path = install.key_path('1.2.3.4')
    os.makedirs(os.path.dirname(path))
    open(path, 'w').write('key')
    monkeypatch.setattr('urllib.request.urlopen', lambda *a, **k: pytest.fail('should not download'))
    assert install.fetch_key('1.2.3.4') == path


def test_unreachable_key_server_exits(monkeypatch, tmp_path):
    monkeypatch.setenv('HOME', str(tmp_path))

    def refuse(*a, **k):
        raise OSError('connection refused')
    monkeypatch.setattr('urllib.request.urlopen', refuse)
    with pytest.raises(SystemExit) as e:
        install.fetch_key('1.2.3.4')
    assert 'Key Server' in str(e.value)


@pytest.mark.parametrize('argv', [['install.py'], ['install.py', 'tv.local'], ['install.py', '1.2.3']])
def test_bad_ip_shows_usage(monkeypatch, argv):
    monkeypatch.setattr('sys.argv', argv)
    with pytest.raises(SystemExit):
        install.main()
