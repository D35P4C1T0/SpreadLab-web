# Monorepo migration verification

Verified locally on 2026-09-25, on `migration/monorepo`. Production deployment and original repositories remain unchanged.
The migration branch is published to the existing SpreadLab-web remote.
The checkout already contained a partial migration and unrelated UI changes;
those were preserved. Sources were imported at the recorded revisions. A history-only merge retains
both upstream commit ancestries while keeping the reviewed monorepo tree.
Original paths are visible in upstream commits; current files live under crates/.

## Source selection

| Component | Baseline | Imported revision |
| --- | --- | --- |
| Web | `bc3adbd` (destination HEAD) | existing web sources and pre-existing working changes |
| Damage engine | `96f55ef04e66d882457af4f009f228de0e73afdc` | same calculation source/data, plus test-only `b055a90736a6a6e7568fd4de87ebfd6b5efba661` |
| Optimizer | `db5d93254aac506d7aa8a70b088a7625b8154f77` | same |

The task mentioned optimizer `13582e8c87ebb15b95ce805413396395961c87df`.
Inspection of the committed web manifest and lockfile showed it had already
advanced to `db5d932`. Between these revisions upstream introduced exact survival
search (`f400166`), nature deduplication/lookup caching (`bcb7dbc`), and irrelevant
defensive-stat pruning (`db5d932`). These are existing production-pinned behavior,
not changes introduced by this migration. Reverting to `13582e8` would discard
that behavior and APIs used by the web app.

Remote main heads were checked directly: optimizer `db5d932`, engine `b055a907`;
neither remote advertised tags. The newer engine commit adds only the Mega Sol
two-direction regression test. All imported calculation source and data match
`96f55ef` byte for byte. Optimizer Rust sources match `db5d932` except for the equivalent
`Option::is_none_or` spelling required by Clippy after raising the MSRV.
`tools/migration/provenance.json` records every upstream tracked file's SHA-256,
revision, and destination. Deliberate differences are component manifests/READMEs (workspace dependencies,
Rust requirements, and monorepo metadata), the Clippy spelling above, Markdown
hard-break syntax in the optimizer handout, and removal of component lockfiles. Licenses, agent instructions,
fixtures, raw/normalized data, tools, and component workflow files are retained.
Nested workflow files are historical references; root workflows execute in CI.

## Baseline and regression results

Fresh isolated source archives of the exact production pins were extracted under
`/tmp/spreadlab-monorepo-verify`. Baseline tests and the original Git-dependency
web build ran there without the destination's old sibling-checkout Cargo patches.
The pinned JavaScript oracle was commit
`1369b359b85f0a6343df006acde92cc4a7d07805`.

| Check | Result |
| --- | --- |
| Exact baseline engine, default features | 75 fixture tests, 14 live oracle tests passed |
| Exact baseline optimizer | 49 unit + 9 relevance + 16 exact-survival tests passed |
| Baseline native web build/start | passed; localhost port 3302 |
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | passed |
| `cargo test --locked --workspace` | passed; includes 24 web tests |
| Engine with serde and live JavaScript oracle | 78 fixtures + 14 differential tests passed |
| Reference inventory/scope | passed: 346 species, 511 moves, 166 items, 216 abilities, 151 sets; 97 functions classified |
| Generated Rust data | generator output byte-identical |
| `cargo build --locked -p spreadlab-rs --lib --target wasm32-unknown-unknown` | passed; actual Wasm build |
| `cargo build --locked -p spreadlab-web --release` | passed |
| Native server start | passed; localhost port 3301 |
| Catalog JavaScript tests | 4 passed |
| Browser interaction tests | 39 passed with installed Chrome |
| Direct Docker build and Compose build | passed on Linux ARM64 Docker |
| Docker runtime | metadata, SSR page, and byte-identical app.js served on localhost port 3303; both isolated cache volumes retained sentinel files across restart |
| Engine package file listing | passed |

