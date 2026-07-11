# Champions unknown-species audit

Audit date: 2026-07-11

Compared the 315 entries in upstream `POKEDEX_CHAMPIONS` with the species keys
accepted by the pinned `pkmn-dmg-lib` / `spreadlab-rs` dependencies. The damage
data contains all corresponding Pokémon and forms, but 44 Showdown-style names
are not recognized as aliases.

## Base names needing a default-form alias

| Rejected selector name | Existing engine species to use as default |
| --- | --- |
| `Castform` | `Castform (Normal)` |
| `Vivillon` | Choose one pattern as the default |
| `Floette-Eternal` | `Floette (Eternal Flower)` |
| `Florges` | `Florges (Red Flower)` |
| `Furfrou` | `Furfrou (Natural Form)` |
| `Meowstic` | `Meowstic (Male)` |
| `Aegislash` | `Aegislash (Shield Forme)` |
| `Gourgeist-Average` | `Gourgeist (Medium Variety)` |
| `Lycanroc-Midday` | `Lycanroc (Midday Form)` |
| `Mimikyu` | `Mimikyu (Disguised Form)` |
| `Polteageist` | `Polteageist (Phony Form)` |
| `Alcremie` | `Alcremie (Vanilla Cream)` |
| `Morpeko` | `Morpeko (Full Belly Mode)` |
| `Basculegion` | `Basculegion (Male)` |
| `Maushold` | `Maushold (Family of Three)` |
| `Palafin` | `Palafin (Zero Form)` |
| `Sinistcha` | `Sinistcha (Unremarkable Form)` |

## Direct form-name aliases

| Rejected selector name | Existing engine species |
| --- | --- |
| `Raichu-Alola` | `Raichu (Alolan)` |
| `Ninetales-Alola` | `Ninetales (Alolan)` |
| `Arcanine-Hisui` | `Arcanine (Hisuian)` |
| `Slowbro-Galar` | `Slowbro (Galarian)` |
| `Tauros-Paldea-Combat` | `Tauros (Paldean Combat Breed)` |
| `Tauros-Paldea-Aqua` | `Tauros (Paldean Aqua Breed)` |
| `Tauros-Paldea-Blaze` | `Tauros (Paldean Blaze Breed)` |
| `Typhlosion-Hisui` | `Typhlosion (Hisuian)` |
| `Slowking-Galar` | `Slowking (Galarian)` |
| `Samurott-Hisui` | `Samurott (Hisuian)` |
| `Zoroark-Hisui` | `Zoroark (Hisuian)` |
| `Stunfisk-Galar` | `Stunfisk (Galarian)` |
| `Meowstic-F` | `Meowstic (Female)` |
| `Aegislash-Shield` | `Aegislash (Shield Forme)` |
| `Aegislash-Blade` | `Aegislash (Blade Forme)` |
| `Goodra-Hisui` | `Goodra (Hisuian)` |
| `Gourgeist-Small` | `Gourgeist (Small Variety)` |
| `Gourgeist-Large` | `Gourgeist (Large Variety)` |
| `Gourgeist-Super` | `Gourgeist (Jumbo Variety)` |
| `Avalugg-Hisui` | `Avalugg (Hisuian)` |
| `Decidueye-Hisui` | `Decidueye (Hisuian)` |
| `Lycanroc-Midnight` | `Lycanroc (Midnight Form)` |
| `Lycanroc-Dusk` | `Lycanroc (Dusk Form)` |
| `Morpeko-Hangry` | `Morpeko (Hangry Mode)` |
| `Basculegion-F` | `Basculegion (Female)` |
| `Maushold-Four` | `Maushold (Family of Four)` |
| `Palafin-Hero` | `Palafin (Hero Form)` |

## Separate UI issue

`GET /api/pokemon-list` currently combines engine species with both complete
PokéAPI species and Pokémon-name catalogs. This exposes many Pokémon outside the
Champions roster; selecting any of those also produces `unknown species`.

The selector should use the Champions roster as its base list. PokéAPI should
only supply sprites or metadata, not add selectable species. Alias support is
still required for the 44 legal Champions names above.

## Resolution

Implemented in the WebUI after this audit: the base selector now uses only
engine-backed Champions species, form names are canonicalized, and alternate
forms live in a separate conditional selector.
