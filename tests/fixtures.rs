#![allow(clippy::field_reassign_with_default)]

use damage_calc::{
    calculate_all_moves, calculate_damage, calculate_hp, calculate_non_hp_stat, Ability,
    BatchCalcInput, Boosts, CalcInput, Category, DamageOutcome, EffectCount, Field, Format, Item,
    Move, Nature, Pokemon, PokemonType, RivalryTarget, Ruleset, SideConditions, Stat, StatTable,
    StatusCondition, Terrain, Weather,
};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug)]
struct GoldenCase {
    expected_damage_rolls: Vec<u16>,
    expected_min: u16,
    expected_max: u16,
}

fn golden(name: &str) -> GoldenCase {
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/js_outputs/champions_cases.json"))
            .expect("valid fixture JSON");
    let cases = data["cases"].as_array().expect("fixture cases array");
    let case = cases
        .iter()
        .find(|case| case["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing golden fixture {name}"));
    GoldenCase {
        expected_damage_rolls: case["expected_damage_rolls"]
            .as_array()
            .expect("damage rolls")
            .iter()
            .map(|value| value.as_u64().expect("u16 roll") as u16)
            .collect(),
        expected_min: case["expected_min"].as_u64().expect("min") as u16,
        expected_max: case["expected_max"].as_u64().expect("max") as u16,
    }
}

fn stat_100_mon(name: &str, type_: PokemonType) -> Pokemon {
    Pokemon::champions(
        name,
        [Some(type_), None],
        StatTable::new(100, 100, 100, 100, 100, 100),
        StatTable::new(0, 0, 0, 0, 0, 0),
        Nature::Hardy,
    )
}

fn calc(
    attacker: Pokemon,
    defender: Pokemon,
    move_: Move,
    field: Field,
) -> damage_calc::DamageResult {
    calculate_damage(CalcInput {
        attacker,
        defender,
        move_,
        field,
        ruleset: Ruleset::Champions,
    })
    .expect("damage calculation succeeds")
}

fn assert_golden(case_name: &str, result: damage_calc::DamageResult) {
    let expected = golden(case_name);
    assert_eq!(result.damage_rolls, expected.expected_damage_rolls);
    assert_eq!(result.min_damage, expected.expected_min);
    assert_eq!(result.max_damage, expected.expected_max);
}

fn normalized_identifier(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

#[test]
fn champions_stat_calculation_matches_js_formula() {
    assert_eq!(calculate_hp(100, 0, Ruleset::Champions).unwrap(), 175);
    assert_eq!(calculate_hp(1, 32, Ruleset::Champions).unwrap(), 1);
    assert_eq!(
        calculate_non_hp_stat(Stat::Attack, 100, 0, Nature::Hardy, Ruleset::Champions).unwrap(),
        120
    );
    assert_eq!(
        calculate_non_hp_stat(Stat::Attack, 100, 0, Nature::Adamant, Ruleset::Champions).unwrap(),
        132
    );
    assert_eq!(
        calculate_non_hp_stat(
            Stat::SpecialAttack,
            100,
            0,
            Nature::Adamant,
            Ruleset::Champions
        )
        .unwrap(),
        108
    );
}

#[test]
fn neutral_damage_matches_fixture() {
    let attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let move_ = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);

    assert_golden(
        "neutral_non_stab",
        calc(attacker, defender, move_, Field::default()),
    );
}

#[test]
fn stab_damage_matches_fixture() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let move_ = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);

    assert_golden("stab", calc(attacker, defender, move_, Field::default()));
}

#[test]
fn type_item_ability_weather_and_terrain_modifiers_are_applied() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Electric);
    attacker.item = Item::Magnet;
    attacker.ability = Ability::Transistor;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let move_ = Move::new("Thunderbolt", 90, PokemonType::Electric, Category::Special);
    let mut field = Field::default();
    field.terrain = Terrain::Electric;

    let result = calc(attacker, defender, move_, field);
    assert_eq!(
        result.damage_rolls,
        vec![206, 210, 212, 216, 216, 218, 222, 224, 228, 230, 230, 234, 236, 240, 242, 246]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "type item"));
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Transistor"));
}

#[test]
fn fire_mane_boosts_fire_type_damage() {
    let mut attacker = Pokemon::champions(
        "Mega Pyroar",
        [Some(PokemonType::Fire), Some(PokemonType::Normal)],
        StatTable::new(86, 88, 92, 129, 86, 126),
        StatTable::new(0, 0, 0, 0, 0, 0),
        Nature::Modest,
    );
    attacker.ability = Ability::FireMane;
    let defender = Pokemon::champions(
        "Aegislash-Shield",
        [Some(PokemonType::Steel), Some(PokemonType::Ghost)],
        StatTable::new(60, 50, 140, 50, 140, 60),
        StatTable::new(32, 0, 0, 0, 2, 0),
        Nature::Hardy,
    );
    let mut move_ = Move::new("Heat Wave", 95, PokemonType::Fire, Category::Special);
    move_.is_spread = true;

    let boosted = calc(
        attacker.clone(),
        defender.clone(),
        move_.clone(),
        Field::default(),
    );
    attacker.ability = Ability::None;
    let neutral = calc(attacker, defender, move_, Field::default());

    assert_eq!(
        boosted.damage_rolls,
        vec![120, 122, 122, 126, 126, 128, 128, 132, 132, 134, 134, 138, 138, 140, 140, 144]
    );
    assert!(boosted.min_damage > neutral.min_damage);
    assert!(boosted
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "attack ability 1.5"));
}

#[test]
fn sun_fire_stab_super_effective_matches_fixture() {
    let attacker = stat_100_mon("Attacker", PokemonType::Fire);
    let defender = stat_100_mon("Defender", PokemonType::Grass);
    let move_ = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let mut field = Field::default();
    field.weather = Weather::Sun;

    assert_golden(
        "sun_fire_stab_super_effective",
        calc(attacker, defender, move_, field),
    );
}

#[test]
fn critical_hit_ignores_negative_attack_and_positive_defense_stages() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Normal);
    attacker.boosts = Boosts {
        attack: -2,
        ..Boosts::default()
    };
    let mut defender = stat_100_mon("Defender", PokemonType::Psychic);
    defender.boosts = Boosts {
        defense: 2,
        ..Boosts::default()
    };
    let mut move_ = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    move_.is_critical = true;

    let result = calc(attacker, defender, move_, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![34, 36, 36, 36, 36, 37, 37, 37, 39, 39, 39, 39, 40, 40, 40, 42]
    );
}

#[test]
fn bad_weather_halves_solar_beam_and_solar_blade_like_js() {
    let attacker = stat_100_mon("Attacker", PokemonType::Grass);
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let solar_beam = Move::new("Solar Beam", 120, PokemonType::Grass, Category::Special);
    let mut rain = Field::default();
    rain.weather = Weather::Rain;

    let result = calc(attacker.clone(), defender.clone(), solar_beam.clone(), rain);
    assert_eq!(
        result.damage_rolls,
        vec![68, 72, 72, 72, 72, 74, 74, 74, 78, 78, 78, 78, 80, 80, 80, 84]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "bad-weather Solar move"));

    let mut umbrella_attacker = attacker.clone();
    umbrella_attacker.item = Item::UtilityUmbrella;
    let result = calc(
        umbrella_attacker,
        defender.clone(),
        solar_beam.clone(),
        rain,
    );
    assert_eq!(
        result.damage_rolls,
        vec![134, 138, 138, 140, 144, 144, 146, 146, 150, 150, 152, 152, 156, 156, 158, 162]
    );

    let mut mega_sol_attacker = attacker;
    mega_sol_attacker.ability = Ability::MegaSol;
    let solar_blade = Move::new("Solar Blade", 125, PokemonType::Grass, Category::Physical);
    let result = calc(mega_sol_attacker, defender, solar_blade, rain);
    assert_eq!(
        result.damage_rolls,
        vec![144, 146, 146, 150, 150, 152, 152, 156, 158, 158, 162, 162, 164, 164, 168, 170]
    );
}

#[test]
fn electromorphosis_and_wind_power_double_electric_base_power_like_js_charge() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Electric);
    attacker.ability = Ability::Electromorphosis;
    attacker.ability_on = true;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let thunderbolt = Move::new("Thunderbolt", 90, PokemonType::Electric, Category::Special);

    let result = calc(
        attacker.clone(),
        defender.clone(),
        thunderbolt.clone(),
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![204, 206, 210, 212, 216, 216, 218, 222, 224, 228, 228, 230, 234, 236, 240, 242]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Charge"));

    attacker.ability_on = false;
    let result = calc(
        attacker.clone(),
        defender.clone(),
        thunderbolt.clone(),
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![102, 104, 104, 108, 108, 108, 110, 110, 114, 114, 114, 116, 116, 120, 120, 122]
    );

    attacker.ability = Ability::WindPower;
    attacker.ability_on = true;
    let result = calc(attacker, defender, thunderbolt, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![204, 206, 210, 212, 216, 216, 218, 222, 224, 228, 228, 230, 234, 236, 240, 242]
    );
}

#[test]
fn burn_and_reflect_match_fixture() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Normal);
    attacker.status = StatusCondition::Burned;
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let move_ = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let mut field = Field {
        format: Format::Singles,
        ..Field::default()
    };
    field.defender_side = SideConditions {
        reflect: true,
        ..SideConditions::default()
    };

    assert_golden("burn_reflect", calc(attacker, defender, move_, field));
}

#[test]
fn light_screen_and_resist_berry_reduce_special_damage() {
    let attacker = stat_100_mon("Attacker", PokemonType::Fire);
    let mut defender = stat_100_mon("Defender", PokemonType::Grass);
    defender.item = Item::OccaBerry;
    let move_ = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let mut field = Field {
        format: Format::Singles,
        ..Field::default()
    };
    field.defender_side.light_screen = true;

    let result = calc(attacker, defender, move_, field);
    assert_eq!(
        result.damage_rolls,
        vec![25, 26, 26, 27, 27, 27, 27, 27, 28, 28, 28, 29, 29, 30, 30, 30]
    );

    let mut infiltrator = stat_100_mon("Infiltrator", PokemonType::Fire);
    infiltrator.ability = Ability::Infiltrator;
    let mut no_berry_defender = stat_100_mon("Defender", PokemonType::Grass);
    no_berry_defender.item = Item::None;
    let move_ = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let result = calc(infiltrator, no_berry_defender.clone(), move_.clone(), field);
    assert_eq!(
        result.damage_rolls,
        vec![102, 104, 104, 108, 108, 108, 110, 110, 114, 114, 114, 116, 116, 120, 120, 122]
    );
    assert!(!result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Light Screen"));

    let mut screen_ignoring_move =
        Move::new("Screen Break", 90, PokemonType::Fire, Category::Special);
    screen_ignoring_move.ignores_screens = true;
    let result = calc(
        stat_100_mon("Attacker", PokemonType::Fire),
        no_berry_defender,
        screen_ignoring_move,
        field,
    );
    assert_eq!(
        result.damage_rolls,
        vec![102, 104, 104, 108, 108, 108, 110, 110, 114, 114, 114, 116, 116, 120, 120, 122]
    );
}

#[test]
fn variable_base_power_moves_match_champions_formulas() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.weight_kg = 220.0;
    let low_kick = Move::new("Low Kick", 1, PokemonType::Fighting, Category::Physical);
    assert_eq!(
        calc(
            attacker.clone(),
            defender.clone(),
            low_kick,
            Field::default()
        )
        .damage_rolls,
        vec![134, 138, 138, 140, 144, 144, 146, 146, 150, 150, 152, 152, 156, 156, 158, 162]
    );

    attacker.weight_kg = 500.0;
    defender.weight_kg = 80.0;
    let heavy_slam = Move::new("Heavy Slam", 1, PokemonType::Steel, Category::Physical);
    assert_eq!(
        calc(
            attacker.clone(),
            defender.clone(),
            heavy_slam,
            Field::default()
        )
        .damage_rolls,
        vec![45, 46, 46, 47, 48, 48, 49, 49, 50, 50, 51, 51, 52, 52, 53, 54]
    );

    attacker.item = Item::IronBall;
    let fling = Move::new("Fling", 1, PokemonType::Dark, Category::Physical);
    assert_eq!(
        calc(attacker.clone(), defender.clone(), fling, Field::default()).damage_rolls,
        vec![50, 50, 51, 51, 52, 53, 53, 54, 54, 55, 56, 56, 57, 57, 58, 59]
    );

    let mut fast = stat_100_mon("Fast", PokemonType::Electric);
    fast.boosts.speed = 6;
    let mut slow = stat_100_mon("Slow", PokemonType::Water);
    slow.boosts.speed = -6;
    let electro_ball = Move::new("Electro Ball", 1, PokemonType::Electric, Category::Special);
    assert_eq!(
        calc(fast, slow, electro_ball, Field::default()).damage_rolls,
        vec![170, 174, 176, 176, 180, 182, 182, 186, 188, 188, 192, 194, 194, 198, 200, 204]
    );

    let attacker = stat_100_mon("Attacker", PokemonType::Dark);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let mut payback = Move::new("Payback", 50, PokemonType::Dark, Category::Physical);
    let base_payback = calc(
        attacker.clone(),
        defender.clone(),
        payback.clone(),
        Field::default(),
    );
    assert_eq!(
        base_payback.damage_rolls,
        vec![60, 60, 60, 62, 62, 62, 62, 66, 66, 66, 66, 68, 68, 68, 68, 72]
    );
    payback.is_double_power = true;
    let boosted_payback = calc(
        attacker.clone(),
        defender.clone(),
        payback,
        Field::default(),
    );
    assert_eq!(
        boosted_payback.damage_rolls,
        vec![116, 116, 120, 120, 120, 122, 122, 126, 126, 128, 128, 132, 132, 134, 134, 138]
    );
    assert!(boosted_payback
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "double-power move flag"));

    let attacker = stat_100_mon("Attacker", PokemonType::Electric);
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let mut bolt_beak = Move::new("Bolt Beak", 85, PokemonType::Electric, Category::Physical);
    let base_bolt_beak = calc(
        attacker.clone(),
        defender.clone(),
        bolt_beak.clone(),
        Field::default(),
    );
    assert_eq!(
        base_bolt_beak.damage_rolls,
        vec![98, 98, 98, 102, 102, 104, 104, 104, 108, 108, 110, 110, 110, 114, 114, 116]
    );
    bolt_beak.is_double_power = true;
    let boosted_bolt_beak = calc(attacker, defender, bolt_beak, Field::default());
    assert_eq!(
        boosted_bolt_beak.damage_rolls,
        vec![192, 194, 198, 198, 200, 204, 206, 206, 210, 212, 216, 216, 218, 222, 224, 228]
    );
    assert!(boosted_bolt_beak
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "double-power move flag"));

    let mut assurance = Move::new("Assurance", 60, PokemonType::Dark, Category::Physical);
    assurance.is_double_power = true;
    let result = calc(
        stat_100_mon("Attacker", PokemonType::Dark),
        stat_100_mon("Defender", PokemonType::Psychic),
        assurance,
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![134, 138, 138, 140, 144, 144, 146, 146, 150, 150, 152, 152, 156, 156, 158, 162]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "double-power move flag"));
}

