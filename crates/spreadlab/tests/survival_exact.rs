use damage_calc::Nature;
use spreadlab_rs::{
    api::{find_min_survival_with_data, SurvivalRequest},
    damage_bridge::{calculate_benchmark, DamageBenchmark},
    data::ChampionsData,
    optimize::{hp_def_survival_search, offensive_ko_search, OffensiveInvestmentStat},
    showdown::{parse_nature_name, parse_set},
    spreads::LockedStats,
    stats::champions_final_stats,
    survival::{
        survival_natures, survival_search, ExactSurvivalSpread, SurvivalEvaluation,
        SurvivalResultMode, SurvivalSearchOptions,
    },
    StatPoints,
};
use std::collections::BTreeSet;

fn benchmark(attacker: &str, defender: &str, move_name: &str) -> DamageBenchmark {
    DamageBenchmark::new(
        parse_set(attacker).unwrap(),
        parse_set(defender).unwrap(),
        move_name,
    )
}

fn dragon() -> DamageBenchmark {
    benchmark(
        "Dragapult @ Life Orb\nSPs: 32 SpA\nHardy Nature",
        "Mega Salamence\nHardy Nature",
        "Dragon Pulse",
    )
}

fn key(spread: &ExactSurvivalSpread) -> (String, u16, u16, u16) {
    (
        format!("{:?}", spread.nature),
        spread.sps.hp,
        spread.sps.defense,
        spread.sps.special_defense,
    )
}

#[test]
fn dragon_pulse_global_minimum_beats_both_reported_spreads() {
    let data = ChampionsData::load().unwrap();
    let mut b = dragon();
    for (hp, spd, rolls) in [
        (
            26,
            0,
            vec![
                174, 174, 179, 179, 182, 182, 187, 187, 190, 190, 195, 195, 198, 198, 203, 205,
            ],
        ),
        (
            19,
            4,
            vec![
                166, 166, 172, 172, 174, 174, 179, 179, 182, 182, 187, 187, 190, 190, 195, 198,
            ],
        ),
    ] {
        b.defender.stat_points = StatPoints::new(hp, 0, 0, 0, spd, 0);
        let damage = calculate_benchmark(&data, &b).unwrap();
        assert_eq!(damage.damage_rolls, rolls);
        assert_eq!(damage.ko_chance, Some(0.25));
    }
    let result = hp_def_survival_search(&data, &b, &[Nature::Hardy], 0.25, 1).unwrap();
    let best = &result.matches[0];
    assert_eq!(best.sps, StatPoints::new(5, 0, 0, 0, 14, 0));
    assert_eq!(best.total_points, 19);
    assert_eq!(best.result.ko_chance, Some(0.1875));
    // Independent exhaustive oracle: count raw rolls, not the optimizer's KO helper.
    let mut minima = Vec::new();
    for hp in 0..=32 {
        for spd in 0..=32 {
            b.defender.stat_points = StatPoints::new(hp, 0, 0, 0, spd, 0);
            let damage = calculate_benchmark(&data, &b).unwrap();
            let hp_stat = champions_final_stats(
                data.species(&b.defender.species).unwrap().base_stats(),
                Nature::Hardy,
                b.defender.stat_points,
            )
            .unwrap()
            .hp;
            let kos = damage
                .damage_rolls
                .iter()
                .filter(|&&d| d >= hp_stat)
                .count();
            if kos <= 4 {
                minima.push((hp + spd, hp, spd));
            }
        }
    }
    minima.sort();
    assert_eq!(minima[0], (19, 5, 14));
    assert_eq!(minima.iter().filter(|x| x.0 == 19).count(), 1);
}

