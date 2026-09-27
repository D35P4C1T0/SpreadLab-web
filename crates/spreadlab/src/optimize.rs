use crate::damage_bridge::{build_pokemon, evaluate_input, prepare_benchmark, DamageBenchmark};
use crate::data::{parse_item, ChampionsData};
use crate::showdown::build_champions_sp_line;
use crate::spreads::{generate_spreads, SpreadSearch};
use crate::stats::{champions_final_stats, FinalStats, StatPoints};
use damage_calc::{
    Ability, DamageResult, Item, Nature, PokemonType, StatusCondition, Terrain, Weather,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OptimizeError {
    #[error("invalid optimization request: {0}")]
    InvalidInput(String),
    #[error("damage engine returned no usable KO probability")]
    UnknownKoProbability,
    #[error(transparent)]
    Bridge(#[from] crate::damage_bridge::BridgeError),
    #[error(transparent)]
    Data(#[from] crate::data::DataError),
    #[error(transparent)]
    Stats(#[from] crate::stats::StatError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationMode {
    Defensive,
    Offensive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedSpread {
    pub rank: usize,
    pub mode: OptimizationMode,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub score: f64,
    pub results: Vec<DamageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageSummary {
    pub min_damage: u16,
    pub max_damage: u16,
    pub percent_min: f32,
    pub percent_max: f32,
    pub ko_chance: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalSpread {
    pub rank: usize,
    pub nature: Nature,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub total_points: u16,
    pub result: DamageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedDamageSummary {
    pub min_damage: u16,
    pub max_damage: u16,
    pub percent_min: f32,
    pub percent_max: f32,
    pub ko_chance: f32,
    pub starting_hp: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedSurvivalSpread {
    pub rank: usize,
    pub nature: Nature,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub total_points: u16,
    pub hits: Vec<DamageSummary>,
    pub combined: CombinedDamageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalSearchResult {
    pub matches: Vec<SurvivalSpread>,
    pub closest_miss: Option<SurvivalSpread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedSurvivalSearchResult {
    pub matches: Vec<CombinedSurvivalSpread>,
    pub closest_miss: Option<CombinedSurvivalSpread>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffensiveInvestmentStat {
    Attack,
    SpecialAttack,
    Defense,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoSpread {
    pub rank: usize,
    pub nature: Nature,
    pub investment_stat: OffensiveInvestmentStat,
    pub sps: StatPoints,
    pub sp_line: String,
    pub final_stats: FinalStats,
    pub total_points: u16,
    pub result: DamageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoSearchResult {
    pub matches: Vec<KoSpread>,
    pub closest_miss: Option<KoSpread>,
}

/// Score-based ranking, not minimum investment. For threshold constraints use
/// `survival::survival_search` (including independent multiple benchmarks).
pub fn optimize_defensive(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    search: SpreadSearch,
    limit: usize,
) -> Result<Vec<RankedSpread>, OptimizeError> {
    optimize(data, benchmarks, search, limit, OptimizationMode::Defensive)
}

/// Score-based ranking, not minimum investment.
pub fn optimize_offensive(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    search: SpreadSearch,
    limit: usize,
) -> Result<Vec<RankedSpread>, OptimizeError> {
    optimize(data, benchmarks, search, limit, OptimizationMode::Offensive)
}

pub fn minimize_hp_def_survival(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    max_ko_chance: f32,
    limit: usize,
) -> Result<Vec<SurvivalSpread>, OptimizeError> {
    Ok(hp_def_survival_search(data, benchmark, natures, max_ko_chance, limit)?.matches)
}

pub fn hp_def_survival_search(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    max_ko_chance: f32,
    limit: usize,
) -> Result<SurvivalSearchResult, OptimizeError> {
    hp_def_survival_search_from_hp_percent(data, benchmark, natures, max_ko_chance, 100.0, limit)
}

pub fn hp_def_survival_search_from_hp_percent(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    max_ko_chance: f32,
    hp_percent: f32,
    limit: usize,
) -> Result<SurvivalSearchResult, OptimizeError> {
    hp_def_survival_search_with_options(
        data,
        benchmark,
        natures,
        max_ko_chance,
        hp_percent,
        &crate::survival::SurvivalSearchOptions {
            limit,
            ..Default::default()
        },
    )
}

pub fn hp_def_survival_search_with_options(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    max_ko_chance: f32,
    hp_percent: f32,
    options: &crate::survival::SurvivalSearchOptions,
) -> Result<SurvivalSearchResult, OptimizeError> {
    let result = crate::survival::survival_search(
        data,
        std::slice::from_ref(benchmark),
        natures,
        &[max_ko_chance],
        hp_percent,
        options,
        &crate::survival::SurvivalEvaluation::Independent,
    )?;
    let convert = |spread: crate::survival::ExactSurvivalSpread| SurvivalSpread {
        rank: spread.rank,
        nature: spread.nature,
        sps: spread.sps,
        sp_line: spread.sp_line,
        final_stats: spread.final_stats,
        total_points: spread.total_points,
        result: spread.results.into_iter().next().expect("one benchmark"),
    };
    Ok(SurvivalSearchResult {
        matches: result.matches.into_iter().map(convert).collect(),
        closest_miss: result.closest_miss.map(convert),
    })
}

/// Compatibility entry point: each attack ends a turn, including the final one.
pub fn hp_def_combined_survival_search(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    natures: &[Nature],
    max_ko_chance: f32,
    hp_percent: f32,
    limit: usize,
) -> Result<CombinedSurvivalSearchResult, OptimizeError> {
    hp_def_combined_survival_search_with_options(
        data,
        benchmarks,
        natures,
        max_ko_chance,
        hp_percent,
        &crate::survival::SurvivalSearchOptions {
            limit,
            ..Default::default()
        },
        &vec![true; benchmarks.len()],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn hp_def_combined_survival_search_with_options(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    natures: &[Nature],
    max_ko_chance: f32,
    hp_percent: f32,
    options: &crate::survival::SurvivalSearchOptions,
    end_turn_after: &[bool],
) -> Result<CombinedSurvivalSearchResult, OptimizeError> {
    let result = crate::survival::survival_search(
        data,
        benchmarks,
        natures,
        &[max_ko_chance],
        hp_percent,
        options,
        &crate::survival::SurvivalEvaluation::Sequence {
            end_turn_after: end_turn_after.to_vec(),
        },
    )?;
    let convert = |spread: crate::survival::ExactSurvivalSpread| CombinedSurvivalSpread {
        rank: spread.rank,
        nature: spread.nature,
        sps: spread.sps,
        sp_line: spread.sp_line,
        final_stats: spread.final_stats,
        total_points: spread.total_points,
        hits: spread.results,
        combined: spread.sequence.expect("sequence evaluation"),
    };
    Ok(CombinedSurvivalSearchResult {
        matches: result.matches.into_iter().map(convert).collect(),
        closest_miss: result.closest_miss.map(convert),
    })
}

pub fn minimize_offensive_ko(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    min_ko_chance: f32,
    limit: usize,
) -> Result<Vec<KoSpread>, OptimizeError> {
    Ok(offensive_ko_search(data, benchmark, natures, min_ko_chance, limit)?.matches)
}

pub fn offensive_ko_search(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    natures: &[Nature],
    min_ko_chance: f32,
    limit: usize,
) -> Result<KoSearchResult, OptimizeError> {
    crate::survival::validate_probability(min_ko_chance)?;
    let natures = canonicalize_natures(natures);
    if natures.is_empty() || limit == 0 {
        return Err(crate::survival::invalid(
            "nature list and result limit must be nonempty",
        ));
    }
    let species = data.species(&benchmark.attacker.species)?;
    let move_data =
        crate::damage_bridge::build_move(data, &benchmark.move_name, &benchmark.attacker)?;
    let investment_stat = if move_data.name == "Body Press" {
        OffensiveInvestmentStat::Defense
    } else if move_data.name == "Foul Play" {
        OffensiveInvestmentStat::None
    } else {
        match move_data.category {
            damage_calc::Category::Physical => OffensiveInvestmentStat::Attack,
            damage_calc::Category::Special => OffensiveInvestmentStat::SpecialAttack,
            _ => {
                return Err(crate::survival::invalid(
                    "KO search requires a damaging move",
                ))
            }
        }
    };
    benchmark.attacker.stat_points.validate()?;
    let mut matches = Vec::new();
    let mut misses = Vec::new();
    let prepared = prepare_benchmark(data, benchmark)?;

    for nature in &natures {
        for points in 0..=32 {
            if investment_stat == OffensiveInvestmentStat::None && points != 0 {
                continue;
            }
            let mut sps = benchmark.attacker.stat_points;
            match investment_stat {
                OffensiveInvestmentStat::Attack => sps.attack = points,
                OffensiveInvestmentStat::SpecialAttack => sps.special_attack = points,
                OffensiveInvestmentStat::Defense => sps.defense = points,
                OffensiveInvestmentStat::None => {}
            }
            if sps.total() > crate::stats::MAX_TOTAL_STAT_POINTS {
                continue;
            }
            let mut candidate = prepared.clone();
            candidate.attacker.nature = *nature;
            candidate.attacker.stat_points = sps.into();

            let mut result = evaluate_input(candidate)?;
            let ko_chance = crate::survival::ko_probability(&result)?;
            result.ko_chance = Some(ko_chance);
            let spread = KoSpread {
                rank: 0,
                nature: *nature,
                investment_stat,
                sps,
                sp_line: build_champions_sp_line(sps),
                final_stats: champions_final_stats(species.base_stats(), *nature, sps)?,
                total_points: sps.total(),
                result: DamageSummary::from(result),
            };
            if ko_chance >= min_ko_chance {
                matches.push(spread);
            } else {
                misses.push(spread);
            }
        }
    }

    matches.sort_by(|left, right| {
        left.total_points
            .cmp(&right.total_points)
            .then_with(|| {
                right
                    .result
                    .ko_chance
                    .partial_cmp(&left.result.ko_chance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.result.min_damage.cmp(&left.result.min_damage))
    });
    matches.truncate(limit);
    for (index, spread) in matches.iter_mut().enumerate() {
        spread.rank = index + 1;
    }

    misses.sort_by(|left, right| {
        right
            .result
            .ko_chance
            .partial_cmp(&left.result.ko_chance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .result
                    .percent_min
                    .partial_cmp(&left.result.percent_min)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.result.min_damage.cmp(&left.result.min_damage))
            .then_with(|| left.total_points.cmp(&right.total_points))
    });
    let mut closest_miss = misses.into_iter().next();
    if let Some(spread) = &mut closest_miss {
        spread.rank = 1;
    }

    Ok(KoSearchResult {
        matches,
        closest_miss,
    })
}

pub fn all_natures() -> [Nature; 25] {
    [
        Nature::Adamant,
        Nature::Bashful,
        Nature::Bold,
        Nature::Brave,
        Nature::Calm,
        Nature::Careful,
        Nature::Docile,
        Nature::Gentle,
        Nature::Hardy,
        Nature::Hasty,
        Nature::Impish,
        Nature::Jolly,
        Nature::Lax,
        Nature::Lonely,
        Nature::Mild,
        Nature::Modest,
        Nature::Naive,
        Nature::Naughty,
        Nature::Quiet,
        Nature::Quirky,
        Nature::Rash,
        Nature::Relaxed,
        Nature::Sassy,
        Nature::Serious,
        Nature::Timid,
    ]
}

/// The canonical representative of a nature.
///
/// The five neutral natures (Hardy, Docile, Serious, Bashful, Quirky) are
/// stat-identical, so the optimizer folds every one of them onto Hardy. Raw set
/// parsing and the stat calculator keep the original names.
pub(crate) fn canonical_nature(nature: Nature) -> Nature {
    match nature {
        Nature::Bashful | Nature::Docile | Nature::Serious | Nature::Quirky => Nature::Hardy,
        other => other,
    }
}

/// Canonicalizes a nature list and removes duplicates, keeping first-seen order.
pub(crate) fn canonicalize_natures(natures: &[Nature]) -> Vec<Nature> {
    let mut canonical = Vec::with_capacity(natures.len());
    for &nature in natures {
        let nature = canonical_nature(nature);
        if !canonical.contains(&nature) {
            canonical.push(nature);
        }
    }
    canonical
}

/// The all-nature optimizer domain: 21 natures after collapsing the neutrals.
pub(crate) fn canonical_natures() -> Vec<Nature> {
    canonicalize_natures(&all_natures())
}

/// Exact nature optimization does not discard natures based on move category.
pub fn optimized_offensive_natures(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
) -> Result<Vec<Nature>, OptimizeError> {
    data.move_data(&benchmark.move_name)?;
    Ok(canonical_natures())
}

pub fn optimized_defensive_natures(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
) -> Result<Vec<Nature>, OptimizeError> {
    data.move_data(&benchmark.move_name)?;
    Ok(canonical_natures())
}

pub fn optimized_combined_defensive_natures(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
) -> Result<Vec<Nature>, OptimizeError> {
    for benchmark in benchmarks {
        data.move_data(&benchmark.move_name)?;
    }
    Ok(canonical_natures())
}

fn optimize(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    mut search: SpreadSearch,
    limit: usize,
    mode: OptimizationMode,
) -> Result<Vec<RankedSpread>, OptimizeError> {
    // A defensive stat the pinned engine cannot read never changes the score, so it is
    // locked to its requested value (zero by default) instead of being varied. Defensive
    // ranking varies the defender's spread, offensive ranking the attacker's. A
    // full-spend request must still reach its exact total, so the pin only applies when
    // the total is a cap.
    if search.exact_total.is_none() {
        let relevance = match mode {
            OptimizationMode::Defensive => {
                crate::relevance::combined_defender_relevance(data, benchmarks)?
            }
            OptimizationMode::Offensive => {
                crate::relevance::combined_attacker_relevance(data, benchmarks)?
            }
        };
        if !relevance.defense {
            search.locked.defense = Some(search.locked.defense.unwrap_or(0));
        }
        if !relevance.special_defense {
            search.locked.special_defense = Some(search.locked.special_defense.unwrap_or(0));
        }
    }
    let mut ranked = Vec::new();
    let prepared = benchmarks
        .iter()
        .map(|benchmark| prepare_benchmark(data, benchmark))
        .collect::<Result<Vec<_>, _>>()?;

    for sps in generate_spreads(search) {
        let mut summaries = Vec::with_capacity(benchmarks.len());
        let mut score = 0.0;
        let final_stats = match mode {
            OptimizationMode::Defensive => {
                let Some(first) = benchmarks.first() else {
                    continue;
                };
                let species = data.species(&first.defender.species)?;
                champions_final_stats(species.base_stats(), first.defender.nature, sps)?
            }
            OptimizationMode::Offensive => {
                let Some(first) = benchmarks.first() else {
                    continue;
                };
                let species = data.species(&first.attacker.species)?;
                champions_final_stats(species.base_stats(), first.attacker.nature, sps)?
            }
        };

        for benchmark in &prepared {
            let mut candidate = benchmark.clone();
            match mode {
                OptimizationMode::Defensive => {
                    candidate.defender.stat_points = sps.into();
                }
                OptimizationMode::Offensive => {
                    candidate.attacker.stat_points = sps.into();
                }
            }

            let result = evaluate_input(candidate)?;
            score += score_result(mode, &result)?;
            summaries.push(DamageSummary::from(result));
        }

        ranked.push(RankedSpread {
            rank: 0,
            mode,
            sps,
            sp_line: build_champions_sp_line(sps),
            final_stats,
            score,
            results: summaries,
        });
    }

    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.sps.total().cmp(&right.sps.total()))
    });
    ranked.truncate(limit);
    for (index, spread) in ranked.iter_mut().enumerate() {
        spread.rank = index + 1;
    }
    Ok(ranked)
}

fn score_result(mode: OptimizationMode, result: &DamageResult) -> Result<f64, OptimizeError> {
    let ko = crate::survival::ko_probability(result)? as f64;
    Ok(match mode {
        OptimizationMode::Defensive => {
            let max_percent = result.percent_range.1 as f64;
            (1.0 - ko) * 10_000.0 - max_percent
        }
        OptimizationMode::Offensive => {
            let min_percent = result.percent_range.0 as f64;
            ko * 10_000.0 + min_percent
        }
    })
}

impl From<DamageResult> for DamageSummary {
    fn from(value: DamageResult) -> Self {
        Self {
            min_damage: value.min_damage,
            max_damage: value.max_damage,
            percent_min: value.percent_range.0,
            percent_max: value.percent_range.1,
            ko_chance: value.ko_chance,
        }
    }
}

pub(crate) fn current_hp_from_percent(max_hp: u16, hp_percent: f32) -> u16 {
    let percent = hp_percent.clamp(0.0, 100.0);
    let hp = (f64::from(max_hp) * f64::from(percent) / 100.0).ceil() as u16;
    hp.clamp(1, max_hp)
}

/// Dynamic programming over the complete state of the supported sequence model.
#[allow(clippy::too_many_arguments)]
pub(crate) fn sequence_damage_summary(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
    nature: Nature,
    sps: StatPoints,
    max_hp: u16,
    starting_hp: u16,
    end_turn_after: &[bool],
    damage: &mut [crate::search_damage::SearchDamage],
) -> Result<CombinedDamageSummary, OptimizeError> {
    let defender_item = benchmarks
        .first()
        .and_then(|b| b.defender.item.as_deref())
        .map(parse_item)
        .transpose()?
        .unwrap_or(Item::None);
    let mut states = std::collections::BTreeMap::new();
    states.insert(
        SequenceState {
            hp: starting_hp,
            item: defender_item,
            toxic_counter: 1,
        },
        SequenceMass {
            probability: 1.0,
            min_damage: 0,
            max_damage: 0,
        },
    );
    let mut ko_probability = 0.0;
    let mut min_damage = u16::MAX;
    let mut max_damage = 0;
    for (index, benchmark) in benchmarks.iter().enumerate() {
        let mut next = std::collections::BTreeMap::new();
        for (state, mass) in states {
            let result = damage[index].calculate(nature, sps, state.hp, state.item)?;
            if !matches!(
                result.outcome,
                damage_calc::DamageOutcome::Damage
                    | damage_calc::DamageOutcome::Fixed
                    | damage_calc::DamageOutcome::ImmuneOrFailed
            ) {
                return Err(crate::survival::invalid(
                    "sequence model supports damaging or immune moves only",
                ));
            }
            let rolls = if result.damage_rolls.is_empty() {
                if result.outcome == damage_calc::DamageOutcome::ImmuneOrFailed {
                    vec![0]
                } else {
                    return Err(OptimizeError::UnknownKoProbability);
                }
            } else {
                result.damage_rolls
            };
            let probability = mass.probability / rolls.len() as f64;
            for damage in rolls {
                let low = mass.min_damage.saturating_add(damage);
                let high = mass.max_damage.saturating_add(damage);
                let mut after = state;
                if state.item == Item::FocusSash && state.hp == max_hp && damage >= state.hp {
                    after.hp = 1;
                    after.item = Item::None;
                } else {
                    after.hp = state.hp.saturating_sub(damage);
                }
                let after = if after.hp == 0 {
                    None
                } else if end_turn_after.get(index).copied().unwrap_or(false) {
                    apply_end_turn_effects(data, benchmark, nature, sps, max_hp, after)?
                } else {
                    Some(after)
                };
                if let Some(after) = after {
                    let entry = next.entry(after).or_insert(SequenceMass {
                        probability: 0.0,
                        min_damage: u16::MAX,
                        max_damage: 0,
                    });
                    entry.probability += probability;
                    entry.min_damage = entry.min_damage.min(low);
                    entry.max_damage = entry.max_damage.max(high);
                } else {
                    ko_probability += probability;
                    min_damage = min_damage.min(low);
                    max_damage = max_damage.max(high);
                }
            }
        }
        states = next;
    }
    for mass in states.values() {
        min_damage = min_damage.min(mass.min_damage);
        max_damage = max_damage.max(mass.max_damage);
    }
    if min_damage == u16::MAX {
        min_damage = 0;
    }
    Ok(CombinedDamageSummary {
        min_damage,
        max_damage,
        percent_min: min_damage as f32 * 100.0 / max_hp as f32,
        percent_max: max_damage as f32 * 100.0 / max_hp as f32,
        ko_chance: (ko_probability as f32).clamp(0.0, 1.0),
        starting_hp,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SequenceState {
    hp: u16,
    item: Item,
    toxic_counter: u8,
}

// Item has no Ord implementation. Its canonical Debug name is stable in this pinned engine.
impl Ord for SequenceState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.hp, self.toxic_counter)
            .cmp(&(other.hp, other.toxic_counter))
            .then_with(|| format!("{:?}", self.item).cmp(&format!("{:?}", other.item)))
    }
}
impl PartialOrd for SequenceState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
struct SequenceMass {
    probability: f64,
    min_damage: u16,
    max_damage: u16,
}

fn apply_end_turn_effects(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
    nature: Nature,
    sps: StatPoints,
    max_hp: u16,
    mut state: SequenceState,
) -> Result<Option<SequenceState>, OptimizeError> {
    if state.hp == 0 {
        return Ok(None);
    }

    let mut defender_set = benchmark.defender.clone();
    defender_set.nature = nature;
    defender_set.stat_points = sps;
    let mut defender = build_pokemon(data, &defender_set)?;
    defender.item = state.item;

    let residual_sixteenth = max_hp / 16;
    let residual_eighth = max_hp / 8;
    let magic_guard = defender.ability == Ability::MagicGuard;
    let mut healing_or_damage = 0i16;

    match benchmark.field.weather {
        Weather::Sun | Weather::HarshSun => {
            if matches!(defender.ability, Ability::DrySkin | Ability::SolarPower) {
                healing_or_damage -= residual_eighth as i16;
            }
        }
        Weather::Rain | Weather::HeavyRain => {
            if defender.ability == Ability::DrySkin {
                healing_or_damage += residual_eighth as i16;
            } else if defender.ability == Ability::RainDish {
                healing_or_damage += residual_sixteenth as i16;
            }
        }
        Weather::Sand => {
            if !magic_guard
                && !defender.has_type(PokemonType::Rock)
                && !defender.has_type(PokemonType::Ground)
                && !defender.has_type(PokemonType::Steel)
                && !matches!(
                    defender.ability,
                    Ability::Overcoat | Ability::SandForce | Ability::SandRush | Ability::SandVeil
                )
            {
                healing_or_damage -= residual_sixteenth as i16;
            }
        }
        Weather::Hail => {
            if defender.ability == Ability::IceBody {
                healing_or_damage += residual_sixteenth as i16;
            } else if !magic_guard
                && !defender.has_type(PokemonType::Ice)
                && !matches!(defender.ability, Ability::Overcoat | Ability::SnowCloak)
            {
                healing_or_damage -= residual_sixteenth as i16;
            }
        }
        Weather::Snow => {
            if defender.ability == Ability::IceBody {
                healing_or_damage += residual_sixteenth as i16;
            }
        }
        Weather::None | Weather::StrongWinds => {}
    }

    if healing_or_damage > 0 {
        state.hp = state
            .hp
            .saturating_add(healing_or_damage as u16)
            .min(max_hp);
    } else if healing_or_damage < 0 {
        state.hp = state.hp.saturating_sub((-healing_or_damage) as u16);
        if state.hp == 0 {
            return Ok(None);
        }
    }

    if state.item == Item::Leftovers {
        state.hp = state.hp.saturating_add(residual_sixteenth).min(max_hp);
    }

    if benchmark.field.terrain == Terrain::Grassy && is_grounded(&defender, &benchmark.field) {
        state.hp = state.hp.saturating_add(residual_sixteenth).min(max_hp);
    }

    if !magic_guard {
        match defender.status {
            StatusCondition::Poisoned => {
                if defender.ability == Ability::PoisonHeal {
                    state.hp = state.hp.saturating_add(residual_eighth).min(max_hp);
                } else {
                    state.hp = state.hp.saturating_sub(residual_eighth);
                }
            }
            StatusCondition::BadlyPoisoned => {
                if defender.ability == Ability::PoisonHeal {
                    state.hp = state.hp.saturating_add(residual_eighth).min(max_hp);
                } else {
                    let toxic_counter = state.toxic_counter.max(1);
                    state.hp = state
                        .hp
                        .saturating_sub(max_hp.saturating_mul(toxic_counter as u16) / 16);
                    state.toxic_counter = toxic_counter.saturating_add(1);
                }
            }
            StatusCondition::Burned => {
                let burn_damage = if defender.ability == Ability::Heatproof {
                    max_hp / 16 / 2
                } else {
                    max_hp / 16
                };
                state.hp = state.hp.saturating_sub(burn_damage);
            }
            StatusCondition::Healthy
            | StatusCondition::Paralyzed
            | StatusCondition::Asleep
            | StatusCondition::Drowsy
            | StatusCondition::Frozen => {}
        }
    }

    if benchmark.field.defender_leech_seed && !magic_guard && !defender.has_type(PokemonType::Grass)
    {
        state.hp = state.hp.saturating_sub(residual_eighth);
    }

    if state.hp == 0 {
        Ok(None)
    } else {
        Ok(Some(state))
    }
}

fn is_grounded(pokemon: &damage_calc::Pokemon, field: &damage_calc::Field) -> bool {
    field.gravity
        || pokemon.item == Item::IronBall
        || (pokemon.ability != Ability::Levitate
            && pokemon.item != Item::AirBalloon
            && !pokemon.has_type(PokemonType::Flying))
}
