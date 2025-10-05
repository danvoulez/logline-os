use chrono::{DateTime, Utc};
use logline_core::config::CoreConfig;
use logline_core::db::DatabasePool;
use logline_core::errors::{LogLineError, Result};
use logline_protocol::timeline::{
    Span, SpanStatus, SpanType, TimelineEntry, TimelineQuery, Visibility,
};
use serde_json::Value;
use sqlx::{query_scalar, FromRow, QueryBuilder};
use uuid::Uuid;

/// Database-backed repository for timeline spans.
#[derive(Clone)]
pub struct TimelineRepository {
    pool: DatabasePool,
}

impl TimelineRepository {
    /// Connects to the database using the supplied configuration and ensures migrations ran.
    pub async fn from_config(config: &CoreConfig) -> Result<Self> {
        let pool = DatabasePool::connect(config).await?;
        Self::from_pool(pool).await
    }

    /// Builds the repository from an existing database pool.
    pub async fn from_pool(pool: DatabasePool) -> Result<Self> {
        sqlx::migrate!("../migrations")
            .run(pool.inner())
            .await
            .map_err(|err| LogLineError::TimelineError(err.to_string()))?;
        Ok(Self { pool })
    }

    /// Resolves a tenant identifier that may be expressed either as an UUID or as an organization alias.
    pub async fn resolve_tenant_key(&self, tenant_id: &str) -> Result<Uuid> {
        if let Ok(uuid) = Uuid::parse_str(tenant_id) {
            return Ok(uuid);
        }

        let resolved = query_scalar::<_, Uuid>("SELECT id FROM organizations WHERE tenant_id = $1")
            .bind(tenant_id)
            .fetch_optional(self.pool.inner())
            .await?;

        resolved.ok_or_else(|| {
            LogLineError::TimelineError(format!("tenant `{tenant_id}` not found in organizations"))
        })
    }

    /// Inserts a new span into the timeline and returns the stored representation.
    pub async fn create_span(&self, tenant_id: &str, span: Span) -> Result<TimelineEntry> {
        let tenant_uuid = self.resolve_tenant_key(tenant_id).await?;

        let payload = span
            .data
            .clone()
            .unwrap_or_else(|| Value::Object(Default::default()));
        let metadata = span
            .metadata
            .clone()
            .unwrap_or_else(|| Value::Object(Default::default()));
        let organization_id = span.organization_id.or(Some(tenant_uuid));
        let span_type = span.span_type.map(Self::span_type_to_str);
        let visibility = span.visibility.map(Self::visibility_to_str);

        let row = sqlx::query_as::<_, TimelineSpanRow>(
            r#"
            INSERT INTO timeline_spans (
                id, timestamp, logline_id, author, title, payload,
                contract_id, workflow_id, flow_id, caused_by, signature,
                status, verification_status, delta_s, replay_count, replay_from,
                tenant_id, organization_id, user_id, span_type, visibility, metadata
            ) VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16,
                $17, $18, $19, $20, $21, $22
            )
            RETURNING
                id, timestamp, logline_id, author, title, payload,
                contract_id, workflow_id, flow_id, caused_by, signature,
                status, verification_status, delta_s, replay_count, replay_from,
                tenant_id, organization_id, user_id, span_type, visibility, metadata,
                created_at, updated_at
            "#,
        )
        .bind(span.id)
        .bind(span.timestamp)
        .bind(&span.logline_id)
        .bind(&span.logline_id)
        .bind(&span.title)
        .bind(payload)
        .bind(&span.contract_id)
        .bind(&span.workflow_id)
        .bind(&span.flow_id)
        .bind(&span.caused_by)
        .bind(
            span.signature
                .clone()
                .unwrap_or_else(|| "unsigned".to_string()),
        )
        .bind(Self::status_to_str(span.status))
        .bind(
            span.verification_status
                .clone()
                .unwrap_or_else(|| "verified".to_string()),
        )
        .bind(span.delta_s.unwrap_or(0.0))
        .bind(span.replay_count.map(|value| value as i32).unwrap_or(0))
        .bind(&span.replay_from)
        .bind(tenant_uuid)
        .bind(organization_id)
        .bind(span.user_id)
        .bind(span_type)
        .bind(visibility)
        .bind(metadata)
        .fetch_one(self.pool.inner())
        .await?;

