"""Shared helpers: import the scripts from webos/, tizen/ and tools/ as modules, and read the packages they build."""
import importlib.util
import io
import os
import sys
import tarfile

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.join(ROOT, 'tools'))


def load(relpath, name):
    """Imports a script by path (webos/build.py and tizen/build.py share the name `build`)."""
    spec = importlib.util.spec_from_file_location(name, os.path.join(ROOT, relpath))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def read_ar(data):
    """[(name, bytes)] from an ar archive (the .ipk format)."""
    assert data[:8] == b'!<arch>\n'
    out, i = [], 8
    while i < len(data):
        header = data[i:i + 60]
        assert header[58:60] == b'`\n', 'bad ar header'
        name = header[:16].decode('ascii').strip()
        size = int(header[48:58].decode('ascii').strip())
        i += 60
        out.append((name, data[i:i + size]))
        i += size + (size % 2)
    return out


def read_tar_gz(data):
    """{name: TarInfo, bytes or None}."""
    out = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as tar:
        for info in tar.getmembers():
            out[info.name] = (info, tar.extractfile(info).read() if info.isfile() else None)
    return out


@pytest.fixture
def root():
    return ROOT


@pytest.fixture
def generic():
    import webapp
    return dict(webapp.GENERIC)
