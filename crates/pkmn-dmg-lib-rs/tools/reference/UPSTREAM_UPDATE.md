# September 2026 upstream update

Audited 2026-09-14. Reference advanced from
`dfbf020d4ed7df8921c6e11bbaa23410f6ca1448` to
`1369b359b85f0a6343df006acde92cc4a7d07805` (2026-09-12, “Reg M-C sets”).

## Data and mechanics

- Regenerated the exact reference inventory and typed Rust data: 346 Pokemon/forms,
  511 moves, 166 items, 216 unique abilities, 151 sets for 90 Pokemon.
- Imported all upstream move changes, including Pound, Eerie Spell, Freeze-Dry,
  Electro Shot, Slash, Meteor Assault, Snipe Shot, and Double Shock metadata.
- Piercing Drill/Unseen Fist Protect quartering now requires effective contact.
  Protect-breaking moves alone do not qualify for quartering.
- Terrain Pulse requires grounding for its type and power changes. Eelevate is
  recognized by Ground immunity, terrain, and Spikes checks.
- Gyro Ball includes the final +1; Pain Split rounds the shared HP before finding
  the signed change. Critical Meteor Beam/Electro Shot ignore negative boosts.
- Plus/Minus affect special moves only. Libero gets Protean-equivalent STAB.
  Mega Sol doubles Weather Ball power even with no actual weather.
- Added Grassy/Psychic Surge identities and automatic terrain setup, following
  the existing Electric Surge convention when no explicit terrain is supplied.
- Added Aqua Ring, Ingrain, Leech Seed, Nightmare, Curse, binding, and Sticky Barb
  residual support, plus the Sea of Fire/Bad Dreams slots in residual ordering.
  Big Root, Magic Guard, healing suppression, and berry timing are modeled.
- KO probabilities use cumulative damage distributions and healing markers in
  upstream order, including entry-hazard and multi-hit interactions.
- Exposed the newly changed attacker HP components as
  `DamageResult.attacker_hp_effects.pain_split` and `.leech_seed`. Positive HP
  changes mean damage, negative changes mean healing. These components are not
  total recoil/drain calculations. The pure `calcUserHP` helper is extracted
  from hash-verified `ap_calc.js` to test them without executing DOM code.

## Compatibility boundaries

This is the generation-10 normal-dex calculator, not a complete turn simulator.
Eelevate's post-KO stat boost, Toxic Spikes/Sticky Web state transitions, and
Spicy Spray's persistent status after the final hit are not simulated upstream.
Spicy Spray's between-hit burn effects remain covered by the multi-hit oracle.

Upstream `pIsGrounded` applies its directional `isIngrain` flag to both attacker
and defender grounding queries. `Field.ingrain` deliberately preserves this.
The Ground-immunity helper separately ignores that flag, also matching upstream.
Nightmare and Leech Seed field toggles are treated as already-applied effects;
no additional sleep or Grass-type eligibility check is imposed.

Binding Band is read from the defender by upstream. Contrary at -6 temporarily
uses -7 for a charging move because of upstream's OR condition. The KO engine
marks first-move berries before hazards, skips residual processing when the net
residual sum is zero, and reuses the second-turn toxic counter for later uses.
These behaviors are preserved rather than silently corrected.

Sticky Barb is not in the active M-C item inventory. Its new upstream residual
branch references out-of-scope `move` and `isItemlessAttacker` variables and throws
`ReferenceError: move is not defined`. Rust implements the evident intended
1/8-HP residual with the itemless-contact and Magic Guard exceptions, verified
with a Rust fixture. The oracle is not patched to conceal this upstream failure.

Existing `Field` JSON can omit the new flags; serde defaults them to false.
Existing result JSON can omit `attacker_hp_effects`. Rust callers constructing
complete `Field` or `DamageResult` literals must supply the added fields or use
`Field::default()` where appropriate. New enum variants may require updating
exhaustive matches.

## Verification

The live differential suite covers all active moves, sets, abilities, items,
field/state partitions, multi-hit recalculation, and KO odds. Added matrices
exercise the updated move/ability behavior, charging-move boost boundaries,
residual effects with items and immunity abilities, and HP/berry thresholds.
The existing fixture suite also runs against the refreshed inventories.

Run the CI checks described in `.github/workflows/ci.yml`, plus:

```sh
cargo test --test reference_oracle upstream_update
node tools/reference/inventory.mjs --check data/champions/generated/reference-inventory.json
node tools/reference/check-scope.mjs
```