#[test]
fn independent_constraints_require_both_defenses() {
    let data = ChampionsData::load().unwrap();
    let benchmarks = [
        dragon(),
        benchmark(
            "Dragonite @ Life Orb\nSPs: 32 Atk\nAdamant Nature",
            "Mega Salamence\nHardy Nature",
            "Dragon Claw",
        ),
    ];
    let options = SurvivalSearchOptions {
        locked: LockedStats {
            hp: Some(0),
            ..Default::default()
        },
        result_mode: SurvivalResultMode::AllMinima,
        ..Default::default()
    };
    let result = survival_search(
        &data,
        &benchmarks,
        &[Nature::Hardy],
        &[0.25, 0.25],
        100.0,
        &options,
        &SurvivalEvaluation::Independent,
    )
    .unwrap();
    assert!(!result.matches.is_empty());
    for best in &result.matches {
        assert!(best.sps.defense > 0 && best.sps.special_defense > 0);
        assert!(best.ko_chances.iter().all(|&p| p <= 0.25));
    }
    let mut minimum = u16::MAX;
    for defense in 0..=32 {
        for spd in 0..=32 {
            let feasible = benchmarks.iter().all(|b| {
                let mut candidate = b.clone();
                candidate.defender.stat_points = StatPoints::new(0, 0, defense, 0, spd, 0);
                calculate_benchmark(&data, &candidate)
                    .unwrap()
                    .ko_chance
                    .unwrap()
                    <= 0.25
            });
            if feasible {
                minimum = minimum.min(defense + spd);
            }
        }
    }
    assert_eq!(result.minimum_total, Some(minimum));
}

#[test]
fn all_minima_matches_three_dimensional_oracle_across_contexts() {
    let data = ChampionsData::load().unwrap();
    let natures = [Nature::Hardy, Nature::Bold, Nature::Calm];
    for move_name in ["Dragon Pulse", "Dragon Claw", "Psyshock"] {
        let b = benchmark("Dragapult", "Mega Salamence", move_name);
        for threshold in [0.0, 0.25, 0.75] {
            let mut minimum = u16::MAX;
            let mut expected = BTreeSet::new();
            for &nature in &natures {
                for hp in 0..=8 {
                    for defense in 0..=8 - hp {
                        for spd in 0..=8 - hp - defense {
                            let sps = StatPoints::new(hp, 0, defense, 0, spd, 0);
                            let mut candidate = b.clone();
                            candidate.defender.nature = nature;
                            candidate.defender.stat_points = sps;
                            let stats = champions_final_stats(
                                data.species(&b.defender.species).unwrap().base_stats(),
                                nature,
                                sps,
                            )
                            .unwrap();
                            let starting_hp = (u32::from(stats.hp) * 60).div_ceil(100) as u16;
                            candidate.defender_current_hp = Some(starting_hp);
                            let result = calculate_benchmark(&data, &candidate).unwrap();
                            let probability = result
                                .damage_rolls
                                .iter()
                                .filter(|&&d| d >= starting_hp)
                                .count() as f32
                                / result.damage_rolls.len() as f32;
                            if probability <= threshold {
                                if sps.total() < minimum {
                                    minimum = sps.total();
                                    expected.clear();
                                }
                                if sps.total() == minimum {
                                    expected.insert((format!("{nature:?}"), hp, defense, spd));
                                }
                            }
                        }
                    }
                }
            }
            let options = SurvivalSearchOptions {
                max_total: 8,
                result_mode: SurvivalResultMode::AllMinima,
                ..Default::default()
            };
            let result = survival_search(
                &data,
                std::slice::from_ref(&b),
                &natures,
                &[threshold],
                60.0,
                &options,
                &SurvivalEvaluation::Independent,
            )
            .unwrap();
            assert_eq!(
                result.matches.iter().map(key).collect::<BTreeSet<_>>(),
                expected,
                "{move_name}, {threshold}"
            );
        }
    }
}

