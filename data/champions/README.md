# Champions Raw Data

This folder stores source-format data used by the Pokemon Champions calculator
port before normalization into Rust models.

- `items.json`: Champions item names.
- `regulation_m_a_pokemon.json`: current Regulation M-A legal Pokemon roster.
- `regulation_m_b_additions.json`: Regulation M-B Pokemon added on top of the
  Regulation M-A roster.
- `champout/raw/*.json`: vendored Project Pokemon `champout` dumps used as a
  richer source for Pokemon/forms, moves, learnsets, and English text.
- `champout/meta/source.json`: source URLs and SHA-256 hashes for the vendored
  raw files.
- `generated/champions-data.json`: normalized data generated from the vendored
  `champout` dumps by `tools/normalize_champout.py`.
- `generated/reference-inventory.json`: exact active normal-dex inventory and
  bundled sets extracted from the pinned NCP JavaScript reference.

Reference source hashes, scope classification, oracle usage, and inventory
regeneration are documented under `tools/reference/`.

Refresh the vendored data with:

```bash
tools/fetch_champout.py
tools/normalize_champout.py
tools/generate_champions_rust.py
```
