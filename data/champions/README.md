# Champions Raw Data

This folder stores source-format data used by the Pokemon Champions calculator
port before normalization into Rust models.

- `items.json`: Champions item names.
- `regulation_m_a_pokemon.json`: current Regulation M-A legal Pokemon roster.
- `regulation_m_b_additions.json`: Regulation M-B Pokemon added on top of the
  Regulation M-A roster.
- `regulation_m_c_additions.json`: Regulation M-C additions over M-B (26 regular
  roster entries for 23 species, plus six Megas). Squawkabilly includes all four
  plumage forms in the normalized data.
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

Regulation M-C was checked against the [official announcement](https://news.pokemon-home.com/en/page/816.html)
on 2026-09-11. Stats, types, abilities, weights, and learnsets were refreshed from
[Project Pokemon champout](https://github.com/projectpokemon/champout); file hashes
are recorded in `champout/meta/source.json`. Item and Mega Stone names were also
checked against the [Champions item list](https://www.serebii.net/pokemonchampions/items.shtml).

`CHAMPIONS_CURRENT_SPECIES` / `champions_current_species` expose typed source-dump
metadata for constructing calculator inputs. `CHAMPIONS_ITEM_VALUES` exposes all
current typed items. The `CHAMPIONS_REFERENCE_*` APIs remain tied to the pinned
M-B browser oracle; its parity claim does not cover M-C additions.

Aura Guard is implemented as a contact damage reducer. Air Balloon, Normal Gem,
and terrain Seeds use the existing damage mechanics. Leek's critical-hit chance,
Rocky Helmet retaliation, switching items, trapping duration, and terrain duration
are outside the single-attack damage calculation; use explicit critical-hit and
battle-state inputs where applicable. Items still have typed identities and Fling
power for damage calculations.
