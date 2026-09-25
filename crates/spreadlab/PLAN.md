# Pokemon Champions Stat Point Optimizer Plan

## Scope

Target format:

```text
[Gen 9 Champions] VGC 2026 Reg M-A (Bo3)
```

Showdown / Smogon monthly stats format id:

```text
gen9championsvgc2026regmabo3
```

Processed pkmn format metadata id:

```text
championsvgc2026regmabo3
```

Confirmed display name from `https://pkmn.github.io/smogon/data/formats/index.json`:

```text
Champions VGC 2026 Reg M-A (Bo3)
```

First version supports only:

- Defensive optimization
- Offensive optimization

This project must not become a general team optimizer yet. It should behave like
a smarter batch damage calculator that searches legal Pokemon Champions Stat
Point spreads.

Out of scope for v1:

- general team optimization
- automatic full metagame spread recommendations
- speed-tier optimization, except as a small helper if naturally needed
- team synergy logic
- automatic full moveset generation beyond requested calculations

## Repositories Inspected

### `pkmn-dmg-lib-rs`

Repository:

```text
https://github.com/D35P4C1T0/pkmn-dmg-lib-rs.git
```

Important findings:

- Crate package name: `pkmn-dmg-lib`
- Library crate name: `damage_calc`
- The crate exposes:
  - `calculate_damage`
  - `CalcInput`
  - `DamageResult`
  - `Pokemon`
  - `Move`
  - `Field`
  - `Ruleset`
  - `Nature`
  - `StatTable`
  - `calculate_stats`
  - `calculate_hp`
  - `calculate_non_hp_stat`
- `Pokemon::champions(...)` constructs level-50 Champions Pokemon.
- Damage input currently takes base stats, stat points, and nature. The damage
  library calculates final raw stats internally through `calculate_stats`.
- `DamageResult` includes:
  - `min_damage`
  - `max_damage`
  - `damage_rolls`
  - `hit_rolls`
  - `percent_range`
  - `ko_chance`
  - `applied_modifiers`
  - `debug`
- The crate exposes generated Champions data as:

```rust
damage_calc::data::CHAMPIONS_DATA_JSON
```

The README says parity is about 89% against the JavaScript Champions calculator.
Known gaps mostly involve extra battle-state modeling, not basic stat or damage
arithmetic.

### `ChampCalc`

Repository:

```text
https://github.com/D35P4C1T0/ChampCalc
```

Important findings:

- Canonical investment format is Champions `SPs:`.
- Legacy `EVs:` are only import/export compatibility.
- Total Champions Stat Point cap: `66`.
- Per-stat cap: `32`.
- EV import converts each stat independently with:

```text
floor((EV + 4) / 8)
```

- Examples:
  - `0 EV -> 0 SP`
  - `4 EV -> 1 SP`
  - `12 EV -> 2 SP`
  - `252 EV -> 32 SP`
- Leftover points are preserved. Import does not auto-fill to 66.
- `SPs:` values above `32` are clamped per stat before total validation.
- Mixed `EVs:` and `SPs:` lines are rejected.
- Approximate legacy EV export uses:

```text
4 + (SP - 1) * 8
```

for positive SP values, capped at `252`.

## Champions Stat Formula

Use the damage library as ground truth.

Verified in `pkmn-dmg-lib-rs/src/stats.rs`.

HP:

```text
floor((base * 2 + 31) * 50 / 100) + 50 + 10 + SP
```

Special case:

```text
base HP == 1 -> final HP = 1
```

Non-HP:

```text
floor((floor((base * 2 + 31) * 50 / 100) + 5 + SP) * nature_modifier)
```

Nature modifier is represented with integer math in the damage library:

- boosted stat: `before_nature * 11 / 10`
- lowered stat: `before_nature * 9 / 10`
- neutral stat: unchanged

ChampCalc UI shows the simplified level-50, 31-IV form:

- HP: `base + SP + 75`
- Non-HP: `floor(nature * (base + SP + 20))`

The Rust optimizer should expose:

