use crate::data::items::{gem_type, is_gem, item_boost_type};
use crate::mechanics::modifiers::{
    apply_mod, chain_mods, MOD_1_1, MOD_1_1_ALT, MOD_1_2, MOD_1_3, MOD_1_33, MOD_1_5, MOD_DOUBLE,
    MOD_HALF,
};
use crate::types::{
    Ability, Category, Field, Item, Move, Pokemon, PokemonType, RivalryTarget, StatusCondition,
    Weather,
};

use super::{cant_remove_item, is_grounded, makes_effective_contact, push_mod, ModifierBreakdown};

#[allow(clippy::too_many_arguments)]
pub(super) fn calc_bp_mods(
    move_: &Move,
    attacker: &Pokemon,
    defender: &Pokemon,
    field: &Field,
    base_power: u16,
    ate_ize_boosted: bool,
    attacker_moves_first: bool,
    defender_current_hp: u16,
    defender_max_hp: u16,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> Vec<i32> {
    let mut mods = Vec::new();

    if ate_ize_boosted {
        push_mod(&mut mods, modifiers, "type-changing ability boost", MOD_1_2);
    }

    if !ate_ize_boosted
        && ((attacker.ability == Ability::Reckless && (move_.has_recoil || move_.has_crash))
            || (attacker.ability == Ability::IronFist && move_.is_punch))
    {
        push_mod(&mut mods, modifiers, "base power ability 1.2", MOD_1_2);
    }

    if field.battery && move_.category == Category::Special {
        push_mod(&mut mods, modifiers, "Battery", MOD_1_3);
    }
    if field.power_spot {
        push_mod(&mut mods, modifiers, "Power Spot", MOD_1_3);
    }
    if field.steely_spirit && move_.type_ == PokemonType::Steel {
        push_mod(&mut mods, modifiers, "Ally Steely Spirit", MOD_1_5);
    }
    if attacker.ability == Ability::FairyAura && move_.type_ == PokemonType::Fairy {
        push_mod(&mut mods, modifiers, "Fairy Aura", MOD_1_33);
    }
    if attacker.ability == Ability::Rivalry {
        match attacker.rivalry_target {
            RivalryTarget::SameGender => {
                push_mod(&mut mods, modifiers, "Rivalry same gender", 0x1400)
            }
            RivalryTarget::OppositeGender => {
                push_mod(&mut mods, modifiers, "Rivalry opposite gender", 0x0c00)
            }
            RivalryTarget::Unspecified => {}
        }
    }

    if (attacker.ability == Ability::SheerForce && move_.has_secondary_effect)
        || (attacker.ability == Ability::SandForce
            && field.weather == Weather::Sand
            && matches!(
                move_.type_,
                PokemonType::Rock | PokemonType::Ground | PokemonType::Steel
            ))
        || (attacker.ability == Ability::Analytic && !attacker_moves_first)
        || (attacker.ability == Ability::ToughClaws && makes_effective_contact(attacker, move_))
        || (attacker.ability == Ability::PunkRock && move_.is_sound)
    {
        push_mod(&mut mods, modifiers, "base power ability 1.3", MOD_1_3);
    }

    let temp_bp = apply_mod(base_power as i32, chain_mods(&mods));
    if (attacker.ability == Ability::Technician && temp_bp <= 60)
        || (attacker.ability == Ability::MegaLauncher && move_.is_pulse)
        || (attacker.ability == Ability::StrongJaw && move_.is_bite)
        || (attacker.ability == Ability::SteelySpirit && move_.type_ == PokemonType::Steel)
    {
        push_mod(&mut mods, modifiers, "base power ability 1.5", MOD_1_5);
    }

    if (attacker.item == Item::MuscleBand && move_.category == Category::Physical)
        || (attacker.item == Item::WiseGlasses && move_.category == Category::Special)
    {
        push_mod(&mut mods, modifiers, "1.1x item", MOD_1_1);
    } else if item_boost_type(attacker.item) == Some(move_.type_) {
        push_mod(&mut mods, modifiers, "type item", MOD_1_2);
    } else if gem_type(attacker.item) == Some(move_.type_) && is_gem(attacker.item) {
        push_mod(&mut mods, modifiers, "gem", MOD_1_3);
    }

    if field.helping_hand {
        push_mod(&mut mods, modifiers, "Helping Hand", MOD_1_5);
    }
    if (field.charge
        || (matches!(
            attacker.ability,
            Ability::Electromorphosis | Ability::WindPower
        ) && attacker.ability_on))
        && move_.type_ == PokemonType::Electric
    {
        push_mod(&mut mods, modifiers, "Charge", MOD_DOUBLE);
    }
    if matches!(move_.name.as_str(), "Solar Beam" | "Solar Blade")
        && !matches!(
            field.weather,
            Weather::None | Weather::Sun | Weather::HarshSun | Weather::StrongWinds
        )
        && attacker.item != Item::UtilityUmbrella
        && attacker.ability != Ability::MegaSol
    {
        push_mod(&mut mods, modifiers, "bad-weather Solar move", MOD_HALF);
    }

    if is_grounded(attacker, field) {
        let terrain_modifier = MOD_1_3;
        if field.terrain == crate::types::Terrain::Electric && move_.type_ == PokemonType::Electric
        {
            push_mod(&mut mods, modifiers, "Electric Terrain", terrain_modifier);
        } else if field.terrain == crate::types::Terrain::Grassy
            && move_.type_ == PokemonType::Grass
        {
            push_mod(&mut mods, modifiers, "Grassy Terrain", terrain_modifier);
        } else if field.terrain == crate::types::Terrain::Psychic
            && move_.type_ == PokemonType::Psychic
        {
            push_mod(&mut mods, modifiers, "Psychic Terrain", terrain_modifier);
        }
    }
    if is_grounded(defender, field)
        && ((field.terrain == crate::types::Terrain::Misty && move_.type_ == PokemonType::Dragon)
            || (field.terrain == crate::types::Terrain::Grassy
                && matches!(move_.name.as_str(), "Earthquake" | "Bulldoze")))
    {
        push_mod(&mut mods, modifiers, "defensive terrain", MOD_HALF);
    }

    if matches!(
        attacker.status,
        StatusCondition::Burned
            | StatusCondition::Paralyzed
            | StatusCondition::Poisoned
            | StatusCondition::BadlyPoisoned
    ) && move_.name == "Facade"
    {
        push_mod(&mut mods, modifiers, "Facade", MOD_DOUBLE);
    }
    if defender_current_hp <= defender_max_hp / 2 && move_.name == "Brine" {
        push_mod(&mut mods, modifiers, "Brine", MOD_DOUBLE);
    }
    if (matches!(move_.name.as_str(), "Venoshock" | "Barb Barrage")
        && matches!(
            defender.status,
            StatusCondition::Poisoned | StatusCondition::BadlyPoisoned
        ))
        || (matches!(
            move_.name.as_str(),
            "Retaliate" | "Fusion Bolt" | "Fusion Flare" | "Lash Out"
        ) && move_.is_double_power)
    {
        push_mod(&mut mods, modifiers, "conditional double power", MOD_DOUBLE);
    }
    if move_.name == "Knock Off" && !cant_remove_item(defender.item, &defender.name) {
        push_mod(&mut mods, modifiers, "Knock Off", MOD_1_5);
    } else if field.terrain == crate::types::Terrain::Electric && move_.name == "Psyblade" {
        push_mod(&mut mods, modifiers, "Psyblade", MOD_1_5);
    } else if (move_.name == "Misty Explosion"
        && field.terrain == crate::types::Terrain::Misty
        && is_grounded(attacker, field))
        || (move_.name == "Grav Apple" && field.gravity)
        || (move_.name == "Expanding Force"
            && field.terrain == crate::types::Terrain::Psychic
            && is_grounded(attacker, field))
    {
        push_mod(&mut mods, modifiers, "field base power boost", MOD_1_5);
    }

    if attacker.ability == Ability::SupremeOverlord {
        let table = [MOD_1_1_ALT, MOD_1_2, MOD_1_3, 0x1666, MOD_1_5];
        if let Some(idx) = attacker.supreme_overlord_effect().supreme_overlord_index() {
            push_mod(&mut mods, modifiers, "Supreme Overlord", table[idx]);
        }
    }
    if attacker.item == Item::PunchingGlove && move_.is_punch {
        push_mod(&mut mods, modifiers, "Punching Glove", MOD_1_1_ALT);
    }
    for &modifier in &attacker.custom_bp_mods {
        push_mod(&mut mods, modifiers, "custom BP modifier", modifier);
    }

    mods
}
