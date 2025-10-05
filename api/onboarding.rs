use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use logline_core::identity::LogLineID;
use logline_protocol::timeline::{SpanStatus, SpanType, Visibility};
use reqwest::StatusCode as ReqwestStatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::discovery::ServiceDiscovery;

#[derive(Clone)]
pub struct OnboardingState {
    client: reqwest::Client,
    identity_base: String,
    timeline_base: String,
    sessions: Arc<RwLock<HashMap<Uuid, OnboardingSession>>>,
}

impl OnboardingState {
    pub fn new(
        client: reqwest::Client,
        discovery: &ServiceDiscovery,
    ) -> Result<Self, OnboardingError> {
        let identity_base = discovery
            .endpoint("id")
            .ok_or_else(|| OnboardingError::internal("serviço de identidade não configurado"))?
            .rest_base()
            .to_string();

        let timeline_base = discovery
            .endpoint("timeline")
            .ok_or_else(|| OnboardingError::internal("serviço de timeline não configurado"))?
            .rest_base()
            .to_string();

        Ok(Self {
            client,
            identity_base,
            timeline_base,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn record_span(
        &self,
        session_id: Uuid,
        logline_id: &LogLineID,
        tenant: Option<&str>,
        title: impl Into<String>,
        step: &str,
        payload: serde_json::Value,
        span_type: SpanType,
    ) -> Result<Uuid, OnboardingError> {
        let span_request = TimelineSpanRequest {
            logline_id: serde_json::to_string(logline_id).map_err(|err| {
                OnboardingError::internal(format!("falha ao serializar LogLine ID: {err}"))
            })?,
            title: title.into(),
            status: Some(SpanStatus::Executed),
            data: Some(payload.clone()),
            tenant_id: tenant.map(|t| t.to_string()),
            span_type: Some(span_type),
            visibility: Some(if tenant.is_some() {
                Visibility::Organization
            } else {
                Visibility::Private
            }),
            metadata: Some(json!({
                "session_id": session_id,
                "step": step,
            })),
            tags: Some(vec!["onboarding".to_string(), step.to_string()]),
        };

        let url = format!("{}/v1/spans", self.timeline_base);
        let response = self
            .client
            .post(url)
            .json(&span_request)
            .send()
            .await
            .map_err(|err| {
                tracing::error!(?err, "falha ao enviar span de onboarding para timeline");
                OnboardingError::internal("falha ao registrar evento de onboarding")
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "sem detalhes".to_string());
            warn!(%status, %text, "timeline rejeitou span de onboarding");
            return Err(OnboardingError::internal(
                "timeline rejeitou span de onboarding",
            ));
        }

        let entry: TimelineEntry = response.json().await.map_err(|err| {
            tracing::error!(?err, "falha ao decodificar resposta da timeline");
            OnboardingError::internal("resposta inválida da timeline")
        })?;

        Ok(entry.id)
    }

    async fn with_session_mut<F, T>(&self, id: &Uuid, f: F) -> Result<T, OnboardingError>
    where
        F: FnOnce(&mut OnboardingSession) -> Result<T, OnboardingError>,
    {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| OnboardingError::not_found("sessão de onboarding não encontrada"))?;
        f(session)
    }
}

pub fn router(state: OnboardingState) -> Router {
    Router::new()
        .route("/onboarding/identity", post(create_identity))
        .route("/onboarding/tenant", post(create_tenant))
        .route("/onboarding/assignment", post(assign_identity))
        .route("/onboarding/template", post(select_template))
        .route("/onboarding/purpose", post(declare_purpose))
        .route("/onboarding/run", post(execute_command))
        .route("/onboarding/:session_id", get(get_session))
        .with_state(state)
}

async fn create_identity(
    State(state): State<OnboardingState>,
    Json(payload): Json<CreateIdentityInput>,
) -> Result<Json<CreateIdentityOutput>, OnboardingError> {
    let identity_payload = IdentityServiceRequest {
        node_name: payload.handle.clone(),
        alias: Some(payload.name.clone()),
        tenant_id: payload.tenant_hint.clone(),
        is_org: Some(false),
        set_active: true,
    };

    let url = format!("{}/v1/ids", state.identity_base);
    let response = state
        .client
        .post(url)
        .json(&identity_payload)
        .send()
        .await
        .map_err(|err| {
            tracing::error!(?err, "falha ao contactar serviço de identidade");
            OnboardingError::internal("falha ao contactar serviço de identidade")
        })?;

    if response.status() == ReqwestStatusCode::BAD_REQUEST {
        let details = response
            .text()
            .await
            .unwrap_or_else(|_| "rejeitado".to_string());
        return Err(OnboardingError::bad_request(format!(
            "serviço de identidade rejeitou solicitação: {details}"
        )));
    }

    if !response.status().is_success() {
        let status = response.status();
        let details = response
            .text()
            .await
            .unwrap_or_else(|_| "erro desconhecido".to_string());
        tracing::error!(%status, %details, "falha ao criar identidade");
        return Err(OnboardingError::internal("falha ao criar identidade"));
    }

    let identity_response: IdentityServiceResponse = response.json().await.map_err(|err| {
        tracing::error!(?err, "resposta inválida do serviço de identidade");
        OnboardingError::internal("resposta inválida do serviço de identidade")
    })?;

    let session_id = Uuid::new_v4();
    let logline_id = identity_response.id.clone();

    let mut session = OnboardingSession::new(
        payload.name.clone(),
        payload.handle.clone(),
        payload.ghost,
        identity_response.id,
        identity_response.signing_key.clone(),
    );

    let span_id = state
        .record_span(
            session_id,
            session.identity.logline_id(),
            None,
            format!("Identidade criada para {}", payload.name),
            "identity",
            json!({
                "name": payload.name,
                "handle": payload.handle,
                "ghost": payload.ghost,
            }),
            if payload.ghost {
                SpanType::Ghost
            } else {
                SpanType::User
            },
        )
        .await?;

    session.timeline_entries.push(span_id);

    {
        let mut sessions = state.sessions.write().await;
        sessions.insert(session_id, session);
    }

    info!(%session_id, handle = %payload.handle, "sessão de onboarding inicializada");

    Ok(Json(CreateIdentityOutput {
        session_id,
        handle: payload.handle,
        identity: IdentitySummary {
            name: payload.name,
            ghost: payload.ghost,
            logline_id: logline_id,
            signing_key: identity_response.signing_key,
        },
        timeline_entry_id: span_id,
    }))
}

async fn create_tenant(
    State(state): State<OnboardingState>,
    Json(payload): Json<CreateTenantInput>,
) -> Result<Json<CreateTenantOutput>, OnboardingError> {
    let tenant_id = slugify(&payload.name);

    let logline_id = state
        .with_session_mut(&payload.session_id, |session| {
            if session.tenant.is_some() {
                return Err(OnboardingError::bad_request(
                    "sessão já possui tenant cadastrado",
                ));
            }

            let logline_id = session.identity.logline_id().clone();
            session.tenant = Some(TenantRecord {
                name: payload.name.clone(),
                tenant_id: tenant_id.clone(),
                created_at: Utc::now(),
                assigned: false,
            });
            Ok(logline_id)
        })
        .await?;

    let span_id = state
        .record_span(
            payload.session_id,
            &logline_id,
            Some(&tenant_id),
            format!("Tenant {} criado", payload.name),
            "tenant_created",
            json!({
                "tenant_name": payload.name,
                "tenant_id": tenant_id,
            }),
            SpanType::Organization,
        )
        .await?;

    state
        .with_session_mut(&payload.session_id, |session| {
            session.timeline_entries.push(span_id);
            Ok(())
        })
        .await?;

    Ok(Json(CreateTenantOutput {
        session_id: payload.session_id,
        tenant_id,
        timeline_entry_id: span_id,
    }))
}

async fn assign_identity(
    State(state): State<OnboardingState>,
    Json(payload): Json<AssignIdentityInput>,
) -> Result<Json<AssignIdentityOutput>, OnboardingError> {
    let (logline_id, signing_key, tenant) = state
        .with_session_mut(&payload.session_id, |session| {
            if session.identity.handle != payload.handle {
                return Err(OnboardingError::bad_request(
                    "handle informado não corresponde à sessão",
                ));
            }

            let tenant = session
                .tenant
                .as_mut()
                .ok_or_else(|| OnboardingError::bad_request("nenhum tenant cadastrado"))?;

            if tenant.tenant_id != payload.tenant_id {
                return Err(OnboardingError::bad_request(
                    "tenant informado não corresponde ao cadastrado",
                ));
            }

            tenant.assigned = true;
            let jwt = issue_token(
                session.identity.logline_id(),
                &session.identity.signing_key,
                Some(&tenant.tenant_id),
            )?;
            session.jwt = Some(jwt.clone());
            Ok((
                session.identity.logline_id().clone(),
                session.identity.signing_key.clone(),
                (tenant.tenant_id.clone(), jwt),
            ))
        })
        .await?;

    let span_id = state
        .record_span(
            payload.session_id,
            &logline_id,
            Some(&tenant.0),
            format!("Identidade {} atribuída ao tenant", payload.handle),
            "tenant_assigned",
            json!({
                "handle": payload.handle,
                "tenant_id": tenant.0,
            }),
            SpanType::Organization,
        )
        .await?;

    state
        .with_session_mut(&payload.session_id, |session| {
            session.timeline_entries.push(span_id);
            Ok(())
        })
        .await?;

    Ok(Json(AssignIdentityOutput {
        session_id: payload.session_id,
        tenant_id: tenant.0,
        jwt: tenant.1,
        timeline_entry_id: span_id,
        signing_key,
    }))
}

async fn select_template(
    State(state): State<OnboardingState>,
    Json(payload): Json<SelectTemplateInput>,
) -> Result<Json<SelectTemplateOutput>, OnboardingError> {
    let (logline_id, tenant_id) = state
        .with_session_mut(&payload.session_id, |session| {
            let tenant = session
                .tenant
                .as_ref()
                .ok_or_else(|| OnboardingError::bad_request("nenhum tenant cadastrado"))?;
            if !tenant.assigned {
                return Err(OnboardingError::bad_request(
                    "identidade ainda não atribuída ao tenant",
                ));
            }
            session.template = Some(TemplateRecord {
                template: payload.template.clone(),
                owner: payload
                    .owner
                    .clone()
                    .unwrap_or_else(|| session.identity.handle.clone()),
                initialized_at: Utc::now(),
            });
            Ok((
                session.identity.logline_id().clone(),
                tenant.tenant_id.clone(),
            ))
        })
        .await?;

    let span_id = state
        .record_span(
            payload.session_id,
            &logline_id,
            Some(&tenant_id),
            format!("Template {} inicializado", payload.template),
            "template",
            json!({
                "template": payload.template,
                "owner": payload.owner,
            }),
            SpanType::System,
        )
        .await?;

    state
        .with_session_mut(&payload.session_id, |session| {
            session.timeline_entries.push(span_id);
            Ok(())
        })
        .await?;

    Ok(Json(SelectTemplateOutput {
        session_id: payload.session_id,
        template: payload.template,
        timeline_entry_id: span_id,
    }))
}

async fn declare_purpose(
    State(state): State<OnboardingState>,
    Json(payload): Json<DeclarePurposeInput>,
) -> Result<Json<DeclarePurposeOutput>, OnboardingError> {
    let (logline_id, tenant_id) = state
        .with_session_mut(&payload.session_id, |session| {
            if session.template.is_none() {
                return Err(OnboardingError::bad_request("template não inicializado"));
            }
            let tenant = session
                .tenant
                .as_ref()
                .ok_or_else(|| OnboardingError::bad_request("nenhum tenant cadastrado"))?;
            session.purpose = Some(PurposeRecord {
                app: payload.app.clone(),
                description: payload.description.clone(),
                declared_at: Utc::now(),
            });
            Ok((
                session.identity.logline_id().clone(),
                tenant.tenant_id.clone(),
            ))
        })
        .await?;

    let span_id = state
        .record_span(
            payload.session_id,
            &logline_id,
            Some(&tenant_id),
            format!("Propósito declarado para {}", payload.app),
            "purpose",
            json!({
                "app": payload.app,
                "description": payload.description,
            }),
            SpanType::System,
        )
        .await?;

    state
        .with_session_mut(&payload.session_id, |session| {
            session.timeline_entries.push(span_id);
            Ok(())
        })
        .await?;

    Ok(Json(DeclarePurposeOutput {
        session_id: payload.session_id,
        timeline_entry_id: span_id,
    }))
}

async fn execute_command(
    State(state): State<OnboardingState>,
    Json(payload): Json<ExecuteCommandInput>,
) -> Result<Json<ExecuteCommandOutput>, OnboardingError> {
    let (logline_id, tenant_id, command_record) = state
        .with_session_mut(&payload.session_id, |session| {
            let tenant = session
                .tenant
                .as_ref()
                .ok_or_else(|| OnboardingError::bad_request("nenhum tenant cadastrado"))?;
            let command = CommandRecord {
                command: payload.command.clone(),
                executed_at: Utc::now(),
            };
            session.commands.push(command.clone());
            Ok((
                session.identity.logline_id().clone(),
                tenant.tenant_id.clone(),
                command,
            ))
        })
        .await?;

    let span_id = state
        .record_span(
            payload.session_id,
            &logline_id,
            Some(&tenant_id),
            "Comando computável executado",
            "execution",
            json!({
                "command": payload.command,
            }),
            SpanType::User,
        )
        .await?;

    state
        .with_session_mut(&payload.session_id, |session| {
            session.timeline_entries.push(span_id);
            Ok(())
        })
        .await?;

    Ok(Json(ExecuteCommandOutput {
        session_id: payload.session_id,
        executed_at: command_record.executed_at,
        timeline_entry_id: span_id,
    }))
}

async fn get_session(
    State(state): State<OnboardingState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<OnboardingSessionSnapshot>, OnboardingError> {
    let sessions = state.sessions.read().await;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| OnboardingError::not_found("sessão de onboarding não encontrada"))?;

    Ok(Json(session.snapshot(session_id)))
}

#[derive(Debug, Clone)]
struct OnboardingSession {
    identity: IdentityRecord,
    tenant: Option<TenantRecord>,
    template: Option<TemplateRecord>,
    purpose: Option<PurposeRecord>,
    commands: Vec<CommandRecord>,
    timeline_entries: Vec<Uuid>,
    jwt: Option<String>,
}

impl OnboardingSession {
    fn new(
        name: String,
        handle: String,
        ghost: bool,
        logline_id: LogLineID,
        signing_key: String,
    ) -> Self {
        Self {
            identity: IdentityRecord {
                name,
                handle,
                ghost,
                logline_id,
                signing_key,
            },
            tenant: None,
            template: None,
            purpose: None,
            commands: Vec::new(),
            timeline_entries: Vec::new(),
            jwt: None,
        }
    }

    fn snapshot(&self, session_id: Uuid) -> OnboardingSessionSnapshot {
        OnboardingSessionSnapshot {
            session_id,
            identity: IdentitySummary {
                name: self.identity.name.clone(),
                ghost: self.identity.ghost,
                logline_id: self.identity.logline_id.clone(),
                signing_key: self.identity.signing_key.clone(),
            },
            handle: self.identity.handle.clone(),
            tenant: self.tenant.clone().map(|tenant| TenantSummary {
                name: tenant.name,
                tenant_id: tenant.tenant_id,
                assigned: tenant.assigned,
                created_at: tenant.created_at,
            }),
            template: self.template.clone().map(|template| TemplateSummary {
                template: template.template,
                owner: template.owner,
                initialized_at: template.initialized_at,
            }),
            purpose: self.purpose.clone().map(|purpose| PurposeSummary {
                app: purpose.app,
                description: purpose.description,
                declared_at: purpose.declared_at,
            }),
            commands: self.commands.clone(),
            timeline_entries: self.timeline_entries.clone(),
            jwt: self.jwt.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct IdentityRecord {
    name: String,
    handle: String,
    ghost: bool,
    logline_id: LogLineID,
    signing_key: String,
}

impl IdentityRecord {
    fn logline_id(&self) -> &LogLineID {
        &self.logline_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CommandRecord {
    command: String,
    executed_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct TenantRecord {
    name: String,
    tenant_id: String,
    created_at: chrono::DateTime<Utc>,
    assigned: bool,
}

#[derive(Debug, Clone)]
struct TemplateRecord {
    template: String,
    owner: String,
    initialized_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct PurposeRecord {
    app: String,
    description: String,
    declared_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateIdentityInput {
    pub name: String,
    pub handle: String,
    #[serde(default)]
    pub ghost: bool,
    #[serde(default)]
    pub tenant_hint: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateIdentityOutput {
    pub session_id: Uuid,
    pub handle: String,
    pub identity: IdentitySummary,
    pub timeline_entry_id: Uuid,
}

#[derive(Debug, Serialize, Clone)]
struct IdentitySummary {
    pub name: String,
    pub ghost: bool,
    pub logline_id: LogLineID,
    pub signing_key: String,
}

#[derive(Debug, Deserialize)]
struct CreateTenantInput {
    pub session_id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
struct CreateTenantOutput {
    pub session_id: Uuid,
    pub tenant_id: String,
    pub timeline_entry_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct AssignIdentityInput {
    pub session_id: Uuid,
    pub handle: String,
    pub tenant_id: String,
}

#[derive(Debug, Serialize)]
struct AssignIdentityOutput {
    pub session_id: Uuid,
    pub tenant_id: String,
    pub jwt: String,
    pub timeline_entry_id: Uuid,
    pub signing_key: String,
}

#[derive(Debug, Deserialize)]
struct SelectTemplateInput {
    pub session_id: Uuid,
    pub template: String,
    pub owner: Option<String>,
}

#[derive(Debug, Serialize)]
struct SelectTemplateOutput {
    pub session_id: Uuid,
    pub template: String,
    pub timeline_entry_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct DeclarePurposeInput {
    pub session_id: Uuid,
    pub app: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
struct DeclarePurposeOutput {
    pub session_id: Uuid,
    pub timeline_entry_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct ExecuteCommandInput {
    pub session_id: Uuid,
    pub command: String,
}

#[derive(Debug, Serialize)]
struct ExecuteCommandOutput {
    pub session_id: Uuid,
    pub executed_at: chrono::DateTime<Utc>,
    pub timeline_entry_id: Uuid,
}

#[derive(Debug, Serialize)]
struct OnboardingSessionSnapshot {
    pub session_id: Uuid,
    pub handle: String,
    pub identity: IdentitySummary,
    pub tenant: Option<TenantSummary>,
    pub template: Option<TemplateSummary>,
    pub purpose: Option<PurposeSummary>,
    pub commands: Vec<CommandRecord>,
    pub timeline_entries: Vec<Uuid>,
    pub jwt: Option<String>,
}

#[derive(Debug, Serialize)]
struct TenantSummary {
    pub name: String,
    pub tenant_id: String,
    pub assigned: bool,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct TemplateSummary {
    pub template: String,
    pub owner: String,
    pub initialized_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PurposeSummary {
    pub app: String,
    pub description: String,
    pub declared_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct TimelineSpanRequest {
    pub logline_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SpanStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_type: Option<SpanType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct IdentityServiceRequest {
    node_name: String,
    alias: Option<String>,
    tenant_id: Option<String>,
    is_org: Option<bool>,
    set_active: bool,
}

#[derive(Debug, Deserialize)]
struct IdentityServiceResponse {
    id: LogLineID,
    signing_key: String,
}

#[derive(Debug, Deserialize)]
struct TimelineEntry {
    id: Uuid,
}

#[derive(Debug, Error)]
pub enum OnboardingError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Internal(String),
}

impl OnboardingError {
    fn bad_request<M: Into<String>>(message: M) -> Self {
        Self::BadRequest(message.into())
    }

    fn not_found<M: Into<String>>(message: M) -> Self {
        Self::NotFound(message.into())
    }

    fn internal<M: Into<String>>(message: M) -> Self {
        Self::Internal(message.into())
    }
}

impl IntoResponse for OnboardingError {
    fn into_response(self) -> axum::response::Response {
        match self {
            OnboardingError::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, Json(json!({"error": message}))).into_response()
            }
            OnboardingError::NotFound(message) => {
                (StatusCode::NOT_FOUND, Json(json!({"error": message}))).into_response()
            }
            OnboardingError::Internal(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": message})),
            )
                .into_response(),
        }
    }
}

fn issue_token(
    logline_id: &LogLineID,
    signing_key: &str,
    tenant_id: Option<&str>,
) -> Result<String, OnboardingError> {
    let key_bytes = URL_SAFE_NO_PAD
        .decode(signing_key.as_bytes())
        .map_err(|err| OnboardingError::internal(format!("chave de assinatura inválida: {err}")))?;

    use jsonwebtoken::{Algorithm, EncodingKey, Header};

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(&key_bytes);

    let now = Utc::now();
    let claims = TokenClaims {
        sub: logline_id.id.to_string(),
        handle: logline_id.node_name.clone(),
        tenant: tenant_id.map(|t| t.to_string()),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(12)).timestamp() as usize,
        iss: "logline-gateway".to_string(),
    };

    jsonwebtoken::encode(&header, &claims, &encoding_key)
        .map_err(|err| OnboardingError::internal(format!("falha ao emitir token: {err}")))
}

#[derive(Debug, Serialize)]
struct TokenClaims {
    sub: String,
    handle: String,
    tenant: Option<String>,
    iat: usize,
    exp: usize,
    iss: String,
}

fn slugify(value: &str) -> String {
    let mut result = String::new();
    let mut previous_hyphen = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            previous_hyphen = false;
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            if !previous_hyphen && !result.is_empty() {
                result.push('-');
                previous_hyphen = true;
            }
        }
    }

    if result.ends_with('-') {
        result.pop();
    }

    if result.is_empty() {
        "tenant".to_string()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_converts_strings() {
        assert_eq!(slugify("VoulezVous"), "voulezvous");
        assert_eq!(slugify("Voulez Vous"), "voulez-vous");
        assert_eq!(slugify("  *Complex Tenant!*  "), "complex-tenant");
        assert_eq!(slugify(""), "tenant");
    }

    #[test]
    fn issue_token_generates_jwt() {
        let keypair = logline_core::identity::LogLineKeyPair::generate(
            "handle",
            Some("Alias".to_string()),
            None,
            false,
        );
        let signing_key = URL_SAFE_NO_PAD.encode(keypair.signing_key.to_bytes());
        let token = issue_token(&keypair.id, &signing_key, Some("tenant")).unwrap();
        assert!(token.split('.').count() == 3);
    }
}
