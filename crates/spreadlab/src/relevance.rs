//! Defensive-stat relevance derived from the pinned damage engine.
//!
//! The pinned engine selects the defender's defensive stat in `damage.rs` as
//! `move.category == Physical || move.deals_physical_damage`, and the attacker's attack
//! stat as Attack, Special Attack, or Defense for Body Press. For a benchmark made of
//! ordinary single-category hits only one defender defense and one attacker attack stat
//! therefore enter the damage formula. Freely searching the others spends points on a
//! stat that cannot change the requested calculation, so they are pinned to their lock
//! or lower bound instead of being varied.
//!
//! Some engine mechanics do read more than one defensive stat. Whenever one of them may
//! apply, relevance falls back to [`DefensiveRelevance::BOTH`] and the search keeps the
//! previous exhaustive behavior:
//!
//! * `highest_stat` compares every modified stat while a paradox ability is active,
//!   and that comparison drives the 1.3x defense modifier, the attacker's offense
//!   boost, and both sides' speed, which feeds move-order base-power modifiers.
//! * `check_download` picks the attacker's boost from the defender's Defense and
//!   Special Defense, so for a Download attacker either stat can change damage.
//! * `check_trace` can hand either side the other side's ability, which can introduce
//!   the two mechanics above after a build step we do not model here.
//! * A Status move or a zero base power move never runs the damage formula, so no
//!   stat is proven irrelevant for it.

use crate::damage_bridge::{build_move, build_pokemon, DamageBenchmark};
use crate::data::ChampionsData;
use crate::optimize::OptimizeError;
use damage_calc::{Ability, Category, Move, Pokemon};

/// Which defensive stats of one side can affect a benchmark's calculation.
///
/// The flags are true when the corresponding stat may change the engine's result and
/// must stay a search dimension. Both flags false means no defensive stat of that side
/// is known to matter at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefensiveRelevance {
    pub defense: bool,
    pub special_defense: bool,
}

impl DefensiveRelevance {
    /// No defensive stat is known to matter. Fold identity, and the attacker-side
    /// answer for an ordinary move whose defense stat the engine never reads.
    pub const NONE: Self = Self {
        defense: false,
        special_defense: false,
    };
    /// Both defenses are kept.
    pub const BOTH: Self = Self {
        defense: true,
        special_defense: true,
    };
    /// Only Defense can affect the calculation.
    pub const DEFENSE: Self = Self {
        defense: true,
        special_defense: false,
    };
    /// Only Special Defense can affect the calculation.
    pub const SPECIAL_DEFENSE: Self = Self {
        defense: false,
        special_defense: true,
    };

    /// Combine two relevance answers; a stat stays relevant if either side needs it.
    pub fn union(self, other: Self) -> Self {
        Self {
            defense: self.defense || other.defense,
            special_defense: self.special_defense || other.special_defense,
        }
    }
}

/// Defender-side relevance for one benchmark: which of the defender's defensive stats
/// can change the incoming damage.
pub fn benchmark_defender_relevance(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
) -> Result<DefensiveRelevance, OptimizeError> {
    let move_ = build_move(data, &benchmark.move_name, &benchmark.attacker)?;
    let attacker = build_pokemon(data, &benchmark.attacker)?;
    let defender = build_pokemon(data, &benchmark.defender)?;
    Ok(defender_relevance(&move_, &attacker, &defender))
}

/// Attacker-side relevance for one benchmark: which of the attacker's defensive stats
/// can change the outgoing damage.
pub fn benchmark_attacker_relevance(
    data: &ChampionsData,
    benchmark: &DamageBenchmark,
) -> Result<DefensiveRelevance, OptimizeError> {
    let move_ = build_move(data, &benchmark.move_name, &benchmark.attacker)?;
    let attacker = build_pokemon(data, &benchmark.attacker)?;
    let defender = build_pokemon(data, &benchmark.defender)?;
    Ok(attacker_relevance(&move_, &attacker, &defender))
}

/// Union of the per-benchmark defender answers. An empty list keeps both defenses.
pub fn combined_defender_relevance(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
) -> Result<DefensiveRelevance, OptimizeError> {
    let mut relevance = DefensiveRelevance::NONE;
    for benchmark in benchmarks {
        relevance = relevance.union(benchmark_defender_relevance(data, benchmark)?);
    }
    // The defender answer is never `NONE`, so this only fires for an empty list.
    if relevance == DefensiveRelevance::NONE {
        relevance = DefensiveRelevance::BOTH;
    }
    Ok(relevance)
}

