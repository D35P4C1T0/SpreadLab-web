//! Cumulative damage distributions matching the pinned JavaScript KO projection.
//! Healing markers are assigned before hazards on the first move; subsequent moves
//! use hazard-adjusted HP. Keep that distinction and upstream's residual ordering.

use std::collections::BTreeMap;

use crate::types::{Ability, Item};

use super::ResidualEffects;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Roll {
    damage: i32,
    healed: bool,
}

type Distribution = BTreeMap<Roll, f64>;

#[derive(Clone, Copy, Default)]
struct Recovery {
    hp: i32,
    threshold: i32,
}

impl Recovery {
    fn activates(self, hp: i32, damage: i32) -> bool {
        self.hp > 0 && hp - damage > 0 && hp - damage <= self.threshold
    }
}

pub(super) fn ko_chances_after_move_uses(
    hit_rolls: &[Vec<u16>],
    current_hp: u16,
    max_hp: u16,
    item: Item,
    residual: ResidualEffects,
    healing_suppressed: bool,
    max_uses: usize,
) -> Vec<f32> {
    if hit_rolls.is_empty() || hit_rolls.iter().any(Vec::is_empty) {
        return vec![0.0; max_uses];
    }
    let recovery = if healing_suppressed {
        Recovery::default()
    } else {
        item_recovery(item, max_hp, residual.defender_ability)
    };
    let current_hp = i32::from(current_hp);
    let target_hp = current_hp - i32::from(residual.initial_damage);
    let max_hp = i32::from(max_hp);
    let mut damage = Distribution::from([(
        Roll {
            damage: 0,
            healed: false,
        },
        1.0,
    )]);
    for rolls in hit_rolls {
        let mut next = Distribution::new();
        for (previous, probability) in damage {
            for &hit in rolls {
                let total = previous.damage + i32::from(hit);
                let roll = Roll {
                    damage: total,
                    healed: previous.healed || recovery.activates(current_hp, total),
                };
                *next.entry(roll).or_default() += probability / rolls.len() as f64;
            }
        }
        damage = next;
    }
    let toxic_counter = if residual.toxic {
        i32::from(residual.initial_toxic_counter)
    } else {
        0
    };
    let eot_sum = residual.effects.iter().sum::<i32>() - toxic_counter * max_hp / 16;
    let mut first = Distribution::new();
    for (&roll, &probability) in &damage {
        let roll = if eot_sum != 0 {
            end_of_turn(roll, target_hp, max_hp, recovery, residual, toxic_counter)
        } else {
            roll
        };
        *first.entry(roll).or_default() += probability;
    }
    let mut chances = Vec::with_capacity(max_uses);
    if max_uses == 0 {
        return chances;
    }
    // Preserve getKOChance's first-use shortcuts, which use the net residual sum.
    let (&min, _) = damage.first_key_value().expect("nonempty damage");
    let (&max, _) = damage.last_key_value().expect("nonempty damage");
    let multihit = hit_rolls.len() > 1;
    let min_target = target_hp
        + if (multihit || min.healed) && recovery.hp > 0 {
            recovery.hp
        } else {
            0
        };
    let max_target = target_hp
        + if (multihit || max.healed) && recovery.hp > 0 {
            recovery.hp
        } else {
            0
        };
    chances.push(if max.damage - eot_sum < max_target {
        0.0
    } else if min.damage - eot_sum >= min_target {
        1.0
    } else {
        chance(&first, target_hp, recovery)
    });

    let mut accumulated = first;
    for _ in 1..max_uses {
        let mut next = Distribution::new();
        for (previous, previous_probability) in accumulated {
            for (pivot, probability) in &damage {
                let total = previous.damage + pivot.damage;
                let mut roll = Roll {
                    damage: total,
                    healed: previous.healed || recovery.activates(target_hp, total),
                };
                if eot_sum != 0 {
                    // verifyKOChance keeps this counter fixed for moves 2 through 4.
                    roll = end_of_turn(
                        roll,
                        target_hp,
                        max_hp,
                        recovery,
                        residual,
                        toxic_counter + 1,
                    );
                }
                *next.entry(roll).or_default() += previous_probability * probability;
            }
        }
        chances.push(chance(&next, target_hp, recovery));
        accumulated = next;
    }
    chances
}

fn chance(distribution: &Distribution, target_hp: i32, recovery: Recovery) -> f32 {
    let mut probability = 0.0;
    let mut all_ko = true;
    for (roll, weight) in distribution {
        if roll.damage - if roll.healed { recovery.hp } else { 0 } >= target_hp {
            probability += weight;
        } else {
            all_ko = false;
        }
    }
    if all_ko {
        1.0
    } else {
        probability.min(1.0) as f32
    }
}

fn end_of_turn(
    mut roll: Roll,
    target_hp: i32,
    max_hp: i32,
    recovery: Recovery,
    residual: ResidualEffects,
    toxic_counter: i32,
) -> Roll {
    for (index, &value) in residual.effects.iter().enumerate() {
        let effect = if index == 7 && residual.toxic {
            -toxic_counter * max_hp / 16
        } else {
            value
        };
        if effect == 0 {
            continue;
        }
        roll.healed |= recovery.activates(target_hp, roll.damage - effect);
        if target_hp - roll.damage + if roll.healed { recovery.hp } else { 0 } > 0 {
            roll.damage -= effect;
        }
    }
    roll
}

fn item_recovery(item: Item, max_hp: u16, ability: Ability) -> Recovery {
    let max_hp = i32::from(max_hp);
    let (mut hp, threshold) = match item {
        Item::OranBerry => (10, max_hp / 2),
        Item::SitrusBerry => (max_hp / 4, max_hp / 2),
        Item::FigyBerry
        | Item::IapapaBerry
        | Item::WikiBerry
        | Item::AguavBerry
        | Item::MagoBerry => (
            max_hp / 3,
            if ability == Ability::Gluttony {
                max_hp / 2
            } else {
                max_hp / 4
            },
        ),
        _ => return Recovery::default(),
    };
    if ability == Ability::Ripen {
        hp *= 2;
    }
    Recovery { hp, threshold }
}