#[test]
fn last_respects_uses_shared_effect_count_for_base_power() {
    let attacker = stat_100_mon("Attacker", PokemonType::Ghost);
    let defender = stat_100_mon("Defender", PokemonType::Fighting);

    let mut last_respects = Move::new("Last Respects", 50, PokemonType::Ghost, Category::Physical);
    last_respects.set_effect_count(EffectCount::Five);
    let equivalent_300_bp = Move::new("Equivalent", 300, PokemonType::Ghost, Category::Physical);

    let result = calc(
        attacker.clone(),
        defender.clone(),
        last_respects,
        Field::default(),
    );
    let equivalent = calc(attacker, defender, equivalent_300_bp, Field::default());

    assert_eq!(result.damage_rolls, equivalent.damage_rolls);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "5x effect"));
}

#[test]
fn zero_effect_last_respects_keeps_printed_base_power() {
    let attacker = stat_100_mon("Attacker", PokemonType::Ghost);
    let defender = stat_100_mon("Defender", PokemonType::Fighting);

    let last_respects = Move::new("Last Respects", 50, PokemonType::Ghost, Category::Physical);
    let equivalent_50_bp = Move::new("Equivalent", 50, PokemonType::Ghost, Category::Physical);

    let result = calc(
        attacker.clone(),
        defender.clone(),
        last_respects,
        Field::default(),
    );
    let equivalent = calc(attacker, defender, equivalent_50_bp, Field::default());

    assert_eq!(result.damage_rolls, equivalent.damage_rolls);
    assert!(!result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label.ends_with("x effect")));
}

#[test]
fn rage_fist_reuses_last_respects_effect_count_path() {
    let attacker = stat_100_mon("Attacker", PokemonType::Ghost);
    let defender = stat_100_mon("Defender", PokemonType::Fighting);

    let mut rage_fist = Move::new("Rage Fist", 50, PokemonType::Ghost, Category::Physical);
    rage_fist.set_effect_count(EffectCount::Six);
    let equivalent_350_bp = Move::new("Equivalent", 350, PokemonType::Ghost, Category::Physical);

    let result = calc(
        attacker.clone(),
        defender.clone(),
        rage_fist,
        Field::default(),
    );
    let equivalent = calc(attacker, defender, equivalent_350_bp, Field::default());

    assert_eq!(result.damage_rolls, equivalent.damage_rolls);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "6x effect"));
}

#[test]
fn supreme_overlord_uses_effect_count_and_caps_at_five_allies() {
    let defender = stat_100_mon("Defender", PokemonType::Fairy);
    let iron_head = Move::new("Iron Head", 80, PokemonType::Steel, Category::Physical);

    let mut five_down = stat_100_mon("Kingambit", PokemonType::Steel);
    five_down.ability = Ability::SupremeOverlord;
    five_down.set_supreme_overlord_effect(EffectCount::Five);

    let mut six_down = five_down.clone();
    six_down.set_supreme_overlord_effect(EffectCount::Six);

    let five_result = calc(
        five_down,
        defender.clone(),
        iron_head.clone(),
        Field::default(),
    );
    let six_result = calc(six_down, defender, iron_head, Field::default());

    assert_eq!(five_result.damage_rolls, six_result.damage_rolls);
    assert!(six_result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Supreme Overlord"));
}

#[test]
fn fixed_and_hp_dependent_damage_moves_match_champions_formulas() {
    let attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    let mut fixed_defender = stat_100_mon("Defender", PokemonType::Normal);
    fixed_defender.current_hp = Some(99);
    fixed_defender.max_hp_override = Some(175);
    let super_fang = Move::new("Super Fang", 1, PokemonType::Normal, Category::Physical);
    assert_eq!(
        calc(
            attacker.clone(),
            fixed_defender.clone(),
            super_fang,
            Field::default()
        )
        .damage_rolls,
        vec![49]
    );

    let endeavor = Move::new("Endeavor", 1, PokemonType::Normal, Category::Physical);
    let mut low_hp_attacker = attacker.clone();
    low_hp_attacker.current_hp = Some(12);
    assert_eq!(
        calc(
            low_hp_attacker,
            fixed_defender.clone(),
            endeavor,
            Field::default()
        )
        .damage_rolls,
        vec![87]
    );

    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);
    assert_eq!(
        calc(attacker, fixed_defender, seismic_toss, Field::default()).damage_rolls,
        vec![50]
    );
}

#[test]
fn counter_style_moves_use_supplied_countered_damage_like_js() {
    let attacker = stat_100_mon("Counter User", PokemonType::Fighting);
    let defender = stat_100_mon("Target", PokemonType::Normal);
    let mut counter = Move::new("Counter", 1, PokemonType::Fighting, Category::Physical);
    counter.countered_damage_rolls = Some(vec![10, 12, 13]);
    counter.countered_move_category = Some(Category::Physical);
    assert_eq!(
        calc(
            attacker.clone(),
            defender.clone(),
            counter,
            Field::default()
        )
        .damage_rolls,
        vec![20, 24, 26]
    );

    let mut mirror_coat = Move::new("Mirror Coat", 1, PokemonType::Psychic, Category::Special);
    mirror_coat.countered_damage_rolls = Some(vec![10, 12, 13]);
    mirror_coat.countered_move_category = Some(Category::Physical);
    assert_eq!(
        calc(
            attacker.clone(),
            defender.clone(),
            mirror_coat,
            Field::default()
        )
        .damage_rolls,
        vec![0]
    );

    let mut metal_burst = Move::new("Metal Burst", 1, PokemonType::Steel, Category::Physical);
    metal_burst.countered_damage_rolls = Some(vec![10, 12, 13]);
    metal_burst.countered_move_category = Some(Category::Special);
    assert_eq!(
        calc(attacker, defender, metal_burst, Field::default()).damage_rolls,
        vec![15, 18, 19]
    );
}

#[test]
fn type_changing_ability_changes_type_and_applies_bp_boost() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fairy);
    attacker.ability = Ability::Pixilate;
    let defender = stat_100_mon("Defender", PokemonType::Dark);
    let move_ = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let result = calc(attacker, defender, move_, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![56, 56, 60, 60, 60, 60, 60, 62, 62, 62, 62, 66, 66, 66, 66, 68]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "type-changing ability"));
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "type-changing ability boost"));
}

#[test]
fn remaining_champions_abilities_plus_minus_ripen_disguise_and_sand_spit_match_js() {
    let mut plus = stat_100_mon("Plus", PokemonType::Electric);
    plus.ability = Ability::Plus;
    plus.ability_on = true;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let thunderbolt = Move::new("Thunderbolt", 90, PokemonType::Electric, Category::Special);
    let result = calc(plus, defender.clone(), thunderbolt, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![152, 156, 158, 158, 162, 162, 164, 168, 168, 170, 170, 174, 176, 176, 180, 182]
    );

    let attacker = stat_100_mon("Attacker", PokemonType::Fire);
    let mut ripen_defender = stat_100_mon("Ripen", PokemonType::Grass);
    ripen_defender.ability = Ability::Ripen;
    ripen_defender.item = Item::OccaBerry;
    let flamethrower = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let result = calc(
        attacker.clone(),
        ripen_defender,
        flamethrower.clone(),
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![25, 26, 26, 27, 27, 27, 27, 27, 28, 28, 28, 29, 29, 30, 30, 30]
    );

    let mut disguised = stat_100_mon("Mimikyu", PokemonType::Ghost);
    disguised.ability = Ability::Disguise;
    disguised.ability_on = true;
    let result = calc(
        attacker.clone(),
        disguised.clone(),
        flamethrower.clone(),
        Field::default(),
    );
    disguised.ability = Ability::None;
    let undisguised = calc(
        attacker.clone(),
        disguised,
        flamethrower.clone(),
        Field::default(),
    );
    assert_eq!(result.damage_rolls, undisguised.damage_rolls);

    let mut sand_spit = stat_100_mon("Sand Spit", PokemonType::Normal);
    sand_spit.ability = Ability::SandSpit;
    let mut weather_ball = Move::new("Weather Ball", 50, PokemonType::Normal, Category::Special);
    weather_ball.hits = 2;
    let result = calc(attacker, sand_spit, weather_ball, Field::default());
    assert_eq!(
        result.hit_rolls[0],
        vec![20, 20, 20, 21, 21, 21, 21, 22, 22, 22, 22, 23, 23, 23, 23, 24]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![20, 20, 20, 21, 21, 21, 21, 22, 22, 22, 22, 23, 23, 23, 23, 24]
    );
}

