# SpreadLab

SpreadLab is an alpha Pokemon Champions Stat Point optimizer for:

```text
[Gen 9 Champions] VGC 2026 Reg M-C (Bo3)
```

> Alpha status: interfaces, CLI output, public API structs, and optimizer reports may
> change while the damage library and Champions rules coverage are still moving.
> Use results as a practical helper, not as a final rules oracle.

## Ground Truth

Damage calculations are delegated to:

```toml
[dependencies]
damage_calc.workspace = true
```

This project generates legal Champions SP spreads, parses sets, and builds
damage inputs. It does not reimplement damage formulas.

## Features

- CLI for parsing Showdown sets, checking final Champions stats, running damage
  calcs, and searching offensive/defensive spreads.
- Public Rust API for external tools and visualizers.
- CLI/library-only crate. The embedded alpha WebUI was removed; see
  `handout.md` for the handoff notes for a future separate WebUI.

## Quick Start

```sh
git clone https://github.com/D35P4C1T0/SpreadLab-web.git
cd SpreadLab-web
cargo test -p spreadlab-rs
cargo run -p spreadlab-rs -- --help
```

## Quality Gate

Run this gate before committing changes:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## WASM

Build the Rust library for browser packaging:

```sh
rustup target add wasm32-unknown-unknown
cargo build -p spreadlab-rs --lib --target wasm32-unknown-unknown
```

The wasm module exports JSON-string functions for browser callers:

- `loadMetadata()`
- `calculateDamage(requestJson)`
- `calculateAllMoves(requestJson)`
- `findMinHpDefSurvival(requestJson)`
- `findMinCombinedHpDefSurvival(requestJson)`
- `findMinOffensiveKo(requestJson)`
- `runDefensiveOptimization(requestJson)`
- `runOffensiveOptimization(requestJson)`

Use `wasm-bindgen` or `wasm-pack` to generate JavaScript glue for the browser.

## Damage library update (Reg M-C)

The pinned revision is `96f55ef04e66d882457af4f009f228de0e73afdc`.
Metadata and `list regulation` now use Regulation M-C. The Rust
`regulation_m_b_names()` method remains available for older consumers.
Move flags, fixed hit counts, and automatic critical hits come from upstream's
typed move metadata. Item and ability names resolve through upstream tables;
explicit `Ability On:` lines override the upstream default toggle state.

`DamageResponse` now includes `outcome`, `resolved_move`, `defender_hp_delta`,
`attacker_hp_effects`, and `ko_chance_by_move_use`. Signed HP effects use positive
values for damage and negative values for healing; attacker effects are partial
components, not total recoil/drain predictions.

`calculate_all_moves_request(AllMovesRequest)` (WASM: `calculateAllMoves`) accepts
`left_set`, `right_set`, four names each in `left_moves` and `right_moves`, and
optional `left_to_right_field` / `right_to_left_field`. It returns upstream's
`BatchDamageResult`, including both resulting Pokemon states. Initial boosts
come from the attacker/defender boosts in `left_to_right_field`.

`FieldRequest` additionally accepts `defender_stealth_rock`, `defender_spikes`
(clamped to 0–3), `defender_salt_cure`, `defender_aqua_ring`, `ingrain`,
`defender_nightmare`, `defender_curse`, `defender_binding`, and
`defender_sea_of_fire`. All default to false/zero.

Upstream's revised cumulative KO projection changes berry odds and does not
apply Focus Sash survival. Single-benchmark searches follow that projection.
The separate multi-attacker sequence search retains its existing turn-state
model (including Focus Sash); it is not an upstream full battle simulation.

## Local Damage Library Development

Edit `crates/pkmn-dmg-lib-rs` in this workspace and rebuild. No Git patches,
publication, or revision updates are required. Commands below run from the
monorepo root.

## Commands

Parse a Showdown set:

```sh
cargo run -p spreadlab-rs -- parse set.txt
```

Print final raw Champions stats:

```sh
cargo run -p spreadlab-rs -- stats set.txt
```

