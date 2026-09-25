use crate::damage_bridge::{calculate_benchmark, DamageBenchmark};
use crate::data::{ChampionsData, DataError};
use crate::optimize::{
    offensive_ko_search, optimize_defensive, optimize_offensive, optimized_offensive_natures,
    CombinedSurvivalSpread, DamageSummary, KoSpread, OptimizeError, RankedSpread, SurvivalSpread,
};
use crate::showdown::{parse_set, ShowdownError};
use crate::spreads::{LockedStats, SpreadSearch};
use crate::survival::{
    survival_natures, survival_search, ExactSurvivalSearchResult, SurvivalEvaluation,
    SurvivalSearchOptions,
};
use damage_calc::{Boosts, DamageResult, Field, Format, Nature, SideConditions, Terrain, Weather};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error(transparent)]
    Data(#[from] DataError),
    #[error(transparent)]
    Showdown(#[from] ShowdownError),
    #[error(transparent)]
    Optimize(#[from] OptimizeError),
    #[error(transparent)]
    Bridge(#[from] crate::damage_bridge::BridgeError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageRequest {
    pub attacker_set: String,
    pub defender_set: String,
    pub move_name: String,
    pub move_times_affected: u8,
    #[serde(default)]
    pub critical: bool,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageResponse {
    pub summary: DamageSummary,
    pub rolls: Vec<u16>,
    pub outcome: damage_calc::DamageOutcome,
    pub resolved_move: Option<damage_calc::ResolvedMove>,
    pub defender_hp_delta: Option<i32>,
    pub attacker_hp_effects: damage_calc::AttackerHpEffects,
    pub ko_chance_by_move_use: Vec<f32>,
}

/// Four moves on each side, calculated using upstream's shared preprocessing pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllMovesRequest {
    pub left_set: String,
    pub right_set: String,
    pub left_moves: [String; 4],
    pub right_moves: [String; 4],
    #[serde(default)]
    pub left_to_right_field: FieldRequest,
    #[serde(default)]
    pub right_to_left_field: FieldRequest,
}

pub fn calculate_all_moves_request(
    request: AllMovesRequest,
) -> Result<damage_calc::BatchDamageResult, ApiError> {
    calculate_all_moves_request_with_data(&ChampionsData::load()?, request)
}

pub fn calculate_all_moves_request_with_data(
    data: &ChampionsData,
    request: AllMovesRequest,
) -> Result<damage_calc::BatchDamageResult, ApiError> {
    let left_set = parse_set(&request.left_set)?;
    let right_set = parse_set(&request.right_set)?;
    let mut left = crate::damage_bridge::build_pokemon(data, &left_set)?;
    let mut right = crate::damage_bridge::build_pokemon(data, &right_set)?;
    left.boosts = request.left_to_right_field.attacker_boosts.into_boosts();
    right.boosts = request.left_to_right_field.defender_boosts.into_boosts();
    if request.left_to_right_field.fairy_aura {
        left.ability = damage_calc::Ability::FairyAura;
    }
    if request.right_to_left_field.fairy_aura {
        right.ability = damage_calc::Ability::FairyAura;
    }
    let moves = |names: &[String; 4],
                 set: &crate::showdown::ParsedSet|
     -> Result<[damage_calc::Move; 4], DataError> {
        Ok([
            crate::damage_bridge::build_move(data, &names[0], set)?,
            crate::damage_bridge::build_move(data, &names[1], set)?,
            crate::damage_bridge::build_move(data, &names[2], set)?,
            crate::damage_bridge::build_move(data, &names[3], set)?,
        ])
    };
    damage_calc::calculate_all_moves(damage_calc::BatchCalcInput {
        left,
        right,
        left_moves: moves(&request.left_moves, &left_set)?,
        right_moves: moves(&request.right_moves, &right_set)?,
        left_to_right_field: request.left_to_right_field.into_field(),
        right_to_left_field: request.right_to_left_field.into_field(),
        ruleset: damage_calc::Ruleset::Champions,
    })
    .map_err(|error| crate::damage_bridge::BridgeError::Damage(error.to_string()).into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataResponse {
    pub species: Vec<String>,
    pub regulation: Vec<String>,
    pub items: Vec<String>,
    pub abilities: Vec<String>,
    pub moves: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeBenchmarkRequest {
    pub attacker_set: String,
    pub defender_set: String,
    pub move_name: String,
    pub move_times_affected: u8,
    #[serde(default)]
    pub critical: bool,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeRequest {
    pub benchmarks: Vec<OptimizeBenchmarkRequest>,
    pub full_spend: bool,
    pub locked: LockedStatsRequest,
    pub limit: usize,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct LockedStatsRequest {
    pub hp: Option<u16>,
    pub attack: Option<u16>,
    pub defense: Option<u16>,
    pub special_attack: Option<u16>,
    pub special_defense: Option<u16>,
    pub speed: Option<u16>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct FieldRequest {
    pub format: Option<Format>,
    pub weather: Option<Weather>,
    pub terrain: Option<Terrain>,
    #[serde(default)]
    pub gravity: bool,
    #[serde(default)]
    pub fairy_aura: bool,
    #[serde(default)]
    pub protect: bool,
    #[serde(default)]
    pub helping_hand: bool,
    #[serde(default)]
    pub attacker_tailwind: bool,
    #[serde(default)]
    pub defender_tailwind: bool,
    #[serde(default)]
    pub defender_reflect: bool,
    #[serde(default)]
    pub defender_light_screen: bool,
    #[serde(default)]
    pub defender_aurora_veil: bool,
    #[serde(default)]
    pub defender_friend_guard: bool,
    #[serde(default)]
    pub defender_leech_seed: bool,
    #[serde(default)]
    pub defender_aqua_ring: bool,
    #[serde(default)]
    pub ingrain: bool,
    #[serde(default)]
    pub defender_nightmare: bool,
    #[serde(default)]
    pub defender_curse: bool,
    #[serde(default)]
    pub defender_binding: bool,
    #[serde(default)]
    pub defender_sea_of_fire: bool,
    #[serde(default)]
    pub defender_stealth_rock: bool,
    #[serde(default)]
    pub defender_salt_cure: bool,
    #[serde(default)]
    pub defender_spikes: u8,
    #[serde(default)]
    pub attacker_boosts: BoostsRequest,
    #[serde(default)]
    pub defender_boosts: BoostsRequest,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BoostsRequest {
    #[serde(default)]
    pub attack: i8,
    #[serde(default)]
    pub defense: i8,
    #[serde(default)]
    pub special_attack: i8,
    #[serde(default)]
    pub special_defense: i8,
    #[serde(default)]
    pub speed: i8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpDefSurvivalRequest {
    #[serde(default)]
    pub search: Option<SurvivalSearchOptions>,
    pub attacker_set: String,
    pub defender_set: String,
    pub move_name: String,
    pub max_ko_chance: f32,
    pub hp_percent: Option<f32>,
    pub nature: Option<Nature>,
    pub optimize_nature: bool,
    pub limit: usize,
    pub move_times_affected: u8,
    #[serde(default)]
    pub critical: bool,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingHitRequest {
    pub attacker_set: String,
    pub move_name: String,
    pub move_times_affected: u8,
    #[serde(default)]
    pub critical: bool,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedHpDefSurvivalRequest {
    #[serde(default)]
    pub search: Option<SurvivalSearchOptions>,
    /// Omitted preserves legacy end-of-turn after every attack; [] means none.
    #[serde(default)]
    pub end_turn_after: Option<Vec<bool>>,
    pub defender_set: String,
    pub hits: Vec<IncomingHitRequest>,
    pub max_ko_chance: f32,
    pub hp_percent: Option<f32>,
    pub nature: Option<Nature>,
    pub optimize_nature: bool,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveKoRequest {
    pub attacker_set: String,
    pub defender_set: String,
    pub move_name: String,
    pub min_ko_chance: f32,
    pub nature: Option<Nature>,
    pub optimize_nature: bool,
    pub limit: usize,
    pub move_times_affected: u8,
    #[serde(default)]
    pub critical: bool,
    #[serde(default)]
    pub field: Option<FieldRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveKoResponse {
    pub best: Option<KoSpread>,
    pub matches: Vec<KoSpread>,
    pub closest_miss: Option<KoSpread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HpDefSurvivalResponse {
    pub best: Option<SurvivalSpread>,
    pub matches: Vec<SurvivalSpread>,
    pub closest_miss: Option<SurvivalSpread>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedHpDefSurvivalResponse {
    pub best: Option<CombinedSurvivalSpread>,
    pub matches: Vec<CombinedSurvivalSpread>,
    pub closest_miss: Option<CombinedSurvivalSpread>,
}

/// Exact minimum-investment API for independent constraints or an ordered sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivalRequest {
    pub defender_set: String,
    pub hits: Vec<IncomingHitRequest>,
    /// One per independent hit, or one for the entire sequence.
    pub max_ko_chances: Vec<f32>,
    #[serde(default)]
    pub hp_percent: Option<f32>,
    #[serde(default)]
    pub nature: Option<Nature>,
    #[serde(default)]
    pub optimize_nature: bool,
    #[serde(default)]
    pub search: SurvivalSearchOptions,
    #[serde(default)]
    pub evaluation: SurvivalEvaluation,
}

pub fn find_min_survival(request: SurvivalRequest) -> Result<ExactSurvivalSearchResult, ApiError> {
    find_min_survival_with_data(&ChampionsData::load()?, request)
}

pub fn find_min_survival_with_data(
    data: &ChampionsData,
    request: SurvivalRequest,
) -> Result<ExactSurvivalSearchResult, ApiError> {
    let defender = parse_set(&request.defender_set)?;
    let natures = survival_natures(
        defender.nature,
        request.nature,
        request.optimize_nature,
        &request.search,
    );
    let benchmarks = request
        .hits
        .into_iter()
        .map(|hit| {
            benchmark_from_sets(
                &hit.attacker_set,
                &request.defender_set,
                hit.move_name,
                hit.move_times_affected,
                hit.critical,
                hit.field,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(survival_search(
        data,
        &benchmarks,
        &natures,
        &request.max_ko_chances,
        request.hp_percent.unwrap_or(100.0),
        &request.search,
        &request.evaluation,
    )?)
}

impl HpDefSurvivalRequest {
    pub fn with_default_limit(mut self) -> Self {
        if self.limit == 0 {
            self.limit = 10;
        }
        self
    }
}

pub fn load_metadata() -> Result<MetadataResponse, ApiError> {
    let data = ChampionsData::load()?;
    let mut response = MetadataResponse {
        species: data.species_names().map(str::to_owned).collect(),
        regulation: data.regulation_m_c_names().map(str::to_owned).collect(),
        items: data.item_names().map(str::to_owned).collect(),
        abilities: data.ability_names().map(str::to_owned).collect(),
        moves: data.move_names().map(str::to_owned).collect(),
    };
    sort_unique(&mut response.species);
    sort_unique(&mut response.regulation);
    sort_unique(&mut response.items);
    sort_unique(&mut response.abilities);
    sort_unique(&mut response.moves);
    Ok(response)
}

pub fn calculate_damage_request(request: DamageRequest) -> Result<DamageResponse, ApiError> {
    let data = ChampionsData::load()?;
    calculate_damage_request_with_data(&data, request)
}

pub fn calculate_damage_request_with_data(
    data: &ChampionsData,
    request: DamageRequest,
) -> Result<DamageResponse, ApiError> {
    let benchmark = benchmark_from_sets(
        &request.attacker_set,
        &request.defender_set,
        request.move_name,
        request.move_times_affected,
        request.critical,
        request.field,
    )?;
    let result = calculate_benchmark(data, &benchmark)?;
    Ok(DamageResponse::from(result))
}

pub fn find_min_hp_def_survival(
    request: HpDefSurvivalRequest,
) -> Result<HpDefSurvivalResponse, ApiError> {
    let data = ChampionsData::load()?;
    find_min_hp_def_survival_with_data(&data, request)
}

pub fn find_min_hp_def_survival_with_data(
    data: &ChampionsData,
    request: HpDefSurvivalRequest,
) -> Result<HpDefSurvivalResponse, ApiError> {
    let request = request.with_default_limit();
    let benchmark = benchmark_from_sets(
        &request.attacker_set,
        &request.defender_set,
        request.move_name,
        request.move_times_affected,
        request.critical,
        request.field,
    )?;
    let options = request.search.unwrap_or_else(|| SurvivalSearchOptions {
        limit: request.limit,
        ..Default::default()
    });
    let natures = survival_natures(
        benchmark.defender.nature,
        request.nature,
        request.optimize_nature,
        &options,
    );
    let result = crate::optimize::hp_def_survival_search_with_options(
        data,
        &benchmark,
        &natures,
        request.max_ko_chance,
        request.hp_percent.unwrap_or(100.0),
        &options,
    )?;
    Ok(HpDefSurvivalResponse {
        best: result.matches.first().cloned(),
        matches: result.matches,
        closest_miss: result.closest_miss,
    })
}

pub fn run_defensive_optimization(request: OptimizeRequest) -> Result<Vec<RankedSpread>, ApiError> {
    let data = ChampionsData::load()?;
    run_defensive_optimization_with_data(&data, request)
}

pub fn run_defensive_optimization_with_data(
    data: &ChampionsData,
    request: OptimizeRequest,
) -> Result<Vec<RankedSpread>, ApiError> {
    let benchmarks = optimize_benchmarks_from_request(request.benchmarks)?;
    let search = spread_search_from_request(request.full_spend, request.locked);
    optimize_defensive(data, &benchmarks, search, default_limit(request.limit)).map_err(Into::into)
}

pub fn run_offensive_optimization(request: OptimizeRequest) -> Result<Vec<RankedSpread>, ApiError> {
    let data = ChampionsData::load()?;
    run_offensive_optimization_with_data(&data, request)
}

pub fn run_offensive_optimization_with_data(
    data: &ChampionsData,
    request: OptimizeRequest,
) -> Result<Vec<RankedSpread>, ApiError> {
    let benchmarks = optimize_benchmarks_from_request(request.benchmarks)?;
    let search = spread_search_from_request(request.full_spend, request.locked);
    optimize_offensive(data, &benchmarks, search, default_limit(request.limit)).map_err(Into::into)
}

pub fn find_min_combined_hp_def_survival(
    request: CombinedHpDefSurvivalRequest,
) -> Result<CombinedHpDefSurvivalResponse, ApiError> {
    let data = ChampionsData::load()?;
    find_min_combined_hp_def_survival_with_data(&data, request)
}

pub fn find_min_combined_hp_def_survival_with_data(
    data: &ChampionsData,
    request: CombinedHpDefSurvivalRequest,
) -> Result<CombinedHpDefSurvivalResponse, ApiError> {
    let limit = if request.limit == 0 {
        10
    } else {
        request.limit
    };
    let defender = parse_set(&request.defender_set)?;
    let benchmarks = request
        .hits
        .into_iter()
        .map(|hit| {
            let mut benchmark = DamageBenchmark::new(
                parse_set(&hit.attacker_set)?,
                defender.clone(),
                hit.move_name,
            );
            benchmark.move_times_affected = hit.move_times_affected;
            benchmark.critical = hit.critical;
            if let Some(field) = hit.field {
                benchmark.fairy_aura = field.fairy_aura;
                benchmark.attacker_boosts = Some(field.attacker_boosts.into_boosts());
                benchmark.defender_boosts = Some(field.defender_boosts.into_boosts());
                benchmark.field = field.into_field();
            }
            Ok(benchmark)
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let options = request.search.unwrap_or_else(|| SurvivalSearchOptions {
        limit,
        ..Default::default()
    });
    let natures = survival_natures(
        defender.nature,
        request.nature,
        request.optimize_nature,
        &options,
    );
    let end_turn_after = request
        .end_turn_after
        .unwrap_or_else(|| vec![true; benchmarks.len()]);
    let result = crate::optimize::hp_def_combined_survival_search_with_options(
        data,
        &benchmarks,
        &natures,
        request.max_ko_chance,
        request.hp_percent.unwrap_or(100.0),
        &options,
        &end_turn_after,
    )?;
    Ok(CombinedHpDefSurvivalResponse {
        best: result.matches.first().cloned(),
        matches: result.matches,
        closest_miss: result.closest_miss,
    })
}

pub fn find_min_offensive_ko(request: OffensiveKoRequest) -> Result<OffensiveKoResponse, ApiError> {
    let data = ChampionsData::load()?;
    find_min_offensive_ko_with_data(&data, request)
}

pub fn find_min_offensive_ko_with_data(
    data: &ChampionsData,
    request: OffensiveKoRequest,
) -> Result<OffensiveKoResponse, ApiError> {
    let limit = if request.limit == 0 {
        10
    } else {
        request.limit
    };
    let benchmark = benchmark_from_sets(
        &request.attacker_set,
        &request.defender_set,
        request.move_name,
        request.move_times_affected,
        request.critical,
        request.field,
    )?;
    let owned_natures;
    let natures = if let Some(nature) = request.nature {
        owned_natures = vec![nature];
        owned_natures.as_slice()
    } else if request.optimize_nature {
        owned_natures = optimized_offensive_natures(data, &benchmark)?.to_vec();
        owned_natures.as_slice()
    } else {
        owned_natures = vec![benchmark.attacker.nature];
        owned_natures.as_slice()
    };
    let result = offensive_ko_search(data, &benchmark, natures, request.min_ko_chance, limit)?;
    Ok(OffensiveKoResponse {
        best: result.matches.first().cloned(),
        matches: result.matches,
        closest_miss: result.closest_miss,
    })
}

fn benchmark_from_sets(
    attacker_set: &str,
    defender_set: &str,
    move_name: String,
    move_times_affected: u8,
    critical: bool,
    field: Option<FieldRequest>,
) -> Result<DamageBenchmark, ApiError> {
    let mut benchmark = DamageBenchmark::new(
        parse_set(attacker_set)?,
        parse_set(defender_set)?,
        move_name,
    );
    benchmark.move_times_affected = move_times_affected;
    benchmark.critical = critical;
    if let Some(field) = field {
        benchmark.fairy_aura = field.fairy_aura;
        benchmark.attacker_boosts = Some(field.attacker_boosts.into_boosts());
        benchmark.defender_boosts = Some(field.defender_boosts.into_boosts());
        benchmark.field = field.into_field();
    }
    Ok(benchmark)
}

fn optimize_benchmarks_from_request(
    benchmarks: Vec<OptimizeBenchmarkRequest>,
) -> Result<Vec<DamageBenchmark>, ApiError> {
    benchmarks
        .into_iter()
        .map(|benchmark| {
            benchmark_from_sets(
                &benchmark.attacker_set,
                &benchmark.defender_set,
                benchmark.move_name,
                benchmark.move_times_affected,
                benchmark.critical,
                benchmark.field,
            )
        })
        .collect()
}

impl FieldRequest {
    pub fn into_field(self) -> Field {
        let mut field = Field {
            format: self.format.unwrap_or(Format::Doubles),
            weather: self.weather.unwrap_or(Weather::None),
            terrain: self.terrain.unwrap_or(Terrain::None),
            gravity: self.gravity,
            protect: self.protect,
            helping_hand: self.helping_hand,
            attacker_tailwind: self.attacker_tailwind,
            defender_tailwind: self.defender_tailwind,
            defender_leech_seed: self.defender_leech_seed,
            defender_aqua_ring: self.defender_aqua_ring,
            ingrain: self.ingrain,
            defender_nightmare: self.defender_nightmare,
            defender_curse: self.defender_curse,
            defender_binding: self.defender_binding,
            defender_sea_of_fire: self.defender_sea_of_fire,

            defender_side: SideConditions {
                reflect: self.defender_reflect,
                light_screen: self.defender_light_screen,
                aurora_veil: self.defender_aurora_veil,
                friend_guard: self.defender_friend_guard,
                stealth_rock: self.defender_stealth_rock,
                salt_cure: self.defender_salt_cure,
                spikes: self.defender_spikes.min(3),
            },
            ..Field::default()
        };
        if field.format == Format::Singles {
            field.helping_hand = false;
        }
        field
    }
}

impl BoostsRequest {
    pub fn into_boosts(self) -> Boosts {
        Boosts {
            attack: self.attack.clamp(-6, 6),
            defense: self.defense.clamp(-6, 6),
            special_attack: self.special_attack.clamp(-6, 6),
            special_defense: self.special_defense.clamp(-6, 6),
            speed: self.speed.clamp(-6, 6),
        }
    }
}

fn spread_search_from_request(full_spend: bool, locked: LockedStatsRequest) -> SpreadSearch {
    let mut search = if full_spend {
        SpreadSearch::full_spend()
    } else {
        SpreadSearch::all_legal()
    };
    search.locked = LockedStats {
        hp: locked.hp,
        attack: locked.attack,
        defense: locked.defense,
        special_attack: locked.special_attack,
        special_defense: locked.special_defense,
        speed: locked.speed,
    };
    search
}

fn default_limit(limit: usize) -> usize {
    if limit == 0 {
        10
    } else {
        limit
    }
}

fn sort_unique(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

impl From<DamageResult> for DamageResponse {
    fn from(value: DamageResult) -> Self {
        Self {
            rolls: value.damage_rolls.clone(),
            outcome: value.outcome,
            resolved_move: value.resolved_move.clone(),
            defender_hp_delta: value.defender_hp_delta,
            attacker_hp_effects: value.attacker_hp_effects,
            ko_chance_by_move_use: value.ko_chance_by_move_use.clone(),
            summary: DamageSummary::from(value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn ability_damage(
        attacker: &str,
        defender: &str,
        move_name: &str,
        field: serde_json::Value,
    ) -> serde_json::Value {
        let request = serde_json::from_value(serde_json::json!({
            "attacker_set": attacker, "defender_set": defender,
            "move_name": move_name, "move_times_affected": 0, "field": field
        }))
        .unwrap();
        serde_json::to_value(calculate_damage_request(request).unwrap()).unwrap()
    }

    #[test]
    fn new_mega_offensive_abilities_respect_move_flags_and_toggle() {
        for (species, ability, boosted, unchanged) in [
            ("Mega Absol Z", "Sharpness", "Night Slash", "Sucker Punch"),
            ("Mega Golisopod", "Tough Claws", "Liquidation", "Surf"),
            ("Mega Salamence", "Aerilate", "Double-Edge", "Dragon Claw"),
            ("Mega Feraligatr", "Dragonize", "Double-Edge", "Liquidation"),
            (
                "Toxtricity (Amped Form)",
                "Punk Rock",
                "Overdrive",
                "Thunderbolt",
            ),
            ("Perrserker", "Steely Spirit", "Iron Head", "Close Combat"),
        ] {
            for (move_name, should_boost) in [(boosted, true), (unchanged, false)] {
                let mut damages = Vec::new();
                for enabled in [false, true] {
                    let attacker =
                        format!("{species}\nAbility: {ability}\nAbility Enabled: {enabled}");
                    let result = ability_damage(
                        &attacker,
                        "Milotic\nAbility: None",
                        move_name,
                        serde_json::json!({}),
                    );
                    damages.push(result["summary"]["max_damage"].as_u64().unwrap());
                }
                if should_boost {
                    assert!(
                        damages[1] > damages[0],
                        "{ability}: {move_name} {damages:?}"
                    );
                } else {
                    assert_eq!(damages[1], damages[0], "{ability}: {move_name}");
                }
            }
        }
    }

    #[test]
    fn levitate_immunity_and_piercing_drill_damage_modifier() {
        for (enabled, gravity, immune) in [
            (true, false, true),
            (false, false, false),
            (true, true, false),
        ] {
            let defender =
                format!("Mega Garchomp Z\nAbility: Levitate\nAbility Enabled: {enabled}");
            let result = ability_damage(
                "Garchomp\nAbility: None",
                &defender,
                "Earthquake",
                serde_json::json!({"gravity": gravity}),
            );
            assert_eq!(
                result["summary"]["max_damage"].as_u64().unwrap() == 0,
                immune
            );
        }
        for (move_name, contact) in [("Iron Head", true), ("Earthquake", false)] {
            let attacker = "Mega Excadrill\nAbility: Piercing Drill";
            let normal = ability_damage(attacker, "Milotic", move_name, serde_json::json!({}));
            let protected = ability_damage(
                attacker,
                "Milotic",
                move_name,
                serde_json::json!({"protect": true}),
            );
            let disabled = ability_damage(
                &format!("{attacker}\nAbility Enabled: false"),
                "Milotic",
                move_name,
                serde_json::json!({"protect": true}),
            );
            let normal = normal["summary"]["max_damage"].as_u64().unwrap();
            let protected = protected["summary"]["max_damage"].as_u64().unwrap();
            // The pinned engine models Protect's damage modifier, not ordinary
            // move blocking. Compare the ability modifier independently.
            let disabled = disabled["summary"]["max_damage"].as_u64().unwrap();
            if contact {
                assert!(protected.abs_diff(normal / 4) <= 1);
                assert!(disabled > protected);
            } else {
                assert_eq!(protected, disabled);
            }
        }
    }

    #[test]
    fn mega_sol_changes_only_its_users_moves() {
        let active = "Mega Meganium\nAbility: Mega Sol";
        let inactive = "Mega Meganium\nAbility: Mega Sol\nAbility Enabled: false";
        let sunny = ability_damage(
            inactive,
            "Milotic",
            "Weather Ball",
            serde_json::json!({"weather": "Sun"}),
        );
        let rain = serde_json::json!({"weather": "Rain"});
        let personal_sun = ability_damage(active, "Milotic", "Weather Ball", rain.clone());
        assert_eq!(personal_sun["rolls"], sunny["rolls"]);
        let incoming = ability_damage("Charizard", active, "Flamethrower", rain.clone());
        let baseline = ability_damage("Charizard", inactive, "Flamethrower", rain);
        assert_eq!(incoming["rolls"], baseline["rolls"]);
    }

    #[test]
    fn spicy_spray_burns_between_hits_and_thermal_exchange_prevents_it() {
        let defender = "Mega Scovillain\nAbility: Spicy Spray";
        let disabled = format!("{defender}\nAbility Enabled: false");
        for ability in ["None", "Thermal Exchange"] {
            let attacker = format!("Mega Baxcalibur\nAbility: {ability}");
            let normal = ability_damage(&attacker, &disabled, "Double Kick", serde_json::json!({}));
            let burned = ability_damage(&attacker, defender, "Double Kick", serde_json::json!({}));
            if ability == "None" {
                assert!(
                    burned["summary"]["max_damage"].as_u64().unwrap()
                        < normal["summary"]["max_damage"].as_u64().unwrap()
                );
            } else {
                assert_eq!(burned["rolls"], normal["rolls"]);
            }
        }
    }

    #[test]
    fn disabling_passive_abilities_is_distinct_from_conditional_activation() {
        let attacker = "Lucario\nAbility: Inner Focus";
        let disabled = ability_damage(
            attacker,
            "Mega Lucario Z\nAbility: Aura Guard\nAbility Enabled: false",
            "Close Combat",
            serde_json::json!({}),
        );
        let guarded = ability_damage(
            attacker,
            "Mega Lucario Z\nAbility: Aura Guard\nAbility On: false",
            "Close Combat",
            serde_json::json!({}),
        );
        for (normal, reduced) in disabled["rolls"]
            .as_array()
            .unwrap()
            .iter()
            .zip(guarded["rolls"].as_array().unwrap())
        {
            assert_eq!(reduced.as_u64().unwrap(), normal.as_u64().unwrap() / 2);
        }
        let immune = ability_damage(
            "Charizard",
            "Arcanine\nAbility: Flash Fire\nAbility On: false",
            "Flamethrower",
            serde_json::json!({}),
        );
        let vulnerable = ability_damage(
            "Charizard",
            "Arcanine\nAbility: Flash Fire\nAbility Enabled: false",
            "Flamethrower",
            serde_json::json!({}),
        );
        assert_eq!(immune["summary"]["max_damage"], 0);
        assert!(vulnerable["summary"]["max_damage"].as_u64().unwrap() > 0);
    }

    const KINGAMBIT: &str = "Kingambit\nAbility: Defiant\nSPs: 32 Atk\nAdamant Nature\n- Iron Head";
    const FLOETTE: &str = "Mega Floette\n- Protect";

    #[test]
    fn new_field_effects_deserialize_and_reach_engine() {
        let empty: FieldRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.into_field().defender_side.spikes, 0);
        let field: FieldRequest = serde_json::from_value(serde_json::json!({
            "defender_aqua_ring": true, "ingrain": true,
            "defender_nightmare": true, "defender_curse": true,
            "defender_binding": true, "defender_sea_of_fire": true,
            "defender_stealth_rock": true, "defender_salt_cure": true,
            "defender_spikes": 8
        }))
        .unwrap();
        let field = field.into_field();
        assert!(field.defender_aqua_ring && field.ingrain && field.defender_nightmare);
        assert!(field.defender_curse && field.defender_binding && field.defender_sea_of_fire);
        assert!(field.defender_side.stealth_rock && field.defender_side.salt_cure);
        assert_eq!(field.defender_side.spikes, 3);
    }

    #[test]
    fn batch_api_returns_four_moves_in_both_directions() {
        let result = calculate_all_moves_request(AllMovesRequest {
            left_set: KINGAMBIT.into(),
            right_set: FLOETTE.into(),
            left_moves: ["Iron Head", "Protect", "Kowtow Cleave", "Sucker Punch"].map(String::from),
            right_moves: ["Moonblast", "Protect", "Psychic", "Energy Ball"].map(String::from),
            left_to_right_field: FieldRequest::default(),
            right_to_left_field: FieldRequest::default(),
        })
        .unwrap();
        assert_eq!(result.left.len(), 4);
        assert_eq!(result.right.len(), 4);
        assert_eq!(result.left[1].outcome, damage_calc::DamageOutcome::Status);
        assert!(result.left[0].max_damage > 0);
    }

    #[test]
    fn finds_min_survival_spread_for_visualizer() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                search: None,
                attacker_set: KINGAMBIT.to_owned(),
                defender_set: FLOETTE.to_owned(),
                move_name: "Iron Head".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: None,
                optimize_nature: true,
                limit: 4,
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        let best = response.best.unwrap();
        assert_eq!(best.total_points, 24);
        assert_eq!(best.sps.hp, 4);
        assert_eq!(best.sps.defense, 20);
        assert_eq!(best.result.max_damage, 158);
        assert_eq!(best.result.ko_chance, Some(0.125));
    }

    #[test]
    fn applies_field_request_to_damage_calculation() {
        let data = ChampionsData::load().unwrap();
        let open = calculate_damage_request_with_data(
            &data,
            DamageRequest {
                attacker_set: KINGAMBIT.to_owned(),
                defender_set: FLOETTE.to_owned(),
                move_name: "Iron Head".to_owned(),
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();
        let reflected = calculate_damage_request_with_data(
            &data,
            DamageRequest {
                attacker_set: KINGAMBIT.to_owned(),
                defender_set: FLOETTE.to_owned(),
                move_name: "Iron Head".to_owned(),
                move_times_affected: 0,
                critical: false,
                field: Some(FieldRequest {
                    defender_reflect: true,
                    ..FieldRequest::default()
                }),
            },
        )
        .unwrap();

        assert!(reflected.summary.max_damage < open.summary.max_damage);
    }

    #[test]
    fn critical_flag_applies_critical_damage() {
        let data = ChampionsData::load().unwrap();
        let attacker = parse_set("Aerodactyl\nSPs: 32 Atk\n- Dual Wingbeat").unwrap();
        let defender = parse_set("Sneasler\nSPs: 0 HP / 21 Def\n- Protect").unwrap();
        let normal = calculate_benchmark(
            &data,
            &DamageBenchmark::new(attacker.clone(), defender.clone(), "Dual Wingbeat"),
        )
        .unwrap();
        let mut critical_benchmark = DamageBenchmark::new(attacker, defender, "Dual Wingbeat");
        critical_benchmark.critical = true;
        let critical = calculate_benchmark(&data, &critical_benchmark).unwrap();

        assert!(critical.max_damage > normal.max_damage);
        assert!(critical
            .applied_modifiers
            .iter()
            .any(|modifier| modifier.label == "critical"));
    }

    #[test]
    fn calculates_sneasler_close_combat_into_chople_kingambit() {
        let data = ChampionsData::load().unwrap();
        let response = calculate_damage_request_with_data(
            &data,
            DamageRequest {
                attacker_set:
                    "Sneasler @ White Herb\nAbility: Unburden\nSPs: 32 Atk\nAdamant Nature\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Chople Berry\nSPs: 2 HP / 9 Def\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        assert_eq!(
            response.rolls,
            vec![182, 182, 186, 188, 192, 192, 194, 198, 198, 200, 204, 206, 206, 210, 212, 216]
        );
        assert_eq!(response.summary.ko_chance, Some(1.0));
    }

    #[test]
    fn dual_wingbeat_counts_sitrus_healing_between_hits() {
        let data = ChampionsData::load().unwrap();
        let attacker = parse_set("Aerodactyl\nSPs: 32 Atk\n- Dual Wingbeat").unwrap();
        let no_item_defender = parse_set("Sneasler\nSPs: 0 HP / 21 Def\n- Protect").unwrap();
        let sitrus_defender =
            parse_set("Sneasler @ Sitrus Berry\nSPs: 0 HP / 21 Def\n- Protect").unwrap();

        let no_item = calculate_benchmark(
            &data,
            &DamageBenchmark::new(attacker.clone(), no_item_defender, "Dual Wingbeat"),
        )
        .unwrap();
        let sitrus = calculate_benchmark(
            &data,
            &DamageBenchmark::new(attacker, sitrus_defender, "Dual Wingbeat"),
        )
        .unwrap();

        assert_eq!(sitrus.hit_rolls.len(), 2);
        assert_eq!((sitrus.min_damage, sitrus.max_damage), (144, 172));
        assert_eq!(no_item.ko_chance, Some(0.62890625));
        // The pinned reference uses cumulative berry healing markers.
        assert_eq!(sitrus.ko_chance, Some(0.0));
    }

    #[test]
    fn focus_sash_search_follows_upstream_ko_projection() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                search: None,
                attacker_set:
                    "Sneasler\nAbility: Unburden\nSPs: 32 Atk\nAdamant Nature\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Focus Sash\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                max_ko_chance: 0.0,
                hp_percent: None,
                nature: Some(Nature::Hardy),
                optimize_nature: false,
                limit: 1,
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        // Upstream's current KO projection does not apply Focus Sash survival.
        assert!(response.best.is_none());
    }

    #[test]
    fn calculates_damage_calc_suffixed_sneasler_close_combat() {
        let data = ChampionsData::load().unwrap();
        let response = calculate_damage_request_with_data(
            &data,
            DamageRequest {
                attacker_set:
                    "Sneasler @ White Herb\nAbility: Unburden\nSPs: 10+ Atk\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Chople Berry\nSPs: 0 HP / 9 Def\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        assert_eq!(
            response.rolls,
            vec![162, 164, 164, 168, 168, 170, 174, 174, 176, 180, 180, 182, 186, 186, 188, 192]
        );
        assert_eq!(response.summary.ko_chance, Some(0.5));
    }

    #[test]
    fn defensive_min_uses_suffixed_attacker_nature() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                search: None,
                attacker_set:
                    "Sneasler @ White Herb\nAbility: Unburden\nSPs: 10+ Atk\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Chople Berry\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: Some(Nature::Adamant),
                optimize_nature: false,
                limit: 10,
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        assert!(response
            .matches
            .iter()
            .all(|spread| spread.total_points > 9));
    }

    #[test]
    fn defensive_min_uses_showdown_attacker_nature_line() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                search: None,
                attacker_set:
                    "Sneasler @ White Herb\nAbility: Unburden\nSPs: 10 Atk\nAdamant Nature\n- Close Combat"
                        .to_owned(),
                defender_set: "Kingambit @ Chople Berry\n- Protect".to_owned(),
                move_name: "Close Combat".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: Some(Nature::Adamant),
                optimize_nature: false,
                limit: 10,
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        assert!(response
            .matches
            .iter()
            .all(|spread| spread.total_points > 9));
    }

    #[test]
    fn defensive_min_treats_low_evs_as_champions_points() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                search: None,
                attacker_set: "Sneasler @ White Herb\nAbility: Unburden\nLevel: 50\nEVs: 20 HP / 10 Atk / 21 Def / 15 Spe\nAdamant Nature\n- Close Combat\n- Fake Out\n- Dire Claw\n- Protect"
                    .to_owned(),
                defender_set: "Kingambit @ Chople Berry\nAbility: Defiant\nSPs: 32 Atk\nAdamant Nature\n- Iron Head\n- Kowtow Cleave"
                    .to_owned(),
                move_name: "Close Combat".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: Some(Nature::Adamant),
                optimize_nature: false,
                limit: 10,
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        assert!(response
            .matches
            .iter()
            .all(|spread| spread.total_points > 9));
    }

    #[test]
    fn damage_benchmark_matches_known_calcs() {
        let data = ChampionsData::load().unwrap();
        let cases = [
            BenchmarkCase {
                name: "Sneasler Close Combat vs Chople Kingambit",
                attacker: "Sneasler @ White Herb\nAbility: Unburden\nSPs: 32+ Atk\n- Close Combat",
                defender: "Kingambit @ Chople Berry\nSPs: 2 HP / 9 Def\n- Protect",
                move_name: "Close Combat",
                field: None,
                expected_min: 182,
                expected_max: 216,
                expected_unique: &[182, 186, 188, 192, 194, 198, 200, 204, 206, 210, 212, 216],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Kingambit Iron Head vs max Mega Floette",
                attacker:
                    "Kingambit @ Black Glasses\nAbility: Defiant\nSPs: 32+ Atk\n- Iron Head",
                defender: "Mega Floette\nSPs: 32 HP / 32 Def\n- Protect",
                move_name: "Iron Head",
                field: None,
                expected_min: 134,
                expected_max: 158,
                expected_unique: &[134, 138, 140, 144, 146, 150, 152, 156, 158],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Magnet Transistor Pikachu Thunderbolt in Electric Terrain",
                attacker: "Pikachu @ Magnet\nAbility: Transistor\n- Thunderbolt",
                defender: "Milotic\n- Protect",
                move_name: "Thunderbolt",
                field: Some(FieldRequest {
                    terrain: Some(Terrain::Electric),
                    ..FieldRequest::default()
                }),
                expected_min: 102,
                expected_max: 120,
                expected_unique: &[102, 104, 108, 110, 114, 116, 120],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Charizard Rock Slide spread in Doubles",
                attacker: "Charizard\n- Rock Slide",
                defender: "Volcarona\n- Protect",
                move_name: "Rock Slide",
                field: None,
                expected_min: 104,
                expected_max: 124,
                expected_unique: &[104, 108, 112, 116, 120, 124],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Charizard Rock Slide single target in Doubles",
                attacker: "Charizard\nTarget: single\n- Rock Slide",
                defender: "Volcarona\n- Protect",
                move_name: "Rock Slide",
                field: None,
                expected_min: 140,
                expected_max: 168,
                expected_unique: &[140, 144, 148, 152, 156, 160, 164, 168],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Burned Machamp Drain Punch through Reflect",
                attacker: "Machamp\nStatus: Burned\n- Drain Punch",
                defender: "Snorlax\n- Protect",
                move_name: "Drain Punch",
                field: Some(FieldRequest {
                    defender_reflect: true,
                    ..FieldRequest::default()
                }),
                expected_min: 51,
                expected_max: 60,
                expected_unique: &[51, 52, 53, 54, 55, 56, 57, 58, 59, 60],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Ninetales Flamethrower in Sun",
                attacker: "Ninetales\n- Flamethrower",
                defender: "Scizor\n- Protect",
                move_name: "Flamethrower",
                field: Some(FieldRequest {
                    weather: Some(Weather::Sun),
                    ..FieldRequest::default()
                }),
                expected_min: 304,
                expected_max: 364,
                expected_unique: &[304, 312, 316, 324, 328, 336, 340, 348, 352, 360, 364],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Pelipper Weather Ball in Rain",
                attacker: "Pelipper\n- Weather Ball",
                defender: "Camerupt\n- Protect",
                move_name: "Weather Ball",
                field: Some(FieldRequest {
                    weather: Some(Weather::Rain),
                    ..FieldRequest::default()
                }),
                expected_min: 412,
                expected_max: 492,
                expected_unique: &[
                    412, 420, 424, 432, 436, 444, 448, 456, 460, 468, 472, 480, 484, 492,
                ],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Abomasnow Blizzard into active Multiscale Dragonite",
                attacker: "Abomasnow\n- Blizzard",
                defender: "Dragonite\nAbility: Multiscale\nAbility On: true\n- Protect",
                move_name: "Blizzard",
                field: None,
                expected_min: 86,
                expected_max: 104,
                expected_unique: &[86, 90, 92, 96, 98, 102, 104],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Gyarados Waterfall into Passho Torkoal",
                attacker: "Gyarados\n- Waterfall",
                defender: "Torkoal @ Passho Berry\n- Protect",
                move_name: "Waterfall",
                field: None,
                expected_min: 42,
                expected_max: 49,
                expected_unique: &[42, 43, 45, 46, 48, 49],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Gengar Shadow Ball into Kasib Clefable",
                attacker: "Gengar\n- Shadow Ball",
                defender: "Clefable @ Kasib Berry\n- Protect",
                move_name: "Shadow Ball",
                field: None,
                expected_min: 63,
                expected_max: 75,
                expected_unique: &[63, 64, 66, 67, 69, 70, 72, 73, 75],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Mega Kangaskhan Parental Bond Double-Edge",
                attacker: "Mega Kangaskhan\nAbility: Parental Bond\n- Double-Edge",
                defender: "Blastoise\n- Protect",
                move_name: "Double-Edge",
                field: None,
                expected_min: 101,
                expected_max: 121,
                expected_unique: &[101, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121],
                expected_roll_count: Some(256),
            },
            BenchmarkCase {
                name: "Skill Link Toucannon Bullet Seed",
                attacker: "Toucannon\nAbility: Skill Link\n- Bullet Seed",
                defender: "Slowbro\n- Protect",
                move_name: "Bullet Seed",
                field: None,
                expected_min: 110,
                expected_max: 130,
                expected_unique: &[110, 112, 114, 116, 118, 120, 122, 124, 126, 128, 130],
                expected_roll_count: Some(1_048_576),
            },
            BenchmarkCase {
                name: "Swift Swim Beartic Electro Ball in Rain",
                attacker: "Beartic\nAbility: Swift Swim\n- Electro Ball",
                defender: "Pelipper\n- Protect",
                move_name: "Electro Ball",
                field: Some(FieldRequest {
                    weather: Some(Weather::Rain),
                    ..FieldRequest::default()
                }),
                expected_min: 92,
                expected_max: 112,
                expected_unique: &[92, 96, 100, 104, 108, 112],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Analytic Starmie Psychic moving last",
                attacker: "Starmie\nAbility: Analytic\n- Psychic",
                defender: "Venusaur\n- Protect",
                move_name: "Psychic",
                field: Some(FieldRequest {
                    defender_tailwind: true,
                    ..FieldRequest::default()
                }),
                expected_min: 134,
                expected_max: 158,
                expected_unique: &[134, 138, 140, 144, 146, 150, 152, 156, 158],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Rivalry Luxray same gender",
                attacker: "Luxray\nAbility: Rivalry\nRivalry: same\n- Wild Charge",
                defender: "Pelipper\n- Protect",
                move_name: "Wild Charge",
                field: None,
                expected_min: 300,
                expected_max: 352,
                expected_unique: &[300, 304, 312, 316, 324, 328, 336, 340, 348, 352],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Rivalry Luxray opposite gender",
                attacker: "Luxray\nAbility: Rivalry\nRivalry: opposite\n- Wild Charge",
                defender: "Pelipper\n- Protect",
                move_name: "Wild Charge",
                field: None,
                expected_min: 180,
                expected_max: 216,
                expected_unique: &[180, 184, 192, 196, 204, 208, 216],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Fairy Aura Mega Floette Moonblast",
                attacker: "Mega Floette\nAbility: Fairy Aura\n- Moonblast",
                defender: "Hydreigon\n- Protect",
                move_name: "Moonblast",
                field: None,
                expected_min: 456,
                expected_max: 540,
                expected_unique: &[
                    456, 460, 468, 472, 480, 484, 492, 496, 504, 508, 516, 520, 528, 532,
                    540,
                ],
                expected_roll_count: None,
            },
            BenchmarkCase {
                name: "Sharpness Gallade Psycho Cut",
                attacker: "Gallade\nAbility: Sharpness\n- Psycho Cut",
                defender: "Toxapex\n- Protect",
                move_name: "Psycho Cut",
                field: None,
                expected_min: 102,
                expected_max: 120,
                expected_unique: &[102, 104, 108, 110, 114, 116, 120],
                expected_roll_count: None,
            },
        BenchmarkCase {
            name: "Supreme Overlord Kingambit with 3 fainted allies",
            attacker:
                "Kingambit\nAbility: Supreme Overlord\nSupreme Overlord Allies: 3\n- Kowtow Cleave",
            defender: "Gengar\n- Protect",
                move_name: "Kowtow Cleave",
                field: None,
                expected_min: 242,
                expected_max: 288,
                expected_unique: &[
                    242, 246, 248, 252, 254, 258, 260, 264, 266, 270, 272, 276, 278, 282,
                    284, 288,
            ],
            expected_roll_count: None,
        },
        BenchmarkCase {
            name: "Fire Mane Mega Pyroar Heat Wave",
            attacker: "Mega Pyroar\nAbility: Fire Mane\nSPs: 0+ SpA\n- Heat Wave",
            defender: "Aegislash (Shield Forme)\nSPs: 32 HP / 2 SpD\n- Protect",
            move_name: "Heat Wave",
            field: None,
            expected_min: 120,
            expected_max: 144,
            expected_unique: &[120, 122, 126, 128, 132, 134, 138, 140, 144],
            expected_roll_count: Some(16),
        },
    ];

        for case in cases {
            assert_benchmark_case(&data, case);
        }
    }

    #[test]
    fn finds_modest_min_survival_spread_for_visualizer() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_hp_def_survival_with_data(
            &data,
            HpDefSurvivalRequest {
                search: None,
                attacker_set: KINGAMBIT.to_owned(),
                defender_set: FLOETTE.to_owned(),
                move_name: "Iron Head".to_owned(),
                max_ko_chance: 0.125,
                hp_percent: None,
                nature: Some(Nature::Modest),
                optimize_nature: false,
                limit: 1,
                move_times_affected: 0,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        let best = response.best.unwrap();
        assert_eq!(best.nature, Nature::Modest);
        assert_eq!(best.total_points, 36);
        assert_eq!(best.sps.hp, 4);
        assert_eq!(best.sps.defense, 32);
    }

    #[test]
    fn finds_combined_survival_spread_for_visualizer() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_combined_hp_def_survival_with_data(
            &data,
            CombinedHpDefSurvivalRequest {
                search: None,
                end_turn_after: None,
                defender_set: FLOETTE.to_owned(),
                hits: vec![
                    IncomingHitRequest {
                        attacker_set: KINGAMBIT.to_owned(),
                        move_name: "Iron Head".to_owned(),
                        move_times_affected: 0,
                        critical: false,
                        field: None,
                    },
                    IncomingHitRequest {
                        attacker_set: KINGAMBIT.to_owned(),
                        move_name: "Iron Head".to_owned(),
                        move_times_affected: 0,
                        critical: false,
                        field: None,
                    },
                ],
                max_ko_chance: 1.0,
                hp_percent: Some(50.0),
                nature: Some(Nature::Bold),
                optimize_nature: false,
                limit: 1,
            },
        )
        .unwrap();

        let best = response.best.unwrap();
        assert_eq!(best.hits.len(), 2);
        assert_eq!(
            best.combined.starting_hp,
            (best.final_stats.hp as f32 * 0.5).ceil() as u16
        );
        assert!(best.combined.ko_chance > 0.0);
    }

    #[test]
    fn leftovers_recovery_counts_between_sequence_hits() {
        let data = ChampionsData::load().unwrap();
        let request = |defender_set: &str| CombinedHpDefSurvivalRequest {
            search: None,
            end_turn_after: None,
            defender_set: defender_set.to_owned(),
            hits: vec![
                IncomingHitRequest {
                    attacker_set: "Sneasler\nSPs: 0 Atk\n- Fake Out".to_owned(),
                    move_name: "Fake Out".to_owned(),
                    move_times_affected: 0,
                    critical: false,
                    field: None,
                },
                IncomingHitRequest {
                    attacker_set: "Sneasler\nSPs: 0 Atk\n- Fake Out".to_owned(),
                    move_name: "Fake Out".to_owned(),
                    move_times_affected: 0,
                    critical: false,
                    field: None,
                },
            ],
            max_ko_chance: 1.0,
            hp_percent: Some(10.0),
            nature: Some(Nature::Hardy),
            optimize_nature: false,
            limit: 1,
        };

        let no_item =
            find_min_combined_hp_def_survival_with_data(&data, request("Kingambit\n- Protect"))
                .unwrap();
        let leftovers = find_min_combined_hp_def_survival_with_data(
            &data,
            request("Kingambit @ Leftovers\n- Protect"),
        )
        .unwrap();

        assert_eq!(no_item.best.unwrap().combined.ko_chance, 0.51171875);
        assert_eq!(leftovers.best.unwrap().combined.ko_chance, 0.0);
    }

    #[test]
    fn poison_tick_counts_between_sequence_hits() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_combined_hp_def_survival_with_data(
            &data,
            CombinedHpDefSurvivalRequest {
                search: None,
                end_turn_after: None,
                defender_set: "Kingambit\nStatus: Poisoned\n- Protect".to_owned(),
                hits: vec![
                    IncomingHitRequest {
                        attacker_set: "Sneasler\nSPs: 0 Atk\n- Fake Out".to_owned(),
                        move_name: "Fake Out".to_owned(),
                        move_times_affected: 0,
                        critical: false,
                        field: None,
                    },
                    IncomingHitRequest {
                        attacker_set: "Sneasler\nSPs: 0 Atk\n- Fake Out".to_owned(),
                        move_name: "Fake Out".to_owned(),
                        move_times_affected: 0,
                        critical: false,
                        field: None,
                    },
                ],
                max_ko_chance: 1.0,
                hp_percent: Some(10.0),
                nature: Some(Nature::Hardy),
                optimize_nature: false,
                limit: 1,
            },
        )
        .unwrap();

        assert_eq!(response.best.unwrap().combined.ko_chance, 1.0);
    }

    #[test]
    fn leech_seed_tick_counts_between_sequence_hits() {
        let data = ChampionsData::load().unwrap();
        let seeded_field = Some(FieldRequest {
            defender_leech_seed: true,
            ..FieldRequest::default()
        });
        let response = find_min_combined_hp_def_survival_with_data(
            &data,
            CombinedHpDefSurvivalRequest {
                search: None,
                end_turn_after: None,
                defender_set: "Kingambit\n- Protect".to_owned(),
                hits: vec![
                    IncomingHitRequest {
                        attacker_set: "Sneasler\nSPs: 0 Atk\n- Fake Out".to_owned(),
                        move_name: "Fake Out".to_owned(),
                        move_times_affected: 0,
                        critical: false,
                        field: seeded_field,
                    },
                    IncomingHitRequest {
                        attacker_set: "Sneasler\nSPs: 0 Atk\n- Fake Out".to_owned(),
                        move_name: "Fake Out".to_owned(),
                        move_times_affected: 0,
                        critical: false,
                        field: seeded_field,
                    },
                ],
                max_ko_chance: 1.0,
                hp_percent: Some(10.0),
                nature: Some(Nature::Hardy),
                optimize_nature: false,
                limit: 1,
            },
        )
        .unwrap();

        assert_eq!(response.best.unwrap().combined.ko_chance, 1.0);
    }

    #[test]
    fn finds_min_offensive_ko_for_visualizer() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_offensive_ko_with_data(
            &data,
            OffensiveKoRequest {
                attacker_set: "Basculegion (Male)\nAbility: Adaptability\n- Last Respects"
                    .to_owned(),
                defender_set: "Aegislash (Shield Forme)\nSPs: 30 HP / 4 Def\n- Protect".to_owned(),
                move_name: "Last Respects".to_owned(),
                min_ko_chance: 1.0,
                nature: None,
                optimize_nature: true,
                limit: 1,
                move_times_affected: 1,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        let best = response.best.unwrap();
        assert_eq!(best.result.ko_chance, Some(1.0));
    }

    #[test]
    fn optimized_nature_mode_keeps_equivalent_boosted_natures() {
        let data = ChampionsData::load().unwrap();
        let response = find_min_offensive_ko_with_data(
            &data,
            OffensiveKoRequest {
                attacker_set: "Basculegion (Male)\nAbility: Adaptability\n- Last Respects"
                    .to_owned(),
                defender_set: "Aegislash (Shield Forme)\nSPs: 30 HP / 4 Def\n- Protect".to_owned(),
                move_name: "Last Respects".to_owned(),
                min_ko_chance: 1.0,
                nature: None,
                optimize_nature: true,
                limit: 10,
                move_times_affected: 1,
                critical: false,
                field: None,
            },
        )
        .unwrap();

        assert!(response.matches.iter().all(|spread| matches!(
            spread.nature,
            Nature::Adamant | Nature::Brave | Nature::Lonely | Nature::Naughty
        )));
        assert!(response
            .matches
            .iter()
            .any(|spread| spread.nature == Nature::Brave));
    }

    #[derive(Clone, Copy)]
    struct BenchmarkCase {
        name: &'static str,
        attacker: &'static str,
        defender: &'static str,
        move_name: &'static str,
        field: Option<FieldRequest>,
        expected_min: u16,
        expected_max: u16,
        expected_unique: &'static [u16],
        expected_roll_count: Option<usize>,
    }

    fn assert_benchmark_case(data: &ChampionsData, case: BenchmarkCase) {
        let response = calculate_damage_request_with_data(
            data,
            DamageRequest {
                attacker_set: case.attacker.to_owned(),
                defender_set: case.defender.to_owned(),
                move_name: case.move_name.to_owned(),
                move_times_affected: 0,
                critical: false,
                field: case.field,
            },
        )
        .unwrap_or_else(|error| panic!("{} failed: {error}", case.name));
        assert_eq!(
            response.summary.min_damage, case.expected_min,
            "{} min damage",
            case.name
        );
        assert_eq!(
            response.summary.max_damage, case.expected_max,
            "{} max damage",
            case.name
        );
        let unique = response
            .rolls
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(unique, case.expected_unique, "{} unique rolls", case.name);
        if let Some(expected_roll_count) = case.expected_roll_count {
            assert_eq!(
                response.rolls.len(),
                expected_roll_count,
                "{} roll count",
                case.name
            );
        }
    }
}
