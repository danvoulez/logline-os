use std::fmt;

use logline_protocol::timeline::Span;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use url::Url;

/// Typed HTTP client used by the engine to interact with the external
/// `logline-rules` microservice.
#[derive(Clone)]
pub struct RulesServiceClient {
    http: reqwest::Client,
    base_url: Url,
}

impl RulesServiceClient {
    /// Creates a new client bound to the provided base URL.
    pub fn new(base_url: &str) -> Result<Self, RulesClientError> {
        let mut url = Url::parse(base_url).map_err(|err| RulesClientError::InvalidUrl {
            url: base_url.to_string(),
            source: err,
        })?;

        if !url.path().ends_with('/') {
            let mut path = url.path().trim_end_matches('/').to_string();
            path.push('/');
            url.set_path(&path);
        }

        Ok(Self {
            http: reqwest::Client::new(),
            base_url: url,
        })
    }

    /// Sends the span for evaluation and returns the enriched outcome produced
    /// by the rules service.
    pub async fn evaluate_span(
        &self,
        tenant_id: &str,
        span: &Span,
    ) -> Result<RulesEvaluation, RulesClientError> {
        let url = self
            .base_url
            .join(&format!(
                "tenants/{}/evaluate",
                encode_path_segment(tenant_id)
            ))
            .map_err(|err| RulesClientError::InvalidUrl {
                url: format!("{}/tenants/{}/evaluate", self.base_url, tenant_id),
                source: err,
            })?;

        let request = EvaluationRequest { span: span.clone() };

        let response = self
            .http
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|err| RulesClientError::Http(err.to_string()))?;

        if !response.status().is_success() {
            return Err(RulesClientError::UnexpectedStatus {
                status: response.status(),
            });
        }

        let payload: EvaluationResponse = response
            .json()
            .await
            .map_err(|err| RulesClientError::Decode(err.to_string()))?;

        Ok(payload.into())
    }

    /// Returns the configured base URL as a string reference.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
}

fn encode_path_segment(segment: &str) -> String {
    url::form_urlencoded::byte_serialize(segment.as_bytes()).collect()
}

#[derive(Debug, Serialize)]
struct EvaluationRequest {
    span: Span,
}

#[derive(Debug, Deserialize)]
struct EvaluationResponse {
    decision: EvaluationDecision,
    applied_rules: Vec<String>,
    notes: Vec<String>,
    #[serde(default, rename = "tags")]
    added_tags: Vec<String>,
    #[serde(default)]
    metadata_updates: Map<String, Value>,
    span: Span,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EvaluationDecision {
    pub state: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl fmt::Display for EvaluationDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(reason) = &self.reason {
            write!(f, "{} ({})", self.state, reason)
        } else if let Some(note) = &self.note {
            write!(f, "{} ({})", self.state, note)
        } else {
            write!(f, "{}", self.state)
        }
    }
}

#[derive(Debug, Clone)]
pub struct RulesEvaluation {
    pub decision: EvaluationDecision,
    pub applied_rules: Vec<String>,
    pub notes: Vec<String>,
    pub added_tags: Vec<String>,
    pub metadata_updates: Map<String, Value>,
    pub span: Span,
}

impl From<EvaluationResponse> for RulesEvaluation {
    fn from(value: EvaluationResponse) -> Self {
        Self {
            decision: value.decision,
            applied_rules: value.applied_rules,
            notes: value.notes,
            added_tags: value.added_tags,
            metadata_updates: value.metadata_updates,
            span: value.span,
        }
    }
}

#[derive(Debug, Error)]
pub enum RulesClientError {
    #[error("invalid rules service url {url}: {source}")]
    InvalidUrl {
        url: String,
        #[source]
        source: url::ParseError,
    },
    #[error("rules HTTP request failed: {0}")]
    Http(String),
    #[error("rules service returned unexpected status {status}")]
    UnexpectedStatus { status: reqwest::StatusCode },
    #[error("failed to decode rules response: {0}")]
    Decode(String),
}
