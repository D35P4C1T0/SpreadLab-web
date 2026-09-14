# Pinned JavaScript Reference Oracle

`oracle.mjs` executes the exact Champions generation-10 calculation path from
the pinned upstream checkout. It verifies every consumed source hash before
running and communicates through canonical JSON on stdin/stdout.

Prepare the ignored reference checkout:

```sh
git clone https://github.com/nerd-of-now/NCP-VGC-Damage-Calculator.git reference/NCP-VGC-Damage-Calculator
git -C reference/NCP-VGC-Damage-Calculator checkout 1369b359b85f0a6343df006acde92cc4a7d07805
```

Run smoke input:

```sh
node tools/reference/oracle.mjs < tools/reference/smoke-input.json
```

Extract exact active inventory:

```sh
node tools/reference/inventory.mjs --summary
```

Verify committed inventory and function classification:

```sh
node tools/reference/inventory.mjs --check data/champions/generated/reference-inventory.json
node tools/reference/check-scope.mjs
```

Set `NCP_REFERENCE` to use another checkout location. Hash verification still
requires the pinned contents.
