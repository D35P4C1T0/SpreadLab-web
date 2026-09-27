# SpreadLab Web Crate

Axum server crate for the repository-level SpreadLab Web application.

See [../../README.md](../../README.md) for setup, routes, and development commands.

## Common sets

The default Metagame view uses current Regulation MC competitive presets from
the September 12, 2026 NCP update (151 sets across 90 entries), including Rillaboom,
Indeedee, and new Mega forms. This is still the latest upstream preset revision
checked on September 26. Missing preset abilities use the reference species
default, so Rillaboom imports with Grassy Surge. Current presets also take
precedence when selecting a Pokémon without explicitly choosing a set.

Smogon's September report is not yet published as of September 26. Do not label
older usage percentages as MC rankings. `assets/common-sets.js` remains an
explicitly historical offline snapshot of the top 125 Pokémon in Smogon's
August 2026 Champions Regulation MB best-of-three usage report (1760 cutoff).
The historical view displays this list in usage order; All presets includes both
the current competitive library and historical samples. Presets are available in both Pokémon
selectors. The snapshot date, regulation, and usage percentages are displayed.

Sample builds combine the most-used ability, item, spread, and four moves from
the monthly moveset report. These are aggregate choices, not observed full teams
or a guarantee that all choices occurred together. Champions spreads are already
SPs and must not be divided by four. Source URLs are stored in the asset.

Refresh from a published monthly report:

```sh
python3 tools/import-common-sets.py --month 2026-08 --regulation MB
cargo test -p spreadlab-rs --test common_sets
```

The importer validates all 125 entries before replacing the snapshot. It also
accepts `--usage-file` and `--movesets-file` for saved reports. Bump the asset query
version in `src/ui.rs` when publishing a new snapshot. No runtime Smogon request
or third-party availability is required by the calculator.

## Optimizer evaluation reuse

Searches prepare engine inputs once rather than resolving Pokémon, abilities,
items, and moves for every candidate. Defensive searches and their sequence states
use bounded, request-local caches. A cache key includes nature, invested HP,
current HP, item state, offensive stats, speed, and every relevant defense.
An audited irrelevant defense can share evaluations with neighboring spreads;
unknown or coupled mechanics retain both defenses. HP-dependent moves and berry
timing retain their original engine evaluation.

Candidate ordering, allowed natures, TopK, all minima, Pareto, and closest-miss
semantics stay exhaustive. No monotonicity assumption or threat-dropping heuristic
is needed for this optimization.

Run the release-mode evaluation benchmark with:

```sh
cargo test -p spreadlab-rs --release --lib benchmark_mixed_grid -- --ignored --nocapture
```

It compares uncached and cached damage evaluation over a legal mixed-attack grid,
checks the damage checksum, and asserts the reduced engine-call count. This is an
evaluation benchmark, not a browser latency measurement.