```rust
fn champions_final_stats(
    base_stats: BaseStats,
    nature: Nature,
    sps: StatPoints,
) -> FinalStats
```

Implementation should delegate to `damage_calc::calculate_stats` where possible,
so the stat layer stays aligned with the damage library.

## Damage Dependency

Initial dependency:

```toml
damage_calc = { package = "pkmn-dmg-lib", git = "https://github.com/D35P4C1T0/pkmn-dmg-lib-rs.git", features = ["serde"] }
```

README should document local development override:

```toml
[patch."https://github.com/D35P4C1T0/pkmn-dmg-lib-rs.git"]
pkmn-dmg-lib = { path = "../pkmn-dmg-lib-rs" }
```

Rule:

Never reimplement damage formulas.

The optimizer may:

- parse inputs
- generate legal SP spreads
- convert Champions SPs into final raw stats for reporting
- build `damage_calc` input objects

But all damage rolls, min/max damage, percent ranges, and KO chances must come
from `pkmn-dmg-lib-rs`.

## Public Data Sources

### Format Metadata

URL:

```text
https://pkmn.github.io/smogon/data/formats/index.json
```

Confirmed entries:

```json
"championsvgc2026regma": "Champions VGC 2026 Reg M-A"
"championsvgc2026regmabo3": "Champions VGC 2026 Reg M-A (Bo3)"
```

### Monthly Smogon Chaos Data

Primary URL pattern:

```text
https://www.smogon.com/stats/{YYYY-MM}/chaos/{format}-{rating}.json.gz
```

Fallback URL pattern:

```text
https://www.smogon.com/stats/{YYYY-MM}/chaos/{format}-{rating}.json
```

Defaults:

```text
format = gen9championsvgc2026regmabo3
rating = 1760
month = latest available month
```

Supported ratings:

```text
0
1500
1630
1760
```

Confirmed sample:

```text
https://www.smogon.com/stats/2026-05/chaos/gen9championsvgc2026regmabo3-1760.json.gz
```

On June 9, 2026:

- `2026-05` gzip exists
- `2026-06` gzip returns 404

Sample `2026-05`, `1760` info:

```json
{
  "metagame": "gen9championsvgc2026regmabo3",
  "cutoff": 1760,
  "cutoff deviation": 0,
  "team type": null,
  "number of battles": 307577
}
```

Sample data shape per Pokemon includes:

- `Raw count`
- `usage`
- `Viability Ceiling`
- `Abilities`
- `Items`
- `Spreads`
- `Moves`
- `Tera Types`
- `Happiness`
- `Teammates`
- `Checks and Counters`

Observed `2026-05`, `1760` sample:

- Pokemon entries: `170`
- First entries: `Basculegion`, `Kingambit`, `Garchomp`, `Sneasler`, `Charizard-Mega-Y`

Spread keys use:

```text
Nature:hp/atk/def/spa/spd/spe
```

Example:

```text
Jolly:0/32/2/0/0/32
```

## Proposed Project Structure

```text
src/
  lib.rs
  main.rs
  stats.rs
  showdown.rs
  data.rs
  smogon.rs
  spreads.rs
  damage_bridge.rs
  optimize/
    mod.rs
    defensive.rs
    offensive.rs
    scoring.rs
  report.rs
tests/
  stats.rs
  showdown.rs
  smogon.rs
  optimize.rs
```

## Proposed Dependencies

```toml
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
directories = "5"
flate2 = "1"
reqwest = { version = "0.12", features = ["blocking", "json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
damage_calc = { package = "pkmn-dmg-lib", git = "https://github.com/D35P4C1T0/pkmn-dmg-lib-rs.git", features = ["serde"] }
```

Adjust versions if project policy requires newer or pinned versions.

## Core Types

### Stat Types