        Ok(row.into())
    }

    /// Fetches a span by its identifier applying tenant isolation.
    pub async fn get_span(&self, tenant_id: &str, id: Uuid) -> Result<Option<TimelineEntry>> {
        let tenant_uuid = self.resolve_tenant_key(tenant_id).await?;
        let row = sqlx::query_as::<_, TimelineSpanRow>(
            r#"
            SELECT
                id, timestamp, logline_id, author, title, payload,
                contract_id, workflow_id, flow_id, caused_by, signature,
                status, verification_status, delta_s, replay_count, replay_from,
                tenant_id, organization_id, user_id, span_type, visibility, metadata,
                created_at, updated_at
            FROM timeline_spans
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_uuid)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(row.map(Into::into))
    }

    /// Lists spans based on the provided query filters for a specific tenant.
    pub async fn list_spans(
        &self,
        tenant_id: &str,
        query: &TimelineQuery,
    ) -> Result<Vec<TimelineEntry>> {
        let tenant_uuid = self.resolve_tenant_key(tenant_id).await?;
        let mut builder = QueryBuilder::new(
            "SELECT id, timestamp, logline_id, author, title, payload, \
             contract_id, workflow_id, flow_id, caused_by, signature, \
             status, verification_status, delta_s, replay_count, replay_from, \
             tenant_id, organization_id, user_id, span_type, visibility, metadata, \
             created_at, updated_at FROM timeline_spans WHERE tenant_id = ",
        );
        builder.push_bind(tenant_uuid);

        if let Some(logline_id) = &query.logline_id {
            builder.push(" AND logline_id = ");
            builder.push_bind(logline_id);
        }

        if let Some(contract_id) = &query.contract_id {
            builder.push(" AND contract_id = ");
            builder.push_bind(contract_id);
        }

        if let Some(workflow_id) = &query.workflow_id {
            builder.push(" AND workflow_id = ");
            builder.push_bind(workflow_id);
        }

        if let Some(organization_id) = &query.organization_id {
            builder.push(" AND organization_id = ");
            builder.push_bind(organization_id);
        }

        if let Some(user_id) = &query.user_id {
            builder.push(" AND user_id = ");
            builder.push_bind(user_id);
        }

        if let Some(span_type) = &query.span_type {
            builder.push(" AND span_type = ");
            builder.push_bind(span_type);
        }

        if let Some(visibility) = &query.visibility {
            builder.push(" AND visibility = ");
            builder.push_bind(visibility);
        }

        builder.push(" ORDER BY timestamp DESC");

        if let Some(limit) = query.limit {
            builder.push(" LIMIT ");
            builder.push_bind(limit);
        }

        if let Some(offset) = query.offset {
            builder.push(" OFFSET ");
            builder.push_bind(offset);
        }

        let rows = builder
            .build_query_as::<TimelineSpanRow>()
            .fetch_all(self.pool.inner())
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    fn status_to_str(status: SpanStatus) -> &'static str {
        match status {
            SpanStatus::Executed => "executed",
            SpanStatus::Simulated => "simulated",
            SpanStatus::Reverted => "reverted",
            SpanStatus::Ghost => "ghost",
        }
    }

    fn span_type_to_str(span_type: SpanType) -> &'static str {
        match span_type {
            SpanType::User => "user",
            SpanType::System => "system",
            SpanType::Organization => "organization",
            SpanType::Ghost => "ghost",
        }
    }

    fn visibility_to_str(visibility: Visibility) -> &'static str {
        match visibility {
            Visibility::Private => "private",
            Visibility::Organization => "organization",
            Visibility::Public => "public",
        }
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &DatabasePool {
        &self.pool
    }
}

