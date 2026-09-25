pub mod abilities;
pub mod champions;
pub mod items;
pub mod moves;
pub mod pokemon;
pub mod type_chart;

/// Normalized Pokemon Champions data generated from vendored `champout` dumps.
///
/// The payload lives as JSON so consumers can deserialize it into their own
/// application schema without tying the core damage engine to a heavy static
/// table representation.
pub const CHAMPIONS_DATA_JSON: &str =
    include_str!("../../data/champions/generated/champions-data.json");