```rust
struct BaseStats {
    hp: u16,
    attack: u16,
    defense: u16,
    special_attack: u16,
    special_defense: u16,
    speed: u16,
}

struct StatPoints {
    hp: u16,
    attack: u16,
    defense: u16,
    special_attack: u16,
    special_defense: u16,
    speed: u16,
}

struct FinalStats {
    hp: u16,
    attack: u16,
    defense: u16,
    special_attack: u16,
    special_defense: u16,
    speed: u16,
}
```

Validation:

- each SP value must be `<= 32`
- total SP must be `<= 66`
- do not auto-fill leftover SPs

### Metagame Stats Types

```rust
struct MetagameStats {
    format_id: String,
    display_name: String,
    month: String,
    rating: u16,
    battles: u64,
    pokemon: Vec<PokemonUsage>,
}

struct PokemonUsage {
    name: String,
    usage: f64,
    raw_count: u64,
    abilities: Vec<WeightedName>,
    items: Vec<WeightedName>,
    moves: Vec<WeightedName>,
    spreads: Vec<WeightedSpread>,
    tera_types: Vec<WeightedName>,
    teammates: Vec<WeightedName>,
}

struct WeightedName {
    name: String,
    weight: f64,
}

struct WeightedSpread {
    nature: Nature,
    sps: StatPoints,
    weight: f64,
}
```

## Implementation Phases

### Phase 1: Rust Project Skeleton

Create the crate with:

- library API
- CLI binary
- error types
- module boundaries
- README with dependency and local path override docs

CLI commands:

```text
fetch
calc
optimize defensive
optimize offensive
```

### Phase 2: Champions Stat Layer

Implement:

```rust
fn champions_final_stats(
    base_stats: BaseStats,
    nature: Nature,
    sps: StatPoints,
) -> Result<FinalStats, Error>
```

Use `damage_calc::Pokemon::champions` and `damage_calc::calculate_stats`.

Also implement:

- `StatPoints::total()`
- `StatPoints::validate()`
- conversion to/from `damage_calc::StatTable`
- tests for formula and validation

### Phase 3: Showdown Import/Export

Implement parser for:

- species
- item
- ability
- nature
- tera type
- moves
- `SPs:`
- legacy `EVs:`

Rules:

- allow exactly one training line type
- reject mixed `EVs:` and `SPs:`
- `EVs:` convert independently with `floor((EV + 4) / 8)`
- `SPs:` parse directly, clamped per stat to 32
- reject total SP over 66
- preserve leftover points

Export:

- canonical `SPs:`
- optional approximate legacy `EVs:`

### Phase 4: Champions Data Loading

Use `damage_calc::data::CHAMPIONS_DATA_JSON` to build local lookup tables for:

- species names
- forms
- types
- base stats
- weights
- moves
- abilities
- items

Provide a resolver that turns Showdown/Smogon names into damage-lib input data.

Handle naming differences carefully:

- case normalization
- spaces/hyphens
- Mega forms
- regional forms
- `nothing` item/tera values from chaos stats

### Phase 5: Smogon Fetcher and Cache

Implement:

```text
fetch --month latest --rating 1760
fetch --month 2026-05 --rating 1760
```

Fetcher behavior:

- if `latest`, start with current month and probe backward
- try `.json.gz` first
- if 404, try `.json`
- cache decompressed JSON locally
- normalize into `MetagameStats`

Cache key:

```text
{month}/{format}-{rating}.json
```

Recommended cache root:

```text
<platform cache dir>/battle-optimizer-rs/smogon/
```

### Phase 6: Damage Bridge

Build functions that convert parsed sets and generated spreads into
`damage_calc::CalcInput`.

Responsibilities:

- construct attacker `Pokemon`
- construct defender `Pokemon`
- construct `Move`
- apply ability/item/status/boosts/field when specified
- call `damage_calc::calculate_damage`
- expose report-friendly result type

Do not compute any damage locally.

### Phase 7: Spread Generation

Generate legal Champions SP spreads:

- six stats
- each stat `0..=32`
- total `<=66`
- optional exact `66` mode
- support locked stats
- support imported baseline spread

For performance, add pruning options:

