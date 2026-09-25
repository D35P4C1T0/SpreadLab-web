use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use spreadlab_rs::damage_bridge::{calculate_benchmark, DamageBenchmark};
use spreadlab_rs::data::ChampionsData;
use spreadlab_rs::optimize::{
    hp_def_combined_survival_search, hp_def_survival_search_from_hp_percent, offensive_ko_search,
    optimize_defensive, optimize_offensive, optimized_combined_defensive_natures,
    optimized_defensive_natures, optimized_offensive_natures, CombinedSurvivalSpread, KoSpread,
    RankedSpread, SurvivalSpread,
};
use spreadlab_rs::showdown::{build_champions_sp_line, parse_nature_name, parse_set};
use spreadlab_rs::spreads::{LockedStats, SpreadSearch};
use spreadlab_rs::stats::champions_final_stats;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "spreadlab-rs")]
#[command(about = "SpreadLab: Pokemon Champions Stat Point optimizer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse a Showdown set and print canonical Champions SPs.
    Parse {
        #[arg(value_name = "SET_FILE")]
        set_file: PathBuf,
    },
    /// Print final Champions raw stats for one set.
    Stats {
        #[arg(value_name = "SET_FILE")]
        set_file: PathBuf,
    },
    /// Run one damage calculation using damage_calc as ground truth.
    Calc {
        #[arg(long)]
        attacker: PathBuf,
        #[arg(long)]
        defender: PathBuf,
        #[arg(long = "move")]
        move_name: String,
        #[arg(long, default_value_t = 0)]
        move_times_affected: u8,
        #[arg(long = "crit")]
        critical: bool,
        #[arg(long)]
        json: bool,
    },
    /// Search legal SP spreads for one benchmark.
    Optimize {
        #[command(subcommand)]
        mode: OptimizeCommand,
    },
    /// Exact survival search from JSON (locks, budgets, independent or sequence constraints).
    SurviveExact {
        #[arg(value_name = "REQUEST_JSON")]
        request: PathBuf,
    },
    /// Find minimum HP/Def/SpD SPs that keep KO chance under a threshold.
    Survive {
        #[arg(long)]
        attacker: PathBuf,
        #[arg(long)]
        defender: PathBuf,
        #[arg(long = "move")]
        move_name: String,
        #[arg(long, default_value_t = 0.125)]
        max_ko_chance: f32,
        #[arg(long, default_value_t = 100.0)]
        hp_percent: f32,
        #[arg(long)]
        nature: Option<String>,
        #[arg(long)]
        optimize_nature: bool,
        #[arg(long)]
        show_closest_miss: bool,
        #[arg(long, default_value_t = 0)]
        move_times_affected: u8,
        #[arg(long = "crit")]
        critical: bool,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Find minimum HP/Def/SpD SPs that survive two attacks in a row.
    SurviveSequence {
        #[arg(long)]
        attacker1: PathBuf,
        #[arg(long = "move1")]
        move_name1: String,
        #[arg(long)]
        attacker2: PathBuf,
        #[arg(long = "move2")]
        move_name2: String,
        #[arg(long)]
        defender: PathBuf,
        #[arg(long, default_value_t = 0.125)]
        max_ko_chance: f32,
        #[arg(long, default_value_t = 100.0)]
        hp_percent: f32,
        #[arg(long)]
        nature: Option<String>,
        #[arg(long)]
        optimize_nature: bool,
        #[arg(long)]
        show_closest_miss: bool,
        #[arg(long, default_value_t = 0)]
        move_times_affected1: u8,
        #[arg(long = "crit1")]
        critical1: bool,
        #[arg(long, default_value_t = 0)]
        move_times_affected2: u8,
        #[arg(long = "crit2")]
        critical2: bool,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Find minimum offensive SPs that reach a KO chance threshold.
    Ko {
        #[arg(long)]
        attacker: PathBuf,
        #[arg(long)]
        defender: PathBuf,
        #[arg(long = "move")]
        move_name: String,
        #[arg(long, default_value_t = 1.0)]
        min_ko_chance: f32,
        #[arg(long)]
        nature: Option<String>,
        #[arg(long)]
        optimize_nature: bool,
        #[arg(long)]
        show_closest_miss: bool,
        #[arg(long, default_value_t = 0)]
        move_times_affected: u8,
        #[arg(long = "crit")]
        critical: bool,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
    /// Print pinned Champions names from the damage library.
    List {
        #[arg(value_enum)]
        kind: ListKind,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ListKind {
    Species,
    Regulation,
    Items,
    Abilities,
    Moves,
}

#[derive(Debug, Subcommand)]
enum OptimizeCommand {
    /// Optimize defender SPs to reduce incoming damage.
    Defensive(OptimizeArgs),
    /// Optimize attacker SPs to increase outgoing damage.
    Offensive(OptimizeArgs),
}

#[derive(Debug, Parser)]
struct OptimizeArgs {
    #[arg(long)]
    benchmarks: Option<PathBuf>,
    #[arg(long)]
    attacker: Option<PathBuf>,
    #[arg(long)]
    defender: Option<PathBuf>,
    #[arg(long = "move")]
    move_name: Option<String>,
    #[arg(long = "crit")]
    critical: bool,
    #[arg(long)]
    full_spend: bool,
    #[arg(long)]
    lock_hp: Option<u16>,
    #[arg(long)]
    lock_atk: Option<u16>,
    #[arg(long)]
    lock_def: Option<u16>,
    #[arg(long)]
    lock_spa: Option<u16>,
    #[arg(long)]
    lock_spd: Option<u16>,
    #[arg(long)]
    lock_spe: Option<u16>,
    #[arg(long, default_value_t = 10)]
    limit: usize,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Deserialize)]
struct BenchmarkFile {
    benchmarks: Vec<BenchmarkEntry>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkEntry {
    attacker: String,
    defender: String,
    #[serde(rename = "move")]
    move_name: String,
    #[serde(default)]
    critical: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Parse { set_file } => {
            let set = parse_file(&set_file)?;
            println!("species: {}", set.species);
            println!("nature: {:?}", set.nature);
            println!("{}", build_champions_sp_line(set.stat_points));
        }
        Command::Stats { set_file } => {
            let data = ChampionsData::load()?;
            let set = parse_file(&set_file)?;
            let species = data.species(&set.species)?;
            let stats = champions_final_stats(species.base_stats(), set.nature, set.stat_points)?;
            println!("species: {}", species.display_name);
            println!("{}", build_champions_sp_line(set.stat_points));
            println!(
                "final stats: {}/{}/{}/{}/{}/{}",
                stats.hp,
                stats.attack,
                stats.defense,
                stats.special_attack,
                stats.special_defense,
                stats.speed
            );
        }
        Command::Calc {
            attacker,
            defender,
            move_name,
            move_times_affected,
            critical,
            json,
        } => {
            let data = ChampionsData::load()?;
            let attacker = parse_file(&attacker)?;
            let defender = parse_file(&defender)?;
            let mut benchmark = DamageBenchmark::new(attacker, defender, move_name);
            benchmark.move_times_affected = move_times_affected;
            benchmark.critical = critical;
            let result = calculate_benchmark(&data, &benchmark)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("damage: {}-{}", result.min_damage, result.max_damage);
                println!(
                    "percent: {:.1}-{:.1}%",
                    result.percent_range.0, result.percent_range.1
                );
                if let Some(ko_chance) = result.ko_chance {
                    println!("KO chance: {:.1}%", ko_chance * 100.0);
                }
                println!("rolls: {:?}", result.damage_rolls);
            }
        }
        Command::Optimize { mode } => match mode {
            OptimizeCommand::Defensive(args) => run_optimize(args, true)?,
            OptimizeCommand::Offensive(args) => run_optimize(args, false)?,
        },
        Command::SurviveExact { request } => {
            let request = serde_json::from_str(&fs::read_to_string(request)?)?;
            let result = spreadlab_rs::api::find_min_survival(request)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Command::Survive {
            attacker,
            defender,
            move_name,
            max_ko_chance,
            hp_percent,
            nature,
            optimize_nature,
            show_closest_miss,
            move_times_affected,
            critical,
            limit,
            json,
        } => {
            let data = ChampionsData::load()?;
            let mut benchmark =
                DamageBenchmark::new(parse_file(&attacker)?, parse_file(&defender)?, move_name);
            benchmark.move_times_affected = move_times_affected;
            benchmark.critical = critical;
            let natures = match nature {
                Some(raw) => vec![parse_nature_name(&raw).context("unknown nature")?],
                None if optimize_nature => optimized_defensive_natures(&data, &benchmark)?.to_vec(),
                None => vec![benchmark.defender.nature],
            };
            let result = hp_def_survival_search_from_hp_percent(
                &data,
                &benchmark,
                &natures,
                max_ko_chance,
                hp_percent,
                limit,
            )?;
            if json {
                if show_closest_miss {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result.matches)?);
                }
            } else {
                print_survival(&result.matches);
                if show_closest_miss {
                    print_survival_closest_miss(result.closest_miss.as_ref());
                }
            }
        }
        Command::SurviveSequence {
            attacker1,
            move_name1,
            attacker2,
            move_name2,
            defender,
            max_ko_chance,
            hp_percent,
            nature,
            optimize_nature,
            show_closest_miss,
            move_times_affected1,
            critical1,
            move_times_affected2,
            critical2,
            limit,
            json,
        } => {
            let data = ChampionsData::load()?;
            let defender = parse_file(&defender)?;
            let mut first =
                DamageBenchmark::new(parse_file(&attacker1)?, defender.clone(), move_name1);
            first.move_times_affected = move_times_affected1;
            first.critical = critical1;
            let mut second = DamageBenchmark::new(parse_file(&attacker2)?, defender, move_name2);
            second.move_times_affected = move_times_affected2;
            second.critical = critical2;
            let benchmarks = vec![first, second];
            let natures = match nature {
                Some(raw) => vec![parse_nature_name(&raw).context("unknown nature")?],
                None if optimize_nature => {
                    optimized_combined_defensive_natures(&data, &benchmarks)?
                }
                None => vec![benchmarks[0].defender.nature],
            };
            let result = hp_def_combined_survival_search(
                &data,
                &benchmarks,
                &natures,
                max_ko_chance,
                hp_percent,
                limit,
            )?;
            if json {
                if show_closest_miss {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result.matches)?);
                }
            } else {
                print_combined_survival(&result.matches);
                if show_closest_miss {
                    print_combined_survival_closest_miss(result.closest_miss.as_ref());
                }
            }
        }
        Command::Ko {
            attacker,
            defender,
            move_name,
            min_ko_chance,
            nature,
            optimize_nature,
            show_closest_miss,
            move_times_affected,
            critical,
            limit,
            json,
        } => {
            let data = ChampionsData::load()?;
            let mut benchmark =
                DamageBenchmark::new(parse_file(&attacker)?, parse_file(&defender)?, move_name);
            benchmark.move_times_affected = move_times_affected;
            benchmark.critical = critical;
            let natures = match nature {
                Some(raw) => vec![parse_nature_name(&raw).context("unknown nature")?],
                None if optimize_nature => optimized_offensive_natures(&data, &benchmark)?.to_vec(),
                None => vec![benchmark.attacker.nature],
            };
            let result = offensive_ko_search(&data, &benchmark, &natures, min_ko_chance, limit)?;
            if json {
                if show_closest_miss {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result.matches)?);
                }
            } else {
                print_ko(&result.matches);
                if show_closest_miss {
                    print_ko_closest_miss(result.closest_miss.as_ref());
                }
            }
        }
        Command::List { kind } => {
            let data = ChampionsData::load()?;
            let mut names = match kind {
                ListKind::Species => data.species_names().map(str::to_owned).collect::<Vec<_>>(),
                ListKind::Regulation => data
                    .regulation_m_c_names()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                ListKind::Items => data.item_names().map(str::to_owned).collect::<Vec<_>>(),
                ListKind::Abilities => data.ability_names().map(str::to_owned).collect::<Vec<_>>(),
                ListKind::Moves => data.move_names().map(str::to_owned).collect::<Vec<_>>(),
            };
            names.sort();
            names.dedup();
            for name in names {
                println!("{name}");
            }
        }
    }
    Ok(())
}

