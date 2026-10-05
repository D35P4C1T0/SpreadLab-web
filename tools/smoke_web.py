#!/usr/bin/env python3
"""Check real HTTP behavior, or launch a server and also verify PORT/shutdown."""
import argparse
import gzip
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
# Never route local readiness probes through a developer's HTTP proxy.
HTTP = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def request(base, route, payload=None):
    data = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(base + route, data=data,
                                 headers={'Content-Type': 'application/json'})
    with HTTP.open(req, timeout=10) as response:
        return response.status, response.read()


def ready(base, process=None):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process is not None and process.poll() is not None:
            raise RuntimeError(f'server exited during startup: {process.returncode}')
        try:
            status, body = request(base, '/healthz')
            if status == 204 and not body:
                return
        except (urllib.error.URLError, TimeoutError):
            pass
        time.sleep(0.1)
    raise RuntimeError(f'server did not become healthy: {base}')


def check(base):
    for route in ['/', '/damage', '/survive', '/sequence', '/ko', '/optimize']:
        status, body = request(base, route)
        assert status == 200 and b'<html' in body.lower(), route
    for asset in ['app.js', 'app.css']:
        status, body = request(base, '/assets/' + asset)
        assert status == 200, asset
        assert body == (ROOT / 'crates/spreadlab-web/assets' / asset).read_bytes(), asset
    status, body = request(base, '/api/meta')
    assert status == 200 and isinstance(json.loads(body), dict), 'metadata'
    fixture = ROOT / 'tools/migration/fixtures/damage.json'
    status, body = request(base, '/api/damage', json.loads(fixture.read_text()))
    result = json.loads(body)
    assert status == 200, result
    assert result['rolls'] == [63, 64, 64, 66, 66, 67, 67, 69,
                               69, 70, 70, 72, 72, 73, 73, 75], result
    assert result['summary']['ko_chance'] == 0.0, result
    survival = json.loads((ROOT / 'tools/migration/fixtures/survive.json').read_text())
    status, body = request(base, '/api/survive', survival)
    result = json.loads(body)
    assert status == 200 and result['best']['total_points'] == 0, result
    assert result['best']['result']['ko_chance'] == 0.0, result
    assert [(row['sps']['hp'], row['sps']['defense'], row['sps']['special_defense'])
            for row in result['matches']] == [(0, 0, 0), (0, 1, 0), (1, 0, 0)], result
    try:
        request(base, '/api/damage', {'attacker_set': 'not a valid request'})
    except urllib.error.HTTPError as error:
        assert 400 <= error.code < 500, error.code
        assert error.headers.get('Cache-Control') == 'no-store'
    else:
        raise AssertionError('invalid request was accepted')
    print('PASS: health, six pages, exact assets, metadata, damage rolls, survival ranking, invalid input')
    check_loading(base)


def check_loading(base):
    with HTTP.open(base + '/damage', timeout=10) as response:
        development = response.headers.get('Cache-Control') == 'no-cache'
        assert response.headers.get('Cache-Control') == (
            'no-cache' if development else 'public, max-age=60')
        html = response.read()
        assert html.count(b'rel="preload"') == 4
    policies = {
        '/assets/app.js?v=20261005-1': 'public, max-age=31536000, immutable',
        '/assets/type-icons/fire.svg': 'public, max-age=86400',
        '/api/meta': 'public, max-age=300',
    }
    for route, expected in policies.items():
        with HTTP.open(base + route, timeout=10) as response:
            assert response.headers.get('Cache-Control') == ('no-cache' if development else expected), route
    req = urllib.request.Request(base + '/assets/app.js?v=20261005-1',
                                 headers={'Accept-Encoding': 'gzip'})
    with HTTP.open(req, timeout=10) as response:
        assert response.headers.get('Content-Encoding') == 'gzip'
        assert 'accept-encoding' in response.headers.get('Vary', '').lower()
        compressed = response.read()
        original = (ROOT / 'crates/spreadlab-web/assets/app.js').read_bytes()
        assert gzip.decompress(compressed) == original
        assert len(compressed) < len(original)
    print(f'PASS: preload hints, cache policies, gzip ({len(original)} to {len(compressed)} bytes)')


def unused_port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


def launch(binary, override=False):
    port = unused_port()
    env = dict(os.environ, PORT=str(port))
    command = [str(binary), 'serve', '--host', '127.0.0.1']
    if override:
        # An explicit CLI argument must take precedence over even an invalid env value.
        env['PORT'] = 'invalid-port'
        command.extend(['--port', str(port)])
    with tempfile.TemporaryFile(mode='w+b') as log:
        process = subprocess.Popen(command, cwd=ROOT, env=env, stdout=log,
                                   stderr=subprocess.STDOUT)
        try:
            base = f'http://127.0.0.1:{port}'
            ready(base, process)
            if not override:
                check(base)
            process.terminate()
            code = process.wait(timeout=15)
            if os.name == 'posix':
                assert code == 0, f'SIGTERM was not handled cleanly: {code}'
            print('PASS: CLI port override' if override else 'PASS: PORT environment and SIGTERM shutdown')
        except BaseException:
            log.seek(0)
            print(log.read().decode(errors='replace'))
            raise
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    target = parser.add_mutually_exclusive_group()
    target.add_argument('--binary', type=Path, help='defaults to target/release/spreadlab-web')
    target.add_argument('--url', help='check an already-running server, including Docker')
    args = parser.parse_args()
    if args.url:
        base = args.url.rstrip('/')
        ready(base)
        check(base)
    else:
        binary = (args.binary or ROOT / 'target/release/spreadlab-web').resolve()
        launch(binary)
        launch(binary, override=True)


if __name__ == '__main__':
    main()