Run one damage calculation:

```sh
cargo run -p spreadlab-rs -- calc --attacker attacker.txt --defender defender.txt --move "Flamethrower"
```

Force a critical hit for damage, survival, KO, or one-off optimization benchmarks:

```sh
cargo run -p spreadlab-rs -- calc --attacker attacker.txt --defender defender.txt --move "Flamethrower" --crit
cargo run -p spreadlab-rs -- survive --attacker attacker.txt --defender defender.txt --move "Iron Head" --crit
```

Print pinned Champions names from the damage library:

```sh
cargo run -p spreadlab-rs -- list species
cargo run -p spreadlab-rs -- list regulation
cargo run -p spreadlab-rs -- list items
cargo run -p spreadlab-rs -- list abilities
cargo run -p spreadlab-rs -- list moves
```

Search defensive spreads for one benchmark:

```sh
cargo run -p spreadlab-rs -- optimize defensive --attacker attacker.txt --defender defender.txt --move "Close Combat" --full-spend --lock-atk 0 --lock-spa 0 --lock-spe 0
```

Search offensive spreads for one benchmark:

```sh
cargo run -p spreadlab-rs -- optimize offensive --attacker attacker.txt --defender defender.txt --move "Flamethrower" --full-spend --lock-atk 0
```

Find minimum offensive investment for a guaranteed KO:

```sh
cargo run -p spreadlab-rs -- ko --attacker attacker.txt --defender defender.txt --move "Last Respects" --move-times-affected 1 --min-ko-chance 1.0
```

Search all natures (otherwise preserve the parsed nature):

```sh
cargo run -p spreadlab-rs -- ko --attacker attacker.txt --defender defender.txt --move "Last Respects" --move-times-affected 1 --min-ko-chance 1.0 --optimize-nature
cargo run -p spreadlab-rs -- survive --attacker attacker.txt --defender defender.txt --move "Iron Head" --max-ko-chance 0.125 --optimize-nature
```

Start a defensive search from partial HP:

```sh
cargo run -p spreadlab-rs -- survive --attacker attacker.txt --defender defender.txt --move "Iron Head" --hp-percent 75 --max-ko-chance 0.125
```

Find a spread that survives two attacks in a row:

```sh
cargo run -p spreadlab-rs -- survive-sequence --attacker1 attacker-a.txt --move1 "Iron Head" --attacker2 attacker-b.txt --move2 "Rock Slide" --defender defender.txt --max-ko-chance 0.125 --hp-percent 100 --optimize-nature
```

Show closest failing spread when nothing satisfies the requested chance:

```sh
cargo run -p spreadlab-rs -- survive --attacker attacker.txt --defender defender.txt --move "Rock Slide" --max-ko-chance 0 --optimize-nature --show-closest-miss
cargo run -p spreadlab-rs -- ko --attacker attacker.txt --defender defender.txt --move "Last Respects" --min-ko-chance 1 --show-closest-miss
```

Search against multiple benchmarks:

```sh
cargo run -p spreadlab-rs -- optimize defensive --benchmarks benchmarks.json --full-spend --lock-atk 0 --lock-spa 0 --lock-spe 0
```

`benchmarks.json`:

```json
{
  "benchmarks": [
    {
      "attacker": "Charizard-Mega-Y @ Charizardite Y\nAbility: Solar Power\nSPs: 2 HP / 32 SpA / 32 Spe\nTimid Nature\n- Flamethrower",
      "defender": "Venusaur @ Sitrus Berry\nAbility: Overgrow\nSPs: 32 HP / 32 SpD / 2 Spe\nCalm Nature\n- Protect",
      "move": "Flamethrower",
      "critical": false
    }
  ]
}
```

## Status

Implemented first:

- Champions `SPs:` parser and canonical export
- Low-value `EVs:` parser for Champions point exports where all values are
  `<= 32`
- Legacy `EVs:` to `SPs:` conversion with `floor((EV + 4) / 8)` when any value
  is greater than `32`
