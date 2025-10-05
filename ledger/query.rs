use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Client-facing query filters for timeline retrieval.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineQuery {
    pub logline_id: Option<String>,
    pub contract_id: Option<String>,
    pub workflow_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub tenant_id: Option<String>,
    pub organization_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub span_type: Option<String>,
    pub visibility: Option<String>,
}
