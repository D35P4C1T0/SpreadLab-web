pub mod scoring;
pub mod spreads;

/// Placeholder result for the future spread optimizer.
///
/// The optimizer is intentionally left thin until fixture parity for core
/// damage is broad enough to make benchmark search meaningful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedSpread {
    pub explanation: String,
}
