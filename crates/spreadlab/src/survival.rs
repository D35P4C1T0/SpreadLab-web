//! Exact minimum-investment search. No stat-dependency or monotonicity pruning.
use crate::damage_bridge::{build_pokemon, calculate_benchmark, DamageBenchmark};
use crate::data::ChampionsData;
use crate::optimize::{
    all_natures, canonical_nature, canonical_natures, canonicalize_natures,
    current_hp_from_percent, sequence_damage_summary, CombinedDamageSummary, DamageSummary,
    OptimizeError,
};
use crate::showdown::build_champions_sp_line;
use crate::spreads::LockedStats;
use crate::stats::{
    champions_final_stats, FinalStats, StatPoints, MAX_STAT_POINTS, MAX_TOTAL_STAT_POINTS,
};
use damage_calc::{DamageOutcome, DamageResult, Nature};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurvivalResultMode {
    /// The best `limit` feasible spreads, including higher-cost layers if needed.
    #[default]
    TopK,
    /// Every feasible spread in the first feasible cost layer. Ignores `limit`.
    AllMinima,
    /// All feasible spreads undominated in cost and every KO probability.
    /// Equal objective vectors are retained. Ignores `limit`.
    Pareto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SurvivalSearchOptions {
    pub max_total: u16,
    /// Exact values. For unlisted Attack/SpA/Speed, preserve the parsed set.
    pub locked: LockedStats,
    /// Lower bounds on the final spread, not additions to the parsed spread.
    pub minimum: StatPoints,
    pub result_mode: SurvivalResultMode,
    pub limit: usize,
    /// Forces a full-domain search, even when matches exist.
    /// Without it, a closest miss is returned only when no match exists.
    pub include_closest_miss: bool,
    /// Overrides the supplied nature list and API nature settings. Empty is invalid.
    pub allowed_natures: Option<Vec<Nature>>,
}

impl Default for SurvivalSearchOptions {
    fn default() -> Self {
        Self {
            max_total: MAX_TOTAL_STAT_POINTS,
            locked: LockedStats::default(),
            minimum: StatPoints::default(),
            result_mode: SurvivalResultMode::TopK,
            limit: 10,
            include_closest_miss: false,
            allowed_natures: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum SurvivalEvaluation {
    /// Each benchmark is a separate incoming move from the same initial HP.
    #[default]
    Independent,
    /// Ordered attacks using SpreadLab's explicit HP/item/residual model.
    /// One flag per attack, including the last. Empty means no end-turn effects.
    Sequence { end_turn_after: Vec<bool> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExactSurvivalSpread {
    pub rank: usize,
    pub nature: Nature,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub total_points: u16,
    /// Independent, initial-HP summaries; not conditional sequence hit results.
    pub results: Vec<DamageSummary>,
    pub sequence: Option<CombinedDamageSummary>,
    /// One per independent benchmark, or one for the entire sequence.
    pub ko_chances: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExactSurvivalSearchResult {
    pub matches: Vec<ExactSurvivalSpread>,
    pub closest_miss: Option<ExactSurvivalSpread>,
    pub minimum_total: Option<u16>,
}

pub(crate) fn invalid(message: impl Into<String>) -> OptimizeError {
    OptimizeError::InvalidInput(message.into())
}

pub(crate) fn validate_probability(value: f32) -> Result<(), OptimizeError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(invalid("KO probability must be finite and in [0, 1]"));
    }
    Ok(())
}

pub(crate) fn ko_probability(result: &DamageResult) -> Result<f32, OptimizeError> {
    if let Some(probability) = result.ko_chance {
        validate_probability(probability)?;
        return Ok(probability);
    }
    match result.outcome {
        DamageOutcome::Status | DamageOutcome::ImmuneOrFailed if result.max_damage == 0 => Ok(0.0),
        _ => Err(OptimizeError::UnknownKoProbability),
    }
}

pub fn survival_natures(
    parsed: Nature,
    explicit: Option<Nature>,
    optimize: bool,
    options: &SurvivalSearchOptions,
) -> Vec<Nature> {
    if let Some(allowed) = &options.allowed_natures {
        return canonicalize_natures(allowed);
    }
    if let Some(nature) = explicit {
        return vec![canonical_nature(nature)];
    }
    if optimize {
        canonical_natures()
    } else {
        vec![canonical_nature(parsed)]
    }
}

fn coordinates(sps: StatPoints) -> [u16; 6] {
    [
        sps.hp,
        sps.attack,
        sps.defense,
        sps.special_attack,
        sps.special_defense,
        sps.speed,
    ]
}

/// Indices of the defensive coordinates, matching [`coordinates`] order.
const DEFENSE: usize = 2;
const SPECIAL_DEFENSE: usize = 4;

/// Collapses a dimension to its lower bound, keeping its lock or lower bound value.
fn pinned(range: (u16, u16)) -> (u16, u16) {
    (range.0, range.0)
}

fn from_coordinates(x: [u16; 6]) -> StatPoints {
    StatPoints::new(x[0], x[1], x[2], x[3], x[4], x[5])
}

fn bounds(
    base: StatPoints,
    options: &SurvivalSearchOptions,
) -> Result<[(u16, u16); 6], OptimizeError> {
    base.validate()?;
    options.minimum.validate()?;
    if options.max_total > MAX_TOTAL_STAT_POINTS {
        return Err(invalid("max_total must be at most 66"));
    }
    let locks = options.locked;
    let locks = [
        locks.hp,
        locks.attack.or(Some(base.attack)),
        locks.defense,
        locks.special_attack.or(Some(base.special_attack)),
        locks.special_defense,
        locks.speed.or(Some(base.speed)),
    ];
    let minimum = coordinates(options.minimum);
    let mut ranges = [(0, MAX_STAT_POINTS); 6];
    for i in 0..6 {
        ranges[i] = match locks[i] {
            Some(value) if value > MAX_STAT_POINTS || value < minimum[i] => {
                return Err(invalid(
                    "locked stat exceeds 32 or conflicts with its lower bound",
                ));
            }
            Some(value) => (value, value),
            None => (minimum[i], MAX_STAT_POINTS),
        };
    }
    Ok(ranges)
}

// Only constructs the requested cost layer. Fixed coordinates collapse naturally.
fn visit_layer(
    ranges: &[(u16, u16); 6],
    index: usize,
    remaining: u16,
    x: &mut [u16; 6],
    visit: &mut impl FnMut(StatPoints) -> Result<(), OptimizeError>,
) -> Result<(), OptimizeError> {
    if index == 6 {
        if remaining == 0 {
            visit(from_coordinates(*x))?;
        }
        return Ok(());
    }
    let rest_min: u16 = ranges[index + 1..].iter().map(|r| r.0).sum();
    let rest_max: u16 = ranges[index + 1..].iter().map(|r| r.1).sum();
    for value in ranges[index].0..=ranges[index].1.min(remaining) {
        let rest = remaining - value;
        if rest < rest_min || rest > rest_max {
            continue;
        }
        x[index] = value;
        visit_layer(ranges, index + 1, rest, x, visit)?;
    }
    Ok(())
}

fn compare(left: &ExactSurvivalSpread, right: &ExactSurvivalSpread) -> Ordering {
    left.total_points
        .cmp(&right.total_points)
        .then_with(|| left.ko_chances.partial_cmp(&right.ko_chances).unwrap())
        .then_with(|| {
            let damage = |spread: &ExactSurvivalSpread| -> u64 {
                spread.sequence.as_ref().map_or_else(
                    || spread.results.iter().map(|r| u64::from(r.max_damage)).sum(),
                    |r| u64::from(r.max_damage),
                )
            };
            damage(left).cmp(&damage(right))
        })
        .then_with(|| coordinates(left.sps).cmp(&coordinates(right.sps)))
        .then_with(|| nature_index(left.nature).cmp(&nature_index(right.nature)))
}

fn nature_index(nature: Nature) -> usize {
    all_natures().iter().position(|n| *n == nature).unwrap()
}

fn dominates(left: &ExactSurvivalSpread, right: &ExactSurvivalSpread) -> bool {
    left.total_points <= right.total_points
        && left
            .ko_chances
            .iter()
            .zip(&right.ko_chances)
            .all(|(l, r)| l <= r)
        && (left.total_points < right.total_points
            || left
                .ko_chances
                .iter()
                .zip(&right.ko_chances)
                .any(|(l, r)| l < r))
}

/// Exact over HP/Defense/SpD and allowed natures, with other stats fixed.
/// Threshold count must match independent benchmarks, or be one for a sequence.
#[allow(clippy::too_many_arguments)]
pub fn survival_search(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    natures: &[Nature],
    thresholds: &[f32],
    hp_percent: f32,
    options: &SurvivalSearchOptions,
    evaluation: &SurvivalEvaluation,
) -> Result<ExactSurvivalSearchResult, OptimizeError> {
    let mut natures = canonicalize_natures(options.allowed_natures.as_deref().unwrap_or(natures));
    let first = benchmarks
        .first()
        .ok_or_else(|| invalid("at least one benchmark is required"))?;
    if natures.is_empty() {
        return Err(invalid("at least one nature is required"));
    }
    if !hp_percent.is_finite() || !(0.0..=100.0).contains(&hp_percent) || hp_percent == 0.0 {
        return Err(invalid("hp_percent must be finite and in (0, 100]"));
    }
    if options.result_mode == SurvivalResultMode::TopK && options.limit == 0 {
        return Err(invalid("TopK limit must be positive"));
    }
    let expected = match evaluation {
        SurvivalEvaluation::Independent => benchmarks.len(),
        SurvivalEvaluation::Sequence { end_turn_after } => {
            if !end_turn_after.is_empty() && end_turn_after.len() != benchmarks.len() {
                return Err(invalid("end_turn_after must have one flag per attack"));
            }
            1
        }
    };
    if thresholds.len() != expected {
        return Err(invalid("wrong number of KO thresholds"));
    }
    for &threshold in thresholds {
        validate_probability(threshold)?;
    }
    let defender = build_pokemon(data, &first.defender)?;
    for benchmark in &benchmarks[1..] {
        if build_pokemon(data, &benchmark.defender)? != defender {
            return Err(invalid("all benchmarks must use the same defender set"));
        }
    }
    // Only defensive stats the engine can read stay search dimensions. The other one
    // keeps its lock or lower bound and is never varied freely, which cannot move the
    // minimum cost or a KO threshold because it does not enter the damage result.
    let relevance = crate::relevance::combined_defender_relevance(data, benchmarks)?;
    let mut ranges = bounds(first.defender.stat_points, options)?;
    if !relevance.defense {
        ranges[DEFENSE] = pinned(ranges[DEFENSE]);
    }
    if !relevance.special_defense {
        ranges[SPECIAL_DEFENSE] = pinned(ranges[SPECIAL_DEFENSE]);
    }
    natures.sort_by_key(|n| nature_index(*n));
    natures.dedup();
    let species = data.species(&first.defender.species)?;
    let mut matches = Vec::new();
    let mut closest_miss: Option<ExactSurvivalSpread> = None;
    let mut minimum_total = None;
    let miss_distance = |spread: &ExactSurvivalSpread| -> f32 {
        spread
            .ko_chances
            .iter()
            .zip(thresholds)
            .map(|(p, t)| (p - t).max(0.0))
            .fold(0.0, f32::max)
    };
    for total in 0..=options.max_total {
        visit_layer(&ranges, 0, total, &mut [0; 6], &mut |sps| {
            for &nature in &natures {
                let final_stats = champions_final_stats(species.base_stats(), nature, sps)?;
                let starting_hp = current_hp_from_percent(final_stats.hp, hp_percent);
                let mut results = Vec::with_capacity(benchmarks.len());
                let mut ko_chances = Vec::with_capacity(expected);
                for benchmark in benchmarks {
                    let mut candidate = benchmark.clone();
                    candidate.defender.nature = nature;
                    candidate.defender.stat_points = sps;
                    candidate.defender_current_hp = Some(starting_hp);
                    let result = calculate_benchmark(data, &candidate)?;
                    if matches!(evaluation, SurvivalEvaluation::Sequence { .. })
                        && !matches!(
                            result.outcome,
                            DamageOutcome::Damage
                                | DamageOutcome::Fixed
                                | DamageOutcome::ImmuneOrFailed
                        )
                    {
                        return Err(invalid(
                            "sequence model supports damaging or immune moves only",
                        ));
                    }
                    let probability = ko_probability(&result)?;
                    let mut summary = DamageSummary::from(result);
                    summary.ko_chance = Some(probability);
                    ko_chances.push(probability);
                    results.push(summary);
                }
                let sequence = if let SurvivalEvaluation::Sequence { end_turn_after } = evaluation {
                    let combined = sequence_damage_summary(
                        data,
                        benchmarks,
                        nature,
                        sps,
                        final_stats.hp,
                        starting_hp,
                        end_turn_after,
                    )?;
                    ko_chances = vec![combined.ko_chance];
                    Some(combined)
                } else {
                    None
                };
                let spread = ExactSurvivalSpread {
                    rank: 0,
                    nature,
                    sps,
                    sp_line: build_champions_sp_line(sps),
                    final_stats,
                    total_points: total,
                    results,
                    sequence,
                    ko_chances,
                };
                if spread
                    .ko_chances
                    .iter()
                    .zip(thresholds)
                    .all(|(p, t)| p <= t)
                {
                    minimum_total.get_or_insert(total);
                    match options.result_mode {
                        SurvivalResultMode::AllMinima if minimum_total != Some(total) => {}
                        SurvivalResultMode::Pareto => {
                            if !matches.iter().any(|other| dominates(other, &spread)) {
                                matches.retain(|other| !dominates(&spread, other));
                                matches.push(spread);
                            }
                        }
                        _ => matches.push(spread),
                    }
                } else if closest_miss.as_ref().is_none_or(|previous| {
                    miss_distance(&spread)
                        .total_cmp(&miss_distance(previous))
                        .then_with(|| compare(&spread, previous))
                        == Ordering::Less
                }) {
                    closest_miss = Some(spread);
                }
            }
            Ok(())
        })?;
        if options.result_mode == SurvivalResultMode::TopK {
            matches.sort_by(compare);
            matches.truncate(options.limit);
        }
        if !options.include_closest_miss
            && match options.result_mode {
                SurvivalResultMode::AllMinima => minimum_total.is_some(),
                SurvivalResultMode::TopK => matches.len() >= options.limit,
                SurvivalResultMode::Pareto => false,
            }
        {
            break;
        }
    }
    matches.sort_by(compare);
    for (i, spread) in matches.iter_mut().enumerate() {
        spread.rank = i + 1;
    }
    if !matches.is_empty() && !options.include_closest_miss {
        closest_miss = None;
    }
    if let Some(spread) = &mut closest_miss {
        spread.rank = 1;
    }
    Ok(ExactSurvivalSearchResult {
        matches,
        closest_miss,
        minimum_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_probability_requires_a_known_zero_damage_outcome() {
        let data = ChampionsData::load().unwrap();
        let b = DamageBenchmark::new(
            crate::showdown::parse_set("Pikachu").unwrap(),
            crate::showdown::parse_set("Mega Salamence").unwrap(),
            "Tackle",
        );
        let mut result = calculate_benchmark(&data, &b).unwrap();
        result.ko_chance = None;
        assert!(ko_probability(&result).is_err());
        result.max_damage = 0;
        // Zero raw damage alone does not establish a valid probability.
        assert!(ko_probability(&result).is_err());
        result.outcome = DamageOutcome::Status;
        assert_eq!(ko_probability(&result).unwrap(), 0.0);
        result.outcome = DamageOutcome::ImmuneOrFailed;
        assert_eq!(ko_probability(&result).unwrap(), 0.0);
        result.ko_chance = Some(f32::NAN);
        assert!(ko_probability(&result).is_err());
    }
}
