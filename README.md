# SpreadLab

Unified Rust workspace for the SpreadLab damage engine, optimizer, CLI, and
Axum/Leptos web application. Supports:

```text
[Gen 9 Champions] VGC 2026 Reg M-C
```

The app is an Axum server with server-rendered Rust UI and a small client-side script for calculator interactions. It consumes the local `spreadlab-rs` public API and does not copy damage formulas, stat conversion logic, or optimizer internals.

## Monorepo layout

This repository is the single Cargo workspace for the whole SpreadLab stack:

| Path | Package | Role |
| --- | --- | --- |
| `crates/pkmn-dmg-lib-rs` | `pkmn-dmg-lib` (library `damage_calc`) | Pokémon damage calculation engine |
| `crates/spreadlab` | `spreadlab-rs` | Optimization library, CLI, and WebAssembly exports |
| `crates/spreadlab-web` | `spreadlab-web` | Axum + Leptos SSR web application |

Dependencies flow one way: `spreadlab-web` -> `spreadlab-rs` -> `pkmn-dmg-lib`;
the web app also uses `pkmn-dmg-lib` directly.
They are wired through workspace path dependencies, so a local edit in the
engine or the optimizer is compiled into the web app on the next build. No
intermediate repository, branch, or pinned-revision update is needed while
developing.

## Features

- Defensive survival optimizer and offensive KO optimizer views.
- OHKO, 2HKO, and 3HKO defensive survival goals using combined sequence odds.
- Editable Showdown-style sets with synchronized UI cards.
- Move selection, crit toggles, status conditions, natures, stat points, and boost stages.
- Field controls for format, terrain, weather, screens, Helping Hand, Protect, Gravity, Fairy Aura, and Friend Guard.
- Ability-aware field adaptation for effects like Fairy Aura, weather/terrain setters, Intimidate, Defiant, Competitive, Contrary, Guard Dog, Mirror Armor, and related Intimidate immunities.
- Browser persistence via `localStorage`.
- Static Pokémon sprites from Pokémon Showdown’s `gen5` collection, with PokéAPI and local fallbacks; item sprite proxy/cache.
- Champions Gen 10 preset selector, vendored from NCP VGC Damage Calculator.

The preset data in `crates/spreadlab-web/assets/setdex_ncp-g10.js` is distributed
under its upstream MIT license in `crates/spreadlab-web/assets/SETDEX_LICENSE`.

