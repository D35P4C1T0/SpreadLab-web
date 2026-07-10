#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const MANIFEST = JSON.parse(fs.readFileSync(path.join(HERE, "source-manifest.json"), "utf8"));
const REFERENCE = process.env.NCP_REFERENCE
  ? path.resolve(process.env.NCP_REFERENCE)
  : path.resolve(HERE, "../../reference/NCP-VGC-Damage-Calculator");

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(1);
}

function verifySources() {
  for (const [relative, expected] of Object.entries(MANIFEST.files)) {
    const filename = path.join(REFERENCE, relative);
    if (!fs.existsSync(filename)) {
      fail(`missing pinned reference file: ${filename}`);
    }
    const actual = crypto.createHash("sha256").update(fs.readFileSync(filename)).digest("hex");
    if (actual !== expected) {
      fail(`reference hash mismatch for ${relative}: expected ${expected}, got ${actual}`);
    }
  }
}

function deepMerge(target, ...sources) {
  for (const source of sources) {
    if (!source || typeof source !== "object") continue;
    for (const [key, value] of Object.entries(source)) {
      if (Array.isArray(value)) target[key] = value.map((entry) =>
        entry && typeof entry === "object" ? deepMerge({}, entry) : entry);
      else if (value && typeof value === "object")
        target[key] = deepMerge(target[key] && typeof target[key] === "object" ? target[key] : {}, value);
      else target[key] = value;
    }
  }
  return target;
}

function jqueryStub() {
  const chain = {
    val: () => undefined,
    text: () => chain,
    prop: () => false,
    is: () => false,
  };
  return chain;
}
jqueryStub.extend = (...args) => {
  const deep = args[0] === true;
  const offset = deep ? 1 : 0;
  const target = args[offset] ?? {};
  const sources = args.slice(offset + 1);
  return deep ? deepMerge(target, ...sources) : Object.assign(target, ...sources);
};
jqueryStub.isEmptyObject = (value) => !value || Object.keys(value).length === 0;

function createContext() {
  const context = vm.createContext({
    console: { log() {}, warn() {}, error() {} },
    document: {},
    $: jqueryStub,
    Math,
    JSON,
    Array,
    Object,
    Number,
    String,
    Boolean,
    parseInt,
    parseFloat,
    isNaN,
    Infinity,
    NaN,
  });
  const files = [
    "script_res/nature_data.js",
    "script_res/stat_data.js",
    "script_res/type_data.js",
    "script_res/ability_data.js",
    "script_res/item_data.js",
    "script_res/move_data.js",
    "script_res/damage_MASTER.js",
    "script_res/damage_SV.js",
    "script_res/ko_chance.js",
  ];
  for (const relative of files) {
    vm.runInContext(fs.readFileSync(path.join(REFERENCE, relative), "utf8"), context, {
      filename: relative,
    });
  }
  vm.runInContext(`
    gen = 10;
    typeChart = TYPE_CHART_SV;
    moves = MOVES_CHAMPIONS;
    STATS = STATS_GSC;
    resultDisplayMode = "SPs";
    lastHighestStat = [-1, -1];
  `, context);
  return context;
}

