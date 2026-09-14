# Champions Porting Plan: One Month to Measurable Parity

## September 2026 reference update

The current pin is `1369b359b85f0a6343df006acde92cc4a7d07805` (Regulation M-C).
See [the update audit](tools/reference/UPSTREAM_UPDATE.md) for implemented changes,
regression coverage, and preserved upstream behavior. The month-long plan and
July completion figures below are historical, not the current inventory.

## Objective

Reach **100% behavior parity for the VGC 2026 Pokemon Champions, Regulation
M-B, normal-dex calculator path** in one full-time developer-month (20 working
days). Parity means the Rust library and the pinned JavaScript reference produce
the same semantic result for every supported input, not that both codebases have
similar-looking branches.

This plan replaces the previous percentage estimate. Claims such as "about 90%"
are not measurable while only four results are stored from JavaScript and most
tests use Rust-authored expected values.

## Completion Status

Completed 2026-07-10 for pinned upstream commit
`dfbf020d4ed7df8921c6e11bbaa23410f6ca1448`.

- scoped reference functions classified: 97/97; unknown: 0
- exact inventory: 315 Pokemon/forms, 496 moves, 148 items, 201 unique
  abilities, and 123 sets
- generated differential partitions: more than 18,000
- unexplained differential mismatches: 0
- ignored or quarantined parity tests: 0
- one-move, four-move/two-direction, automatic counter, typed data/set, and
  serde APIs complete
- hazards, healing, status, multi-hit, entry preprocessing, and four-use KO
  probabilities verified against live JavaScript
- source hashes, inventory drift, scope drift, generated Rust drift, formatting,
  clippy, default/serde tests, and package contents checked in CI

Implementation found and fixed several errors hidden by Rust-authored fixtures:
Parental Bond used a half final modifier instead of the generation-10 quarter
general modifier; Tera's 60-BP floor and Expanding Force conditional spread
were missing; Struggle typing, Grassy Glide priority, Electro Shot's mid-move
boost, and early multi-hit KO probability mass were incomplete. Rust-only
Disguise, Sand Spit, Speed Boost, Opportunist, Flower Veil/Intimidate, and Focus
Sash projections were removed from compatibility behavior where pinned
JavaScript does not model them.

## Pre-implementation Audit Baseline

Audit performed 2026-07-10 against:

- Rust: `1a5807aa2f608c63d79aaf387b672270cf2c9ad3`
- Reference: `dfbf020d4ed7df8921c6e11bbaa23410f6ca1448`
  (`nerd-of-now/NCP-VGC-Damage-Calculator`, 2026-07-02)
- Reference entry page: `index.html`, generation `10`
- Main calculation entrypoint: `script_res/damage_SV.js`
- Shared mechanics: `script_res/damage_MASTER.js`
- KO calculation: `script_res/ko_chance.js`
- Stats and types: `script_res/stat_data.js`, `script_res/type_data.js`
- Champions data: `pokedex.js`, `move_data.js`, `item_data.js`,
  `ability_data.js`, and `setdex_ncp-g10.js`

Current verification:

- `cargo fmt --check`: passes
- `cargo test`: 64 tests pass
- `cargo test --features serde`: 64 tests pass
- Stored JavaScript golden cases: 4
- Champions reference inventory: 315 Pokemon/forms, 496 moves, 148 items,
  202 ability entries (201 unique), and 123 sets for 77 Pokemon
- Rust generated inventory: 361 species/forms, 920 general move records,
  97 items, 202 abilities, and no typed set API

The different data counts do not directly indicate missing data: Rust imports a
larger `champout` dataset. They do show that the public Rust data surface is not
an exact, auditable representation of the reference's active Champions subset.

## Definition of 100%

All following gates must pass at the same pinned upstream commit:

1. Every Regulation M-B Pokemon/form, move, item, ability, and bundled set used
   by the normal-dex Champions page can be loaded into typed Rust inputs without
   callers manually recreating reference metadata.
2. Every generation-10 branch reachable from `CALCULATE_ALL_MOVES_SV`,
   `GET_DAMAGE_SV`, applicable shared damage helpers, and applicable KO helpers
   is either implemented or listed in the scope manifest as unreachable with a
   tested reason.
