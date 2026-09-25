//! Reproduce the special-defense search omission and enumerate an independent oracle.
use damage_calc::Nature;
use spreadlab_rs::{
    damage_bridge::{calculate_benchmark, DamageBenchmark},
    data::ChampionsData,
    optimize::hp_def_survival_search,
    showdown::parse_set,
    StatPoints,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = ChampionsData::load()?;
    let mut benchmark = DamageBenchmark::new(
        parse_set("Dragapult @ Life Orb\nSPs: 32 SpA\nHardy Nature")?,
        parse_set("Mega Salamence\nHardy Nature")?,
        "Dragon Pulse",
    );
    let result = hp_def_survival_search(&data, &benchmark, &[Nature::Hardy], 0.25, 1)?;
    let best = &result.matches[0];
    println!(
        "Exact search: {} ({} points), KO {:?}",
        best.sp_line, best.total_points, best.result.ko_chance
    );
    for (hp, spd) in [(26, 0), (19, 4)] {
        benchmark.defender.stat_points = StatPoints::new(hp, 0, 0, 0, spd, 0);
        let damage = calculate_benchmark(&data, &benchmark)?;
        println!(
            "{hp} HP / {spd} SpD: {:?}, KO {:?}",
            damage.damage_rolls, damage.ko_chance
        );
    }
    let mut minimum = u16::MAX;
    let mut minima = Vec::new();
    for hp in 0..=32 {
        for spd in 0..=32 {
            benchmark.defender.stat_points = StatPoints::new(hp, 0, 0, 0, spd, 0);
            let damage = calculate_benchmark(&data, &benchmark)?;
            if damage.ko_chance.is_some_and(|ko| ko <= 0.25) {
                if hp + spd < minimum {
                    minimum = hp + spd;
                    minima.clear();
                }
                if hp + spd == minimum {
                    minima.push((hp, spd, damage.ko_chance));
                }
            }
        }
    }
    println!("Exhaustive HP/SpD minimum: {minimum} points; (HP, SpD, KO): {minima:?}");
    assert_eq!(minimum, 19);
    assert_eq!(best.total_points, minimum);
    assert_eq!(best.sps, StatPoints::new(5, 0, 0, 0, 14, 0));
    Ok(())
}
