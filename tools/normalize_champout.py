#!/usr/bin/env python3
"""Normalize vendored champout JSON into compact Rust-friendly dataset."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / "data" / "champions" / "champout" / "raw"
OUT = ROOT / "data" / "champions" / "generated" / "champions-data.json"
ROSTER_M_A = ROOT / "data" / "champions" / "regulation_m_a_pokemon.json"
ROSTER_M_B_ADDITIONS = ROOT / "data" / "champions" / "regulation_m_b_additions.json"
ROSTER_M_C_ADDITIONS = ROOT / "data" / "champions" / "regulation_m_c_additions.json"

TYPE_BY_CODE = {
    "0": "Normal",
    "1": "Fighting",
    "2": "Flying",
    "3": "Poison",
    "4": "Ground",
    "5": "Rock",
    "6": "Bug",
    "7": "Ghost",
    "8": "Steel",
    "9": "Fire",
    "10": "Water",
    "11": "Grass",
    "12": "Electric",
    "13": "Psychic",
    "14": "Ice",
    "15": "Dragon",
    "16": "Dark",
    "17": "Fairy",
}

CATEGORY_BY_CODE = {
    "0": "Physical",
    "1": "Special",
    "2": "Status",
}


def read_json(path: Path):
    return json.loads(path.read_text())


def regulation_m_b_roster() -> list[str]:
    additions = read_json(ROSTER_M_B_ADDITIONS)
    return read_json(ROSTER_M_A) + additions["regular"] + additions["mega"]


def regulation_m_c_roster() -> list[str]:
    additions = read_json(ROSTER_M_C_ADDITIONS)
    return regulation_m_b_roster() + additions["regular"] + additions["mega"]


def regulation_m_b_additions() -> set[str]:
    additions = read_json(ROSTER_M_B_ADDITIONS)
    return set(additions["regular"]) | set(additions["mega"])


def text_by_label(filename: str) -> dict[str, str]:
    data = read_json(RAW / filename)
    return {
        row["LabelName"]: row.get("OriginalText", "")
        for row in data["mSDataSet"]
    }


def text_by_index(filename: str) -> dict[int, str]:
    data = read_json(RAW / filename)
    return {
        int(row["Index"]): row.get("OriginalText", "")
        for row in data["mSDataSet"]
    }


def int_field(row: dict[str, str], key: str) -> int:
    return int(row[key])


def type_name(code: str) -> str:
    try:
        return TYPE_BY_CODE[code]
    except KeyError as error:
        raise ValueError(f"unknown type code {code}") from error


def category_name(code: str) -> str:
    try:
        return CATEGORY_BY_CODE[code]
    except KeyError as error:
        raise ValueError(f"unknown category code {code}") from error


def normalize_form_display(species: str, form: str) -> str:
    if not form or form == "Normal Form":
        return species
    if form.startswith("Mega "):
        return form
    if form in {"Alola Form", "Alolan Form"}:
        return f"{species} (Alolan)"
    if form == "Galarian Form":
        return f"{species} (Galarian)"
    if form == "Hisuian Form":
        return f"{species} (Hisuian)"
    if form == "Paldean Form":
        return f"{species} (Paldean)"
    if form.startswith("Paldean Form (") and form.endswith(")"):
        return f"{species} (Paldean {form[len('Paldean Form ('):-1]})"
    if form.endswith(" Rotom"):
        return f"Rotom ({form.removesuffix(' Rotom')})"
    if form == species:
        return species
    if form.startswith("Eternal Flower"):
        return f"{species} (Eternal Flower)"
    if form in {"Male", "Female"}:
        return f"{species} ({form})"
    return f"{species} ({form})"


def build_abilities() -> dict[int, dict[str, object]]:
    names = text_by_index("tokusei.json")
    descriptions = text_by_index("tokuseiinfo_syn.json")
    abilities = {}
    for ability_id, name in names.items():
        abilities[ability_id] = {
            "id": ability_id,
            "name": name,
            "description": descriptions.get(ability_id - 1, ""),
        }
    return abilities


def build_moves() -> dict[int, dict[str, object]]:
    names = text_by_label("wazaname.json")
    descriptions = text_by_label("wazainfo_syn.json")
    moves = {}
    for row in read_json(RAW / "waza.json"):
        move_id = int_field(row, "id")
        moves[move_id] = {
            "id": move_id,
            "name": names.get(row["ms_lbl"], row["ms_lbl"]),
            "type": type_name(row["type"]),
            "category": category_name(row["category"]),
            "power": int_field(row, "power"),
            "accuracy": int_field(row, "accuracy"),
            "pp": int_field(row, "pp"),
            "priority": int_field(row, "priority"),
            "makesContact": row["direct"] == "1",
            "available": row["available"] == "1",
            "description": descriptions.get(row["ms_lbl_info"], ""),
            "raw": {
                "target": int_field(row, "target"),
                "classificationA": int_field(row, "classification_a"),
                "classificationB": int_field(row, "classification_b"),
                "textPattern": int_field(row, "text_pattern"),
                "trainingCost": int_field(row, "tcost"),
            },
        }
    return moves


def roster_match(display_name: str, species_name: str, roster: set[str]) -> str | None:
    if display_name in roster:
        return "displayName"
    if species_name in roster:
        return "species"
    return None


def build_species(
    abilities: dict[int, dict[str, object]],
    moves: dict[int, dict[str, object]],
):
    species_names = text_by_label("monsname_syn.json")
    form_names = text_by_label("zkn_form_syn.json")
    learnsets = {
        row["id"]: [int(value) for value in row["waza"].split(",") if value]
        for row in read_json(RAW / "waza_learn.json")
    }
    legal_roster_m_a = set(read_json(ROSTER_M_A))
    legal_roster_m_b = set(regulation_m_b_roster())
    new_m_b_entries = regulation_m_b_additions()
    additions_c = read_json(ROSTER_M_C_ADDITIONS)
    new_m_c_entries = set(additions_c["regular"] + additions_c["mega"])
    legal_roster_m_c = set(regulation_m_c_roster())
    species = []
    for row in read_json(RAW / "personal.json"):
        species_name = species_names.get(row["ms_name_lbl"], row["ms_name_lbl"])
        form_name = form_names.get(row["ms_form_lbl"], "")
        display_name = normalize_form_display(species_name, form_name)
        ability_ids = [
            int_field(row, "toku0"),
            int_field(row, "toku1"),
            int_field(row, "toku2"),
        ]
        move_ids = learnsets.get(row["id"], [])
        regulation_m_a_match = None
        if (
            display_name not in new_m_b_entries | new_m_c_entries
            and species_name not in new_m_c_entries
        ):
            regulation_m_a_match = roster_match(
                display_name,
                species_name,
                legal_roster_m_a,
            )
        regulation_m_b_match = roster_match(
            display_name,
            species_name,
            legal_roster_m_b,
        )
        if display_name in new_m_c_entries or species_name in new_m_c_entries:
            regulation_m_b_match = None
        regulation_m_c_match = roster_match(display_name, species_name, legal_roster_m_c)
        species.append(
            {
                "id": row["id"],
                "nationalDex": int_field(row, "no"),
                "formIndex": int_field(row, "fo"),
                "name": species_name,
                "formName": form_name,
                "displayName": display_name,
                "isRegulationMA": regulation_m_a_match is not None,
                "regulationMatch": regulation_m_a_match,
                "regulationMAMatch": regulation_m_a_match,
                "isRegulationMB": regulation_m_b_match is not None,
                "isRegulationMC": regulation_m_c_match is not None,
                "regulationMCMatch": regulation_m_c_match,
                "regulationMBMatch": regulation_m_b_match,
                "types": [
                    type_name(row["type1"]),
                    type_name(row["type2"]),
                ],
                "baseStats": {
                    "hp": int_field(row, "hp"),
                    "attack": int_field(row, "atk"),
                    "defense": int_field(row, "def"),
                    "specialAttack": int_field(row, "spatk"),
                    "specialDefense": int_field(row, "spdef"),
                    "speed": int_field(row, "agi"),
                },
                "abilities": [
                    {
                        "id": ability_id,
                        "name": abilities.get(ability_id, {}).get("name", ""),
                    }
                    for ability_id in ability_ids
                ],
                "weightKg": int_field(row, "weight") / 10.0,
                "legalMoveIds": move_ids,
                "legalMoves": [
                    {"id": move_id, "name": moves.get(move_id, {}).get("name", "")}
                    for move_id in move_ids
                ],
                "raw": {
                    "isValid": row["is_valid"] == "1",
                    "regNo": int_field(row, "reg_no"),
                    "isSame": row["is_same"] == "1",
                    "pokeClass": int_field(row, "poke_class"),
                    "sex": int_field(row, "sex"),
                    "ff": row["ff"],
                    "rr": row["rr"],
                },
            }
        )
    return species


def main() -> None:
    abilities = build_abilities()
    moves = build_moves()
    species = build_species(abilities, moves)
    payload = {
        "schemaVersion": 1,
        "source": "projectpokemon/champout",
        "ruleset": "Pokemon Champions Regulation M-A/M-B/M-C",
        "counts": {
            "species": len(species),
            "regulationMAForms": sum(1 for entry in species if entry["isRegulationMA"]),
            "regulationMARosterNames": len(read_json(ROSTER_M_A)),
            "regulationMBForms": sum(1 for entry in species if entry["isRegulationMB"]),
            "regulationMBRosterNames": len(regulation_m_b_roster()),
            "regulationMCForms": sum(1 for entry in species if entry["isRegulationMC"]),
            "regulationMCRosterNames": len(regulation_m_c_roster()),
            "moves": len(moves),
            "abilities": len(abilities),
        },
        "species": species,
        "moves": list(moves.values()),
        "abilities": list(abilities.values()),
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps(payload["counts"], indent=2))


if __name__ == "__main__":
    main()
