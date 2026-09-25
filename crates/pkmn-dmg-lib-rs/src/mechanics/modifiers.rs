use crate::types::{CalcError, Stat};

pub const MODIFIER_DENOMINATOR: i32 = 0x1000;
pub const MOD_HALF: i32 = 0x0800;
pub const MOD_THREE_QUARTERS: i32 = 0x0C00;
pub const MOD_DOUBLE: i32 = 0x2000;
pub const MOD_0_67_DOUBLES_SCREEN: i32 = 0x0AAC;
pub const MOD_1_1: i32 = 0x1199;
pub const MOD_1_1_ALT: i32 = 0x119A;
pub const MOD_1_2: i32 = 0x1333;
pub const MOD_1_25: i32 = 0x1400;
pub const MOD_1_3: i32 = 0x14CD;
pub const MOD_LIFE_ORB: i32 = 0x14CC;
pub const MOD_1_33: i32 = 0x1555;
pub const MOD_1_5: i32 = 0x1800;

/// Game Freak's modern rounding rule, matching JS `pokeRound`.
///
/// The original implementation rounds down on exact `.5`:
/// `return (num % 1 > 0.5) ? Math.ceil(num) : Math.floor(num);`
pub fn poke_round_ratio(numerator: i64, denominator: i64) -> i32 {
    let quotient = numerator.div_euclid(denominator);
    let remainder = numerator.rem_euclid(denominator);
    if remainder * 2 > denominator {
        (quotient + 1) as i32
    } else {
        quotient as i32
    }
}

pub fn chain_mods(mods: &[i32]) -> i32 {
    let mut chained = MODIFIER_DENOMINATOR;
    for &modifier in mods {
        if modifier != MODIFIER_DENOMINATOR {
            chained = poke_round_ratio(
                chained as i64 * modifier as i64,
                MODIFIER_DENOMINATOR as i64,
            );
        }
    }
    chained
}

pub fn apply_mod(value: i32, modifier: i32) -> i32 {
    poke_round_ratio(value as i64 * modifier as i64, MODIFIER_DENOMINATOR as i64)
}

pub fn modified_stat(stat: u16, stage: i8) -> Result<u16, CalcError> {
    if !(-6..=6).contains(&stage) {
        return Err(CalcError::InvalidBoost { value: stage });
    }
    let stat = stat as i32;
    let value = if stage > 0 {
        stat * (2 + stage as i32) / 2
    } else if stage < 0 {
        stat * 2 / (2 - stage as i32)
    } else {
        stat
    };
    Ok(value as u16)
}

pub fn stat_key(stat: Stat) -> &'static str {
    match stat {
        Stat::Hp => "hp",
        Stat::Attack => "at",
        Stat::Defense => "df",
        Stat::SpecialAttack => "sa",
        Stat::SpecialDefense => "sd",
        Stat::Speed => "sp",
    }
}