fn print_survival(ranked: &[SurvivalSpread]) {
    if ranked.is_empty() {
        println!("No spread satisfies requested chance.");
        return;
    }
    for spread in ranked {
        println!("Rank {}", spread.rank);
        println!("nature: {:?}", spread.nature);
        println!("{}", spread.sp_line);
        println!("total points: {}", spread.total_points);
        println!(
            "final stats: {}/{}/{}/{}/{}/{}",
            spread.final_stats.hp,
            spread.final_stats.attack,
            spread.final_stats.defense,
            spread.final_stats.special_attack,
            spread.final_stats.special_defense,
            spread.final_stats.speed
        );
        println!(
            "damage: {}-{} ({:.1}-{:.1}%), KO chance {}",
            spread.result.min_damage,
            spread.result.max_damage,
            spread.result.percent_min,
            spread.result.percent_max,
            spread
                .result
                .ko_chance
                .map(|chance| format!("{:.1}%", chance * 100.0))
                .unwrap_or_else(|| "n/a".to_owned())
        );
        println!();
    }
}

fn print_survival_closest_miss(spread: Option<&SurvivalSpread>) {
    println!("Closest miss");
    match spread {
        Some(spread) => print_survival(std::slice::from_ref(spread)),
        None => println!("none"),
    }
}

