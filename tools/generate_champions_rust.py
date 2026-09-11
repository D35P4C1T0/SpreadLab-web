#!/usr/bin/env python3
"""Generate Rust summary constants from vendored Champions JSON."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data" / "champions" / "generated" / "champions-data.json"
ITEMS = ROOT / "data" / "champions" / "items.json"
ROSTER_M_A = ROOT / "data" / "champions" / "regulation_m_a_pokemon.json"
ROSTER_M_B_ADDITIONS = ROOT / "data" / "champions" / "regulation_m_b_additions.json"
ROSTER_M_C_ADDITIONS = ROOT / "data" / "champions" / "regulation_m_c_additions.json"
OUT = ROOT / "src" / "data" / "champions.rs"
REFERENCE = ROOT / "data" / "champions" / "generated" / "reference-inventory.json"


def rust_str(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def enum_variant(value: str) -> str:
    variant = re.sub(r"[^A-Za-z0-9]", "", value)
    return {
        "MindsEye": "MindEye",
        "GoodasGold": "GoodAsGold",
        "ZerotoHero": "ZeroToHero",
    }.get(variant, variant)


def rust_float(value: float) -> str:
    rendered = repr(float(value))
    return rendered if "." in rendered else f"{rendered}.0"


def push_string_list(lines: list[str], values: list[str]) -> None:
    for value in values:
        lines.append(f"    {rust_str(value)},")


def regulation_m_b_roster(roster_m_a: list[str]) -> list[str]:
    additions = json.loads(ROSTER_M_B_ADDITIONS.read_text())
    return roster_m_a + additions["regular"] + additions["mega"]


def main() -> None:
    data = json.loads(DATA.read_text())
    items = json.loads(ITEMS.read_text())
    roster_m_a = json.loads(ROSTER_M_A.read_text())
    roster_m_b = regulation_m_b_roster(roster_m_a)
    additions_c = json.loads(ROSTER_M_C_ADDITIONS.read_text())
    roster_m_c = roster_m_b + additions_c["regular"] + additions_c["mega"]
    reference = json.loads(REFERENCE.read_text())
    reference_species_by_name = {entry["name"]: entry for entry in reference["pokemon"]}

    lines = [
        "//! Pinned Pokemon Champions data lists exposed for downstream tools.",
        "//!",
        "//! These constants are generated from the vendored Champions data in `data/champions`.",
        "//! They intentionally avoid runtime fetching so optimizer crates can treat this crate as",
        "//! local ground truth for legal names and lookup menus.",
        "",
        "use crate::types::{Ability, Category, Item, Move, Nature, Pokemon, PokemonType, StatTable};",
        "",
        "/// Lightweight Pokemon/form entry from normalized Champions data.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ChampionSpeciesSummary {",
        "    /// Internal normalized form id from the generated Champions data.",
        "    pub id: &'static str,",
        "    /// Display name, including suffixes where applicable.",
        "    pub display_name: &'static str,",
        "    /// Whether this species/form is usable under the vendored Regulation M-A roster.",
        "    pub is_regulation_m_a: bool,",
        "    /// Whether this species/form is usable under the vendored Regulation M-B roster.",
        "    pub is_regulation_m_b: bool,",
        "    /// Whether this species/form is usable under Regulation M-C.",
        "    pub is_regulation_m_c: bool,",
        "}",
        "",
        "/// Lightweight ability entry from normalized Champions data.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ChampionAbilitySummary {",
        "    /// Champions ability id.",
        "    pub id: u16,",
        "    /// English ability name.",
        "    pub name: &'static str,",
        "    /// English ability description from `champout`.",
        "    pub description: &'static str,",
        "}",
        "",
        "/// Typed Pokemon/form metadata from a Champions data source.",
        "#[derive(Debug, Clone, Copy, PartialEq)]",
        "pub struct ChampionReferenceSpecies {",
        "    pub name: &'static str,",
        "    pub types: [Option<PokemonType>; 2],",
        "    pub base_stats: StatTable,",
        "    pub weight_kg: f32,",
        "    pub default_ability: Ability,",
        "}",
        "",
        "impl ChampionReferenceSpecies {",
        "    pub fn pokemon(self, stat_points: StatTable, nature: Nature) -> Pokemon {",
        "        let mut pokemon = Pokemon::champions(self.name, self.types, self.base_stats, stat_points, nature);",
        "        pokemon.weight_kg = self.weight_kg;",
        "        pokemon.ability = self.default_ability;",
        "        pokemon.ability_on = self.default_ability.is_on_by_default();",
        "        pokemon",
        "    }",
        "}",
        "",
        "/// Exact move metadata used by the pinned browser reference.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ChampionReferenceMove {",
        "    pub name: &'static str,",
        "    pub base_power: u16,",
        "    pub type_: PokemonType,",
        "    pub category: Category,",
        "    pub deals_physical_damage: bool,",
        "    pub is_spread: bool,",
        "    pub is_critical: bool,",
        "    pub makes_contact: bool,",
        "    pub has_secondary_effect: bool,",
        "    pub is_punch: bool,",
        "    pub is_bite: bool,",
        "    pub is_pulse: bool,",
        "    pub is_sound: bool,",
        "    pub is_slice: bool,",
        "    pub is_bullet: bool,",
        "    pub is_wind: bool,",
        "    pub is_priority: bool,",
        "    pub is_ohko: bool,",
        "    pub has_recoil: bool,",
        "    pub has_crash: bool,",
        "    pub ignores_screens: bool,",
        "    pub ignores_defense_boosts: bool,",
        "    pub ignores_burn: bool,",
        "    pub can_double: bool,",
        "    pub is_multi_hit: bool,",
        "    pub default_hits: u8,",
        "}",
        "",
        "impl ChampionReferenceMove {",
        "    pub fn move_(self) -> Move {",
        "        let mut move_ = Move::new(self.name, self.base_power, self.type_, self.category);",
        "        move_.deals_physical_damage = self.deals_physical_damage;",
        "        move_.is_spread = self.is_spread;",
        "        move_.is_critical = self.is_critical;",
        "        move_.makes_contact = self.makes_contact;",
        "        move_.has_secondary_effect = self.has_secondary_effect;",
        "        move_.is_punch = self.is_punch;",
        "        move_.is_bite = self.is_bite;",
        "        move_.is_pulse = self.is_pulse;",
        "        move_.is_sound = self.is_sound;",
        "        move_.is_slice = self.is_slice;",
        "        move_.is_bullet = self.is_bullet;",
        "        move_.is_wind = self.is_wind;",
        "        move_.is_priority = self.is_priority;",
        "        move_.is_ohko = self.is_ohko;",
        "        move_.has_recoil = self.has_recoil;",
        "        move_.has_crash = self.has_crash;",
        "        move_.ignores_screens = self.ignores_screens;",
        "        move_.ignores_defense_boosts = self.ignores_defense_boosts;",
        "        move_.ignores_burn = self.ignores_burn;",
        "        move_.hits = self.default_hits;",
        "        move_.is_multi_hit = self.is_multi_hit;",
        "        move_",
        "    }",
        "}",
        "",
        "/// Exact bundled set from the pinned Champions reference.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub struct ChampionReferenceSet {",
        "    pub pokemon: &'static str,",
        "    pub name: &'static str,",
        "    pub stat_points: StatTable,",
        "    pub nature: Nature,",
        "    pub ability: Ability,",
        "    pub item: Item,",
        "    pub moves: [&'static str; 4],",
        "}",
        "",
        "impl ChampionReferenceSet {",
        "    pub fn pokemon_(self) -> Option<Pokemon> {",
        "        let species = champions_reference_species(self.pokemon)?;",
        "        let mut pokemon = species.pokemon(self.stat_points, self.nature);",
        "        pokemon.ability = self.ability;",
        "        pokemon.ability_on = self.ability.is_on_by_default();",
        "        pokemon.item = self.item;",
        "        Some(pokemon)",
        "    }",
        "",
        "    pub fn moves_(self) -> Option<[Move; 4]> {",
        "        Some([",
        "            champions_reference_move(self.moves[0])?.move_(),",
        "            champions_reference_move(self.moves[1])?.move_(),",
        "            champions_reference_move(self.moves[2])?.move_(),",
        "            champions_reference_move(self.moves[3])?.move_(),",
        "        ])",
        "    }",
        "}",
        "",
        "/// Raw JSON list of Champions held item names.",
        'pub const CHAMPIONS_ITEMS_JSON: &str = include_str!("../../data/champions/items.json");',
        "",
        "/// Raw JSON list of Regulation M-A legal Pokemon roster names.",
        "pub const REGULATION_M_A_POKEMON_JSON: &str =",
        '    include_str!("../../data/champions/regulation_m_a_pokemon.json");',
        "",
        "/// Raw JSON object with Regulation M-B additions over M-A.",
        "pub const REGULATION_M_B_ADDITIONS_JSON: &str =",
        '    include_str!("../../data/champions/regulation_m_b_additions.json");',
        "",
        "/// Champions held item names currently available in the vendored item list.",
        "pub const CHAMPIONS_ITEMS: &[&str] = &[",
    ]
    push_string_list(lines, items)
    lines.extend(
        [
            "];",
            "",
            "/// Regulation M-A legal Pokemon roster names.",
            "pub const REGULATION_M_A_POKEMON: &[&str] = &[",
        ]
    )
    push_string_list(lines, roster_m_a)
    lines.extend(
        [
            "];",
            "",
            "/// Regulation M-B legal Pokemon roster names.",
            "pub const REGULATION_M_B_POKEMON: &[&str] = &[",
        ]
    )
    push_string_list(lines, roster_m_b)
    lines.extend([
        "];",
        "",
        "/// Regulation M-C additions over M-B.",
        'pub const REGULATION_M_C_ADDITIONS_JSON: &str = include_str!("../../data/champions/regulation_m_c_additions.json");',
        "/// Regulation M-C legal Pokemon roster names.",
        "pub const REGULATION_M_C_POKEMON: &[&str] = &[",
    ])
    push_string_list(lines, roster_m_c)
    lines.extend(
        [
            "];",
            "",
            "/// Pokemon/form summaries from normalized Champions data.",
            "pub const CHAMPIONS_SPECIES: &[ChampionSpeciesSummary] = &[",
        ]
    )
    for entry in data["species"]:
        lines.extend(
            [
                "    ChampionSpeciesSummary {",
                f"        id: {rust_str(entry['id'])},",
                f"        display_name: {rust_str(entry['displayName'])},",
                f"        is_regulation_m_a: {str(entry['isRegulationMA']).lower()},",
                f"        is_regulation_m_b: {str(entry['isRegulationMB']).lower()},",
                f"        is_regulation_m_c: {str(entry['isRegulationMC']).lower()},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "/// Ability summaries from normalized Champions data.",
            "pub const CHAMPIONS_ABILITIES: &[ChampionAbilitySummary] = &[",
        ]
    )
    for entry in data["abilities"]:
        lines.extend(
            [
                "    ChampionAbilitySummary {",
                f"        id: {entry['id']},",
                f"        name: {rust_str(entry['name'])},",
                f"        description: {rust_str(entry['description'])},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "/// Exact normal-dex Pokemon/forms active in the pinned Champions reference.",
            "pub const CHAMPIONS_REFERENCE_SPECIES: &[ChampionReferenceSpecies] = &[",
        ]
    )
    for entry in reference["pokemon"]:
        bs = entry["bs"]
        second_type = (
            f"Some(PokemonType::{enum_variant(entry['t2'])})"
            if entry.get("t2")
            else "None"
        )
        ability = enum_variant(entry.get("ab", "")) or "None"
        lines.extend(
            [
                "    ChampionReferenceSpecies {",
                f"        name: {rust_str(entry['name'])},",
                f"        types: [Some(PokemonType::{enum_variant(entry['t1'])}), {second_type}],",
                f"        base_stats: StatTable::new({bs['hp']}, {bs['at']}, {bs['df']}, {bs['sa']}, {bs['sd']}, {bs['sp']}),",
                f"        weight_kg: {rust_float(entry.get('w', 0))},",
                f"        default_ability: Ability::{ability},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "/// Exact moves active in the pinned Champions reference.",
            "pub const CHAMPIONS_REFERENCE_MOVES: &[ChampionReferenceMove] = &[",
        ]
    )
    for entry in reference["moves"]:
        hit_range = entry.get("hitRange", 1)
        default_hits = hit_range if isinstance(hit_range, int) else hit_range[0]
        lines.extend(
            [
                "    ChampionReferenceMove {",
                f"        name: {rust_str(entry['name'])},",
                f"        base_power: {entry.get('bp', 0)},",
                f"        type_: PokemonType::{enum_variant(entry['type'])},",
                f"        category: Category::{enum_variant(entry['category'])},",
                f"        deals_physical_damage: {str(entry.get('dealsPhysicalDamage', False)).lower()},",
                f"        is_spread: {str(entry.get('isSpread', False)).lower()},",
                f"        is_critical: {str(entry.get('alwaysCrit', False)).lower()},",
                f"        makes_contact: {str(entry.get('makesContact', False)).lower()},",
                f"        has_secondary_effect: {str(entry.get('hasSecondaryEffect', False)).lower()},",
                f"        is_punch: {str(entry.get('isPunch', False)).lower()},",
                f"        is_bite: {str(entry.get('isBite', False)).lower()},",
                f"        is_pulse: {str(entry.get('isPulse', False)).lower()},",
                f"        is_sound: {str(entry.get('isSound', False)).lower()},",
                f"        is_slice: {str(entry.get('isSlice', False)).lower()},",
                f"        is_bullet: {str(entry.get('isBullet', False)).lower()},",
                f"        is_wind: {str(entry.get('isWind', False)).lower()},",
                f"        is_priority: {str(entry.get('isPriority', False)).lower()},",
                f"        is_ohko: {str(entry.get('isOHKO', False)).lower()},",
                f"        has_recoil: {str(bool(entry.get('recoilHP'))).lower()},",
                f"        has_crash: {str(entry.get('hasCrash', False)).lower()},",
                f"        ignores_screens: {str(entry.get('ignoresScreens', False)).lower()},",
                f"        ignores_defense_boosts: {str(entry.get('ignoresDefenseBoosts', False)).lower()},",
                f"        ignores_burn: {str(entry.get('ignoresBurn', False)).lower()},",
                f"        can_double: {str(entry.get('canDouble', False)).lower()},",
                f"        is_multi_hit: {str('hitRange' in entry).lower()},",
                f"        default_hits: {default_hits},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "/// Unique abilities active in the pinned Champions reference.",
            "pub const CHAMPIONS_REFERENCE_ABILITIES: &[&str] = &[",
        ]
    )
    push_string_list(lines, reference["abilities"])
    lines.extend(
        [
            "];",
            "",
            "/// Typed unique abilities active in the pinned Champions reference.",
            "pub const CHAMPIONS_REFERENCE_ABILITY_VALUES: &[(&str, Ability)] = &[",
        ]
    )
    for ability in reference["abilities"]:
        lines.append(f"    ({rust_str(ability)}, Ability::{enum_variant(ability)}),")
    lines.extend(
        [
            "];",
            "",
            "/// Typed items active in the pinned Champions reference.",
            "pub const CHAMPIONS_REFERENCE_ITEM_VALUES: &[(&str, Item)] = &[",
        ]
    )
    for item in reference["items"]:
        lines.append(f"    ({rust_str(item)}, Item::{enum_variant(item)}),")
    lines.extend(
        [
            "];",
            "",
            "/// Bundled competitive sets active in the pinned Champions reference.",
            "pub const CHAMPIONS_REFERENCE_SETS: &[ChampionReferenceSet] = &[",
        ]
    )
    for entry in reference["sets"]:
        sps = entry["sps"]
        species = reference_species_by_name[entry["pokemon"]]
        ability_name = entry.get("ability", species.get("ab", ""))
        item_name = entry.get("item", "")
        moves = list(entry["moves"])
        while len(moves) < 4:
            moves.append("(No Move)")
        lines.extend(
            [
                "    ChampionReferenceSet {",
                f"        pokemon: {rust_str(entry['pokemon'])},",
                f"        name: {rust_str(entry['name'])},",
                f"        stat_points: StatTable::new({sps.get('hp', 0)}, {sps.get('at', 0)}, {sps.get('df', 0)}, {sps.get('sa', 0)}, {sps.get('sd', 0)}, {sps.get('sp', 0)}),",
                f"        nature: Nature::{enum_variant(entry['nature'])},",
                f"        ability: Ability::{enum_variant(ability_name) or 'None'},",
                f"        item: Item::{enum_variant(item_name) or 'None'},",
                f"        moves: [{', '.join(rust_str(move) for move in moves[:4])}],",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "/// Find a Champions item name by exact English name.",
            "pub fn champions_item(name: &str) -> Option<&'static str> {",
            "    CHAMPIONS_ITEMS.iter().copied().find(|item| *item == name)",
            "}",
            "",
            "/// Find a Regulation M-A Pokemon roster name by exact English name.",
            "pub fn regulation_m_a_pokemon(name: &str) -> Option<&'static str> {",
            "    REGULATION_M_A_POKEMON",
            "        .iter()",
            "        .copied()",
            "        .find(|pokemon| *pokemon == name)",
            "}",
            "",
            "/// Find a Regulation M-B Pokemon roster name by exact English name.",
            "pub fn regulation_m_b_pokemon(name: &str) -> Option<&'static str> {",
            "    REGULATION_M_B_POKEMON",
            "        .iter()",
            "        .copied()",
            "        .find(|pokemon| *pokemon == name)",
            "}",
            "",
            "/// Find a Champions Pokemon/form summary by exact display name.",
            "pub fn champions_species(display_name: &str) -> Option<ChampionSpeciesSummary> {",
            "    CHAMPIONS_SPECIES",
            "        .iter()",
            "        .copied()",
            "        .find(|species| species.display_name == display_name)",
            "}",
            "",
            "/// Find a Champions ability summary by exact English name.",
            "pub fn champions_ability(name: &str) -> Option<ChampionAbilitySummary> {",
            "    CHAMPIONS_ABILITIES",
            "        .iter()",
            "        .copied()",
            "        .find(|ability| ability.name == name)",
            "}",
            "",
            "/// Find exact pinned-reference Pokemon/form metadata.",
            "pub fn champions_reference_species(name: &str) -> Option<ChampionReferenceSpecies> {",
            "    CHAMPIONS_REFERENCE_SPECIES.iter().copied().find(|entry| entry.name == name)",
            "}",
            "",
            "/// Find exact pinned-reference move metadata.",
            "pub fn champions_reference_move(name: &str) -> Option<ChampionReferenceMove> {",
            "    CHAMPIONS_REFERENCE_MOVES.iter().copied().find(|entry| entry.name == name)",
            "}",
            "",
            "/// Find a bundled set by exact Pokemon and set names.",
            "pub fn champions_reference_set(pokemon: &str, name: &str) -> Option<ChampionReferenceSet> {",
            "    CHAMPIONS_REFERENCE_SETS.iter().copied().find(|entry| entry.pokemon == pokemon && entry.name == name)",
            "}",
            "",
        ]
    )
    lines.extend([
        "/// Look up a Regulation M-C roster name.",
        "pub fn regulation_m_c_pokemon(name: &str) -> Option<&'static str> {",
        "    REGULATION_M_C_POKEMON.iter().copied().find(|entry| *entry == name)",
        "}",
        "",
        "/// Typed items available in Champions, including Regulation M-C additions.",
        "pub const CHAMPIONS_ITEM_VALUES: &[(&str, Item)] = &[",
    ])
    for item in items:
        lines.append(f"    ({rust_str(item)}, Item::{enum_variant(item)}),")
    lines.extend(["];", "", "/// Current source-dump species metadata (independent of the pinned browser reference).",
                  "pub const CHAMPIONS_CURRENT_SPECIES: &[ChampionReferenceSpecies] = &["])
    for entry in data["species"]:
        bs = entry["baseStats"]
        type1, type2 = entry["types"]
        second = "None" if type1 == type2 else f"Some(PokemonType::{type2})"
        ability = enum_variant(entry["abilities"][0]["name"])
        lines.extend([
            "    ChampionReferenceSpecies {",
            f"        name: {rust_str(entry['displayName'])},",
            f"        types: [Some(PokemonType::{type1}), {second}],",
            f"        base_stats: StatTable::new({bs['hp']}, {bs['attack']}, {bs['defense']}, {bs['specialAttack']}, {bs['specialDefense']}, {bs['speed']}),",
            f"        weight_kg: {rust_float(entry['weightKg'])},",
            f"        default_ability: Ability::{ability},",
            "    },",
        ])
    lines.extend(["];", "", "/// Find current source-dump metadata by exact display name.",
        "pub fn champions_current_species(name: &str) -> Option<ChampionReferenceSpecies> {",
        "    CHAMPIONS_CURRENT_SPECIES.iter().copied().find(|entry| entry.name == name)", "}", ""])
    OUT.write_text("\n".join(lines))
    subprocess.run(["rustfmt", "--edition", "2021", str(OUT)], check=True)
    print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