The Wasm module and target-specific wasm-bindgen dependency are unchanged. There
were no browser-executed Wasm tests in the imported optimizer. Native Axum is not
compiled for Wasm. Local Rust/Cargo, CI, and Docker are pinned to 1.94.1. Every package inherits
`rust-version = "1.94.1"`; edition 2021 remains unchanged. This replaces the
unverified 1.75 declaration (the existing Leptos dependency requires at least
1.88). `rust-toolchain.toml` installs the exact toolchain and Wasm target. No
unrelated dependency upgrades were made.

`tools/migration/compare.py` compared exact JSON responses from both running
servers for metadata, Pokémon/items/move types/species types/species abilities,
unsupported items, damage, survival, KO, defensive optimization, and offensive
optimization. All matched. Six page routes and both main assets returned 200.
CLI help, parse, stats, regulation listing, damage, survival, KO, and both ranked
optimization commands matched byte for byte. For the fixture damage matchup,
rolls remain 63–75 (37.1–44.1%, 0% KO).

Repeat against two running servers and their independently built CLI executables:

```sh
python3 tools/migration/compare.py http://127.0.0.1:3302 http://127.0.0.1:3301 \
  --baseline-cli /path/to/baseline/target/debug/spreadlab-rs \
  --migrated-cli target/debug/spreadlab-rs
```

Local execution logs are in `/tmp/sl-mig/logs/current-*` and
`/tmp/sl-mig/logs/exact-baseline-*`; these temporary logs are not repository inputs.

## Workspace, deployment, and resource paths

All three packages resolve with `source: null` and manifests inside this checkout
in `cargo metadata --locked --offline --no-deps`. Internal dependencies inherit
versioned workspace paths. There is one root lockfile. Existing package/library
names, APIs, licenses, metadata, and dependency features are preserved.

Compile-time data paths remain relative to engine source files. Fixtures and
reference helpers use CARGO_MANIFEST_DIR; Python generators anchor to their own
crate directory. No regulation data was regenerated or updated. Web assets and
runtime cache locations remain under `crates/spreadlab-web/assets`. Run the server
from the repository root, matching Docker's `/app` working directory.

Docker still builds a native release executable in rust:1.94.1-bookworm, then copies it
and web assets into debian:bookworm-slim. It binds 0.0.0.0:3000. The build now uses
`--locked`; the complete workspace is in the build context. Compose is unchanged:
`sprite-cache` mounts at `/app/crates/spreadlab-web/assets/sprites-static` and
`item-cache` at `/app/crates/spreadlab-web/assets/item-sprites`. Existing caches
are not migrated or deleted. Keep the existing Compose project name on production
so Docker selects the same named volumes. Do not run `down --volumes`.

Root CI validates formatting, strict linting, native tests, Wasm, release server,
live reference parity, generated data, packaging contents, browser interactions,
and the Compose image build. Every push/PR validates downstream consumers; no
deployment job is introduced. Hosted GitHub Actions execution remains to be
observed after a push; local equivalents passed.

## Local commands and review notes

```sh
cargo run -p spreadlab-web -- serve --host 127.0.0.1 --port 3000
cargo watch -x 'run -p spreadlab-web -- serve --host 127.0.0.1 --port 3000'
cargo run -p spreadlab-rs -- --help
cargo test --locked --workspace
cargo test --locked -p pkmn-dmg-lib
rustup target add wasm32-unknown-unknown
cargo build --locked -p spreadlab-rs --lib --target wasm32-unknown-unknown
cargo build --locked --release -p spreadlab-web
docker compose build
docker compose up -d
```

cargo-watch watches the repository, including the two new dependency directories;
existing debug-only live reload remains intact. No new frontend tooling was added.
The ignored local `.cargo/config.toml` had stale sibling patches; these were
removed, with a backup in `/tmp/sl-mig/obsolete-cargo-config.toml`.

The pre-existing move-selector performance change is committed separately;
formatting and Clippy cleanup are included with the workspace migration. Both
upstream histories are retained as merge parents, without changing the imported
source revisions. Original repositories remain intact. Independent publication
of the optimizer still requires the matching engine version in the target
registry; local development never requires publication.