fn print_combined_survival(ranked: &[CombinedSurvivalSpread]) {
    if ranked.is_empty() {
        println!("No spread satisfies requested chance.");
        return;
    }
    for spread in ranked {
        println!("Rank {}", spread.rank);
        println!("nature: {:?}", spread.nature);
        println!("{}", spread.sp_line);
        println!("total points: {}", spread.total_points);
        println!(
            "final stats: {}/{}/{}/{}/{}/{}",
            spread.final_stats.hp,
            spread.final_stats.attack,
            spread.final_stats.defense,
            spread.final_stats.special_attack,
            spread.final_stats.special_defense,
            spread.final_stats.speed
        );
        for (index, hit) in spread.hits.iter().enumerate() {
            println!(
                "hit {}: {}-{} ({:.1}-{:.1}%), KO chance {}",
                index + 1,
                hit.min_damage,
                hit.max_damage,
                hit.percent_min,
                hit.percent_max,
                hit.ko_chance
                    .map(|chance| format!("{:.1}%", chance * 100.0))
                    .unwrap_or_else(|| "n/a".to_owned())
            );
        }
        println!(
            "combined: {}-{} ({:.1}-{:.1}%), starting HP {}, KO chance {:.1}%",
            spread.combined.min_damage,
            spread.combined.max_damage,
            spread.combined.percent_min,
            spread.combined.percent_max,
            spread.combined.starting_hp,
            spread.combined.ko_chance * 100.0
        );
        println!();
    }
}

