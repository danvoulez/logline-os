use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Materialised view of a span stored inside a timeline backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub logline_id: String,
    pub author: String,
    pub title: String,
    pub payload: serde_json::Value,
    pub contract_id: Option<String>,
    pub workflow_id: Option<String>,
    pub flow_id: Option<String>,
    pub caused_by: Option<Uuid>,
    pub signature: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub tenant_id: Option<String>,
    pub organization_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub span_type: Option<String>,
    pub visibility: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub organization_name: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub delta_s: Option<f64>,
    pub replay_count: Option<u32>,
    pub verification_status: Option<String>,
}
