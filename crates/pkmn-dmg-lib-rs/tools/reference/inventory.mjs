#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(fs.readFileSync(path.join(here, "source-manifest.json"), "utf8"));
const reference = process.env.NCP_REFERENCE
  ? path.resolve(process.env.NCP_REFERENCE)
  : path.resolve(here, "../../reference/NCP-VGC-Damage-Calculator");

function merge(target, ...sources) {
  for (const source of sources) {
    if (!source || typeof source !== "object") continue;
    for (const [key, value] of Object.entries(source)) {
      if (Array.isArray(value)) target[key] = value.map((entry) =>
        entry && typeof entry === "object" ? merge({}, entry) : entry);
      else if (value && typeof value === "object")
        target[key] = merge(target[key] && typeof target[key] === "object" ? target[key] : {}, value);
      else target[key] = value;
    }
  }
  return target;
}

for (const [relative, expected] of Object.entries(manifest.files)) {
  const filename = path.join(reference, relative);
  if (!fs.existsSync(filename)) throw new Error(`missing pinned reference file: ${filename}`);
  const actual = crypto.createHash("sha256").update(fs.readFileSync(filename)).digest("hex");
  if (actual !== expected) throw new Error(`reference hash mismatch for ${relative}`);
}

const jquery = () => ({ val: () => undefined });
jquery.extend = (...args) => {
  const deep = args[0] === true;
  const offset = deep ? 1 : 0;
  const target = args[offset] ?? {};
  const sources = args.slice(offset + 1);
  return deep ? merge(target, ...sources) : Object.assign(target, ...sources);
};
const context = vm.createContext({ $: jquery, console });

for (const relative of [
  "script_res/ability_data.js",
  "script_res/item_data.js",
  "script_res/move_data.js",
  "script_res/pokedex.js",
  "script_res/setdex_ncp-g10.js",
]) {
  vm.runInContext(fs.readFileSync(path.join(reference, relative), "utf8"), context, { filename: relative });
}

const output = vm.runInContext(`(() => {
  const sets = [];
  for (const [pokemon, pokemonSets] of Object.entries(SETDEX_GEN10)) {
    for (const [name, set] of Object.entries(pokemonSets)) sets.push({ pokemon, name, ...set });
  }
  return {
    schemaVersion: 1,
    sourceCommit: ${JSON.stringify(manifest.commit)},
    pokemon: Object.entries(POKEDEX_CHAMPIONS).map(([name, data]) => ({ name, ...data })),
    moves: Object.entries(MOVES_CHAMPIONS).map(([name, data]) => ({ name, ...data })),
    items: ITEMS_CHAMPIONS,
    abilities: [...new Set(ABILITIES_CHAMPIONS)],
    duplicateAbilities: ABILITIES_CHAMPIONS.filter((name, index, all) => all.indexOf(name) !== index),
    sets,
  };
})()`, context);
output.counts = {
  pokemon: output.pokemon.length,
  moves: output.moves.length,
  items: output.items.length,
  abilities: output.abilities.length,
  sets: output.sets.length,
  setPokemon: new Set(output.sets.map((set) => set.pokemon)).size,
};

const serialized = `${JSON.stringify(process.argv.includes("--summary") ? output.counts : output, null, 2)}\n`;
const checkIndex = process.argv.indexOf("--check");
if (checkIndex !== -1) {
  const expectedPath = process.argv[checkIndex + 1];
  if (!expectedPath) throw new Error("--check requires a committed inventory path");
  const expected = fs.readFileSync(expectedPath, "utf8");
  if (serialized !== expected) throw new Error(`reference inventory drift: regenerate ${expectedPath}`);
  process.stdout.write(`${JSON.stringify(output.counts, null, 2)}\n`);
  process.exit(0);
}
const writeIndex = process.argv.indexOf("--write");
if (writeIndex !== -1) {
  const destination = process.argv[writeIndex + 1];
  if (!destination) throw new Error("--write requires a destination");
  fs.mkdirSync(path.dirname(path.resolve(destination)), { recursive: true });
  fs.writeFileSync(destination, serialized);
} else process.stdout.write(serialized);
