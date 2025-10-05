use std::env;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use logline_core::errors::ConfigError;
use tracing::warn;
use url::Url;

/// REST e WebSocket URLs para um microserviço.
#[derive(Debug, Clone)]
pub struct ServiceUrls {
    pub key: &'static str,
    pub service_name: &'static str,
    pub rest_url: String,
    pub ws_url: Option<String>,
    pub health_path: &'static str,
}

impl ServiceUrls {
    pub fn new(
        key: &'static str,
        service_name: &'static str,
        rest_url: String,
        ws_url: Option<String>,
    ) -> Self {
        Self {
            key,
            service_name,
            rest_url,
            ws_url,
            health_path: "/health",
        }
    }
}

/// Configuração global do gateway carregada a partir das variáveis de ambiente.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub bind_address: String,
    pub engine: ServiceUrls,
    pub rules: ServiceUrls,
    pub timeline: ServiceUrls,
    pub identity: ServiceUrls,
    pub federation: ServiceUrls,
    pub security: SecurityConfig,
    pub resilience: ResilienceConfig,
    pub tls: Option<TlsConfig>,
}

impl GatewayConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_address = env::var("GATEWAY_BIND").unwrap_or_else(|_| "0.0.0.0:8070".to_string());

        let engine_rest = read_http_url("ENGINE_URL", "http://127.0.0.1:8090")?;
        let engine_ws = read_ws_url("ENGINE_WS_URL")?.or_else(|| derive_ws_url(&engine_rest).ok());

        let rules_rest = read_http_url("RULES_URL", "http://127.0.0.1:8081")?;
        let rules_ws = read_ws_url("RULES_WS_URL")?.or_else(|| derive_ws_url(&rules_rest).ok());

        let timeline_rest = read_http_url("TIMELINE_URL", "http://127.0.0.1:8082")?;
        let timeline_ws =
            read_ws_url("TIMELINE_WS_URL")?.or_else(|| derive_ws_url(&timeline_rest).ok());

        let identity_rest = read_http_url("ID_URL", "http://127.0.0.1:8083")?;
        let identity_ws = read_ws_url("ID_WS_URL")?.or_else(|| derive_ws_url(&identity_rest).ok());

        let federation_rest = read_http_url("FEDERATION_URL", "http://127.0.0.1:8084")?;
        let federation_ws =
            read_ws_url("FEDERATION_WS_URL")?.or_else(|| derive_ws_url(&federation_rest).ok());

        let security = SecurityConfig::from_env()?;
        let resilience = ResilienceConfig::from_env()?;
        let tls = TlsConfig::maybe_from_env()?;

        Ok(Self {
            bind_address,
            engine: ServiceUrls::new("engine", "logline-engine", engine_rest, engine_ws),
            rules: ServiceUrls::new("rules", "logline-rules", rules_rest, rules_ws),
            timeline: ServiceUrls::new("timeline", "logline-timeline", timeline_rest, timeline_ws),
            identity: ServiceUrls::new("id", "logline-id", identity_rest, identity_ws),
            federation: ServiceUrls::new(
                "federation",
                "logline-federation",
                federation_rest,
                federation_ws,
            ),
            security,
            resilience,
            tls,
        })
    }

    pub fn services(&self) -> Vec<ServiceUrls> {
        vec![
            self.engine.clone(),
            self.rules.clone(),
            self.timeline.clone(),
            self.identity.clone(),
            self.federation.clone(),
        ]
    }

    pub fn bind_address(&self) -> &str {
        &self.bind_address
    }

    pub fn security(&self) -> &SecurityConfig {
        &self.security
    }

    pub fn resilience(&self) -> &ResilienceConfig {
        &self.resilience
    }

    pub fn tls(&self) -> Option<&TlsConfig> {
        self.tls.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub rate_limit_per_minute: u64,
    pub max_concurrent_requests: usize,
    pub cors_allowed_origins: Vec<String>,
    pub cors_allow_credentials: bool,
    pub service_token: Option<String>,
    pub public_paths: Vec<String>,
}

