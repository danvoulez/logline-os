use std::sync::Arc;

use axum::extract::State;
use axum::http::{self, header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use base64::Engine;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use thiserror::Error;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tracing::{info, warn};

use crate::config::SecurityConfig;

/// Contexto autenticado extraído do token JWT do cliente.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub roles: Vec<String>,
    pub issued_at: Option<i64>,
    pub expires_at: Option<i64>,
}

impl AuthContext {
    fn roles_header(&self) -> Option<String> {
        if self.roles.is_empty() {
            None
        } else {
            Some(self.roles.join(" "))
        }
    }
}

#[derive(Clone)]
pub struct SecurityState {
    config: SecurityConfig,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl SecurityState {
    pub fn new(config: SecurityConfig) -> Self {
        let secret_bytes = decode_secret(&config.jwt_secret);
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.set_audience(&[config.jwt_audience.clone()]);
        validation.set_issuer(&[config.jwt_issuer.clone()]);

        Self {
            decoding_key: DecodingKey::from_secret(&secret_bytes),
            validation,
            config,
        }
    }

    pub fn cors_layer(&self) -> CorsLayer {
        let origins: Vec<_> = self
            .config
            .cors_allowed_origins
            .iter()
            .filter_map(|origin| match origin.parse::<HeaderValue>() {
                Ok(value) => Some(value),
                Err(err) => {
                    warn!(%origin, ?err, "origem inválida configurada para CORS");
                    None
                }
            })
            .collect();

        let allow_origin = if origins.is_empty() {
            AllowOrigin::any()
        } else {
            AllowOrigin::list(origins)
        };

        let methods = vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ];

        let headers = vec![
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ORIGIN,
        ];

        CorsLayer::new()
            .allow_methods(AllowMethods::list(methods))
            .allow_origin(allow_origin)
            .allow_headers(AllowHeaders::list(headers))
            .allow_credentials(self.config.cors_allow_credentials)
    }

    pub fn max_concurrent_requests(&self) -> usize {
        self.config.max_concurrent_requests
    }

    pub fn rate_limit_per_minute(&self) -> u64 {
        self.config.rate_limit_per_minute
    }

    pub fn is_public_path(&self, method: &Method, path: &str) -> bool {
        if *method == Method::OPTIONS {
            return true;
        }

        self.config
            .public_paths
            .iter()
            .any(|prefix| path.starts_with(prefix))
    }

    pub fn validate_token(&self, token: &str) -> Result<AuthContext, SecurityError> {
        let data = decode::<AuthClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|err| SecurityError::InvalidToken(err.to_string()))?;
        let claims = data.claims;
        let roles = claims
            .scope
            .unwrap_or_default()
            .split_whitespace()
            .map(|value| value.to_string())
            .collect();

        Ok(AuthContext {
            user_id: claims.sub,
            tenant_id: claims.tenant,
            roles,
            issued_at: claims.iat,
            expires_at: claims.exp,
        })
    }

    pub fn apply_outbound_headers(
        &self,
        mut builder: reqwest::RequestBuilder,
        auth: &AuthContext,
    ) -> reqwest::RequestBuilder {
        if let Some(token) = &self.config.service_token {
            builder = builder.header("X-Service-Token", token.as_str());
        }

        if let Err(err) = HeaderValue::from_str(&auth.user_id) {
            warn!(user_id = %auth.user_id, ?err, "valor inválido para header X-User-ID");
        } else {
            builder = builder.header("X-User-ID", auth.user_id.as_str());
        }

        if let Some(tenant) = &auth.tenant_id {
            if let Err(err) = HeaderValue::from_str(tenant) {
                warn!(tenant = %tenant, ?err, "valor inválido para header X-Tenant-ID");
            } else {
                builder = builder.header("X-Tenant-ID", tenant.as_str());
            }
        }

        if let Some(roles) = auth.roles_header() {
            if let Err(err) = HeaderValue::from_str(&roles) {
                warn!(roles = %roles, ?err, "valor inválido para header X-User-Roles");
            } else {
                builder = builder.header("X-User-Roles", roles);
            }
        }

        builder
    }

    pub fn audit_failure(&self, reason: &SecurityError, path: &str) {
        warn!(?reason, path, "falha de autenticação detectada");
    }
}

pub async fn enforce_auth<B>(
    State(state): State<Arc<SecurityState>>,
    mut request: axum::http::Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();
    let method = request.method().clone();

    if state.is_public_path(&method, &path) {
        return Ok(next.run(request).await);
    }

    let token = extract_bearer(request.headers()).ok_or_else(|| {
        let err = SecurityError::MissingAuthorization;
        state.audit_failure(&err, &path);
        StatusCode::UNAUTHORIZED
    })?;

    let context = match state.validate_token(&token) {
        Ok(context) => context,
        Err(err) => {
            state.audit_failure(&err, &path);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    info!(user_id = %context.user_id, path, "requisição autenticada");
    request.extensions_mut().insert(context);

    Ok(next.run(request).await)
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| value.trim().to_string())
}

fn decode_secret(secret: &str) -> Vec<u8> {
    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(secret) {
        if !decoded.is_empty() {
            return decoded;
        }
    }

    secret.as_bytes().to_vec()
}

#[derive(Debug, Deserialize)]
struct AuthClaims {
    sub: String,
    iss: String,
    aud: String,
    exp: Option<i64>,
    iat: Option<i64>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    tenant: Option<String>,
}

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("cabeçalho Authorization ausente")]
    MissingAuthorization,
    #[error("token JWT inválido: {0}")]
    InvalidToken(String),
}
