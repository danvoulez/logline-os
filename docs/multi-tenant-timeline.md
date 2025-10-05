# Multi-tenant Timeline Integration for LogLine

This module integrates the timeline system with multi-tenant infrastructure in the LogLine project.

## Features

- **Tenant-aware spans**: Create and manage timeline spans with tenant context
- **Organization-based visibility**: Control access to spans with organization-level permissions
- **Tenant isolation**: Database-level isolation for tenant data with row-level security
- **Tenant statistics**: Track usage and activity per tenant
- **Tenant-specific views**: Database views for efficient tenant data access

## Architecture

The tenant-aware timeline system consists of several components:

1. **Database Schema**: New columns and constraints for tenant data in the timeline_spans table
2. **SQL Functions**: PostgreSQL functions for tenant-specific operations
3. **Timeline API**: Rust APIs for tenant-specific timeline operations
4. **Engine Integration**: Tenant context in the LogLine engine for span creation

## API Usage

### Creating a Tenant-Aware Timeline

```rust
// Create a base timeline
let timeline = Timeline::new(database_url).await?;

// Create a tenant-specific timeline
let tenant_timeline = create_tenant_timeline(
    &timeline,
    "my-tenant-id",
    Some(user_id),
    Some(organization_id)
);

// Create a span in the tenant context
let span_id = tenant_timeline.create_span(
    "My Tenant Span",
    &id_with_keys,
    serde_json::json!({ "key": "value" }),
    Some(TenantSpanOptions {
        visibility: Some(Visibility::Organization),
        ..Default::default()
    })
).await?;

// Get tenant statistics
let stats = tenant_timeline.get_stats().await?;
```

### Using Tenant Context in Engine

```rust
// Create an engine with tenant context
let engine = Engine::new()
    .with_logline_id("logline-id://node-name")
    .with_tenant_context(
        "my-tenant-id", 
        Some(organization_id),
        Some(user_id)
    );

// Create a span with the engine (tenant context applied automatically)
let span = engine.create_span("My action", Some(payload));

// Execute a contract (tenant context applied automatically)
let result = engine.execute(&contract);
```

## Database Schema

The timeline_spans table has been extended with the following fields:

- `tenant_id`: References the tenant/organization ID (UUID)
- `organization_id`: References the specific organization within the tenant (UUID)
- `user_id`: References the user who created the span (UUID)
- `span_type`: Type of span ('user', 'system', 'organization', 'ghost')
- `visibility`: Access control level ('private', 'organization', 'public')
- `metadata`: Additional tenant-specific metadata (JSONB)
- `verification_status`: Status of span verification ('verified', 'pending', 'failed')
- `delta_s`: Reputation/effort score for the span (FLOAT)
- `replay_count`: Number of times the span has been replayed (INTEGER)
- `replay_from`: References the origin span for replays (UUID)

## SQL Functions

- `get_tenant_timeline`: Get spans for a specific tenant with access control
- `create_tenant_span`: Create a new span with tenant context and validation
- `get_tenant_stats`: Get usage statistics for a specific tenant
- `auto_populate_tenant_context`: Trigger to automatically fill tenant context

## Future Work

- Add tenant-aware federation support
- Implement tenant-specific replay mechanisms
- Add tenant migration and history tracking
- Implement cross-tenant authentication flows

## License

This code is part of the LogLine project.