fn print_combined_survival_closest_miss(spread: Option<&CombinedSurvivalSpread>) {
    println!("Closest miss");
    match spread {
        Some(spread) => print_combined_survival(std::slice::from_ref(spread)),
        None => println!("none"),
    }
}

fn print_ko(ranked: &[KoSpread]) {
    if ranked.is_empty() {
        println!("No spread satisfies requested chance.");
        return;
    }
    for spread in ranked {
        println!("Rank {}", spread.rank);
        println!("nature: {:?}", spread.nature);
        println!("investment: {:?}", spread.investment_stat);
        println!("{}", spread.sp_line);
        println!("total points: {}", spread.total_points);
        println!(
            "final stats: {}/{}/{}/{}/{}/{}",
            spread.final_stats.hp,
            spread.final_stats.attack,
            spread.final_stats.defense,
            spread.final_stats.special_attack,
            spread.final_stats.special_defense,
            spread.final_stats.speed
        );
        println!(
            "damage: {}-{} ({:.1}-{:.1}%), KO chance {}",
            spread.result.min_damage,
            spread.result.max_damage,
            spread.result.percent_min,
            spread.result.percent_max,
            spread
                .result
                .ko_chance
                .map(|chance| format!("{:.1}%", chance * 100.0))
                .unwrap_or_else(|| "n/a".to_owned())
        );
        println!();
    }
}

