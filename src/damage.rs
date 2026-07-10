use crate::data::items::{
    drive_type, item_boost_type, locked_item_for_species, memory_type, natural_gift,
};
use crate::data::type_chart::move_effectiveness;
use crate::mechanics::modifiers::{
    apply_mod, chain_mods, modified_stat, poke_round_ratio, MODIFIER_DENOMINATOR, MOD_1_2, MOD_1_3,
    MOD_1_5, MOD_DOUBLE, MOD_HALF, MOD_THREE_QUARTERS,
};
use crate::stats::calculate_stats;
use crate::types::{
    Ability, CalcError, Category, Field, Format, Item, Move, Pokemon, PokemonType, Ruleset, Stat,
    StatusCondition, Weather,
};
use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

mod attack_mods;
mod base_power;
mod bp_mods;
mod final_mods;

use attack_mods::calc_attack_mods;
use base_power::calc_base_power;
use bp_mods::calc_bp_mods;
use final_mods::calc_final_mods;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CalcInput {
    pub attacker: Pokemon,
    pub defender: Pokemon,
    pub move_: Move,
    pub field: Field,
    pub ruleset: Ruleset,
}

/// Four-move, two-direction calculation input matching `CALCULATE_ALL_MOVES_SV`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BatchCalcInput {
    pub left: Pokemon,
    pub right: Pokemon,
    pub left_moves: [Move; 4],
    pub right_moves: [Move; 4],
    /// Field oriented with `left` attacking `right`.
    pub left_to_right_field: Field,
    /// Field oriented with `right` attacking `left`.
    pub right_to_left_field: Field,
    pub ruleset: Ruleset,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ModifierBreakdown {
    pub label: String,
    pub modifier: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResolvedMove {
    pub name: String,
    pub type_: PokemonType,
    pub category: Category,
    pub base_power: u16,
    pub hits: u8,
    pub is_critical: bool,
    pub is_spread: bool,
    pub makes_contact: bool,
    pub is_priority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DamageOutcome {
    Damage,
    Status,
    ImmuneOrFailed,
    Fixed,
    Counter,
    HpShare,
}

impl ModifierBreakdown {
    fn new(label: impl Into<String>, modifier: i32) -> Self {
        Self {
            label: label.into(),
            modifier,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DamageResult {
    pub min_damage: u16,
    pub max_damage: u16,
    pub damage_rolls: Vec<u16>,
    pub hit_rolls: Vec<Vec<u16>>,
    pub percent_range: (f32, f32),
    /// Cumulative KO chance after using this move 1, 2, 3, and 4 times.
    ///
    /// This models one consumed defender healing item in between hits and uses,
    /// so callers can surface text such as "guaranteed 2HKO after Sitrus Berry
    /// recovery" without reimplementing item-trigger timing.
    pub ko_chance_by_move_use: Vec<f32>,
    pub ko_chance: Option<f32>,
    pub outcome: DamageOutcome,
    pub resolved_move: Option<ResolvedMove>,
    /// Signed defender HP change for effects such as Pain Split.
    /// Positive values damage the defender; negative values heal it.
    pub defender_hp_delta: Option<i32>,
    pub applied_modifiers: Vec<ModifierBreakdown>,
    pub debug: Vec<String>,
}

/// Results for all four moves in both directions after one preprocessing pass.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BatchDamageResult {
    pub left: Vec<DamageResult>,
    pub right: Vec<DamageResult>,
    pub left_state: Pokemon,
    pub right_state: Pokemon,
}

const KO_CHANCE_MAX_USES: usize = 4;
const KO_CHANCE_SEQUENCE_LIMIT: usize = 65_536;

pub fn calculate_damage(input: CalcInput) -> Result<DamageResult, CalcError> {
    match input.ruleset {
        Ruleset::Champions => calculate_champions_damage(input),
    }
}

pub fn calculate_all_moves(input: BatchCalcInput) -> Result<BatchDamageResult, CalcError> {
    let mut left = input.left;
    let mut right = input.right;
    let mut field = input.left_to_right_field;
    let mut entry_modifiers = Vec::new();
    preprocess_battle_state(&mut left, &mut right, &mut field, &mut entry_modifiers)?;
    let mut reverse_field = input.right_to_left_field;
    reverse_field.weather = field.weather;
    reverse_field.terrain = field.terrain;
    reverse_field.neutralizing_gas = field.neutralizing_gas;
    if entry_modifiers
        .iter()
        .any(|modifier| modifier.label == "Screen Cleaner")
    {
        reverse_field.defender_side.reflect = false;
        reverse_field.defender_side.light_screen = false;
        reverse_field.defender_side.aurora_veil = false;
    }
    let left_moves = input.left_moves;
    let right_moves = input.right_moves;

    let mut left_results = Vec::with_capacity(4);
    for mut move_ in left_moves.clone() {
        hydrate_counter_move(
            &mut move_,
            &right_moves,
            &right,
            &left,
            reverse_field,
            input.ruleset,
            &entry_modifiers,
        )?;
        left_results.push(calculate_champions_damage_preprocessed(
            CalcInput {
                attacker: left.clone(),
                defender: right.clone(),
                move_,
                field,
                ruleset: input.ruleset,
            },
            entry_modifiers.clone(),
        )?);
    }

    let mut right_results = Vec::with_capacity(4);
    for mut move_ in right_moves {
        hydrate_counter_move(
            &mut move_,
            &left_moves,
            &left,
            &right,
            field,
            input.ruleset,
            &entry_modifiers,
        )?;
        right_results.push(calculate_champions_damage_preprocessed(
            CalcInput {
                attacker: right.clone(),
                defender: left.clone(),
                move_,
                field: reverse_field,
                ruleset: input.ruleset,
            },
            entry_modifiers.clone(),
        )?);
    }

    Ok(BatchDamageResult {
        left: left_results,
        right: right_results,
        left_state: left,
        right_state: right,
    })
}

fn hydrate_counter_move(
    move_: &mut Move,
    opponent_moves: &[Move; 4],
    opponent: &Pokemon,
    user: &Pokemon,
    field: Field,
    ruleset: Ruleset,
    entry_modifiers: &[ModifierBreakdown],
) -> Result<(), CalcError> {
    if !matches!(
        move_.name.as_str(),
        "Counter" | "Mirror Coat" | "Metal Burst" | "Comeuppance"
    ) || move_.countered_damage_rolls.is_some()
    {
        return Ok(());
    }
    let Some(countered_move) = opponent_moves.get(move_.countered_move_index as usize) else {
        return Ok(());
    };
    if matches!(
        countered_move.name.as_str(),
        "Counter" | "Mirror Coat" | "Metal Burst" | "Comeuppance"
    ) {
        return Ok(());
    }
    let result = calculate_champions_damage_preprocessed(
        CalcInput {
            attacker: opponent.clone(),
            defender: user.clone(),
            move_: countered_move.clone(),
            field,
            ruleset,
        },
        entry_modifiers.to_vec(),
    )?;
    move_.countered_move_category = Some(countered_move.category);
    move_.countered_damage_rolls = Some(if result.hit_rolls.len() > 1 {
        result.hit_rolls.last().cloned().unwrap_or_default()
    } else {
        result.damage_rolls
    });
    Ok(())
}

fn calculate_champions_damage(mut input: CalcInput) -> Result<DamageResult, CalcError> {
    apply_move_metadata_defaults(&mut input.move_);
    let mut entry_modifiers = Vec::new();
    preprocess_battle_state(
        &mut input.attacker,
        &mut input.defender,
        &mut input.field,
        &mut entry_modifiers,
    )?;
    calculate_champions_damage_preprocessed(input, entry_modifiers)
}

fn calculate_champions_damage_preprocessed(
    mut input: CalcInput,
    entry_modifiers: Vec<ModifierBreakdown>,
) -> Result<DamageResult, CalcError> {
    apply_move_metadata_defaults(&mut input.move_);
    apply_hit_count_defaults(&mut input.move_, input.attacker.ability);

    let requested_hits = input.move_.hits.max(1);
    let parental_bond_hits = input.attacker.ability == Ability::ParentalBond
        && requested_hits == 1
        && (input.field.format == Format::Singles
            || !input.move_.is_spread
            || input.move_.targets_single_target);
    let hit_count = if parental_bond_hits {
        2
    } else {
        requested_hits
    };

    if hit_count == 1 {
        return calculate_champions_single_hit(input, entry_modifiers);
    }

    let defender_max_hp = input
        .defender
        .max_hp_override
        .unwrap_or(calculate_stats(&input.defender, Ruleset::Champions)?.hp);
    let defender_current_hp = input.defender.current_hp.unwrap_or(defender_max_hp);
    let mut hit_rolls = Vec::with_capacity(hit_count as usize);
    let mut applied_modifiers = entry_modifiers;
    let mut debug = vec![format!("hits={hit_count}")];
    let mut resolved_move = None;
    let mut outcome = DamageOutcome::Damage;
    let mut attacker = input.attacker;
    let mut defender = input.defender;
    let initial_defender_item = defender.item;
    let mut field = input.field;

    for hit_index in 0..hit_count {
        let mut hit_move = input.move_.clone();
        if matches!(hit_move.name.as_str(), "Triple Kick" | "Triple Axel") {
            hit_move.current_triple_hit = Some(hit_index + 1);
        }
        let hit_attacker = attacker.clone();
        if parental_bond_hits && hit_index == 1 {
            hit_move.is_parental_bond_child = true;
        }
        let hit_result = calculate_champions_single_hit(
            CalcInput {
                attacker: hit_attacker,
                defender: defender.clone(),
                move_: hit_move,
                field,
                ruleset: input.ruleset,
            },
            Vec::new(),
        )?;
        applied_modifiers.extend(hit_result.applied_modifiers.clone());
        if resolved_move.is_none() {
            resolved_move = hit_result.resolved_move.clone();
            outcome = hit_result.outcome;
        }
        debug.extend(hit_result.debug);
        hit_rolls.push(hit_result.damage_rolls);

        if hit_index + 1 < hit_count {
            apply_between_hit_effects(
                &mut attacker,
                &mut defender,
                &input.move_,
                &mut field,
                &hit_result.applied_modifiers,
            );
        }
    }

    let damage_rolls = combine_hit_rolls(&hit_rolls);
    let min_damage = *damage_rolls.first().unwrap_or(&0);
    let max_damage = *damage_rolls.last().unwrap_or(&0);
    let percent_range = (
        min_damage as f32 * 100.0 / defender_max_hp as f32,
        max_damage as f32 * 100.0 / defender_max_hp as f32,
    );
    let ko_chance_by_move_use = ko_chances_after_move_uses(
        &hit_rolls,
        defender_current_hp,
        defender_max_hp,
        initial_defender_item,
        residual_effects(
            &defender,
            &field,
            defender_max_hp,
            input.move_.name == "Psychic Noise",
        ),
        healing_item_suppressed(&input.move_, &attacker),
        KO_CHANCE_MAX_USES,
    );
    let ko_chance = ko_chance_by_move_use.first().copied();

    Ok(DamageResult {
        min_damage,
        max_damage,
        damage_rolls,
        hit_rolls,
        percent_range,
        ko_chance_by_move_use,
        ko_chance,
        outcome,
        resolved_move,
        defender_hp_delta: None,
        applied_modifiers,
        debug,
    })
}

fn calculate_champions_single_hit(
    input: CalcInput,
    initial_modifiers: Vec<ModifierBreakdown>,
) -> Result<DamageResult, CalcError> {
    let mut attacker = input.attacker;
    let mut defender = input.defender;
    let mut move_ = input.move_;
    let field = input.field;
    let mut modifiers = initial_modifiers;
    let mut debug = Vec::new();
    let original_attacker_types = attacker.types;

    let attacker_stats = calculate_stats(&attacker, Ruleset::Champions)?;
    let defender_stats = calculate_stats(&defender, Ruleset::Champions)?;
    let defender_max_hp = defender.max_hp_override.unwrap_or(defender_stats.hp);
    let defender_current_hp = defender.current_hp.unwrap_or(defender_max_hp);
    let attacker_max_hp = attacker.max_hp_override.unwrap_or(attacker_stats.hp);
    let attacker_current_hp = attacker.current_hp.unwrap_or(attacker_max_hp);

    if attacker.is_terastalized {
        if let Some(tera_type) = attacker.tera_type {
            if tera_type != PokemonType::Stellar {
                attacker.types = [Some(tera_type), None];
            }
        }
    }
    if defender.is_terastalized {
        if let Some(tera_type) = defender.tera_type {
            if tera_type != PokemonType::Stellar {
                defender.types = [Some(tera_type), None];
            }
        }
    }

    apply_move_metadata_defaults(&mut move_);
    apply_move_type_changes(&mut move_, &attacker, &field);
    apply_priority_defaults(
        &mut move_,
        &attacker,
        attacker_current_hp,
        attacker_max_hp,
        &field,
    );
    if move_.name == "Expanding Force"
        && field.terrain == crate::types::Terrain::Psychic
        && is_grounded(&attacker, &field)
    {
        move_.is_spread = true;
    }
    let ate_ize_boosted = apply_ability_type_change(&mut move_, &attacker, &mut modifiers);

    if move_.name == "Pain Split" && attacker.item != Item::AssaultVest {
        let hp_delta = (defender_current_hp as i32 - attacker_current_hp as i32).div_euclid(2);
        let damage = hp_delta.max(0) as u16;
        let mut result = single_damage_result(
            damage,
            DamageResolutionContext {
                defender_max_hp,
                defender_current_hp,
                defender_item: defender.item,
                residual: residual_effects(&defender, &field, defender_max_hp, false),
                healing_suppressed: false,
            },
            modifiers,
            debug,
        );
        result.outcome = DamageOutcome::HpShare;
        result.resolved_move = Some(resolved_move(&move_, move_.base_power, false));
        result.defender_hp_delta = Some(hp_delta);
        return Ok(result);
    }

    if move_.base_power == 0 || move_.category == Category::Status {
        return Ok(DamageResult {
            min_damage: 0,
            max_damage: 0,
            damage_rolls: vec![0],
            hit_rolls: vec![vec![0]],
            percent_range: (0.0, 0.0),
            ko_chance_by_move_use: vec![0.0; KO_CHANCE_MAX_USES],
            ko_chance: Some(0.0),
            outcome: DamageOutcome::Status,
            resolved_move: Some(resolved_move(&move_, move_.base_power, false)),
            defender_hp_delta: None,
            applied_modifiers: modifiers,
            debug,
        });
    }

    let attack_modified_stats = ModifiedStats::from(&attacker, attacker_stats)?;
    let defense_modified_stats = ModifiedStats::from(&defender, defender_stats)?;
    let attacker_highest_stat = highest_stat(&attacker, attack_modified_stats);
    let defender_highest_stat = highest_stat(&defender, defense_modified_stats);
    let attacker_speed = final_speed(
        &attacker,
        attack_modified_stats.speed,
        field.attacker_tailwind,
        field.attacker_swamp,
        attacker_highest_stat,
        &field,
    );
    let defender_speed = final_speed(
        &defender,
        defense_modified_stats.speed,
        field.defender_tailwind,
        field.defender_swamp,
        defender_highest_stat,
        &field,
    );
    let attacker_moves_first = attacker_speed > defender_speed;

    let def_ability =
        ability_after_ignore(attacker.ability, defender.ability, &move_, &mut modifiers);
    let mut type_effectiveness = move_effectiveness(
        &move_.name,
        move_.type_,
        defender.types,
        field.foresight,
        matches!(attacker.ability, Ability::Scrappy | Ability::MindEye),
        field.gravity,
        defender.item == Item::IronBall,
        defender.item == Item::RingTarget,
        field.weather == Weather::StrongWinds,
    );
    if move_.type_ == PokemonType::Ground
        && move_.name == "Thousand Arrows"
        && defender.has_type(PokemonType::Flying)
        && !field.gravity
    {
        type_effectiveness = 1.0;
        modifiers.push(ModifierBreakdown::new("Thousand Arrows type override", 0));
    } else if move_.type_ == PokemonType::Stellar && defender.is_terastalized {
        type_effectiveness = 2.0;
        modifiers.push(ModifierBreakdown::new("Stellar vs Tera", 0));
    } else if def_ability == Ability::TeraShell
        && defender_current_hp == defender_max_hp
        && type_effectiveness > 0.5
    {
        type_effectiveness = 0.5;
        modifiers.push(ModifierBreakdown::new("Tera Shell type override", 0));
    }

    if is_immune(
        &move_,
        &attacker,
        &defender,
        def_ability,
        &field,
        type_effectiveness,
        &mut modifiers,
    ) {
        let mut result = zero_damage(defender_max_hp, modifiers, debug);
        result.outcome = DamageOutcome::ImmuneOrFailed;
        result.resolved_move = Some(resolved_move(&move_, move_.base_power, false));
        return Ok(result);
    }

    if let Some(result) = set_damage_result(
        &move_,
        &attacker,
        &defender,
        attacker_current_hp,
        defender_current_hp,
        defender_max_hp,
        field.protect,
        &field,
        modifiers.clone(),
        debug.clone(),
    ) {
        let mut result = result;
        result.resolved_move = Some(resolved_move(&move_, move_.base_power, false));
        if result.outcome == DamageOutcome::Damage {
            result.outcome = DamageOutcome::Fixed;
        }
        return Ok(result);
    }

    let mut is_critical = move_.is_critical
        || (attacker.ability == Ability::Merciless
            && matches!(
                defender.status,
                StatusCondition::Poisoned | StatusCondition::BadlyPoisoned
            ));
    if is_critical && matches!(def_ability, Ability::BattleArmor | Ability::ShellArmor) {
        is_critical = false;
        modifiers.push(ModifierBreakdown::new("critical blocked", 0));
    }
    let base_power = calc_base_power(
        &move_,
        &attacker,
        &defender,
        attacker_current_hp,
        attacker_max_hp,
        defender_current_hp,
        defender_max_hp,
        &field,
        attacker_speed,
        defender_speed,
        &mut modifiers,
    )?;
    if base_power == 0 {
        let mut result = zero_damage(defender_max_hp, modifiers, debug);
        result.outcome = DamageOutcome::ImmuneOrFailed;
        result.resolved_move = Some(resolved_move(&move_, 0, is_critical));
        return Ok(result);
    }
    let bp_mods = calc_bp_mods(
        &move_,
        &attacker,
        &defender,
        &field,
        base_power,
        ate_ize_boosted,
        attacker_moves_first,
        defender_current_hp,
        defender_max_hp,
        &mut modifiers,
    );
    let mut base_power = apply_mod(base_power as i32, chain_mods(&bp_mods)).max(1);
    if base_power < 60 && can_tera_boost_to_sixty(&move_, &attacker) {
        base_power = 60;
        modifiers.push(ModifierBreakdown::new("Tera 60 BP floor", 0));
    }

    let hits_physical = move_.category == Category::Physical || move_.deals_physical_damage;
    let attack_stat = if move_.name == "Body Press" {
        Stat::Defense
    } else if move_.category == Category::Physical {
        Stat::Attack
    } else {
        Stat::SpecialAttack
    };
    let defense_stat = if hits_physical {
        Stat::Defense
    } else {
        Stat::SpecialDefense
    };

    let (attack_source_raw, attack_source_mod, attack_source_boosts) = if move_.name == "Foul Play"
    {
        (defender_stats, defense_modified_stats, defender.boosts)
    } else {
        (attacker_stats, attack_modified_stats, attacker.boosts)
    };

    let mid_move_attack_boost =
        if move_.name == "Spectral Thief" && defender.boosts.get(attack_stat) > 0 {
            Some((attacker.boosts.get(attack_stat) + defender.boosts.get(attack_stat)).min(6))
        } else if matches!(move_.name.as_str(), "Meteor Beam" | "Electro Shot") {
            let change = if attacker.ability == Ability::Contrary {
                -1
            } else {
                1
            };
            let current = attacker.boosts.get(attack_stat);
            (current + change)
                .clamp(-6, 6)
                .ne(&current)
                .then_some(current + change)
        } else {
            None
        };

    let mut attack =
        if def_ability == Ability::Unaware && attack_source_boosts.get(attack_stat) != 0 {
            modifiers.push(ModifierBreakdown::new("Unaware ignores attack boost", 0));
            attack_source_raw.get(attack_stat)
        } else if let Some(boost) = mid_move_attack_boost {
            modifiers.push(ModifierBreakdown::new("mid-move attack boost", 0));
            modified_stat(attack_source_raw.get(attack_stat), boost)?
        } else if is_critical && attack_source_boosts.get(attack_stat) < 0 {
            attack_source_raw.get(attack_stat)
        } else {
            attack_source_mod.get(attack_stat)
        } as i32;

    if attacker.ability == Ability::Hustle && move_.category == Category::Physical {
        attack = poke_round_ratio(attack as i64 * 3, 2);
        modifiers.push(ModifierBreakdown::new("Hustle direct attack", MOD_1_5));
    }

    let at_mods = calc_attack_mods(
        &move_,
        &attacker,
        attacker_highest_stat,
        def_ability,
        &field,
        &mut modifiers,
    );
    attack = apply_mod(attack, chain_mods(&at_mods)).max(1);

    let mut defense =
        if attacker.ability == Ability::Unaware && defender.boosts.get(defense_stat) != 0 {
            modifiers.push(ModifierBreakdown::new("Unaware ignores defense boost", 0));
            defender_stats.get(defense_stat)
        } else if move_.ignores_defense_boosts && defender.boosts.get(defense_stat) != 0 {
            modifiers.push(ModifierBreakdown::new("move ignores defense boosts", 0));
            defender_stats.get(defense_stat)
        } else if is_critical && defender.boosts.get(defense_stat) > 0 {
            defender_stats.get(defense_stat)
        } else {
            defense_modified_stats.get(defense_stat)
        } as i32;

    if ((field.weather == Weather::Sand && defender.has_type(PokemonType::Rock) && !hits_physical)
        || (field.weather == Weather::Snow && defender.has_type(PokemonType::Ice) && hits_physical))
        && attacker.ability != Ability::MegaSol
    {
        defense = poke_round_ratio(defense as i64 * 3, 2);
        modifiers.push(ModifierBreakdown::new("weather defense", MOD_1_5));
    }

    let df_mods = calc_defense_mods(
        &move_,
        &defender,
        defender_highest_stat,
        def_ability,
        &field,
        hits_physical,
        &mut modifiers,
    );
    defense = apply_mod(defense, chain_mods(&df_mods)).max(1);

    let mut base_damage =
        ((((2 * attacker.level as i32) / 5 + 2) * base_power * attack) / defense) / 50 + 2;
    debug.push(format!(
        "base_damage={base_damage}, bp={base_power}, attack={attack}, defense={defense}"
    ));

    if field.format != Format::Singles && move_.is_spread && !move_.targets_single_target {
        base_damage = apply_mod(base_damage, MOD_THREE_QUARTERS);
        modifiers.push(ModifierBreakdown::new("spread", MOD_THREE_QUARTERS));
    }
    if move_.is_parental_bond_child {
        base_damage = apply_mod(base_damage, 0x0400);
        modifiers.push(ModifierBreakdown::new("Parental Bond child", 0x0400));
    }

    if weather_damage_boost(
        &move_,
        attacker.ability,
        field.weather,
        attacker.item,
        defender.item,
    ) {
        base_damage = apply_mod(base_damage, MOD_1_5);
        modifiers.push(ModifierBreakdown::new("weather damage boost", MOD_1_5));
    } else if weather_damage_drop(&move_, attacker.ability, field.weather, defender.item) {
        base_damage = apply_mod(base_damage, MOD_HALF);
        modifiers.push(ModifierBreakdown::new("weather damage drop", MOD_HALF));
    }

    if field.glaive_rush {
        base_damage = apply_mod(base_damage, MOD_DOUBLE);
        modifiers.push(ModifierBreakdown::new("Glaive Rush", MOD_DOUBLE));
    }

    if is_critical {
        base_damage = base_damage * 3 / 2;
        modifiers.push(ModifierBreakdown::new("critical", MOD_1_5));
    }

    let stab_mod = stab_modifier(&attacker, original_attacker_types, &move_, &mut modifiers);
    let apply_burn = attacker.status == StatusCondition::Burned
        && move_.category == Category::Physical
        && attacker.ability != Ability::Guts
        && !move_.ignores_burn;
    if apply_burn {
        modifiers.push(ModifierBreakdown::new("burn", MOD_HALF));
    }

    let final_mods = calc_final_mods(
        &move_,
        &attacker,
        &defender,
        def_ability,
        &field,
        is_critical,
        type_effectiveness,
        defender_current_hp,
        defender_max_hp,
        &mut modifiers,
    );
    let final_mod = chain_mods(&final_mods);
    let quartered_by_protect = field.protect
        && (move_.is_z
            || move_.is_max
            || move_.breaks_protect
            || matches!(
                attacker.ability,
                Ability::PiercingDrill | Ability::UnseenFist
            ));
    if quartered_by_protect {
        modifiers.push(ModifierBreakdown::new("Protect", 0x0400));
    }

    let mut rolls = Vec::with_capacity(16);
    for random_factor in 85..=100 {
        let mut damage = base_damage * random_factor / 100;
        damage = apply_mod(damage, stab_mod);
        damage = (damage as f32 * type_effectiveness).floor() as i32;
        if apply_burn {
            damage /= 2;
        }
        damage = apply_mod(damage, final_mod);
        if quartered_by_protect {
            damage = apply_mod(damage, 0x0400);
        }
        damage = damage.max(1);
        if damage > 65535 {
            damage %= 65536;
        }
        rolls.push(damage as u16);
    }
    rolls.sort_unstable();

    let min_damage = *rolls.first().unwrap_or(&0);
    let max_damage = *rolls.last().unwrap_or(&0);
    let percent_range = (
        min_damage as f32 * 100.0 / defender_max_hp as f32,
        max_damage as f32 * 100.0 / defender_max_hp as f32,
    );
    let hit_rolls = vec![rolls.clone()];
    let ko_chance_by_move_use = ko_chances_after_move_uses(
        &hit_rolls,
        defender_current_hp,
        defender_max_hp,
        defender.item,
        residual_effects(
            &defender,
            &field,
            defender_max_hp,
            move_.name == "Psychic Noise",
        ),
        healing_item_suppressed(&move_, &attacker),
        KO_CHANCE_MAX_USES,
    );
    let ko_chance = ko_chance_by_move_use.first().copied();

    Ok(DamageResult {
        min_damage,
        max_damage,
        damage_rolls: rolls.clone(),
        hit_rolls,
        percent_range,
        ko_chance_by_move_use,
        ko_chance,
        outcome: DamageOutcome::Damage,
        resolved_move: Some(resolved_move(&move_, base_power as u16, is_critical)),
        defender_hp_delta: None,
        applied_modifiers: modifiers,
        debug,
    })
}

fn combine_hit_rolls(hit_rolls: &[Vec<u16>]) -> Vec<u16> {
    let mut totals = vec![0u16];
    for rolls in hit_rolls {
        let mut next = Vec::with_capacity(totals.len() * rolls.len());
        for total in &totals {
            for roll in rolls {
                next.push(total.saturating_add(*roll));
            }
        }
        totals = next;
    }
    totals.sort_unstable();
    totals
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct KoState {
    hp: u16,
    item: Item,
    toxic_counter: u8,
}

#[derive(Debug, Clone, Copy)]
struct HealingItemRecovery {
    heal: u16,
    threshold: u16,
}

#[derive(Debug, Clone, Copy)]
struct ResidualEffects {
    defender_ability: Ability,
    initial_damage: u16,
    initial_toxic_counter: u8,
    healing_or_damage: i16,
    burn_damage: u16,
    poison_damage: u16,
    toxic: bool,
    leech_seed_damage: u16,
    salt_cure_damage: u16,
    prevents_healing: bool,
}

#[derive(Debug, Clone, Copy)]
struct DamageResolutionContext {
    defender_max_hp: u16,
    defender_current_hp: u16,
    defender_item: Item,
    residual: ResidualEffects,
    healing_suppressed: bool,
}

fn ko_chances_after_move_uses(
    hit_rolls: &[Vec<u16>],
    defender_current_hp: u16,
    defender_max_hp: u16,
    defender_item: Item,
    residual: ResidualEffects,
    healing_suppressed: bool,
    max_uses: usize,
) -> Vec<f32> {
    if hit_rolls.is_empty() || hit_rolls.iter().any(Vec::is_empty) {
        return vec![0.0; max_uses];
    }

    let Some(sequence_count) = hit_sequence_count(hit_rolls) else {
        return vec![raw_ko_chance(hit_rolls, defender_current_hp)];
    };
    if sequence_count > KO_CHANCE_SEQUENCE_LIMIT {
        return vec![raw_ko_chance(hit_rolls, defender_current_hp)];
    }

    let sequence_probability = 1.0 / sequence_count as f64;
    if residual.initial_damage >= defender_current_hp {
        return vec![1.0; max_uses];
    }
    let mut active_states = HashMap::from([(
        KoState {
            hp: defender_current_hp - residual.initial_damage,
            item: defender_item,
            toxic_counter: if residual.toxic {
                residual.initial_toxic_counter.max(1)
            } else {
                0
            },
        },
        1.0f64,
    )]);
    let mut cumulative_ko = 0.0f64;
    let mut chances = Vec::with_capacity(max_uses);

    for _ in 0..max_uses {
        let mut next_states = HashMap::<KoState, f64>::new();
        let mut ko_this_use = 0.0f64;

        for (state, state_probability) in active_states {
            visit_hit_sequence_outcomes(
                hit_rolls,
                0,
                state,
                defender_max_hp,
                residual.defender_ability,
                healing_suppressed,
                &mut |outcome| {
                    let probability = state_probability * sequence_probability;
                    if let Some(next_state) = outcome {
                        if let Some(next_state) =
                            apply_end_of_turn_effects(next_state, defender_max_hp, residual)
                        {
                            *next_states.entry(next_state).or_insert(0.0) += probability;
                        } else {
                            ko_this_use += probability;
                        }
                    } else {
                        ko_this_use += probability;
                    }
                },
            );
        }

        cumulative_ko = (cumulative_ko + ko_this_use).min(1.0);
        chances.push(cumulative_ko as f32);
        active_states = next_states;

        if cumulative_ko >= 1.0 || active_states.is_empty() {
            chances.resize(max_uses, cumulative_ko as f32);
            break;
        }
    }

    chances
}

fn hit_sequence_count(hit_rolls: &[Vec<u16>]) -> Option<usize> {
    hit_rolls
        .iter()
        .try_fold(1usize, |count, rolls| count.checked_mul(rolls.len()))
}

fn raw_ko_chance(hit_rolls: &[Vec<u16>], defender_current_hp: u16) -> f32 {
    let damage_rolls = combine_hit_rolls(hit_rolls);
    if damage_rolls.is_empty() {
        return 0.0;
    }
    let ko_rolls = damage_rolls
        .iter()
        .filter(|&&damage| damage >= defender_current_hp)
        .count();
    ko_rolls as f32 / damage_rolls.len() as f32
}

fn visit_hit_sequence_outcomes(
    hit_rolls: &[Vec<u16>],
    hit_index: usize,
    state: KoState,
    defender_max_hp: u16,
    defender_ability: Ability,
    healing_suppressed: bool,
    on_outcome: &mut impl FnMut(Option<KoState>),
) {
    if hit_index == hit_rolls.len() {
        on_outcome(Some(state));
        return;
    }

    for damage in &hit_rolls[hit_index] {
        let mut next_state = state;
        next_state.hp = next_state.hp.saturating_sub(*damage);
        if next_state.hp == 0 {
            visit_remaining_ko_outcomes(hit_rolls, hit_index + 1, on_outcome);
            continue;
        }
        if !healing_suppressed {
            if let Some(recovery) =
                healing_item_recovery(next_state.item, defender_max_hp, defender_ability)
            {
                if next_state.hp <= recovery.threshold {
                    next_state.hp = next_state
                        .hp
                        .saturating_add(recovery.heal)
                        .min(defender_max_hp);
                    next_state.item = Item::None;
                }
            }
        }
        visit_hit_sequence_outcomes(
            hit_rolls,
            hit_index + 1,
            next_state,
            defender_max_hp,
            defender_ability,
            healing_suppressed,
            on_outcome,
        );
    }
}

fn visit_remaining_ko_outcomes(
    hit_rolls: &[Vec<u16>],
    hit_index: usize,
    on_outcome: &mut impl FnMut(Option<KoState>),
) {
    if hit_index == hit_rolls.len() {
        on_outcome(None);
        return;
    }
    for _ in &hit_rolls[hit_index] {
        visit_remaining_ko_outcomes(hit_rolls, hit_index + 1, on_outcome);
    }
}

fn residual_effects(
    defender: &Pokemon,
    field: &Field,
    defender_max_hp: u16,
    prevents_healing: bool,
) -> ResidualEffects {
    let mut healing_or_damage = 0i16;
    let residual_sixteenth = defender_max_hp / 16;
    let residual_eighth = defender_max_hp / 8;
    let magic_guard = defender.ability == Ability::MagicGuard;
    let initial_damage = initial_hazard_damage(defender, field, defender_max_hp, magic_guard);

    match field.weather {
        Weather::Sun | Weather::HarshSun => {
            if matches!(defender.ability, Ability::DrySkin | Ability::SolarPower) {
                healing_or_damage -= residual_eighth as i16;
            }
        }
        Weather::Rain | Weather::HeavyRain => {
            if defender.ability == Ability::DrySkin {
                healing_or_damage += residual_eighth as i16;
            } else if defender.ability == Ability::RainDish {
                healing_or_damage += residual_sixteenth as i16;
            }
        }
        Weather::Sand => {
            if !magic_guard
                && !defender.has_type(PokemonType::Rock)
                && !defender.has_type(PokemonType::Ground)
                && !defender.has_type(PokemonType::Steel)
                && !matches!(
                    defender.ability,
                    Ability::Overcoat | Ability::SandForce | Ability::SandRush | Ability::SandVeil
                )
            {
                healing_or_damage -= residual_sixteenth as i16;
            }
        }
        Weather::Hail => {
            if defender.ability == Ability::IceBody {
                healing_or_damage += residual_sixteenth as i16;
            } else if !magic_guard
                && !defender.has_type(PokemonType::Ice)
                && !matches!(defender.ability, Ability::Overcoat | Ability::SnowCloak)
            {
                healing_or_damage -= residual_sixteenth as i16;
            }
        }
        Weather::Snow => {
            if defender.ability == Ability::IceBody {
                healing_or_damage += residual_sixteenth as i16;
            }
        }
        Weather::None | Weather::StrongWinds => {}
    }

    if defender.item == Item::Leftovers {
        healing_or_damage += residual_sixteenth as i16;
    }

    if field.terrain == crate::types::Terrain::Grassy && is_grounded(defender, field) {
        healing_or_damage += residual_sixteenth as i16;
    }

    let mut burn_damage = 0;
    let mut poison_damage = 0;
    let mut toxic = false;
    match defender.status {
        StatusCondition::Poisoned => {
            if defender.ability == Ability::PoisonHeal {
                healing_or_damage += residual_eighth as i16;
            } else if !magic_guard {
                poison_damage = residual_eighth;
            }
        }
        StatusCondition::BadlyPoisoned => {
            if defender.ability == Ability::PoisonHeal {
                healing_or_damage += residual_eighth as i16;
            } else if !magic_guard {
                toxic = true;
            }
        }
        StatusCondition::Burned if !magic_guard => {
            burn_damage = if defender.ability == Ability::Heatproof {
                defender_max_hp / 16 / 2
            } else {
                defender_max_hp / 16
            };
        }
        StatusCondition::Healthy
        | StatusCondition::Paralyzed
        | StatusCondition::Asleep
        | StatusCondition::Drowsy
        | StatusCondition::Frozen
        | StatusCondition::Burned => {}
    }

    let leech_seed_damage =
        if field.defender_leech_seed && !magic_guard && !defender.has_type(PokemonType::Grass) {
            residual_eighth
        } else {
            0
        };
    let salt_cure_damage = if field.defender_side.salt_cure && !magic_guard {
        if defender.has_type(PokemonType::Water) || defender.has_type(PokemonType::Steel) {
            defender_max_hp / 8
        } else {
            defender_max_hp / 16
        }
    } else {
        0
    };

    ResidualEffects {
        defender_ability: defender.ability,
        initial_damage,
        initial_toxic_counter: defender.toxic_counter,
        healing_or_damage,
        burn_damage,
        poison_damage,
        toxic,
        leech_seed_damage,
        salt_cure_damage,
        prevents_healing,
    }
}

fn initial_hazard_damage(
    defender: &Pokemon,
    field: &Field,
    defender_max_hp: u16,
    magic_guard: bool,
) -> u16 {
    if magic_guard {
        return 0;
    }
    let mut damage = 0u16;
    if field.defender_side.stealth_rock {
        let effectiveness = move_effectiveness(
            "Stealth Rock",
            PokemonType::Rock,
            defender.types,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        damage = damage
            .saturating_add(((defender_max_hp as f32 * effectiveness / 8.0).floor() as u16).max(1));
    }
    if is_grounded(defender, field) {
        let spikes = match field.defender_side.spikes.min(3) {
            1 => (defender_max_hp / 8).max(1),
            2 => defender_max_hp / 6,
            3 => defender_max_hp / 4,
            _ => 0,
        };
        damage = damage.saturating_add(spikes);
    }
    damage
}

fn apply_end_of_turn_effects(
    mut state: KoState,
    defender_max_hp: u16,
    residual: ResidualEffects,
) -> Option<KoState> {
    if state.hp == 0 {
        return None;
    }

    if residual.healing_or_damage > 0 && !residual.prevents_healing {
        state.hp = state
            .hp
            .saturating_add(residual.healing_or_damage as u16)
            .min(defender_max_hp);
    } else if residual.healing_or_damage < 0 {
        state.hp = state
            .hp
            .saturating_sub((-residual.healing_or_damage) as u16);
        if state.hp == 0 {
            return None;
        }
    }

    for damage in [
        residual.poison_damage,
        residual.burn_damage,
        residual.leech_seed_damage,
        residual.salt_cure_damage,
    ] {
        state.hp = state.hp.saturating_sub(damage);
        if state.hp == 0 {
            return None;
        }
    }

    if residual.toxic {
        let toxic_counter = state.toxic_counter.max(1);
        state.hp = state
            .hp
            .saturating_sub(defender_max_hp.saturating_mul(toxic_counter as u16) / 16);
        if state.hp == 0 {
            return None;
        }
        state.toxic_counter = toxic_counter.saturating_add(1);
    }

    Some(state)
}

fn healing_item_suppressed(move_: &Move, attacker: &Pokemon) -> bool {
    matches!(attacker.ability, Ability::Unnerve | Ability::AsOne)
        || matches!(
            move_.name.as_str(),
            "Knock Off" | "Psychic Noise" | "Bug Bite" | "Pluck" | "Incinerate"
        )
        || (attacker.item == Item::None && matches!(move_.name.as_str(), "Thief" | "Covet"))
}

fn healing_item_recovery(
    item: Item,
    max_hp: u16,
    defender_ability: Ability,
) -> Option<HealingItemRecovery> {
    let (mut heal, threshold) = match item {
        Item::OranBerry => (10, max_hp / 2),
        Item::SitrusBerry => (max_hp / 4, max_hp / 2),
        Item::FigyBerry
        | Item::IapapaBerry
        | Item::WikiBerry
        | Item::AguavBerry
        | Item::MagoBerry => {
            let threshold = if defender_ability == Ability::Gluttony {
                max_hp / 2
            } else {
                max_hp / 4
            };
            (max_hp / 3, threshold)
        }
        _ => return None,
    };

    if defender_ability == Ability::Ripen && heal > 0 {
        heal = heal.saturating_mul(2);
    }

    Some(HealingItemRecovery { heal, threshold })
}

fn apply_between_hit_effects(
    attacker: &mut Pokemon,
    defender: &mut Pokemon,
    move_: &Move,
    field: &mut Field,
    hit_modifiers: &[ModifierBreakdown],
) {
    if hit_modifiers
        .iter()
        .any(|modifier| modifier.label == "resist berry")
    {
        defender.item = Item::None;
    }
    if hit_modifiers
        .iter()
        .any(|modifier| modifier.label == "Multiscale")
    {
        defender.ability = Ability::None;
    }
    if defender.item == Item::KeeBerry
        && physical_hit_for_between_hit_effects(move_)
        && defender.boosts.defense < 6
    {
        defender.boosts.defense += 1;
        defender.item = Item::None;
    } else if defender.item == Item::MarangaBerry
        && !physical_hit_for_between_hit_effects(move_)
        && defender.boosts.special_defense < 6
    {
        defender.boosts.special_defense += 1;
        defender.item = Item::None;
    }
    if defender.ability == Ability::Stamina && physical_hit_for_between_hit_effects(move_) {
        defender.boosts.defense = (defender.boosts.defense + 1).min(6);
    }
    if defender.ability == Ability::WeakArmor && physical_hit_for_between_hit_effects(move_) {
        defender.boosts.defense = (defender.boosts.defense - 1).max(-6);
        defender.boosts.speed = (defender.boosts.speed + 2).min(6);
    }
    if gooey_like_between_hit_effect_applies(attacker, defender, move_) {
        attacker.boosts.speed = (attacker.boosts.speed - 1).max(-6);
        if attacker.ability == Ability::Defiant {
            attacker.boosts.attack = (attacker.boosts.attack + 2).min(6);
        } else if attacker.ability == Ability::Competitive {
            attacker.boosts.special_attack = (attacker.boosts.special_attack + 2).min(6);
        }
    }
    if spicy_spray_between_hit_effect_applies(attacker, defender, move_, field) {
        if matches!(attacker.item, Item::RawstBerry | Item::LumBerry) {
            attacker.item = Item::None;
        } else {
            attacker.status = StatusCondition::Burned;
        }
    }
}

fn physical_hit_for_between_hit_effects(move_: &Move) -> bool {
    move_.category == Category::Physical || move_.deals_physical_damage
}

fn gooey_like_between_hit_effect_applies(
    attacker: &Pokemon,
    defender: &Pokemon,
    move_: &Move,
) -> bool {
    if matches!(defender.ability, Ability::Gooey | Ability::TanglingHair)
        && makes_effective_contact(attacker, move_)
    {
        return true;
    }
    defender.ability == Ability::CottonDown
        && defender.boosts.speed > -6
        && (matches!(attacker.ability, Ability::Defiant | Ability::Competitive)
            || matches!(move_.name.as_str(), "Electro Ball" | "Gyro Ball"))
}

fn spicy_spray_between_hit_effect_applies(
    attacker: &Pokemon,
    defender: &Pokemon,
    move_: &Move,
    field: &Field,
) -> bool {
    defender.ability == Ability::SpicySpray
        && (attacker.ability == Ability::FlareBoost || move_.category == Category::Physical)
        && can_be_burned(attacker, move_, field)
}

fn can_be_burned(attacker: &Pokemon, move_: &Move, field: &Field) -> bool {
    attacker.status != StatusCondition::Burned
        && !attacker.has_type(PokemonType::Fire)
        && !(matches!(attacker.ability, Ability::Protean | Ability::Libero)
            && attacker.ability_on
            && move_.type_ == PokemonType::Fire)
        && !(attacker.ability == Ability::LeafGuard && field.weather.is_sun())
        && !matches!(
            attacker.ability,
            Ability::WaterVeil
                | Ability::WaterBubble
                | Ability::Comatose
                | Ability::ThermalExchange
                | Ability::PurifyingSalt
        )
        && (field.terrain != crate::types::Terrain::Misty || !is_grounded(attacker, field))
}

pub(super) fn makes_effective_contact(attacker: &Pokemon, move_: &Move) -> bool {
    if move_.makes_contact
        && (attacker.item == Item::ProtectivePads
            || (attacker.item == Item::PunchingGlove && move_.is_punch)
            || attacker.ability == Ability::LongReach)
    {
        return false;
    }
    if move_.name == "Shell Side Arm" && move_.category == Category::Physical {
        return true;
    }
    move_.makes_contact
}

#[derive(Debug, Clone, Copy)]
struct ModifiedStats {
    attack: u16,
    defense: u16,
    special_attack: u16,
    special_defense: u16,
    speed: u16,
}

impl ModifiedStats {
    fn from(pokemon: &Pokemon, raw: crate::types::StatTable) -> Result<Self, CalcError> {
        Ok(Self {
            attack: modified_stat(raw.attack, pokemon.boosts.attack)?,
            defense: modified_stat(raw.defense, pokemon.boosts.defense)?,
            special_attack: modified_stat(raw.special_attack, pokemon.boosts.special_attack)?,
            special_defense: modified_stat(raw.special_defense, pokemon.boosts.special_defense)?,
            speed: modified_stat(raw.speed, pokemon.boosts.speed)?,
        })
    }

    fn get(self, stat: Stat) -> u16 {
        match stat {
            Stat::Attack => self.attack,
            Stat::Defense => self.defense,
            Stat::SpecialAttack => self.special_attack,
            Stat::SpecialDefense => self.special_defense,
            Stat::Speed => self.speed,
            Stat::Hp => unreachable!("HP is never stage-modified"),
        }
    }
}

fn highest_stat(pokemon: &Pokemon, stats: ModifiedStats) -> Stat {
    if let Some(stat) = pokemon.highest_stat_override {
        return stat;
    }
    let ordered = [
        (Stat::Attack, stats.attack),
        (Stat::Defense, stats.defense),
        (Stat::SpecialAttack, stats.special_attack),
        (Stat::SpecialDefense, stats.special_defense),
        (Stat::Speed, stats.speed),
    ];
    let mut best = ordered[0];
    for candidate in ordered.into_iter().skip(1) {
        if candidate.1 > best.1 {
            best = candidate;
        }
    }
    best.0
}

fn preprocess_battle_state(
    attacker: &mut Pokemon,
    defender: &mut Pokemon,
    field: &mut Field,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> Result<(), CalcError> {
    check_trace(attacker, defender, modifiers);
    check_trace(defender, attacker, modifiers);
    check_neutralizing_gas(attacker, defender, field, modifiers);
    check_weather_setters(attacker, defender, field, modifiers);
    check_terrain_setters(attacker, defender, field, modifiers);
    check_screen_cleaner(attacker, defender, field, modifiers);

    preprocess_pokemon(attacker, field.weather, field.terrain);
    preprocess_pokemon(defender, field.weather, field.terrain);
    if matches!(attacker.ability, Ability::AirLock | Ability::CloudNine)
        || matches!(defender.ability, Ability::AirLock | Ability::CloudNine)
    {
        field.weather = Weather::None;
        modifiers.push(ModifierBreakdown::new("weather suppressed", 0));
    }

    check_klutz(attacker, modifiers);
    check_klutz(defender, modifiers);
    check_entry_boost_toggles(attacker, defender, field, modifiers);
    check_paradox_abilities(attacker, field.terrain, field.weather, modifiers);
    check_paradox_abilities(defender, field.terrain, field.weather, modifiers);
    check_seeds(attacker, field.terrain, modifiers);
    check_seeds(defender, field.terrain, modifiers);
    check_sword_shield(attacker, modifiers);
    check_sword_shield(defender, modifiers);
    check_wind_rider(attacker, field.attacker_tailwind, modifiers);
    check_wind_rider(defender, field.defender_tailwind, modifiers);
    check_intimidate(attacker, defender, modifiers);
    check_intimidate(defender, attacker, modifiers);
    check_supersweet_syrup(attacker, defender, modifiers);
    check_supersweet_syrup(defender, attacker, modifiers);
    check_download(attacker, defender, modifiers)?;
    check_download(defender, attacker, modifiers)?;
    check_embody_aspect(attacker, modifiers);
    check_embody_aspect(defender, modifiers);
    check_battle_bond(attacker, modifiers);
    check_battle_bond(defender, modifiers);

    Ok(())
}

fn check_entry_boost_toggles(
    attacker: &mut Pokemon,
    defender: &mut Pokemon,
    field: &Field,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    if field.attacker_evo_boost {
        apply_all_non_hp_boost(attacker, 2, 6);
        modifiers.push(ModifierBreakdown::new("attacker Evo/Tatsugiri boost", 0));
    }
    if field.defender_evo_boost {
        apply_all_non_hp_boost(defender, 2, 6);
        modifiers.push(ModifierBreakdown::new("defender Evo/Tatsugiri boost", 0));
    }
    if field.attacker_clangorous_soul {
        apply_special_clangorous_boost(attacker);
        modifiers.push(ModifierBreakdown::new("attacker Clangorous Soul", 0));
    }
    if field.defender_clangorous_soul {
        apply_special_clangorous_boost(defender);
        modifiers.push(ModifierBreakdown::new("defender Clangorous Soul", 0));
    }
    if field.attacker_weakness_policy {
        apply_weakness_policy_boost(attacker);
        modifiers.push(ModifierBreakdown::new("attacker Weakness Policy", 0));
    }
    if field.defender_weakness_policy {
        apply_weakness_policy_boost(defender);
        modifiers.push(ModifierBreakdown::new("defender Weakness Policy", 0));
    }
}

fn apply_all_non_hp_boost(pokemon: &mut Pokemon, stages: i8, cap: i8) {
    pokemon.boosts.attack = (pokemon.boosts.attack + stages).min(cap);
    pokemon.boosts.defense = (pokemon.boosts.defense + stages).min(cap);
    pokemon.boosts.special_attack = (pokemon.boosts.special_attack + stages).min(cap);
    pokemon.boosts.special_defense = (pokemon.boosts.special_defense + stages).min(cap);
    pokemon.boosts.speed = (pokemon.boosts.speed + stages).min(cap);
}

fn apply_special_clangorous_boost(pokemon: &mut Pokemon) {
    pokemon.boosts.special_attack = (pokemon.boosts.special_attack + 2).min(6);
    pokemon.boosts.special_defense = (pokemon.boosts.special_defense + 2).min(6);
    pokemon.boosts.speed = (pokemon.boosts.speed + 2).min(6);
}

fn apply_weakness_policy_boost(pokemon: &mut Pokemon) {
    pokemon.boosts.attack = (pokemon.boosts.attack + 2).min(6);
    pokemon.boosts.special_attack = (pokemon.boosts.special_attack + 2).min(6);
}

fn check_trace(source: &mut Pokemon, target: &Pokemon, modifiers: &mut Vec<ModifierBreakdown>) {
    if source.ability != Ability::Trace
        || !source.ability_on
        || source.item == Item::AbilityShield
        || cannot_trace(target.ability)
    {
        return;
    }
    source.ability = target.ability;
    modifiers.push(ModifierBreakdown::new("Trace", 0));
}

fn cannot_trace(ability: Ability) -> bool {
    matches!(
        ability,
        Ability::AsOne
            | Ability::BattleBond
            | Ability::Comatose
            | Ability::Disguise
            | Ability::FlowerGift
            | Ability::Forecast
            | Ability::Illusion
            | Ability::Imposter
            | Ability::Protosynthesis
            | Ability::QuarkDrive
            | Ability::Receiver
            | Ability::StanceChange
            | Ability::Trace
            | Ability::WonderGuard
            | Ability::ZeroToHero
    )
}

fn check_neutralizing_gas(
    p1: &mut Pokemon,
    p2: &mut Pokemon,
    field: &Field,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    if !(field.neutralizing_gas
        || p1.ability == Ability::NeutralizingGas
        || p2.ability == Ability::NeutralizingGas)
    {
        return;
    }
    if !cannot_suppress_ability(p1.ability) && p1.item != Item::AbilityShield {
        p1.ability = Ability::None;
    }
    if !cannot_suppress_ability(p2.ability) && p2.item != Item::AbilityShield {
        p2.ability = Ability::None;
    }
    modifiers.push(ModifierBreakdown::new("Neutralizing Gas", 0));
}

fn cannot_suppress_ability(ability: Ability) -> bool {
    matches!(
        ability,
        Ability::AsOne
            | Ability::BattleBond
            | Ability::Comatose
            | Ability::Disguise
            | Ability::StanceChange
            | Ability::ZeroToHero
    )
}

fn check_klutz(pokemon: &mut Pokemon, modifiers: &mut Vec<ModifierBreakdown>) {
    if pokemon.ability == Ability::Klutz && pokemon.item != Item::None {
        pokemon.item = Item::KlutzSuppressed;
        modifiers.push(ModifierBreakdown::new("Klutz", 0));
    }
}

fn check_weather_setters(
    attacker: &Pokemon,
    defender: &Pokemon,
    field: &mut Field,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    let weather = if attacker.ability == Ability::Drizzle || defender.ability == Ability::Drizzle {
        Some((Weather::Rain, "Drizzle"))
    } else if attacker.ability == Ability::Drought || defender.ability == Ability::Drought {
        Some((Weather::Sun, "Drought"))
    } else if attacker.ability == Ability::SandStream || defender.ability == Ability::SandStream {
        Some((Weather::Sand, "Sand Stream"))
    } else if attacker.ability == Ability::SnowWarning || defender.ability == Ability::SnowWarning {
        Some((Weather::Snow, "Snow Warning"))
    } else {
        None
    };

    if let Some((weather, label)) = weather {
        if field.weather == Weather::None {
            field.weather = weather;
            modifiers.push(ModifierBreakdown::new(label, 0));
        }
    }
}

fn check_terrain_setters(
    attacker: &Pokemon,
    defender: &Pokemon,
    field: &mut Field,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    if field.terrain == crate::types::Terrain::None
        && matches!(
            (attacker.ability, defender.ability),
            (Ability::ElectricSurge, _) | (_, Ability::ElectricSurge)
        )
    {
        field.terrain = crate::types::Terrain::Electric;
        modifiers.push(ModifierBreakdown::new("Electric Surge", 0));
    }
}

fn check_screen_cleaner(
    attacker: &Pokemon,
    defender: &Pokemon,
    field: &mut Field,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    if matches!(
        (attacker.ability, defender.ability),
        (Ability::ScreenCleaner, _) | (_, Ability::ScreenCleaner)
    ) {
        field.defender_side.reflect = false;
        field.defender_side.light_screen = false;
        field.defender_side.aurora_veil = false;
        modifiers.push(ModifierBreakdown::new("Screen Cleaner", 0));
    }
}

fn check_paradox_abilities(
    pokemon: &mut Pokemon,
    terrain: crate::types::Terrain,
    weather: Weather,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    if !matches!(
        pokemon.ability,
        Ability::Protosynthesis | Ability::QuarkDrive
    ) {
        return;
    }
    let field_active = (pokemon.ability == Ability::Protosynthesis && weather == Weather::Sun)
        || (pokemon.ability == Ability::QuarkDrive && terrain == crate::types::Terrain::Electric);
    if field_active {
        pokemon.paradox_ability_boost = true;
        modifiers.push(ModifierBreakdown::new("Paradox ability", 0));
    } else if pokemon.item == Item::BoosterEnergy {
        pokemon.paradox_ability_boost = true;
        pokemon.item = Item::None;
        modifiers.push(ModifierBreakdown::new("Booster Energy", 0));
    }
}

fn check_seeds(
    pokemon: &mut Pokemon,
    terrain: crate::types::Terrain,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    let triggered = matches!(
        (pokemon.item, terrain),
        (Item::ElectricSeed, crate::types::Terrain::Electric)
            | (Item::GrassySeed, crate::types::Terrain::Grassy)
            | (Item::MistySeed, crate::types::Terrain::Misty)
            | (Item::PsychicSeed, crate::types::Terrain::Psychic)
    );
    if !triggered {
        return;
    }
    if matches!(
        terrain,
        crate::types::Terrain::Electric | crate::types::Terrain::Grassy
    ) {
        boost_stat(pokemon, Stat::Defense, 1);
    } else {
        boost_stat(pokemon, Stat::SpecialDefense, 1);
    }
    pokemon.item = Item::None;
    modifiers.push(ModifierBreakdown::new("terrain seed", 0));
}

fn check_sword_shield(pokemon: &mut Pokemon, modifiers: &mut Vec<ModifierBreakdown>) {
    if pokemon.ability == Ability::IntrepidSword {
        boost_stat(pokemon, Stat::Attack, 1);
        modifiers.push(ModifierBreakdown::new("Intrepid Sword", 0));
    } else if pokemon.ability == Ability::DauntlessShield {
        boost_stat(pokemon, Stat::Defense, 1);
        modifiers.push(ModifierBreakdown::new("Dauntless Shield", 0));
    }
}

fn check_wind_rider(pokemon: &mut Pokemon, tailwind: bool, modifiers: &mut Vec<ModifierBreakdown>) {
    if pokemon.ability == Ability::WindRider && tailwind {
        boost_stat(pokemon, Stat::Attack, 1);
        modifiers.push(ModifierBreakdown::new("Wind Rider", 0));
    }
}

fn check_intimidate(
    source: &mut Pokemon,
    target: &mut Pokemon,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    if source.ability != Ability::Intimidate || !source.ability_on {
        return;
    }
    if matches!(target.ability, Ability::Contrary | Ability::GuardDog) {
        boost_stat(target, Stat::Attack, 1);
    } else if matches!(
        target.ability,
        Ability::ClearBody
            | Ability::WhiteSmoke
            | Ability::HyperCutter
            | Ability::FullMetalBody
            | Ability::InnerFocus
            | Ability::Oblivious
            | Ability::OwnTempo
            | Ability::Scrappy
    ) || target.item == Item::ClearAmulet
    {
        modifiers.push(ModifierBreakdown::new("Intimidate blocked", 0));
        return;
    } else if target.ability == Ability::MirrorArmor {
        boost_stat(source, Stat::Attack, -1);
    } else {
        let stages = if target.ability == Ability::Simple {
            -2
        } else {
            -1
        };
        boost_stat(target, Stat::Attack, stages);
        if target.ability == Ability::Defiant {
            boost_stat(target, Stat::Attack, 2);
        } else if target.ability == Ability::Competitive {
            boost_stat(target, Stat::SpecialAttack, 2);
        }
    }
    if target.item == Item::AdrenalineOrb && target.ability != Ability::MirrorArmor {
        let stages = if target.ability == Ability::Simple {
            2
        } else {
            1
        };
        boost_stat(target, Stat::Speed, stages);
        target.item = Item::None;
    }
    if target.ability == Ability::Rattled && target.item != Item::ClearAmulet {
        boost_stat(target, Stat::Speed, 1);
    }
    modifiers.push(ModifierBreakdown::new("Intimidate", 0));
}

fn check_supersweet_syrup(
    source: &Pokemon,
    target: &mut Pokemon,
    modifiers: &mut Vec<ModifierBreakdown>,
) {
    if source.ability == Ability::SupersweetSyrup
        && source.ability_on
        && target.item != Item::ClearAmulet
    {
        if target.ability == Ability::Defiant {
            boost_stat(target, Stat::Attack, 2);
        } else if target.ability == Ability::Competitive {
            target.boosts.attack = (target.boosts.special_attack + 2).min(6);
        }
        modifiers.push(ModifierBreakdown::new("Supersweet Syrup", 0));
    }
}

fn check_download(
    source: &mut Pokemon,
    target: &Pokemon,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> Result<(), CalcError> {
    if source.ability != Ability::Download {
        return Ok(());
    }
    let target_stats = calculate_stats(target, Ruleset::Champions)?;
    let defense = modified_stat(target_stats.defense, target.boosts.defense)?;
    let special_defense =
        modified_stat(target_stats.special_defense, target.boosts.special_defense)?;
    if special_defense <= defense {
        boost_stat(source, Stat::SpecialAttack, 1);
    } else {
        boost_stat(source, Stat::Attack, 1);
    }
    modifiers.push(ModifierBreakdown::new("Download", 0));
    Ok(())
}

fn check_embody_aspect(pokemon: &mut Pokemon, modifiers: &mut Vec<ModifierBreakdown>) {
    if pokemon.ability != Ability::EmbodyAspect {
        return;
    }
    match (pokemon.name.as_str(), pokemon.item) {
        ("Ogerpon", _) => boost_stat(pokemon, Stat::Speed, 1),
        ("Ogerpon-Wellspring", Item::WellspringMask) => {
            boost_stat(pokemon, Stat::SpecialDefense, 1)
        }
        ("Ogerpon-Hearthflame", Item::HearthflameMask) => boost_stat(pokemon, Stat::Attack, 1),
        ("Ogerpon-Cornerstone", Item::CornerstoneMask) => boost_stat(pokemon, Stat::Defense, 1),
        _ => return,
    }
    modifiers.push(ModifierBreakdown::new("Embody Aspect", 0));
}

fn check_battle_bond(pokemon: &mut Pokemon, modifiers: &mut Vec<ModifierBreakdown>) {
    let _ = (pokemon, modifiers);
}

fn boost_stat(pokemon: &mut Pokemon, stat: Stat, stages: i8) {
    let clamp = |value: i8| value.clamp(-6, 6);
    match stat {
        Stat::Attack => pokemon.boosts.attack = clamp(pokemon.boosts.attack + stages),
        Stat::Defense => pokemon.boosts.defense = clamp(pokemon.boosts.defense + stages),
        Stat::SpecialAttack => {
            pokemon.boosts.special_attack = clamp(pokemon.boosts.special_attack + stages)
        }
        Stat::SpecialDefense => {
            pokemon.boosts.special_defense = clamp(pokemon.boosts.special_defense + stages)
        }
        Stat::Speed => pokemon.boosts.speed = clamp(pokemon.boosts.speed + stages),
        Stat::Hp => {}
    }
}

fn preprocess_pokemon(pokemon: &mut Pokemon, weather: Weather, terrain: crate::types::Terrain) {
    if pokemon.ability == Ability::Forecast && pokemon.name == "Castform" {
        pokemon.types = [
            Some(if weather.is_sun() {
                PokemonType::Fire
            } else if weather.is_rain() {
                PokemonType::Water
            } else if matches!(weather, Weather::Hail | Weather::Snow) {
                PokemonType::Ice
            } else {
                PokemonType::Normal
            }),
            None,
        ];
    }
    if pokemon.ability == Ability::Mimicry && terrain != crate::types::Terrain::None {
        pokemon.types = [
            Some(match terrain {
                crate::types::Terrain::Electric => PokemonType::Electric,
                crate::types::Terrain::Grassy => PokemonType::Grass,
                crate::types::Terrain::Misty => PokemonType::Fairy,
                crate::types::Terrain::Psychic => PokemonType::Psychic,
                crate::types::Terrain::None => PokemonType::Normal,
            }),
            None,
        ];
    }
}

fn apply_move_type_changes(move_: &mut Move, attacker: &Pokemon, field: &Field) {
    match move_.name.as_str() {
        "Struggle" => {
            move_.type_ = PokemonType::Typeless;
        }
        "Weather Ball" => {
            move_.type_ = if (field.weather.is_sun() && attacker.item != Item::UtilityUmbrella)
                || attacker.ability == Ability::MegaSol
            {
                PokemonType::Fire
            } else if field.weather.is_rain() && attacker.item != Item::UtilityUmbrella {
                PokemonType::Water
            } else if field.weather == Weather::Sand {
                PokemonType::Rock
            } else if matches!(field.weather, Weather::Hail | Weather::Snow) {
                PokemonType::Ice
            } else {
                PokemonType::Normal
            };
        }
        "Natural Gift" => {
            if let Some((type_, _power)) = natural_gift(attacker.item) {
                move_.type_ = type_;
            }
        }
        "Techno Blast" => {
            move_.type_ = drive_type(attacker.item).unwrap_or(PokemonType::Normal);
        }
        "Multi-Attack" => {
            if let Some(type_) = memory_type(attacker.item) {
                move_.type_ = type_;
            }
        }
        "Judgment" => {
            if let Some(type_) = item_boost_type(attacker.item) {
                move_.type_ = type_;
            }
        }
        "Revelation Dance" => {
            move_.type_ = attacker.types[0]
                .or(attacker.types[1])
                .unwrap_or(PokemonType::Typeless);
        }
        "Aura Wheel" if attacker.name == "Morpeko-Hangry" => {
            move_.type_ = PokemonType::Dark;
        }
        "Raging Bull" => {
            move_.type_ = match attacker.name.as_str() {
                "Tauros-Paldea-Combat" => PokemonType::Fighting,
                "Tauros-Paldea-Blaze" => PokemonType::Fire,
                "Tauros-Paldea-Aqua" => PokemonType::Water,
                _ => PokemonType::Normal,
            };
        }
        "Ivy Cudgel" => {
            move_.type_ = match attacker.name.as_str() {
                "Ogerpon-Wellspring" => PokemonType::Water,
                "Ogerpon-Hearthflame" => PokemonType::Fire,
                "Ogerpon-Cornerstone" => PokemonType::Rock,
                _ => PokemonType::Grass,
            };
        }
        "Terrain Pulse" => {
            move_.type_ = match field.terrain {
                crate::types::Terrain::Electric => PokemonType::Electric,
                crate::types::Terrain::Grassy => PokemonType::Grass,
                crate::types::Terrain::Misty => PokemonType::Fairy,
                crate::types::Terrain::Psychic => PokemonType::Psychic,
                crate::types::Terrain::None => PokemonType::Normal,
            };
        }
        "Tera Blast" if attacker.is_terastalized => {
            if let Some(tera_type) = attacker.tera_type {
                move_.type_ = tera_type;
            }
        }
        "Tera Starstorm" if attacker.name == "Terapagos-Stellar" => {
            move_.type_ = PokemonType::Stellar;
            move_.is_spread = true;
        }
        _ => {}
    }
}

fn apply_move_metadata_defaults(move_: &mut Move) {
    if is_champions_spread_move(&move_.name) {
        move_.is_spread = true;
    }
}

fn apply_hit_count_defaults(move_: &mut Move, attacker_ability: Ability) {
    if attacker_ability == Ability::SkillLink && move_.hits <= 1 {
        if move_.name == "Population Bomb" {
            move_.hits = 10;
        } else if is_skill_link_move(&move_.name) {
            move_.hits = 5;
        }
    }
}

fn apply_priority_defaults(
    move_: &mut Move,
    attacker: &Pokemon,
    attacker_current_hp: u16,
    attacker_max_hp: u16,
    field: &Field,
) {
    if attacker.ability == Ability::GaleWings
        && move_.type_ == PokemonType::Flying
        && attacker_current_hp == attacker_max_hp
    {
        move_.is_priority = true;
    }
    if move_.name == "Grassy Glide"
        && field.terrain == crate::types::Terrain::Grassy
        && is_grounded(attacker, field)
    {
        move_.is_priority = true;
    }
}

fn is_skill_link_move(name: &str) -> bool {
    matches!(
        name,
        "Arm Thrust"
            | "Bone Rush"
            | "Bullet Seed"
            | "Double Slap"
            | "Fury Attack"
            | "Fury Swipes"
            | "Icicle Spear"
            | "Pin Missile"
            | "Rock Blast"
            | "Tail Slap"
    )
}

fn is_champions_spread_move(name: &str) -> bool {
    matches!(
        name,
        "Surf"
            | "Earthquake"
            | "Self-Destruct"
            | "Explosion"
            | "Discharge"
            | "Lava Plume"
            | "Sludge Wave"
            | "Bulldoze"
            | "Parabolic Charge"
            | "Petal Blizzard"
            | "Boomburst"
            | "Sparkling Aria"
            | "Brutal Swing"
            | "Misty Explosion"
            | "Blizzard"
            | "Rock Slide"
            | "Icy Wind"
            | "Heat Wave"
            | "Eruption"
            | "Hyper Voice"
            | "Air Cutter"
            | "Water Spout"
            | "Muddy Water"
            | "Struggle Bug"
            | "Electroweb"
            | "Snarl"
            | "Dazzling Gleam"
            | "Clanging Scales"
            | "Breaking Swipe"
            | "Burning Jealousy"
            | "Mortal Spin"
            | "Matcha Gotcha"
    )
}

fn apply_ability_type_change(
    move_: &mut Move,
    attacker: &Pokemon,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> bool {
    if matches!(
        move_.name.as_str(),
        "Hidden Power"
            | "Weather Ball"
            | "Natural Gift"
            | "Judgment"
            | "Techno Blast"
            | "Revelation Dance"
            | "Multi-Attack"
            | "Terrain Pulse"
    ) {
        return false;
    }
    if attacker.ability == Ability::LiquidVoice && move_.is_sound {
        move_.type_ = PokemonType::Water;
        modifiers.push(ModifierBreakdown::new("Liquid Voice type change", 0));
        return false;
    }
    if attacker.ability == Ability::Normalize {
        move_.type_ = PokemonType::Normal;
        modifiers.push(ModifierBreakdown::new("Normalize type change", 0));
        return true;
    }
    if move_.type_ != PokemonType::Normal {
        return false;
    }
    let new_type = match attacker.ability {
        Ability::Aerilate => PokemonType::Flying,
        Ability::Pixilate => PokemonType::Fairy,
        Ability::Refrigerate => PokemonType::Ice,
        Ability::Galvanize => PokemonType::Electric,
        Ability::Dragonize => PokemonType::Dragon,
        _ => return false,
    };
    move_.type_ = new_type;
    modifiers.push(ModifierBreakdown::new("type-changing ability", 0));
    true
}

fn ability_after_ignore(
    attacker_ability: Ability,
    defender_ability: Ability,
    move_: &Move,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> Ability {
    if attacker_ability == Ability::MoldBreaker
        && !matches!(
            defender_ability,
            Ability::PrismArmor
                | Ability::BeadsOfRuin
                | Ability::ShadowShield
                | Ability::SwordOfRuin
                | Ability::TabletsOfRuin
                | Ability::TeraShell
                | Ability::VesselOfRuin
        )
    {
        modifiers.push(ModifierBreakdown::new("Mold Breaker ignores ability", 0));
        Ability::None
    } else if matches!(
        move_.name.as_str(),
        "Moongeist Beam" | "Sunsteel Strike" | "Photon Geyser"
    ) {
        Ability::None
    } else {
        defender_ability
    }
}

fn is_immune(
    move_: &Move,
    attacker: &Pokemon,
    defender: &Pokemon,
    def_ability: Ability,
    field: &Field,
    type_effectiveness: f32,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> bool {
    if type_effectiveness == 0.0 {
        return true;
    }
    if def_ability == Ability::WonderGuard
        && type_effectiveness <= 1.0
        && move_.type_ != PokemonType::Typeless
    {
        modifiers.push(ModifierBreakdown::new("Wonder Guard", 0));
        return true;
    }
    if (move_.type_ == PokemonType::Grass && def_ability == Ability::SapSipper)
        || (move_.type_ == PokemonType::Fire && matches!(def_ability, Ability::FlashFire))
        || (move_.type_ == PokemonType::Water
            && matches!(
                def_ability,
                Ability::DrySkin
                    | Ability::StormDrain
                    | Ability::WaterAbsorb
                    | Ability::WaterBubble
            ))
        || (move_.type_ == PokemonType::Electric
            && matches!(
                def_ability,
                Ability::LightningRod | Ability::MotorDrive | Ability::VoltAbsorb
            ))
        || (move_.is_bullet && def_ability == Ability::Bulletproof)
        || (move_.is_sound && def_ability == Ability::Soundproof)
        || (move_.is_wind && def_ability == Ability::WindRider)
    {
        modifiers.push(ModifierBreakdown::new("defensive immunity ability", 0));
        return true;
    }
    if move_.type_ == PokemonType::Ground
        && !field.gravity
        && defender.item != Item::IronBall
        && defender.item != Item::AirBalloon
        && matches!(def_ability, Ability::Levitate | Ability::EarthEater)
        && attacker.ability != Ability::MoldBreaker
    {
        modifiers.push(ModifierBreakdown::new("Levitate", 0));
        return true;
    }
    if move_.type_ == PokemonType::Ground
        && !field.gravity
        && defender.item == Item::AirBalloon
        && move_.name != "Thousand Arrows"
    {
        modifiers.push(ModifierBreakdown::new("Air Balloon", 0));
        return true;
    }
    if matches!(
        def_ability,
        Ability::QueenlyMajesty | Ability::Dazzling | Ability::ArmorTail
    ) && move_.is_priority
    {
        modifiers.push(ModifierBreakdown::new("priority-blocking ability", 0));
        return true;
    }
    if field.terrain == crate::types::Terrain::Psychic
        && move_.is_priority
        && is_grounded(defender, field)
    {
        modifiers.push(ModifierBreakdown::new("Psychic Terrain priority block", 0));
        return true;
    }
    if move_.name == "Sky Drop"
        && (defender.has_type(PokemonType::Flying) || defender.weight_kg >= 200.0 || field.gravity)
    {
        modifiers.push(ModifierBreakdown::new("Sky Drop failed", 0));
        return true;
    }
    if move_.name == "Synchronoise" && !shares_any_type(attacker, defender) {
        modifiers.push(ModifierBreakdown::new("Synchronoise failed", 0));
        return true;
    }
    if matches!(
        move_.name.as_str(),
        "Self-Destruct" | "Explosion" | "Mind Blown" | "Misty Explosion"
    ) && (def_ability == Ability::Damp || attacker.ability == Ability::Damp)
    {
        modifiers.push(ModifierBreakdown::new("Damp", 0));
        return true;
    }
    if move_.is_ohko && def_ability == Ability::Sturdy {
        modifiers.push(ModifierBreakdown::new("Sturdy", 0));
        return true;
    }
    if move_.name == "Dream Eater"
        && defender.status != StatusCondition::Asleep
        && def_ability != Ability::Comatose
    {
        modifiers.push(ModifierBreakdown::new("Dream Eater failed", 0));
        return true;
    }
    defender.name.is_empty()
}

fn shares_any_type(attacker: &Pokemon, defender: &Pokemon) -> bool {
    attacker
        .types
        .iter()
        .flatten()
        .any(|attacker_type| defender.has_type(*attacker_type))
}

fn zero_damage(
    defender_max_hp: u16,
    modifiers: Vec<ModifierBreakdown>,
    debug: Vec<String>,
) -> DamageResult {
    let _ = defender_max_hp;
    DamageResult {
        min_damage: 0,
        max_damage: 0,
        damage_rolls: vec![0],
        hit_rolls: vec![vec![0]],
        percent_range: (0.0, 0.0),
        ko_chance_by_move_use: vec![0.0; KO_CHANCE_MAX_USES],
        ko_chance: Some(0.0),
        outcome: DamageOutcome::ImmuneOrFailed,
        resolved_move: None,
        defender_hp_delta: None,
        applied_modifiers: modifiers,
        debug,
    }
}

#[allow(clippy::too_many_arguments)]
fn set_damage_result(
    move_: &Move,
    attacker: &Pokemon,
    defender: &Pokemon,
    attacker_current_hp: u16,
    defender_current_hp: u16,
    defender_max_hp: u16,
    protect: bool,
    field: &Field,
    modifiers: Vec<ModifierBreakdown>,
    debug: Vec<String>,
) -> Option<DamageResult> {
    let context = DamageResolutionContext {
        defender_max_hp,
        defender_current_hp,
        defender_item: defender.item,
        residual: residual_effects(
            defender,
            field,
            defender_max_hp,
            move_.name == "Psychic Noise",
        ),
        healing_suppressed: healing_item_suppressed(move_, attacker),
    };

    if let Some(result) = counter_damage_result(move_, context, modifiers.clone(), debug.clone()) {
        return Some(result);
    }
    let damage = match move_.name.as_str() {
        "Super Fang" | "Nature's Madness" | "Ruination" => defender_current_hp / 2,
        "Guardian of Alola" => {
            if protect {
                defender_current_hp * 3 / 16
            } else {
                defender_current_hp * 3 / 4
            }
        }
        "Endeavor" => defender_current_hp.saturating_sub(attacker_current_hp),
        "Final Gambit" => attacker_current_hp,
        "Sonic Boom" => 20,
        "Dragon Rage" => 40,
        "Seismic Toss" | "Night Shade" => attacker.level as u16,
        _ if move_.is_ohko => {
            if move_.name == "Sheer Cold" && defender.has_type(PokemonType::Ice) {
                0
            } else {
                defender_current_hp
            }
        }
        _ => return None,
    };
    Some(single_damage_result(damage, context, modifiers, debug))
}

fn counter_damage_result(
    move_: &Move,
    context: DamageResolutionContext,
    mut modifiers: Vec<ModifierBreakdown>,
    debug: Vec<String>,
) -> Option<DamageResult> {
    let rolls = move_.countered_damage_rolls.as_ref()?;
    let category = move_.countered_move_category?;
    let multiplier = match move_.name.as_str() {
        "Counter" if category == Category::Physical => 2.0,
        "Mirror Coat" if category == Category::Special => 2.0,
        "Metal Burst" | "Comeuppance" if category != Category::Status => 1.5,
        "Counter" | "Mirror Coat" | "Metal Burst" | "Comeuppance" => 0.0,
        _ => return None,
    };
    if multiplier == 0.0 {
        return Some(single_damage_result(0, context, modifiers, debug));
    }
    let mut damage_rolls = rolls
        .iter()
        .map(|roll| {
            if multiplier == 2.0 {
                roll.saturating_mul(2)
            } else {
                ((*roll as u32 * 3) / 2).min(u16::MAX as u32) as u16
            }
        })
        .collect::<Vec<_>>();
    damage_rolls.sort_unstable();
    let min_damage = *damage_rolls.first().unwrap_or(&0);
    let max_damage = *damage_rolls.last().unwrap_or(&0);
    let percent_range = (
        min_damage as f32 * 100.0 / context.defender_max_hp as f32,
        max_damage as f32 * 100.0 / context.defender_max_hp as f32,
    );
    let hit_rolls = vec![damage_rolls.clone()];
    let ko_chance_by_move_use = ko_chances_after_move_uses(
        &hit_rolls,
        context.defender_current_hp,
        context.defender_max_hp,
        context.defender_item,
        context.residual,
        context.healing_suppressed,
        KO_CHANCE_MAX_USES,
    );
    let ko_chance = ko_chance_by_move_use.first().copied();
    modifiers.push(ModifierBreakdown::new("counter-style damage", 0));
    Some(DamageResult {
        min_damage,
        max_damage,
        hit_rolls,
        damage_rolls,
        percent_range,
        ko_chance_by_move_use,
        ko_chance,
        outcome: DamageOutcome::Counter,
        resolved_move: None,
        defender_hp_delta: None,
        applied_modifiers: modifiers,
        debug,
    })
}

fn single_damage_result(
    damage: u16,
    context: DamageResolutionContext,
    modifiers: Vec<ModifierBreakdown>,
    debug: Vec<String>,
) -> DamageResult {
    let percent = damage as f32 * 100.0 / context.defender_max_hp as f32;
    let hit_rolls = vec![vec![damage]];
    let ko_chance_by_move_use = ko_chances_after_move_uses(
        &hit_rolls,
        context.defender_current_hp,
        context.defender_max_hp,
        context.defender_item,
        context.residual,
        context.healing_suppressed,
        KO_CHANCE_MAX_USES,
    );
    let ko_chance = ko_chance_by_move_use.first().copied();
    DamageResult {
        min_damage: damage,
        max_damage: damage,
        damage_rolls: vec![damage],
        hit_rolls,
        percent_range: (percent, percent),
        ko_chance_by_move_use,
        ko_chance,
        outcome: DamageOutcome::Damage,
        resolved_move: None,
        defender_hp_delta: None,
        applied_modifiers: modifiers,
        debug,
    }
}

fn resolved_move(move_: &Move, base_power: u16, is_critical: bool) -> ResolvedMove {
    ResolvedMove {
        name: move_.name.clone(),
        type_: move_.type_,
        category: move_.category,
        base_power,
        hits: move_.hits.max(1),
        is_critical,
        is_spread: move_.is_spread,
        makes_contact: move_.makes_contact,
        is_priority: move_.is_priority,
    }
}

fn can_tera_boost_to_sixty(move_: &Move, attacker: &Pokemon) -> bool {
    let tera_type_matches = attacker.tera_type.is_some_and(|tera_type| {
        move_.type_ == tera_type || (tera_type == PokemonType::Stellar && move_.gets_stellar_boost)
    });
    if !attacker.is_terastalized
        || !tera_type_matches
        || move_.is_priority
        || move_.is_multi_hit
        || move_.hits > 1
    {
        return false;
    }
    !matches!(
        move_.name.as_str(),
        "Crush Grip"
            | "Dragon Energy"
            | "Electro Ball"
            | "Eruption"
            | "Flail"
            | "Fling"
            | "Grass Knot"
            | "Gyro Ball"
            | "Heat Crash"
            | "Heavy Slam"
            | "Low Kick"
            | "Reversal"
            | "Water Spout"
            | "Hard Press"
    )
}

fn calc_defense_mods(
    _move_: &Move,
    defender: &Pokemon,
    defender_highest_stat: Stat,
    def_ability: Ability,
    field: &Field,
    hits_physical: bool,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> Vec<i32> {
    let mut mods = Vec::new();
    if field.sword_of_ruin && hits_physical && def_ability != Ability::SwordOfRuin {
        push_mod(&mut mods, modifiers, "Sword of Ruin", MOD_THREE_QUARTERS);
    } else if field.beads_of_ruin && !hits_physical && def_ability != Ability::BeadsOfRuin {
        push_mod(&mut mods, modifiers, "Beads of Ruin", MOD_THREE_QUARTERS);
    }
    if ((def_ability == Ability::FlowerGift && field.weather.is_sun())
        || field.flower_gift_special_defense)
        && !hits_physical
        && defender.item != Item::UtilityUmbrella
    {
        push_mod(&mut mods, modifiers, "Flower Gift SpD", MOD_1_5);
    }
    if (def_ability == Ability::MarvelScale
        && defender.status != StatusCondition::Healthy
        && hits_physical)
        || (def_ability == Ability::GrassPelt
            && field.terrain == crate::types::Terrain::Grassy
            && hits_physical)
    {
        push_mod(&mut mods, modifiers, "defense ability 1.5", MOD_1_5);
    } else if defender.paradox_ability_boost
        && ((defender_highest_stat == Stat::Defense && hits_physical)
            || (defender_highest_stat == Stat::SpecialDefense && !hits_physical))
    {
        push_mod(&mut mods, modifiers, "Paradox ability defense", MOD_1_3);
    } else if def_ability == Ability::FurCoat && hits_physical {
        push_mod(&mut mods, modifiers, "defense ability 2.0", MOD_DOUBLE);
    }
    if defender.item == Item::AssaultVest && !hits_physical {
        push_mod(&mut mods, modifiers, "Assault Vest", MOD_1_5);
    } else if defender.item == Item::Eviolite && defender.can_evolve {
        push_mod(&mut mods, modifiers, "Eviolite", MOD_1_5);
    }
    for &modifier in &defender.custom_defense_mods {
        push_mod(&mut mods, modifiers, "custom defense modifier", modifier);
    }
    mods
}

fn stab_modifier(
    attacker: &Pokemon,
    original_types: [Option<PokemonType>; 2],
    move_: &Move,
    modifiers: &mut Vec<ModifierBreakdown>,
) -> i32 {
    if move_.type_ == PokemonType::Typeless {
        return MODIFIER_DENOMINATOR;
    }
    let original_has_type = original_types
        .into_iter()
        .flatten()
        .any(|t| t == move_.type_);
    if attacker.is_terastalized && attacker.tera_type != Some(PokemonType::Stellar) {
        let tera_type = attacker.tera_type.unwrap_or(move_.type_);
        if move_.type_ == tera_type && original_has_type {
            if attacker.ability == Ability::Adaptability {
                modifiers.push(ModifierBreakdown::new("Adaptability Tera STAB", 0x2400));
                return 0x2400;
            }
            modifiers.push(ModifierBreakdown::new("Tera boosted STAB", MOD_DOUBLE));
            return MOD_DOUBLE;
        }
        if (move_.type_ != tera_type && original_has_type) || move_.type_ == tera_type {
            if attacker.ability == Ability::Adaptability && move_.type_ == tera_type {
                modifiers.push(ModifierBreakdown::new("Adaptability Tera STAB", MOD_DOUBLE));
                return MOD_DOUBLE;
            }
            modifiers.push(ModifierBreakdown::new("Tera STAB", MOD_1_5));
            return MOD_1_5;
        }
    } else if attacker.is_terastalized
        && attacker.tera_type == Some(PokemonType::Stellar)
        && move_.gets_stellar_boost
    {
        if original_has_type {
            modifiers.push(ModifierBreakdown::new(
                "Stellar original-type STAB",
                MOD_DOUBLE,
            ));
            return MOD_DOUBLE;
        }
        modifiers.push(ModifierBreakdown::new("Stellar boost", MOD_1_2));
        return MOD_1_2;
    }

    let current_has_type = attacker.has_type(move_.type_);
    if current_has_type {
        if attacker.ability == Ability::Adaptability {
            modifiers.push(ModifierBreakdown::new("Adaptability STAB", MOD_DOUBLE));
            MOD_DOUBLE
        } else {
            modifiers.push(ModifierBreakdown::new("STAB", MOD_1_5));
            MOD_1_5
        }
    } else if attacker.ability == Ability::Protean && attacker.ability_on {
        modifiers.push(ModifierBreakdown::new("Protean", MOD_1_5));
        MOD_1_5
    } else {
        MODIFIER_DENOMINATOR
    }
}

fn weather_damage_boost(
    move_: &Move,
    attacker_ability: Ability,
    weather: Weather,
    attacker_item: Item,
    defender_item: Item,
) -> bool {
    (((weather.is_sun() || attacker_ability == Ability::MegaSol)
        && move_.type_ == PokemonType::Fire)
        || (weather.is_rain() && move_.type_ == PokemonType::Water))
        && defender_item != Item::UtilityUmbrella
        || ((weather.is_sun() || attacker_ability == Ability::MegaSol)
            && move_.name == "Hydro Steam"
            && attacker_item != Item::UtilityUmbrella)
}

fn weather_damage_drop(
    move_: &Move,
    attacker_ability: Ability,
    weather: Weather,
    defender_item: Item,
) -> bool {
    ((weather == Weather::Sun && move_.type_ == PokemonType::Water)
        || (weather == Weather::Rain
            && move_.type_ == PokemonType::Fire
            && attacker_ability != Ability::MegaSol))
        && defender_item != Item::UtilityUmbrella
}

pub(super) fn push_mod(
    mods: &mut Vec<i32>,
    breakdown: &mut Vec<ModifierBreakdown>,
    label: &'static str,
    modifier: i32,
) {
    mods.push(modifier);
    breakdown.push(ModifierBreakdown::new(label, modifier));
}

pub(super) fn is_half_hp(pokemon: &Pokemon) -> bool {
    match (pokemon.current_hp, pokemon.max_hp_override) {
        (Some(cur), Some(max)) => cur <= max / 2,
        _ => false,
    }
}

pub(super) fn is_third_hp(pokemon: &Pokemon) -> bool {
    match (pokemon.current_hp, pokemon.max_hp_override) {
        (Some(cur), Some(max)) => cur <= max / 3,
        _ => false,
    }
}

pub(super) fn count_positive_boosts(boosts: crate::types::Boosts) -> u16 {
    [
        boosts.attack,
        boosts.defense,
        boosts.special_attack,
        boosts.special_defense,
        boosts.speed,
    ]
    .into_iter()
    .filter(|boost| *boost > 0)
    .map(|boost| boost as u16)
    .sum()
}

pub(super) fn effective_weight(pokemon: &Pokemon) -> f32 {
    let mut weight = pokemon.weight_kg;
    if pokemon.ability == Ability::HeavyMetal {
        weight *= 2.0;
    } else if pokemon.ability == Ability::LightMetal {
        weight /= 2.0;
    }
    if pokemon.item == Item::FloatStone {
        weight /= 2.0;
    }
    weight.max(0.1)
}

fn final_speed(
    pokemon: &Pokemon,
    staged_speed: u16,
    tailwind: bool,
    swamp: bool,
    highest_stat: Stat,
    field: &Field,
) -> u16 {
    let mut speed = staged_speed as i32;
    let mut speed_mods = Vec::new();
    if pokemon.item == Item::ChoiceScarf {
        speed_mods.push(MOD_1_5);
    } else if pokemon.item == Item::IronBall {
        speed_mods.push(MOD_HALF);
    }
    if tailwind {
        speed_mods.push(MOD_DOUBLE);
    }
    if swamp {
        speed_mods.push(0x0400);
    }
    if pokemon.paradox_ability_boost && highest_stat == Stat::Speed {
        speed_mods.push(MOD_1_5);
    }
    if speed_ability_active(pokemon, field) {
        speed_mods.push(MOD_DOUBLE);
    }
    if pokemon.ability == Ability::Unburden && pokemon.ability_on {
        speed_mods.push(MOD_DOUBLE);
    }
    if pokemon.ability == Ability::QuickFeet && pokemon.status != StatusCondition::Healthy {
        speed_mods.push(MOD_1_5);
    }
    if !speed_mods.is_empty() {
        speed = apply_mod(speed, chain_mods(&speed_mods));
    }
    if pokemon.status == StatusCondition::Paralyzed && pokemon.ability != Ability::QuickFeet {
        speed /= 2;
    }
    if speed > 65535 {
        speed %= 65536;
    }
    speed.clamp(0, 10000) as u16
}

fn speed_ability_active(pokemon: &Pokemon, field: &Field) -> bool {
    match pokemon.ability {
        Ability::SwiftSwim => field.weather.is_rain(),
        Ability::Chlorophyll => field.weather.is_sun(),
        Ability::SandRush => field.weather == Weather::Sand,
        Ability::SlushRush => matches!(field.weather, Weather::Hail | Weather::Snow),
        Ability::SurgeSurfer => field.terrain == crate::types::Terrain::Electric,
        _ => false,
    }
}

pub(super) fn cant_remove_item(item: Item, species: &str) -> bool {
    if matches!(item, Item::None | Item::KlutzSuppressed) {
        return true;
    }
    if locked_item_for_species(species) == Some(item) {
        return true;
    }
    if species == "Arceus" && item_boost_type(item).is_some() {
        return true;
    }
    false
}

pub(super) fn is_grounded(pokemon: &Pokemon, field: &Field) -> bool {
    field.gravity
        || pokemon.item == Item::IronBall
        || (pokemon.ability != Ability::Levitate
            && pokemon.item != Item::AirBalloon
            && !pokemon.has_type(PokemonType::Flying))
}
