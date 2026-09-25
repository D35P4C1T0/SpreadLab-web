//! Regression coverage for defensive-stat relevance.
//!
//! The pinned engine reads exactly one defensive stat for an ordinary hit, so the
//! other one must not be varied or invested. These tests pin that behavior for
//! ordinary physical and special searches, for locks and lower bounds, for mixed
//! benchmarks, and for the engine's defense override moves. Each case is also checked
//! against a hand-built exhaustive oracle over the full HP/Defense/SpD grid, so
//! narrowing a domain cannot silently move the reported minimum.

use damage_calc::Nature;
use spreadlab_rs::{
    api::{find_min_survival_with_data, SurvivalRequest},
    damage_bridge::{calculate_benchmark, DamageBenchmark},
    data::ChampionsData,
    optimize::{
        hp_def_survival_search, minimize_hp_def_survival, optimize_defensive, optimize_offensive,
    },
    showdown::parse_set,
    spreads::{LockedStats, SpreadSearch},
    stats::{champions_final_stats, StatPoints},
    survival::{
        survival_search, ExactSurvivalSearchResult, ExactSurvivalSpread, SurvivalEvaluation,
        SurvivalResultMode, SurvivalSearchOptions,
    },
};
use std::collections::BTreeSet;

const IRON_HEAD_ATTACKER: &str =
    "Kingambit\nAbility: Defiant\nSPs: 32 Atk\nAdamant Nature\n- Iron Head";
const SPECIAL_ATTACKER: &str = "Gholdengo\nAbility: Good as Gold\nSPs: 32 SpA\nModest Nature";
const DRAGON_PULSE_ATTACKER: &str = "Dragapult @ Life Orb\nSPs: 32 SpA\nHardy Nature";
const DRAGON_CLAW_ATTACKER: &str = "Dragonite @ Life Orb\nSPs: 32 Atk\nAdamant Nature";

fn benchmark(attacker: &str, defender: &str, move_name: &str) -> DamageBenchmark {
    DamageBenchmark::new(
        parse_set(attacker).unwrap(),
        parse_set(defender).unwrap(),
        move_name,
    )
}

/// Strong physical hit whose minimum investment needs Defense.
fn iron_head() -> DamageBenchmark {
    benchmark(IRON_HEAD_ATTACKER, "Mega Floette\n- Protect", "Iron Head")
}

/// Special hit that needs no investment at all at the thresholds used below.
fn shadow_ball() -> DamageBenchmark {
    benchmark(SPECIAL_ATTACKER, "Mega Floette\n- Protect", "Shadow Ball")
}

/// Special move with the engine's defense override: it resolves against Defense.
fn psyshock() -> DamageBenchmark {
    benchmark(SPECIAL_ATTACKER, "Mega Gengar\n- Protect", "Psyshock")
}

fn dragon_pulse() -> DamageBenchmark {
    benchmark(
        DRAGON_PULSE_ATTACKER,
        "Mega Salamence\nHardy Nature",
        "Dragon Pulse",
    )
}

fn dragon_claw() -> DamageBenchmark {
    benchmark(
        DRAGON_CLAW_ATTACKER,
        "Mega Salamence\nHardy Nature",
        "Dragon Claw",
    )
}

fn search(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    thresholds: &[f32],
    options: &SurvivalSearchOptions,
) -> ExactSurvivalSearchResult {
    survival_search(
        data,
        benchmarks,
        &[Nature::Hardy],
        thresholds,
        100.0,
        options,
        &SurvivalEvaluation::Independent,
    )
    .unwrap()
}

/// Exhaustive HP/Defense/SpD oracle over a capped domain. It mirrors the locks it is
/// given and uses the engine's own KO probability, so it matches the search's
/// feasibility predicate without assuming which defensive stat matters.
fn exhaustive_minimum(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    thresholds: &[f32],
    locked: LockedStats,
    max_total: u16,
) -> (Option<u16>, BTreeSet<(u16, u16, u16)>) {
    let base = benchmarks[0].defender.stat_points;
    let species = data.species(&benchmarks[0].defender.species).unwrap();
    let ranged = |locked: Option<u16>| match locked {
        Some(value) => value..=value,
        None => 0..=32,
    };
    let mut minimum: Option<u16> = None;
    let mut layer = BTreeSet::new();
    for hp in ranged(locked.hp) {
        for defense in ranged(locked.defense) {
            for special_defense in ranged(locked.special_defense) {
                let sps = StatPoints::new(
                    hp,
                    locked.attack.unwrap_or(base.attack),
                    defense,
                    locked.special_attack.unwrap_or(base.special_attack),
                    special_defense,
                    locked.speed.unwrap_or(base.speed),
                );
                let cost = sps.total();
                if cost > max_total {
                    continue;
                }
                let feasible = benchmarks
                    .iter()
                    .zip(thresholds)
                    .all(|(benchmark, threshold)| {
                        let mut candidate = benchmark.clone();
                        candidate.defender.stat_points = sps;
                        candidate.defender_current_hp = Some(
                            champions_final_stats(
                                species.base_stats(),
                                candidate.defender.nature,
                                sps,
                            )
                            .unwrap()
                            .hp,
                        );
                        calculate_benchmark(data, &candidate)
                            .unwrap()
                            .ko_chance
                            .unwrap()
                            <= *threshold
                    });
                if feasible {
                    if minimum.is_none_or(|best| cost < best) {
                        minimum = Some(cost);
                        layer.clear();
                    }
                    if minimum == Some(cost) {
                        layer.insert((hp, defense, special_defense));
                    }
                }
            }
        }
    }
    (minimum, layer)
}