#[test]
fn missing_champions_entry_and_defensive_abilities_affect_damage() {
    let mut rain_attacker = stat_100_mon("Drizzle", PokemonType::Water);
    rain_attacker.ability = Ability::Drizzle;
    let defender = stat_100_mon("Defender", PokemonType::Normal);
    let water = Move::new("Water Pulse", 60, PokemonType::Water, Category::Special);
    let drizzle_result = calc(
        rain_attacker,
        defender.clone(),
        water.clone(),
        Field::default(),
    );
    let manual_rain = {
        let mut field = Field::default();
        field.weather = Weather::Rain;
        calc(
            stat_100_mon("Attacker", PokemonType::Water),
            defender.clone(),
            water,
            field,
        )
    };
    assert_eq!(drizzle_result.damage_rolls, manual_rain.damage_rolls);
    assert!(drizzle_result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Drizzle"));

    let mut attacker = stat_100_mon("Flower Veil", PokemonType::Grass);
    attacker.ability = Ability::FlowerVeil;
    let mut intimidate_defender = defender.clone();
    intimidate_defender.ability = Ability::Intimidate;
    intimidate_defender.ability_on = true;
    let tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let blocked = calc(
        attacker.clone(),
        intimidate_defender.clone(),
        tackle.clone(),
        Field::default(),
    );
    attacker.ability = Ability::None;
    let lowered = calc(
        attacker,
        intimidate_defender,
        tackle.clone(),
        Field::default(),
    );
    assert_eq!(blocked.damage_rolls, lowered.damage_rolls);
    assert!(!blocked
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Flower Veil blocked Intimidate"));

    let mut screened = Field::default();
    screened.defender_side.reflect = true;
    let reflected = calc(
        stat_100_mon("Attacker", PokemonType::Normal),
        defender.clone(),
        tackle.clone(),
        screened,
    );
    let mut cleaner_defender = defender;
    cleaner_defender.ability = Ability::ScreenCleaner;
    let mut screened = Field::default();
    screened.defender_side.reflect = true;
    let cleaned = calc(
        stat_100_mon("Attacker", PokemonType::Normal),
        cleaner_defender,
        tackle,
        screened,
    );
    assert!(cleaned.min_damage > reflected.min_damage);
    assert!(cleaned
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Screen Cleaner"));
}

#[test]
fn missing_champions_speed_crit_and_aura_abilities_affect_damage() {
    let mut swift = stat_100_mon("Swift Swim", PokemonType::Water);
    swift.ability = Ability::SwiftSwim;
    let defender = stat_100_mon("Defender", PokemonType::Normal);
    let electro_ball = Move::new("Electro Ball", 1, PokemonType::Electric, Category::Special);
    let mut rain = Field::default();
    rain.weather = Weather::Rain;
    let boosted_speed = calc(swift.clone(), defender.clone(), electro_ball.clone(), rain);
    swift.ability = Ability::None;
    let mut rain = Field::default();
    rain.weather = Weather::Rain;
    let normal_speed = calc(swift, defender.clone(), electro_ball, rain);
    assert!(boosted_speed.min_damage > normal_speed.min_damage);

    let mut unburden = stat_100_mon("Unburden", PokemonType::Normal);
    unburden.ability = Ability::Unburden;
    unburden.ability_on = true;
    let unburden_damage = calc(
        unburden.clone(),
        defender.clone(),
        Move::new("Electro Ball", 1, PokemonType::Electric, Category::Special),
        Field::default(),
    );
    unburden.ability_on = false;
    let normal_damage = calc(
        unburden,
        defender.clone(),
        Move::new("Electro Ball", 1, PokemonType::Electric, Category::Special),
        Field::default(),
    );
    assert!(unburden_damage.min_damage > normal_damage.min_damage);

    let mut battle_armor = defender.clone();
    battle_armor.ability = Ability::BattleArmor;
    let mut crit = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    crit.is_critical = true;
    let blocked = calc(
        stat_100_mon("Attacker", PokemonType::Normal),
        battle_armor,
        crit,
        Field::default(),
    );
    assert!(blocked
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "critical blocked"));
    assert!(!blocked
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "critical"));

    let mut merciless = stat_100_mon("Merciless", PokemonType::Poison);
    merciless.ability = Ability::Merciless;
    let mut poisoned = defender.clone();
    poisoned.status = StatusCondition::Poisoned;
    let result = calc(
        merciless,
        poisoned,
        Move::new("Poison Jab", 80, PokemonType::Poison, Category::Physical),
        Field::default(),
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "critical"));

    let mut fairy = stat_100_mon("Fairy Aura", PokemonType::Fairy);
    fairy.ability = Ability::FairyAura;
    let fairy_move = Move::new("Moonblast", 95, PokemonType::Fairy, Category::Special);
    let boosted = calc(
        fairy.clone(),
        defender.clone(),
        fairy_move.clone(),
        Field::default(),
    );
    fairy.ability = Ability::None;
    let normal = calc(fairy, defender, fairy_move, Field::default());
    assert!(boosted.min_damage > normal.min_damage);
    assert!(boosted
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Fairy Aura"));

    let mut rivalry = stat_100_mon("Rivalry", PokemonType::Normal);
    rivalry.ability = Ability::Rivalry;
    let tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let neutral = calc(
        rivalry.clone(),
        stat_100_mon("Defender", PokemonType::Normal),
        tackle.clone(),
        Field::default(),
    );
    assert_eq!(
        neutral.damage_rolls,
        vec![24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 27, 27, 27, 27, 27, 28]
    );

    rivalry.rivalry_target = RivalryTarget::SameGender;
    let same_gender = calc(
        rivalry.clone(),
        stat_100_mon("Defender", PokemonType::Normal),
        tackle.clone(),
        Field::default(),
    );
    assert_eq!(
        same_gender.damage_rolls,
        vec![30, 30, 30, 31, 31, 31, 31, 33, 33, 33, 33, 34, 34, 34, 34, 36]
    );
    assert!(same_gender
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Rivalry same gender"));

    rivalry.rivalry_target = RivalryTarget::OppositeGender;
    let opposite_gender = calc(
        rivalry,
        stat_100_mon("Defender", PokemonType::Normal),
        tackle,
        Field::default(),
    );
    assert_eq!(
        opposite_gender.damage_rolls,
        vec![18, 18, 19, 19, 19, 19, 19, 19, 19, 21, 21, 21, 21, 21, 21, 22]
    );
    assert!(opposite_gender
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Rivalry opposite gender"));
    assert!(same_gender.min_damage > neutral.min_damage);
    assert!(opposite_gender.max_damage < neutral.max_damage);
}

#[test]
fn additional_champions_abilities_that_change_damage_state_are_modeled() {
    let mut skill_link = stat_100_mon("Skill Link", PokemonType::Grass);
    skill_link.ability = Ability::SkillLink;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let bullet_seed = Move::new("Bullet Seed", 25, PokemonType::Grass, Category::Physical);
    let result = calc(
        skill_link,
        defender.clone(),
        bullet_seed.clone(),
        Field::default(),
    );
    assert_eq!(result.hit_rolls.len(), 5);
    assert!(
        result.max_damage
            > calc(
                stat_100_mon("Attacker", PokemonType::Grass),
                defender.clone(),
                bullet_seed,
                Field::default()
            )
            .max_damage
    );

    let mut speed_boost = stat_100_mon("Speed Boost", PokemonType::Electric);
    speed_boost.ability = Ability::SpeedBoost;
    speed_boost.ability_on = true;
    let electro_ball = Move::new("Electro Ball", 1, PokemonType::Electric, Category::Special);
    let mut slower_defender = defender.clone();
    slower_defender.boosts.speed = -1;
    let boosted_speed = calc(
        speed_boost.clone(),
        slower_defender.clone(),
        electro_ball.clone(),
        Field::default(),
    );
    speed_boost.ability_on = false;
    let normal_speed = calc(speed_boost, slower_defender, electro_ball, Field::default());
    assert_eq!(boosted_speed.damage_rolls, normal_speed.damage_rolls);
    assert!(!boosted_speed
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Speed Boost"));

    let mut opportunist = stat_100_mon("Opportunist", PokemonType::Psychic);
    opportunist.ability = Ability::Opportunist;
    opportunist.ability_on = true;
    let mut boosted_target = defender.clone();
    boosted_target.boosts.special_attack = 2;
    let psychic = Move::new("Psychic", 90, PokemonType::Psychic, Category::Special);
    let copied = calc(
        opportunist.clone(),
        boosted_target.clone(),
        psychic.clone(),
        Field::default(),
    );
    opportunist.ability_on = false;
    let uncopied = calc(opportunist, boosted_target, psychic, Field::default());
    assert_eq!(copied.damage_rolls, uncopied.damage_rolls);
    assert!(!copied
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Opportunist"));

    let mut gale_wings = stat_100_mon("Gale Wings", PokemonType::Flying);
    gale_wings.ability = Ability::GaleWings;
    let mut armor_tail = stat_100_mon("Armor Tail", PokemonType::Normal);
    armor_tail.ability = Ability::ArmorTail;
    let aerial_ace = Move::new("Aerial Ace", 60, PokemonType::Flying, Category::Physical);
    let blocked = calc(
        gale_wings.clone(),
        armor_tail.clone(),
        aerial_ace.clone(),
        Field::default(),
    );
    assert_eq!(blocked.damage_rolls, vec![0]);
    assert!(blocked
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "priority-blocking ability"));
    gale_wings.max_hp_override = Some(200);
    gale_wings.current_hp = Some(199);
    let unblocked = calc(gale_wings, armor_tail, aerial_ace, Field::default());
    assert!(unblocked.min_damage > 0);
}

#[test]
fn ruin_field_modifiers_apply_at_the_correct_stage() {
    let attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let move_ = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let mut field = Field::default();
    field.tablets_of_ruin = true;
    let result = calc(attacker, defender, move_, field);
    assert_eq!(
        result.damage_rolls,
        vec![12, 12, 13, 13, 13, 13, 13, 13, 13, 14, 14, 14, 14, 14, 14, 15]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Tablets of Ruin"));
}

#[test]
fn named_spread_moves_apply_doubles_spread_modifier_without_manual_flag() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let defender = stat_100_mon("Defender", PokemonType::Normal);
    let rock_slide = Move::new("Rock Slide", 75, PokemonType::Rock, Category::Physical);

    let mut singles = Field::default();
    singles.format = Format::Singles;
    let singles_result = calc(
        attacker.clone(),
        defender.clone(),
        rock_slide.clone(),
        singles,
    );
    assert_eq!(
        singles_result.damage_rolls,
        vec![29, 30, 30, 30, 31, 31, 31, 32, 32, 32, 33, 33, 33, 34, 34, 35]
    );
    assert!(!singles_result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "spread"));

    let doubles_result = calc(
        attacker.clone(),
        defender.clone(),
        rock_slide,
        Field::default(),
    );
    assert_eq!(
        doubles_result.damage_rolls,
        vec![22, 22, 22, 22, 23, 23, 23, 23, 24, 24, 24, 24, 25, 25, 25, 26]
    );
    assert!(doubles_result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "spread" && modifier.modifier == 0x0C00));

    let mut single_target_rock_slide =
        Move::new("Rock Slide", 75, PokemonType::Rock, Category::Physical);
    single_target_rock_slide.targets_single_target = true;
    let single_target_result = calc(
        attacker,
        defender,
        single_target_rock_slide,
        Field::default(),
    );
    assert_eq!(
        single_target_result.damage_rolls,
        singles_result.damage_rolls
    );
    assert!(!single_target_result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "spread"));
}

#[test]
fn priority_blocking_abilities_and_psychic_terrain_prevent_damage() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let mut defender = stat_100_mon("Defender", PokemonType::Psychic);
    defender.ability = Ability::ArmorTail;
    let mut move_ = Move::new("Quick Attack", 40, PokemonType::Normal, Category::Physical);
    move_.is_priority = true;

    let result = calc(
        attacker.clone(),
        defender.clone(),
        move_.clone(),
        Field::default(),
    );
    assert_eq!(result.damage_rolls, vec![0]);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "priority-blocking ability"));

    defender.ability = Ability::None;
    let mut field = Field::default();
    field.terrain = Terrain::Psychic;
    let result = calc(attacker, defender, move_, field);
    assert_eq!(result.damage_rolls, vec![0]);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Psychic Terrain priority block"));

    let attacker = stat_100_mon("Attacker", PokemonType::Flying);
    let mut wind_defender = stat_100_mon("Defender", PokemonType::Grass);
    wind_defender.ability = Ability::WindRider;
    let mut gust = Move::new("Gust", 40, PokemonType::Flying, Category::Special);
    gust.is_wind = true;
    let result = calc(attacker, wind_defender, gust, Field::default());
    assert_eq!(result.damage_rolls, vec![0]);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "defensive immunity ability"));
}

#[test]
fn terastal_stab_uses_original_and_tera_types_like_js() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fire);
    attacker.is_terastalized = true;
    attacker.tera_type = Some(PokemonType::Fire);
    let defender = stat_100_mon("Defender", PokemonType::Grass);
    let move_ = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let result = calc(
        attacker.clone(),
        defender.clone(),
        move_.clone(),
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![136, 140, 140, 144, 144, 144, 148, 148, 152, 152, 152, 156, 156, 160, 160, 164]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Tera boosted STAB"));

    attacker.tera_type = Some(PokemonType::Water);
    let result = calc(attacker, defender, move_, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![102, 104, 104, 108, 108, 108, 110, 110, 114, 114, 114, 116, 116, 120, 120, 122]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Tera STAB"));
}

#[test]
fn natural_gift_uses_berry_type_and_power_table() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fire);
    attacker.item = Item::WatmelBerry;
    let defender = stat_100_mon("Defender", PokemonType::Grass);
    let move_ = Move::new("Natural Gift", 1, PokemonType::Normal, Category::Physical);
    let result = calc(attacker, defender, move_, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![116, 116, 120, 120, 120, 122, 122, 126, 126, 128, 128, 132, 132, 134, 134, 138]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Natural Gift base power"));
}

