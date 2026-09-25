use damage_calc::{calculate_stats, Nature, Pokemon, PokemonType, Ruleset, StatTable};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAX_STAT_POINTS: u16 = 32;
pub const MAX_TOTAL_STAT_POINTS: u16 = 66;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StatError {
    #[error("stat points for {stat} exceed {MAX_STAT_POINTS}: {value}")]
    StatOverCap { stat: &'static str, value: u16 },
    #[error("damage calculator rejected stats: {0}")]
    DamageCalc(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub special_attack: u16,
    pub special_defense: u16,
    pub speed: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StatPoints {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub special_attack: u16,
    pub special_defense: u16,
    pub speed: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub special_attack: u16,
    pub special_defense: u16,
    pub speed: u16,
}

impl BaseStats {
    pub const fn new(
        hp: u16,
        attack: u16,
        defense: u16,
        special_attack: u16,
        special_defense: u16,
        speed: u16,
    ) -> Self {
        Self {
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        }
    }
}

impl StatPoints {
    pub const fn new(
        hp: u16,
        attack: u16,
        defense: u16,
        special_attack: u16,
        special_defense: u16,
        speed: u16,
    ) -> Self {
        Self {
            hp,
            attack,
            defense,
            special_attack,
            special_defense,
            speed,
        }
    }

    pub const fn total(self) -> u16 {
        self.hp
            + self.attack
            + self.defense
            + self.special_attack
            + self.special_defense
            + self.speed
    }

    pub fn validate(self) -> Result<(), StatError> {
        for (stat, value) in [
            ("HP", self.hp),
            ("Attack", self.attack),
            ("Defense", self.defense),
            ("Special Attack", self.special_attack),
            ("Special Defense", self.special_defense),
            ("Speed", self.speed),
        ] {
            if value > MAX_STAT_POINTS {
                return Err(StatError::StatOverCap { stat, value });
            }
        }

        Ok(())
    }
}

pub fn champions_final_stats(
    base_stats: BaseStats,
    nature: Nature,
    sps: StatPoints,
) -> Result<FinalStats, StatError> {
    sps.validate()?;

    let pokemon = Pokemon::champions(
        "Stat Probe",
        [Some(PokemonType::Normal), None],
        base_stats.into(),
        sps.into(),
        nature,
    );

    calculate_stats(&pokemon, Ruleset::Champions)
        .map(FinalStats::from)
        .map_err(|error| StatError::DamageCalc(error.to_string()))
}

impl From<BaseStats> for StatTable {
    fn from(value: BaseStats) -> Self {
        Self::new(
            value.hp,
            value.attack,
            value.defense,
            value.special_attack,
            value.special_defense,
            value.speed,
        )
    }
}

impl From<StatPoints> for StatTable {
    fn from(value: StatPoints) -> Self {
        Self::new(
            value.hp,
            value.attack,
            value.defense,
            value.special_attack,
            value.special_defense,
            value.speed,
        )
    }
}

impl From<StatTable> for BaseStats {
    fn from(value: StatTable) -> Self {
        Self::new(
            value.hp,
            value.attack,
            value.defense,
            value.special_attack,
            value.special_defense,
            value.speed,
        )
    }
}

impl From<StatTable> for StatPoints {
    fn from(value: StatTable) -> Self {
        Self::new(
            value.hp,
            value.attack,
            value.defense,
            value.special_attack,
            value.special_defense,
            value.speed,
        )
    }
}

impl From<StatTable> for FinalStats {
    fn from(value: StatTable) -> Self {
        Self {
            hp: value.hp,
            attack: value.attack,
            defense: value.defense,
            special_attack: value.special_attack,
            special_defense: value.special_defense,
            speed: value.speed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_spread_caps() {
        assert_eq!(
            StatPoints::new(33, 0, 0, 0, 0, 0).validate(),
            Err(StatError::StatOverCap {
                stat: "HP",
                value: 33
            })
        );
        assert_eq!(StatPoints::new(32, 32, 32, 32, 32, 32).validate(), Ok(()));
    }

    #[test]
    fn delegates_champions_stats() {
        let stats = champions_final_stats(
            BaseStats::new(100, 100, 100, 100, 100, 100),
            Nature::Adamant,
            StatPoints::new(1, 2, 3, 4, 5, 6),
        )
        .unwrap();

        assert_eq!(stats.hp, 176);
        assert_eq!(stats.attack, 134);
        assert_eq!(stats.defense, 123);
        assert_eq!(stats.special_attack, 111);
        assert_eq!(stats.special_defense, 125);
        assert_eq!(stats.speed, 126);
    }
}
