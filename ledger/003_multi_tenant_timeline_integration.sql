-- Migration 003: Multi-Tenant Timeline Integration
-- Integrate timeline system with multi-tenant infrastructure

-- Add multi-tenant columns to timeline_spans (if not already added by main migration)
DO $$ 
BEGIN
    -- Check if columns exist before adding them
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'timeline_spans' AND column_name = 'tenant_id') THEN
        ALTER TABLE timeline_spans 
        ADD COLUMN tenant_id UUID REFERENCES organizations(id),
        ADD COLUMN organization_id UUID REFERENCES organizations(id),
        ADD COLUMN user_id UUID, -- References auth.users
        ADD COLUMN span_type TEXT DEFAULT 'user' CHECK (span_type IN ('user', 'system', 'organization', 'ghost')),
        ADD COLUMN visibility TEXT DEFAULT 'private' CHECK (visibility IN ('private', 'organization', 'public')),
        ADD COLUMN metadata JSONB DEFAULT '{}';
    END IF;
    
    -- Add verification columns if they don't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'timeline_spans' AND column_name = 'verification_status') THEN
        ALTER TABLE timeline_spans 
        ADD COLUMN verification_status TEXT DEFAULT 'verified' CHECK (verification_status IN ('verified', 'pending', 'failed')),
        ADD COLUMN delta_s FLOAT DEFAULT 0.0,
        ADD COLUMN replay_count INTEGER DEFAULT 0,
        ADD COLUMN replay_from UUID REFERENCES timeline_spans(id);
    END IF;
    
    -- Add updated_at if it doesn't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_name = 'timeline_spans' AND column_name = 'updated_at') THEN
        ALTER TABLE timeline_spans 
        ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();
    END IF;
END $$;

-- Add missing indexes
CREATE INDEX IF NOT EXISTS idx_timeline_spans_tenant_id ON timeline_spans(tenant_id);
CREATE INDEX IF NOT EXISTS idx_timeline_spans_organization_id ON timeline_spans(organization_id);
CREATE INDEX IF NOT EXISTS idx_timeline_spans_user_id ON timeline_spans(user_id);
CREATE INDEX IF NOT EXISTS idx_timeline_spans_span_type ON timeline_spans(span_type);
CREATE INDEX IF NOT EXISTS idx_timeline_spans_visibility ON timeline_spans(visibility);
CREATE INDEX IF NOT EXISTS idx_timeline_spans_replay_from ON timeline_spans(replay_from);
CREATE INDEX IF NOT EXISTS idx_timeline_spans_verification ON timeline_spans(verification_status);

-- Update the timeline view to include multi-tenant context
DROP VIEW IF EXISTS timeline_view;
CREATE VIEW timeline_view AS
SELECT 
    ts.id,
    ts.timestamp,
    ts.logline_id,
    ts.author,
    ts.title,
    ts.contract_id,
    ts.workflow_id,
    ts.flow_id,
    ts.status,
    ts.span_type,
    ts.visibility,
    ts.verification_status,
    ts.delta_s,
    ts.replay_count,
    -- Organization context
    o.name as organization_name,
    o.tenant_id,
    o.display_name as organization_display_name,
    -- Causation chain
    CASE 
        WHEN ts.caused_by IS NOT NULL THEN 
            (SELECT title FROM timeline_spans WHERE id = ts.caused_by)
        ELSE NULL 
    END as caused_by_title,
    -- Replay chain
    CASE 
        WHEN ts.replay_from IS NOT NULL THEN 
            (SELECT title FROM timeline_spans WHERE id = ts.replay_from)
        ELSE NULL 
    END as replay_from_title,
    ts.created_at,
    ts.updated_at
FROM timeline_spans ts
LEFT JOIN organizations o ON ts.organization_id = o.id
ORDER BY ts.timestamp DESC;

-- Create tenant-specific timeline views
CREATE VIEW organization_timeline_view AS
SELECT 
    tv.*,
    om.role as viewer_role,
    om.permissions as viewer_permissions
FROM timeline_view tv
JOIN organization_members om ON tv.organization_id = om.organization_id
WHERE tv.visibility IN ('organization', 'public')
   OR (tv.visibility = 'private' AND tv.user_id = om.user_id);