const BUILDERS = String.raw`
const ORACLE_STAT_KEYS = ["at", "df", "sa", "sd", "sp"];
var setHasTypeFunc = function (...types) {
  return types.some((type) => type === this.type1 || type === this.type2);
};
const ORACLE_LONG_STAT_KEYS = {
  at: "attack", df: "defense", sa: "specialAttack", sd: "specialDefense", sp: "speed"
};

function oracleNonHpStat(base, points, nature, key) {
  const natureMods = NATURES[nature] || ["", ""];
  const modifier = natureMods[0] === key ? 1.1 : natureMods[1] === key ? 0.9 : 1;
  return Math.floor((Math.floor((base * 2 + 31) * 50 / 100) + 5 + points) * modifier);
}

function oraclePokemon(input) {
  const base = input.baseStats || {};
  const points = input.statPoints || {};
  const nature = input.nature || "Hardy";
  const rawStats = {};
  const hpBase = base.hp ?? 100;
  const maxHP = hpBase === 1 ? 1 : Math.floor((hpBase * 2 + 31) * 50 / 100) + 60 + (points.hp ?? 0);
  for (const key of ORACLE_STAT_KEYS) {
    const longKey = ORACLE_LONG_STAT_KEYS[key];
    rawStats[key] = oracleNonHpStat(base[longKey] ?? 100, points[longKey] ?? 0, nature, key);
  }
  const boosts = {};
  const sps = {};
  const evs = {};
  const ivs = {};
  for (const key of ORACLE_STAT_KEYS) {
    const longKey = ORACLE_LONG_STAT_KEYS[key];
    boosts[key] = input.boosts?.[longKey] ?? 0;
    sps[key] = points[longKey] ?? 0;
    evs[key] = 0;
    ivs[key] = 31;
  }
  const types = input.types || ["Normal", ""];
  return {
    name: input.name || "Pokemon",
    level: 50,
    type1: types[0] || "Normal",
    type2: types[1] || "",
    tera_type: input.teraType || types[0] || "Normal",
    teraSTAB1: types[0] || "",
    teraSTAB2: types[1] || "",
    maxHP: input.maxHp ?? maxHP,
    curHP: input.currentHp ?? input.maxHp ?? maxHP,
    HPSPs: points.hp ?? 0,
    HPEVs: 0,
    HPIVs: 31,
    HPraw: input.maxHp ?? maxHP,
    isDynamax: false,
    gmax_factor: 0,
    isTerastalize: input.isTerastalized ?? false,
    rawStats,
    boosts,
    stats: { ...rawStats },
    sps,
    evs,
    ivs,
    nature,
    ability: input.ability || "",
    abilityOn: input.abilityOn ?? ![
      "", "Flash Fire", "Plus", "Minus", "Trace", "Stakeout", "Sand Spit",
      "Battle Bond", "Electromorphosis", "Wind Power", "Seed Sower"
    ].includes(input.ability || ""),
    supremeOverlord: input.supremeOverlord ?? 0,
    rivalryGender: input.rivalryTarget || "",
    highestStat: input.highestStat ?? -1,
    item: input.item || "",
    status: input.status || "Healthy",
    toxicCounter: input.toxicCounter ?? (input.status === "Badly Poisoned" ? 1 : 0),
    glaiveRushMod: input.glaiveRushMod ?? false,
    weight: input.weightKg ?? 10,
    canEvolve: input.canEvolve ?? false,
    isTransformed: false,
    paradoxAbilityBoost: input.paradoxAbilityBoost ?? false,
    consumeResistBerry: false,
    hasCustomModifiers: false,
    customModifiers: {},
    hasType(...wanted) { return wanted.some((type) => type === this.type1 || type === this.type2); },
    moves: (input.moves || []).map(oracleMove),
  };
}

function oracleMove(input = {}) {
  const reference = moves[input.name] || {};
  return {
    ...reference,
    name: input.name || "(No Move)",
    bp: input.basePower ?? reference.bp ?? 0,
    type: input.type || reference.type || "Normal",
    category: input.category || reference.category || "Status",
    dealsPhysicalDamage: input.dealsPhysicalDamage ?? reference.dealsPhysicalDamage ?? false,
    isSpread: input.isSpread ?? reference.isSpread ?? false,
    isCrit: input.isCritical ?? reference.isCrit ?? reference.alwaysCrit ?? false,
    makesContact: input.makesContact ?? reference.makesContact ?? false,
    hasSecondaryEffect: input.hasSecondaryEffect ?? reference.hasSecondaryEffect ?? false,
    isPunch: input.isPunch ?? reference.isPunch ?? false,
    isBite: input.isBite ?? reference.isBite ?? false,
    isPulse: input.isPulse ?? reference.isPulse ?? false,
    isSound: input.isSound ?? reference.isSound ?? false,
    isSlice: input.isSlice ?? reference.isSlice ?? false,
    isBullet: input.isBullet ?? reference.isBullet ?? false,
    isWind: input.isWind ?? reference.isWind ?? false,
    isPriority: input.isPriority ?? reference.isPriority ?? false,
    isOHKO: input.isOhko ?? reference.isOHKO ?? false,
    getsStellarBoost: input.getsStellarBoost ?? reference.getsStellarBoost ?? false,
    isZ: false,
    isSignatureZ: false,
    isMax: false,
    breaksProtect: input.breaksProtect ?? reference.breaksProtect ?? false,
    hasRecoil: input.hasRecoil ?? reference.hasRecoil ?? false,
    hasCrashDamage: input.hasCrash ?? reference.hasCrashDamage ?? false,
    ignoresScreens: input.ignoresScreens ?? reference.ignoresScreens ?? false,
    ignoresDefenseBoosts: input.ignoresDefenseBoosts ?? reference.ignoresDefenseBoosts ?? false,
    ignoresBurn: input.ignoresBurn ?? reference.ignoresBurn ?? false,
    isDouble: input.isDoublePower ? 1 : 0,
    canDouble: input.canDouble ?? reference.canDouble ?? false,
    timesAffected: input.timesAffected ?? 0,
    currTripleHit: input.currentTripleHit ?? 1,
    hits: input.hits ?? 1,
    hitRange: input.hitRange || reference.hitRange || false,
    usedOppMoveIndex: input.usedOppMoveIndex ?? 0,
    isNextMove: false,
  };
}

function oracleSide(common, side = {}) {
  return {
    format: common.format || "Doubles",
    terrain: common.terrain || "",
    weather: common.weather || "",
    isGravity: common.gravity ?? false,
    isSR: side.stealthRock ?? false,
    spikes: side.spikes ?? 0,
    isReflect: side.reflect ?? false,
    isLightScreen: side.lightScreen ?? false,
    isForesight: side.foresight ?? false,
    isHelpingHand: side.helpingHand ?? false,
    isFriendGuard: side.friendGuard ?? false,
    isBattery: side.battery ?? false,
    isProtect: side.protect ?? false,
    isPowerSpot: side.powerSpot ?? false,
    isSteelySpirit: side.steelySpirit ?? false,
    isNeutralizingGas: common.neutralizingGas ?? false,
    isGMaxField: false,
    isFlowerGiftSpD: side.flowerGiftSpecialDefense ?? false,
    isFlowerGiftAtk: side.flowerGiftAttack ?? false,
    isTailwind: side.tailwind ?? false,
    isSaltCure: side.saltCure ?? false,
    isAuroraVeil: side.auroraVeil ?? false,
    isSwamp: side.swamp ?? false,
    isSeaFire: false,
    isRedItem: false,
    isBlueItem: false,
    isCharge: side.charge ?? false,
  };
}

function oracleField(input = {}) {
  let weather = input.weather || "";
  let terrain = input.terrain || "";
  const sides = [oracleSide(input, input.left), oracleSide(input, input.right)];
  return {
    getNeutralGas: () => input.neutralizingGas ?? false,
    getTailwind: (index) => sides[index].isTailwind,
    getWeather: () => weather,
    getTerrain: () => terrain,
    getSwamp: (index) => sides[index].isSwamp,
    clearWeather: () => { weather = ""; for (const side of sides) side.weather = ""; },
    clearTerrain: () => { terrain = ""; for (const side of sides) side.terrain = ""; },
    getSide: (index) => sides[index],
    _sides: sides,
  };
}

function oracleKoChances(damageIn, move, defender, field, attacker) {
  const preventsHeal = move.name === "Psychic Noise";
  const preventsHealItem = ["Knock Off", "Psychic Noise"].includes(move.name)
    || (["Thief", "Covet"].includes(move.name) && attacker.item === "");
  const preventsRestoreHP = preventsHealItem
    || (defender.item.includes(" Berry") && ["Bug Bite", "Pluck", "Incinerate"].includes(move.name));
  let restoreHP = getRestoreHP(defender.item, defender.maxHP, preventsRestoreHP);
  if (applyRipen(defender.ability === "Ripen", defender.item, restoreHP)) restoreHP *= 2;
  const [restoreThreshold] = getRestoreThreshold(
    defender.item, restoreHP, defender.maxHP, defender.ability === "Gluttony"
  );
  const multihit = move.hits > 1 || (damageIn.length > 1 && Array.isArray(damageIn[0]));
  const damage = damageArrToDict(damageIn, move.hits, defender.curHP, restoreHP, restoreThreshold);
  let hazards = 0;
  if (field.isSR && defender.ability !== "Magic Guard" && defender.item !== "Heavy-Duty Boots") {
    const effectiveness = typeChart.Rock[defender.type1] * (defender.type2 ? typeChart.Rock[defender.type2] : 1);
    hazards += Math.max(1, Math.floor(effectiveness * defender.maxHP / 8));
  }
  if (pIsGrounded(defender, field) && defender.ability !== "Magic Guard" && defender.item !== "Heavy-Duty Boots") {
    if (field.spikes === 1) hazards += Math.max(1, Math.floor(defender.maxHP / 8));
    else if (field.spikes === 2) hazards += Math.floor(defender.maxHP / 6);
    else if (field.spikes === 3) hazards += Math.floor(defender.maxHP / 4);
  }
  const eotDict = getAllEndOfTurnEffects(
    defender, field, false, preventsHeal, preventsHealItem, preventsRestoreHP
  );
  let eot = 0;
  let toxicCounter = 0;
  for (const effect of Object.values(eotDict)) {
    if (!effect.val) continue;
    if (effect.isToxic) {
      toxicCounter = effect.val;
      eot -= Math.floor(toxicCounter * defender.maxHP / 16);
    } else eot += effect.val;
  }
  return [1, 2, 3, 4].map((uses) => getKOChance(
    damage, multihit, defender.curHP - hazards, eot, uses, defender.maxHP,
    toxicCounter, restoreHP, restoreThreshold, 1, eotDict
  ));
}

function oracleRun(input) {
  lastHighestStat = [-1, -1];
  const left = oraclePokemon(input.left);
  const right = oraclePokemon(input.right);
  while (left.moves.length < 4) left.moves.push(oracleMove());
  while (right.moves.length < 4) right.moves.push(oracleMove());
  left.moves.length = 4;
  right.moves.length = 4;
  for (const pokemon of [left, right]) {
    if (pokemon.ability !== "Skill Link") continue;
    for (const move of pokemon.moves) {
      if (Array.isArray(move.hitRange)) move.hits = move.name === "Population Bomb" ? 10 : move.hitRange[1];
    }
  }
  const fieldInput = { ...(input.field || {}) };
  if (!fieldInput.weather) {
    const abilities = [left.ability, right.ability];
    if (abilities.includes("Drizzle")) fieldInput.weather = "Rain";
    else if (abilities.includes("Drought")) fieldInput.weather = "Sun";
    else if (abilities.includes("Sand Stream")) fieldInput.weather = "Sand";
    else if (abilities.includes("Snow Warning")) fieldInput.weather = "Snow";
  }
  if (!fieldInput.terrain && [left.ability, right.ability].includes("Electric Surge")) {
    fieldInput.terrain = "Electric";
  }
  const field = oracleField(fieldInput);
  const results = CALCULATE_ALL_MOVES_SV(left, right, field);
  return {
    schemaVersion: 1,
    left: results[0].map((result, index) => ({
      damage: result.damage,
      description: result.description,
      koText: getKOChanceText(result.damage, left.moves[index], right, field._sides[1], false, left.item === ""),
      koChances: oracleKoChances(result.damage, left.moves[index], right, field._sides[1], left),
      move: left.moves[index],
    })),
    right: results[1].map((result, index) => ({
      damage: result.damage,
      description: result.description,
      koText: getKOChanceText(result.damage, right.moves[index], left, field._sides[0], false, right.item === ""),
      koChances: oracleKoChances(result.damage, right.moves[index], left, field._sides[0], right),
      move: right.moves[index],
    })),
    state: {
      left: { ability: left.ability, item: left.item, types: [left.type1, left.type2], stats: left.stats, boosts: left.boosts },
      right: { ability: right.ability, item: right.item, types: [right.type1, right.type2], stats: right.stats, boosts: right.boosts },
    },
  };
}
`;

verifySources();
let inputText = "";
for await (const chunk of process.stdin) inputText += chunk;
if (!inputText.trim()) fail("oracle expects canonical JSON on stdin");
const context = createContext();
vm.runInContext(BUILDERS, context, { filename: "oracle-builders.js" });
context.__oracleInput = JSON.parse(inputText);
const output = vm.runInContext("Array.isArray(__oracleInput) ? __oracleInput.map(oracleRun) : oracleRun(__oracleInput)", context);
function compact(result) {
  const compactSide = (side) => side.map(({ damage, koChances, move }) => ({
    damage, koChances,
    move: {
      name: move.name, bp: move.bp, type: move.type, category: move.category,
      hits: move.hits, isCrit: move.isCrit, isSpread: move.isSpread,
      makesContact: move.makesContact, isPriority: move.isPriority,
    },
  }));
  return { schemaVersion: result.schemaVersion, left: compactSide(result.left), right: compactSide(result.right), state: result.state };
}
const serialized = process.env.NCP_ORACLE_COMPACT === "1"
  ? (Array.isArray(output) ? output.map(compact) : compact(output))
  : output;
process.stdout.write(`${JSON.stringify(serialized, null, 2)}\n`);
