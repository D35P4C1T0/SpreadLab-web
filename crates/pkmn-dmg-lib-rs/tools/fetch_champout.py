#!/usr/bin/env python3
"""Fetch pinned raw Pokemon Champions data from projectpokemon/champout.

This script vendors raw JSON into data/champions/champout/raw and updates the
source manifest with SHA-256 hashes. Runtime library code must use local data,
not GitHub URLs.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from urllib.request import urlopen


BASE = "https://raw.githubusercontent.com/projectpokemon/champout/main"
ROOT = Path(__file__).resolve().parents[1]
RAW_DIR = ROOT / "data" / "champions" / "champout" / "raw"
META_PATH = ROOT / "data" / "champions" / "champout" / "meta" / "source.json"

FILES = {
    "personal.json": f"{BASE}/masterdata/personal.json",
    "waza.json": f"{BASE}/masterdata/waza.json",
    "waza_learn.json": f"{BASE}/masterdata/waza_learn.json",
    "wazaname.json": f"{BASE}/rom-txt/usa/wazaname.json",
    "wazainfo_syn.json": f"{BASE}/rom-txt/usa/wazainfo_syn.json",
    "monsname_syn.json": f"{BASE}/rom-txt/usa/monsname_syn.json",
    "zkn_form_syn.json": f"{BASE}/rom-txt/usa/zkn_form_syn.json",
    "tokusei.json": f"{BASE}/rom-txt/usa/tokusei.json",
    "tokuseiinfo_syn.json": f"{BASE}/rom-txt/usa/tokuseiinfo_syn.json",
}


def fetch(url: str) -> bytes:
    with urlopen(url, timeout=30) as response:
        return response.read()


def main() -> None:
    RAW_DIR.mkdir(parents=True, exist_ok=True)
    META_PATH.parent.mkdir(parents=True, exist_ok=True)

    entries = []
    for name, url in FILES.items():
        data = fetch(url)
        json.loads(data.decode("utf-8"))
        path = RAW_DIR / name
        path.write_bytes(data)
        digest = hashlib.sha256(data).hexdigest()
        entries.append(
            {
                "path": f"raw/{name}",
                "url": url,
                "sha256": digest,
            }
        )
        print(f"{name}: {len(data)} bytes {digest}")

    manifest = {
        "source": {
            "repository": "https://github.com/projectpokemon/champout",
            "raw_base": BASE,
            "description": "Project Pokemon champout static JSON dumps from Pokemon Champions",
            "license": "MIT",
            "fetched_from_branch": "main",
        },
        "files": entries,
    }
    META_PATH.write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