impl SecurityConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let jwt_secret = env::var("GATEWAY_JWT_SECRET")
            .map_err(|_| ConfigError::MissingEnvVar("GATEWAY_JWT_SECRET".to_string()))?;

        if jwt_secret.trim().is_empty() {
            return Err(ConfigError::MissingEnvVar(
                "GATEWAY_JWT_SECRET precisa ser fornecido".to_string(),
            ));
        }

        let jwt_issuer = env::var("GATEWAY_JWT_ISSUER")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "logline-identity".to_string());

        let jwt_audience = env::var("GATEWAY_JWT_AUDIENCE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "logline".to_string());

        let rate_limit_per_minute = parse_env::<u64>("GATEWAY_RATE_LIMIT_PER_MINUTE", 240)?;
        let max_concurrent_requests = parse_env::<usize>("GATEWAY_MAX_CONCURRENCY", 128)?;

        let cors_allowed_origins = env::var("GATEWAY_ALLOWED_ORIGINS")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .filter_map(|origin| {
                        let trimmed = origin.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["*".to_string()]);

        let cors_allow_credentials = parse_env::<u64>("GATEWAY_ALLOW_CREDENTIALS", 1)? != 0;

        let service_token = env::var("GATEWAY_SERVICE_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let public_paths = env::var("GATEWAY_PUBLIC_PATHS")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .filter_map(|segment| {
                        let trimmed = segment.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["/healthz".to_string()]);

        Ok(Self {
            jwt_secret,
            jwt_issuer,
            jwt_audience,
            rate_limit_per_minute,
            max_concurrent_requests,
            cors_allowed_origins,
            cors_allow_credentials,
            service_token,
            public_paths,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ResilienceConfig {
    pub request_timeout: Duration,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_reset: Duration,
    pub retry_attempts: u32,
    pub retry_backoff: Duration,
    pub dead_letter_capacity: usize,
}

impl ResilienceConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let request_timeout_ms = parse_env::<u64>("GATEWAY_REQUEST_TIMEOUT_MS", 15_000)?;
        let circuit_breaker_threshold = parse_env::<u32>("GATEWAY_CIRCUIT_FAILURE_THRESHOLD", 5)?;
        let circuit_breaker_reset_secs = parse_env::<u64>("GATEWAY_CIRCUIT_RESET_SECS", 30)?;
        let retry_attempts = parse_env::<u32>("GATEWAY_RETRY_ATTEMPTS", 2)?;
        let retry_backoff_ms = parse_env::<u64>("GATEWAY_RETRY_BACKOFF_MS", 200)?;
        let dead_letter_capacity = parse_env::<usize>("GATEWAY_DEAD_LETTER_CAPACITY", 200)?;

        Ok(Self {
            request_timeout: Duration::from_millis(request_timeout_ms.max(100)),
            circuit_breaker_threshold,
            circuit_breaker_reset: Duration::from_secs(circuit_breaker_reset_secs.max(1)),
            retry_attempts,
            retry_backoff: Duration::from_millis(retry_backoff_ms.max(10)),
            dead_letter_capacity: dead_letter_capacity.max(10),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub certificate_path: String,
    pub private_key_path: String,
}

impl TlsConfig {
    fn maybe_from_env() -> Result<Option<Self>, ConfigError> {
        let cert_path = env::var("GATEWAY_TLS_CERT").ok();
        let key_path = env::var("GATEWAY_TLS_KEY").ok();

        match (cert_path, key_path) {
            (Some(cert), Some(key)) => {
                if cert.trim().is_empty() || key.trim().is_empty() {
                    return Err(ConfigError::Internal(
                        "GATEWAY_TLS_CERT e GATEWAY_TLS_KEY precisam ser definidos".into(),
                    ));
                }
                Ok(Some(Self {
                    certificate_path: cert,
                    private_key_path: key,
                }))
            }
            (None, None) => Ok(None),
            _ => Err(ConfigError::Internal(
                "ambos GATEWAY_TLS_CERT e GATEWAY_TLS_KEY são necessários".into(),
            )),
        }
    }

    pub async fn load(&self) -> Result<axum_server::tls_rustls::RustlsConfig, ConfigError> {
        let cert = Path::new(&self.certificate_path);
        let key = Path::new(&self.private_key_path);

        axum_server::tls_rustls::RustlsConfig::from_pem_file(cert, key)
            .await
            .map_err(|err| {
                ConfigError::Internal(format!("falha ao carregar certificados TLS: {err}"))
            })
    }
}

fn read_http_url(key: &'static str, default: &str) -> Result<String, ConfigError> {
    match env::var(key) {
        Ok(value) => sanitize_http_url(key, value.trim()),
        Err(env::VarError::NotPresent) => sanitize_http_url(key, default),
        Err(err) => Err(ConfigError::InvalidEnvVar { key, source: err }),
    }
}

fn read_ws_url(key: &'static str) -> Result<Option<String>, ConfigError> {
    match env::var(key) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                sanitize_ws_url(key, trimmed).map(Some)
            }
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(err) => Err(ConfigError::InvalidEnvVar { key, source: err }),
    }
}

fn sanitize_http_url(key: &'static str, value: &str) -> Result<String, ConfigError> {
    let parsed = Url::parse(value)
        .map_err(|err| ConfigError::Internal(format!("URL inválida para {key}: {err}")))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(ConfigError::Internal(format!(
            "URL de {key} precisa usar http ou https"
        )));
    }

    Ok(trim_trailing_slash(value))
}

fn sanitize_ws_url(key: &'static str, value: &str) -> Result<String, ConfigError> {
    let parsed = Url::parse(value)
        .map_err(|err| ConfigError::Internal(format!("URL inválida para {key}: {err}")))?;

    if parsed.scheme() != "ws" && parsed.scheme() != "wss" {
        return Err(ConfigError::Internal(format!(
            "URL de {key} precisa usar ws ou wss"
        )));
    }

    Ok(value.to_string())
}

fn derive_ws_url(http_url: &str) -> Result<String, ConfigError> {
    let mut parsed = Url::parse(http_url)
        .map_err(|err| ConfigError::Internal(format!("URL inválida: {err}")))?;
    let scheme = match parsed.scheme() {
        "http" => "ws",
        "https" => "wss",
        other => {
            warn!("esquema {other} não suportado para conversão em WebSocket");
            return Err(ConfigError::Internal(format!(
                "não foi possível derivar URL WebSocket a partir de {http_url}"
            )));
        }
    };

    parsed
        .set_scheme(scheme)
        .map_err(|_| ConfigError::Internal("falha ao definir esquema de WebSocket".into()))?;

    let mut path = parsed.path().trim_end_matches('/').to_string();
    if path.is_empty() {
        path.push('/');
    }
    if !path.ends_with('/') {
        path.push('/');
    }
    path.push_str("ws/service");
    parsed.set_path(&path);
    parsed.set_query(None);
    parsed.set_fragment(None);

    Ok(parsed.to_string())
}

fn trim_trailing_slash(value: &str) -> String {
    if value.ends_with('/') {
        value.trim_end_matches('/').to_string()
    } else {
        value.to_string()
    }
}

fn parse_env<T>(key: &'static str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(default)
            } else {
                T::from_str(trimmed).map_err(|err| {
                    ConfigError::Internal(format!("valor inválido para {key}: {err}"))
                })
            }
        }
        Err(env::VarError::NotPresent) => Ok(default),
        Err(err) => Err(ConfigError::InvalidEnvVar { key, source: err }),
    }
}