#[test]
fn dream_eater_requires_sleep_or_comatose_like_js() {
    let attacker = stat_100_mon("Attacker", PokemonType::Psychic);
    let defender = stat_100_mon("Defender", PokemonType::Fighting);
    let dream_eater = Move::new("Dream Eater", 100, PokemonType::Psychic, Category::Special);

    let result = calc(
        attacker.clone(),
        defender.clone(),
        dream_eater.clone(),
        Field::default(),
    );
    assert_eq!(result.damage_rolls, vec![0]);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Dream Eater failed"));

    let mut sleeping_defender = defender.clone();
    sleeping_defender.status = StatusCondition::Asleep;
    let result = calc(
        attacker.clone(),
        sleeping_defender,
        dream_eater.clone(),
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![116, 116, 120, 120, 120, 122, 122, 126, 126, 128, 128, 132, 132, 134, 134, 138]
    );

    let mut comatose_defender = defender;
    comatose_defender.ability = Ability::Comatose;
    let result = calc(attacker, comatose_defender, dream_eater, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![116, 116, 120, 120, 120, 122, 122, 126, 126, 128, 128, 132, 132, 134, 134, 138]
    );
}

#[test]
fn move_failure_gates_match_js_for_sky_drop_synchronoise_damp_and_sturdy() {
    let attacker = stat_100_mon("Attacker", PokemonType::Psychic);
    let flying_defender = stat_100_mon("Defender", PokemonType::Flying);
    let sky_drop = Move::new("Sky Drop", 60, PokemonType::Flying, Category::Physical);
    let result = calc(
        attacker.clone(),
        flying_defender,
        sky_drop,
        Field::default(),
    );
    assert_eq!(result.damage_rolls, vec![0]);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Sky Drop failed"));

    let synchronoise = Move::new("Synchronoise", 120, PokemonType::Psychic, Category::Special);
    let no_shared_type = stat_100_mon("No Shared Type", PokemonType::Fighting);
    let result = calc(
        attacker.clone(),
        no_shared_type,
        synchronoise.clone(),
        Field::default(),
    );
    assert_eq!(result.damage_rolls, vec![0]);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Synchronoise failed"));

    let shared_type = stat_100_mon("Shared Type", PokemonType::Psychic);
    let result = calc(
        attacker.clone(),
        shared_type,
        synchronoise,
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![33, 34, 34, 35, 36, 36, 36, 36, 37, 37, 38, 38, 39, 39, 39, 40]
    );

    let mut damp_defender = stat_100_mon("Damp", PokemonType::Normal);
    damp_defender.ability = Ability::Damp;
    let explosion = Move::new("Explosion", 250, PokemonType::Normal, Category::Physical);
    let result = calc(
        stat_100_mon("Boom", PokemonType::Normal),
        damp_defender,
        explosion,
        Field::default(),
    );
    assert_eq!(result.damage_rolls, vec![0]);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Damp"));

    let mut sturdy_defender = stat_100_mon("Sturdy", PokemonType::Normal);
    sturdy_defender.ability = Ability::Sturdy;
    let mut fissure = Move::new("Fissure", 1, PokemonType::Ground, Category::Physical);
    fissure.is_ohko = true;
    let result = calc(
        stat_100_mon("Attacker", PokemonType::Ground),
        sturdy_defender,
        fissure,
        Field::default(),
    );
    assert_eq!(result.damage_rolls, vec![0]);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Sturdy"));
}

#[test]
fn champions_type_effectiveness_overrides_match_js() {
    let attacker = stat_100_mon("Attacker", PokemonType::Ground);
    let defender = stat_100_mon("Defender", PokemonType::Flying);
    let arrows = Move::new(
        "Thousand Arrows",
        90,
        PokemonType::Ground,
        Category::Physical,
    );
    let result = calc(attacker.clone(), defender, arrows, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![51, 52, 52, 54, 54, 54, 55, 55, 57, 57, 57, 58, 58, 60, 60, 61]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Thousand Arrows type override"));

    let mut tera_shell_defender = stat_100_mon("Defender", PokemonType::Grass);
    tera_shell_defender.ability = Ability::TeraShell;
    let fire = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let result = calc(
        stat_100_mon("Attacker", PokemonType::Fire),
        tera_shell_defender,
        fire,
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![25, 26, 26, 27, 27, 27, 27, 27, 28, 28, 28, 29, 29, 30, 30, 30]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Tera Shell type override"));

    let nihil_light = Move::new("Nihil Light", 90, PokemonType::Dark, Category::Special);
    let fairy_defender = stat_100_mon("Defender", PokemonType::Fairy);
    let result = calc(
        stat_100_mon("Attacker", PokemonType::Dark),
        fairy_defender,
        nihil_light,
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![51, 52, 52, 54, 54, 54, 55, 55, 57, 57, 57, 58, 58, 60, 60, 61]
    );
}

#[test]
fn final_speed_modifiers_feed_speed_based_moves_and_analytic() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Electric);
    attacker.item = Item::ChoiceScarf;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let electro_ball = Move::new("Electro Ball", 1, PokemonType::Electric, Category::Special);
    assert_eq!(
        calc(
            attacker.clone(),
            defender.clone(),
            electro_ball,
            Field::default()
        )
        .damage_rolls,
        vec![68, 72, 72, 72, 72, 74, 74, 74, 78, 78, 78, 78, 80, 80, 80, 84]
    );

    let mut analytic = stat_100_mon("Analytic", PokemonType::Normal);
    analytic.ability = Ability::Analytic;
    let tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let mut fast_field = Field::default();
    fast_field.attacker_tailwind = true;
    assert_eq!(
        calc(
            analytic.clone(),
            defender.clone(),
            tackle.clone(),
            fast_field
        )
        .damage_rolls,
        vec![24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 27, 27, 27, 27, 27, 28]
    );
    let mut slow_field = Field::default();
    slow_field.attacker_swamp = true;
    let result = calc(analytic, defender, tackle, slow_field);
    assert_eq!(
        result.damage_rolls,
        vec![30, 30, 30, 31, 31, 31, 31, 33, 33, 33, 33, 34, 34, 34, 34, 36]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "base power ability 1.3"));
}

#[test]
fn ring_target_and_signature_super_effective_modifiers_match_js() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let mut defender = stat_100_mon("Defender", PokemonType::Ghost);
    let tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    assert_eq!(
        calc(
            attacker.clone(),
            defender.clone(),
            tackle.clone(),
            Field::default()
        )
        .damage_rolls,
        vec![0]
    );
    defender.item = Item::RingTarget;
    assert_eq!(
        calc(attacker, defender, tackle, Field::default()).damage_rolls,
        vec![24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 27, 27, 27, 27, 27, 28]
    );

    let attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    let defender = stat_100_mon("Defender", PokemonType::Dark);
    let collision = Move::new(
        "Collision Course",
        100,
        PokemonType::Fighting,
        Category::Physical,
    );
    let result = calc(attacker, defender, collision, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![155, 155, 160, 160, 160, 163, 163, 168, 168, 171, 171, 176, 176, 179, 179, 184]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "super-effective signature move"));
}

#[test]
fn special_stat_source_moves_use_js_attack_and_defense_stats() {
    let attacker = stat_100_mon("Attacker", PokemonType::Dark);
    let mut defender = stat_100_mon("Defender", PokemonType::Psychic);
    defender.boosts.attack = 4;
    let foul_play = Move::new("Foul Play", 95, PokemonType::Dark, Category::Physical);
    assert_eq!(
        calc(
            attacker.clone(),
            defender.clone(),
            foul_play,
            Field::default()
        )
        .damage_rolls,
        vec![320, 326, 330, 332, 338, 342, 344, 348, 354, 356, 360, 362, 368, 372, 374, 380]
    );

    let mut body_attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    body_attacker.boosts.defense = 4;
    let body_press = Move::new("Body Press", 80, PokemonType::Fighting, Category::Physical);
    assert_eq!(
        calc(
            body_attacker,
            defender.clone(),
            body_press,
            Field::default()
        )
        .damage_rolls,
        vec![67, 69, 69, 70, 71, 72, 72, 73, 74, 75, 75, 76, 77, 78, 78, 80]
    );

    let mut psyshock = Move::new("Psyshock", 80, PokemonType::Psychic, Category::Special);
    psyshock.deals_physical_damage = true;
    assert_eq!(
        calc(attacker, defender, psyshock, Field::default()).damage_rolls,
        vec![15, 15, 16, 16, 16, 16, 16, 17, 17, 17, 17, 17, 17, 18, 18, 18]
    );
}

#[test]
fn protect_only_quarters_js_qualified_moves() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let move_ = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let mut field = Field::default();
    field.protect = true;
    assert_eq!(
        calc(attacker.clone(), defender.clone(), move_.clone(), field).damage_rolls,
        vec![24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 27, 27, 27, 27, 27, 28]
    );

    let mut z_move = move_;
    z_move.is_z = true;
    let result = calc(attacker, defender, z_move, field);
    assert_eq!(
        result.damage_rolls,
        vec![6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Protect"));
}

#[test]
fn custom_modifiers_apply_in_js_modifier_stages() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Normal);
    attacker.custom_final_mods.push(0x2000);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let move_ = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let result = calc(attacker, defender, move_, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![48, 48, 48, 48, 48, 50, 50, 50, 50, 50, 54, 54, 54, 54, 54, 56]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "custom final modifier"));
}

#[test]
fn preprocessing_and_named_move_type_changes_match_js() {
    let mut castform = stat_100_mon("Castform", PokemonType::Normal);
    castform.ability = Ability::Forecast;
    let defender = stat_100_mon("Defender", PokemonType::Grass);
    let move_ = Move::new(
        "Revelation Dance",
        90,
        PokemonType::Normal,
        Category::Special,
    );
    let mut field = Field::default();
    field.weather = Weather::Sun;
    assert_eq!(
        calc(castform, defender.clone(), move_.clone(), field).damage_rolls,
        vec![152, 156, 158, 158, 162, 162, 164, 168, 168, 170, 170, 174, 176, 176, 180, 182]
    );

    let morpeko = stat_100_mon("Morpeko-Hangry", PokemonType::Electric);
    let aura = Move::new("Aura Wheel", 110, PokemonType::Electric, Category::Physical);
    assert_eq!(
        calc(morpeko.clone(), defender.clone(), aura, Field::default()).damage_rolls,
        vec![42, 43, 43, 44, 44, 45, 45, 46, 46, 47, 47, 48, 48, 49, 49, 50]
    );

    let cudgel = Move::new("Ivy Cudgel", 100, PokemonType::Grass, Category::Physical);
    let ogerpon = stat_100_mon("Ogerpon-Hearthflame", PokemonType::Grass);
    assert_eq!(
        calc(ogerpon, defender, cudgel, Field::default()).damage_rolls,
        vec![78, 78, 80, 80, 80, 82, 82, 84, 84, 86, 86, 88, 88, 90, 90, 92]
    );
}

#[test]
fn mimicry_forecast_and_air_lock_preprocessing_match_js() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Water);
    attacker.ability = Ability::Mimicry;
    let defender = stat_100_mon("Defender", PokemonType::Ground);
    let move_ = Move::new("Leafage", 40, PokemonType::Grass, Category::Physical);
    let mut field = Field::default();
    field.terrain = Terrain::Grassy;
    assert_eq!(
        calc(attacker, defender.clone(), move_.clone(), field).damage_rolls,
        vec![60, 60, 60, 62, 62, 62, 62, 66, 66, 66, 66, 68, 68, 68, 68, 72]
    );

    let mut airlock = stat_100_mon("Attacker", PokemonType::Fire);
    airlock.ability = Ability::AirLock;
    let fire = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let mut sun = Field::default();
    sun.weather = Weather::Sun;
    let result = calc(airlock, defender, fire, sun);
    assert_eq!(
        result.damage_rolls,
        vec![51, 52, 52, 54, 54, 54, 55, 55, 57, 57, 57, 58, 58, 60, 60, 61]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "weather suppressed"));
}

#[test]
fn multi_hit_totals_preserve_js_per_hit_rolls() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let mut double_slap = Move::new("Double Slap", 15, PokemonType::Normal, Category::Physical);
    double_slap.hits = 2;

    let result = calc(attacker, defender, double_slap, Field::default());
    assert_eq!(
        result.hit_rolls,
        vec![
            vec![9, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 12],
            vec![9, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 12],
        ]
    );
    assert_eq!(result.damage_rolls.len(), 256);
    assert_eq!(result.min_damage, 18);
    assert_eq!(result.max_damage, 24);
}