#[test]
fn preserves_other_stats_and_enforces_locks_lower_bounds_and_budget() {
    let data = ChampionsData::load().unwrap();
    let mut b = dragon();
    b.defender.stat_points = StatPoints::new(20, 32, 20, 0, 20, 15);
    let options = SurvivalSearchOptions {
        minimum: StatPoints::new(5, 0, 0, 0, 14, 0),
        locked: LockedStats {
            hp: Some(5),
            ..Default::default()
        },
        limit: 1,
        ..Default::default()
    };
    let result = survival_search(
        &data,
        std::slice::from_ref(&b),
        &[Nature::Hardy],
        &[0.25],
        100.0,
        &options,
        &SurvivalEvaluation::Independent,
    )
    .unwrap();
    assert_eq!(result.matches[0].sps, StatPoints::new(5, 32, 0, 0, 14, 15));
    assert_eq!(result.minimum_total, Some(66));
    let no_budget = SurvivalSearchOptions {
        max_total: 65,
        ..options.clone()
    };
    assert!(survival_search(
        &data,
        std::slice::from_ref(&b),
        &[Nature::Hardy],
        &[0.25],
        100.0,
        &no_budget,
        &SurvivalEvaluation::Independent
    )
    .unwrap()
    .matches
    .is_empty());
    let conflict = SurvivalSearchOptions {
        minimum: StatPoints::new(6, 0, 0, 0, 0, 0),
        ..options
    };
    assert!(survival_search(
        &data,
        &[b],
        &[Nature::Hardy],
        &[0.25],
        100.0,
        &conflict,
        &SurvivalEvaluation::Independent
    )
    .is_err());
}

#[test]
fn output_modes_and_closest_miss_have_distinct_contracts() {
    let data = ChampionsData::load().unwrap();
    let b = benchmark("Pikachu", "Mega Salamence", "Tackle");
    let options = SurvivalSearchOptions {
        max_total: 2,
        limit: 5,
        ..Default::default()
    };
    let run = |options: &SurvivalSearchOptions| {
        survival_search(
            &data,
            std::slice::from_ref(&b),
            &[Nature::Hardy, Nature::Hardy],
            &[0.0],
            100.0,
            options,
            &SurvivalEvaluation::Independent,
        )
        .unwrap()
    };
    let top = run(&options);
    assert_eq!(top.matches.len(), 5);
    assert!(top
        .matches
        .windows(2)
        .all(|w| w[0].total_points <= w[1].total_points));
    assert!(top.matches.iter().any(|x| x.total_points > 0));
    for mode in [SurvivalResultMode::AllMinima, SurvivalResultMode::Pareto] {
        let result = run(&SurvivalSearchOptions {
            result_mode: mode,
            limit: 0,
            ..options.clone()
        });
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].total_points, 0);
    }
    let d = dragon();
    let impossible = SurvivalSearchOptions {
        max_total: 0,
        ..Default::default()
    };
    let miss = survival_search(
        &data,
        &[d],
        &[Nature::Hardy],
        &[0.0],
        100.0,
        &impossible,
        &SurvivalEvaluation::Independent,
    )
    .unwrap();
    assert!(miss.matches.is_empty());
    assert!(miss.closest_miss.unwrap().ko_chances[0] > 0.0);
}

