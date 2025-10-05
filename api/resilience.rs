use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::config::ResilienceConfig;

#[derive(Clone)]
pub struct ResilienceState {
    config: ResilienceConfig,
    inner: Arc<Mutex<ResilienceInner>>,
}

struct ResilienceInner {
    circuits: HashMap<String, CircuitState>,
    dead_letters: VecDeque<DeadLetterRecord>,
}

struct CircuitState {
    failures: u32,
    open_until: Option<Instant>,
}

impl ResilienceState {
    pub fn new(config: ResilienceConfig) -> Self {
        let inner = ResilienceInner {
            circuits: HashMap::new(),
            dead_letters: VecDeque::new(),
        };
        Self {
            config,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn config(&self) -> &ResilienceConfig {
        &self.config
    }

    pub async fn before_request(&self, service: &str) -> Result<(), StatusCode> {
        let mut guard = self.inner.lock().await;
        let entry = guard
            .circuits
            .entry(service.to_string())
            .or_insert_with(|| CircuitState {
                failures: 0,
                open_until: None,
            });

        if let Some(until) = entry.open_until {
            if Instant::now() < until {
                warn!(service, "circuit breaker aberto");
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }

            entry.open_until = None;
            entry.failures = 0;
            info!(service, "circuito em estado half-open");
        }

        Ok(())
    }

    pub async fn record_success(&self, service: &str) {
        let mut guard = self.inner.lock().await;
        if let Some(entry) = guard.circuits.get_mut(service) {
            if entry.failures > 0 {
                debug!(
                    service,
                    failures = entry.failures,
                    "resetando contador de falhas"
                );
            }
            entry.failures = 0;
            entry.open_until = None;
        }
    }

    pub async fn record_failure(
        &self,
        service: &str,
        target: &str,
        error: &str,
        payload_size: usize,
        store_dead_letter: bool,
    ) {
        let mut guard = self.inner.lock().await;
        let entry = guard
            .circuits
            .entry(service.to_string())
            .or_insert_with(|| CircuitState {
                failures: 0,
                open_until: None,
            });
        entry.failures = entry.failures.saturating_add(1);

        if entry.failures >= self.config.circuit_breaker_threshold {
            entry.open_until = Some(Instant::now() + self.config.circuit_breaker_reset);
            warn!(
                service,
                failures = entry.failures,
                "circuit breaker aberto após falhas consecutivas"
            );
        }

        if store_dead_letter {
            let record = DeadLetterRecord {
                id: Uuid::new_v4(),
                service: service.to_string(),
                target: target.to_string(),
                error: error.to_string(),
                occurred_at: Utc::now(),
                payload_size,
            };
            guard.dead_letters.push_front(record);
            while guard.dead_letters.len() > self.config.dead_letter_capacity {
                guard.dead_letters.pop_back();
            }
        }
    }

    pub fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        let exponent = attempt.min(8);
        let multiplier = 2u32.saturating_pow(exponent) as f64;
        let delay = self.config.retry_backoff.mul_f64(multiplier);
        delay.min(Duration::from_secs(5 * 60))
    }

    pub async fn open_circuits(&self) -> Vec<String> {
        let guard = self.inner.lock().await;
        guard
            .circuits
            .iter()
            .filter_map(|(service, state)| {
                state.open_until.and_then(|until| {
                    if Instant::now() < until {
                        Some(service.clone())
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    pub async fn dead_letter_count(&self) -> usize {
        let guard = self.inner.lock().await;
        guard.dead_letters.len()
    }

    pub async fn dead_letters(&self) -> Vec<DeadLetterRecord> {
        let guard = self.inner.lock().await;
        guard.dead_letters.iter().cloned().collect()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DeadLetterRecord {
    pub id: Uuid,
    pub service: String,
    pub target: String,
    pub error: String,
    pub occurred_at: DateTime<Utc>,
    pub payload_size: usize,
}

#[derive(Debug, Serialize)]
struct DeadLetterResponse {
    dead_letters: Vec<DeadLetterRecord>,
}

pub fn router(state: ResilienceState) -> Router {
    Router::new()
        .route("/_system/deadletters", get(list_dead_letters))
        .with_state(state)
}

async fn list_dead_letters(State(state): State<ResilienceState>) -> impl IntoResponse {
    let records = state.dead_letters().await;
    Json(DeadLetterResponse {
        dead_letters: records,
    })
}