- only vary relevant stats for mode
- deduplicate equivalent final stats
- early reject spreads that cannot affect benchmark outcome
- keep top N ranked results

### Phase 8: Defensive Optimization

Input:

- defender set
- one or more attacker benchmarks
- optional locked defender stats
- optional full-spend flag

Benchmark includes:

- attacking Pokemon set
- move
- field
- required survival target, such as avoid OHKO or avoid 2HKO

Scoring ideas:

- prefer guaranteed survival over chance-based survival
- minimize max damage percent
- minimize KO chance
- preserve leftover points when outcomes tie
- tie-break by lower total SP, then stable stat order

Output:

- ranked spreads
- canonical `SPs:` line
- raw final stats
- per-benchmark min/max damage
- percent range
- KO chance
- pass/fail target labels

### Phase 9: Offensive Optimization

Input:

- attacker set
- one or more defender benchmarks
- optional locked attacker stats
- optional full-spend flag

Benchmark includes:

- defender Pokemon set
- move
- field
- required KO target, such as guaranteed OHKO, chance OHKO, or 2HKO

Scoring ideas:

- prefer guaranteed KO over chance-based KO
- maximize min damage percent
- maximize KO chance
- preserve leftover points when outcomes tie
- tie-break by lower total SP, then stable stat order

Output:

- ranked spreads
- canonical `SPs:` line
- raw final stats
- per-benchmark min/max damage
- percent range
- KO chance
- pass/fail target labels

### Phase 10: CLI Reporting

Text output should be readable and copy/paste-friendly.

Example shape:

```text
Rank 1
SPs: 32 HP / 4 Def / 30 SpD
Final stats: 181 / 120 / 116 / 100 / 142 / 90

Benchmark: +252 Atk Examplemon Close Combat
Damage: 144-170 (79.5-93.9%)
KO chance: 0.0%
Target: survives OHKO - pass
```

Also consider JSON output:

```text
--json
```

for scripting.

## Testing Plan

### Unit Tests

Stat conversion:

- `0 EV -> 0 SP`
- `4 EV -> 1 SP`
- `12 EV -> 2 SP`
- `252 EV -> 32 SP`
- per-stat SP cap `32`
- total SP cap `66`
- leftover preserved

Showdown parsing:

- `SPs:` direct parse
- `EVs:` legacy parse
- mixed `EVs:` and `SPs:` rejected
- duplicate training lines rejected
- malformed line rejected
- canonical SP export
- approximate legacy EV export

Stats:

- HP formula
- base HP `1 -> 1`
- non-HP neutral nature
- boosted nature
- lowered nature
- compare wrapper output to `damage_calc::calculate_stats`

Chaos data:

- parse gzip sample
- normalize `info`
- normalize weighted names
- parse spread key `Jolly:0/32/2/0/0/32`

### Integration Tests

- Build one attacker and defender from parsed inputs.
- Run `damage_calc::calculate_damage`.
- Assert damage rolls come from the library.
- Run one defensive search with tiny constrained spread space.
- Run one offensive search with tiny constrained spread space.

## Important Design Choices

- Damage library remains ground truth.
- Stat wrapper delegates to damage library instead of duplicating formula.
- `SPs:` are canonical everywhere internally.
- `EVs:` exist only at parser/export boundary.
- Leftover SP budget is preserved unless a mode explicitly asks to spend full
  66.
- Smogon stats are optional metagame context, not required for direct user
  benchmarks.
- v1 optimizes requested benchmarks only. No automatic metagame-wide spread
  recommendations yet.

## Open Questions

- Exact benchmark input format: TOML, JSON, YAML, or Showdown-like blocks.
- Whether to expose both library API and CLI in v1, or CLI only with internal
  library modules.
- How much resolver logic should live here versus upstream in `pkmn-dmg-lib-rs`.
- Whether optimizer should include common field presets for Bo3 VGC doubles.
- Whether full-spend should be default for optimization, or opt-in. Prompt says
  leftover points must be preserved unless optimization explicitly requires full
  66, so safest default is no auto-fill.
