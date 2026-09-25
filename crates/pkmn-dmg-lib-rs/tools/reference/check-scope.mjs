#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const scope = JSON.parse(fs.readFileSync(path.join(here, "scope-manifest.json"), "utf8"));
const reference = process.env.NCP_REFERENCE
  ? path.resolve(process.env.NCP_REFERENCE)
  : path.resolve(here, "../../reference/NCP-VGC-Damage-Calculator");
const files = [
  "script_res/damage_SV.js",
  "script_res/damage_MASTER.js",
  "script_res/ko_chance.js",
  "script_res/stat_data.js",
];
const discovered = [];
for (const relative of files) {
  const source = fs.readFileSync(path.join(reference, relative), "utf8");
  for (const match of source.matchAll(/^function\s+([A-Za-z0-9_]+)\s*\(/gm)) {
    discovered.push({ file: relative, name: match[1] });
  }
}
const classified = new Set();
for (const group of Object.values(scope.functions)) {
  for (const name of group.names) classified.add(name);
}
const unknown = discovered.filter(({ name }) => !classified.has(name));
if (unknown.length || scope.unknown.length) {
  process.stderr.write(`${JSON.stringify({ unclassifiedFunctions: unknown, manifestUnknown: scope.unknown }, null, 2)}\n`);
  process.exit(1);
}
process.stdout.write(`${JSON.stringify({ discovered: discovered.length, classified: classified.size }, null, 2)}\n`);
