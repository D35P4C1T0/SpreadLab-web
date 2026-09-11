# SpreadLab Web

Dedicated Rust WebUI for [SpreadLab](https://github.com/D35P4C1T0/SpreadLab), a Pokémon Champions damage and spread optimization tool for:

```text
[Gen 9 Champions] VGC 2026 Reg M-C
```

The app is an Axum server with server-rendered Rust UI and a small client-side script for calculator interactions. It consumes the upstream `spreadlab-rs` public API and does not copy damage formulas, stat conversion logic, or optimizer internals.

## Features

- Defensive survival optimizer and offensive KO optimizer views.
- OHKO, 2HKO, and 3HKO defensive survival goals using combined sequence odds.
- Editable Showdown-style sets with synchronized UI cards.
- Move selection, crit toggles, status conditions, natures, stat points, and boost stages.
- Field controls for format, terrain, weather, screens, Helping Hand, Protect, Gravity, Fairy Aura, and Friend Guard.
- Ability-aware field adaptation for effects like Fairy Aura, weather/terrain setters, Intimidate, Defiant, Competitive, Contrary, Guard Dog, Mirror Armor, and related Intimidate immunities.
- Browser persistence via `localStorage`.
- Pokémon and item sprite proxy/cache with local fallbacks.
- Champions Gen 10 preset selector, vendored from NCP VGC Damage Calculator.

The preset data in `crates/spreadlab-web/assets/setdex_ncp-g10.js` is distributed
under its upstream MIT license in `crates/spreadlab-web/assets/SETDEX_LICENSE`.

Pokémon type SVGs are from
[`partywhale/pokemon-type-icons`](https://github.com/partywhale/pokemon-type-icons)
by James Watkins and are used under the MIT license. The retained license is in
`crates/spreadlab-web/assets/type-icons/LICENSE`.

## Run

From the repository root:

```sh
cargo run -p spreadlab-web -- serve --host 127.0.0.1 --port 3000
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
cargo fmt --all --check
cargo check -p spreadlab-web
node --check crates/spreadlab-web/assets/app.js
```

Cached remote sprites are written under `crates/spreadlab-web/assets/sprites-static` and `crates/spreadlab-web/assets/item-sprites`; those directories are intentionally ignored by git.

## Regulation M-C data

The web app pins the M-C [damage engine](https://github.com/D35P4C1T0/pkmn-dmg-lib-rs/commit/b8df8c49122f0f6016ecd049efca1824d59effe4)
and [SpreadLab adapter](https://github.com/D35P4C1T0/SpreadLab/commit/134666268d180ccb032601d36a02d438b112d655).
The adapter commit is on upstream's `agent/update-damage-library` branch.
Species, moves, abilities, and items come from these dependencies, including all
23 newly usable species, their forms, six Megas, and the 12 new held items.
The engine refreshed its source data from [Project Pokémon champout](https://github.com/projectpokemon/champout)
and checked the [official M-C announcement](https://news.pokemon-home.com/en/page/816.html).

NCP presets were checked against upstream commit
`1c9bf83961dd954f5b3b64e0d101e4a8f12fcf1f` on 2026-09-11; the vendored file was unchanged.
New Pokémon can be configured manually even when no preset exists.

Direct damage effects use the engine's mechanics. Leek's critical-hit probability,
Rocky Helmet retaliation, switching items, trapping duration, and terrain duration
are not simulated by the single-attack calculator. Set critical hits and battle
state explicitly where applicable.

## UI development and browser checks

The responsive workspace uses the shared controls in `src/ui.rs`, the hand-written
`assets/app.css` stylesheet, and presentation helpers in `assets/app.js` (paths
relative to `crates/spreadlab-web`). Calculator requests and domain logic stay in
the existing adapter. The optimized side's six SP values are a read-only result
preview; they do not add constraints to optimization.

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
