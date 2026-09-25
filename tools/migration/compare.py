"""Compare baseline and migrated HTTP/CLI behavior; requires two running servers."""
import argparse
import json
import subprocess
import urllib.request
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('baseline_url')
parser.add_argument('migrated_url')
parser.add_argument('--baseline-cli', required=True)
parser.add_argument('--migrated-cli', required=True)
args = parser.parse_args()
fixtures = Path(__file__).resolve().parent / 'fixtures'


def request(base, route, payload=None):
    req = urllib.request.Request(base + route, data=payload,
                                 headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(req, timeout=120) as response:
        return json.load(response)


for route in ['meta', 'pokemon-list', 'item-list', 'move-types',
              'species-types', 'species-abilities', 'unsupported-items']:
    assert request(args.baseline_url, '/api/' + route) == request(args.migrated_url, '/api/' + route), route
    print('PASS GET', route)
for route, fixture in [('damage', 'damage'), ('survive', 'survive'), ('ko', 'ko'),
                       ('optimize/defensive', 'optimize'), ('optimize/offensive', 'optimize')]:
    payload = (fixtures / (fixture + '.json')).read_bytes()
    assert request(args.baseline_url, '/api/' + route, payload) == request(args.migrated_url, '/api/' + route, payload), route
    print('PASS POST', route)
for route in ['/', '/damage', '/survive', '/sequence', '/ko', '/optimize', '/assets/app.js', '/assets/app.css']:
    with urllib.request.urlopen(args.migrated_url + route) as response:
        assert response.status == 200 and response.read(), route
    print('PASS page/asset', route)
matchup = ['--attacker', str(fixtures / 'attacker.txt'), '--defender', str(fixtures / 'defender.txt'), '--move', 'Iron Head']
commands = [['--help'], ['parse', str(fixtures / 'attacker.txt')], ['stats', str(fixtures / 'attacker.txt')],
            ['list', 'regulation'], ['calc', *matchup], ['survive', *matchup], ['ko', *matchup],
            ['optimize', 'defensive', *matchup, '--full-spend', '--lock-atk', '0', '--lock-spa', '0', '--lock-spe', '0'],
            ['optimize', 'offensive', *matchup, '--full-spend', '--lock-atk', '0']]
for command in commands:
    before = subprocess.check_output([args.baseline_cli, *command])
    after = subprocess.check_output([args.migrated_cli, *command])
    assert before == after, command
    print('PASS CLI', command[0:2])
