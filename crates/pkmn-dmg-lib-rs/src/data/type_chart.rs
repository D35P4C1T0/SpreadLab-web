use crate::types::PokemonType;

pub fn effectiveness(attack: PokemonType, defend: PokemonType) -> f32 {
    use PokemonType::*;
    match attack {
        Normal => match defend {
            Rock | Steel => 0.5,
            Ghost => 0.0,
            _ => 1.0,
        },
        Fire => match defend {
            Grass | Ice | Bug | Steel => 2.0,
            Fire | Water | Rock | Dragon => 0.5,
            _ => 1.0,
        },
        Water => match defend {
            Fire | Ground | Rock => 2.0,
            Water | Grass | Dragon => 0.5,
            _ => 1.0,
        },
        Electric => match defend {
            Water | Flying => 2.0,
            Electric | Grass | Dragon => 0.5,
            Ground => 0.0,
            _ => 1.0,
        },
        Grass => match defend {
            Water | Ground | Rock => 2.0,
            Fire | Grass | Poison | Flying | Bug | Dragon | Steel => 0.5,
            _ => 1.0,
        },
        Ice => match defend {
            Grass | Ground | Flying | Dragon => 2.0,
            Fire | Water | Ice | Steel => 0.5,
            _ => 1.0,
        },
        Fighting => match defend {
            Normal | Ice | Rock | Dark | Steel => 2.0,
            Poison | Flying | Psychic | Bug | Fairy => 0.5,
            Ghost => 0.0,
            _ => 1.0,
        },
        Poison => match defend {
            Grass | Fairy => 2.0,
            Poison | Ground | Rock | Ghost => 0.5,
            Steel => 0.0,
            _ => 1.0,
        },
        Ground => match defend {
            Fire | Electric | Poison | Rock | Steel => 2.0,
            Grass | Bug => 0.5,
            Flying => 0.0,
            _ => 1.0,
        },
        Flying => match defend {
            Grass | Fighting | Bug => 2.0,
            Electric | Rock | Steel => 0.5,
            _ => 1.0,
        },
        Psychic => match defend {
            Fighting | Poison => 2.0,
            Psychic | Steel => 0.5,
            Dark => 0.0,
            _ => 1.0,
        },
        Bug => match defend {
            Grass | Psychic | Dark => 2.0,
            Fire | Fighting | Poison | Flying | Ghost | Steel | Fairy => 0.5,
            _ => 1.0,
        },
        Rock => match defend {
            Fire | Ice | Flying | Bug => 2.0,
            Fighting | Ground | Steel => 0.5,
            _ => 1.0,
        },
        Ghost => match defend {
            Psychic | Ghost => 2.0,
            Dark => 0.5,
            Normal => 0.0,
            _ => 1.0,
        },
        Dragon => match defend {
            Dragon => 2.0,
            Steel => 0.5,
            Fairy => 0.0,
            _ => 1.0,
        },
        Dark => match defend {
            Psychic | Ghost => 2.0,
            Fighting | Dark | Fairy => 0.5,
            _ => 1.0,
        },
        Steel => match defend {
            Ice | Rock | Fairy => 2.0,
            Fire | Water | Electric | Steel => 0.5,
            _ => 1.0,
        },
        Fairy => match defend {
            Fighting | Dragon | Dark => 2.0,
            Fire | Poison | Steel => 0.5,
            _ => 1.0,
        },
        Stellar | Typeless => 1.0,
    }
}

/// Calculates a move's combined type effectiveness against defender typing.
///
/// This keeps the flat flag list used by the public damage path and existing callers.
#[allow(clippy::too_many_arguments)]
pub fn move_effectiveness(
    move_name: &str,
    move_type: PokemonType,
    defender_types: [Option<PokemonType>; 2],
    foresight: bool,
    scrappy: bool,
    gravity: bool,
    iron_ball: bool,
    ring_target: bool,
    strong_winds: bool,
) -> f32 {
    let mut total = 1.0;
    let mut seen_types = Vec::new();
    for defend_type in defender_types.into_iter().flatten() {
        if seen_types.contains(&defend_type) {
            continue;
        }
        seen_types.push(defend_type);
        let ignores_immunity = ((foresight || scrappy)
            && defend_type == PokemonType::Ghost
            && matches!(move_type, PokemonType::Normal | PokemonType::Fighting))
            || ((gravity || iron_ball)
                && defend_type == PokemonType::Flying
                && move_type == PokemonType::Ground)
            || (move_name == "Nihil Light" && defend_type == PokemonType::Fairy);

        let single = if ignores_immunity {
            1.0
        } else if move_name == "Freeze-Dry" && defend_type == PokemonType::Water {
            2.0
        } else {
            let mut value = effectiveness(move_type, defend_type);
            if ring_target && value == 0.0 {
                value = 1.0;
            }
            if strong_winds && defend_type == PokemonType::Flying && value > 1.0 {
                value = 1.0;
            }
            if move_name == "Flying Press" {
                value *= effectiveness(PokemonType::Flying, defend_type);
            }
            value
        };
        total *= single;
    }
    total
}