/// Union of the per-benchmark attacker answers. An empty list keeps both defenses.
pub fn combined_attacker_relevance(
    data: &ChampionsData,
    benchmarks: &[DamageBenchmark],
) -> Result<DefensiveRelevance, OptimizeError> {
    let mut relevance = DefensiveRelevance::NONE;
    for benchmark in benchmarks {
        relevance = relevance.union(benchmark_attacker_relevance(data, benchmark)?);
    }
    // An ordinary move answers `NONE`, which is a real answer here, not "unknown".
    if benchmarks.is_empty() {
        relevance = DefensiveRelevance::BOTH;
    }
    Ok(relevance)
}

/// Mirrors `hits_physical` in the pinned engine's damage path.
fn hits_physical(move_: &Move) -> bool {
    move_.category == Category::Physical || move_.deals_physical_damage
}

fn defender_relevance(move_: &Move, attacker: &Pokemon, defender: &Pokemon) -> DefensiveRelevance {
    if no_damage_formula(move_) || reads_both_defensive_stats(attacker, defender) {
        return DefensiveRelevance::BOTH;
    }
    if hits_physical(move_) {
        DefensiveRelevance::DEFENSE
    } else {
        DefensiveRelevance::SPECIAL_DEFENSE
    }
}

fn attacker_relevance(move_: &Move, attacker: &Pokemon, defender: &Pokemon) -> DefensiveRelevance {
    if no_damage_formula(move_) || reads_both_defensive_stats(attacker, defender) {
        return DefensiveRelevance::BOTH;
    }
    // Body Press is the only move whose attack stat is the user's Defense.
    if move_.name == "Body Press" {
        return DefensiveRelevance::DEFENSE;
    }
    DefensiveRelevance::NONE
}

/// The damage formula is not reached for these, so nothing about them is proven.
fn no_damage_formula(move_: &Move) -> bool {
    move_.category == Category::Status || move_.base_power == 0
}

