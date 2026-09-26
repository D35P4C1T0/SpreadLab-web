use spreadlab_rs::{
    damage_bridge::{calculate_benchmark, DamageBenchmark},
    data::ChampionsData,
    showdown::parse_set,
};

#[test]
fn bundled_common_sets_are_ranked_legal_and_calculable() {
    let source = include_str!("../../spreadlab-web/assets/common-sets.js");
    let json = source
        .split_once("const COMMON_SETS = ")
        .unwrap()
        .1
        .trim()
        .trim_end_matches(';');
    let catalog: serde_json::Value = serde_json::from_str(json).unwrap();
    let sets = catalog["sets"].as_array().unwrap();
    assert_eq!(sets.len(), 125);
    let data = ChampionsData::load().unwrap();
    let mut errors = Vec::new();
    let mut previous_usage = f64::INFINITY;
    for (index, set) in sets.iter().enumerate() {
        assert_eq!(set["rank"].as_u64().unwrap(), index as u64 + 1);
        let usage = set["usage"].as_f64().unwrap();
        assert!(usage <= previous_usage);
        previous_usage = usage;
        let name = set["pokemon"].as_str().unwrap();
        let item = set["item"].as_str().unwrap();
        let points = [
            ("hp", "HP"),
            ("at", "Atk"),
            ("df", "Def"),
            ("sa", "SpA"),
            ("sd", "SpD"),
            ("sp", "Spe"),
        ]
        .map(|(key, label)| format!("{} {label}", set["sps"][key]))
        .join(" / ");
        let text = format!(
            "{name}{}\nAbility: {}\n{} Nature\nSPs: {points}",
            if item.is_empty() {
                String::new()
            } else {
                format!(" @ {item}")
            },
            set["ability"].as_str().unwrap(),
            set["nature"].as_str().unwrap()
        );
        let attacker = parse_set(&text).unwrap();
        assert!(attacker.stat_points.total() <= 66, "{name}");
        for move_name in set["moves"].as_array().unwrap() {
            let benchmark = DamageBenchmark::new(
                attacker.clone(),
                parse_set("Kingambit").unwrap(),
                move_name.as_str().unwrap(),
            );
            if let Err(error) = calculate_benchmark(&data, &benchmark) {
                errors.push(format!("{name}: {error}"));
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
