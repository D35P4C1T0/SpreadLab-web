use crate::stats::{StatError, StatPoints, MAX_STAT_POINTS};
use damage_calc::{Nature, StatusCondition};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShowdownError {
    #[error("Showdown set is malformed: include only one training line type, either EVs or SPs")]
    MixedTrainingLines,
    #[error("Showdown set is malformed: include only one EVs/SPs line")]
    MultipleTrainingLines,
    #[error("Showdown set has a malformed {0} line")]
    MalformedTrainingLine(&'static str),
    #[error("unknown nature: {0}")]
    UnknownNature(String),
    #[error(transparent)]
    Stat(#[from] StatError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingFormat {
    Evs,
    Sps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedSet {
    pub species: String,
    pub item: Option<String>,
    pub ability: Option<String>,
    pub nature: Nature,
    pub tera_type: Option<String>,
    pub moves: Vec<String>,
    pub training_format: Option<TrainingFormat>,
    pub stat_points: StatPoints,
    pub status: StatusCondition,
    /// Whether the ability exists for this calculation. Independent of its
    /// conditional activation (for example Flash Fire's damage boost).
    #[serde(default = "default_ability_enabled")]
    pub ability_enabled: bool,
    pub ability_on: bool,
    pub supreme_overlord_allies: u8,
    pub rivalry: Option<RivalryMode>,
    pub move_targets_single_target: bool,
    pub original_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RivalryMode {
    SameGender,
    OppositeGender,
}

#[derive(Debug, Clone, Copy)]
struct TrainingLine<'a> {
    format: TrainingFormat,
    payload: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct ParsedTrainingValue {
    value: u16,
    nature_hint: Option<Nature>,
}

pub fn champions_points_from_ev(value: u16) -> u16 {
    MAX_STAT_POINTS.min((value + 4) / 8)
}

pub fn approximate_ev_from_champions(value: u16) -> u16 {
    let points = MAX_STAT_POINTS.min(value);
    if points == 0 {
        0
    } else {
        252.min(4 + (points - 1) * 8)
    }
}

fn default_ability_enabled() -> bool {
    true
}

pub fn parse_set(text: &str) -> Result<ParsedSet, ShowdownError> {
    let normalized = normalize_text(text);
    let species = parse_species(&normalized).unwrap_or_default();
    let item = parse_item(&normalized);
    let ability = parse_prefixed_line(&normalized, "Ability:");
    let tera_type = parse_prefixed_line(&normalized, "Tera Type:");
    let moves = normalized
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let (training_format, stat_points, nature_hint) = parse_training(&normalized)?;
    let nature = parse_nature(&normalized)?
        .or(nature_hint)
        .unwrap_or(Nature::Hardy);
    let status = parse_status(&normalized);
    let ability_on = parse_bool_line(&normalized, "Ability On:").unwrap_or_else(|| {
        ability
            .as_deref()
            .and_then(|name| crate::data::parse_ability(name).ok())
            .is_some_and(damage_calc::Ability::is_on_by_default)
    });
    let supreme_overlord_allies = parse_u8_line(&normalized, "Supreme Overlord Allies:")
        .or_else(|| parse_u8_line(&normalized, "Fainted Allies:"))
        .unwrap_or(0);
    let rivalry = parse_rivalry(&normalized);
    let move_targets_single_target = parse_prefixed_line(&normalized, "Target:")
        .map(|value| normalize_key(&value).contains("single"))
        .unwrap_or(false);

    Ok(ParsedSet {
        species,
        item,
        ability,
        nature,
        tera_type,
        moves,
        training_format,
        stat_points,
        status,
        ability_enabled: parse_bool_line(&normalized, "Ability Enabled:").unwrap_or(true),
        ability_on,
        supreme_overlord_allies,
        rivalry,
        move_targets_single_target,
        original_text: normalized,
    })
}

pub fn build_champions_sp_line(points: StatPoints) -> String {
    build_training_line("SPs", points, |value| value)
}

pub fn build_approximate_legacy_ev_line(points: StatPoints) -> String {
    build_training_line("EVs", points, approximate_ev_from_champions)
}

fn parse_training(
    text: &str,
) -> Result<(Option<TrainingFormat>, StatPoints, Option<Nature>), ShowdownError> {
    let lines = collect_training_lines(text);
    if lines.is_empty() {
        return Ok((None, StatPoints::default(), None));
    }
    if lines.len() > 1 {
        let has_evs = lines.iter().any(|line| line.format == TrainingFormat::Evs);
        let has_sps = lines.iter().any(|line| line.format == TrainingFormat::Sps);
        return if has_evs && has_sps {
            Err(ShowdownError::MixedTrainingLines)
        } else {
            Err(ShowdownError::MultipleTrainingLines)
        };
    }

    let line = lines[0];
    let (points, nature_hint) = match line.format {
        TrainingFormat::Evs => parse_evs_payload(line.payload)?,
        TrainingFormat::Sps => parse_sps_payload(line.payload)?,
    };
    points.validate()?;

    Ok((Some(line.format), points, nature_hint))
}

fn parse_evs_payload(payload: &str) -> Result<(StatPoints, Option<Nature>), ShowdownError> {
    if evs_payload_looks_like_champions_points(payload) {
        parse_training_payload(payload, MAX_STAT_POINTS, |value| value, "EVs")
    } else {
        parse_training_payload(payload, 252, champions_points_from_ev, "EVs")
    }
}

fn parse_sps_payload(payload: &str) -> Result<(StatPoints, Option<Nature>), ShowdownError> {
    parse_training_payload(payload, MAX_STAT_POINTS, |value| value, "SPs")
}

fn parse_training_payload(
    payload: &str,
    max_value: u16,
    convert: impl Fn(u16) -> u16,
    label: &'static str,
) -> Result<(StatPoints, Option<Nature>), ShowdownError> {
    let mut points = StatPoints::default();
    let mut parsed = 0;
    let mut nature_hint = None;

    for segment in payload.split('/') {
        let mut parts = segment.split_whitespace();
        let Some(raw_value) = parts.next() else {
            continue;
        };
        let Some(raw_stat) = parts.next() else {
            continue;
        };
        if parts.next().is_some() {
            continue;
        }
        let Some(parsed_value) = parse_training_value(raw_value, raw_stat) else {
            continue;
        };
        let value = max_value.min(parsed_value.value);
        let value = convert(value);
        match raw_stat.to_ascii_lowercase().as_str() {
            "hp" => points.hp = value,
            "atk" => points.attack = value,
            "def" => points.defense = value,
            "spa" => points.special_attack = value,
            "spd" => points.special_defense = value,
            "spe" => points.speed = value,
            _ => continue,
        }
        if nature_hint.is_none() {
            nature_hint = parsed_value.nature_hint;
        }
        parsed += 1;
    }

    if parsed == 0 {
        return Err(ShowdownError::MalformedTrainingLine(label));
    }
    Ok((points, nature_hint))
}

fn parse_training_value(raw_value: &str, raw_stat: &str) -> Option<ParsedTrainingValue> {
    let trimmed = raw_value.trim();
    let (number, suffix) = trimmed
        .strip_suffix('+')
        .map(|value| (value, Some('+')))
        .or_else(|| trimmed.strip_suffix('-').map(|value| (value, Some('-'))))
        .unwrap_or((trimmed, None));
    let value = number.parse::<u16>().ok()?;
    Some(ParsedTrainingValue {
        value,
        nature_hint: suffix.and_then(|suffix| nature_hint_from_suffix(raw_stat, suffix)),
    })
}

fn evs_payload_looks_like_champions_points(payload: &str) -> bool {
    let values = payload
        .split('/')
        .filter_map(|segment| {
            let mut parts = segment.split_whitespace();
            let raw_value = parts.next()?;
            let raw_stat = parts.next()?;
            parse_training_value(raw_value, raw_stat).map(|parsed| parsed.value)
        })
        .collect::<Vec<_>>();
    !values.is_empty() && values.iter().all(|value| *value <= MAX_STAT_POINTS)
}

fn nature_hint_from_suffix(raw_stat: &str, suffix: char) -> Option<Nature> {
    match (raw_stat.to_ascii_lowercase().as_str(), suffix) {
        ("atk", '+') => Some(Nature::Adamant),
        ("def", '+') => Some(Nature::Bold),
        ("spa", '+') => Some(Nature::Modest),
        ("spd", '+') => Some(Nature::Calm),
        ("spe", '+') => Some(Nature::Timid),
        ("atk", '-') => Some(Nature::Modest),
        ("def", '-') => Some(Nature::Mild),
        ("spa", '-') => Some(Nature::Adamant),
        ("spd", '-') => Some(Nature::Naughty),
        ("spe", '-') => Some(Nature::Brave),
        _ => None,
    }
}

fn collect_training_lines(text: &str) -> Vec<TrainingLine<'_>> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (label, payload) = trimmed.split_once(':')?;
            let format = match label.to_ascii_lowercase().as_str() {
                "evs" => TrainingFormat::Evs,
                "sps" => TrainingFormat::Sps,
                _ => return None,
            };
            Some(TrainingLine {
                format,
                payload: payload.trim(),
            })
        })
        .collect()
}

fn parse_species(text: &str) -> Option<String> {
    let first = text.lines().find(|line| !line.trim().is_empty())?.trim();
    let before_item = first.split_once('@').map_or(first, |(left, _)| left).trim();
    (!before_item.is_empty()).then(|| before_item.to_owned())
}

fn parse_item(text: &str) -> Option<String> {
    let first = text.lines().find(|line| !line.trim().is_empty())?.trim();
    first
        .split_once('@')
        .map(|(_, item)| item.trim().to_owned())
        .filter(|item| !item.is_empty())
}

fn parse_prefixed_line(text: &str, prefix: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_status(text: &str) -> StatusCondition {
    let Some(status) = parse_prefixed_line(text, "Status:") else {
        return StatusCondition::Healthy;
    };
    match normalize_key(&status).as_str() {
        "brn" | "burn" | "burned" => StatusCondition::Burned,
        "par" | "paralyzed" => StatusCondition::Paralyzed,
        "psn" | "poison" | "poisoned" => StatusCondition::Poisoned,
        "tox" | "badlypoisoned" => StatusCondition::BadlyPoisoned,
        "slp" | "asleep" | "sleep" => StatusCondition::Asleep,
        "drowsy" => StatusCondition::Drowsy,
        "frz" | "frozen" => StatusCondition::Frozen,
        _ => StatusCondition::Healthy,
    }
}

fn parse_bool_line(text: &str, prefix: &str) -> Option<bool> {
    parse_prefixed_line(text, prefix).and_then(|value| match normalize_key(&value).as_str() {
        "true" | "yes" | "on" | "active" | "1" => Some(true),
        "false" | "no" | "off" | "inactive" | "0" => Some(false),
        _ => None,
    })
}

fn parse_u8_line(text: &str, prefix: &str) -> Option<u8> {
    parse_prefixed_line(text, prefix)?.parse().ok()
}

fn parse_rivalry(text: &str) -> Option<RivalryMode> {
    parse_prefixed_line(text, "Rivalry:").and_then(|value| match normalize_key(&value).as_str() {
        "same" | "samegender" => Some(RivalryMode::SameGender),
        "opposite" | "oppositegender" => Some(RivalryMode::OppositeGender),
        _ => None,
    })
}

fn parse_nature(text: &str) -> Result<Option<Nature>, ShowdownError> {
    let Some(line) = text
        .lines()
        .map(str::trim)
        .find(|line| line.ends_with(" Nature"))
    else {
        return Ok(None);
    };
    let raw = line.trim_end_matches(" Nature").trim();
    parse_nature_name(raw)
        .map(Some)
        .ok_or_else(|| ShowdownError::UnknownNature(raw.to_owned()))
}

pub fn parse_nature_name(raw: &str) -> Option<Nature> {
    Some(match raw.to_ascii_lowercase().as_str() {
        "adamant" => Nature::Adamant,
        "bashful" => Nature::Bashful,
        "bold" => Nature::Bold,
        "brave" => Nature::Brave,
        "calm" => Nature::Calm,
        "careful" => Nature::Careful,
        "docile" => Nature::Docile,
        "gentle" => Nature::Gentle,
        "hardy" => Nature::Hardy,
        "hasty" => Nature::Hasty,
        "impish" => Nature::Impish,
        "jolly" => Nature::Jolly,
        "lax" => Nature::Lax,
        "lonely" => Nature::Lonely,
        "mild" => Nature::Mild,
        "modest" => Nature::Modest,
        "naive" => Nature::Naive,
        "naughty" => Nature::Naughty,
        "quiet" => Nature::Quiet,
        "quirky" => Nature::Quirky,
        "rash" => Nature::Rash,
        "relaxed" => Nature::Relaxed,
        "sassy" => Nature::Sassy,
        "serious" => Nature::Serious,
        "timid" => Nature::Timid,
        _ => return None,
    })
}

fn build_training_line(label: &str, points: StatPoints, map: impl Fn(u16) -> u16) -> String {
    let parts = [
        ("HP", points.hp),
        ("Atk", points.attack),
        ("Def", points.defense),
        ("SpA", points.special_attack),
        ("SpD", points.special_defense),
        ("Spe", points.speed),
    ]
    .into_iter()
    .filter(|(_, value)| *value > 0)
    .map(|(stat, value)| format!("{} {}", map(value), stat))
    .collect::<Vec<_>>();

    format!(
        "{label}: {}",
        if parts.is_empty() {
            "0 HP".to_owned()
        } else {
            parts.join(" / ")
        }
    )
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .collect::<String>()
        .trim()
        .to_owned()
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_enabled_is_independent_and_backwards_compatible() {
        let original = parse_set("Arcanine\nAbility: Flash Fire\nAbility On: false").unwrap();
        assert!(original.ability_enabled);
        assert!(!original.ability_on);
        let disabled =
            parse_set("Mega Lucario Z\nAbility: Aura Guard\nAbility Enabled: false").unwrap();
        assert!(!disabled.ability_enabled);
        assert!(disabled.ability_on);
        let mut old_json = serde_json::to_value(&original).unwrap();
        old_json.as_object_mut().unwrap().remove("ability_enabled");
        assert!(
            serde_json::from_value::<ParsedSet>(old_json)
                .unwrap()
                .ability_enabled
        );
    }

    #[test]
    fn ability_toggle_defaults_follow_upstream_and_allow_overrides() {
        assert!(
            parse_set("Salamence\nAbility: Intimidate")
                .unwrap()
                .ability_on
        );
        assert!(
            !parse_set("Salamence\nAbility: Intimidate\nAbility On: false")
                .unwrap()
                .ability_on
        );
        assert!(
            !parse_set("Charizard\nAbility: Flash Fire")
                .unwrap()
                .ability_on
        );
        assert!(
            parse_set("Charizard\nAbility: Flash Fire\nAbility On: true")
                .unwrap()
                .ability_on
        );
    }

    #[test]
    fn converts_evs_to_sps() {
        assert_eq!(champions_points_from_ev(0), 0);
        assert_eq!(champions_points_from_ev(4), 1);
        assert_eq!(champions_points_from_ev(12), 2);
        assert_eq!(champions_points_from_ev(252), 32);
    }

    #[test]
    fn parses_evs_line() {
        let parsed = parse_set("Pikachu @ Light Ball\nAbility: Static\nEVs: 252 Atk / 4 SpD / 252 Spe\nJolly Nature\n- Volt Tackle").unwrap();
        assert_eq!(parsed.species, "Pikachu");
        assert_eq!(parsed.item.as_deref(), Some("Light Ball"));
        assert_eq!(parsed.ability.as_deref(), Some("Static"));
        assert_eq!(parsed.nature, Nature::Jolly);
        assert_eq!(parsed.stat_points, StatPoints::new(0, 32, 0, 0, 1, 32));
    }

    #[test]
    fn parses_low_evs_as_champions_points() {
        let parsed = parse_set(
            "Sneasler @ White Herb\nAbility: Unburden\nEVs: 20 HP / 10 Atk / 21 Def / 15 Spe\nAdamant Nature\n- Close Combat",
        )
        .unwrap();

        assert_eq!(parsed.stat_points, StatPoints::new(20, 10, 21, 0, 0, 15));
        assert_eq!(parsed.nature, Nature::Adamant);
    }

    #[test]
    fn parses_spreads_over_66_total_points() {
        let parsed = parse_set(
            "Sneasler\nSPs: 32 HP / 32 Atk / 32 Def / 32 SpA / 32 SpD / 32 Spe\n- Close Combat",
        )
        .unwrap();

        assert_eq!(parsed.stat_points, StatPoints::new(32, 32, 32, 32, 32, 32));
    }

    #[test]
    fn parses_damage_calc_nature_suffixes() {
        let parsed = parse_set("Sneasler\nSPs: 10+ Atk\n- Close Combat").unwrap();
        assert_eq!(parsed.nature, Nature::Adamant);
        assert_eq!(parsed.stat_points.attack, 10);

        let explicit = parse_set("Sneasler\nSPs: 10+ Atk\nJolly Nature\n- Close Combat").unwrap();
        assert_eq!(explicit.nature, Nature::Jolly);
        assert_eq!(explicit.stat_points.attack, 10);
    }

    #[test]
    fn parses_damage_annotations() {
        let parsed = parse_set(
            "Kingambit\nStatus: Burned\nAbility On: true\nSupreme Overlord Allies: 3\nTarget: single\n- Kowtow Cleave",
        )
        .unwrap();

        assert_eq!(parsed.status, StatusCondition::Burned);
        assert!(parsed.ability_on);
        assert_eq!(parsed.supreme_overlord_allies, 3);
        assert!(parsed.move_targets_single_target);

        let same = parse_set("Luxray\nRivalry: same\n- Wild Charge").unwrap();
        assert_eq!(same.rivalry, Some(RivalryMode::SameGender));
        let opposite = parse_set("Luxray\nRivalry: opposite\n- Wild Charge").unwrap();
        assert_eq!(opposite.rivalry, Some(RivalryMode::OppositeGender));
    }

    #[test]
    fn rejects_mixed_training_lines() {
        let err = parse_set("Pikachu\nEVs: 252 Atk\nSPs: 32 Atk\nJolly Nature").unwrap_err();
        assert_eq!(err, ShowdownError::MixedTrainingLines);
    }

    #[test]
    fn exports_training_lines() {
        let points = StatPoints::new(0, 32, 0, 0, 1, 32);
        assert_eq!(
            build_champions_sp_line(points),
            "SPs: 32 Atk / 1 SpD / 32 Spe"
        );
        assert_eq!(
            build_approximate_legacy_ev_line(points),
            "EVs: 252 Atk / 4 SpD / 252 Spe"
        );
    }
}