#[test]
fn invalid_inputs_are_rejected_without_silent_clamping() {
    let data = ChampionsData::load().unwrap();
    let b = dragon();
    let options = SurvivalSearchOptions::default();
    for p in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        assert!(survival_search(
            &data,
            std::slice::from_ref(&b),
            &[Nature::Hardy],
            &[p],
            100.0,
            &options,
            &SurvivalEvaluation::Independent
        )
        .is_err());
    }
    for hp in [f32::NAN, f32::INFINITY, -1.0, 0.0, 101.0] {
        assert!(survival_search(
            &data,
            std::slice::from_ref(&b),
            &[Nature::Hardy],
            &[0.25],
            hp,
            &options,
            &SurvivalEvaluation::Independent
        )
        .is_err());
    }
    for bad in [
        SurvivalSearchOptions {
            max_total: 67,
            ..options.clone()
        },
        SurvivalSearchOptions {
            locked: LockedStats {
                special_defense: Some(33),
                ..Default::default()
            },
            ..options.clone()
        },
        SurvivalSearchOptions {
            limit: 0,
            ..options.clone()
        },
    ] {
        assert!(survival_search(
            &data,
            std::slice::from_ref(&b),
            &[Nature::Hardy],
            &[0.25],
            100.0,
            &bad,
            &SurvivalEvaluation::Independent
        )
        .is_err());
    }
    assert!(survival_search(
        &data,
        &[],
        &[Nature::Hardy],
        &[],
        100.0,
        &options,
        &SurvivalEvaluation::Independent
    )
    .is_err());
    assert!(survival_search(
        &data,
        std::slice::from_ref(&b),
        &[],
        &[0.25],
        100.0,
        &options,
        &SurvivalEvaluation::Independent
    )
    .is_err());
    assert!(survival_search(
        &data,
        std::slice::from_ref(&b),
        &[Nature::Hardy],
        &[],
        100.0,
        &options,
        &SurvivalEvaluation::Independent
    )
    .is_err());
    assert!(survival_search(
        &data,
        std::slice::from_ref(&b),
        &[Nature::Hardy],
        &[0.25],
        100.0,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![true, true]
        }
    )
    .is_err());
    let other = benchmark("Dragapult", "Milotic", "Dragon Pulse");
    assert!(survival_search(
        &data,
        &[b, other],
        &[Nature::Hardy],
        &[0.25, 0.25],
        100.0,
        &options,
        &SurvivalEvaluation::Independent
    )
    .is_err());
}

#[test]
fn api_preserves_nature_and_accepts_explicit_allowed_natures() {
    let data = ChampionsData::load().unwrap();
    let request: SurvivalRequest = serde_json::from_value(serde_json::json!({
        "defender_set": "Mega Salamence\nJolly Nature",
        "hits": [{"attacker_set": "Pikachu", "move_name": "Tackle", "move_times_affected": 0}],
        "max_ko_chances": [0.0], "search": {"result_mode": "AllMinima"}
    }))
    .unwrap();
    let fixed = find_min_survival_with_data(&data, request.clone()).unwrap();
    assert_eq!(fixed.matches.len(), 1);
    assert_eq!(fixed.matches[0].nature, Nature::Jolly);
    let mut restricted = request.clone();
    restricted.search.allowed_natures = Some(vec![Nature::Calm, Nature::Bold, Nature::Calm]);
    assert_eq!(
        find_min_survival_with_data(&data, restricted)
            .unwrap()
            .matches
            .len(),
        2
    );
    let mut optimized = request.clone();
    optimized.optimize_nature = true;
    let optimized = find_min_survival_with_data(&data, optimized).unwrap();
    assert_eq!(optimized.matches.len(), 21);
    assert!(optimized.matches.iter().all(|spread| !matches!(
        spread.nature,
        Nature::Bashful | Nature::Docile | Nature::Serious | Nature::Quirky
    )));
    let mut empty = request;
    empty.search.allowed_natures = Some(vec![]);
    assert!(find_min_survival_with_data(&data, empty).is_err());
}