- stat conversion wrapper around `pkmn-dmg-lib-rs`
- Champions data resolver from `damage_calc::data::CHAMPIONS_DATA_JSON`
- pinned Champions species/item/ability lists from `pkmn-dmg-lib-rs`
- legal SP spread generation
- single benchmark damage bridge
- basic ranked defensive/offensive search
- normalized item/ability resolver for damage-lib enum names
- JSON benchmark files for batch defensive/offensive searches
- public API methods for external visualizers:
  - `calculate_damage_request`
  - `calculate_damage_request_with_data`
  - `find_min_hp_def_survival`
  - `find_min_hp_def_survival_with_data`
  - `find_min_combined_hp_def_survival`
  - `find_min_combined_hp_def_survival_with_data`
  - `find_min_offensive_ko`
  - `find_min_offensive_ko_with_data`

## Library API Example

```rust
use spreadlab_rs::api::{
    find_min_hp_def_survival, HpDefSurvivalRequest,
};

let result = find_min_hp_def_survival(HpDefSurvivalRequest {
    search: None,
    attacker_set: "Kingambit\nAbility: Defiant\nSPs: 32 Atk\nAdamant Nature\n- Iron Head".into(),
    defender_set: "Mega Floette\n- Protect".into(),
    move_name: "Iron Head".into(),
    max_ko_chance: 0.125,
    hp_percent: None,
    nature: None,
    optimize_nature: true,
    limit: 10,
    move_times_affected: 0,
    critical: false,
    field: None,
})?;

let best = result.best.expect("at least one survival spread");
assert_eq!(best.total_points, 24);
```

Survival searches now enumerate HP/Defense/SpD exactly. Use
`find_min_survival(SurvivalRequest)` / WASM `findMinSurvival` for independent
benchmarks, explicit sequence timing, locks, lower bounds, budgets, all global
minima, or Pareto results. CLI: `cargo run -p spreadlab-rs -- survive-exact examples/survival.json`.
See [exact search contract and migration guide](docs/survival-search.md).

A defensive stat the pinned engine cannot read is not searched: an ordinary
physical-only benchmark never invests SpD, and an ordinary special-only benchmark
never invests Defense. The stat is held at its lock or lower bound, so add
`minimum` or `locked` when a specific value matters. Mixed benchmark sets keep
both defenses, Psyshock keeps the Defense it resolves against, and the reported
minimum cost and KO probabilities are unaffected by the narrower domain.

`optimize defensive`/`optimize offensive` rank scores rather than minimum
investment, and they apply the same rule whenever the request is not a full-spend
one, so ranked rows no longer differ only in an irrelevant defensive stat.
Unrelated offensive stats are still generated and ranked as before. A full-spend
request still has to reach its exact total: with the offensive stats locked, the
remaining points land in whatever free stat can absorb them (`32 HP / 32 Def /
2 SpD` for a physical benchmark), which is the requested total, not freely varied
investment.

Still to build:

- richer item/ability resolver coverage
- report output for ranked results

## License

MIT. See [LICENSE](LICENSE).

## Ability state and viewer integration

`Ability Enabled: false` disables the ability for damage, move construction,
entry effects, and optimization. It defaults to true, including deserialization
of older `ParsedSet` values. `Ability On:` keeps its existing meaning: conditional
activation, such as Flash Fire's boost. Turning that activation off does not
remove passive effects such as Flash Fire's immunity.

Viewers should submit selected abilities and explicit field/stage inputs without
applying ability rules themselves. The damage library handles entry effects and
move interactions. In particular, Mega Sol supplies personal sunlight only for
its user's moves; it must not set shared weather. Intimidate must not be applied
to input stages by a viewer, since engine preprocessing already applies it.

The pinned engine models Piercing Drill's quarter damage through Protect but
does not block ordinary attacks when Protect is selected. Seed Sower's terrain,
Emergency Exit's switch, and Thermal Exchange's Attack boost are not simulated
across turns. Set subsequent battle state explicitly. Spicy Spray does apply
burn between hits of a multi-hit move; separate calculations do not persist it.
