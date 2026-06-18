# SpreadLab Web

Dedicated Rust WebUI for [SpreadLab](https://github.com/D35P4C1T0/SpreadLab), a Pokémon Champions damage and spread optimization tool for:

```text
[Gen 9 Champions] VGC 2026 Reg M-B
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

## Run

From the repository root:

```sh
cargo run -p spreadlab-web -- serve --host 127.0.0.1 --port 3000
```

Then open:

```text
http://127.0.0.1:3000/survive
```

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
