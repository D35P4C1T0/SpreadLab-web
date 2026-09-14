//! A behavior-first Rust port of the Nimbasa City Post Pokemon Champions
//! damage calculator.
//!
//! The JavaScript calculator under `reference/NCP-VGC-Damage-Calculator` is
//! treated as the source of truth. This crate currently implements the
//! Champions stat model and the modern Scarlet/Violet/Champions damage pipeline
//! for the common mechanics covered by the regression fixtures.

pub mod damage;
pub mod data;
pub mod mechanics;
pub mod optimizer;
pub mod stats;
pub mod types;

pub use damage::{
    calculate_all_moves, calculate_damage, AttackerHpEffects, BatchCalcInput, BatchDamageResult,
    CalcInput, DamageOutcome, DamageResult, ModifierBreakdown, ResolvedMove,
};
pub use stats::{calculate_hp, calculate_non_hp_stat, calculate_stats};
pub use types::{
    Ability, Boosts, CalcError, Category, EffectCount, Field, Format, Item, Move, Nature, Pokemon,
    PokemonType, RivalryTarget, Ruleset, SideConditions, Stat, StatTable, StatusCondition, Terrain,
    Weather,
};
