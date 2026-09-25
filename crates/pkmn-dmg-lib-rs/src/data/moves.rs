use crate::types::{Category, Move, PokemonType};

pub fn tackle() -> Move {
    Move::new("Tackle", 40, PokemonType::Normal, Category::Physical)
}

pub fn flamethrower() -> Move {
    let mut move_ = Move::new("Flamethrower", 90, PokemonType::Fire, Category::Special);
    move_.has_secondary_effect = true;
    move_
}

pub fn earthquake() -> Move {
    let mut move_ = Move::new("Earthquake", 100, PokemonType::Ground, Category::Physical);
    move_.is_spread = true;
    move_
}

pub fn thunderbolt() -> Move {
    let mut move_ = Move::new("Thunderbolt", 90, PokemonType::Electric, Category::Special);
    move_.has_secondary_effect = true;
    move_
}
