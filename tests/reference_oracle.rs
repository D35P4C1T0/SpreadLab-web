#![allow(clippy::field_reassign_with_default)]

use std::{
    collections::HashMap,
    env,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use damage_calc::{
    calculate_all_moves, calculate_damage, BatchCalcInput, CalcInput, Category, Field, Move,
    Nature, Pokemon, PokemonType, Ruleset, StatTable,
};

fn pinned_reference() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let reference = env::var_os("NCP_REFERENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("reference/NCP-VGC-Damage-Calculator"));
    reference.exists().then_some(reference)
}

fn run_oracle(input: &serde_json::Value, reference: &PathBuf) -> serde_json::Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut child = Command::new("node")
        .arg(root.join("tools/reference/oracle.mjs"))
        .env("NCP_REFERENCE", reference)
        .env("NCP_ORACLE_COMPACT", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run Node oracle");
    let write_result = child
        .stdin
        .as_mut()
        .expect("oracle stdin")
        .write_all(serde_json::to_string(input).expect("input JSON").as_bytes());
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("oracle output");
    assert!(
        write_result.is_ok(),
        "write oracle input failed: {}; oracle stderr: {}",
        write_result.expect_err("write error"),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "oracle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("oracle JSON output")
}

#[test]
fn pinned_javascript_oracle_matches_rust_smoke_case() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live oracle smoke test: set NCP_REFERENCE to pinned checkout");
        return;
    };

    let input = root.join("tools/reference/smoke-input.json");
    let output = Command::new("node")
        .arg(root.join("tools/reference/oracle.mjs"))
        .env("NCP_REFERENCE", reference)
        .stdin(std::fs::File::open(input).expect("oracle smoke input"))
        .output()
        .expect("run Node oracle");
    assert!(
        output.status.success(),
        "oracle failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let oracle: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("oracle JSON output");
    let expected = oracle["left"][0]["damage"]
        .as_array()
        .expect("left first move damage")
        .iter()
        .map(|value| value.as_u64().expect("integer damage") as u16)
        .collect::<Vec<_>>();

    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let attacker = Pokemon::champions(
        "Attacker",
        [Some(PokemonType::Normal), None],
        stats,
        points,
        Nature::Hardy,
    );
    let defender = Pokemon::champions(
        "Defender",
        [Some(PokemonType::Normal), None],
        stats,
        points,
        Nature::Hardy,
    );
    let mut body_slam = Move::new("Body Slam", 85, PokemonType::Normal, Category::Physical);
    body_slam.makes_contact = true;
    let actual = calculate_damage(CalcInput {
        attacker,
        defender,
        move_: body_slam,
        field: Field::default(),
        ruleset: Ruleset::Champions,
    })
    .expect("Rust calculation")
    .damage_rolls;

    assert_eq!(actual, expected);
}

#[test]
fn all_bundled_set_moves_match_pinned_javascript_first_hit_rolls() {
    use damage_calc::data::champions::CHAMPIONS_REFERENCE_SETS;

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live set corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let inventory: serde_json::Value = serde_json::from_str(include_str!(
        "../data/champions/generated/reference-inventory.json"
    ))
    .expect("reference inventory");
    let species = inventory["pokemon"]
        .as_array()
        .expect("Pokemon inventory")
        .iter()
        .map(|entry| (entry["name"].as_str().expect("Pokemon name"), entry))
        .collect::<HashMap<_, _>>();
    let move_metadata = inventory["moves"]
        .as_array()
        .expect("move inventory")
        .iter()
        .map(|entry| (entry["name"].as_str().expect("move name"), entry))
        .collect::<HashMap<_, _>>();
    let sets = inventory["sets"].as_array().expect("set inventory");
    let cases = sets
        .iter()
        .map(|set| {
            let pokemon = species[set["pokemon"].as_str().expect("set Pokemon")];
            let moves = set["moves"]
                .as_array()
                .expect("set moves")
                .iter()
                .map(|name| {
                    let name = name.as_str().expect("move name");
                    let metadata = move_metadata[name];
                    let hits = metadata
                        .get("hitRange")
                        .map(|range| {
                            range
                                .as_u64()
                                .or_else(|| range[0].as_u64())
                                .unwrap_or(1)
                        })
                        .unwrap_or(1);
                    serde_json::json!({ "name": name, "hits": hits })
                })
                .collect::<Vec<_>>();
            let battle_pokemon = serde_json::json!({
                "name": pokemon["name"],
                "types": [pokemon["t1"].clone(), pokemon.get("t2").cloned().unwrap_or(serde_json::json!(""))],
                "baseStats": {
                    "hp": pokemon["bs"]["hp"],
                    "attack": pokemon["bs"]["at"],
                    "defense": pokemon["bs"]["df"],
                    "specialAttack": pokemon["bs"]["sa"],
                    "specialDefense": pokemon["bs"]["sd"],
                    "speed": pokemon["bs"]["sp"]
                },
                "statPoints": {
                    "hp": set["sps"]["hp"],
                    "attack": set["sps"]["at"],
                    "defense": set["sps"]["df"],
                    "specialAttack": set["sps"]["sa"],
                    "specialDefense": set["sps"]["sd"],
                    "speed": set["sps"]["sp"]
                },
                "nature": set["nature"],
                "ability": set.get("ability").unwrap_or(&pokemon["ab"]),
                "item": set.get("item").cloned().unwrap_or(serde_json::json!("")),
                "weightKg": pokemon["w"],
                "moves": moves
            });
            serde_json::json!({
                "left": battle_pokemon.clone(),
                "right": battle_pokemon,
                "field": { "format": "Doubles" }
            })
        })
        .collect::<Vec<_>>();
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    assert_eq!(oracle_cases.len(), CHAMPIONS_REFERENCE_SETS.len());

    for (case_index, set) in CHAMPIONS_REFERENCE_SETS.iter().enumerate() {
        let pokemon = set.pokemon_().expect("set Pokemon");
        let moves = set.moves_().expect("set moves");
        let batch = calculate_all_moves(BatchCalcInput {
            left: pokemon.clone(),
            right: pokemon,
            left_moves: moves.clone(),
            right_moves: moves,
            left_to_right_field: Field::default(),
            right_to_left_field: Field::default(),
            ruleset: Ruleset::Champions,
        })
        .unwrap_or_else(|error| panic!("{} ({}): {error}", set.pokemon, set.name));

        for (move_index, result) in batch.left.iter().enumerate() {
            let damage = &oracle_cases[case_index]["left"][move_index]["damage"];
            let damage_array = damage.as_array().expect("damage rolls");
            let expected = if damage_array
                .first()
                .is_some_and(serde_json::Value::is_array)
            {
                damage_array[0].as_array().expect("first-hit rolls")
            } else {
                damage_array
            }
            .iter()
            .map(|value| value.as_i64().expect("integer roll").max(0) as u16)
            .collect::<Vec<_>>();
            assert_eq!(
                result.hit_rolls[0], expected,
                "{} ({}) move {}",
                set.pokemon, set.name, set.moves[move_index]
            );
            let expected_ko = oracle_cases[case_index]["left"][move_index]["koChances"]
                .as_array()
                .expect("KO chances");
            for (use_index, expected) in expected_ko.iter().enumerate() {
                let expected = expected.as_f64().expect("KO probability") as f32;
                assert!(
                    (result.ko_chance_by_move_use[use_index] - expected).abs() < 0.000_001,
                    "{} ({}) move {} use {}: Rust {}, JS {}, all Rust {:?}, hits {:?}, percent {:?}",
                    set.pokemon,
                    set.name,
                    set.moves[move_index],
                    use_index + 1,
                    result.ko_chance_by_move_use[use_index],
                    expected,
                    result.ko_chance_by_move_use,
                    result.hit_rolls,
                    result.percent_range
                );
            }
        }
    }
}

#[test]
fn every_active_move_matches_pinned_javascript_neutral_first_hit() {
    use damage_calc::data::champions::CHAMPIONS_REFERENCE_MOVES;

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live move corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let neutral = serde_json::json!({
        "name": "Neutral",
        "types": ["Normal", ""],
        "baseStats": {
            "hp": 100, "attack": 100, "defense": 100,
            "specialAttack": 100, "specialDefense": 100, "speed": 100
        },
        "statPoints": {},
        "nature": "Hardy"
    });
    let cases = CHAMPIONS_REFERENCE_MOVES
        .iter()
        .map(|move_| {
            let mut left = neutral.clone();
            left["moves"] = serde_json::json!([{ "name": move_.name }]);
            serde_json::json!({
                "left": left,
                "right": neutral.clone(),
                "field": { "format": "Doubles" }
            })
        })
        .collect::<Vec<_>>();
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let pokemon = Pokemon::champions(
        "Neutral",
        [Some(PokemonType::Normal), None],
        stats,
        points,
        Nature::Hardy,
    );
    let counter_moves = ["Counter", "Mirror Coat", "Metal Burst", "Comeuppance"];

    for (index, move_) in CHAMPIONS_REFERENCE_MOVES.iter().enumerate() {
        if counter_moves.contains(&move_.name) {
            continue;
        }
        let result = calculate_damage(CalcInput {
            attacker: pokemon.clone(),
            defender: pokemon.clone(),
            move_: move_.move_(),
            field: Field::default(),
            ruleset: Ruleset::Champions,
        })
        .unwrap_or_else(|error| panic!("{}: {error}", move_.name));
        let damage = oracle_cases[index]["left"][0]["damage"]
            .as_array()
            .expect("damage rolls");
        let expected_values = if damage.first().is_some_and(serde_json::Value::is_array) {
            damage[0].as_array().expect("first-hit rolls")
        } else {
            damage
        };
        let expected = expected_values
            .iter()
            .map(|value| value.as_i64().expect("integer roll").max(0) as u16)
            .collect::<Vec<_>>();
        assert_eq!(result.hit_rolls[0], expected, "{}", move_.name);
    }
}

#[test]
fn every_active_ability_matches_pinned_javascript_neutral_contact_case() {
    use damage_calc::data::champions::{
        champions_reference_move, CHAMPIONS_REFERENCE_ABILITY_VALUES,
    };

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live ability corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let neutral = serde_json::json!({
        "name": "Neutral",
        "types": ["Normal", ""],
        "baseStats": {
            "hp": 100, "attack": 100, "defense": 100,
            "specialAttack": 100, "specialDefense": 100, "speed": 100
        },
        "statPoints": {}, "nature": "Hardy",
        "moves": [{ "name": "Body Slam" }]
    });
    let mut cases = Vec::with_capacity(CHAMPIONS_REFERENCE_ABILITY_VALUES.len() * 2);
    for (name, _) in CHAMPIONS_REFERENCE_ABILITY_VALUES {
        let mut attacker = neutral.clone();
        attacker["ability"] = serde_json::json!(name);
        attacker["abilityOn"] = serde_json::json!(false);
        cases.push(serde_json::json!({
            "left": attacker, "right": neutral.clone(), "field": { "format": "Doubles" }
        }));
        let mut defender = neutral.clone();
        defender["ability"] = serde_json::json!(name);
        defender["abilityOn"] = serde_json::json!(false);
        cases.push(serde_json::json!({
            "left": neutral.clone(), "right": defender, "field": { "format": "Doubles" }
        }));
    }
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let neutral_rust = || {
        Pokemon::champions(
            "Neutral",
            [Some(PokemonType::Normal), None],
            stats,
            points,
            Nature::Hardy,
        )
    };
    let body_slam = champions_reference_move("Body Slam")
        .expect("Body Slam metadata")
        .move_();

    for (ability_index, (name, ability)) in CHAMPIONS_REFERENCE_ABILITY_VALUES.iter().enumerate() {
        for defender_role in [false, true] {
            let mut attacker = neutral_rust();
            let mut defender = neutral_rust();
            if defender_role {
                defender.ability = *ability;
            } else {
                attacker.ability = *ability;
            }
            let result = calculate_damage(CalcInput {
                attacker,
                defender,
                move_: body_slam.clone(),
                field: Field::default(),
                ruleset: Ruleset::Champions,
            })
            .unwrap_or_else(|error| panic!("{name}: {error}"));
            let case_index = ability_index * 2 + usize::from(defender_role);
            let damage = oracle_cases[case_index]["left"][0]["damage"]
                .as_array()
                .expect("damage rolls");
            let expected_values = if damage.first().is_some_and(serde_json::Value::is_array) {
                damage[0].as_array().expect("first-hit rolls")
            } else {
                damage
            };
            let expected = expected_values
                .iter()
                .map(|value| value.as_i64().expect("integer roll").max(0) as u16)
                .collect::<Vec<_>>();
            assert_eq!(
                result.hit_rolls[0],
                expected,
                "{name} as {}",
                if defender_role {
                    "defender"
                } else {
                    "attacker"
                }
            );
        }
    }
}

#[test]
fn every_active_item_matches_pinned_javascript_neutral_contact_case() {
    use damage_calc::data::champions::{champions_reference_move, CHAMPIONS_REFERENCE_ITEM_VALUES};

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live item corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let neutral = serde_json::json!({
        "name": "Neutral",
        "types": ["Normal", ""],
        "baseStats": {
            "hp": 100, "attack": 100, "defense": 100,
            "specialAttack": 100, "specialDefense": 100, "speed": 100
        },
        "statPoints": {}, "nature": "Hardy",
        "moves": [{ "name": "Body Slam" }]
    });
    let mut cases = Vec::with_capacity(CHAMPIONS_REFERENCE_ITEM_VALUES.len() * 2);
    for (name, _) in CHAMPIONS_REFERENCE_ITEM_VALUES {
        let mut attacker = neutral.clone();
        attacker["item"] = serde_json::json!(name);
        cases.push(serde_json::json!({
            "left": attacker, "right": neutral.clone(), "field": { "format": "Doubles" }
        }));
        let mut defender = neutral.clone();
        defender["item"] = serde_json::json!(name);
        cases.push(serde_json::json!({
            "left": neutral.clone(), "right": defender, "field": { "format": "Doubles" }
        }));
    }
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let neutral_rust = || {
        Pokemon::champions(
            "Neutral",
            [Some(PokemonType::Normal), None],
            stats,
            points,
            Nature::Hardy,
        )
    };
    let body_slam = champions_reference_move("Body Slam")
        .expect("Body Slam metadata")
        .move_();

    for (item_index, (name, item)) in CHAMPIONS_REFERENCE_ITEM_VALUES.iter().enumerate() {
        for defender_role in [false, true] {
            let mut attacker = neutral_rust();
            let mut defender = neutral_rust();
            if defender_role {
                defender.item = *item;
            } else {
                attacker.item = *item;
            }
            let result = calculate_damage(CalcInput {
                attacker,
                defender,
                move_: body_slam.clone(),
                field: Field::default(),
                ruleset: Ruleset::Champions,
            })
            .unwrap_or_else(|error| panic!("{name}: {error}"));
            let case_index = item_index * 2 + usize::from(defender_role);
            let expected = oracle_cases[case_index]["left"][0]["damage"]
                .as_array()
                .expect("damage rolls")
                .iter()
                .map(|value| value.as_i64().expect("integer roll").max(0) as u16)
                .collect::<Vec<_>>();
            assert_eq!(
                result.hit_rolls[0],
                expected,
                "{name} as {}",
                if defender_role {
                    "defender"
                } else {
                    "attacker"
                }
            );
        }
    }
}

#[test]
fn entry_preprocessing_matches_trace_gas_and_intimidate_exception_lists() {
    use damage_calc::data::champions::{
        champions_reference_move, CHAMPIONS_REFERENCE_ABILITY_VALUES,
    };
    use damage_calc::Ability;

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live entry corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let neutral = serde_json::json!({
        "name": "Neutral",
        "types": ["Normal", ""],
        "baseStats": {
            "hp": 100, "attack": 100, "defense": 100,
            "specialAttack": 100, "specialDefense": 100, "speed": 100
        },
        "statPoints": {}, "nature": "Hardy",
        "moves": [{ "name": "Body Slam" }]
    });
    let mut cases = Vec::with_capacity(CHAMPIONS_REFERENCE_ABILITY_VALUES.len() * 3);
    for (name, _) in CHAMPIONS_REFERENCE_ABILITY_VALUES {
        let mut tracer = neutral.clone();
        tracer["ability"] = serde_json::json!("Trace");
        tracer["abilityOn"] = serde_json::json!(true);
        let mut target = neutral.clone();
        target["ability"] = serde_json::json!(name);
        cases.push(serde_json::json!({
            "left": tracer, "right": target.clone(), "field": { "format": "Doubles" }
        }));
        cases.push(serde_json::json!({
            "left": target.clone(), "right": neutral.clone(),
            "field": { "format": "Doubles", "neutralizingGas": true }
        }));
        let mut intimidator = neutral.clone();
        intimidator["ability"] = serde_json::json!("Intimidate");
        intimidator["abilityOn"] = serde_json::json!(true);
        cases.push(serde_json::json!({
            "left": intimidator, "right": target, "field": { "format": "Doubles" }
        }));
    }
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    let ability_by_name = CHAMPIONS_REFERENCE_ABILITY_VALUES
        .iter()
        .copied()
        .collect::<HashMap<_, _>>();
    let parse_ability = |name: &str| {
        if name.is_empty() {
            Ability::None
        } else {
            *ability_by_name
                .get(name)
                .unwrap_or_else(|| panic!("unmapped ability {name}"))
        }
    };
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let neutral_rust = || {
        Pokemon::champions(
            "Neutral",
            [Some(PokemonType::Normal), None],
            stats,
            points,
            Nature::Hardy,
        )
    };
    let body_slam = champions_reference_move("Body Slam")
        .expect("Body Slam metadata")
        .move_();
    let no_move = champions_reference_move("(No Move)")
        .expect("No Move metadata")
        .move_();
    let moves = || {
        [
            body_slam.clone(),
            no_move.clone(),
            no_move.clone(),
            no_move.clone(),
        ]
    };

    for (ability_index, (name, ability)) in CHAMPIONS_REFERENCE_ABILITY_VALUES.iter().enumerate() {
        let base = ability_index * 3;

        let mut tracer = neutral_rust();
        tracer.ability = Ability::Trace;
        tracer.ability_on = true;
        let mut target = neutral_rust();
        target.ability = *ability;
        let trace = calculate_all_moves(BatchCalcInput {
            left: tracer,
            right: target.clone(),
            left_moves: moves(),
            right_moves: moves(),
            left_to_right_field: Field::default(),
            right_to_left_field: Field::default(),
            ruleset: Ruleset::Champions,
        })
        .expect("Trace batch");
        let expected_trace = oracle_cases[base]["state"]["left"]["ability"]
            .as_str()
            .expect("Trace ability");
        assert_eq!(
            trace.left_state.ability,
            parse_ability(expected_trace),
            "Trace vs {name}"
        );

        let mut gas_field = Field::default();
        gas_field.neutralizing_gas = true;
        let gas = calculate_all_moves(BatchCalcInput {
            left: target.clone(),
            right: neutral_rust(),
            left_moves: moves(),
            right_moves: moves(),
            left_to_right_field: gas_field,
            right_to_left_field: gas_field,
            ruleset: Ruleset::Champions,
        })
        .expect("gas batch");
        let expected_gas = oracle_cases[base + 1]["state"]["left"]["ability"]
            .as_str()
            .expect("gas ability");
        assert_eq!(
            gas.left_state.ability,
            parse_ability(expected_gas),
            "gas vs {name}"
        );

        let mut intimidator = neutral_rust();
        intimidator.ability = Ability::Intimidate;
        intimidator.ability_on = true;
        let intimidated = calculate_all_moves(BatchCalcInput {
            left: intimidator,
            right: target,
            left_moves: moves(),
            right_moves: moves(),
            left_to_right_field: Field::default(),
            right_to_left_field: Field::default(),
            ruleset: Ruleset::Champions,
        })
        .expect("Intimidate batch");
        let boosts = &oracle_cases[base + 2]["state"]["right"]["boosts"];
        assert_eq!(
            intimidated.right_state.boosts.attack,
            boosts["at"].as_i64().expect("attack boost") as i8,
            "Intimidate attack vs {name}"
        );
        assert_eq!(
            intimidated.right_state.boosts.special_attack,
            boosts["sa"].as_i64().expect("special attack boost") as i8,
            "Intimidate special attack vs {name}"
        );
        assert_eq!(
            intimidated.right_state.boosts.speed,
            boosts["sp"].as_i64().expect("speed boost") as i8,
            "Intimidate speed vs {name}"
        );
    }
}

#[test]
fn pairwise_ability_item_move_and_field_partitions_match_pinned_javascript() {
    use damage_calc::data::champions::{
        champions_reference_move, CHAMPIONS_REFERENCE_ABILITY_VALUES,
        CHAMPIONS_REFERENCE_ITEM_VALUES,
    };
    use damage_calc::{Format, Terrain, Weather};

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live pairwise corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let move_names = [
        "Body Slam",
        "Flamethrower",
        "Water Pulse",
        "Crunch",
        "Bullet Punch",
        "Hyper Voice",
        "Air Cutter",
        "Earthquake",
    ];
    let neutral_json = serde_json::json!({
        "name": "Neutral", "types": ["Normal", ""],
        "baseStats": {
            "hp": 100, "attack": 100, "defense": 100,
            "specialAttack": 100, "specialDefense": 100, "speed": 100
        },
        "statPoints": {}, "nature": "Hardy"
    });
    let mut cases = Vec::with_capacity(CHAMPIONS_REFERENCE_ABILITY_VALUES.len());
    for index in 0..CHAMPIONS_REFERENCE_ABILITY_VALUES.len() {
        let (left_ability, _) = CHAMPIONS_REFERENCE_ABILITY_VALUES[index];
        let (right_ability, _) = CHAMPIONS_REFERENCE_ABILITY_VALUES
            [(index * 37 + 11) % CHAMPIONS_REFERENCE_ABILITY_VALUES.len()];
        let (left_item, _) =
            CHAMPIONS_REFERENCE_ITEM_VALUES[index % CHAMPIONS_REFERENCE_ITEM_VALUES.len()];
        let (right_item, _) = CHAMPIONS_REFERENCE_ITEM_VALUES
            [(index * 53 + 7) % CHAMPIONS_REFERENCE_ITEM_VALUES.len()];
        let move_name = move_names[index % move_names.len()];
        let mut left = neutral_json.clone();
        left["ability"] = serde_json::json!(left_ability);
        left["item"] = serde_json::json!(left_item);
        left["moves"] = serde_json::json!([{ "name": move_name }]);
        let mut right = neutral_json.clone();
        right["ability"] = serde_json::json!(right_ability);
        right["item"] = serde_json::json!(right_item);
        let field = match index % 12 {
            1 => serde_json::json!({ "format": "Doubles", "weather": "Sun" }),
            2 => serde_json::json!({ "format": "Doubles", "weather": "Rain" }),
            3 => serde_json::json!({ "format": "Doubles", "weather": "Sand" }),
            4 => serde_json::json!({ "format": "Doubles", "terrain": "Electric" }),
            5 => serde_json::json!({ "format": "Doubles", "terrain": "Grassy" }),
            6 => serde_json::json!({ "format": "Doubles", "right": { "reflect": true } }),
            7 => serde_json::json!({ "format": "Doubles", "right": { "lightScreen": true } }),
            8 => serde_json::json!({ "format": "Doubles", "right": { "helpingHand": true } }),
            9 => serde_json::json!({ "format": "Doubles", "right": { "friendGuard": true } }),
            10 => serde_json::json!({ "format": "Doubles", "right": { "protect": true } }),
            11 => serde_json::json!({ "format": "Singles", "left": { "tailwind": true } }),
            _ => serde_json::json!({ "format": "Doubles" }),
        };
        cases.push(serde_json::json!({ "left": left, "right": right, "field": field }));
    }
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let neutral_rust = || {
        Pokemon::champions(
            "Neutral",
            [Some(PokemonType::Normal), None],
            stats,
            points,
            Nature::Hardy,
        )
    };

    for index in 0..CHAMPIONS_REFERENCE_ABILITY_VALUES.len() {
        let (left_ability_name, left_ability) = CHAMPIONS_REFERENCE_ABILITY_VALUES[index];
        let (right_ability_name, right_ability) = CHAMPIONS_REFERENCE_ABILITY_VALUES
            [(index * 37 + 11) % CHAMPIONS_REFERENCE_ABILITY_VALUES.len()];
        let (left_item_name, left_item) =
            CHAMPIONS_REFERENCE_ITEM_VALUES[index % CHAMPIONS_REFERENCE_ITEM_VALUES.len()];
        let (right_item_name, right_item) = CHAMPIONS_REFERENCE_ITEM_VALUES
            [(index * 53 + 7) % CHAMPIONS_REFERENCE_ITEM_VALUES.len()];
        let move_name = move_names[index % move_names.len()];
        let mut attacker = neutral_rust();
        attacker.ability = left_ability;
        attacker.ability_on = left_ability.is_on_by_default();
        attacker.item = left_item;
        let mut defender = neutral_rust();
        defender.ability = right_ability;
        defender.ability_on = right_ability.is_on_by_default();
        defender.item = right_item;
        let mut field = Field::default();
        match index % 12 {
            1 => field.weather = Weather::Sun,
            2 => field.weather = Weather::Rain,
            3 => field.weather = Weather::Sand,
            4 => field.terrain = Terrain::Electric,
            5 => field.terrain = Terrain::Grassy,
            6 => field.defender_side.reflect = true,
            7 => field.defender_side.light_screen = true,
            8 => field.helping_hand = true,
            9 => field.defender_side.friend_guard = true,
            10 => field.protect = true,
            11 => {
                field.format = Format::Singles;
                field.attacker_tailwind = true;
            }
            _ => {}
        }
        let result = calculate_damage(CalcInput {
            attacker,
            defender,
            move_: champions_reference_move(move_name)
                .expect("move metadata")
                .move_(),
            field,
            ruleset: Ruleset::Champions,
        })
        .unwrap_or_else(|error| panic!("case {index}: {error}"));
        let damage = oracle_cases[index]["left"][0]["damage"]
            .as_array()
            .expect("damage rolls");
        let expected_values = if damage.first().is_some_and(serde_json::Value::is_array) {
            damage[0].as_array().expect("first-hit rolls")
        } else {
            damage
        };
        let expected = expected_values
            .iter()
            .map(|value| value.as_i64().expect("integer roll").max(0) as u16)
            .collect::<Vec<_>>();
        assert_eq!(
            result.hit_rolls[0], expected,
            "case {index}: {left_ability_name}/{left_item_name} {move_name} vs {right_ability_name}/{right_item_name}"
        );
    }
}

#[test]
fn every_active_move_matches_across_field_partitions() {
    use damage_calc::data::champions::CHAMPIONS_REFERENCE_MOVES;
    use damage_calc::{Format, Terrain, Weather};

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live field corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let neutral = serde_json::json!({
        "name": "Neutral", "types": ["Normal", ""],
        "baseStats": {
            "hp": 100, "attack": 100, "defense": 100,
            "specialAttack": 100, "specialDefense": 100, "speed": 100
        },
        "statPoints": {}, "nature": "Hardy"
    });
    let field_count = 17usize;
    let field_json = |index| match index {
        1 => serde_json::json!({ "format": "Singles" }),
        2 => serde_json::json!({ "format": "Doubles", "weather": "Sun" }),
        3 => serde_json::json!({ "format": "Doubles", "weather": "Rain" }),
        4 => serde_json::json!({ "format": "Doubles", "weather": "Sand" }),
        5 => serde_json::json!({ "format": "Doubles", "weather": "Snow" }),
        6 => serde_json::json!({ "format": "Doubles", "terrain": "Electric" }),
        7 => serde_json::json!({ "format": "Doubles", "terrain": "Grassy" }),
        8 => serde_json::json!({ "format": "Doubles", "terrain": "Psychic" }),
        9 => serde_json::json!({ "format": "Doubles", "terrain": "Misty" }),
        10 => serde_json::json!({ "format": "Doubles", "right": { "reflect": true } }),
        11 => serde_json::json!({ "format": "Doubles", "right": { "lightScreen": true } }),
        12 => serde_json::json!({ "format": "Doubles", "right": { "auroraVeil": true } }),
        13 => serde_json::json!({ "format": "Doubles", "right": { "helpingHand": true } }),
        14 => serde_json::json!({ "format": "Doubles", "right": { "friendGuard": true } }),
        15 => serde_json::json!({ "format": "Doubles", "gravity": true }),
        16 => serde_json::json!({ "format": "Doubles", "right": { "charge": true } }),
        _ => serde_json::json!({ "format": "Doubles" }),
    };
    let mut cases = Vec::with_capacity(CHAMPIONS_REFERENCE_MOVES.len() * field_count);
    for move_ in CHAMPIONS_REFERENCE_MOVES {
        for field_index in 0..field_count {
            let mut left = neutral.clone();
            left["moves"] = serde_json::json!([{ "name": move_.name }]);
            cases.push(serde_json::json!({
                "left": left, "right": neutral.clone(), "field": field_json(field_index)
            }));
        }
    }
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let pokemon = Pokemon::champions(
        "Neutral",
        [Some(PokemonType::Normal), None],
        stats,
        points,
        Nature::Hardy,
    );
    let counter_moves = ["Counter", "Mirror Coat", "Metal Burst", "Comeuppance"];

    for (move_index, move_) in CHAMPIONS_REFERENCE_MOVES.iter().enumerate() {
        if counter_moves.contains(&move_.name) {
            continue;
        }
        for field_index in 0..field_count {
            let mut field = Field::default();
            match field_index {
                1 => field.format = Format::Singles,
                2 => field.weather = Weather::Sun,
                3 => field.weather = Weather::Rain,
                4 => field.weather = Weather::Sand,
                5 => field.weather = Weather::Snow,
                6 => field.terrain = Terrain::Electric,
                7 => field.terrain = Terrain::Grassy,
                8 => field.terrain = Terrain::Psychic,
                9 => field.terrain = Terrain::Misty,
                10 => field.defender_side.reflect = true,
                11 => field.defender_side.light_screen = true,
                12 => field.defender_side.aurora_veil = true,
                13 => field.helping_hand = true,
                14 => field.defender_side.friend_guard = true,
                15 => field.gravity = true,
                16 => field.charge = true,
                _ => {}
            }
            let result = calculate_damage(CalcInput {
                attacker: pokemon.clone(),
                defender: pokemon.clone(),
                move_: move_.move_(),
                field,
                ruleset: Ruleset::Champions,
            })
            .unwrap_or_else(|error| panic!("{} field {field_index}: {error}", move_.name));
            let case_index = move_index * field_count + field_index;
            let damage = oracle_cases[case_index]["left"][0]["damage"]
                .as_array()
                .expect("damage rolls");
            let expected_values = if damage.first().is_some_and(serde_json::Value::is_array) {
                damage[0].as_array().expect("first-hit rolls")
            } else {
                damage
            };
            let expected = expected_values
                .iter()
                .map(|value| value.as_i64().expect("integer roll").max(0) as u16)
                .collect::<Vec<_>>();
            assert_eq!(
                result.hit_rolls[0], expected,
                "{} field {field_index}",
                move_.name
            );
        }
    }
}

#[test]
fn every_active_move_matches_across_state_boundaries() {
    use damage_calc::data::champions::CHAMPIONS_REFERENCE_MOVES;
    use damage_calc::{StatTable, StatusCondition};

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live state corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let neutral = serde_json::json!({
        "name": "Neutral", "types": ["Normal", ""],
        "baseStats": {
            "hp": 100, "attack": 100, "defense": 100,
            "specialAttack": 100, "specialDefense": 100, "speed": 100
        },
        "statPoints": {}, "nature": "Hardy"
    });
    let state_count = 15usize;
    let mut cases = Vec::with_capacity(CHAMPIONS_REFERENCE_MOVES.len() * state_count);
    for move_ in CHAMPIONS_REFERENCE_MOVES {
        for state_index in 0..state_count {
            let mut left = neutral.clone();
            let mut right = neutral.clone();
            let mut move_json = serde_json::json!({ "name": move_.name });
            match state_index {
                0 => left["status"] = serde_json::json!("Burned"),
                1 => left["status"] = serde_json::json!("Paralyzed"),
                2 => right["status"] = serde_json::json!("Poisoned"),
                3 => right["status"] = serde_json::json!("Asleep"),
                4 => left["currentHp"] = serde_json::json!(50),
                5 => right["currentHp"] = serde_json::json!(50),
                6 => {
                    left["boosts"] = serde_json::json!({
                        "attack": 2, "defense": 2, "specialAttack": 2,
                        "specialDefense": 2, "speed": 2
                    })
                }
                7 => {
                    right["boosts"] = serde_json::json!({
                        "attack": 2, "defense": 2, "specialAttack": 2,
                        "specialDefense": 2, "speed": 2
                    })
                }
                8 => {
                    left["boosts"] = serde_json::json!({
                        "attack": -2, "specialAttack": -2, "speed": -2
                    })
                }
                9 => {
                    right["boosts"] = serde_json::json!({
                        "defense": -2, "specialDefense": -2, "speed": -2
                    })
                }
                10 => {
                    left["isTerastalized"] = serde_json::json!(true);
                    left["teraType"] = serde_json::json!("Normal");
                }
                11 => {
                    right["isTerastalized"] = serde_json::json!(true);
                    right["teraType"] = serde_json::json!("Ghost");
                }
                12 => {
                    left["weightKg"] = serde_json::json!(500.0);
                    right["weightKg"] = serde_json::json!(10.0);
                }
                13 => {
                    left["weightKg"] = serde_json::json!(10.0);
                    right["weightKg"] = serde_json::json!(500.0);
                }
                14 if move_.can_double => move_json["isDoublePower"] = serde_json::json!(true),
                14 => {}
                _ => unreachable!(),
            }
            left["moves"] = serde_json::json!([move_json]);
            cases.push(serde_json::json!({
                "left": left, "right": right, "field": { "format": "Doubles" }
            }));
        }
    }
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let neutral_rust = || {
        Pokemon::champions(
            "Neutral",
            [Some(PokemonType::Normal), None],
            stats,
            points,
            Nature::Hardy,
        )
    };
    let counter_moves = ["Counter", "Mirror Coat", "Metal Burst", "Comeuppance"];

    for (move_index, reference_move) in CHAMPIONS_REFERENCE_MOVES.iter().enumerate() {
        if counter_moves.contains(&reference_move.name) {
            continue;
        }
        for state_index in 0..state_count {
            let mut attacker = neutral_rust();
            let mut defender = neutral_rust();
            let mut move_ = reference_move.move_();
            match state_index {
                0 => attacker.status = StatusCondition::Burned,
                1 => attacker.status = StatusCondition::Paralyzed,
                2 => defender.status = StatusCondition::Poisoned,
                3 => defender.status = StatusCondition::Asleep,
                4 => attacker.current_hp = Some(50),
                5 => defender.current_hp = Some(50),
                6 => {
                    attacker.boosts = damage_calc::Boosts {
                        attack: 2,
                        defense: 2,
                        special_attack: 2,
                        special_defense: 2,
                        speed: 2,
                    }
                }
                7 => {
                    defender.boosts = damage_calc::Boosts {
                        attack: 2,
                        defense: 2,
                        special_attack: 2,
                        special_defense: 2,
                        speed: 2,
                    }
                }
                8 => {
                    attacker.boosts.attack = -2;
                    attacker.boosts.special_attack = -2;
                    attacker.boosts.speed = -2;
                }
                9 => {
                    defender.boosts.defense = -2;
                    defender.boosts.special_defense = -2;
                    defender.boosts.speed = -2;
                }
                10 => {
                    attacker.is_terastalized = true;
                    attacker.tera_type = Some(PokemonType::Normal);
                }
                11 => {
                    defender.is_terastalized = true;
                    defender.tera_type = Some(PokemonType::Ghost);
                }
                12 => {
                    attacker.weight_kg = 500.0;
                    defender.weight_kg = 10.0;
                }
                13 => {
                    attacker.weight_kg = 10.0;
                    defender.weight_kg = 500.0;
                }
                14 => move_.is_double_power = reference_move.can_double,
                _ => unreachable!(),
            }
            let result = calculate_damage(CalcInput {
                attacker,
                defender,
                move_,
                field: Field::default(),
                ruleset: Ruleset::Champions,
            })
            .unwrap_or_else(|error| panic!("{} state {state_index}: {error}", reference_move.name));
            let case_index = move_index * state_count + state_index;
            let damage = oracle_cases[case_index]["left"][0]["damage"]
                .as_array()
                .expect("damage rolls");
            let expected_values = if damage.first().is_some_and(serde_json::Value::is_array) {
                damage[0].as_array().expect("first-hit rolls")
            } else {
                damage
            };
            let expected = expected_values
                .iter()
                .map(|value| value.as_i64().expect("integer roll").max(0) as u16)
                .collect::<Vec<_>>();
            assert_eq!(
                result.hit_rolls[0], expected,
                "{} state {state_index}",
                reference_move.name
            );
        }
    }
}

#[test]
fn multi_hit_recalculation_matches_pinned_javascript_per_hit() {
    use damage_calc::data::champions::champions_reference_move;
    use damage_calc::{Ability, Format, Item};

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live multi-hit corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let json_pokemon =
        |name: &str, type_: &str, ability: &str, item: &str, move_name: &str, hits: u8| {
            serde_json::json!({
                "name": name, "types": [type_, ""],
                "baseStats": {
                    "hp": 100, "attack": 100, "defense": 100,
                    "specialAttack": 100, "specialDefense": 100, "speed": 100
                },
                "statPoints": {}, "nature": "Hardy", "ability": ability, "item": item,
                "moves": [{ "name": move_name, "hits": hits }]
            })
        };
    let json_defender = |name: &str, type_: &str, ability: &str, item: &str| {
        json_pokemon(name, type_, ability, item, "(No Move)", 1)
    };
    let mut oracle_inputs = Vec::new();
    let mut rust_inputs = Vec::new();
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let rust_pokemon = |name: &str, type_: PokemonType, ability: Ability, item: Item| {
        let mut pokemon =
            Pokemon::champions(name, [Some(type_), None], stats, points, Nature::Hardy);
        pokemon.ability = ability;
        pokemon.ability_on = ability.is_on_by_default();
        pokemon.item = item;
        pokemon
    };
    let mut add_case = |move_name: &str,
                        hits: u8,
                        attacker_type: PokemonType,
                        attacker_ability: Ability,
                        attacker_ability_name: &str,
                        defender_type: PokemonType,
                        defender_type_name: &str,
                        defender_ability: Ability,
                        defender_ability_name: &str,
                        defender_item: Item,
                        defender_item_name: &str,
                        field: Field,
                        format_name: &str| {
        let mut move_ = champions_reference_move(move_name)
            .unwrap_or_else(|| panic!("missing {move_name}"))
            .move_();
        move_.hits = hits;
        let attacker_type_name = match attacker_type {
            PokemonType::Grass => "Grass",
            PokemonType::Normal => "Normal",
            PokemonType::Fire => "Fire",
            PokemonType::Ice => "Ice",
            _ => panic!("unmapped attacker type"),
        };
        oracle_inputs.push(serde_json::json!({
            "left": json_pokemon("Attacker", attacker_type_name, attacker_ability_name, "", move_name, hits),
            "right": json_defender("Defender", defender_type_name, defender_ability_name, defender_item_name),
            "field": { "format": format_name }
        }));
        rust_inputs.push(CalcInput {
            attacker: rust_pokemon("Attacker", attacker_type, attacker_ability, Item::None),
            defender: rust_pokemon("Defender", defender_type, defender_ability, defender_item),
            move_,
            field,
            ruleset: Ruleset::Champions,
        });
    };

    add_case(
        "Bullet Seed",
        3,
        PokemonType::Grass,
        Ability::None,
        "",
        PokemonType::Water,
        "Water",
        Ability::Stamina,
        "Stamina",
        Item::None,
        "",
        Field::default(),
        "Doubles",
    );
    add_case(
        "Bullet Seed",
        3,
        PokemonType::Grass,
        Ability::None,
        "",
        PokemonType::Water,
        "Water",
        Ability::WeakArmor,
        "Weak Armor",
        Item::None,
        "",
        Field::default(),
        "Doubles",
    );
    add_case(
        "Bullet Seed",
        3,
        PokemonType::Grass,
        Ability::None,
        "",
        PokemonType::Water,
        "Water",
        Ability::Multiscale,
        "Multiscale",
        Item::None,
        "",
        Field::default(),
        "Doubles",
    );
    add_case(
        "Bullet Seed",
        3,
        PokemonType::Grass,
        Ability::None,
        "",
        PokemonType::Water,
        "Water",
        Ability::None,
        "",
        Item::KeeBerry,
        "Kee Berry",
        Field::default(),
        "Doubles",
    );
    add_case(
        "Double Hit",
        2,
        PokemonType::Normal,
        Ability::Defiant,
        "Defiant",
        PokemonType::Normal,
        "Normal",
        Ability::Gooey,
        "Gooey",
        Item::None,
        "",
        Field::default(),
        "Doubles",
    );
    add_case(
        "Double Hit",
        2,
        PokemonType::Normal,
        Ability::None,
        "",
        PokemonType::Normal,
        "Normal",
        Ability::SpicySpray,
        "Spicy Spray",
        Item::None,
        "",
        Field::default(),
        "Doubles",
    );
    let mut singles = Field::default();
    singles.format = Format::Singles;
    add_case(
        "Body Slam",
        1,
        PokemonType::Normal,
        Ability::ParentalBond,
        "Parental Bond",
        PokemonType::Normal,
        "Normal",
        Ability::None,
        "",
        Item::None,
        "",
        singles,
        "Singles",
    );
    add_case(
        "Triple Axel",
        3,
        PokemonType::Ice,
        Ability::None,
        "",
        PokemonType::Normal,
        "Normal",
        Ability::None,
        "",
        Item::None,
        "",
        Field::default(),
        "Doubles",
    );
    add_case(
        "Weather Ball",
        2,
        PokemonType::Fire,
        Ability::None,
        "",
        PokemonType::Normal,
        "Normal",
        Ability::SandSpit,
        "Sand Spit",
        Item::None,
        "",
        Field::default(),
        "Doubles",
    );

    let oracle = run_oracle(&serde_json::Value::Array(oracle_inputs), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    for (index, input) in rust_inputs.into_iter().enumerate() {
        let result =
            calculate_damage(input).unwrap_or_else(|error| panic!("case {index}: {error}"));
        let oracle_damage = oracle_cases[index]["left"][0]["damage"]
            .as_array()
            .expect("damage rolls");
        let mut expected_hits = if oracle_damage
            .first()
            .is_some_and(serde_json::Value::is_array)
        {
            oracle_damage
                .iter()
                .map(|hit| {
                    hit.as_array()
                        .expect("hit rolls")
                        .iter()
                        .map(|value| value.as_i64().expect("roll").max(0) as u16)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        } else {
            let hit = oracle_damage
                .iter()
                .map(|value| value.as_i64().expect("roll").max(0) as u16)
                .collect::<Vec<_>>();
            vec![hit; result.hit_rolls.len()]
        };
        if let Some(last) = expected_hits.last().cloned() {
            expected_hits.resize(result.hit_rolls.len(), last);
        }
        assert_eq!(result.hit_rolls, expected_hits, "multi-hit case {index}");
    }
}

#[test]
fn ko_probabilities_match_hazards_healing_status_and_residual_order() {
    use damage_calc::data::champions::champions_reference_move;
    use damage_calc::{Ability, Item, StatusCondition, Terrain, Weather};

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live KO corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let mut oracle_inputs = Vec::new();
    let mut rust_inputs = Vec::new();
    let mut labels = Vec::new();
    let mut add_case = |label: &str,
                        item: Item,
                        item_name: &str,
                        ability: Ability,
                        ability_name: &str,
                        status: StatusCondition,
                        status_name: &str,
                        toxic_counter: u8,
                        field: Field,
                        field_json: serde_json::Value,
                        move_name: &str,
                        base_power: Option<u16>| {
        let mut attacker = Pokemon::champions(
            "Attacker",
            [Some(PokemonType::Normal), None],
            stats,
            points,
            Nature::Hardy,
        );
        attacker.level = 50;
        let mut defender = Pokemon::champions(
            "Defender",
            [Some(PokemonType::Normal), None],
            stats,
            points,
            Nature::Hardy,
        );
        defender.max_hp_override = Some(100);
        defender.current_hp = Some(100);
        defender.item = item;
        defender.ability = ability;
        defender.ability_on = ability.is_on_by_default();
        defender.status = status;
        defender.toxic_counter = toxic_counter;
        let mut move_ = champions_reference_move(move_name)
            .unwrap_or_else(|| panic!("missing {move_name}"))
            .move_();
        if let Some(base_power) = base_power {
            move_.base_power = base_power;
        }
        let move_json = if let Some(base_power) = base_power {
            serde_json::json!({ "name": move_name, "basePower": base_power })
        } else {
            serde_json::json!({ "name": move_name })
        };
        let left = serde_json::json!({
            "name": "Attacker", "types": ["Normal", ""],
            "baseStats": { "hp": 100, "attack": 100, "defense": 100, "specialAttack": 100, "specialDefense": 100, "speed": 100 },
            "statPoints": {}, "nature": "Hardy", "moves": [move_json]
        });
        let right = serde_json::json!({
            "name": "Defender", "types": ["Normal", ""],
            "baseStats": { "hp": 100, "attack": 100, "defense": 100, "specialAttack": 100, "specialDefense": 100, "speed": 100 },
            "statPoints": {}, "nature": "Hardy", "maxHp": 100, "currentHp": 100,
            "item": item_name, "ability": ability_name, "status": status_name,
            "toxicCounter": toxic_counter, "moves": []
        });
        oracle_inputs
            .push(serde_json::json!({ "left": left, "right": right, "field": field_json }));
        rust_inputs.push(CalcInput {
            attacker,
            defender,
            move_,
            field,
            ruleset: Ruleset::Champions,
        });
        labels.push(label.to_owned());
    };

    add_case(
        "neutral",
        Item::None,
        "",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Seismic Toss",
        None,
    );
    add_case(
        "Sitrus",
        Item::SitrusBerry,
        "Sitrus Berry",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Seismic Toss",
        None,
    );
    add_case(
        "Oran",
        Item::OranBerry,
        "Oran Berry",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Seismic Toss",
        None,
    );
    add_case(
        "Leftovers",
        Item::Leftovers,
        "Leftovers",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Seismic Toss",
        None,
    );
    add_case(
        "burn",
        Item::None,
        "",
        Ability::None,
        "",
        StatusCondition::Burned,
        "Burned",
        1,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Seismic Toss",
        None,
    );
    add_case(
        "poison",
        Item::None,
        "",
        Ability::None,
        "",
        StatusCondition::Poisoned,
        "Poisoned",
        1,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Seismic Toss",
        None,
    );
    add_case(
        "toxic",
        Item::None,
        "",
        Ability::None,
        "",
        StatusCondition::BadlyPoisoned,
        "Badly Poisoned",
        3,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Seismic Toss",
        None,
    );
    let mut rain = Field::default();
    rain.weather = Weather::Rain;
    add_case(
        "rain Dry Skin",
        Item::None,
        "",
        Ability::DrySkin,
        "Dry Skin",
        StatusCondition::Healthy,
        "Healthy",
        1,
        rain,
        serde_json::json!({ "format": "Doubles", "weather": "Rain" }),
        "Seismic Toss",
        None,
    );
    let mut sun = Field::default();
    sun.weather = Weather::Sun;
    add_case(
        "sun Dry Skin",
        Item::None,
        "",
        Ability::DrySkin,
        "Dry Skin",
        StatusCondition::Healthy,
        "Healthy",
        1,
        sun,
        serde_json::json!({ "format": "Doubles", "weather": "Sun" }),
        "Seismic Toss",
        None,
    );
    let mut grassy = Field::default();
    grassy.terrain = Terrain::Grassy;
    add_case(
        "Grassy Terrain",
        Item::None,
        "",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        grassy,
        serde_json::json!({ "format": "Doubles", "terrain": "Grassy" }),
        "Seismic Toss",
        None,
    );
    let mut rocks = Field::default();
    rocks.defender_side.stealth_rock = true;
    add_case(
        "Stealth Rock",
        Item::None,
        "",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        rocks,
        serde_json::json!({ "format": "Doubles", "right": { "stealthRock": true } }),
        "Seismic Toss",
        None,
    );
    for layers in 1..=3 {
        let mut spikes = Field::default();
        spikes.defender_side.spikes = layers;
        add_case(
            &format!("Spikes {layers}"),
            Item::None,
            "",
            Ability::None,
            "",
            StatusCondition::Healthy,
            "Healthy",
            1,
            spikes,
            serde_json::json!({ "format": "Doubles", "right": { "spikes": layers } }),
            "Seismic Toss",
            None,
        );
    }
    let mut salt = Field::default();
    salt.defender_side.salt_cure = true;
    add_case(
        "Salt Cure",
        Item::None,
        "",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        salt,
        serde_json::json!({ "format": "Doubles", "right": { "saltCure": true } }),
        "Seismic Toss",
        None,
    );
    let mut guarded = Field::default();
    guarded.defender_side.stealth_rock = true;
    guarded.defender_side.spikes = 3;
    add_case(
        "Magic Guard",
        Item::None,
        "",
        Ability::MagicGuard,
        "Magic Guard",
        StatusCondition::Burned,
        "Burned",
        1,
        guarded,
        serde_json::json!({ "format": "Doubles", "right": { "stealthRock": true, "spikes": 3 } }),
        "Seismic Toss",
        None,
    );
    add_case(
        "Psychic Noise",
        Item::Leftovers,
        "Leftovers",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Psychic Noise",
        Some(130),
    );
    add_case(
        "Knock Off Sitrus",
        Item::SitrusBerry,
        "Sitrus Berry",
        Ability::None,
        "",
        StatusCondition::Healthy,
        "Healthy",
        1,
        Field::default(),
        serde_json::json!({ "format": "Doubles" }),
        "Knock Off",
        None,
    );

    let oracle = run_oracle(&serde_json::Value::Array(oracle_inputs), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    for (index, input) in rust_inputs.into_iter().enumerate() {
        let result =
            calculate_damage(input).unwrap_or_else(|error| panic!("{}: {error}", labels[index]));
        let expected = oracle_cases[index]["left"][0]["koChances"]
            .as_array()
            .expect("KO chances");
        for use_index in 0..4 {
            let expected = expected[use_index].as_f64().expect("KO probability") as f32;
            assert!(
                (result.ko_chance_by_move_use[use_index] - expected).abs() < 0.000_001,
                "{} use {}: Rust {}, JS {}",
                labels[index],
                use_index + 1,
                result.ko_chance_by_move_use[use_index],
                expected
            );
        }
    }
}

#[test]
fn batch_counter_moves_infer_opposing_move_like_pinned_javascript() {
    use damage_calc::data::champions::champions_reference_move;

    let Some(reference) = pinned_reference() else {
        eprintln!("skipping live counter corpus: set NCP_REFERENCE to pinned checkout");
        return;
    };
    let pairs = [
        ("Counter", "Body Slam"),
        ("Mirror Coat", "Flamethrower"),
        ("Metal Burst", "Flamethrower"),
        ("Comeuppance", "Body Slam"),
    ];
    let base = serde_json::json!({
        "name": "Neutral", "types": ["Normal", ""],
        "baseStats": { "hp": 100, "attack": 100, "defense": 100, "specialAttack": 100, "specialDefense": 100, "speed": 100 },
        "statPoints": {}, "nature": "Hardy"
    });
    let cases = pairs
        .iter()
        .map(|(counter, opposing)| {
            let mut left = base.clone();
            left["moves"] = serde_json::json!([{ "name": counter, "usedOppMoveIndex": 0 }]);
            let mut right = base.clone();
            right["moves"] = serde_json::json!([{ "name": opposing }]);
            serde_json::json!({ "left": left, "right": right, "field": { "format": "Doubles" } })
        })
        .collect::<Vec<_>>();
    let oracle = run_oracle(&serde_json::Value::Array(cases), &reference);
    let oracle_cases = oracle.as_array().expect("oracle cases");
    let stats = StatTable::new(100, 100, 100, 100, 100, 100);
    let points = StatTable::new(0, 0, 0, 0, 0, 0);
    let pokemon = Pokemon::champions(
        "Neutral",
        [Some(PokemonType::Normal), None],
        stats,
        points,
        Nature::Hardy,
    );
    let no_move = champions_reference_move("(No Move)")
        .expect("No Move")
        .move_();

    for (index, (counter, opposing)) in pairs.iter().enumerate() {
        let counter = champions_reference_move(counter).expect("counter").move_();
        let opposing = champions_reference_move(opposing)
            .expect("opposing")
            .move_();
        let result = calculate_all_moves(BatchCalcInput {
            left: pokemon.clone(),
            right: pokemon.clone(),
            left_moves: [counter, no_move.clone(), no_move.clone(), no_move.clone()],
            right_moves: [opposing, no_move.clone(), no_move.clone(), no_move.clone()],
            left_to_right_field: Field::default(),
            right_to_left_field: Field::default(),
            ruleset: Ruleset::Champions,
        })
        .expect("counter batch");
        let expected = oracle_cases[index]["left"][0]["damage"]
            .as_array()
            .expect("counter rolls")
            .iter()
            .map(|roll| roll.as_u64().expect("counter roll") as u16)
            .collect::<Vec<_>>();
        assert_eq!(result.left[0].damage_rolls, expected, "{}", pairs[index].0);
    }
}
