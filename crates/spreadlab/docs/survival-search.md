# Exact survival search

`api::find_min_survival(SurvivalRequest)` (WASM `findMinSurvival`) minimizes
total Champions Stat Points under explicit KO-probability constraints.
The CLI accepts the same JSON:

```sh
cargo run -- survive-exact examples/survival.json
```

The checked-in example returns 5 HP / 14 SpD, total 19, KO probability 0.1875.
Both reported 26 HP / 0 SpD and 19 HP / 4 SpD spreads cost more. This result
holds for the example's fixed Hardy nature, full HP, and pinned damage engine.

## Domain and guarantees

For each allowed nature, enumerate integer HP, Defense, and Special Defense
allocations in `[0,32]`, subject to exact locks and lower bounds. Preserve the
parsed Attack, Special Attack, and Speed investments unless explicitly overridden
by locks. The total of **all six stats** must not exceed `search.max_total`
(default 66; values above 66 are rejected). A defense the engine cannot read for
the request is held at its lock or lower bound instead of being enumerated; see
Defensive-stat relevance below.

Minimize the total of all six stats. This is final investment, not investment
added to a baseline. Parsed HP/Defense/SpD are replaceable: use `search.minimum`
to preserve lower bounds or `search.locked` to preserve exact values. For example:

```json
{
  "max_total": 66,
  "locked": {"attack": 32, "speed": 15, "hp": 5},
  "minimum": {"special_defense": 14},
  "result_mode": "AllMinima"
}
```

This reserves 47 points outside defense and leaves 19 points for HP/Def/SpD.
Conflicting locks/lower bounds and per-stat values above 32 are errors. A valid
domain with no allocation fitting its budget returns no matches.

The engine evaluates **every allocation in a total-cost layer before stopping**,
where that layer spans the retained domain described under Defensive-stat
relevance. No greedy allocation, binary search, or monotonicity assumption is
used, and no dimension is dropped from move names or from category alone: a
defense is pinned only when the pinned engine's own resolved metadata proves it
cannot enter the damage result. It therefore finds global minima within the
declared domain and feasibility model. This is not unrestricted six-stat
optimization: Attack/SpA/Speed stay fixed even for moves affected by those stats.
It also does not independently verify the pinned damage engine's battle mechanics.

## Defensive-stat relevance

The pinned engine reads the defender's Defense for
`category == Physical || deals_physical_damage` moves and Special Defense
otherwise. A defensive stat the engine cannot read is **pinned to its lock or lower
bound** instead of being searched, so an ordinary physical-only benchmark never
invests SpD and an ordinary special-only benchmark never invests Defense. A lock
or `minimum` on the pinned stat is honored exactly; only the values above it are
dropped.

The pin cannot change the answer: the pinned stat does not enter the damage result,
so `minimum_total`, every KO threshold, and the `AllMinima` layer are the same as
with the full HP/Defense/SpD domain. What changes is the reported allocation.
`TopK` rows no longer include spreads that differ only in a defensive stat the
engine cannot read for the request, and `closest_miss` and `Pareto` report the
pinned representative of an objective, because the alternatives were dominated by
it.

Relevance is decided per benchmark and unioned across benchmarks, so a mixed set
keeps both defenses.

Overrides are not fallbacks. When the resolved metadata names a different
defensive stat than the move's category does, that stat is the one searched:
Psyshock is Special in the pinned metadata but carries the engine's
`deals_physical_damage` flag (it is the only pinned move that does), so Defense
stays a dimension for it and SpD is the pinned one. The rule reads the same
resolved metadata the damage engine receives rather than the category alone.

The search falls back to the full dual-defense domain whenever the engine may read
both defensive stats:

- The move's damage does not come from the standard formula (Status category or
  zero base power), so nothing about it is proven.
- The engine compares both defensive stats. `check_download` picks the attacker's
  boost from the defender's Defense and SpD, `check_trace` can move an ability
  between the two sides, and an active paradox ability makes `highest_stat`
  compare every stat for the defense modifier and for move order. None of these
  abilities are in the pinned Champions ability list today; the checks exist so a
  future re-pin cannot silently under-search.

## Nature selection

Precedence, highest first:

1. `search.allowed_natures`, a nonempty list (duplicates are removed).
2. Explicit `nature`.
3. All 21 canonical natures when `optimize_nature` is true.
4. The defender set's parsed nature, Hardy if omitted.

The five neutral natures (Hardy, Docile, Serious, Bashful, Quirky) are
stat-identical, so every optimizer boundary canonicalizes them to Hardy. An
explicit non-Hardy neutral request, a parsed neutral set, and a neutral entry in
an allowed list all resolve to a single Hardy candidate; duplicate entries are
removed. All-nature search therefore explores the 21 distinct natures. Raw set
parsing and the stat calculator still accept and return the original neutral
names.

All-nature search preserves natures with identical defensive outcomes. It does
not decide which offensive stat the player can afford to reduce. Use the allowed
list to enforce that choice. Defensive-stat overrides such as Psyshock are handled
by relevance, not by nature selection: every allocation in the kept defensive
dimension is evaluated by the engine (see Defensive-stat relevance above).

## Feasibility models

`evaluation` defaults to `"Independent"`. Supply one `max_ko_chances` entry per
hit. Every hit is evaluated separately from the requested initial HP, and **all**
constraints must hold. This is the appropriate mode for separate matchup targets.

For an ordered attack sequence:

```json
{
  "evaluation": {"Sequence": {"end_turn_after": [false, true]}},
  "max_ko_chances": [0.25]
}
```

The two attacks occur before one end-of-turn phase. Supply exactly one threshold
for the whole sequence. The flag array must be empty (no end-of-turn phases) or
have one flag per attack; the last flag explicitly controls final residuals.

Independent mode uses upstream's one-move `ko_chance`, including its item
projection. The sequence mode is SpreadLab's **limited HP/item/residual model**:
it consumes Focus Sash, models Leftovers and the existing weather/terrain/status/
Leech Seed residuals, and carries HP, item, and toxic counter between attacks.
It supports damaging and immune/failed outcomes; status, counter, and HP-share
outcomes are rejected. Weather damage is resolved before later healing, so a
weather KO cannot be undone by Leftovers.

Sequence mode is not a full battle simulation. Other item consumption, healing
berries, switching, move-induced stat/status/ability changes, and field changes
are not persisted. Multi-hit moves use the upstream aggregate roll distribution;
Focus Sash is applied to that aggregate, not to individual internal hits. These
limitations are deliberate model boundaries, not search optimality guarantees.
In particular, upstream's single-hit Focus Sash projection differs from the
sequence model. A one-element sequence and an independent query need only agree
when their mechanics and turn boundaries agree.

Sequence evaluation merges probability mass for identical future-relevant states
after every attack. It retains minimum and maximum accumulated direct damage
separately. `sequence.min_damage`/`max_damage` exclude residual damage/healing and
stop accumulating on KO. `results` always contains independent initial-HP hit
summaries; use `sequence.ko_chance` for sequence feasibility.

`hp_percent` defaults to 100 and must be finite and in `(0,100]`. Initial HP is
`ceil(max_hp * hp_percent / 100)` using double precision for conversion. KO
thresholds must be finite probabilities in `[0,1]`, not percentages. Damage equal
to remaining HP is lethal. Missing KO probabilities are accepted as zero only
for zero-damage Status/ImmuneOrFailed outcomes; other missing probabilities fail.

## Result modes

- `TopK` (default): return the best `search.limit` feasible spreads (default 10).
  Higher-cost spreads can appear. The limit must be positive.
- `AllMinima`: return every feasible allocation/nature in the first feasible cost
  layer. Ignore `limit`; do not truncate equally optimal answers.
- `Pareto`: return all feasible spreads undominated in total cost and every KO
  probability. Equal objective vectors are retained. Ignore `limit`.

Deterministic ranking: total cost, lexicographic KO probabilities in benchmark
order, summed maximum direct damage (sequence maximum for sequences), six-stat
allocation in HP/Atk/Def/SpA/SpD/Spe order, then canonical nature order from
`optimize::all_natures`. These tie-breakers never override minimum cost.

`minimum_total` is the smallest feasible total, or null if none exists. If no
match exists, return `closest_miss` when there was at least one legal candidate.
It minimizes the largest positive KO-threshold violation, then uses the same
ranking. By default it is null when matches exist, allowing safe early exit.
`search.include_closest_miss: true` requests the global closest miss even on
success and forces evaluation of the full domain. It may still be null if no
candidate fails. Pareto mode also requires full-domain evaluation.

The default domain contains at most 35,937 allocations per nature before total
budget filtering, or 1,089 when relevance pins one of the two defenses. All-nature,
Pareto, and full closest-miss searches can be substantially more expensive than
first-layer minimization.

## Migration

The old `find_min_hp_def_survival` and combined variants retain their names and
response types, but now search HP/Defense/**SpD**. Their request types add
`search: Option<SurvivalSearchOptions>`; omitted JSON keeps the legacy `limit`
field, while a supplied `search` owns the limit and other search settings.
Rust struct literals must add `search: None` or supply options.

Combined requests additionally add `end_turn_after: Option<Vec<bool>>`. Omitted
JSON (`None` in Rust) preserves the old behavior: every attack ends a turn,
including the last. Supply `Some(vec![])` for same-turn attacks without final
residuals. The low-level `*_with_options` functions expose the same controls.

Nature behavior is intentionally corrected across survival and offensive KO APIs
and CLI commands: `optimize_nature: false` no longer silently searches all natures.
`true` searches the 21 canonical natures rather than assuming
Bold/Calm/Adamant/Modest suffice, with every neutral nature (Hardy, Docile,
Serious, Bashful, Quirky) folded onto Hardy.
The offensive one-stat search now uses Defense for Body Press, no attacker
investment dimension for Foul Play, and preserves other parsed investments.
Its guarantee remains restricted to its selected investment stat and nature list.

Defensive-stat relevance narrows the searched domain in the survival and score
ranking APIs: rows no longer report a defense that cannot affect the request. The
declared minimum, locks, lower bounds, KO thresholds, nature list, and total budget
all keep their meaning. Callers that relied on seeing SpD variants of a physical
minimum, or Defense variants of a special one, should add `minimum` or `locked` for
that stat instead.

`run_defensive_optimization`/`run_offensive_optimization` remain **score-ranking**
APIs for compatibility, not minimum-investment solvers. Use `find_min_survival`
with independent constraints instead of interpreting their scores as a minimum.
When such a request is not a full-spend one, the defense the engine cannot read is
held at its locked value (zero unless locked), and Body Press keeps the attacker's
Defense because that is its attack stat.

The legacy raw stat calculator/parser can still represent spreads over 66 for
damage inspection; legal optimization candidates always obey the total cap.