#[test]
fn healing_items_are_counted_for_repeated_ko_odds() {
    let attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(80);
    defender.current_hp = Some(80);
    defender.item = Item::SitrusBerry;
    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);

    let result = calc(attacker, defender, seismic_toss, Field::default());
    assert_eq!(result.damage_rolls, vec![50]);
    assert_eq!(result.ko_chance, Some(0.0));
    assert_eq!(result.ko_chance_by_move_use, vec![0.0, 1.0, 1.0, 1.0]);
}

#[test]
fn healing_items_can_trigger_between_multi_hit_hits() {
    let attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    defender.item = Item::SitrusBerry;
    let mut seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);
    seismic_toss.hits = 2;

    let result = calc(attacker, defender, seismic_toss, Field::default());
    assert_eq!(result.damage_rolls, vec![100]);
    assert_eq!(result.percent_range, (100.0, 100.0));
    assert_eq!(result.ko_chance, Some(0.0));
    assert_eq!(result.ko_chance_by_move_use, vec![0.0, 1.0, 1.0, 1.0]);
}

#[test]
fn reference_ko_projection_does_not_model_focus_sash() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    attacker.level = 100;
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    defender.item = Item::FocusSash;
    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);

    let result = calc(attacker, defender, seismic_toss, Field::default());
    assert_eq!(result.damage_rolls, vec![100]);
    assert_eq!(result.ko_chance, Some(1.0));
    assert_eq!(result.ko_chance_by_move_use, vec![1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn focus_sash_can_be_broken_by_later_multi_hit_hits() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    attacker.level = 100;
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    defender.item = Item::FocusSash;
    let mut seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);
    seismic_toss.hits = 2;

    let result = calc(attacker, defender, seismic_toss, Field::default());
    assert_eq!(result.damage_rolls, vec![200]);
    assert_eq!(result.ko_chance, Some(1.0));
    assert_eq!(result.ko_chance_by_move_use, vec![1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn leftovers_recovery_is_counted_between_repeated_ko_odds() {
    let attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    defender.item = Item::Leftovers;
    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);

    let result = calc(attacker, defender, seismic_toss, Field::default());
    assert_eq!(result.damage_rolls, vec![50]);
    assert_eq!(result.ko_chance_by_move_use, vec![0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn burn_tick_is_counted_after_damage_for_ko_odds() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    attacker.level = 94;
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    defender.status = StatusCondition::Burned;
    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);

    let result = calc(attacker, defender, seismic_toss, Field::default());
    assert_eq!(result.damage_rolls, vec![94]);
    assert_eq!(result.ko_chance, Some(1.0));
}

#[test]
fn poison_and_bad_poison_ticks_are_counted_after_damage_for_ko_odds() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    attacker.level = 88;
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    defender.status = StatusCondition::Poisoned;
    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);

    let poisoned = calc(
        attacker.clone(),
        defender.clone(),
        seismic_toss.clone(),
        Field::default(),
    );
    assert_eq!(poisoned.damage_rolls, vec![88]);
    assert_eq!(poisoned.ko_chance, Some(1.0));

    attacker.level = 94;
    defender.status = StatusCondition::BadlyPoisoned;
    let badly_poisoned = calc(attacker, defender, seismic_toss, Field::default());
    assert_eq!(badly_poisoned.damage_rolls, vec![94]);
    assert_eq!(badly_poisoned.ko_chance, Some(1.0));
}

#[test]
fn leech_seed_tick_is_counted_after_damage_for_ko_odds() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    attacker.level = 88;
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);
    let field = Field {
        defender_leech_seed: true,
        ..Field::default()
    };

    let result = calc(attacker, defender, seismic_toss, field);
    assert_eq!(result.damage_rolls, vec![88]);
    assert_eq!(result.ko_chance, Some(1.0));
}

#[test]
fn parental_bond_second_hit_uses_gen_10_quarter_general_modifier() {
    let mut attacker = stat_100_mon("Kangaskhan-Mega", PokemonType::Normal);
    attacker.ability = Ability::ParentalBond;
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);

    let result = calc(attacker, defender, tackle, Field::default());
    assert_eq!(
        result.hit_rolls[0],
        vec![24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 27, 27, 27, 27, 27, 28]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7]
    );
    assert_eq!(result.min_damage, 30);
    assert_eq!(result.max_damage, 35);
}

#[test]
fn stamina_and_weak_armor_recalculate_between_multi_hit_hits_like_js() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let mut defender = stat_100_mon("Defender", PokemonType::Psychic);
    defender.ability = Ability::Stamina;
    let mut double_slap = Move::new("Double Slap", 15, PokemonType::Normal, Category::Physical);
    double_slap.hits = 2;

    let result = calc(
        attacker.clone(),
        defender.clone(),
        double_slap.clone(),
        Field::default(),
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 9]
    );
    assert_eq!(result.min_damage, 16);
    assert_eq!(result.max_damage, 21);

    defender.ability = Ability::WeakArmor;
    let result = calc(attacker, defender, double_slap, Field::default());
    assert_eq!(
        result.hit_rolls[1],
        vec![13, 13, 13, 13, 13, 13, 15, 15, 15, 15, 15, 15, 15, 15, 15, 16]
    );
    assert_eq!(result.min_damage, 22);
    assert_eq!(result.max_damage, 28);
}

#[test]
fn multi_hit_consumables_and_multiscale_recalculate_like_js() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let mut defender = stat_100_mon("Defender", PokemonType::Psychic);
    defender.item = Item::KeeBerry;
    let mut double_slap = Move::new("Double Slap", 15, PokemonType::Normal, Category::Physical);
    double_slap.hits = 2;

    let result = calc(
        attacker.clone(),
        defender.clone(),
        double_slap.clone(),
        Field::default(),
    );
    assert_eq!(
        result.hit_rolls[0],
        vec![9, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 12]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 9]
    );

    defender.item = Item::None;
    defender.ability = Ability::Multiscale;
    let mut tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    tackle.hits = 2;
    let result = calc(attacker.clone(), defender.clone(), tackle, Field::default());
    assert_eq!(
        result.hit_rolls[0],
        vec![12, 12, 12, 12, 12, 12, 12, 12, 12, 12, 13, 13, 13, 13, 13, 14]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![24, 24, 24, 24, 24, 25, 25, 25, 25, 25, 27, 27, 27, 27, 27, 28]
    );

    let fire_attacker = stat_100_mon("Attacker", PokemonType::Fire);
    let mut grass_defender = stat_100_mon("Defender", PokemonType::Grass);
    grass_defender.item = Item::OccaBerry;
    let mut flame = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    flame.hits = 2;
    let result = calc(fire_attacker, grass_defender, flame, Field::default());
    assert_eq!(
        result.hit_rolls[0],
        vec![51, 52, 52, 54, 54, 54, 55, 55, 57, 57, 57, 58, 58, 60, 60, 61]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![102, 104, 104, 108, 108, 108, 110, 110, 114, 114, 114, 116, 116, 120, 120, 122]
    );

    let mut maranga_defender = stat_100_mon("Defender", PokemonType::Psychic);
    maranga_defender.item = Item::MarangaBerry;
    let mut special_multi = Move::new("Double Beam", 15, PokemonType::Normal, Category::Special);
    special_multi.hits = 2;
    let result = calc(attacker, maranga_defender, special_multi, Field::default());
    assert_eq!(
        result.hit_rolls[0],
        vec![9, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 12]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 9]
    );
}

#[test]
fn gooey_tangling_hair_and_cotton_down_recalculate_between_hits_like_js() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Normal);
    attacker.ability = Ability::Defiant;
    let mut defender = stat_100_mon("Defender", PokemonType::Psychic);
    defender.ability = Ability::Gooey;
    let mut double_slap = Move::new("Double Slap", 15, PokemonType::Normal, Category::Physical);
    double_slap.hits = 2;
    double_slap.makes_contact = true;

    let result = calc(
        attacker.clone(),
        defender.clone(),
        double_slap.clone(),
        Field::default(),
    );
    assert_eq!(
        result.hit_rolls[0],
        vec![9, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 12]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![18, 18, 19, 19, 19, 19, 19, 19, 19, 21, 21, 21, 21, 21, 21, 22]
    );

    defender.ability = Ability::TanglingHair;
    let result = calc(attacker, defender, double_slap, Field::default());
    assert_eq!(
        result.hit_rolls[1],
        vec![18, 18, 19, 19, 19, 19, 19, 19, 19, 21, 21, 21, 21, 21, 21, 22]
    );

    let mut long_reach_attacker = stat_100_mon("Long Reach", PokemonType::Normal);
    long_reach_attacker.ability = Ability::LongReach;
    let mut gooey_defender = stat_100_mon("Defender", PokemonType::Psychic);
    gooey_defender.ability = Ability::Gooey;
    let mut contact_double_slap =
        Move::new("Double Slap", 15, PokemonType::Normal, Category::Physical);
    contact_double_slap.hits = 2;
    contact_double_slap.makes_contact = true;
    let result = calc(
        long_reach_attacker,
        gooey_defender.clone(),
        contact_double_slap.clone(),
        Field::default(),
    );
    assert_eq!(result.hit_rolls[0], result.hit_rolls[1]);

    let mut padded_attacker = stat_100_mon("Pads", PokemonType::Normal);
    padded_attacker.item = Item::ProtectivePads;
    let result = calc(
        padded_attacker,
        gooey_defender.clone(),
        contact_double_slap.clone(),
        Field::default(),
    );
    assert_eq!(result.hit_rolls[0], result.hit_rolls[1]);

    let mut gloved_attacker = stat_100_mon("Glove", PokemonType::Normal);
    gloved_attacker.item = Item::PunchingGlove;
    let mut punch = contact_double_slap;
    punch.is_punch = true;
    let result = calc(gloved_attacker, gooey_defender, punch, Field::default());
    assert_eq!(result.hit_rolls[0], result.hit_rolls[1]);

    let fast = stat_100_mon("Fast", PokemonType::Electric);
    let mut cotton = stat_100_mon("Cotton", PokemonType::Water);
    cotton.ability = Ability::CottonDown;
    cotton.boosts.speed = -2;
    let mut electro_ball = Move::new("Electro Ball", 1, PokemonType::Electric, Category::Special);
    electro_ball.hits = 2;
    let result = calc(fast, cotton, electro_ball, Field::default());
    assert_eq!(
        result.hit_rolls[0],
        vec![92, 92, 96, 96, 96, 98, 98, 102, 102, 102, 104, 104, 104, 108, 108, 110]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![68, 72, 72, 72, 72, 74, 74, 74, 78, 78, 78, 78, 80, 80, 80, 84]
    );
}

#[test]
fn spicy_spray_recalculates_burn_and_burn_heal_between_hits_like_js() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let mut defender = stat_100_mon("Defender", PokemonType::Psychic);
    defender.ability = Ability::SpicySpray;
    let mut double_slap = Move::new("Double Slap", 15, PokemonType::Normal, Category::Physical);
    double_slap.hits = 2;

    let result = calc(
        attacker.clone(),
        defender.clone(),
        double_slap.clone(),
        Field::default(),
    );
    assert_eq!(
        result.hit_rolls[0],
        vec![9, 9, 9, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 12]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6]
    );

    let mut rawst_attacker = attacker.clone();
    rawst_attacker.item = Item::RawstBerry;
    let mut triple_slap = double_slap.clone();
    triple_slap.hits = 3;
    let result = calc(
        rawst_attacker,
        defender.clone(),
        triple_slap,
        Field::default(),
    );
    assert_eq!(result.hit_rolls[0], result.hit_rolls[1]);
    assert_eq!(
        result.hit_rolls[2],
        vec![4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6]
    );

    let mut flare = stat_100_mon("Flare", PokemonType::Normal);
    flare.ability = Ability::FlareBoost;
    let mut double_beam = Move::new("Double Beam", 15, PokemonType::Psychic, Category::Special);
    double_beam.hits = 2;
    let result = calc(flare, defender.clone(), double_beam, Field::default());
    assert_eq!(
        result.hit_rolls[0],
        vec![3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4]
    );
    assert_eq!(
        result.hit_rolls[1],
        vec![4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5]
    );

    let fire_attacker = stat_100_mon("Fire", PokemonType::Fire);
    let result = calc(
        fire_attacker,
        defender.clone(),
        double_slap.clone(),
        Field::default(),
    );
    assert_eq!(result.hit_rolls[0], result.hit_rolls[1]);

    let mut misty = Field::default();
    misty.terrain = Terrain::Misty;
    let result = calc(attacker, defender, double_slap, misty);
    assert_eq!(result.hit_rolls[0], result.hit_rolls[1]);
}

