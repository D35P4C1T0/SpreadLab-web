use crate::data::items::berry_resist_type;
use crate::mechanics::modifiers::{
    MODIFIER_DENOMINATOR, MOD_0_67_DOUBLES_SCREEN, MOD_1_2, MOD_1_25, MOD_1_33, MOD_1_5,
    MOD_DOUBLE, MOD_HALF, MOD_LIFE_ORB, MOD_THREE_QUARTERS,
};
use crate::types::{Ability, Category, Field, Format, Item, Move, Pokemon, PokemonType};

use super::{makes_effective_contact, push_mod, ModifierBreakdown};

#[allow(clippy::too_many_arguments)]
pub(super) fn calc_final_mods(
    move_: &Move,
    attacker: &Pokemon,
    defender: &Pokemon,
    def_ability: Ability,
    field: &Field,
    is_critical: bool,
    type_effectiveness: f32,
    defender_current_hp: u16,
    defender_max_hp: u16,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> Vec<i32> {
    let mut mods = Vec::new();
    let ignores_screens = move_.ignores_screens || attacker.ability == Ability::Infiltrator;
    if field.defender_side.aurora_veil && !is_critical && !ignores_screens {
        let modifier = if field.format == Format::Singles {
            MOD_HALF
        } else {
            MOD_0_67_DOUBLES_SCREEN
        };
        push_mod(&mut mods, modifiers, "Aurora Veil", modifier);
    } else if field.defender_side.reflect
        && move_.category == Category::Physical
        && !is_critical
        && !ignores_screens
    {
        let modifier = if field.format == Format::Singles {
            MOD_HALF
        } else {
            MOD_0_67_DOUBLES_SCREEN
        };
        push_mod(&mut mods, modifiers, "Reflect", modifier);
    } else if field.defender_side.light_screen
        && move_.category == Category::Special
        && !is_critical
        && !ignores_screens
    {
        let modifier = if field.format == Format::Singles {
            MOD_HALF
        } else {
            MOD_0_67_DOUBLES_SCREEN
        };
        push_mod(&mut mods, modifiers, "Light Screen", modifier);
    }

    if attacker.ability == Ability::Neuroforce && type_effectiveness > 1.0 {
        push_mod(&mut mods, modifiers, "Neuroforce", MOD_1_25);
    }
    if matches!(move_.name.as_str(), "Collision Course" | "Electro Drift")
        && type_effectiveness > 1.0
    {
        push_mod(
            &mut mods,
            modifiers,
            "super-effective signature move",
            MOD_1_33,
        );
    }
    if attacker.ability == Ability::Sniper && is_critical {
        push_mod(&mut mods, modifiers, "Sniper", MOD_1_5);
    }
    if attacker.ability == Ability::TintedLens && type_effectiveness < 1.0 {
        push_mod(&mut mods, modifiers, "Tinted Lens", MOD_DOUBLE);
    }

    if matches!(def_ability, Ability::Multiscale | Ability::ShadowShield)
        && defender_current_hp == defender_max_hp
    {
        push_mod(&mut mods, modifiers, "Multiscale", MOD_HALF);
    }
    if def_ability == Ability::Fluffy && makes_effective_contact(attacker, move_) {
        push_mod(&mut mods, modifiers, "Fluffy contact", MOD_HALF);
    }
    if def_ability == Ability::AuraGuard && makes_effective_contact(attacker, move_) {
        push_mod(&mut mods, modifiers, "Aura Guard", MOD_HALF);
    }
    if def_ability == Ability::PunkRock && move_.is_sound {
        push_mod(&mut mods, modifiers, "Punk Rock defense", MOD_HALF);
    }
    if def_ability == Ability::IceScales && move_.category == Category::Special {
        push_mod(&mut mods, modifiers, "Ice Scales", MOD_HALF);
    }
    if field.defender_side.friend_guard {
        push_mod(&mut mods, modifiers, "Friend Guard", MOD_THREE_QUARTERS);
    }
    if matches!(
        def_ability,
        Ability::SolidRock | Ability::Filter | Ability::PrismArmor
    ) && type_effectiveness > 1.0
    {
        push_mod(
            &mut mods,
            modifiers,
            "super-effective reducer",
            MOD_THREE_QUARTERS,
        );
    }
    if def_ability == Ability::Fluffy && move_.type_ == PokemonType::Fire {
        push_mod(&mut mods, modifiers, "Fluffy fire", MOD_DOUBLE);
    }
    if attacker.item == Item::ExpertBelt && type_effectiveness > 1.0 {
        push_mod(&mut mods, modifiers, "Expert Belt", MOD_1_2);
    } else if attacker.item == Item::LifeOrb {
        push_mod(&mut mods, modifiers, "Life Orb", MOD_LIFE_ORB);
    }
    if berry_resist_type(defender.item) == Some(move_.type_)
        && (type_effectiveness > 1.0 || move_.type_ == PokemonType::Normal)
        && !matches!(attacker.ability, Ability::Unnerve | Ability::AsOne)
    {
        let modifier = if def_ability == Ability::Ripen {
            MODIFIER_DENOMINATOR / 4
        } else {
            MOD_HALF
        };
        push_mod(&mut mods, modifiers, "resist berry", modifier);
    }
    for &modifier in &attacker.custom_final_mods {
        push_mod(&mut mods, modifiers, "custom final modifier", modifier);
    }
    mods
}