Pokémon type SVGs are from
[`partywhale/pokemon-type-icons`](https://github.com/partywhale/pokemon-type-icons)
by James Watkins and are used under the MIT license. The retained license is in
`crates/spreadlab-web/assets/type-icons/LICENSE`.

## Run

Install Rust through rustup; `rust-toolchain.toml` selects Rust/Cargo 1.94.1,
including Clippy, rustfmt, and the optimizer Wasm target. All packages declare
Rust 1.94.1 as the supported minimum. CI and Docker use the same pinned toolchain.

From the repository root:

```sh
cargo run -p spreadlab-web -- serve --host 127.0.0.1 --port 3000
```

The optimizer CLI and the full workspace are available from the same root:

```sh
cargo run -p spreadlab-rs -- --help
cargo test --workspace
cargo test -p pkmn-dmg-lib            # calculation engine in isolation
cargo build -p spreadlab-rs --lib --target wasm32-unknown-unknown
cargo build --release -p spreadlab-web
```

Then open:

```text
http://127.0.0.1:3000/survive
```

Debug builds inject live-reload support automatically. Changes to `app.css`,
`app.js`, or the vendored setdex reload connected browsers without restarting
the server. For Rust source changes, run through `cargo-watch` so the server is
rebuilt; the browser reconnects and reloads after restart:

```sh
cargo watch -x 'run -p spreadlab-web -- serve --host 127.0.0.1 --port 3000'
```

Heavy survival searches run roughly an order of magnitude faster under the
release profile than under the debug profile. A release build is an optional
performance mitigation for a responsive local workflow, not a substitute for the
request coalescing and admission limits that keep calculations from piling up:

```sh
cargo run --release -p spreadlab-web -- serve --host 127.0.0.1 --port 3000
```

Release builds do not include or run live-reload middleware.

## Docker

Build and run with Docker:

```sh
docker build -t spreadlab-web .
docker run -d \
  --name spreadlab-web \
  --restart unless-stopped \
  -p 3000:3000 \
  spreadlab-web
```

Or with Compose:

```sh
docker compose up -d --build
```

The container serves on `0.0.0.0:3000`. Put it behind Caddy, Traefik, Nginx, or another reverse proxy for HTTPS.

Sprite and item images are fetched on demand and cached under:

```text
crates/spreadlab-web/assets/sprites-static
crates/spreadlab-web/assets/item-sprites
```

`compose.yaml` mounts named volumes for those caches so they survive container rebuilds.

## API Routes

- `GET /api/meta`
- `POST /api/damage`
- `POST /api/survive`
- `POST /api/survive-sequence`
- `POST /api/ko`
- `POST /api/optimize/defensive`
- `POST /api/optimize/offensive`
- `GET /api/sprite/:name`
- `GET /api/item-sprite/:name`
- `GET /api/move-types`
- `GET /api/species-types`
- `GET /api/species-abilities`
- `GET /api/unsupported-items`

## Development

Useful checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p spreadlab-web
node --check crates/spreadlab-web/assets/app.js
```

Cached remote sprites are written under `crates/spreadlab-web/assets/sprites-static` and `crates/spreadlab-web/assets/item-sprites`; those directories are intentionally ignored by git.

## Regulation M-C data

The M-C damage engine (`crates/pkmn-dmg-lib-rs`) and SpreadLab adapter
(`crates/spreadlab`) now live in this workspace. They were imported from
[`pkmn-dmg-lib-rs` commit `96f55ef`](https://github.com/D35P4C1T0/pkmn-dmg-lib-rs/commit/96f55ef04e66d882457af4f009f228de0e73afdc)
and [`SpreadLab` commit `db5d932`](https://github.com/D35P4C1T0/SpreadLab/commit/db5d93254aac506d7aa8a70b088a7625b8154f77),
the revisions the web application consumed in production before the migration.
The engine also includes the test-only `b055a90736a6a6e7568fd4de87ebfd6b5efba661`
commit; no newer formulas or data were imported. See [migration verification](MIGRATION.md).
Species, moves, abilities, and items come from these dependencies, including all
23 newly usable species, their forms, six Megas, and the 12 new held items.
The engine refreshed its source data from [Project Pokémon champout](https://github.com/projectpokemon/champout)
and checked the [official M-C announcement](https://news.pokemon-home.com/en/page/816.html).

NCP presets were refreshed from [upstream commit
`1369b359b85f0a6343df006acde92cc4a7d07805`](https://github.com/nerd-of-now/NCP-VGC-Damage-Calculator/commit/1369b359b85f0a6343df006acde92cc4a7d07805)
on 2026-09-14, including the latest M-B and M-C updates (151 sets across 90 Pokémon entries).
Only the preset data is vendored. The pinned Rust engine also syncs with this
NCP revision, including damage, residual, and KO updates. The adapter's KO
projection follows upstream berry behavior and does not apply Focus Sash
survival; its separate sequence model retains its existing behavior.
New Pokémon can be configured manually even when no preset exists.

Direct damage effects use the engine's mechanics. Leek's critical-hit probability,
Rocky Helmet retaliation, switching items, trapping duration, and terrain duration
are not simulated by the single-attack calculator. Set critical hits and battle
state explicitly where applicable.

Ability behavior is owned by SpreadLab and the damage engine. The web app sends
ability state and manual field/stage inputs without applying entry effects.
`Ability Enabled:` controls the Active checkbox; `Ability On:` remains the
library's conditional activation input. Mechanics regression tests live in the
library repositories.

Local library development happens in this repository. The engine and the
optimizer are ordinary workspace members, referenced from the web app through
`[workspace.dependencies]` in the root `Cargo.toml`:

```toml
[dependencies]
damage_calc.workspace = true
spreadlab-rs.workspace = true
```

Editing `crates/pkmn-dmg-lib-rs` or `crates/spreadlab` and rebuilding the web
app is enough; there is no git pin to bump and no `[patch]` override to maintain.
The old `.cargo/config.toml` sibling-checkout patches are obsolete and can be
deleted. Concurrency and calculation behavior remain owned by the library
crates.

## UI development and browser checks

The responsive workspace uses the shared controls in `src/ui.rs`, the hand-written
`assets/app.css` stylesheet, and presentation helpers in `assets/app.js` (paths
relative to `crates/spreadlab-web`). Calculator requests and domain logic stay in
the existing adapter. The six SP values shown under a set are its current
investment, not the optimizer's answer; ranked rows stay advisory until you
choose Apply spread, which copies that row's SPs and nature onto the matching
side.

With the server running, install and run the browser checks using pnpm:

```sh
pnpm install --frozen-lockfile
pnpm exec playwright install chromium
pnpm test:catalog
pnpm test:ui
```

To use an existing Chromium installation, set `CHROMIUM_PATH=/usr/bin/chromium`.
`SPREADLAB_URL` overrides the default `http://127.0.0.1:3000` test server.
`SPREADLAB_SCREENSHOTS=/tmp/spreadlab-screenshots` saves captures at 1440×900,
1280×800, 1024×768, and 390×844. Tests cover real damage, both optimizer modes,
keyboard conditions, crit persistence, sets, forms, swapping, and failure states.

The desktop app shell keeps Optimization above the matchup and Field Conditions
above Results, with a compact left navigation rail. Saved Sets and Metagame open
searchable local set libraries; Metagame defaults to Regulation MC competitive
presets, including Rillaboom and Indeedee. The August MB usage snapshot remains
available as an explicitly historical view. See
[common-set refresh instructions](crates/spreadlab-web/README.md#common-sets).
Guides explains the calculator, and Settings selects a persisted dark
palette. Below 1200px the right workspace follows the matchup; below 900px the
order is Attacker, Defender, Battle Conditions, Optimization, Results.

Sidebar SVG icons are from [Lucide](https://github.com/lucide-icons/lucide).
Their ISC license is retained in `crates/spreadlab-web/assets/sidebar-icons/LICENSE`.
