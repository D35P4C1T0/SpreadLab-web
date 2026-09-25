# SpreadLab WebUI Handoff

This document is for the agent that will rebuild the removed legacy WebUI as a separate crate or app. The optimizer core is good; the WebUI should be a thin client over the public API surface and must preserve the behavior described here.

## Current Shape

The legacy WebUI was embedded behind the optional `webui` feature before it was removed from this crate:

- Static files: `src/web/static/index.html`, `src/web/static/app.css`, `src/web/static/app.js`
- Axum wrapper: `src/web/mod.rs`
- Public API DTOs and entry points: `src/api.rs`
- Showdown/custom set parser: `src/showdown.rs`
- Damage bridge into `pkmn-dmg-lib`: `src/damage_bridge.rs`
- Optimizer algorithms: `src/optimize.rs`

Legacy local run:

```sh
cargo run --features webui -- serve --host 127.0.0.1 --port 3000
```

The legacy static assets were compiled with `include_str!`, so that version required a server restart after CSS/JS/HTML changes.

## HTTP API

The current Axum routes are:

- `GET /api/meta`
- `POST /api/damage`
- `POST /api/survive`
- `POST /api/survive-sequence`
- `POST /api/ko`
- `POST /api/optimize/defensive`
- `POST /api/optimize/offensive`

Payload structs live in `src/api.rs`. Keep the new WebUI aligned with those structs unless the core crate exposes a cleaner generated client.

Important common fields:

```json
{
  "attacker_set": "Showdown/custom text",
  "defender_set": "Showdown/custom text",
  "move_name": "Close Combat",
  "move_times_affected": 0,
  "field": {
    "format": "Doubles",
    "weather": "None",
    "terrain": "None",
    "gravity": false,
    "fairy_aura": false,
    "protect": false,
    "helping_hand": false,
    "attacker_tailwind": false,
    "defender_tailwind": false,
    "defender_reflect": false,
    "defender_light_screen": false,
    "defender_aurora_veil": false,
    "defender_friend_guard": false,
    "attacker_boosts": { "attack": 0, "defense": 0, "special_attack": 0, "special_defense": 0, "speed": 0 },
    "defender_boosts": { "attack": 0, "defense": 0, "special_attack": 0, "special_defense": 0, "speed": 0 }
  }
}
```

`/api/survive` adds:

```json
{
  "max_ko_chance": 0.125,
  "hp_percent": 100,
  "nature": "Adamant",
  "optimize_nature": false,
  "limit": 10
}
```

`/api/survive-sequence` uses:

```json
{
  "defender_set": "...",
  "hits": [
    { "attacker_set": "...", "move_name": "Move 1", "move_times_affected": 0, "field": { } },
    { "attacker_set": "...", "move_name": "Move 2", "move_times_affected": 0, "field": { } }
  ],
  "max_ko_chance": 0.125,
  "hp_percent": 100,
  "nature": null,
  "optimize_nature": true,
  "limit": 10
}
```

For sequence survival, attacks are applied in order. Later attacks are recalculated with the defender's leftover current HP. If an earlier hit KOs, the sequence stops for that roll branch.

`/api/ko` adds:

```json
{
  "min_ko_chance": 1,
  "nature": null,
  "optimize_nature": true,
  "limit": 10
}
```

The current UI labels offensive result chance as "Outlive chance" by displaying `1 - ko_chance`.

## Set Text Parsing Contract

The text areas are the source of truth. Do not add extra controls for attacker nature, item state, gender, or status unless the core API gets explicit first-class fields. Use set text.

Supported set text basics:

```txt
Sneasler @ White Herb
Ability: Unburden
Level: 50
SPs: 10 Atk
Adamant Nature
- Close Combat
```

Nature may be inferred from calc shorthand:

```txt
SPs: 10+ Atk
```

This means `10 Atk` with an Adamant nature hint when there is no explicit `Nature` line. Explicit nature lines win.

Very important EV/SP rule:

- If an `EVs:` line has every value `<= 32`, treat those values as direct Champions stat points.
- If any `EVs:` value is `> 32`, convert legacy EVs to Champions SPs with `floor((EV + 4) / 8)`.

This was the source of a major WebUI bug. Example:

```txt
EVs: 20 HP / 10 Atk / 21 Def / 15 Spe
Adamant Nature
```

Must parse as `20 HP / 10 Atk / 21 Def / 15 Spe`, not as `3 HP / 1 Atk / 3 Def / 2 Spe`.

Supported damage annotations in set text:

```txt
Status: Burned
Ability On: true
Supreme Overlord Allies: 3
Fainted Allies: 3
Rivalry: same
Rivalry: opposite
Target: single
```

