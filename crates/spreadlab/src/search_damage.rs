//! Request-local, bounded memoization of defensive search evaluations.
//!
//! Keep HP, nature, item and current HP in the key: damage and healing can depend
//! on them. Only omit a defense already proven irrelevant by the engine audit.
use crate::damage_bridge::{evaluate_input, prepare_benchmark, DamageBenchmark};
use crate::data::ChampionsData;
use crate::optimize::OptimizeError;
use crate::relevance::{benchmark_defender_relevance, DefensiveRelevance};
use crate::stats::StatPoints;
use damage_calc::{CalcInput, DamageResult, Item, Nature};
use std::collections::HashMap;

const MAX_ENTRIES: usize = 4096;

#[derive(Hash, PartialEq, Eq)]
struct Key {
    nature: Nature,
    points: [u16; 6],
    current_hp: u16,
    item: Item,
}

pub(crate) struct SearchDamage {
    input: CalcInput,
    relevance: DefensiveRelevance,
    cache: HashMap<Key, DamageResult>,
    #[cfg(test)]
    calculations: usize,
}

impl SearchDamage {
    pub(crate) fn new(
        data: &ChampionsData,
        benchmark: &DamageBenchmark,
    ) -> Result<Self, OptimizeError> {
        Ok(Self {
            input: prepare_benchmark(data, benchmark)?,
            relevance: benchmark_defender_relevance(data, benchmark)?,
            cache: HashMap::new(),
            #[cfg(test)]
            calculations: 0,
        })
    }

    pub(crate) fn initial_item(&self) -> Item {
        self.input.defender.item
    }

    pub(crate) fn calculate(
        &mut self,
        nature: Nature,
        points: StatPoints,
        current_hp: u16,
        item: Item,
    ) -> Result<DamageResult, OptimizeError> {
        let key = Key {
            nature,
            points: [
                points.hp,
                points.attack,
                if self.relevance.defense {
                    points.defense
                } else {
                    0
                },
                points.special_attack,
                if self.relevance.special_defense {
                    points.special_defense
                } else {
                    0
                },
                points.speed,
            ],
            current_hp,
            item,
        };
        if let Some(result) = self.cache.get(&key) {
            return Ok(result.clone());
        }
        let mut input = self.input.clone();
        input.defender.nature = nature;
        input.defender.stat_points = points.into();
        input.defender.current_hp = Some(current_hp);
        input.defender.item = item;
        let result = evaluate_input(input)?;
        // Bound memory on long sequences and large nature domains. Eviction changes
        // only runtime, never the result. Caches never outlive a request.
        if self.cache.len() >= MAX_ENTRIES {
            self.cache.clear();
        }
        self.cache.insert(key, result.clone());
        #[cfg(test)]
        {
            self.calculations += 1;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage_bridge::calculate_benchmark;
    use crate::showdown::parse_set;

    #[test]
    fn cached_search_matches_engine_and_reuses_irrelevant_defense() {
        let data = ChampionsData::load().unwrap();
        for move_name in ["Iron Head", "Shadow Ball", "Psyshock", "Foul Play"] {
            let mut benchmark = DamageBenchmark::new(
                parse_set("Kingambit\nAbility: Defiant\nSPs: 32 Atk").unwrap(),
                parse_set("Mega Floette @ Sitrus Berry").unwrap(),
                move_name,
            );
            let mut cache = SearchDamage::new(&data, &benchmark).unwrap();
            for nature in [Nature::Hardy, Nature::Bold] {
                for hp in [0, 8] {
                    for def in [0, 4] {
                        for spd in [0, 4] {
                            for current_hp in [50, 150] {
                                for item in [Item::None, Item::SitrusBerry] {
                                    let points = StatPoints::new(hp, 0, def, 0, spd, 0);
                                    benchmark.defender.nature = nature;
                                    benchmark.defender.stat_points = points;
                                    benchmark.defender.item = Some(format!("{item:?}"));
                                    benchmark.defender_current_hp = Some(current_hp);
                                    let expected = calculate_benchmark(&data, &benchmark).unwrap();
                                    assert_eq!(
                                        cache.calculate(nature, points, current_hp, item).unwrap(),
                                        expected,
                                        "{move_name}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            assert!(
                cache.calculations <= 64,
                "irrelevant axis must share evaluations"
            );
        }
    }

    #[test]
    #[ignore = "manual release-mode performance comparison"]
    fn benchmark_mixed_grid() {
        use std::time::Instant;
        let data = ChampionsData::load().unwrap();
        let mut benchmarks = ["Iron Head", "Shadow Ball"].map(|name| {
            DamageBenchmark::new(
                parse_set("Kingambit\nSPs: 32 Atk / 32 SpA").unwrap(),
                parse_set("Mega Floette @ Sitrus Berry").unwrap(),
                name,
            )
        });
        let species = data.species("Mega Floette").unwrap();
        let mut cached = benchmarks
            .iter()
            .map(|b| SearchDamage::new(&data, b).unwrap())
            .collect::<Vec<_>>();
        let run = |use_cache: bool,
                   benchmarks: &mut [DamageBenchmark],
                   cached: &mut [SearchDamage]| {
            let start = Instant::now();
            let mut checksum = 0u64;
            for hp in 0..=32 {
                for def in 0..=32 {
                    for spd in 0..=32 {
                        if hp + def + spd > 66 {
                            continue;
                        }
                        let points = StatPoints::new(hp, 0, def, 0, spd, 0);
                        let current_hp = crate::stats::champions_final_stats(
                            species.base_stats(),
                            Nature::Hardy,
                            points,
                        )
                        .unwrap()
                        .hp;
                        for (benchmark, evaluator) in benchmarks.iter_mut().zip(cached.iter_mut()) {
                            let result = if use_cache {
                                evaluator
                                    .calculate(Nature::Hardy, points, current_hp, Item::SitrusBerry)
                                    .unwrap()
                            } else {
                                benchmark.defender.stat_points = points;
                                benchmark.defender_current_hp = Some(current_hp);
                                calculate_benchmark(&data, benchmark).unwrap()
                            };
                            checksum += u64::from(result.max_damage);
                        }
                    }
                }
            }
            (checksum, start.elapsed())
        };
        let baseline = run(false, &mut benchmarks, &mut cached);
        let optimized = run(true, &mut benchmarks, &mut cached);
        assert_eq!(baseline.0, optimized.0);
        assert!(cached.iter().all(|c| c.calculations == 33 * 33));
        eprintln!(
            "mixed legal grid: uncached {:?}, prepared/cache {:?}; {} engine calls",
            baseline.1,
            optimized.1,
            cached.iter().map(|c| c.calculations).sum::<usize>()
        );
    }
}
