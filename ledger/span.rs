use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use uuid::Uuid;

/// Status for a span entry on the timeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    Executed,
    Simulated,
    Reverted,
    Ghost,
}

impl Default for SpanStatus {
    fn default() -> Self {
        SpanStatus::Executed
    }
}

/// Semantic categorisation for a span.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    User,
    System,
    Organization,
    Ghost,
}

/// Visibility constraints for a span within a multi-tenant environment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Organization,
    Public,
}

/// Primary data structure describing a unit of work/event on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub logline_id: String,
    pub title: String,
    #[serde(default)]
    pub status: SpanStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caused_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_s: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_from: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_type: Option<SpanType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub processed: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub related_spans: Vec<String>,
}

impl Span {
    /// Create a new span with the minimum required information.
    pub fn new(logline_id: impl Into<String>, title: impl Into<String>) -> Self {
        SpanBuilder::new(logline_id, title).build()
    }

    /// Convenience constructor that immediately attaches payload data.
    pub fn with_payload(mut self, payload: Value) -> Self {
        self.data = Some(payload);
        self
    }

    /// Mark the span as processed.
    pub fn mark_processed(&mut self) {
        self.processed = true;
    }

    /// Attach a signature string to the span.
    pub fn sign(&mut self, signature: impl Into<String>) {
        self.signature = Some(signature.into());
        if self.verification_status.is_none() {
            self.verification_status = Some("verified".to_string());
        }
    }

    /// Add a tag for later filtering.
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let mut tags: BTreeSet<String> = self.tags.iter().cloned().collect();
        tags.insert(tag.into());
        self.tags = tags.into_iter().collect();
    }

    /// Relate this span to another span reference.
    pub fn relate_to(&mut self, reference: impl Into<String>) {
        let mut related: BTreeSet<String> = self.related_spans.iter().cloned().collect();
        related.insert(reference.into());
        self.related_spans = related.into_iter().collect();
    }

    /// Insert/override a metadata key.
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        let entry = self
            .metadata
            .get_or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(map) = entry {
            map.insert(key.into(), value.into());
        }
    }

    /// Check if the span contains a specific tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Calculate a deterministic hash for the span.
    pub fn hash(&self) -> String {
        use sha2::{Digest, Sha256};

        let json = serde_json::to_string(self).unwrap_or_else(|_| json!({}).to_string());
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Builder helper to create spans with many optional fields.
pub struct SpanBuilder {
    span: Span,
}

impl SpanBuilder {
    pub fn new(logline_id: impl Into<String>, title: impl Into<String>) -> Self {
        let span = Span {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            logline_id: logline_id.into(),
            title: title.into(),
            status: SpanStatus::Executed,
            data: None,
            contract_id: None,
            workflow_id: None,
            flow_id: None,
            caused_by: None,
            signature: None,
            verification_status: None,
            delta_s: None,
            replay_count: None,
            replay_from: None,
            tenant_id: None,
            organization_id: None,
            user_id: None,
            span_type: None,
            visibility: None,
            metadata: None,
            processed: false,
            tags: Vec::new(),
            related_spans: Vec::new(),
        };

        Self { span }
    }

    pub fn status(mut self, status: SpanStatus) -> Self {
        self.span.status = status;
        self
    }

    pub fn payload(mut self, payload: Value) -> Self {
        self.span.data = Some(payload);
        self
    }

    pub fn tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.span.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn organization_id(mut self, organization_id: Uuid) -> Self {
        self.span.organization_id = Some(organization_id);
        self
    }

    pub fn user_id(mut self, user_id: Uuid) -> Self {
        self.span.user_id = Some(user_id);
        self
    }

    pub fn span_type(mut self, span_type: SpanType) -> Self {
        self.span.span_type = Some(span_type);
        self
    }

    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.span.visibility = Some(visibility);
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.span.metadata = Some(metadata);
        self
    }

    pub fn build(self) -> Span {
        self.span
    }
}