#[derive(FromRow)]
struct TimelineSpanRow {
    id: Uuid,
    timestamp: DateTime<Utc>,
    logline_id: String,
    author: String,
    title: String,
    payload: Value,
    contract_id: Option<String>,
    workflow_id: Option<String>,
    flow_id: Option<String>,
    caused_by: Option<Uuid>,
    signature: String,
    status: String,
    verification_status: String,
    delta_s: Option<f64>,
    replay_count: Option<i32>,
    #[sqlx(rename = "replay_from")]
    _replay_from: Option<Uuid>,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    user_id: Option<Uuid>,
    span_type: Option<String>,
    visibility: Option<String>,
    metadata: Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<TimelineSpanRow> for TimelineEntry {
    fn from(row: TimelineSpanRow) -> Self {
        TimelineEntry {
            id: row.id,
            timestamp: row.timestamp,
            logline_id: row.logline_id,
            author: row.author,
            title: row.title,
            payload: row.payload,
            contract_id: row.contract_id,
            workflow_id: row.workflow_id,
            flow_id: row.flow_id,
            caused_by: row.caused_by,
            signature: Some(row.signature),
            status: row.status,
            created_at: row.created_at,
            tenant_id: row.tenant_id.map(|uuid| uuid.to_string()),
            organization_id: row.organization_id,
            user_id: row.user_id,
            span_type: row.span_type,
            visibility: row.visibility,
            metadata: Some(row.metadata),
            organization_name: None,
            updated_at: Some(row.updated_at),
            delta_s: row.delta_s,
            replay_count: row.replay_count.map(|value| value as u32),
            verification_status: Some(row.verification_status),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result as AnyResult;
    use logline_core::db::DatabasePool;
    use pg_embed::pg_enums::PgAuthMethod;
    use pg_embed::pg_fetch::{PgFetchSettings, PG_V15};
    use pg_embed::postgres::{PgEmbed, PgSettings};
    use portpicker::pick_unused_port;
    use serde_json::json;
    use std::time::Duration;
    use tempfile::TempDir;

    struct EmbeddedPg {
        instance: PgEmbed,
        _data_dir: TempDir,
        db_name: String,
    }

    impl EmbeddedPg {
        async fn new() -> AnyResult<Self> {
            let data_dir = TempDir::new()?;
            let port = pick_unused_port().expect("unused port");

            let pg_settings = PgSettings {
                database_dir: data_dir.path().to_path_buf(),
                port,
                user: "postgres".into(),
                password: "password".into(),
                auth_method: PgAuthMethod::Plain,
                persistent: false,
                timeout: Some(Duration::from_secs(15)),
                migration_dir: None,
            };

            let fetch_settings = PgFetchSettings {
                version: PG_V15,
                ..Default::default()
            };

            let mut instance = PgEmbed::new(pg_settings, fetch_settings).await?;
            instance.setup().await?;
            instance.start_db().await?;

            Ok(Self {
                instance,
                _data_dir: data_dir,
                db_name: "postgres".to_string(),
            })
        }

        fn database_url(&self) -> String {
            self.instance.full_db_uri(&self.db_name)
        }

        async fn stop(mut self) -> AnyResult<()> {
            self.instance.stop_db().await?;
            Ok(())
        }
    }

    #[tokio::test]
    async fn enforces_tenant_isolation() -> AnyResult<()> {
        let embedded = match EmbeddedPg::new().await {
            Ok(pg) => pg,
            Err(err) => {
                eprintln!("skipping tenant isolation test: {err}");
                return Ok(());
            }
        };
        let database_url = embedded.database_url();
        let pool = DatabasePool::connect_with_url(&database_url).await?;
        sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";")
            .execute(pool.inner())
            .await?;

        let repo = TimelineRepository::from_pool(pool.clone()).await?;

        let tenant_a_alias = "tenant-alpha";
        let tenant_b_alias = "tenant-beta";

        let org_a = insert_organization(&repo, tenant_a_alias).await?;
        let org_b = insert_organization(&repo, tenant_b_alias).await?;

        let mut span_a = Span::new("logline-a", "Tenant A span");
        span_a.tenant_id = Some(tenant_a_alias.to_string());
        span_a.organization_id = Some(org_a);
        span_a.span_type = Some(SpanType::User);
        span_a.visibility = Some(Visibility::Private);
        span_a.metadata = Some(json!({"scope": "alpha"}));

        let mut span_b = Span::new("logline-b", "Tenant B span");
        span_b.tenant_id = Some(tenant_b_alias.to_string());
        span_b.organization_id = Some(org_b);
        span_b.span_type = Some(SpanType::System);
        span_b.visibility = Some(Visibility::Organization);

        let entry_a = repo.create_span(tenant_a_alias, span_a.clone()).await?;
        let entry_b = repo.create_span(tenant_b_alias, span_b.clone()).await?;

        let list_a = repo
            .list_spans(tenant_a_alias, &TimelineQuery::default())
            .await?;
        assert_eq!(list_a.len(), 1);
        assert_eq!(list_a[0].id, entry_a.id);

        let list_b = repo
            .list_spans(tenant_b_alias, &TimelineQuery::default())
            .await?;
        assert_eq!(list_b.len(), 1);
        assert_eq!(list_b[0].id, entry_b.id);

        let cross = repo.get_span(tenant_a_alias, entry_b.id).await?;
        assert!(cross.is_none(), "tenant A should not access tenant B spans");

        let direct = repo
            .get_span(&org_b.to_string(), entry_b.id)
            .await?
            .expect("tenant resolved by uuid");
        assert_eq!(direct.id, entry_b.id);
        assert_eq!(direct.tenant_id, Some(org_b.to_string()));

        embedded.stop().await?;
        Ok(())
    }

    async fn insert_organization(repo: &TimelineRepository, alias: &str) -> AnyResult<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO organizations (id, tenant_id, name, display_name) VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(alias)
        .bind(format!("{alias} name"))
        .bind(format!("{alias} display"))
        .execute(repo.pool.inner())
        .await?;
        Ok(id)
    }
}