#[test]
fn entry_preprocessing_seeds_intimidate_and_download_match_js_order() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let mut seed_defender = stat_100_mon("Defender", PokemonType::Psychic);
    seed_defender.item = Item::ElectricSeed;
    let tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let mut electric_field = Field::default();
    electric_field.terrain = Terrain::Electric;
    let result = calc(
        attacker.clone(),
        seed_defender,
        tackle.clone(),
        electric_field,
    );
    assert_eq!(
        result.damage_rolls,
        vec![16, 16, 16, 16, 16, 16, 16, 16, 18, 18, 18, 18, 18, 18, 18, 19]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "terrain seed"));

    let mut intimidator = stat_100_mon("Intimidator", PokemonType::Psychic);
    intimidator.ability = Ability::Intimidate;
    intimidator.ability_on = true;
    let result = calc(
        attacker.clone(),
        intimidator,
        tackle.clone(),
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![16, 16, 16, 16, 16, 16, 16, 16, 18, 18, 18, 18, 18, 18, 18, 19]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Intimidate"));

    let mut download = stat_100_mon("Download", PokemonType::Electric);
    download.ability = Ability::Download;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let thunderbolt = Move::new("Thunderbolt", 90, PokemonType::Electric, Category::Special);
    let result = calc(download, defender, thunderbolt, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![152, 156, 158, 158, 162, 162, 164, 168, 168, 170, 170, 174, 176, 176, 180, 182]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Download"));
}

#[test]
fn entry_preprocessing_evo_clangorous_and_weakness_policy_toggles_match_js() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let psychic = Move::new("Psychic", 90, PokemonType::Psychic, Category::Special);

    let mut evo_field = Field::default();
    evo_field.attacker_evo_boost = true;
    let result = calc(
        attacker.clone(),
        defender.clone(),
        tackle.clone(),
        evo_field,
    );
    assert_eq!(
        result.damage_rolls,
        vec![46, 46, 48, 48, 48, 49, 49, 51, 51, 51, 52, 52, 52, 54, 54, 55]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "attacker Evo/Tatsugiri boost"));

    let mut defender_evo_field = Field::default();
    defender_evo_field.defender_evo_boost = true;
    let result = calc(
        attacker.clone(),
        defender.clone(),
        tackle.clone(),
        defender_evo_field,
    );
    assert_eq!(
        result.damage_rolls,
        vec![12, 12, 12, 12, 12, 13, 13, 13, 13, 13, 13, 13, 13, 13, 13, 15]
    );

    let mut clang_field = Field::default();
    clang_field.attacker_clangorous_soul = true;
    let result = calc(
        attacker.clone(),
        defender.clone(),
        psychic.clone(),
        clang_field,
    );
    assert_eq!(
        result.damage_rolls,
        vec![34, 34, 35, 35, 36, 36, 36, 37, 37, 38, 38, 38, 39, 39, 40, 40]
    );

    let mut weakness_policy_field = Field::default();
    weakness_policy_field.attacker_weakness_policy = true;
    let result = calc(attacker, defender, psychic, weakness_policy_field);
    assert_eq!(
        result.damage_rolls,
        vec![34, 34, 35, 35, 36, 36, 36, 37, 37, 38, 38, 38, 39, 39, 40, 40]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "attacker Weakness Policy"));
}