Meanings:

- `Status: Burned` applies `StatusCondition::Burned` to that Pokemon.
- `Ability On: true` activates ability-state mechanics like Multiscale.
- `Supreme Overlord Allies: 3` sets the fainted ally count.
- `Rivalry: same` / `opposite` models Rivalry through bridge base-power modifiers.
- `Target: single` makes a spread move such as Rock Slide behave as a single-target hit in Doubles.

Known bridge shims over the current damage library:

- `White Herb` currently maps as a neutral item placeholder because the locked `pkmn-dmg-lib` enum does not expose `Item::WhiteHerb`.
- Skill Link sets known multi-hit moves to 5 hits in the bridge.
- Sharpness slicing moves are marked in the bridge, including Psycho Cut.
- Rivalry is modeled as a base-power modifier: same gender 1.25x, opposite gender 0.75x.

## Field Controls

The existing UI has buttons for the field panel. Supported by the current API:

- Singles / Doubles
- Terrain: None, Electric, Grassy, Misty, Psychic
- Weather: None, Sun, Rain, Sand, Snow
- Fairy Aura
- Gravity
- Attacker Helping Hand
- Attacker Tailwind
- Defender Protect
- Defender Reflect
- Defender Light Screen
- Defender Aurora Veil
- Defender Tailwind
- Defender Friend Guard
- Attacker and defender stat boosts from -6 to +6 for Atk, Def, SpA, SpD, Spe

Currently visible but unsupported by the damage Field API:

- Attacker Protect
- Attacker Aurora Veil
- Attacker Reflect / Light Screen
- Attacker Friend Guard
- Stealth Rock
- Spikes
- Salt Cure
- Defender Helping Hand

If these are shown, mark them disabled/unsupported and explain through a small note or tooltip. Do not silently wire fake behavior.

## UX Notes

Keep the WebUI dense and functional. It is an optimizer/calculator, not a landing page.

Current useful layout:

- Controls rail on the left.
- Attacker and defender text areas in one column.
- Compact field panel beside them.
- Results table full width below.
- Multiple attackers for defensive sequence:
  - `+` beside Attacker title.
  - Tab per attacker.
  - Remove button only for attacker 2+.
  - One move selector per attacker, populated live from `- Move` lines.

Do not add redundant nature controls. Attacker nature must come from the pasted attacker set. The existing `Nature` selector is for the optimization target/candidate nature.

Percent display should truncate to one decimal like standard damage calculators:

- `92.571...` displays as `92.5`, not `92.6`.

For long roll lists:

- Show full rolls in summary for normal 16-roll calcs.
- For Parental Bond / Skill Link style roll explosions, consider showing roll count and unique totals instead of rendering a million values.

## Required Benchmark Suite

The next WebUI must reproduce these through its API path. The current repo has this as `api::tests::damage_benchmark_matches_known_calcs`.

1. `32+ Atk Sneasler Close Combat vs. 2 HP / 9 Def Chople Berry Kingambit`\
   `182-216 (102.8-122.0%)`\
   unique rolls: `[182, 186, 188, 192, 194, 198, 200, 204, 206, 210, 212, 216]`

2. `32+ Atk Kingambit Iron Head vs. 32 HP / 32 Def Mega Floette`\
   `134-158 (74.0-87.3%)`\
   unique rolls: `[134, 138, 140, 144, 146, 150, 152, 156, 158]`

3. `0 SpA Magnet Transistor Pikachu Thunderbolt vs. 0 HP / 0 SpD Milotic in Electric Terrain`\
   `102-120 (60.0-70.6%)`\
   unique rolls: `[102, 104, 108, 110, 114, 116, 120]`

4. `0 Atk Charizard Rock Slide vs. 0 HP / 0 Def Volcarona in Doubles`\
   `104-124 (65.0-77.5%)`\
   unique rolls: `[104, 108, 112, 116, 120, 124]`

5. `0 Atk Charizard Rock Slide vs. 0 HP / 0 Def Volcarona as single target in Doubles`\
   `140-168 (87.5-105.0%)`\
   unique rolls: `[140, 144, 148, 152, 156, 160, 164, 168]`

6. `0 Atk burned Machamp Drain Punch vs. 0 HP / 0 Def Snorlax through Reflect`\
   `51-60 (21.7-25.5%)`\
   unique rolls: `[51, 52, 53, 54, 55, 56, 57, 58, 59, 60]`

