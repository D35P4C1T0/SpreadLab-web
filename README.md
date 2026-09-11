# pkmn-dmg-lib

Rust library port of the Nimbasa City Post web damage calculator, focused on the
Pokemon Champions ruleset.

The JavaScript implementation in `reference/NCP-VGC-Damage-Calculator` is the
source of truth. The Rust code mirrors the Champions path through
`script_res/damage_SV.js` and shared helpers in `script_res/damage_MASTER.js`,
especially integer floors, Game Freak rounding, and modifier chaining.

## Status

**100% parity for the pinned VGC 2026 Champions Regulation M-B normal-dex
scope.** The parity baseline is upstream commit
`dfbf020d4ed7df8921c6e11bbaa23410f6ca1448`.

The suite executes the pinned JavaScript engine and compares more than 18,000
generated partitions covering every active move, bundled set, ability and item
in both roles, field and battle-state boundaries, entry preprocessing,
multi-hit recalculation, counter moves, and KO probabilities. Source hashes,
exact inventories, and all 97 relevant reference functions are checked in CI;
current unexplained mismatch count is zero.

This claim excludes legacy generations, Champions NatDex, Legends Z-A, and
browser presentation. See [Scope Boundary](#scope-boundary).

## Example

```rust
use damage_calc::{
    calculate_damage, CalcInput, Category, Field, Move, Nature, Pokemon, PokemonType,
    Ruleset, StatTable,
};

let attacker = Pokemon::champions(
    "Attacker",
    [Some(PokemonType::Fire), None],
    StatTable::new(100, 100, 100, 100, 100, 100),
    StatTable::new(0, 0, 0, 0, 0, 0),
    Nature::Hardy,
);
let defender = Pokemon::champions(
    "Defender",
    [Some(PokemonType::Grass), None],
    StatTable::new(100, 100, 100, 100, 100, 100),
    StatTable::new(0, 0, 0, 0, 0, 0),
    Nature::Hardy,
);
let move_ = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);

let result = calculate_damage(CalcInput {
    attacker,
    defender,
    move_,
    field: Field::default(),
    ruleset: Ruleset::Champions,
})?;

assert_eq!(result.damage_rolls.len(), 16);
assert_eq!(result.hit_rolls.len(), 1);
assert_eq!(result.ko_chance_by_move_use.len(), 4);
# Ok::<(), damage_calc::CalcError>(())
```

`result.ko_chance` is the chance to KO with this move use. For UI text such as
`guaranteed 2HKO after Sitrus Berry recovery`, use
`result.ko_chance_by_move_use`: it reports cumulative KO odds after 1, 2, 3,
and 4 uses while modeling one consumed defender healing item after every hit.
This includes mid-move recovery for multi-hit attacks.

`Field::default()` assumes Doubles. Named Champions spread moves such as Rock
Slide automatically receive the Doubles 0.75 spread modifier. To calculate a
spread move into only one target, set:

```rust
let mut rock_slide = Move::new("Rock Slide", 75, PokemonType::Rock, Category::Physical);
rock_slide.targets_single_target = true;
```

For the reference calculator's full eight-result pass, use
`calculate_all_moves(BatchCalcInput { ... })`. It evaluates four moves in both
directions after applying entry preprocessing once. Direction-oriented fields
allow each side to carry its own screens, hazards, ally effects, and Protect
state. Batch counter moves automatically calculate their selected opposing
move; single-move callers can provide prior rolls explicitly.

## Champions Data API

The crate vendors pinned Champions data so downstream tools do not need to
fetch Pokemon, item, or ability lists.

```rust
use damage_calc::data::champions::{
    champions_ability, champions_item, champions_reference_move,
    champions_reference_set, champions_reference_species, regulation_m_a_pokemon,
    regulation_m_b_pokemon, CHAMPIONS_ABILITIES, CHAMPIONS_ITEMS,
    CHAMPIONS_REFERENCE_MOVES, CHAMPIONS_REFERENCE_SETS,
    CHAMPIONS_REFERENCE_SPECIES, CHAMPIONS_SPECIES, REGULATION_M_A_POKEMON,
    REGULATION_M_B_POKEMON,
};

assert!(CHAMPIONS_ITEMS.contains(&"Light Ball"));
assert!(REGULATION_M_A_POKEMON.contains(&"Hydrapple"));
assert!(REGULATION_M_B_POKEMON.contains(&"Mega Raichu X"));
assert_eq!(champions_item("Scope Lens"), Some("Scope Lens"));
assert_eq!(regulation_m_a_pokemon("Venusaur"), Some("Venusaur"));
assert_eq!(regulation_m_b_pokemon("Mega Raichu X"), Some("Mega Raichu X"));
assert_eq!(champions_ability("Overgrow").unwrap().id, 65);

// Lightweight full lists for menus and optimizer setup.
let item_count = CHAMPIONS_ITEMS.len();
let ability_count = CHAMPIONS_ABILITIES.len();
let species_form_count = CHAMPIONS_SPECIES.len();

// Exact reference metadata produces battle-ready typed values.
let body_slam = champions_reference_move("Body Slam").unwrap().move_();
let venusaur = champions_reference_species("Venusaur").unwrap();
let set = champions_reference_set("Venusaur", "Sun Sleep Offense").unwrap();
let pokemon = set.pokemon_().unwrap();
let moves = set.moves_().unwrap();
assert_eq!(body_slam.name, "Body Slam");
assert_eq!(venusaur.name, "Venusaur");
assert_eq!(pokemon.name, "Venusaur");
assert_eq!(moves.len(), 4);
assert_eq!(CHAMPIONS_REFERENCE_SPECIES.len(), 315);
assert_eq!(CHAMPIONS_REFERENCE_MOVES.len(), 496);
assert_eq!(CHAMPIONS_REFERENCE_SETS.len(), 123);
```

For richer imports, `damage_calc::data::CHAMPIONS_DATA_JSON` still exposes the
full normalized JSON with species/forms, base stats, legal moves, moves, and
ability descriptions.

## Current Parity Coverage

Implemented and covered by tests:

- Champions stat formula (`CALC_HP_CHAMP`, `CALC_STAT_CHAMP`)
- modern damage base formula and random rolls
- JS `pokeRound` and `chainMods`
- STAB, Adaptability-style STAB, and Champions Terastal STAB
- Champions Tera 60-BP floor and its priority/multi-hit exceptions
- modern type chart
- type items, gems, Life Orb, Expert Belt, resist berries, Choice items
- Champions raw item list in `data/champions/items.json`, with typed enum
  coverage for every listed name
- vendored Project Pokemon `champout` raw dumps for Pokemon/form data, moves,
  learnsets, and English text, with a source manifest and SHA-256 validation
- normalized `champions-data.json` generated from `champout`, exposed as
  `damage_calc::data::CHAMPIONS_DATA_JSON`
- public zero-fetch list constants in `damage_calc::data::champions` for
  Champions items, Regulation M-A Pokemon, Pokemon/form summaries, and
  abilities
- typed `Ability` variants for every ability name in the vendored Champions
  ability list, with direct/preprocessing mechanics implemented where they
  affect damage calculation state
- Light Ball, Shell Bell/Fling power, and Champions Mega-stone Fling/Knock Off
  restrictions
- common offensive and defensive ability modifiers
- type-changing abilities such as Pixilate/Aerilate/Galvanize/Refrigerate
- variable base-power moves such as Low Kick, Gyro Ball, Heavy Slam, Fling,
  Electro Ball, Stored Power, Hex, and related `basePowerFunc` branches
- named move BP/effectiveness/failure branches such as Solar Beam/Solar Blade,
  Nihil Light, Dream Eater, Sky Drop, Synchronoise, Damp, and Sturdy OHKO
- Electromorphosis/Wind Power Electric BP doubling and Wind Rider wind-move
  immunity
- Heatproof Fire reduction, Infiltrator screen bypass, and contact suppression
  from Long Reach, Protective Pads, and Punching Glove
- Natural Gift berry type/power table
- move type changes for Weather Ball, Terrain Pulse, Natural Gift, Judgment,
  Techno Blast, Multi-Attack, Revelation Dance, Aura Wheel, Raging Bull, Ivy
  Cudgel, Tera Blast, and Tera Starstorm
- `CALCULATE_ALL_MOVES_SV` preprocessing for Trace, Neutralizing Gas, Forecast,
  Mimicry, Air Lock/Cloud Nine, Klutz, terrain seeds, Intimidate, Download,
  Embody Aspect, Intrepid Sword, Dauntless Shield, Wind Rider, and
  Supersweet Syrup
- Protosynthesis/Quark Drive activation from Sun/Electric Terrain or Booster
  Energy, including highest-stat attack/defense/speed modifiers
- Champions entry toggles for Evo/Tatsugiri boosts, Clangorous Soul, and
  Weakness Policy
- fixed and HP-dependent damage moves such as Super Fang, Endeavor, Final
  Gambit, Seismic Toss, and OHKO moves
- recovery-aware KO odds for Sitrus, Oran, and modern pinch berries, including
  recovery between hits of the same multi-hit move
- counter-style damage moves (`Counter`, `Mirror Coat`, `Metal Burst`, and
  `Comeuppance`), with automatic opposing-move inference in the batch API and
  explicit prior-damage input in the single-move API
- multi-hit totals and per-hit rolls, including Triple Kick/Triple Axel hit
  power, Parental Bond second-hit reduction, and Stamina/Weak Armor between-hit
  recalculation
- multi-hit first-hit consumables/effects for resist berries, Kee Berry,
  Maranga Berry, Multiscale, Shadow Shield, Gooey, Tangling Hair, Cotton Down,
  Spicy Spray, Defiant/Competitive follow-up
  boosts, and burn-heal berries
- Ruin field modifiers
- priority-blocking abilities and Psychic Terrain priority prevention
- Champions type-effectiveness overrides for Thousand Arrows, Stellar vs Tera,
  Tera Shell, Ring Target, Foresight/Scrappy/Mind's Eye
- final speed modifiers for speed-based moves and Analytic ordering
- special stat-source moves such as Foul Play, Body Press, and Psyshock-style
  physical-defense special moves
- Champions ability branches for Plus, Minus, and Ripen-enhanced berries
- JS custom modifier hooks for BP/Attack/Defense/Final modifiers
- Protect quartering only for JS-qualified move/ability paths
- Sun/Rain damage modifiers
- Electric/Grassy/Psychic/Misty terrain power modifiers
- critical hit boost-ignore behavior
- burn, Reflect, Light Screen, Aurora Veil
- Stealth Rock, one-to-three Spikes layers, Salt Cure, initial toxic counter,
  healing suppression/removal, and reference-ordered end-of-turn KO projection
- exact typed inventories for 315 Pokemon/forms, 496 moves, 148 items, 201
  unique abilities, and 123 bundled sets
- one-move and two-direction four-move calculation APIs, with entry
  preprocessing applied once
- semantic result metadata for resolved move state, outcome kind, and signed
  Pain Split HP changes

Regression tests live in `tests/fixtures.rs`. Live differential tests and the
pinned Node oracle live in `tests/reference_oracle.rs` and `tools/reference/`.

## Scope Boundary

Parity covers generation-10 Regulation M-B normal dex and semantic library
results. It does not claim parity for:

- HTML/CSS/JQuery behavior, DOM description strings, local storage, audio, or
  images
- generations 1-9, Dynamax/Max Moves, Z-Moves, and G-Max residual fields
- Legends Z-A cooldowns, Plus Moves, link items, and damage reduction
- Champions NatDex
- full turn simulation beyond state consumed by damage and four-use KO
  projection
- optimizer search/ranking, a Rust-only future feature absent from the
  reference damage engine

Rust-only extensions, such as the explicit Leech Seed KO input, are outside
the compatibility comparison and do not alter scoped reference behavior.

Local JS reference checkouts may live under `reference/` for behavior audits,
but that folder is ignored and intentionally not published in this repository.
The published crate contains normalized `champout` data plus a generated exact
reference inventory. It does not publish the full upstream checkout.

## Thanks To

- Nimbasa City Post / `NCP-VGC-Damage-Calculator`, whose JavaScript calculator
  is the behavior reference for this Rust port.
- Project Pokemon / `champout`, for the Pokemon Champions static data dumps
  used to generate and validate the vendored Champions data.
- Pokemon Showdown and the `@pkmn` ecosystem, for normalized Pokemon data and
  mechanics references used to cross-check ambiguous cases.

## Repository

GitHub remote:
<https://github.com/D35P4C1T0/pkmn-dmg-lib-rs>

The crate is prepared with package metadata, an MIT license, pinned Champions
data, and a `.gitignore` that excludes build output.

## Development

```sh
cargo test
```

The crate has optional serde derives behind the `serde` feature:

```sh
cargo test --features serde
```

Before publishing or tagging a release, run:

```sh
cargo fmt --check
cargo test
cargo test --features serde
cargo package --list
```

Regulation M-C data is also available through `REGULATION_M_C_POKEMON`,
`regulation_m_c_pokemon`, and `ChampionSpeciesSummary::is_regulation_m_c`.
Use `champions_current_species` for current source-dump stats, types, weights,
and default abilities, and `CHAMPIONS_ITEM_VALUES` for current typed items.
Full ability choices and learnsets are in `CHAMPIONS_DATA_JSON`.
See [data sources and calculation scope](data/champions/README.md) for details.
