use crate::data::items::{can_fling, fling_power, natural_gift};
use crate::mechanics::modifiers::{poke_round_ratio, MODIFIER_DENOMINATOR, MOD_DOUBLE};
use crate::types::{CalcError, Field, Item, Move, Pokemon, PokemonType, StatusCondition, Weather};

use super::{count_positive_boosts, effective_weight, is_grounded, ModifierBreakdown};

#[allow(clippy::too_many_arguments)]
pub(super) fn calc_base_power(
    move_: &Move,
    attacker: &Pokemon,
    defender: &Pokemon,
    attacker_current_hp: u16,
    attacker_max_hp: u16,
    defender_current_hp: u16,
    defender_max_hp: u16,
    field: &Field,
    attacker_speed: u16,
    defender_speed: u16,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> Result<u16, CalcError> {
    let bp = match move_.name.as_str() {
        "Gyro Ball" => {
            let speed = attacker_speed.max(1);
            let bp = (25 * defender_speed / speed).min(150);
            modifiers.push(ModifierBreakdown::new("Gyro Ball base power", 0));
            bp
        }
        "Electro Ball" => {
            let ratio = attacker_speed.checked_div(defender_speed).unwrap_or(0);
            let bp = if ratio >= 4 {
                150
            } else if ratio >= 3 {
                120
            } else if ratio >= 2 {
                80
            } else if ratio >= 1 {
                60
            } else {
                40
            };
            modifiers.push(ModifierBreakdown::new("Electro Ball base power", 0));
            bp
        }
        "Low Kick" | "Grass Knot" => {
            let weight = effective_weight(defender);
            let bp = if weight >= 200.0 {
                120
            } else if weight >= 100.0 {
                100
            } else if weight >= 50.0 {
                80
            } else if weight >= 25.0 {
                60
            } else if weight >= 10.0 {
                40
            } else {
                20
            };
            modifiers.push(ModifierBreakdown::new("weight-based base power", 0));
            bp
        }
        "Heavy Slam" | "Heat Crash" => {
            let defender_weight = effective_weight(defender).max(0.1);
            let ratio = effective_weight(attacker) / defender_weight;
            let bp = if ratio >= 5.0 {
                120
            } else if ratio >= 4.0 {
                100
            } else if ratio >= 3.0 {
                80
            } else if ratio >= 2.0 {
                60
            } else {
                40
            };
            modifiers.push(ModifierBreakdown::new("weight-ratio base power", 0));
            bp
        }
        "Eruption" | "Water Spout" | "Dragon Energy" => {
            let bp = (150 * attacker_current_hp / attacker_max_hp).max(1);
            modifiers.push(ModifierBreakdown::new("HP-based base power", 0));
            bp
        }
        "Flail" | "Reversal" => {
            let p = 48 * attacker_current_hp / attacker_max_hp;
            let bp = if p <= 1 {
                200
            } else if p <= 4 {
                150
            } else if p <= 9 {
                100
            } else if p <= 16 {
                80
            } else if p <= 32 {
                40
            } else {
                20
            };
            modifiers.push(ModifierBreakdown::new("HP-based base power", 0));
            bp
        }
        "Crush Grip" | "Wring Out" => {
            let hp_ratio =
                (defender_current_hp as i64 * MODIFIER_DENOMINATOR as i64) / defender_max_hp as i64;
            let rounded = poke_round_ratio(120 * 100 * hp_ratio, MODIFIER_DENOMINATOR as i64);
            let bp = (rounded / 100).max(1) as u16;
            modifiers.push(ModifierBreakdown::new("HP-ratio base power", 0));
            bp
        }
        "Hard Press" => {
            let hp_ratio =
                (defender_current_hp as i64 * MODIFIER_DENOMINATOR as i64) / defender_max_hp as i64;
            let rounded = poke_round_ratio(100 * 100 * hp_ratio, MODIFIER_DENOMINATOR as i64);
            let bp = (rounded / 100).max(1) as u16;
            modifiers.push(ModifierBreakdown::new("HP-ratio base power", 0));
            bp
        }
        "Stored Power" | "Power Trip" => {
            let bp = 20 + 20 * count_positive_boosts(attacker.boosts);
            modifiers.push(ModifierBreakdown::new("boost-count base power", 0));
            bp
        }
        "Punishment" => {
            let bp = (60 + 20 * count_positive_boosts(defender.boosts)).min(200);
            modifiers.push(ModifierBreakdown::new("target boost-count base power", 0));
            bp
        }
        "Acrobatics" => {
            let bp = if matches!(attacker.item, Item::None | Item::FlyingGem) {
                110
            } else {
                move_.base_power
            };
            if bp != move_.base_power {
                modifiers.push(ModifierBreakdown::new("Acrobatics base power", MOD_DOUBLE));
            }
            bp
        }
        "Hex" | "Infernal Parade" => {
            let bp = if defender.status != StatusCondition::Healthy {
                move_.base_power * 2
            } else {
                move_.base_power
            };
            if bp != move_.base_power {
                modifiers.push(ModifierBreakdown::new("status base power", MOD_DOUBLE));
            }
            bp
        }
        "Smelling Salts" => {
            let bp = if defender.status == StatusCondition::Paralyzed {
                move_.base_power * 2
            } else {
                move_.base_power
            };
            if bp != move_.base_power {
                modifiers.push(ModifierBreakdown::new(
                    "Smelling Salts base power",
                    MOD_DOUBLE,
                ));
            }
            bp
        }
        "Wake-Up Slap" => {
            let bp = if defender.status == StatusCondition::Asleep {
                move_.base_power * 2
            } else {
                move_.base_power
            };
            if bp != move_.base_power {
                modifiers.push(ModifierBreakdown::new(
                    "Wake-Up Slap base power",
                    MOD_DOUBLE,
                ));
            }
            bp
        }
        "Weather Ball"
            if field.weather != Weather::None && field.weather != Weather::StrongWinds =>
        {
            modifiers.push(ModifierBreakdown::new(
                "Weather Ball base power",
                MOD_DOUBLE,
            ));
            move_.base_power * 2
        }
        "Terrain Pulse" if field.terrain != crate::types::Terrain::None => {
            modifiers.push(ModifierBreakdown::new(
                "Terrain Pulse base power",
                MOD_DOUBLE,
            ));
            move_.base_power * 2
        }
        "Rising Voltage"
            if field.terrain == crate::types::Terrain::Electric && is_grounded(defender, field) =>
        {
            modifiers.push(ModifierBreakdown::new(
                "Rising Voltage base power",
                MOD_DOUBLE,
            ));
            move_.base_power * 2
        }
        "Tera Blast" if move_.type_ == PokemonType::Stellar => {
            modifiers.push(ModifierBreakdown::new("Stellar Tera Blast base power", 0));
            100
        }
        "Fling" => {
            if !can_fling(attacker.item, &attacker.name, defender.ability) {
                return Ok(0);
            }
            let bp = fling_power(attacker.item).unwrap_or(10);
            modifiers.push(ModifierBreakdown::new("Fling base power", 0));
            bp
        }
        "Natural Gift" => {
            if let Some((_type, power)) = natural_gift(attacker.item) {
                modifiers.push(ModifierBreakdown::new("Natural Gift base power", 0));
                power
            } else {
                0
            }
        }
        "Triple Kick" | "Triple Axel" => {
            move_.base_power * move_.current_triple_hit.unwrap_or(1) as u16
        }
        "Last Respects" | "Rage Fist" => {
            let effect = move_.effect_count();
            let bp = move_.base_power * effect.base_power_multiplier();
            if effect.is_active() {
                modifiers.push(ModifierBreakdown::new(effect.label(), 0));
            }
            bp
        }
        _ if move_.is_double_power
            && matches!(
                move_.name.as_str(),
                "Assurance"
                    | "Avalanche"
                    | "Bolt Beak"
                    | "Fickle Beam"
                    | "Fishious Rend"
                    | "Payback"
                    | "Round"
                    | "Stomping Tantrum"
                    | "Temper Flare"
            ) =>
        {
            modifiers.push(ModifierBreakdown::new("double-power move flag", MOD_DOUBLE));
            move_.base_power * 2
        }
        _ => move_.base_power,
    };
    Ok(bp)
}