#[test]
fn neutral_natures_canonicalize_to_hardy_at_optimizer_boundaries() {
    let data = ChampionsData::load().unwrap();
    let request: SurvivalRequest = serde_json::from_value(serde_json::json!({
        "defender_set": "Mega Salamence\nBashful Nature",
        "hits": [{"attacker_set": "Pikachu", "move_name": "Tackle", "move_times_affected": 0}],
        "max_ko_chances": [0.0], "search": {"result_mode": "AllMinima"}
    }))
    .unwrap();

    // Parsed neutral defender set yields Hardy.
    let parsed = find_min_survival_with_data(&data, request.clone()).unwrap();
    assert_eq!(parsed.matches.len(), 1);
    assert_eq!(parsed.matches[0].nature, Nature::Hardy);

    // Explicit non-Hardy neutral request yields Hardy.
    let mut explicit = request.clone();
    explicit.nature = Some(Nature::Quirky);
    let explicit = find_min_survival_with_data(&data, explicit).unwrap();
    assert!(explicit
        .matches
        .iter()
        .all(|spread| spread.nature == Nature::Hardy));

    // Allowed list with mixed neutrals and duplicates canonicalizes and dedups.
    let mut allowed = request.clone();
    allowed.search.allowed_natures = Some(vec![
        Nature::Serious,
        Nature::Bold,
        Nature::Docile,
        Nature::Bold,
    ]);
    let allowed = find_min_survival_with_data(&data, allowed).unwrap();
    let allowed_natures = distinct_natures(&allowed.matches);
    assert_eq!(allowed_natures, vec![Nature::Bold, Nature::Hardy]);

    // All-neutral allowed list collapses to a single Hardy search.
    let mut neutrals = request.clone();
    neutrals.search.allowed_natures = Some(vec![Nature::Bashful, Nature::Hardy, Nature::Serious]);
    let neutrals = find_min_survival_with_data(&data, neutrals).unwrap();
    assert!(neutrals
        .matches
        .iter()
        .all(|spread| spread.nature == Nature::Hardy));
    assert_eq!(neutrals.matches.len(), 1);

    // The raw parser and stat calculator still understand the original names.
    assert_eq!(parse_nature_name("Bashful"), Some(Nature::Bashful));
    assert_eq!(parse_nature_name("Docile"), Some(Nature::Docile));
    let sps = spreadlab_rs::StatPoints::new(4, 0, 0, 0, 0, 0);
    let species = data.species("Mega Salamence").unwrap();
    assert_eq!(
        champions_final_stats(species.base_stats(), Nature::Bashful, sps).unwrap(),
        champions_final_stats(species.base_stats(), Nature::Hardy, sps).unwrap()
    );
}

fn distinct_natures(matches: &[spreadlab_rs::survival::ExactSurvivalSpread]) -> Vec<Nature> {
    let mut seen = Vec::new();
    for spread in matches {
        if !seen.contains(&spread.nature) {
            seen.push(spread.nature);
        }
    }
    seen
}

#[test]
fn nature_selectors_canonicalize_neutrals() {
    let options = SurvivalSearchOptions::default();
    assert_eq!(
        survival_natures(Nature::Docile, None, false, &options),
        vec![Nature::Hardy]
    );
    assert_eq!(
        survival_natures(Nature::Hardy, Some(Nature::Serious), false, &options),
        vec![Nature::Hardy]
    );
    let mut allowed = options.clone();
    allowed.allowed_natures = Some(vec![Nature::Quirky, Nature::Jolly, Nature::Bashful]);
    assert_eq!(
        survival_natures(Nature::Hardy, None, false, &allowed),
        vec![Nature::Hardy, Nature::Jolly]
    );
    assert_eq!(
        survival_natures(Nature::Hardy, None, true, &options).len(),
        21
    );
}

#[test]
fn low_level_searches_canonicalize_neutral_inputs() {
    let data = ChampionsData::load().unwrap();

    // Offensive KO search backstop: a neutral-heavy list equals Hardy alone.
    let offense = benchmark(
        "Kingambit\nAbility: Defiant\nSPs: 32 Atk\nAdamant Nature",
        "Mega Salamence",
        "Iron Head",
    );
    let neutral = offensive_ko_search(
        &data,
        &offense,
        &[Nature::Quirky, Nature::Hardy, Nature::Docile],
        0.0,
        10,
    )
    .unwrap();
    let hardy = offensive_ko_search(&data, &offense, &[Nature::Hardy], 0.0, 10).unwrap();
    assert_eq!(
        serde_json::to_value(&neutral).unwrap(),
        serde_json::to_value(&hardy).unwrap()
    );
    assert!(neutral
        .matches
        .iter()
        .all(|spread| spread.nature == Nature::Hardy));

    // Direct survival_search happy path, bypassing the survival_natures selector.
    let options = SurvivalSearchOptions::default();
    let dragon = dragon();
    let neutral = survival_search(
        &data,
        std::slice::from_ref(&dragon),
        &[Nature::Serious, Nature::Bashful],
        &[0.25],
        100.0,
        &options,
        &SurvivalEvaluation::Independent,
    )
    .unwrap();
    let hardy = survival_search(
        &data,
        std::slice::from_ref(&dragon),
        &[Nature::Hardy],
        &[0.25],
        100.0,
        &options,
        &SurvivalEvaluation::Independent,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&neutral).unwrap(),
        serde_json::to_value(&hardy).unwrap()
    );
}