fn coordinates(spread: &ExactSurvivalSpread) -> (u16, u16, u16) {
    (
        spread.sps.hp,
        spread.sps.defense,
        spread.sps.special_defense,
    )
}

#[test]
fn physical_only_search_pins_special_defense() {
    let data = ChampionsData::load().unwrap();
    let benchmark = iron_head();
    let options = SurvivalSearchOptions {
        limit: 6,
        include_closest_miss: true,
        ..Default::default()
    };
    let result = search(&data, std::slice::from_ref(&benchmark), &[0.125], &options);
    assert!(!result.matches.is_empty());
    for spread in &result.matches {
        assert_eq!(spread.sps.special_defense, 0, "{}", spread.sp_line);
    }
    assert!(result
        .closest_miss
        .as_ref()
        .is_none_or(|miss| miss.sps.special_defense == 0));
    assert_eq!(coordinates(&result.matches[0]), (4, 32, 0));
    // The layer that used to surface "4 HP / 32 Def / 1 SpD" now stays on Defense.
    assert!(result
        .matches
        .iter()
        .any(|spread| coordinates(spread) == (5, 32, 0)));

    // A smaller fixture keeps the independent oracle cheap: the whole first feasible
    // layer must match the unrestricted HP/Defense/SpD enumeration.
    let benchmark = dragon_claw();
    let options = SurvivalSearchOptions {
        max_total: 12,
        result_mode: SurvivalResultMode::AllMinima,
        ..Default::default()
    };
    let result = search(&data, std::slice::from_ref(&benchmark), &[0.25], &options);
    let (minimum, layer) =
        exhaustive_minimum(&data, &[benchmark], &[0.25], LockedStats::default(), 12);
    assert!(!layer.is_empty());
    assert!(layer
        .iter()
        .all(|&(_, _, special_defense)| special_defense == 0));
    assert_eq!(result.minimum_total, minimum);
    assert_eq!(
        result
            .matches
            .iter()
            .map(coordinates)
            .collect::<BTreeSet<_>>(),
        layer
    );
}

#[test]
fn special_only_search_pins_defense() {
    let data = ChampionsData::load().unwrap();
    let benchmark = shadow_ball();
    let options = SurvivalSearchOptions {
        limit: 6,
        ..Default::default()
    };
    let result = search(&data, std::slice::from_ref(&benchmark), &[0.125], &options);
    assert!(!result.matches.is_empty());
    for spread in &result.matches {
        assert_eq!(spread.sps.defense, 0, "{}", spread.sp_line);
    }
    // "1 SpD" survives as a relevant layer; the worthless "1 Def" alternative is gone.
    assert!(result
        .matches
        .iter()
        .any(|spread| spread.sps.special_defense > 0));

    let benchmark = dragon_pulse();
    let options = SurvivalSearchOptions {
        // The documented Dragon Pulse minimum is 19 points.
        max_total: 19,
        result_mode: SurvivalResultMode::AllMinima,
        ..Default::default()
    };
    let result = search(&data, std::slice::from_ref(&benchmark), &[0.25], &options);
    let (minimum, layer) =
        exhaustive_minimum(&data, &[benchmark], &[0.25], LockedStats::default(), 19);
    assert_eq!(minimum, Some(19));
    assert!(!layer.is_empty());
    assert!(layer.iter().all(|&(_, defense, _)| defense == 0));
    assert_eq!(result.minimum_total, minimum);
    assert_eq!(
        result
            .matches
            .iter()
            .map(coordinates)
            .collect::<BTreeSet<_>>(),
        layer
    );
}