7. `0 SpA Ninetales Flamethrower vs. 0 HP / 0 SpD Scizor in Sun`\
   `304-364 (209.7-251.0%)`\
   unique rolls: `[304, 312, 316, 324, 328, 336, 340, 348, 352, 360, 364]`

8. `0 SpA Pelipper Weather Ball vs. 0 HP / 0 SpD Camerupt in Rain`\
   `412-492 (284.1-339.3%)`\
   unique rolls: `[412, 420, 424, 432, 436, 444, 448, 456, 460, 468, 472, 480, 484, 492]`

9. `0 SpA Abomasnow Blizzard vs. 0 HP / 0 SpD Dragonite with Multiscale active`\
   `86-104 (51.8-62.7%)`\
   unique rolls: `[86, 90, 92, 96, 98, 102, 104]`

10. `0 Atk Gyarados Waterfall vs. 0 HP / 0 Def Passho Berry Torkoal`\
    `42-49 (29.0-33.8%)`\
    unique rolls: `[42, 43, 45, 46, 48, 49]`

11. `0 SpA Gengar Shadow Ball vs. 0 HP / 0 SpD Kasib Berry Clefable`\
    `63-75 (37.1-44.1%)`\
    unique rolls: `[63, 64, 66, 67, 69, 70, 72, 73, 75]`

12. `0 Atk Mega Kangaskhan Parental Bond Double-Edge vs. 0 HP / 0 Def Blastoise`\
    `123-145 (79.9-94.2%)`, `roll_count=256`\
    unique totals: `[123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145]`

13. `0 Atk Skill Link Toucannon Bullet Seed vs. 0 HP / 0 Def Slowbro`\
    `110-130 (64.7-76.5%)`, `roll_count=1048576`\
    unique totals: `[110, 112, 114, 116, 118, 120, 122, 124, 126, 128, 130]`

14. `0 SpA Swift Swim Beartic Electro Ball vs. 0 HP / 0 SpD Pelipper in Rain`\
    `92-112 (68.1-83.0%)`\
    unique rolls: `[92, 96, 100, 104, 108, 112]`

15. `0 SpA Analytic Starmie Psychic vs. 0 HP / 0 SpD Venusaur moving last`\
    `134-158 (86.5-101.9%)`\
    unique rolls: `[134, 138, 140, 144, 146, 150, 152, 156, 158]`

16. `0 Atk Rivalry Luxray Wild Charge vs. 0 HP / 0 Def Pelipper, same gender`\
    `300-352 (222.2-260.7%)`\
    unique rolls: `[300, 304, 312, 316, 324, 328, 336, 340, 348, 352]`

17. `0 Atk Rivalry Luxray Wild Charge vs. 0 HP / 0 Def Pelipper, opposite gender`\
    `180-216 (133.3-160.0%)`\
    unique rolls: `[180, 184, 192, 196, 204, 208, 216]`

18. `0 SpA Fairy Aura Mega Floette Moonblast vs. 0 HP / 0 SpD Hydreigon`\
    `456-540 (273.1-323.4%)`\
    unique rolls: `[456, 460, 468, 472, 480, 484, 492, 496, 504, 508, 516, 520, 528, 532, 540]`

19. `0 Atk Sharpness Gallade Psycho Cut vs. 0 HP / 0 Def Toxapex`\
    `102-120 (81.6-96.0%)`\
    unique rolls: `[102, 104, 108, 110, 114, 116, 120]`

20. `0 Atk Supreme Overlord Kingambit Kowtow Cleave vs. 0 HP / 0 Def Gengar with 3 fainted allies`\
    `242-288 (179.3-213.3%)`\
    unique rolls: `[242, 246, 248, 252, 254, 258, 260, 264, 266, 270, 272, 276, 278, 282, 284, 288]`

## Last Known Important Bug

If the user pastes:

```txt
Sneasler @ White Herb
Ability: Unburden
Level: 50
EVs: 20 HP / 10 Atk / 21 Def / 15 Spe
Adamant Nature
- Close Combat
```

and runs Defensive min into Kingambit, the parser must treat `10 Atk` as 10 Champions points. The old parser down-converted it to 1 point and produced bad rows like `SPs: 9 Def` with `152-180`. Correct behavior:

- Raw `SPs: 9 Def` damage is `162-192`, 50% KO.
- Defensive min with `max_ko_chance=0.125` must not pick `SPs: 9 Def`; it needs more bulk.

This is covered by `defensive_min_treats_low_evs_as_champions_points`.

## Validation Commands

Run before handing off WebUI changes:

```sh
cargo test
cargo check
```

For browser verification in the external WebUI project, ensure its dev server/API adapter exercises the same API structs and benchmark cases.