#[test]
fn offensive_stat_dependencies_and_preserved_investments() {
    let data = ChampionsData::load().unwrap();
    let press = benchmark("Corviknight\nSPs: 8 HP / 12 Spe", "Kingambit", "Body Press");
    let result = offensive_ko_search(&data, &press, &[Nature::Hardy], 1.0, 1).unwrap();
    let best = result
        .matches
        .first()
        .or(result.closest_miss.as_ref())
        .unwrap();
    assert_eq!(best.investment_stat, OffensiveInvestmentStat::Defense);
    assert_eq!(best.sps.hp, 8);
    assert_eq!(best.sps.speed, 12);
    assert_eq!(best.sps.attack, 0);
    let foul = benchmark("Umbreon\nSPs: 8 HP", "Dragapult\nSPs: 32 Atk", "Foul Play");
    let result = offensive_ko_search(&data, &foul, &[Nature::Hardy], 0.0, 10).unwrap();
    assert_eq!(result.matches.len(), 1);
    assert_eq!(
        result.matches[0].investment_stat,
        OffensiveInvestmentStat::None
    );
    assert_eq!(result.matches[0].sps, foul.attacker.stat_points);
}

#[test]
fn sequence_timing_is_explicit_and_plain_single_hit_agrees() {
    let data = ChampionsData::load().unwrap();
    let b = dragon();
    let options = SurvivalSearchOptions {
        max_total: 0,
        limit: 1,
        ..Default::default()
    };
    let single = survival_search(
        &data,
        std::slice::from_ref(&b),
        &[Nature::Hardy],
        &[1.0],
        100.0,
        &options,
        &SurvivalEvaluation::Independent,
    )
    .unwrap();
    let sequence = survival_search(
        &data,
        &[b],
        &[Nature::Hardy],
        &[1.0],
        100.0,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![],
        },
    )
    .unwrap();
    assert_eq!(single.matches[0].ko_chances, sequence.matches[0].ko_chances);
    let poisoned = benchmark("Sneasler", "Kingambit\nStatus: Poisoned", "Fake Out");
    let run = |turn| {
        survival_search(
            &data,
            std::slice::from_ref(&poisoned),
            &[Nature::Hardy],
            &[1.0],
            10.0,
            &options,
            &SurvivalEvaluation::Sequence {
                end_turn_after: vec![turn],
            },
        )
        .unwrap()
        .matches[0]
            .ko_chances[0]
    };
    assert_eq!(run(false), 0.0);
    assert_eq!(run(true), 1.0);
    // Reject unsupported status transitions even if the first attack always KOs.
    let lethal = benchmark("Sneasler\nSPs: 32 Atk", "Kingambit", "Close Combat");
    let status = benchmark("Pikachu", "Kingambit", "Protect");
    assert!(survival_search(
        &data,
        &[lethal, status],
        &[Nature::Hardy],
        &[1.0],
        1.0,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![]
        }
    )
    .is_err());
}