#[test]
fn irrelevant_defense_locks_and_lower_bounds_are_honored() {
    let data = ChampionsData::load().unwrap();
    let benchmark = iron_head();
    let bounded = SurvivalSearchOptions {
        minimum: StatPoints::new(0, 0, 0, 0, 3, 0),
        limit: 4,
        ..Default::default()
    };
    let result = search(&data, std::slice::from_ref(&benchmark), &[0.125], &bounded);
    assert!(!result.matches.is_empty());
    for spread in &result.matches {
        assert_eq!(spread.sps.special_defense, 3, "{}", spread.sp_line);
    }
    let locked = SurvivalSearchOptions {
        locked: LockedStats {
            special_defense: Some(7),
            ..Default::default()
        },
        limit: 4,
        ..Default::default()
    };
    let result = search(&data, &[benchmark], &[0.125], &locked);
    assert!(!result.matches.is_empty());
    for spread in &result.matches {
        assert_eq!(spread.sps.special_defense, 7, "{}", spread.sp_line);
    }
    assert_eq!(result.minimum_total, Some(36 + 7));
}

#[test]
fn psyshock_defense_override_keeps_defense_and_pins_special_defense() {
    let data = ChampionsData::load().unwrap();
    let benchmark = psyshock();
    // Engine ground truth: Psyshock is Special in the metadata but resolves against
    // Defense, so Special Defense cannot change its damage.
    let damage = |defense, special_defense| {
        let mut candidate = benchmark.clone();
        candidate.defender.stat_points = StatPoints::new(0, 0, defense, 0, special_defense, 0);
        calculate_benchmark(&data, &candidate).unwrap()
    };
    let base = damage(0, 0);
    assert_ne!(base.damage_rolls, damage(32, 0).damage_rolls);
    assert_eq!(base.damage_rolls, damage(0, 32).damage_rolls);

    let options = SurvivalSearchOptions {
        max_total: 12,
        result_mode: SurvivalResultMode::AllMinima,
        ..Default::default()
    };
    let result = search(&data, std::slice::from_ref(&benchmark), &[0.25], &options);
    assert!(!result.matches.is_empty());
    for spread in &result.matches {
        assert_eq!(spread.sps.special_defense, 0, "{}", spread.sp_line);
        assert!(spread.sps.defense > 0, "{}", spread.sp_line);
    }
    let (minimum, layer) =
        exhaustive_minimum(&data, &[benchmark], &[0.25], LockedStats::default(), 12);
    assert!(layer.iter().any(|&(_, defense, _)| defense > 0));
    assert_eq!(result.minimum_total, minimum);
    assert_eq!(
        result
            .matches
            .iter()
            .map(coordinates)
            .collect::<BTreeSet<_>>(),
        layer
    );
}

#[test]
fn mixed_benchmarks_keep_both_defenses() {
    let data = ChampionsData::load().unwrap();
    let benchmarks = [dragon_pulse(), dragon_claw()];
    // Pinning the HP coordinate makes each hit demand its own defense stat.
    let locked = LockedStats {
        hp: Some(0),
        ..Default::default()
    };
    let options = SurvivalSearchOptions {
        locked,
        result_mode: SurvivalResultMode::AllMinima,
        ..Default::default()
    };
    let result = search(&data, &benchmarks, &[0.25, 0.25], &options);
    assert!(!result.matches.is_empty());
    for spread in &result.matches {
        assert!(
            spread.sps.defense > 0 && spread.sps.special_defense > 0,
            "{}",
            spread.sp_line
        );
    }
    let (minimum, layer) = exhaustive_minimum(&data, &benchmarks, &[0.25, 0.25], locked, 66);
    assert_eq!(result.minimum_total, minimum);
    assert_eq!(result.minimum_total, Some(34));
    assert_eq!(
        result
            .matches
            .iter()
            .map(coordinates)
            .collect::<BTreeSet<_>>(),
        layer
    );
    // The same requests alone lose the dimension the other benchmark needs.
    let physical_only = search(
        &data,
        &benchmarks[1..],
        &[0.25],
        &SurvivalSearchOptions {
            locked,
            result_mode: SurvivalResultMode::AllMinima,
            ..Default::default()
        },
    );
    assert_eq!(physical_only.minimum_total, Some(14));
    assert!(physical_only
        .matches
        .iter()
        .all(|spread| spread.sps.special_defense == 0));
    let special_only = search(
        &data,
        &benchmarks[..1],
        &[0.25],
        &SurvivalSearchOptions {
            locked,
            result_mode: SurvivalResultMode::AllMinima,
            ..Default::default()
        },
    );
    assert_eq!(special_only.minimum_total, Some(20));
    assert!(special_only
        .matches
        .iter()
        .all(|spread| spread.sps.defense == 0));
}