-- Create function to get timeline for specific tenant
CREATE OR REPLACE FUNCTION get_tenant_timeline(
    p_tenant_id TEXT,
    p_user_id UUID DEFAULT NULL,
    p_limit INTEGER DEFAULT 100,
    p_offset INTEGER DEFAULT 0
)
RETURNS TABLE(
    id UUID,
    timestamp TIMESTAMPTZ,
    logline_id TEXT,
    author TEXT,
    title TEXT,
    span_type TEXT,
    visibility TEXT,
    organization_name TEXT,
    can_view BOOLEAN
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        ts.id,
        ts.timestamp,
        ts.logline_id,
        ts.author,
        ts.title,
        ts.span_type,
        ts.visibility,
        o.name as organization_name,
        CASE 
            WHEN ts.visibility = 'public' THEN true
            WHEN ts.visibility = 'organization' AND EXISTS(
                SELECT 1 FROM organization_members om2 
                WHERE om2.organization_id = ts.organization_id 
                  AND om2.user_id = p_user_id 
                  AND om2.status = 'active'
            ) THEN true
            WHEN ts.visibility = 'private' AND ts.user_id = p_user_id THEN true
            ELSE false
        END as can_view
    FROM timeline_spans ts
    LEFT JOIN organizations o ON ts.organization_id = o.id
    WHERE (o.tenant_id = p_tenant_id OR (p_tenant_id IS NULL AND o.tenant_id IS NULL))
    ORDER BY ts.timestamp DESC
    LIMIT p_limit OFFSET p_offset;
END;
$$ LANGUAGE plpgsql;

-- Function to create tenant-scoped span
CREATE OR REPLACE FUNCTION create_tenant_span(
    p_tenant_id TEXT,
    p_user_id UUID,
    p_logline_id TEXT,
    p_author TEXT,
    p_title TEXT,
    p_payload JSONB DEFAULT '{}',
    p_contract_id TEXT DEFAULT NULL,
    p_workflow_id TEXT DEFAULT NULL,
    p_flow_id TEXT DEFAULT NULL,
    p_span_type TEXT DEFAULT 'user',
    p_visibility TEXT DEFAULT 'private'
)
RETURNS UUID AS $$
DECLARE
    v_organization_id UUID;
    v_span_id UUID;
BEGIN
    -- Get organization ID from tenant_id
    SELECT id INTO v_organization_id 
    FROM organizations 
    WHERE tenant_id = p_tenant_id AND is_active = true;
    
    IF v_organization_id IS NULL AND p_tenant_id IS NOT NULL THEN
        RAISE EXCEPTION 'Invalid tenant_id: %', p_tenant_id;
    END IF;
    
    -- Verify user has access to this tenant
    IF p_tenant_id IS NOT NULL AND NOT can_access_tenant_data(p_user_id, p_tenant_id) THEN
        RAISE EXCEPTION 'User % does not have access to tenant %', p_user_id, p_tenant_id;
    END IF;
    
    -- Insert the span
    INSERT INTO timeline_spans (
        logline_id, author, title, payload, contract_id, workflow_id, flow_id,
        tenant_id, organization_id, user_id, span_type, visibility
    ) VALUES (
        p_logline_id, p_author, p_title, p_payload, p_contract_id, p_workflow_id, p_flow_id,
        (SELECT id FROM organizations WHERE tenant_id = p_tenant_id), 
        v_organization_id, p_user_id, p_span_type, p_visibility
    ) RETURNING id INTO v_span_id;
    
    RETURN v_span_id;
END;
$$ LANGUAGE plpgsql;

-- Function to get tenant statistics
CREATE OR REPLACE FUNCTION get_tenant_stats(p_tenant_id TEXT)
RETURNS TABLE(
    total_spans BIGINT,
    active_users BIGINT,
    spans_today BIGINT,
    spans_this_week BIGINT,
    most_active_author TEXT,
    latest_activity TIMESTAMPTZ
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        (SELECT COUNT(*) FROM timeline_spans ts 
         JOIN organizations o ON ts.organization_id = o.id 
         WHERE o.tenant_id = p_tenant_id) as total_spans,
         
        (SELECT COUNT(DISTINCT user_id) FROM timeline_spans ts 
         JOIN organizations o ON ts.organization_id = o.id 
         WHERE o.tenant_id = p_tenant_id 
           AND ts.created_at > now() - interval '30 days') as active_users,
           
        (SELECT COUNT(*) FROM timeline_spans ts 
         JOIN organizations o ON ts.organization_id = o.id 
         WHERE o.tenant_id = p_tenant_id 
           AND ts.created_at > current_date) as spans_today,
           
        (SELECT COUNT(*) FROM timeline_spans ts 
         JOIN organizations o ON ts.organization_id = o.id 
         WHERE o.tenant_id = p_tenant_id 
           AND ts.created_at > current_date - interval '7 days') as spans_this_week,
           
        (SELECT ts.author FROM timeline_spans ts 
         JOIN organizations o ON ts.organization_id = o.id 
         WHERE o.tenant_id = p_tenant_id 
         GROUP BY ts.author 
         ORDER BY COUNT(*) DESC 
         LIMIT 1) as most_active_author,
         
        (SELECT MAX(ts.created_at) FROM timeline_spans ts 
         JOIN organizations o ON ts.organization_id = o.id 
         WHERE o.tenant_id = p_tenant_id) as latest_activity;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-populate tenant context
CREATE OR REPLACE FUNCTION auto_populate_tenant_context()
RETURNS TRIGGER AS $$
BEGIN
    -- If organization_id is provided but tenant_id is not, populate it
    IF NEW.organization_id IS NOT NULL AND NEW.tenant_id IS NULL THEN
        SELECT id INTO NEW.tenant_id 
        FROM organizations 
        WHERE id = NEW.organization_id;
    END IF;
    
    -- If user_id is provided but organization_id is not, try to infer it
    IF NEW.user_id IS NOT NULL AND NEW.organization_id IS NULL THEN
        SELECT organization_id INTO NEW.organization_id
        FROM organization_members 
        WHERE user_id = NEW.user_id 
          AND status = 'active'
        LIMIT 1; -- Take the first active membership
    END IF;
    
    -- Set updated_at
    NEW.updated_at = now();
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply the trigger
CREATE TRIGGER timeline_spans_auto_tenant_context
    BEFORE INSERT OR UPDATE ON timeline_spans
    FOR EACH ROW
    EXECUTE FUNCTION auto_populate_tenant_context();

-- Comments
COMMENT ON FUNCTION get_tenant_timeline IS 'Get timeline spans for a specific tenant with access control';
COMMENT ON FUNCTION create_tenant_span IS 'Create a new timeline span with proper tenant context and validation';
COMMENT ON FUNCTION get_tenant_stats IS 'Get usage statistics for a specific tenant';
COMMENT ON VIEW organization_timeline_view IS 'Timeline view with organization member access control';