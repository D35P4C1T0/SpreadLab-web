use crate::types::{CalcError, Nature, Pokemon, Ruleset, Stat, StatTable};

/// Calculates a Champions HP stat.
///
/// JS source: `CALC_HP_CHAMP`
/// `Math.floor((base * 2 + 31) * 50 / 100) + 50 + 10 + statPoints`.
pub fn calculate_hp(base: u16, stat_points: u16, ruleset: Ruleset) -> Result<u16, CalcError> {
    if stat_points > 32 {
        return Err(CalcError::InvalidStatPoints { value: stat_points });
    }
    match ruleset {
        Ruleset::Champions => {
            if base == 1 {
                Ok(1)
            } else {
                Ok(((base * 2 + 31) * 50 / 100) + 50 + 10 + stat_points)
            }
        }
    }
}

/// Calculates a non-HP Champions stat.
///
/// JS source: `CALC_STAT_CHAMP`
/// `Math.floor(((Math.floor((base * 2 + 31) * 50 / 100) + 5) + statPoints) * nature)`.
pub fn calculate_non_hp_stat(
    stat: Stat,
    base: u16,
    stat_points: u16,
    nature: Nature,
    ruleset: Ruleset,
) -> Result<u16, CalcError> {
    if stat_points > 32 {
        return Err(CalcError::InvalidStatPoints { value: stat_points });
    }
    match ruleset {
        Ruleset::Champions => {
            let before_nature = ((base * 2 + 31) * 50 / 100) + 5 + stat_points;
            let value = if nature.increased_stat() == Some(stat) {
                before_nature as u32 * 11 / 10
            } else if nature.decreased_stat() == Some(stat) {
                before_nature as u32 * 9 / 10
            } else {
                before_nature as u32
            };
            Ok(value as u16)
        }
    }
}

pub fn calculate_stats(pokemon: &Pokemon, ruleset: Ruleset) -> Result<StatTable, CalcError> {
    Ok(StatTable::new(
        calculate_hp(pokemon.base_stats.hp, pokemon.stat_points.hp, ruleset)?,
        calculate_non_hp_stat(
            Stat::Attack,
            pokemon.base_stats.attack,
            pokemon.stat_points.attack,
            pokemon.nature,
            ruleset,
        )?,
        calculate_non_hp_stat(
            Stat::Defense,
            pokemon.base_stats.defense,
            pokemon.stat_points.defense,
            pokemon.nature,
            ruleset,
        )?,
        calculate_non_hp_stat(
            Stat::SpecialAttack,
            pokemon.base_stats.special_attack,
            pokemon.stat_points.special_attack,
            pokemon.nature,
            ruleset,
        )?,
        calculate_non_hp_stat(
            Stat::SpecialDefense,
            pokemon.base_stats.special_defense,
            pokemon.stat_points.special_defense,
            pokemon.nature,
            ruleset,
        )?,
        calculate_non_hp_stat(
            Stat::Speed,
            pokemon.base_stats.speed,
            pokemon.stat_points.speed,
            pokemon.nature,
            ruleset,
        )?,
    ))
}
