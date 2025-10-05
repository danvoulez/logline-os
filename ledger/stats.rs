use serde::{Deserialize, Serialize};

/// Aggregated statistics returned by timeline backends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineStats {
    pub total_spans: u64,
    pub signed_spans: u64,
    pub contract_spans: u64,
    pub executed_spans: u64,
    pub simulated_spans: u64,
    pub ghost_spans: u64,
    pub other_spans: u64,
    pub unique_logline_ids: Vec<String>,
}