3. Differential corpus has zero unexplained mismatches for damage rolls,
   per-hit rolls, immunity/failure, effective move type/category/base power,
   critical state, and KO probabilities through four move uses.
4. Every reachable named branch has at least one direct fixture; every modifier
   stage has interaction fixtures covering rounding and order.
5. Normal-dex inventory equality tests pass against the pinned reference for
   names, aliases, metadata, and bundled sets.
6. Public APIs can calculate one move, all four moves for one side, or all four
   moves for both sides after applying entry preprocessing exactly once.
7. `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
   `cargo test`, `cargo test --features serde`, and the differential suite pass
   in CI.
8. No `UnsupportedMechanic` is reachable inside declared scope. No ignored,
   quarantined, or undocumented differential failures remain.

An intentional correction to a JavaScript bug is not parity. If one is desired,
first preserve reference-compatible behavior behind a compatibility policy,
then document and test the corrected alternative.

## Month Scope

### Included

- Pokemon Champions generation-10 stat-point formulas
- Regulation M-B normal-dex Pokemon/forms and bundled sets
- All normal-dex Champions moves, items, and abilities that affect calculation
- Both attacker/defender directions and four move slots
- Entry preprocessing and its ordering
- Damage, fixed-damage, counter, multi-hit, and between-hit behavior
- KO odds, switch-in hazards, healing items, and end-of-turn state used by the
  active Champions page
- Stable typed Rust results equivalent to the JavaScript result semantics
- Optional serde representation of all public input/output types

### Excluded from the one-month parity claim

- HTML, CSS, JQuery event handling, DOM descriptions, local storage, audio, and
  images
- Generation 1-9 calculators and their old mechanics
- Legends Z-A (`gen == 9.5`), cooldowns, Plus Moves, red/blue link items, and its
  damage reduction
- Dynamax, Max Moves, G-Max residual fields, Z-Moves, and signature Z-Moves
- Champions NatDex mode beyond what Regulation M-B normal dex can reach
- April Fools calculators and other non-Pokemon assets
- Optimizer search/ranking, which is a Rust project goal but not a feature of
  the reference damage engine
- Battle simulation beyond state explicitly consumed by one calculation or KO
  projection

These exclusions must be encoded in a checked scope manifest. They must not be
silently skipped by the oracle generator.

## Gap Matrix

| Area | Current state | Required parity work | Done signal |
| --- | --- | --- | --- |
| Oracle and fixtures | Four stored JS cases; 64 passing Rust tests mostly use Rust expectations | Add executable pinned JS oracle, deterministic JSON protocol, generated fixtures, and mismatch classifier | Zero unexplained corpus mismatches |
| Scope tracking | README/notes use estimates and prose gap lists | Create branch/data manifest with `implemented`, `unreachable`, or `excluded` status and evidence | Manifest has no unknown entries |
| Public data API | Raw generated JSON plus lists; only four move constructors; no Pokemon or set constructors | Generate typed Champions Pokemon, moves, items, abilities, aliases, and 123 bundled sets from the exact active reference subset | Inventory and round-trip tests pass |
| Input model | Good direct-damage fields; missing several KO and prior-turn states | Add per-side hazards, toxic counter, item-presence/prior-turn flags, and other inputs found by branch audit; separate battle state from immutable species data | Every reachable oracle input serializes losslessly |
| Batch calculation | One attacker, defender, and move per call | Add one-side and two-side four-move APIs with preprocessing applied once in reference order | Batch results equal eight independent oracle results |
| Preprocessing | Trace, gas, weather/type changes, seeds, entry boosts, Intimidate, Download, and related paths broadly exist | Audit exact ordering and exception lists; finish copy/suppression/immunity cases; test paired interactions | Every `CALCULATE_ALL_MOVES_SV` step covered |
| Move preprocessing | Many type changes, spread defaults, priority, contact, and hit defaults exist | Cover all 496 active move records and conditional flags; remove name-based defaults where generated metadata exists | Metadata parity for every active move |
| Core mechanics | Broad base power, attack, defense, final modifier, type, weather, terrain, and Terastal support | Branch-by-branch audit for reachable generation-10 paths; add missing Pain Split and prior-turn/conditional move state; verify modifier ordering | Every reachable shared helper branch has fixture |
| Multi-hit | Per-hit rolls, Parental Bond, berries, Stamina, Weak Armor, Gooey/Tangling Hair/Cotton Down, and Spicy Spray exist | Differentially verify independent roll distributions, item consumption, stat changes, and weather/terrain mutations for all multi-hit metadata | Exhaustive multi-hit corpus passes |
| KO engine | Four-use probabilities, common healing berries, Leftovers, weather/status/terrain, and a Rust-only Leech Seed extension exist | Add Stealth Rock, Spikes, hazard immunity, Salt Cure, exact healing suppression/removal, initial toxic counter, residual ordering, and missing active item/ability interactions | KO probability distribution matches oracle |
| Result model | Structured rolls/modifiers/debug output; no exact reference description representation | Define semantic result schema for effective move state and KO context; keep browser sentence formatting out of scope | Oracle comparison needs no string parsing |
| Error policy | `UnsupportedMechanic` exists but is not currently returned | Replace silent fallbacks with typed invalid-input/out-of-scope errors; ensure valid scoped inputs never error | Negative tests plus zero scoped errors |
| CI and maintenance | Rust tests only; no upstream drift detection | Pin upstream SHA, record source hashes, run oracle corpus, inventory diff, and standard Rust checks | Clean CI from fresh checkout |

## Architecture Changes Required Before More Branches

### 1. Pinned reference oracle

Add a small Node runner under `tools/reference/` that loads only required
reference scripts with a minimal DOM/JQuery shim. It accepts canonical JSON and
returns canonical JSON. Vendor a source manifest containing repository URL,
commit, and hashes; do not vendor the whole reference repository in the crate.

Oracle output must include:

- final damage rolls and per-hit roll arrays
- effective move name, type, category, base power, priority, spread/contact flags
- immunity or failure reason
- applied entry state and effective stats
- KO probabilities for one through four uses
- consumed item and residual/hazard context needed to explain KO results

### 2. Separate data from battle state

Generated species/move/item/ability definitions should contain immutable
reference metadata. `Pokemon`, `Move`, `Field`, and per-side state should contain
only selections and mutable battle conditions. This prevents callers from
having to know flags hidden in `move_data.js` and prevents repeated calculations
from mutating shared definitions.

### 3. Explicit calculation pipeline

Keep stages independently testable and in reference order:

1. Load and validate definitions.
2. Apply entry preprocessing once to both Pokemon.
3. Resolve move type, priority, spread, contact, hit count, and special mode.
4. Resolve immunity, failure, fixed damage, or counter damage.
5. Resolve base power, attack, defense, base damage, general modifiers, and
   final modifiers with exact integer rounding.
6. Recalculate state between hits.
7. Project hazards, item triggers, and end-of-turn effects for KO odds.
8. Return semantic result without presentation-layer text.

## Four-Week Schedule

Assumption: one developer, 20 focused working days, no unrelated feature work.
Each day ends with tests and a reviewable commit. A failed exit criterion moves
remaining feature work out; it does not lower the parity gate.

### Week 1: Make parity measurable

**Day 1 - Freeze scope and inventory**

- Add upstream source manifest and scope manifest.
- Extract active normal-dex Champions inventories and all reachable function
  branches from pinned JavaScript.
- Record exclusions with reachability evidence.
- Replace percentage tracking with manifest counts.

**Day 2 - Build reference runner**

- Implement canonical JSON input/output runner around generation 10.
- Stub only browser dependencies; fail if calculation reads an unstubbed DOM
  value.
- Add smoke cases for neutral, immunity, fixed damage, multi-hit, and KO state.

**Day 3 - Build differential harness**

- Run identical cases through JS and Rust.
- Compare exact integer rolls and normalized semantic state.
- Emit smallest useful reproduction for each mismatch.
- Classify mismatches by pipeline stage, never by ad hoc allowlist.

**Day 4 - Finish input/result schema**

- Add missing per-side hazards and residual state, initial toxic counter,
  conditional/prior-turn move state, and item-presence state.
- Add effective-move and state details to `DamageResult`.
- Add serde derives and round-trip tests for public inputs/results.

**Day 5 - Establish baseline corpus and CI**

- Generate boundary and pairwise cases from reference inventories.
- Produce first mismatch report grouped by branch.
- Add oracle smoke suite, source-hash check, inventory check, and Rust quality
  commands to CI.

Week 1 exit: oracle runs from a fresh checkout; every discovered branch has a
manifest status; mismatch report is deterministic.

### Week 2: Close damage-pipeline gaps

**Day 6 - Entry preprocessing parity**

- Audit Trace and Neutralizing Gas exception lists, weather suppression,
  Forecast, Mimicry, Klutz, paradox activation, seeds, and entry ability order.
- Add paired-ability tests where order changes final state.

**Day 7 - Move resolution parity**

- Generate metadata for all active moves.
- Complete conditional type, category, priority, spread, contact, hit count,
  move failure, and ability-ignore handling.
- Implement Pain Split and any other active special-damage branch missing from
  Rust.

**Day 8 - Base power and offensive stages**

- Audit every reachable `basePowerFunc`, `calcBPMods`, `calcAttack`, and
  `calcAtMods` branch.
- Add explicit prior-turn/effect-count inputs instead of inferring history.
- Add rounding-boundary fixtures for every chained modifier value.

**Day 9 - Defense and final stages**

- Audit `calcDefense`, `calcDefMods`, general damage, STAB/Terastal, screens,
  berries, and `calcFinalMods`.
- Cover modifier stacking, critical boost-ignore rules, suppression, and
  species-locked items.

**Day 10 - Multi-hit and state mutation**

- Verify every active multi-hit move and Skill Link/Parental Bond interaction.
- Match first-hit item consumption and between-hit stat/weather/terrain changes.
- Remove all damage-stage mismatches from targeted branch corpus.

Week 2 exit: zero unexplained mismatches for single-use damage and per-hit rolls
across targeted branch corpus.

### Week 3: KO engine and complete data API

**Day 11 - Hazards and initial state**

- Add Stealth Rock and one-to-three Spikes layers per side.
- Match grounding, type effectiveness, Magic Guard, and active Champions
  hazard-immunity rules.
- Start KO projection from exact current HP and toxic counter.

**Day 12 - Healing and residual ordering**

- Match healing-item thresholds, Ripen/Gluttony, Psychic Noise, Knock Off,
  Thief/Covet, Bug Bite/Pluck/Incinerate, and item consumption.
- Match active weather, Grassy Terrain, Leftovers, status, Salt Cure, and Bad
  Dreams ordering. Keep extensions absent from the reference outside the parity
  comparison.

**Day 13 - KO probability parity**

- Compare full state probability maps, not only formatted percentages.
- Cover independent multi-hit rolls, healing between hits, the reference's
  non-modeling of Focus Sash, toxic escalation, and one through four uses.
- Eliminate sequence-limit fallback for all scoped move/hit combinations or
  replace it with exact dynamic programming.

**Day 14 - Exact Champions data subset**

- Generate typed active inventories from pinned reference while retaining
  `champout` as optional richer source data.
- Reconcile 315 reference Pokemon/forms, 496 moves, 148 items, and 201 unique
  abilities by name, aliases, flags, and legality.
- Treat duplicate reference entries explicitly rather than silently deduping.

**Day 15 - Constructors, sets, and batch API**

- Add name/ID lookup and constructors for battle-ready Pokemon and moves.
- Import 123 bundled sets for 77 Pokemon.
- Add one-side and two-side four-move calculation APIs.
- Prove preprocessing happens once and does not leak mutation between moves.

Week 3 exit: normal-dex inventory equality passes; every reference set can be
loaded and calculated; targeted KO corpus has zero unexplained mismatches.

### Week 4: Exhaustion, stabilization, and release

**Day 16 - Generated combinatorial corpus**

- Generate equivalence partitions and pairwise interactions across abilities,
  items, move flags, weather, terrain, side conditions, status, boosts, HP
  boundaries, formats, and Terastal state.
- Add focused three-way cases for modifier order and suppression interactions.

**Day 17 - Boundary and metamorphic testing**

- Cover HP 1/max, stat stages -6/+6, stat points 0/32, level bounds, weights,
  speed ties, 0/1 BP boundaries, type immunity, and modifier overflow.
- Add invariants such as sorted rolls, min/max consistency, probability bounds,
  no mutation of caller input, and batch/single equivalence.

**Day 18 - Resolve final mismatches**

- Minimize and fix every remaining differential failure.
- Record confirmed upstream ambiguities with reference-compatible fixtures.
- Require scope manifest to have no unknown or partially implemented entries.

**Day 19 - Public API and documentation review**

- Review naming, ownership, errors, serde schema, compatibility, and examples.
- Update README status from manifest results, not estimates.
- Document source update workflow and how to classify future mismatches.

**Day 20 - Clean-room release gate**

- Regenerate all data and fixtures from pinned sources in a clean checkout.
- Run full CI and package checks.
- Tag parity baseline only if every Definition of 100% gate passes.
- Publish final manifest totals and zero-mismatch corpus summary.

Week 4 exit: all parity gates pass from a fresh checkout. Otherwise release is
correctly labeled partial; remaining failures stay visible.

## Test Strategy

### Fixture layers

1. **Arithmetic units:** rounding, modifier chaining, stats, type chart.
2. **Branch fixtures:** one minimal case for every reachable JavaScript branch.
3. **Interaction fixtures:** pairwise and selected three-way modifier ordering.
4. **Inventory fixtures:** exact active names, metadata, aliases, and sets.
5. **Differential corpus:** generated canonical inputs executed by both engines.
6. **Regression fixtures:** every fixed mismatch remains as a small named case.

Generated fixtures must store upstream commit and runner schema version. Fixture
updates require a human-readable diff of changed inputs and outputs.

### Mismatch policy

Every mismatch receives exactly one classification:

- Rust port bug: fix Rust and add regression fixture.
- Oracle adapter bug: fix adapter and prove raw JavaScript behavior.
- Upstream ambiguity/bug: preserve compatible behavior and document it.
- Out of declared scope: add tested reachability proof to scope manifest.

"Close enough," float tolerances for integer mechanics, and unexplained
allowlists are forbidden.

## Risks and Controls

- **Reference uses mutable globals and DOM state.** Minimal runner fails on
  unmodeled reads; canonical input resets globals between cases.
- **A one-month target encourages false completion.** Completion is gated by
  manifests and zero mismatches, never a subjective percentage.
- **Combinatorial input space is huge.** Branch coverage plus equivalence
  partitions, pairwise generation, boundaries, and selected three-way cases
  provide tractable evidence; inventory-wide smoke cases catch metadata gaps.
- **Reference changes during month.** Work stays pinned. Upstream update happens
  only after parity baseline, as a separate reviewed manifest/fixture change.
- **Data sources disagree.** Active reference subset controls parity; `champout`
  remains available as richer data but cannot silently overwrite reference
  mechanics metadata.
- **Public structs may require breaking changes.** Crate is `0.1.0`; make model
  corrections during Week 1, provide constructors/builders, and avoid repeated
  field churn after schema freeze.

## Daily Tracking

Track these numbers in the scope manifest and CI summary:

- reachable branches: implemented / excluded / unknown
- active Pokemon/forms, moves, items, abilities, and sets matched / total
- differential cases passed / total
- mismatches by pipeline stage
- branch fixtures present / required
- ignored or quarantined tests

Target on Day 20: unknown `0`, unexplained mismatches `0`, ignored parity tests
`0`, and every scoped inventory/branch total complete.

## After the Month

Once Champions Regulation M-B normal-dex parity is tagged, create separate
milestones for Champions NatDex, Legends Z-A, legacy generations, UI-equivalent
description formatting, and the Rust-only spread optimizer. None should dilute
or retroactively redefine the Champions parity baseline.
