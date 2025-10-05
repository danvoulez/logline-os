use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{OriginalUri, State};
use axum::http::{self, HeaderMap, Method, StatusCode, Uri};
use axum::response::Response;
use axum::routing::any;
use axum::{Extension, Router};
use http_body_util::BodyExt;
use tokio::time::sleep;
use tracing::{instrument, warn};

use crate::discovery::ServiceDiscovery;
use crate::resilience::ResilienceState;
use crate::security::{AuthContext, SecurityState};

#[derive(Clone)]
pub struct RestProxyState {
    client: reqwest::Client,
    targets: HashMap<String, String>,
    security: Arc<SecurityState>,
    resilience: ResilienceState,
}

impl RestProxyState {
    pub fn new(
        client: reqwest::Client,
        discovery: &ServiceDiscovery,
        security: Arc<SecurityState>,
        resilience: ResilienceState,
    ) -> Self {
        Self {
            client,
            targets: discovery.rest_targets(),
            security,
            resilience,
        }
    }

    fn resolve(&self, uri: &Uri) -> Result<(String, String), StatusCode> {
        let path = uri.path();
        let mut segments = path.trim_start_matches('/').splitn(2, '/');
        let service_key = segments.next().unwrap_or("");

        if service_key.is_empty() {
            return Err(StatusCode::NOT_FOUND);
        }

        let target_base = self
            .targets
            .get(service_key)
            .ok_or(StatusCode::NOT_FOUND)?
            .clone();
        let remainder = segments.next().unwrap_or("");
        let mut forward_url = target_base.trim_end_matches('/').to_string();
        if !remainder.is_empty() {
            if !forward_url.ends_with('/') {
                forward_url.push('/');
            }
            forward_url.push_str(remainder);
        }

        if let Some(query) = uri.query() {
            forward_url.push('?');
            forward_url.push_str(query);
        }

        Ok((service_key.to_string(), forward_url))
    }
}

pub fn router(state: RestProxyState) -> Router {
    Router::new()
        .route("/engine", any(proxy_request))
        .route("/engine/*rest", any(proxy_request))
        .route("/rules", any(proxy_request))
        .route("/rules/*rest", any(proxy_request))
        .route("/timeline", any(proxy_request))
        .route("/timeline/*rest", any(proxy_request))
        .route("/id", any(proxy_request))
        .route("/id/*rest", any(proxy_request))
        .route("/federation", any(proxy_request))
        .route("/federation/*rest", any(proxy_request))
        .with_state(state)
}

#[instrument(skip_all, fields(method = tracing::field::Empty, service = tracing::field::Empty))]
async fn proxy_request(
    State(state): State<RestProxyState>,
    Extension(auth): Extension<AuthContext>,
    method: Method,
    headers: HeaderMap,
    OriginalUri(original_uri): OriginalUri,
    body: Body,
) -> Result<Response, StatusCode> {
    let (service_key, target_url) = state.resolve(&original_uri)?;
    tracing::Span::current().record("method", &tracing::field::display(&method));
    tracing::Span::current().record("service", &tracing::field::display(&service_key));

    state.resilience.before_request(&service_key).await?;

    let req_method = reqwest::Method::from_bytes(method.as_str().as_bytes())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let forwarded_headers = build_forward_headers(&headers)?;

    let body_bytes = body
        .collect()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .to_bytes();
    let body_vec = body_bytes.to_vec();

    let max_retries = state.resilience.config().retry_attempts;
    let mut attempt = 0u32;

    loop {
        let mut builder = state.client.request(req_method.clone(), &target_url);
        for (name, value) in &forwarded_headers {
            builder = builder.header(name.clone(), value.clone());
        }

        if !body_vec.is_empty() {
            builder = builder.body(body_vec.clone());
        }

        builder = state.security.apply_outbound_headers(builder, &auth);

        let response = builder.send().await;
        match response {
            Ok(response) => {
                state.resilience.record_success(&service_key).await;
                return convert_response(response).await;
            }
            Err(err) => {
                let store_dead_letter = attempt >= max_retries;
                state
                    .resilience
                    .record_failure(
                        &service_key,
                        &target_url,
                        &err.to_string(),
                        body_vec.len(),
                        store_dead_letter,
                    )
                    .await;

                warn!(%service_key, ?err, attempt, "falha ao encaminhar requisição REST");
                if attempt >= max_retries {
                    return Err(StatusCode::BAD_GATEWAY);
                }

                attempt += 1;
                let delay = state.resilience.backoff_for_attempt(attempt);
                sleep(delay).await;
            }
        }
    }
}

fn build_forward_headers(
    headers: &HeaderMap,
) -> Result<Vec<(reqwest::header::HeaderName, reqwest::header::HeaderValue)>, StatusCode> {
    let mut result = Vec::new();
    for (name, value) in headers.iter() {
        if name == http::header::HOST || name == http::header::CONTENT_LENGTH {
            continue;
        }

        let header_name = reqwest::header::HeaderName::from_bytes(name.as_str().as_bytes())
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let header_value = reqwest::header::HeaderValue::from_bytes(value.as_bytes())
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        result.push((header_name, header_value));
    }
    Ok(result)
}

async fn convert_response(response: reqwest::Response) -> Result<Response, StatusCode> {
    let status =
        StatusCode::from_u16(response.status().as_u16()).map_err(|_| StatusCode::BAD_GATEWAY)?;
    let mut builder = Response::builder().status(status);

    for (name, value) in response.headers() {
        let name_str = name.as_str();
        if name_str.eq_ignore_ascii_case(http::header::CONTENT_LENGTH.as_str())
            || name_str.eq_ignore_ascii_case(http::header::TRANSFER_ENCODING.as_str())
        {
            continue;
        }

        if let (Ok(header_name), Ok(header_value)) = (
            http::header::HeaderName::from_bytes(name_str.as_bytes()),
            http::HeaderValue::from_bytes(value.as_bytes()),
        ) {
            builder = builder.header(header_name, header_value);
        }
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    builder
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::BAD_GATEWAY)
}