#[test]
fn sequence_probability_matches_unmerged_roll_tree() {
    let data = ChampionsData::load().unwrap();
    // Three mixed attacks; independently enumerate complete 16^3 paths.
    let benchmarks = [
        benchmark("Pikachu", "Mega Salamence", "Tackle"),
        benchmark("Pikachu", "Mega Salamence", "Thunderbolt"),
        benchmark("Pikachu", "Mega Salamence", "Tackle"),
    ];
    let options = SurvivalSearchOptions {
        max_total: 0,
        ..Default::default()
    };
    let result = survival_search(
        &data,
        &benchmarks,
        &[Nature::Hardy],
        &[1.0],
        60.0,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![],
        },
    )
    .unwrap();
    let summary = result.matches[0].sequence.as_ref().unwrap();
    fn traverse(
        data: &ChampionsData,
        benchmarks: &[DamageBenchmark],
        hp: u16,
        damage: u16,
    ) -> (f64, u16, u16) {
        if hp == 0 {
            return (1.0, damage, damage);
        }
        let Some((first, rest)) = benchmarks.split_first() else {
            return (0.0, damage, damage);
        };
        let mut b = first.clone();
        b.defender_current_hp = Some(hp);
        let rolls = calculate_benchmark(data, &b).unwrap().damage_rolls;
        let mut probability = 0.0;
        let mut minimum = u16::MAX;
        let mut maximum = 0;
        for &roll in &rolls {
            let (p, low, high) = traverse(data, rest, hp.saturating_sub(roll), damage + roll);
            probability += p / rolls.len() as f64;
            minimum = minimum.min(low);
            maximum = maximum.max(high);
        }
        (probability, minimum, maximum)
    }
    let expected = traverse(&data, &benchmarks, summary.starting_hp, 0);
    assert_eq!(summary.ko_chance, expected.0 as f32);
    assert_eq!(
        (summary.min_damage, summary.max_damage),
        (expected.1, expected.2)
    );
}

#[test]
fn sequence_focus_sash_is_consumed_and_weather_ko_precedes_healing() {
    let data = ChampionsData::load().unwrap();
    let b = benchmark(
        "Sneasler\nSPs: 32 Atk\nAdamant Nature",
        "Kingambit @ Focus Sash",
        "Close Combat",
    );
    let options = SurvivalSearchOptions {
        max_total: 0,
        ..Default::default()
    };
    let one = survival_search(
        &data,
        std::slice::from_ref(&b),
        &[Nature::Hardy],
        &[1.0],
        100.0,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![],
        },
    )
    .unwrap();
    assert_eq!(one.matches[0].ko_chances, vec![0.0]);
    let two = survival_search(
        &data,
        &[b.clone(), b],
        &[Nature::Hardy],
        &[1.0],
        100.0,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![],
        },
    )
    .unwrap();
    assert_eq!(two.matches[0].ko_chances, vec![1.0]);
    let mut sand = benchmark("Pikachu", "Gengar @ Leftovers", "Tackle");
    sand.field.weather = damage_calc::Weather::Sand;
    let result = survival_search(
        &data,
        &[sand],
        &[Nature::Hardy],
        &[1.0],
        1.0,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![true],
        },
    )
    .unwrap();
    assert_eq!(result.matches[0].ko_chances, vec![1.0]);
}

#[test]
fn pareto_and_global_closest_miss_match_exhaustive_objectives() {
    let data = ChampionsData::load().unwrap();
    let b = dragon();
    let mut oracle = Vec::new();
    for hp in 0..=24 {
        for spd in 0..=24 - hp {
            let mut candidate = b.clone();
            candidate.defender.stat_points = StatPoints::new(hp, 0, 0, 0, spd, 0);
            let p = calculate_benchmark(&data, &candidate)
                .unwrap()
                .ko_chance
                .unwrap();
            oracle.push((hp + spd, hp, spd, p));
        }
    }
    let expected: BTreeSet<_> = oracle
        .iter()
        .filter(|&&(cost, _, _, p)| {
            !oracle.iter().any(|&(other_cost, _, _, other_p)| {
                other_cost <= cost && other_p <= p && (other_cost < cost || other_p < p)
            })
        })
        .map(|&(_, hp, spd, _)| (hp, spd))
        .collect();
    let options = SurvivalSearchOptions {
        max_total: 24,
        locked: LockedStats {
            defense: Some(0),
            ..Default::default()
        },
        result_mode: SurvivalResultMode::Pareto,
        ..Default::default()
    };
    let result = survival_search(
        &data,
        std::slice::from_ref(&b),
        &[Nature::Hardy],
        &[1.0],
        100.0,
        &options,
        &SurvivalEvaluation::Independent,
    )
    .unwrap();
    assert_eq!(
        result
            .matches
            .iter()
            .map(|s| (s.sps.hp, s.sps.special_defense))
            .collect::<BTreeSet<_>>(),
        expected
    );
    let options = SurvivalSearchOptions {
        result_mode: SurvivalResultMode::AllMinima,
        include_closest_miss: true,
        ..options
    };
    let result = survival_search(
        &data,
        &[b],
        &[Nature::Hardy],
        &[0.25],
        100.0,
        &options,
        &SurvivalEvaluation::Independent,
    )
    .unwrap();
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.minimum_total, Some(19));
    let miss = result.closest_miss.unwrap();
    let best_miss_probability = oracle
        .iter()
        .filter(|x| x.3 > 0.25)
        .map(|x| x.3)
        .min_by(f32::total_cmp)
        .unwrap();
    assert_eq!(miss.ko_chances[0], best_miss_probability);
}