/// Engine mechanics that can read a defensive stat the damage formula does not use.
///
/// The checks are deliberately loose (ability presence rather than activation state),
/// because a false positive only widens the search domain back to the exhaustive one.
fn reads_both_defensive_stats(attacker: &Pokemon, defender: &Pokemon) -> bool {
    if matches!(
        defender.ability,
        Ability::Protosynthesis | Ability::QuarkDrive
    ) {
        return true;
    }
    if matches!(
        attacker.ability,
        Ability::Protosynthesis | Ability::QuarkDrive
    ) {
        return true;
    }
    if attacker.ability == Ability::Download {
        return true;
    }
    attacker.ability == Ability::Trace || defender.ability == Ability::Trace
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage_bridge::DamageBenchmark;
    use crate::showdown::parse_set;

    fn benchmark(attacker: &str, defender: &str, move_name: &str) -> DamageBenchmark {
        DamageBenchmark::new(
            parse_set(attacker).unwrap(),
            parse_set(defender).unwrap(),
            move_name,
        )
    }

    #[test]
    fn ordinary_categories_select_the_engine_defensive_stat() {
        let data = ChampionsData::load().unwrap();
        let physical = benchmark("Pikachu", "Mega Floette", "Iron Head");
        assert_eq!(
            benchmark_defender_relevance(&data, &physical).unwrap(),
            DefensiveRelevance::DEFENSE
        );
        let special = benchmark("Pikachu", "Mega Floette", "Shadow Ball");
        assert_eq!(
            benchmark_defender_relevance(&data, &special).unwrap(),
            DefensiveRelevance::SPECIAL_DEFENSE
        );
        // Outgoing damage never reads the attacker's own defensive stats.
        assert_eq!(
            benchmark_attacker_relevance(&data, &physical).unwrap(),
            DefensiveRelevance::NONE
        );
        assert_eq!(
            benchmark_attacker_relevance(&data, &special).unwrap(),
            DefensiveRelevance::NONE
        );
        let press = benchmark("Corviknight", "Kingambit", "Body Press");
        assert_eq!(
            benchmark_attacker_relevance(&data, &press).unwrap(),
            DefensiveRelevance::DEFENSE
        );
    }

    #[test]
    fn defense_overrides_and_unknown_paths_keep_the_right_dimension() {
        let data = ChampionsData::load().unwrap();
        // Psyshock is Special in the pinned metadata but carries the engine's
        // `deals_physical_damage` flag, so it resolves against Defense.
        let psyshock = benchmark("Pikachu", "Mega Floette", "Psyshock");
        assert_eq!(
            benchmark_defender_relevance(&data, &psyshock).unwrap(),
            DefensiveRelevance::DEFENSE
        );
        let build_up = benchmark("Pikachu", "Mega Floette", "Swords Dance");
        assert_eq!(
            benchmark_defender_relevance(&data, &build_up).unwrap(),
            DefensiveRelevance::BOTH
        );
        assert_eq!(
            benchmark_attacker_relevance(&data, &build_up).unwrap(),
            DefensiveRelevance::BOTH
        );
    }

    #[test]
    fn stat_comparison_abilities_keep_both_defenses() {
        // The pinned Champions ability list does not currently expose Download or the
        // paradox abilities, so drive the engine-level predicate directly.
        let pokemon = |ability, item| {
            let mut pokemon = Pokemon::champions(
                "Stat Probe",
                [None, None],
                damage_calc::StatTable::new(100, 100, 100, 100, 100, 100),
                crate::stats::StatPoints::default().into(),
                damage_calc::Nature::Hardy,
            );
            pokemon.ability = ability;
            pokemon.item = item;
            pokemon
        };
        let tackle = damage_calc::Move::new(
            "Tackle",
            40,
            damage_calc::PokemonType::Normal,
            Category::Physical,
        );
        let plain = pokemon(Ability::None, damage_calc::Item::None);
        assert_eq!(
            defender_relevance(&tackle, &plain, &plain),
            DefensiveRelevance::DEFENSE
        );
        // Download reads the defender's Defense and Special Defense for a physical move.
        assert_eq!(
            defender_relevance(
                &tackle,
                &pokemon(Ability::Download, damage_calc::Item::None),
                &plain
            ),
            DefensiveRelevance::BOTH
        );
        // Trace can copy an ability that does, on either side of the calculation.
        assert_eq!(
            defender_relevance(
                &tackle,
                &pokemon(Ability::Trace, damage_calc::Item::None),
                &plain
            ),
            DefensiveRelevance::BOTH
        );
        assert_eq!(
            defender_relevance(
                &tackle,
                &plain,
                &pokemon(Ability::Trace, damage_calc::Item::None)
            ),
            DefensiveRelevance::BOTH
        );
        // An active paradox ability makes `highest_stat` compare both defenses.
        for ability in [Ability::Protosynthesis, Ability::QuarkDrive] {
            assert_eq!(
                defender_relevance(
                    &tackle,
                    &plain,
                    &pokemon(ability, damage_calc::Item::BoosterEnergy)
                ),
                DefensiveRelevance::BOTH
            );
            assert_eq!(
                attacker_relevance(
                    &tackle,
                    &pokemon(ability, damage_calc::Item::BoosterEnergy),
                    &plain
                ),
                DefensiveRelevance::BOTH
            );
        }
    }

    #[test]
    fn mixed_benchmarks_union_to_both_defenses() {
        let data = ChampionsData::load().unwrap();
        let benchmarks = [
            benchmark("Pikachu", "Mega Floette", "Iron Head"),
            benchmark("Pikachu", "Mega Floette", "Shadow Ball"),
        ];
        assert_eq!(
            combined_defender_relevance(&data, &benchmarks).unwrap(),
            DefensiveRelevance::BOTH
        );
        assert_eq!(
            combined_defender_relevance(&data, &[]).unwrap(),
            DefensiveRelevance::BOTH
        );
        // Physical and special hits agree that the attacker's defenses do not matter.
        assert_eq!(
            combined_attacker_relevance(&data, &benchmarks).unwrap(),
            DefensiveRelevance::NONE
        );
        assert_eq!(
            combined_attacker_relevance(
                &data,
                &[
                    benchmark("Pikachu", "Mega Floette", "Iron Head"),
                    benchmark("Corviknight", "Kingambit", "Body Press"),
                ]
            )
            .unwrap(),
            DefensiveRelevance::DEFENSE
        );
        assert_eq!(
            combined_attacker_relevance(&data, &[]).unwrap(),
            DefensiveRelevance::BOTH
        );
    }
}