#[test]
fn trace_neutralizing_gas_klutz_and_entry_boosts_match_js_preprocessing() {
    let mut tracer = stat_100_mon("Tracer", PokemonType::Electric);
    tracer.ability = Ability::Trace;
    tracer.ability_on = true;
    let mut target = stat_100_mon("Target", PokemonType::Water);
    target.ability = Ability::ThickFat;
    let fire = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let result = calc(tracer, target, fire, Field::default());
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Trace"));
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "defensive attack reduction"));

    let attacker = stat_100_mon("Attacker", PokemonType::Fire);
    let mut heatproof = stat_100_mon("Heatproof", PokemonType::Steel);
    heatproof.ability = Ability::Heatproof;
    let result = calc(
        attacker,
        heatproof,
        Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special),
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![50, 54, 54, 54, 54, 54, 56, 56, 56, 56, 56, 60, 60, 60, 60, 62]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "defensive attack reduction"));

    let mut transistor = stat_100_mon("Attacker", PokemonType::Electric);
    transistor.ability = Ability::Transistor;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let thunderbolt = Move::new("Thunderbolt", 90, PokemonType::Electric, Category::Special);
    let mut gas_field = Field::default();
    gas_field.neutralizing_gas = true;
    let result = calc(transistor, defender, thunderbolt, gas_field);
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Neutralizing Gas"));
    assert!(!result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Transistor"));

    let mut klutz = stat_100_mon("Klutz", PokemonType::Electric);
    klutz.ability = Ability::Klutz;
    klutz.item = Item::Magnet;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let result = calc(
        klutz,
        defender,
        Move::new("Thunderbolt", 90, PokemonType::Electric, Category::Special),
        Field::default(),
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Klutz"));
    assert!(!result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "type item"));

    let mut ogerpon = stat_100_mon("Ogerpon-Hearthflame", PokemonType::Grass);
    ogerpon.ability = Ability::EmbodyAspect;
    ogerpon.item = Item::HearthflameMask;
    let defender = stat_100_mon("Defender", PokemonType::Grass);
    let cudgel = Move::new("Ivy Cudgel", 100, PokemonType::Grass, Category::Physical);
    let result = calc(ogerpon, defender, cudgel, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![114, 116, 118, 118, 120, 122, 122, 124, 126, 126, 128, 130, 130, 132, 134, 136]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Embody Aspect"));
}

#[test]
fn paradox_abilities_apply_highest_stat_modifiers_like_js() {
    let mut attacker = stat_100_mon("Great Tusk", PokemonType::Normal);
    attacker.ability = Ability::Protosynthesis;
    attacker.highest_stat_override = Some(Stat::Attack);
    let defender = stat_100_mon("Defender", PokemonType::Psychic);
    let tackle = Move::new("Tackle", 40, PokemonType::Normal, Category::Physical);
    let mut sun = Field::default();
    sun.weather = Weather::Sun;
    let result = calc(attacker, defender.clone(), tackle.clone(), sun);
    assert_eq!(
        result.damage_rolls,
        vec![30, 30, 30, 31, 31, 31, 31, 33, 33, 33, 33, 34, 34, 34, 34, 36]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Paradox ability attack"));

    let mut defender_boosted = stat_100_mon("Iron Defense", PokemonType::Psychic);
    defender_boosted.ability = Ability::QuarkDrive;
    defender_boosted.item = Item::BoosterEnergy;
    defender_boosted.highest_stat_override = Some(Stat::Defense);
    let result = calc(
        stat_100_mon("Attacker", PokemonType::Normal),
        defender_boosted,
        tackle,
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![18, 18, 19, 19, 19, 19, 19, 19, 19, 21, 21, 21, 21, 21, 21, 22]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Paradox ability defense"));
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Booster Energy"));

    let mut speedster = stat_100_mon("Iron Speed", PokemonType::Electric);
    speedster.ability = Ability::QuarkDrive;
    speedster.highest_stat_override = Some(Stat::Speed);
    let mut slow_defender = defender;
    slow_defender.boosts.speed = -1;
    let mut electric = Field::default();
    electric.terrain = Terrain::Electric;
    let electro_ball = Move::new("Electro Ball", 1, PokemonType::Electric, Category::Special);
    let result = calc(speedster, slow_defender, electro_ball, electric);
    assert_eq!(
        result.damage_rolls,
        vec![58, 60, 60, 61, 61, 63, 63, 64, 64, 66, 66, 67, 67, 69, 69, 70]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Paradox ability"));
}

#[test]
fn champions_item_json_names_align_with_typed_item_variants() {
    let items: Vec<String> =
        serde_json::from_str(include_str!("../data/champions/items.json")).expect("items JSON");
    let variants = [
        Item::SilkScarf,
        Item::MiracleSeed,
        Item::Charcoal,
        Item::MysticWater,
        Item::Magnet,
        Item::SilverPowder,
        Item::SharpBeak,
        Item::HardStone,
        Item::PoisonBarb,
        Item::SoftSand,
        Item::NeverMeltIce,
        Item::BlackBelt,
        Item::TwistedSpoon,
        Item::SpellTag,
        Item::DragonFang,
        Item::BlackGlasses,
        Item::MetalCoat,
        Item::FairyFeather,
        Item::FocusSash,
        Item::Leftovers,
        Item::MentalHerb,
        Item::ShellBell,
        Item::CheriBerry,
        Item::ChestoBerry,
        Item::PechaBerry,
        Item::RawstBerry,
        Item::AspearBerry,
        Item::PersimBerry,
        Item::LeppaBerry,
        Item::OranBerry,
        Item::ChilanBerry,
        Item::RindoBerry,
        Item::OccaBerry,
        Item::PasshoBerry,
        Item::WacanBerry,
        Item::TangaBerry,
        Item::CobaBerry,
        Item::ChartiBerry,
        Item::KebiaBerry,
        Item::ShucaBerry,
        Item::YacheBerry,
        Item::ChopleBerry,
        Item::PayapaBerry,
        Item::KasibBerry,
        Item::HabanBerry,
        Item::ColburBerry,
        Item::BabiriBerry,
        Item::RoseliBerry,
        Item::ScopeLens,
        Item::LightBall,
        Item::Venusaurite,
        Item::CharizarditeX,
        Item::CharizarditeY,
        Item::Blastoisinite,
        Item::Pidgeotite,
        Item::Clefablite,
        Item::Alakazite,
        Item::Victreebelite,
        Item::Slowbronite,
        Item::Gengarite,
        Item::Kangaskhanite,
        Item::Starminite,
        Item::Pinsirite,
        Item::Aerodactylite,
        Item::Dragoninite,
        Item::Meganiumite,
        Item::Feraligite,
        Item::Ampharosite,
        Item::Scizorite,
        Item::Skarmorite,
        Item::Houndoominite,
        Item::Tyranitarite,
        Item::Gardevoirite,
        Item::Sablenite,
        Item::Medichamite,
        Item::Sharpedonite,
        Item::Cameruptite,
        Item::Altarianite,
        Item::Banettite,
        Item::Chimechite,
        Item::Absolite,
        Item::Glalitite,
        Item::Lopunnite,
        Item::Lucarionite,
        Item::Galladite,
        Item::Froslassite,
        Item::Emboarite,
        Item::Excadrite,
        Item::Audinite,
        Item::Chandelurite,
        Item::Golurkite,
        Item::Meowsticite,
        Item::Hawluchanite,
        Item::Crabominite,
        Item::Drampanite,
        Item::Scovillainite,
        Item::Glimmoranite,
        Item::LumBerry,
        Item::SitrusBerry,
        Item::WhiteHerb,
        Item::QuickClaw,
        Item::KingsRock,
        Item::FocusBand,
        Item::ChoiceScarf,
        Item::BrightPowder,
        Item::Aggronite,
        Item::Abomasite,
        Item::Gyaradosite,
        Item::Heracronite,
        Item::Manectite,
        Item::Garchompite,
        Item::Steelixite,
        Item::Beedrillite,
        Item::Chesnaughtite,
        Item::Delphoxite,
        Item::Greninjite,
        Item::Floettite,
        Item::BigRoot,
        Item::DampRock,
        Item::ExpertBelt,
        Item::HeatRock,
        Item::IcyRock,
        Item::IronBall,
        Item::LifeOrb,
        Item::LightClay,
        Item::Metronome,
        Item::MuscleBand,
        Item::ShedShell,
        Item::SmoothRock,
        Item::WideLens,
        Item::WiseGlasses,
        Item::ZoomLens,
        Item::RaichuniteX,
        Item::RaichuniteY,
        Item::Falinksite,
        Item::Staraptite,
        Item::Blazikenite,
        Item::Mawilite,
        Item::Swampertite,
        Item::Sceptilite,
        Item::Metagrossite,
        Item::Scolipite,
        Item::Scraftinite,
        Item::Eelektrossite,
        Item::Pyroarite,
        Item::Malamarite,
        Item::Barbaracite,
        Item::Dragalgite,
        Item::Leek,
        Item::RockyHelmet,
        Item::AirBalloon,
        Item::RedCard,
        Item::BindingBand,
        Item::EjectButton,
        Item::NormalGem,
        Item::TerrainExtender,
        Item::ElectricSeed,
        Item::PsychicSeed,
        Item::MistySeed,
        Item::GrassySeed,
        Item::AbsoliteZ,
        Item::Salamencite,
        Item::GarchompiteZ,
        Item::LucarioniteZ,
        Item::Golisopite,
        Item::Baxcalibrite,
    ];
    let variant_names = variants
        .into_iter()
        .map(|item| normalized_identifier(&format!("{item:?}")))
        .collect::<HashSet<_>>();

    let missing = items
        .iter()
        .filter(|item| !variant_names.contains(&normalized_identifier(item)))
        .collect::<Vec<_>>();
    assert!(missing.is_empty(), "missing item variants: {missing:?}");
}

#[test]
fn champions_regulation_m_a_roster_json_is_valid_and_unique() {
    let pokemon: Vec<String> = serde_json::from_str(include_str!(
        "../data/champions/regulation_m_a_pokemon.json"
    ))
    .expect("roster JSON");
    let unique = pokemon.iter().map(String::as_str).collect::<HashSet<_>>();

    assert_eq!(pokemon.len(), 209);
    assert_eq!(unique.len(), pokemon.len());
    assert!(unique.contains("Venusaur"));
    assert!(unique.contains("Hydrapple"));
}

#[test]
fn vendored_champout_json_sources_are_valid_and_pinned() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../data/champions/champout/meta/source.json"))
            .expect("champout source manifest");
    let files = manifest["files"].as_array().expect("manifest files");
    assert_eq!(files.len(), 9);

    for file in files {
        let relative_path = file["path"].as_str().expect("path");
        let expected_hash = file["sha256"].as_str().expect("sha256");
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data/champions/champout")
            .join(relative_path);
        let bytes = fs::read(&path).expect("vendored champout file");
        serde_json::from_slice::<serde_json::Value>(&bytes).expect("valid champout JSON");
        assert_eq!(sha256_hex(&bytes), expected_hash);
    }
}

#[test]
fn normalized_champions_data_is_generated_from_champout() {
    let data: serde_json::Value =
        serde_json::from_str(damage_calc::data::CHAMPIONS_DATA_JSON).expect("generated data JSON");

    assert_eq!(data["schemaVersion"], 1);
    assert_eq!(data["counts"]["species"], 396);
    assert_eq!(data["counts"]["regulationMAForms"], 321);
    assert_eq!(data["counts"]["regulationMARosterNames"], 209);
    assert_eq!(data["counts"]["regulationMBForms"], 359);
    assert_eq!(data["counts"]["regulationMBRosterNames"], 247);
    assert_eq!(data["counts"]["moves"], 920);
    assert_eq!(data["counts"]["abilities"], 217);

    let species = data["species"].as_array().expect("species array");
    let venusaur = species
        .iter()
        .find(|entry| entry["displayName"] == "Venusaur")
        .expect("Venusaur");
    assert_eq!(venusaur["types"], serde_json::json!(["Grass", "Poison"]));
    assert_eq!(venusaur["baseStats"]["specialAttack"], 100);
    assert!(venusaur["legalMoves"]
        .as_array()
        .expect("legal moves")
        .iter()
        .any(|move_| move_["name"] == "Knock Off"));

    let moves = data["moves"].as_array().expect("moves array");
    let pound = moves.iter().find(|entry| entry["id"] == 1).expect("Pound");
    assert_eq!(pound["name"], "Pound");
    assert_eq!(pound["type"], "Normal");
    assert_eq!(pound["category"], "Physical");

    let abilities = data["abilities"].as_array().expect("abilities array");
    let overgrow = abilities
        .iter()
        .find(|entry| entry["id"] == 65)
        .expect("Overgrow");
    assert_eq!(overgrow["name"], "Overgrow");
}

#[test]
fn normalized_champions_data_covers_regulation_m_a_roster_names() {
    let roster: Vec<String> = serde_json::from_str(include_str!(
        "../data/champions/regulation_m_a_pokemon.json"
    ))
    .expect("roster JSON");
    let data: serde_json::Value =
        serde_json::from_str(damage_calc::data::CHAMPIONS_DATA_JSON).expect("generated data JSON");
    let species = data["species"].as_array().expect("species array");
    let display_names = species
        .iter()
        .map(|entry| entry["displayName"].as_str().expect("displayName"))
        .collect::<HashSet<_>>();
    let base_names = species
        .iter()
        .map(|entry| entry["name"].as_str().expect("name"))
        .collect::<HashSet<_>>();

    let missing = roster
        .iter()
        .filter(|name| {
            !display_names.contains(name.as_str()) && !base_names.contains(name.as_str())
        })
        .collect::<Vec<_>>();
    assert!(missing.is_empty(), "missing roster names: {missing:?}");
}

#[test]
fn public_champions_lists_match_vendored_json_sources() {
    let items: Vec<String> =
        serde_json::from_str(damage_calc::data::champions::CHAMPIONS_ITEMS_JSON)
            .expect("items JSON");
    let roster: Vec<String> =
        serde_json::from_str(damage_calc::data::champions::REGULATION_M_A_POKEMON_JSON)
            .expect("roster JSON");
    let m_b_additions: serde_json::Value =
        serde_json::from_str(damage_calc::data::champions::REGULATION_M_B_ADDITIONS_JSON)
            .expect("Regulation M-B additions JSON");
    let m_b_addition_count = m_b_additions["regular"]
        .as_array()
        .expect("regular M-B additions")
        .len()
        + m_b_additions["mega"]
            .as_array()
            .expect("mega M-B additions")
            .len();
    let data: serde_json::Value =
        serde_json::from_str(damage_calc::data::CHAMPIONS_DATA_JSON).expect("generated data JSON");

    assert_eq!(
        damage_calc::data::champions::CHAMPIONS_ITEMS.len(),
        items.len()
    );
    assert_eq!(
        damage_calc::data::champions::REGULATION_M_A_POKEMON.len(),
        roster.len()
    );
    assert_eq!(
        damage_calc::data::champions::REGULATION_M_B_POKEMON.len(),
        roster.len() + m_b_addition_count
    );
    assert_eq!(
        damage_calc::data::champions::CHAMPIONS_SPECIES.len(),
        data["counts"]["species"].as_u64().expect("species count") as usize
    );
    assert_eq!(
        damage_calc::data::champions::CHAMPIONS_ABILITIES.len(),
        data["counts"]["abilities"]
            .as_u64()
            .expect("abilities count") as usize
    );

    assert_eq!(
        damage_calc::data::champions::champions_item("Light Ball"),
        Some("Light Ball")
    );
    assert_eq!(
        damage_calc::data::champions::regulation_m_a_pokemon("Hydrapple"),
        Some("Hydrapple")
    );
    assert_eq!(
        damage_calc::data::champions::regulation_m_b_pokemon("Mega Raichu X"),
        Some("Mega Raichu X")
    );
    assert!(
        !damage_calc::data::champions::champions_species("Mega Raichu X")
            .expect("Mega Raichu X")
            .is_regulation_m_a
    );
    assert!(
        damage_calc::data::champions::champions_species("Mega Raichu X")
            .expect("Mega Raichu X")
            .is_regulation_m_b
    );
    assert_eq!(
        damage_calc::data::champions::champions_species("Mega Venusaur")
            .expect("Mega Venusaur")
            .id,
        "0003001"
    );
    assert_eq!(
        damage_calc::data::champions::champions_ability("Overgrow")
            .expect("Overgrow")
            .id,
        65
    );
}

#[test]
fn pinned_reference_inventory_is_complete_and_battle_ready() {
    use damage_calc::data::champions::{
        CHAMPIONS_REFERENCE_ABILITIES, CHAMPIONS_REFERENCE_MOVES, CHAMPIONS_REFERENCE_SETS,
        CHAMPIONS_REFERENCE_SPECIES,
    };

    assert_eq!(CHAMPIONS_REFERENCE_SPECIES.len(), 315);
    assert_eq!(CHAMPIONS_REFERENCE_MOVES.len(), 496);
    assert_eq!(
        damage_calc::data::champions::CHAMPIONS_REFERENCE_ITEM_VALUES.len(),
        148
    );
    assert_eq!(CHAMPIONS_REFERENCE_ABILITIES.len(), 201);
    assert_eq!(CHAMPIONS_REFERENCE_SETS.len(), 123);

    for set in CHAMPIONS_REFERENCE_SETS {
        let pokemon = set
            .pokemon_()
            .unwrap_or_else(|| panic!("missing species for {} ({})", set.pokemon, set.name));
        assert_eq!(pokemon.name, set.pokemon);
        let moves = set
            .moves_()
            .unwrap_or_else(|| panic!("missing move for {} ({})", set.pokemon, set.name));
        assert_eq!(moves.len(), 4);
    }
}

#[test]
fn every_pinned_reference_set_move_calculates_without_error() {
    use damage_calc::data::champions::CHAMPIONS_REFERENCE_SETS;

    for set in CHAMPIONS_REFERENCE_SETS {
        let attacker = set.pokemon_().expect("set Pokemon");
        let defender = set.pokemon_().expect("set Pokemon");
        for move_ in set.moves_().expect("set moves") {
            let result = calculate_damage(CalcInput {
                attacker: attacker.clone(),
                defender: defender.clone(),
                move_,
                field: Field::default(),
                ruleset: Ruleset::Champions,
            })
            .unwrap_or_else(|error| panic!("{} ({}): {error}", set.pokemon, set.name));
            assert_eq!(
                result.damage_rolls.first().copied(),
                Some(result.min_damage)
            );
            assert_eq!(result.damage_rolls.last().copied(), Some(result.max_damage));
            assert!(result
                .damage_rolls
                .windows(2)
                .all(|pair| pair[0] <= pair[1]));
            assert_eq!(result.ko_chance_by_move_use.len(), 4);
            assert!(result
                .ko_chance_by_move_use
                .iter()
                .all(|chance| (0.0..=1.0).contains(chance)));
            assert!(result
                .ko_chance_by_move_use
                .windows(2)
                .all(|pair| pair[0] <= pair[1]));
            assert!(result.resolved_move.is_some());
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn public_batch_schema_round_trips_with_serde() {
    let pokemon = stat_100_mon("Pokemon", PokemonType::Normal);
    let no_move = Move::new("(No Move)", 0, PokemonType::Typeless, Category::Status);
    let input = BatchCalcInput {
        left: pokemon.clone(),
        right: pokemon,
        left_moves: std::array::from_fn(|_| no_move.clone()),
        right_moves: std::array::from_fn(|_| no_move.clone()),
        left_to_right_field: Field::default(),
        right_to_left_field: Field::default(),
        ruleset: Ruleset::Champions,
    };
    let json = serde_json::to_string(&input).expect("serialize batch input");
    let round_trip: BatchCalcInput = serde_json::from_str(&json).expect("deserialize batch input");
    assert_eq!(round_trip, input);

    let result = calculate_all_moves(input).expect("batch calculation");
    let json = serde_json::to_string(&result).expect("serialize batch result");
    let round_trip: damage_calc::BatchDamageResult =
        serde_json::from_str(&json).expect("deserialize batch result");
    assert_eq!(round_trip, result);
}

#[test]
fn duplicate_monotype_entries_are_treated_as_single_type() {
    let mut kingambit = Pokemon::champions(
        "Kingambit",
        [Some(PokemonType::Dark), Some(PokemonType::Steel)],
        StatTable::new(100, 135, 120, 60, 85, 50),
        StatTable::new(0, 32, 0, 0, 0, 0),
        Nature::Adamant,
    );
    kingambit.ability = Ability::Defiant;
    kingambit.weight_kg = 120.0;

    let mut floette = Pokemon::champions(
        "Mega Floette",
        [Some(PokemonType::Fairy), Some(PokemonType::Fairy)],
        StatTable::new(74, 85, 87, 155, 148, 102),
        StatTable::new(32, 0, 32, 0, 0, 0),
        Nature::Hardy,
    );
    floette.weight_kg = 100.8;

    let result = calc(
        kingambit,
        floette,
        Move::new("Iron Head", 80, PokemonType::Steel, Category::Physical),
        Field::default(),
    );
    assert_eq!(
        result.damage_rolls,
        vec![134, 134, 138, 138, 140, 140, 144, 144, 146, 146, 150, 150, 152, 152, 156, 158]
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::process::{Command, Stdio};

    let mut child = Command::new("shasum")
        .args(["-a", "256"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("shasum available");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(bytes)
        .expect("write bytes");
    let output = child.wait_with_output().expect("shasum output");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("utf8 hash")
        .split_whitespace()
        .next()
        .expect("hash")
        .to_string()
}

#[test]
fn champions_missing_item_mechanics_match_js() {
    let mut pikachu = stat_100_mon("Pikachu", PokemonType::Electric);
    pikachu.item = Item::LightBall;
    let defender = stat_100_mon("Defender", PokemonType::Water);
    let thunderbolt = Move::new("Thunderbolt", 90, PokemonType::Electric, Category::Special);
    let result = calc(pikachu, defender.clone(), thunderbolt, Field::default());
    assert_eq!(
        result.damage_rolls,
        vec![204, 206, 210, 212, 216, 216, 218, 222, 224, 228, 228, 230, 234, 236, 240, 242]
    );
    assert!(result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "attack/item 2.0"));

    let mut charizard = stat_100_mon("Charizard", PokemonType::Fire);
    charizard.item = Item::CharizarditeX;
    let fling = Move::new("Fling", 1, PokemonType::Dark, Category::Physical);
    assert_eq!(
        calc(charizard, defender.clone(), fling, Field::default()).damage_rolls,
        vec![0]
    );

    let attacker = stat_100_mon("Attacker", PokemonType::Dark);
    let mut mega_gengar = stat_100_mon("Mega Gengar", PokemonType::Ghost);
    mega_gengar.item = Item::Gengarite;
    let knock = Move::new("Knock Off", 65, PokemonType::Dark, Category::Physical);
    let result = calc(attacker, mega_gengar, knock, Field::default());
    assert!(!result
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "Knock Off"));
}

#[test]
fn switch_in_hazards_are_applied_before_ko_projection() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    attacker.level = 88;
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);

    let without_hazards = calc(
        attacker.clone(),
        defender.clone(),
        seismic_toss.clone(),
        Field::default(),
    );
    assert_eq!(without_hazards.ko_chance, Some(0.0));

    let mut rocks = Field::default();
    rocks.defender_side.stealth_rock = true;
    assert_eq!(
        calc(
            attacker.clone(),
            defender.clone(),
            seismic_toss.clone(),
            rocks
        )
        .ko_chance,
        Some(1.0)
    );

    let mut spikes = Field::default();
    spikes.defender_side.spikes = 1;
    assert_eq!(
        calc(attacker, defender, seismic_toss, spikes).ko_chance,
        Some(1.0)
    );
}

#[test]
fn salt_cure_and_initial_toxic_counter_affect_ko_projection() {
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);

    let mut salt_attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    salt_attacker.level = 94;
    let seismic_toss = Move::new("Seismic Toss", 1, PokemonType::Fighting, Category::Physical);
    let mut salt = Field::default();
    salt.defender_side.salt_cure = true;
    assert_eq!(
        calc(salt_attacker, defender.clone(), seismic_toss.clone(), salt).ko_chance,
        Some(1.0)
    );

    let mut toxic_attacker = stat_100_mon("Attacker", PokemonType::Fighting);
    toxic_attacker.level = 70;
    defender.status = StatusCondition::BadlyPoisoned;
    defender.toxic_counter = 5;
    assert_eq!(
        calc(toxic_attacker, defender, seismic_toss, Field::default()).ko_chance,
        Some(1.0)
    );
}

#[test]
fn psychic_noise_prevents_leftovers_recovery_in_ko_projection() {
    let mut attacker = stat_100_mon("Attacker", PokemonType::Dark);
    attacker.level = 50;
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    defender.max_hp_override = Some(100);
    defender.current_hp = Some(100);
    defender.item = Item::Leftovers;

    let ordinary = Move::new("Psychic", 130, PokemonType::Psychic, Category::Special);
    let ordinary_result = calc(
        attacker.clone(),
        defender.clone(),
        ordinary,
        Field::default(),
    );
    assert!(ordinary_result.ko_chance_by_move_use[1] < 1.0);

    let psychic_noise = Move::new(
        "Psychic Noise",
        130,
        PokemonType::Psychic,
        Category::Special,
    );
    let noise_result = calc(attacker, defender, psychic_noise, Field::default());
    assert_eq!(noise_result.ko_chance_by_move_use[1], 1.0);
}

#[test]
fn batch_calculation_matches_single_calls_and_shares_entry_weather() {
    let mut left = stat_100_mon("Left", PokemonType::Fire);
    left.ability = Ability::Drought;
    let right = stat_100_mon("Right", PokemonType::Water);
    let left_move = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    let right_move = Move::new("Surf", 90, PokemonType::Water, Category::Special);
    let no_move = Move::new("(No Move)", 0, PokemonType::Typeless, Category::Status);

    let batch = calculate_all_moves(BatchCalcInput {
        left: left.clone(),
        right: right.clone(),
        left_moves: [
            left_move.clone(),
            no_move.clone(),
            no_move.clone(),
            no_move.clone(),
        ],
        right_moves: [right_move, no_move.clone(), no_move.clone(), no_move],
        left_to_right_field: Field::default(),
        right_to_left_field: Field::default(),
        ruleset: Ruleset::Champions,
    })
    .expect("batch calculation");

    let single = calc(left, right, left_move, Field::default());
    assert_eq!(batch.left[0].damage_rolls, single.damage_rolls);
    assert!(batch.right[0]
        .applied_modifiers
        .iter()
        .any(|modifier| modifier.label == "weather damage drop"));
    assert_eq!(batch.left.len(), 4);
    assert_eq!(batch.right.len(), 4);
}

#[test]
fn result_reports_resolved_move_and_signed_pain_split_delta() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let defender = stat_100_mon("Defender", PokemonType::Normal);
    let body_slam = Move::new("Body Slam", 85, PokemonType::Normal, Category::Physical);
    let result = calc(
        attacker.clone(),
        defender.clone(),
        body_slam,
        Field::default(),
    );
    let resolved = result.resolved_move.expect("resolved move");
    assert_eq!(resolved.name, "Body Slam");
    assert_eq!(resolved.base_power, 85);
    assert_eq!(result.outcome, DamageOutcome::Damage);

    let pain_split = Move::new("Pain Split", 0, PokemonType::Normal, Category::Status);
    let mut low_attacker = attacker.clone();
    low_attacker.max_hp_override = Some(100);
    low_attacker.current_hp = Some(40);
    let mut full_defender = defender.clone();
    full_defender.max_hp_override = Some(100);
    full_defender.current_hp = Some(100);
    let damage = calc(
        low_attacker,
        full_defender,
        pain_split.clone(),
        Field::default(),
    );
    assert_eq!(damage.outcome, DamageOutcome::HpShare);
    assert_eq!(damage.defender_hp_delta, Some(30));
    assert_eq!(damage.damage_rolls, vec![30]);

    let mut full_attacker = attacker;
    full_attacker.max_hp_override = Some(100);
    full_attacker.current_hp = Some(100);
    let mut low_defender = defender;
    low_defender.max_hp_override = Some(100);
    low_defender.current_hp = Some(40);
    let healing = calc(full_attacker, low_defender, pain_split, Field::default());
    assert_eq!(healing.defender_hp_delta, Some(-30));
    assert_eq!(healing.damage_rolls, vec![0]);
}

#[test]
fn regulation_m_c_additions_have_complete_current_data() {
    use damage_calc::data::champions::*;
    let additions: serde_json::Value = serde_json::from_str(REGULATION_M_C_ADDITIONS_JSON).unwrap();
    let data: serde_json::Value =
        serde_json::from_str(damage_calc::data::CHAMPIONS_DATA_JSON).unwrap();
    assert_eq!(additions["regular"].as_array().unwrap().len(), 26);
    assert_eq!(additions["mega"].as_array().unwrap().len(), 6);
    assert_eq!(REGULATION_M_C_POKEMON.len(), 279);
    assert_eq!(
        REGULATION_M_C_POKEMON.iter().collect::<HashSet<_>>().len(),
        279
    );
    assert!(REGULATION_M_B_POKEMON
        .iter()
        .all(|name| regulation_m_c_pokemon(name).is_some()));
    for name in additions["regular"]
        .as_array()
        .unwrap()
        .iter()
        .chain(additions["mega"].as_array().unwrap())
    {
        let name = name.as_str().unwrap();
        assert_eq!(regulation_m_c_pokemon(name), Some(name));
        assert!(regulation_m_b_pokemon(name).is_none());
        let forms: Vec<_> = data["species"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["displayName"] == name || s["name"] == name)
            .collect();
        assert!(!forms.is_empty(), "missing {name}");
        for form in forms {
            assert_eq!(form["isRegulationMA"], false);
            assert_eq!(form["isRegulationMB"], false);
            assert_eq!(form["isRegulationMC"], true);
            assert!(!form["legalMoves"].as_array().unwrap().is_empty());
            let metadata =
                champions_current_species(form["displayName"].as_str().unwrap()).unwrap();
            assert!(metadata.weight_kg > 0.0);
            assert_ne!(metadata.default_ability, Ability::None);
        }
    }
    let lucario = champions_current_species("Mega Lucario Z").unwrap();
    assert_eq!(
        lucario.base_stats,
        StatTable::new(70, 100, 70, 164, 70, 151)
    );
    assert_eq!(lucario.default_ability, Ability::AuraGuard);
    assert_eq!(
        champions_current_species("Mega Golisopod").unwrap().types,
        [Some(PokemonType::Bug), Some(PokemonType::Steel)]
    );
    assert_eq!(
        champions_current_species("Mega Garchomp Z")
            .unwrap()
            .default_ability,
        Ability::Levitate
    );
    assert_ne!(
        champions_current_species("Indeedee (Male)")
            .unwrap()
            .base_stats,
        champions_current_species("Indeedee (Female)")
            .unwrap()
            .base_stats
    );
}

#[test]
fn regulation_m_c_items_and_mega_stones_are_typed() {
    use damage_calc::data::{champions::*, items::*};
    for name in [
        "Leek",
        "Rocky Helmet",
        "Air Balloon",
        "Red Card",
        "Binding Band",
        "Eject Button",
        "Normal Gem",
        "Terrain Extender",
        "Electric Seed",
        "Psychic Seed",
        "Misty Seed",
        "Grassy Seed",
    ] {
        assert_eq!(champions_item(name), Some(name));
        assert!(CHAMPIONS_ITEM_VALUES.iter().any(|(n, _)| *n == name));
    }
    for (mega, base, stone) in [
        ("Mega Absol Z", "Absol", Item::AbsoliteZ),
        ("Mega Salamence", "Salamence", Item::Salamencite),
        ("Mega Garchomp Z", "Garchomp", Item::GarchompiteZ),
        ("Mega Lucario Z", "Lucario", Item::LucarioniteZ),
        ("Mega Golisopod", "Golisopod", Item::Golisopite),
        ("Mega Baxcalibur", "Baxcalibur", Item::Baxcalibrite),
    ] {
        assert_eq!(locked_item_for_species(mega), Some(stone));
        assert!(can_mega(stone, base));
        assert!(!can_mega(stone, "Pikachu"));
        assert!(!can_fling(stone, base, Ability::None));
    }
    assert_eq!(fling_power(Item::Leek), Some(60));
    assert_eq!(fling_power(Item::RockyHelmet), Some(60));
    assert_eq!(fling_power(Item::BindingBand), Some(30));
    assert_eq!(fling_power(Item::TerrainExtender), Some(30));
    assert!(!can_fling(Item::NormalGem, "Persian", Ability::None));
}

#[test]
fn aura_guard_halves_contact_damage_and_can_be_bypassed() {
    let attacker = stat_100_mon("Attacker", PokemonType::Normal);
    let mut defender = stat_100_mon("Defender", PokemonType::Normal);
    let mut move_ = Move::new("Contact", 80, PokemonType::Normal, Category::Physical);
    move_.makes_contact = true;
    let baseline = calc(
        attacker.clone(),
        defender.clone(),
        move_.clone(),
        Field::default(),
    );
    defender.ability = Ability::AuraGuard;
    let guarded = calc(
        attacker.clone(),
        defender.clone(),
        move_.clone(),
        Field::default(),
    );
    assert!(guarded.max_damage < baseline.max_damage);
    for (reduced, normal) in guarded.damage_rolls.iter().zip(&baseline.damage_rolls) {
        assert_eq!(*reduced, normal / 2);
    }
    for ability in [Ability::MoldBreaker, Ability::LongReach] {
        let mut bypass = attacker.clone();
        bypass.ability = ability;
        assert_eq!(
            calc(bypass, defender.clone(), move_.clone(), Field::default()).damage_rolls,
            baseline.damage_rolls
        );
    }
    move_.makes_contact = false;
    assert_eq!(
        calc(attacker, defender, move_, Field::default()).damage_rolls,
        baseline.damage_rolls
    );
}
