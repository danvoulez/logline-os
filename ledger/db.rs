use std::time::Duration;

use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

use crate::config::CoreConfig;
use crate::errors::Result;

/// Wrapper around a Postgres connection pool used by most services.
#[derive(Clone)]
pub struct DatabasePool {
    pool: Pool<Postgres>,
}

impl DatabasePool {
    /// Establishes a new connection pool based on the core configuration.
    pub async fn connect(config: &CoreConfig) -> Result<Self> {
        Self::connect_with_url(config.database_url()).await
    }

    /// Establishes a connection pool directly from a database URL.
    pub async fn connect_with_url(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub fn inner(&self) -> &Pool<Postgres> {
        &self.pool
    }
}

/// Trait implemented by services that need to run database migrations.
#[async_trait]
pub trait DatabaseMigrator {
    async fn run_migrations(&self, pool: &DatabasePool) -> Result<()>;
}

/// Run migrations by delegating to the provided migrators.
pub async fn run_migrations(
    pool: &DatabasePool,
    migrators: &[Box<dyn DatabaseMigrator + Send + Sync>],
) -> Result<()> {
    for migrator in migrators {
        migrator.run_migrations(pool).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeMigrator;

    #[async_trait]
    impl DatabaseMigrator for FakeMigrator {
        async fn run_migrations(&self, _pool: &DatabasePool) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn runs_all_migrators() {
        let pool = DatabasePool {
            pool: PgPoolOptions::new()
                .connect_lazy("postgres://localhost/postgres")
                .unwrap(),
        };
        let migrators: Vec<Box<dyn DatabaseMigrator + Send + Sync>> = vec![Box::new(FakeMigrator)];
        run_migrations(&pool, &migrators)
            .await
            .expect("should run migrations");
    }
}