fn print_ko_closest_miss(spread: Option<&KoSpread>) {
    println!("Closest miss");
    match spread {
        Some(spread) => print_ko(std::slice::from_ref(spread)),
        None => println!("none"),
    }
}

fn parse_file(path: &PathBuf) -> Result<spreadlab_rs::showdown::ParsedSet> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    parse_set(&text).with_context(|| format!("parse {}", path.display()))
}

fn run_optimize(args: OptimizeArgs, defensive: bool) -> Result<()> {
    let data = ChampionsData::load()?;
    let benchmarks = load_optimize_benchmarks(&args)?;
    let mut search = if args.full_spend {
        SpreadSearch::full_spend()
    } else {
        SpreadSearch::all_legal()
    };
    search.locked = LockedStats {
        hp: args.lock_hp,
        attack: args.lock_atk,
        defense: args.lock_def,
        special_attack: args.lock_spa,
        special_defense: args.lock_spd,
        speed: args.lock_spe,
    };
    let ranked = if defensive {
        optimize_defensive(&data, &benchmarks, search, args.limit)?
    } else {
        optimize_offensive(&data, &benchmarks, search, args.limit)?
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&ranked)?);
    } else {
        print_ranked(&ranked);
    }
    Ok(())
}

fn load_optimize_benchmarks(args: &OptimizeArgs) -> Result<Vec<DamageBenchmark>> {
    if let Some(path) = &args.benchmarks {
        let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let file: BenchmarkFile =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        return file
            .benchmarks
            .into_iter()
            .map(|entry| {
                let attacker = parse_set(&entry.attacker).context("parse benchmark attacker")?;
                let defender = parse_set(&entry.defender).context("parse benchmark defender")?;
                let mut benchmark = DamageBenchmark::new(attacker, defender, entry.move_name);
                benchmark.critical = entry.critical;
                Ok(benchmark)
            })
            .collect();
    }

    let attacker = args
        .attacker
        .as_ref()
        .context("--attacker is required unless --benchmarks is used")?;
    let defender = args
        .defender
        .as_ref()
        .context("--defender is required unless --benchmarks is used")?;
    let move_name = args
        .move_name
        .as_ref()
        .context("--move is required unless --benchmarks is used")?;
    let mut benchmark = DamageBenchmark::new(
        parse_file(attacker)?,
        parse_file(defender)?,
        move_name.clone(),
    );
    benchmark.critical = args.critical;
    Ok(vec![benchmark])
}

fn print_ranked(ranked: &[RankedSpread]) {
    for spread in ranked {
        println!("Rank {}", spread.rank);
        println!("{}", spread.sp_line);
        println!(
            "final stats: {}/{}/{}/{}/{}/{}",
            spread.final_stats.hp,
            spread.final_stats.attack,
            spread.final_stats.defense,
            spread.final_stats.special_attack,
            spread.final_stats.special_defense,
            spread.final_stats.speed
        );
        for (index, result) in spread.results.iter().enumerate() {
            println!(
                "benchmark {}: {}-{} ({:.1}-{:.1}%), KO chance {}",
                index + 1,
                result.min_damage,
                result.max_damage,
                result.percent_min,
                result.percent_max,
                result
                    .ko_chance
                    .map(|chance| format!("{:.1}%", chance * 100.0))
                    .unwrap_or_else(|| "n/a".to_owned())
            );
        }
        println!();
    }
}
