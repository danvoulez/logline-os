# 🚀 Railway Manual Setup Guide - LogLine Universe

Deploy the LogLine microservices with a single Dockerfile and the minimum configuration required.

## Step 1: Create the Railway Project

1. Go to [railway.app](https://railway.app)
2. Log in with your account
3. Click **"New Project"**
4. Name it **`logline-universe`** (or your preferred name)
5. Optionally add the description **"LogLine Universe - Distributed logging and identity system"**

## Step 2: Provision PostgreSQL

1. Inside the project, click **"Add Service"**
2. Choose **"Database" → "PostgreSQL"**
3. Railway will provision a PostgreSQL 15 instance
4. Keep the generated connection string handy – it will be referenced as `${{Postgres.DATABASE_URL}}`

## Step 3: Deploy the Microservices

All services now share the same root `Dockerfile`. Create one Railway service per microservice and set the build arguments so the image knows which binary to compile.

For each microservice:

1. Click **"Add Service" → "GitHub Repo"** and select **`danvoulez/UniverseLogLine`**
2. **Root Directory**: `.` (repository root)
3. **Dockerfile**: `Dockerfile`
4. Under **Settings → Build → Build Args**, configure the following values:

| Service            | `SERVICE`           | `SERVICE_PORT` | Notes |
|--------------------|--------------------|----------------|-------|
| `logline-id`       | `logline-id`       | `8079`         | Default health path `/health`
| `logline-timeline` | `logline-timeline` | `8080`         | Requires Postgres URL (next step)
| `logline-engine`   | `logline-engine`   | `8082`         | Configure `RULES_URL` to point at the deployed `logline-rules` service |

> ℹ️ Railway applies build args during image build. The default command matches `SERVICE`, so you only need to override `SERVICE_CMD` if you wrap the binary in a custom script.

## Step 4: Configure Environment Variables

Only the timeline needs a database connection. Add the variable to the `logline-timeline` service (or the project-level environment if you prefer a single definition):

```bash
TIMELINE_DATABASE_URL=${{Postgres.DATABASE_URL}}
```

Optional variables:

- `ENGINE_RULES_URL` (engine service) – overrides the default `RULES_URL` when the engine must contact a private rules endpoint.
- `TIMELINE_HTTP_BIND`, `ENGINE_HTTP_BIND`, etc., if you need to override the default bind addresses from code (defaults are already `0.0.0.0`).

## Step 5: Run Database Migrations

Once PostgreSQL is ready:

1. Open the PostgreSQL service → **"Connect"** → **"Query"**
2. Execute the migration files in order:
   - `migrations/001_create_timeline_spans.sql`
   - `migrations/002_implement_multi_tenant_infrastructure.sql`
   - `migrations/003_multi_tenant_timeline_integration.sql`

## Step 6: Verify Deployment

- Each service should report **"Active"** in the Railway dashboard.
- Health endpoints (replace with your generated Railway URLs):

```bash
curl https://logline-id-production.up.railway.app/health
curl https://logline-timeline-production.up.railway.app/health
curl https://logline-engine-production.up.railway.app/health
```

## 🎯 Expected Result

After completing the steps you will have:

- ✅ PostgreSQL + three LogLine microservices running from the same Dockerfile
- ✅ Timeline ledger isolated from rule execution
- ✅ Engine service delegating rule evaluation via HTTP when `RULES_URL` está configurado

## 📊 Service Architecture

```
┌─────────────────┐
│   PostgreSQL    │
│   (Database)    │
└───────┬────────┘
        │
 ┌──────▼──────┐   ┌────────────────┐   ┌────────────────┐
 │ logline-id  │   │ logline-timeline│   │ logline-engine │
 │    :8079    │   │      :8080      │   │      :8082      │
 └─────────────┘   └────────────────┘   └────────────────┘
```

## 🆘 Troubleshooting

- **Build failures**: confirm the correct `SERVICE` build arg, inspect Railway build logs for Cargo errors.
- **Connection issues**: ensure `TIMELINE_DATABASE_URL` is present and migrations ran successfully.
- **Rule evaluation**: verify the engine can reach the rules endpoint defined by `RULES_URL`/`ENGINE_RULES_URL`.

---

**Estimated setup time**: ~10 minutes (Postgres + 3 services)

