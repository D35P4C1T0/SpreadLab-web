use crate::stats::{StatPoints, MAX_STAT_POINTS, MAX_TOTAL_STAT_POINTS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpreadSearch {
    pub exact_total: Option<u16>,
    pub max_total: u16,
    pub locked: LockedStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LockedStats {
    pub hp: Option<u16>,
    pub attack: Option<u16>,
    pub defense: Option<u16>,
    pub special_attack: Option<u16>,
    pub special_defense: Option<u16>,
    pub speed: Option<u16>,
}

impl SpreadSearch {
    pub fn all_legal() -> Self {
        Self {
            exact_total: None,
            max_total: MAX_TOTAL_STAT_POINTS,
            locked: LockedStats::default(),
        }
    }

    pub fn full_spend() -> Self {
        Self {
            exact_total: Some(MAX_TOTAL_STAT_POINTS),
            max_total: MAX_TOTAL_STAT_POINTS,
            locked: LockedStats::default(),
        }
    }
}

pub fn generate_spreads(search: SpreadSearch) -> Vec<StatPoints> {
    let max_total = search.max_total.min(MAX_TOTAL_STAT_POINTS);
    let hp_values = values(search.locked.hp);
    let atk_values = values(search.locked.attack);
    let def_values = values(search.locked.defense);
    let spa_values = values(search.locked.special_attack);
    let spd_values = values(search.locked.special_defense);
    let spe_values = values(search.locked.speed);
    let mut out = Vec::new();

    for hp in hp_values {
        for attack in atk_values.clone() {
            for defense in def_values.clone() {
                for special_attack in spa_values.clone() {
                    for special_defense in spd_values.clone() {
                        let subtotal = hp + attack + defense + special_attack + special_defense;
                        if subtotal > max_total {
                            continue;
                        }
                        for speed in spe_values.clone() {
                            let total = subtotal + speed;
                            if total > max_total {
                                continue;
                            }
                            if search.exact_total.is_some_and(|exact| total != exact) {
                                continue;
                            }
                            out.push(StatPoints::new(
                                hp,
                                attack,
                                defense,
                                special_attack,
                                special_defense,
                                speed,
                            ));
                        }
                    }
                }
            }
        }
    }

    out
}

fn values(locked: Option<u16>) -> Vec<u16> {
    match locked {
        Some(value) => vec![value.min(MAX_STAT_POINTS)],
        None => (0..=MAX_STAT_POINTS).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_locked_full_spend() {
        let search = SpreadSearch {
            exact_total: Some(MAX_TOTAL_STAT_POINTS),
            max_total: MAX_TOTAL_STAT_POINTS,
            locked: LockedStats {
                attack: Some(32),
                speed: Some(32),
                ..LockedStats::default()
            },
        };
        let spreads = generate_spreads(search);
        assert!(spreads.contains(&StatPoints::new(0, 32, 2, 0, 0, 32)));
        assert!(spreads
            .iter()
            .all(|spread| spread.total() == MAX_TOTAL_STAT_POINTS));
    }
}