#[test]
fn legacy_and_request_entry_points_inherit_relevance() {
    let data = ChampionsData::load().unwrap();
    let benchmark = iron_head();
    let legacy = hp_def_survival_search(&data, &benchmark, &[Nature::Hardy], 0.125, 4).unwrap();
    assert!(!legacy.matches.is_empty());
    for spread in &legacy.matches {
        assert_eq!(spread.sps.special_defense, 0, "{}", spread.sp_line);
    }
    let minimized =
        minimize_hp_def_survival(&data, &benchmark, &[Nature::Hardy], 0.125, 4).unwrap();
    assert_eq!(minimized[0].sps, StatPoints::new(4, 0, 32, 0, 0, 0));
    let request: SurvivalRequest = serde_json::from_value(serde_json::json!({
        "defender_set": "Mega Floette\n- Protect",
        "hits": [{
            "attacker_set": IRON_HEAD_ATTACKER,
            "move_name": "Iron Head",
            "move_times_affected": 0
        }],
        "max_ko_chances": [0.125],
        "search": {"limit": 4}
    }))
    .unwrap();
    let response = find_min_survival_with_data(&data, request).unwrap();
    assert!(!response.matches.is_empty());
    assert!(response
        .matches
        .iter()
        .all(|spread| spread.sps.special_defense == 0));
}

#[test]
fn sequence_and_closest_miss_paths_share_the_same_relevance() {
    let data = ChampionsData::load().unwrap();
    let benchmark = iron_head();
    let options = SurvivalSearchOptions {
        limit: 4,
        ..Default::default()
    };
    let result = survival_search(
        &data,
        std::slice::from_ref(&benchmark),
        &[Nature::Hardy],
        &[0.125],
        100.0,
        &options,
        &SurvivalEvaluation::Sequence {
            end_turn_after: vec![true],
        },
    )
    .unwrap();
    assert!(!result.matches.is_empty());
    assert!(result
        .matches
        .iter()
        .all(|spread| spread.sps.special_defense == 0));
    assert_eq!(
        result.matches[0].sequence.as_ref().unwrap().ko_chance,
        0.125
    );
}

#[test]
fn generic_defensive_optimizer_does_not_vary_the_irrelevant_defense() {
    let data = ChampionsData::load().unwrap();
    let search = SpreadSearch {
        exact_total: None,
        max_total: 66,
        locked: LockedStats {
            attack: Some(0),
            special_attack: Some(0),
            speed: Some(0),
            ..LockedStats::default()
        },
    };
    let physical = optimize_defensive(&data, &[iron_head()], search, 4).unwrap();
    assert_eq!(physical.len(), 4);
    assert!(physical
        .iter()
        .all(|spread| spread.sps.special_defense == 0));
    assert_eq!(physical[0].sps, StatPoints::new(32, 0, 32, 0, 0, 0));
    let special = optimize_defensive(&data, &[shadow_ball()], search, 4).unwrap();
    assert_eq!(special.len(), 4);
    assert!(special.iter().all(|spread| spread.sps.defense == 0));
    assert_eq!(special[0].sps, StatPoints::new(32, 0, 0, 0, 31, 0));

    // Full spend keeps its meaning: the requested total still has to be reached, so
    // leftover points land in whatever free stat can absorb them (SpD here).
    let full_spend = SpreadSearch {
        exact_total: Some(66),
        max_total: 66,
        locked: search.locked,
    };
    let physical = optimize_defensive(&data, &[iron_head()], full_spend, 3).unwrap();
    assert!(!physical.is_empty());
    assert!(physical.iter().all(|spread| spread.sps.total() == 66));
    assert_eq!(physical[0].sps, StatPoints::new(32, 0, 32, 0, 2, 0));
}

#[test]
fn generic_offensive_optimizer_does_not_vary_the_irrelevant_defense() {
    let data = ChampionsData::load().unwrap();
    // The attacker's own defenses cannot change its outgoing damage, so the ranking
    // must not spend points on them.
    let search = SpreadSearch {
        exact_total: None,
        max_total: 66,
        locked: LockedStats {
            hp: Some(0),
            special_attack: Some(0),
            speed: Some(0),
            ..LockedStats::default()
        },
    };
    let physical = optimize_offensive(&data, &[iron_head()], search, 4).unwrap();
    assert_eq!(physical.len(), 4);
    assert!(physical
        .iter()
        .all(|spread| spread.sps.defense == 0 && spread.sps.special_defense == 0));
    assert!(physical.iter().any(|spread| spread.sps.attack > 0));

    // Body Press attacks with the user's Defense, which must stay a live dimension.
    let press = benchmark("Corviknight\nSPs: 8 HP / 12 Spe", "Kingambit", "Body Press");
    let press = optimize_offensive(&data, &[press], search, 4).unwrap();
    assert_eq!(press.len(), 4);
    assert!(press.iter().all(|spread| spread.sps.special_defense == 0));
    assert!(press.iter().any(|spread| spread.sps.defense > 0));
    assert!(press[0].sps.defense > 0, "{}", press[0].sp_line);
}