#[test]
fn mixed_sequence_minimum_matches_full_product_oracle() {
    let data = ChampionsData::load().unwrap();
    let benchmarks = [
        benchmark("Pikachu", "Mega Salamence", "Tackle"),
        benchmark("Pikachu", "Mega Salamence", "Thunderbolt"),
    ];
    // Place initial HP at a lethal percentile boundary, ensuring a nonzero optimum.
    let initial: Vec<_> = benchmarks
        .iter()
        .map(|b| calculate_benchmark(&data, b).unwrap().damage_rolls)
        .collect();
    let mut sums: Vec<_> = initial[0]
        .iter()
        .flat_map(|&a| initial[1].iter().map(move |&b| a + b))
        .collect();
    sums.sort();
    let base_hp = champions_final_stats(
        data.species("Mega Salamence").unwrap().base_stats(),
        Nature::Hardy,
        StatPoints::default(),
    )
    .unwrap()
    .hp;
    let hp_percent = (f32::from(sums[sums.len() * 3 / 4 - 1]) - 0.25) * 100.0 / f32::from(base_hp);
    let mut expected = BTreeSet::new();
    let mut minimum = u16::MAX;
    for hp in 0..=8 {
        for defense in 0..=8 - hp {
            for spd in 0..=8 - hp - defense {
                let sps = StatPoints::new(hp, 0, defense, 0, spd, 0);
                let stats = champions_final_stats(
                    data.species("Mega Salamence").unwrap().base_stats(),
                    Nature::Hardy,
                    sps,
                )
                .unwrap();
                let starting_hp =
                    (f64::from(stats.hp) * f64::from(hp_percent) / 100.0).ceil() as u16;
                let rolls: Vec<_> = benchmarks
                    .iter()
                    .map(|b| {
                        let mut candidate = b.clone();
                        candidate.defender.stat_points = sps;
                        candidate.defender_current_hp = Some(starting_hp);
                        calculate_benchmark(&data, &candidate).unwrap().damage_rolls
                    })
                    .collect();
                let kos = rolls[0]
                    .iter()
                    .flat_map(|&a| rolls[1].iter().map(move |&b| a + b))
                    .filter(|&sum| sum >= starting_hp)
                    .count();
                let probability = kos as f32 / (rolls[0].len() * rolls[1].len()) as f32;
                if probability <= 0.25 {
                    if sps.total() < minimum {
                        minimum = sps.total();
                        expected.clear();
                    }
                    if sps.total() == minimum {
                        expected.insert((hp, defense, spd));
                    }
                }
            }
        }
    }
    let options = SurvivalSearchOptions {
        max_total: 8,
        result_mode: SurvivalResultMode::AllMinima,
        ..Default::default()
    };
    let result = survival_search(
        &data,
        &benchmarks,
        &[Nature::Hardy],
        &[0.25],
        hp_percent,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![],
        },
    )
    .unwrap();
    assert!(!expected.is_empty());
    assert!(minimum > 0);
    assert_eq!(
        result
            .matches
            .iter()
            .map(|s| (s.sps.hp, s.sps.defense, s.sps.special_defense))
            .collect::<BTreeSet<_>>(),
        expected
    );
}
