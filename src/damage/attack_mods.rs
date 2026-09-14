use crate::mechanics::modifiers::{
    MOD_1_3, MOD_1_33, MOD_1_5, MOD_DOUBLE, MOD_HALF, MOD_THREE_QUARTERS,
};
use crate::types::{
    Ability, Category, Field, Item, Move, Pokemon, PokemonType, Stat, StatusCondition, Weather,
};

use super::{is_half_hp, is_third_hp, push_mod, ModifierBreakdown};

pub(super) fn calc_attack_mods(
    move_: &Move,
    attacker: &Pokemon,
    attacker_highest_stat: Stat,
    def_ability: Ability,
    field: &Field,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> Vec<i32> {
    let mut mods = Vec::new();
    if field.tablets_of_ruin
        && move_.category == Category::Physical
        && attacker.ability != Ability::TabletsOfRuin
    {
        push_mod(&mut mods, modifiers, "Tablets of Ruin", MOD_THREE_QUARTERS);
    } else if field.vessel_of_ruin
        && move_.category == Category::Special
        && attacker.ability != Ability::VesselOfRuin
    {
        push_mod(&mut mods, modifiers, "Vessel of Ruin", MOD_THREE_QUARTERS);
    }
    if attacker.ability == Ability::Defeatist && is_half_hp(attacker) {
        push_mod(&mut mods, modifiers, "attack ability 0.5", MOD_HALF);
    }
    if ((attacker.ability == Ability::FlowerGift && field.weather.is_sun())
        || field.flower_gift_attack)
        && move_.category == Category::Physical
        && attacker.item != Item::UtilityUmbrella
    {
        push_mod(&mut mods, modifiers, "Flower Gift attack", MOD_1_5);
    }

    if (attacker.ability == Ability::Guts
        && attacker.status != StatusCondition::Healthy
        && move_.category == Category::Physical)
        || (attacker.ability == Ability::FlareBoost
            && attacker.status == StatusCondition::Burned
            && move_.category == Category::Special)
        || (attacker.ability == Ability::Overgrow
            && is_third_hp(attacker)
            && move_.type_ == PokemonType::Grass)
        || (attacker.ability == Ability::Blaze
            && is_third_hp(attacker)
            && move_.type_ == PokemonType::Fire)
        || (attacker.ability == Ability::Torrent
            && is_third_hp(attacker)
            && move_.type_ == PokemonType::Water)
        || (attacker.ability == Ability::Swarm
            && is_third_hp(attacker)
            && move_.type_ == PokemonType::Bug)
        || (attacker.ability == Ability::DragonMaw && move_.type_ == PokemonType::Dragon)
        || (attacker.ability == Ability::FlashFire
            && attacker.ability_on
            && move_.type_ == PokemonType::Fire)
        || (attacker.ability == Ability::Steelworker && move_.type_ == PokemonType::Steel)
        || (matches!(attacker.ability, Ability::Plus | Ability::Minus)
            && attacker.ability_on
            && move_.category == Category::Special)
        || (attacker.ability == Ability::Sharpness && move_.is_slice)
        || (attacker.ability == Ability::RockyPayload && move_.type_ == PokemonType::Rock)
        || (attacker.ability == Ability::FireMane && move_.type_ == PokemonType::Fire)
    {
        push_mod(&mut mods, modifiers, "attack ability 1.5", MOD_1_5);
    } else if attacker.ability == Ability::SolarPower
        && field.weather.is_sun()
        && move_.category == Category::Special
        && attacker.item != Item::UtilityUmbrella
    {
        push_mod(&mut mods, modifiers, "Solar Power", MOD_1_5);
    } else if paradox_offense_boosts(attacker, attacker_highest_stat, move_) {
        push_mod(&mut mods, modifiers, "Paradox ability attack", MOD_1_3);
    } else if attacker.ability == Ability::Transistor && move_.type_ == PokemonType::Electric {
        push_mod(&mut mods, modifiers, "Transistor", MOD_1_3);
    } else if (attacker.ability == Ability::OrichalcumPulse
        && field.weather == Weather::Sun
        && move_.category == Category::Physical
        && attacker.item != Item::UtilityUmbrella)
        || (attacker.ability == Ability::HadronEngine
            && field.terrain == crate::types::Terrain::Electric
            && move_.category == Category::Special)
    {
        push_mod(&mut mods, modifiers, "box legend attack", MOD_1_33);
    }

    if (attacker.item == Item::LightBall
        && matches!(attacker.name.as_str(), "Pikachu" | "Pikachu-Gmax"))
        || (attacker.ability == Ability::WaterBubble && move_.type_ == PokemonType::Water)
        || (matches!(attacker.ability, Ability::HugePower | Ability::PurePower)
            && move_.category == Category::Physical)
        || (attacker.ability == Ability::Stakeout && attacker.ability_on)
    {
        push_mod(&mut mods, modifiers, "attack/item 2.0", MOD_DOUBLE);
    }

    if (def_ability == Ability::ThickFat
        && matches!(move_.type_, PokemonType::Fire | PokemonType::Ice))
        || (def_ability == Ability::WaterBubble && move_.type_ == PokemonType::Fire)
        || (def_ability == Ability::PurifyingSalt && move_.type_ == PokemonType::Ghost)
        || (def_ability == Ability::Heatproof && move_.type_ == PokemonType::Fire)
    {
        push_mod(&mut mods, modifiers, "defensive attack reduction", MOD_HALF);
    }

    if (attacker.item == Item::ChoiceBand && move_.category == Category::Physical)
        || (attacker.item == Item::ChoiceSpecs && move_.category == Category::Special)
    {
        push_mod(&mut mods, modifiers, "Choice item", MOD_1_5);
    }
    for &modifier in &attacker.custom_attack_mods {
        push_mod(&mut mods, modifiers, "custom attack modifier", modifier);
    }

    mods
}

fn paradox_offense_boosts(attacker: &Pokemon, attacker_highest_stat: Stat, move_: &Move) -> bool {
    attacker.paradox_ability_boost
        && ((attacker_highest_stat == Stat::Attack && move_.category == Category::Physical)
            || (attacker_highest_stat == Stat::SpecialAttack
                && move_.category == Category::Special))
}
