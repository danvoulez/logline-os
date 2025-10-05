use std::env;

use crate::errors::{ConfigError, LogLineError};

/// Runtime environment used by the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    fn from_str(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "production" | "prod" => Environment::Production,
            "staging" | "stage" => Environment::Staging,
            _ => Environment::Development,
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Development
    }
}

/// Global configuration shared across the services.
#[derive(Debug, Clone)]
pub struct CoreConfig {
    pub database_url: String,
    pub redis_url: Option<String>,
    pub environment: Environment,
    pub node_name: String,
    pub websocket_bind: Option<String>,
    pub http_bind: Option<String>,
}

impl CoreConfig {
    /// Loads configuration from the process environment.
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingEnvVar("DATABASE_URL".into()))?;

        let redis_url = env::var("REDIS_URL").ok();
        let environment = env::var("LOGLINE_ENV")
            .map(|raw| Environment::from_str(&raw))
            .unwrap_or_default();

        let node_name =
            env::var("LOGLINE_NODE_NAME").unwrap_or_else(|_| "logline-node".to_string());
        let websocket_bind = env::var("LOGLINE_WS_BIND").ok();
        let http_bind = env::var("LOGLINE_HTTP_BIND").ok();

        Ok(Self {
            database_url,
            redis_url,
            environment,
            node_name,
            websocket_bind,
            http_bind,
        })
    }

    /// Loads configuration from env vars prefixed with the provided value (e.g. `TIMELINE_`).
    pub fn from_env_with_prefix(prefix: &str) -> Result<Self, ConfigError> {
        let key = |suffix: &str| format!("{}{}", prefix, suffix);

        let db_key = key("DATABASE_URL");
        let database_url =
            env::var(&db_key).map_err(|_| ConfigError::MissingEnvVar(db_key.clone()))?;

        let redis_key = key("REDIS_URL");
        let redis_url = env::var(&redis_key).ok();

        let env_key = key("ENV");
        let environment = env::var(&env_key)
            .map(|raw| Environment::from_str(&raw))
            .unwrap_or_default();

        let node_key = key("NODE_NAME");
        let node_name = env::var(&node_key).unwrap_or_else(|_| "logline-node".to_string());

        let ws_bind = key("WS_BIND");
        let websocket_bind = env::var(&ws_bind).ok();

        let http_bind_key = key("HTTP_BIND");
        let http_bind = env::var(&http_bind_key).ok();

        Ok(Self {
            database_url,
            redis_url,
            environment,
            node_name,
            websocket_bind,
            http_bind,
        })
    }

    /// Returns the base Postgres URL.
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    /// Whether the service is running in production.
    pub fn is_production(&self) -> bool {
        matches!(self.environment, Environment::Production)
    }

    /// Returns redis URL if configured.
    pub fn redis_url(&self) -> Option<&str> {
        self.redis_url.as_deref()
    }
}

/// Helper that loads config and converts to the canonical LogLine error type.
pub fn load_core_config() -> Result<CoreConfig, LogLineError> {
    Ok(CoreConfig::from_env()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_default_environment() {
        std::env::remove_var("LOGLINE_ENV");
        std::env::set_var("DATABASE_URL", "postgres://example");
        let cfg = CoreConfig::from_env().expect("config should load");
        assert_eq!(cfg.environment, Environment::Development);
    }
}
