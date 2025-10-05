-- Migration 002: Implement Multi-Tenant Infrastructure
-- Complete multi-tenancy support for LogLineOS

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Organizations/Tenants table
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id TEXT NOT NULL UNIQUE, -- URL-safe identifier (e.g., 'acme', 'startup')
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    domain TEXT, -- Primary domain (e.g., 'acme.com')
    subdomain TEXT UNIQUE, -- Subdomain for LogLineOS (e.g., 'acme')
    
    -- Branding configuration
    logo_url TEXT,
    primary_color TEXT DEFAULT '#007AFF',
    secondary_color TEXT DEFAULT '#00D4FF',
    accent_color TEXT DEFAULT '#34C759',
    custom_css TEXT,
    
    -- Policies
    require_biometric BOOLEAN DEFAULT true,
    allow_skip_identity BOOLEAN DEFAULT false,
    custom_terms_url TEXT,
    custom_privacy_url TEXT,
    
    -- Features
    enable_ghost_identities BOOLEAN DEFAULT true,
    enable_organization_roles BOOLEAN DEFAULT true,
    enable_advanced_security BOOLEAN DEFAULT false,
    enable_federation BOOLEAN DEFAULT false,
    
    -- Metadata
    subscription_tier TEXT DEFAULT 'free' CHECK (subscription_tier IN ('free', 'pro', 'enterprise')),
    max_users INTEGER DEFAULT 10,
    metadata JSONB DEFAULT '{}',
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID, -- References auth.users
    is_active BOOLEAN DEFAULT true
);

-- Organization users/members
CREATE TABLE organization_members (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL, -- References auth.users
    
    -- Role and permissions
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
    permissions JSONB DEFAULT '{}',
    
    -- Invitation details
    invited_by UUID, -- References auth.users
    invitation_token TEXT UNIQUE,
    invitation_expires_at TIMESTAMPTZ,
    invitation_accepted_at TIMESTAMPTZ,
    
    -- Status
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('invited', 'active', 'suspended', 'removed')),
    
    -- Audit
    joined_at TIMESTAMPTZ DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    UNIQUE(organization_id, user_id)
);

-- Update timeline_spans to be tenant-aware
DROP VIEW IF EXISTS timeline_view;

ALTER TABLE timeline_spans 
ADD COLUMN tenant_id UUID REFERENCES organizations(id),
ADD COLUMN organization_id UUID REFERENCES organizations(id),
ADD COLUMN user_id UUID, -- References auth.users
ADD COLUMN span_type TEXT DEFAULT 'user' CHECK (span_type IN ('user', 'system', 'organization', 'ghost')),
ADD COLUMN visibility TEXT DEFAULT 'private' CHECK (visibility IN ('private', 'organization', 'public')),
ADD COLUMN metadata JSONB DEFAULT '{}';

-- Identities table (multi-tenant aware)
CREATE TABLE identities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    logline_id TEXT NOT NULL UNIQUE,
    
    -- Basic identity info
    display_name TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    
    -- Tenant context
    tenant_id UUID REFERENCES organizations(id),
    organization_id UUID REFERENCES organizations(id),
    user_id UUID, -- References auth.users
    
    -- Identity type and status
    identity_type TEXT NOT NULL DEFAULT 'individual' CHECK (identity_type IN ('individual', 'organization_member', 'ghost', 'system')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('pending', 'active', 'suspended', 'archived')),
    
    -- Biometric data (encrypted)
    passkey_id TEXT,
    biometric_setup BOOLEAN DEFAULT false,
    biometric_data_encrypted TEXT, -- Encrypted biometric identifiers
    
    -- Federation
    federated_from TEXT, -- Source federation node
    federation_verified BOOLEAN DEFAULT false,
    
    -- Audit and metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_active_at TIMESTAMPTZ,
    
    -- Constraints
    UNIQUE(organization_id, email) -- Email unique within organization
);

-- Federated nodes (updated for multi-tenancy)
CREATE TABLE federated_nodes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    node_id TEXT NOT NULL UNIQUE,
    
    -- Node identity
    display_name TEXT NOT NULL,
    description TEXT,
    public_key TEXT NOT NULL,
    
    -- Tenant context
    organization_id UUID REFERENCES organizations(id),
    node_type TEXT NOT NULL DEFAULT 'peer' CHECK (node_type IN ('peer', 'hub', 'satellite')),
    
    -- Trust and verification
    trust_level INTEGER DEFAULT 0 CHECK (trust_level >= 0 AND trust_level <= 100),
    verified_by UUID, -- References auth.users
    verification_signature TEXT,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_ping_at TIMESTAMPTZ,
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID -- References auth.users
);

-- Federated links (tenant-aware)
CREATE TABLE federated_links (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    source_node_id UUID NOT NULL REFERENCES federated_nodes(id),
    target_node_id UUID NOT NULL REFERENCES federated_nodes(id),
    
    -- Link properties
    link_type TEXT NOT NULL DEFAULT 'trust' CHECK (link_type IN ('trust', 'mirror', 'backup', 'forward')),
    trust_score DECIMAL(3,2) DEFAULT 0.00 CHECK (trust_score >= 0.00 AND trust_score <= 1.00),
    
    -- Tenant context
    organization_id UUID REFERENCES organizations(id),
    
    -- Configuration
    is_bidirectional BOOLEAN DEFAULT false,
    metadata JSONB DEFAULT '{}',
    
    -- Status
    is_active BOOLEAN DEFAULT true,
    last_sync_at TIMESTAMPTZ,
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID, -- References auth.users
    
    UNIQUE(source_node_id, target_node_id)
);

-- Events bus (tenant-aware)
CREATE TABLE events_bus (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    
    -- Event details
    topic TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    
    -- Tenant context
    tenant_id UUID REFERENCES organizations(id),
    organization_id UUID REFERENCES organizations(id),
    user_id UUID, -- References auth.users
    
    -- Routing and processing
    target_audiences TEXT[] DEFAULT '{}', -- ['organization', 'federation', 'public']
    processing_status TEXT DEFAULT 'pending' CHECK (processing_status IN ('pending', 'processing', 'completed', 'failed')),
    
    -- Metadata
    source_span_id UUID REFERENCES timeline_spans(id),
    correlation_id UUID,
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ
);

-- Audit trail for all changes
CREATE TABLE audit_trail (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    
    -- What was changed
    table_name TEXT NOT NULL,
    record_id UUID NOT NULL,
    operation TEXT NOT NULL CHECK (operation IN ('INSERT', 'UPDATE', 'DELETE')),
    
    -- Changes
    old_data JSONB,
    new_data JSONB,
    changed_fields TEXT[],
    
    -- Tenant context
    tenant_id UUID REFERENCES organizations(id),
    organization_id UUID REFERENCES organizations(id),
    
    -- Who and when
    executed_by UUID, -- References auth.users
    executed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    
    -- Context
    source_ip INET,
    user_agent TEXT,
    session_id TEXT,
    
    -- Metadata
    metadata JSONB DEFAULT '{}'
);

-- Universal objects (tenant-aware)
CREATE TABLE universal_objects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    
    -- Object identity
    name TEXT NOT NULL,
    alias TEXT,
    object_type TEXT NOT NULL,
    subtype TEXT,
    category TEXT,
    
    -- Tenant context
    tenant_id UUID REFERENCES organizations(id),
    organization_id UUID REFERENCES organizations(id),
    owner_id UUID, -- References auth.users
    
    -- Object properties
    is_ghost BOOLEAN DEFAULT false,
    is_verified BOOLEAN DEFAULT false,
    
    -- File properties (if applicable)
    filename TEXT,
    mimetype TEXT,
    storage_path TEXT,
    size_bytes BIGINT,
    hash_sha256 TEXT,
    
    -- Access control
    access_scope TEXT DEFAULT 'private' CHECK (access_scope IN ('private', 'organization', 'public')),
    access_tokens JSONB DEFAULT '{}',
    
    -- Physical properties (for physical objects)
    location TEXT,
    condition TEXT,
    rfid_tag TEXT UNIQUE,
    nfc_data JSONB,
    tag_last_scanned TIMESTAMPTZ,
    
    -- Lifecycle
    acquisition_date DATE,
    expiration_date DATE,
    retention_policy TEXT,
    
    -- Rich metadata
    metadata JSONB DEFAULT '{}',
    external_links JSONB DEFAULT '{}',
    
    -- Audit
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- === INDEXES FOR PERFORMANCE ===

-- Organizations
CREATE INDEX idx_organizations_tenant_id ON organizations(tenant_id);
CREATE INDEX idx_organizations_subdomain ON organizations(subdomain);
CREATE INDEX idx_organizations_domain ON organizations(domain);
CREATE INDEX idx_organizations_active ON organizations(is_active) WHERE is_active = true;

-- Organization members
CREATE INDEX idx_org_members_org_id ON organization_members(organization_id);
CREATE INDEX idx_org_members_user_id ON organization_members(user_id);
CREATE INDEX idx_org_members_role ON organization_members(role);
CREATE INDEX idx_org_members_status ON organization_members(status);
CREATE INDEX idx_org_members_invitation_token ON organization_members(invitation_token) WHERE invitation_token IS NOT NULL;

-- Timeline spans (updated indexes)
CREATE INDEX idx_timeline_spans_tenant_id ON timeline_spans(tenant_id);
CREATE INDEX idx_timeline_spans_organization_id ON timeline_spans(organization_id);
CREATE INDEX idx_timeline_spans_user_id ON timeline_spans(user_id);
CREATE INDEX idx_timeline_spans_span_type ON timeline_spans(span_type);
CREATE INDEX idx_timeline_spans_visibility ON timeline_spans(visibility);

-- Identities
CREATE INDEX idx_identities_tenant_id ON identities(tenant_id);
CREATE INDEX idx_identities_organization_id ON identities(organization_id);
CREATE INDEX idx_identities_user_id ON identities(user_id);
CREATE INDEX idx_identities_logline_id ON identities(logline_id);
CREATE INDEX idx_identities_email ON identities(email);
CREATE INDEX idx_identities_type_status ON identities(identity_type, status);
CREATE INDEX idx_identities_passkey_id ON identities(passkey_id) WHERE passkey_id IS NOT NULL;

-- Events bus
CREATE INDEX idx_events_bus_tenant_id ON events_bus(tenant_id);
CREATE INDEX idx_events_bus_organization_id ON events_bus(organization_id);
CREATE INDEX idx_events_bus_topic ON events_bus(topic);
CREATE INDEX idx_events_bus_created_at ON events_bus(created_at DESC);
CREATE INDEX idx_events_bus_processing_status ON events_bus(processing_status);

-- Audit trail
CREATE INDEX idx_audit_trail_tenant_id ON audit_trail(tenant_id);
CREATE INDEX idx_audit_trail_organization_id ON audit_trail(organization_id);
CREATE INDEX idx_audit_trail_table_record ON audit_trail(table_name, record_id);
CREATE INDEX idx_audit_trail_executed_by ON audit_trail(executed_by);
CREATE INDEX idx_audit_trail_executed_at ON audit_trail(executed_at DESC);

-- Universal objects
CREATE INDEX idx_universal_objects_tenant_id ON universal_objects(tenant_id);
CREATE INDEX idx_universal_objects_organization_id ON universal_objects(organization_id);
CREATE INDEX idx_universal_objects_owner_id ON universal_objects(owner_id);
CREATE INDEX idx_universal_objects_type ON universal_objects(object_type);
CREATE INDEX idx_universal_objects_access_scope ON universal_objects(access_scope);
CREATE INDEX idx_universal_objects_rfid_tag ON universal_objects(rfid_tag) WHERE rfid_tag IS NOT NULL;

-- === VIEWS FOR COMMON QUERIES ===

-- Multi-tenant timeline view
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
    -- Organization context
    o.name as organization_name,
    o.tenant_id,
    -- Causation chain
    CASE 
        WHEN ts.caused_by IS NOT NULL THEN 
            (SELECT title FROM timeline_spans WHERE id = ts.caused_by)
        ELSE NULL 
    END as caused_by_title,
    ts.created_at
FROM timeline_spans ts
LEFT JOIN organizations o ON ts.organization_id = o.id
ORDER BY ts.timestamp DESC;

-- Organization member view
CREATE VIEW organization_members_view AS
SELECT 
    om.*,
    o.name as organization_name,
    o.tenant_id,
    i.display_name as member_name,
    i.email as member_email
FROM organization_members om
JOIN organizations o ON om.organization_id = o.id
LEFT JOIN identities i ON om.user_id = i.user_id AND i.organization_id = o.id
WHERE om.status = 'active';

-- === FUNCTIONS FOR TENANT ISOLATION ===

-- Function to get user's organization context
CREATE OR REPLACE FUNCTION get_user_organization_context(user_uuid UUID)
RETURNS TABLE(
    organization_id UUID,
    tenant_id TEXT,
    role TEXT,
    permissions JSONB
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        om.organization_id,
        o.tenant_id,
        om.role,
        om.permissions
    FROM organization_members om
    JOIN organizations o ON om.organization_id = o.id
    WHERE om.user_id = user_uuid 
      AND om.status = 'active'
      AND o.is_active = true;
END;
$$ LANGUAGE plpgsql;

-- Function to check if user can access tenant data
CREATE OR REPLACE FUNCTION can_access_tenant_data(user_uuid UUID, target_tenant_id TEXT)
RETURNS BOOLEAN AS $$
DECLARE
    has_access BOOLEAN := false;
BEGIN
    SELECT EXISTS(
        SELECT 1 
        FROM organization_members om
        JOIN organizations o ON om.organization_id = o.id
        WHERE om.user_id = user_uuid 
          AND o.tenant_id = target_tenant_id
          AND om.status = 'active'
          AND o.is_active = true
    ) INTO has_access;
    
    RETURN has_access;
END;
$$ LANGUAGE plpgsql;

-- === ROW LEVEL SECURITY (RLS) ===

-- Enable RLS on tenant-aware tables
ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE timeline_spans ENABLE ROW LEVEL SECURITY;
ALTER TABLE identities ENABLE ROW LEVEL SECURITY;
ALTER TABLE events_bus ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_trail ENABLE ROW LEVEL SECURITY;
ALTER TABLE universal_objects ENABLE ROW LEVEL SECURITY;

-- RLS policies will be added in a separate migration once auth is configured

-- === TRIGGERS FOR AUDIT TRAIL ===

-- Generic audit trigger function
CREATE OR REPLACE FUNCTION audit_trigger_function()
RETURNS TRIGGER AS $$
DECLARE
    old_data JSONB := NULL;
    new_data JSONB := NULL;
    changed_fields TEXT[] := '{}';
BEGIN
    IF TG_OP = 'DELETE' THEN
        old_data := to_jsonb(OLD);
        INSERT INTO audit_trail (
            table_name, record_id, operation, old_data, executed_at
        ) VALUES (
            TG_TABLE_NAME, OLD.id, TG_OP, old_data, now()
        );
        RETURN OLD;
    END IF;
    
    IF TG_OP = 'INSERT' THEN
        new_data := to_jsonb(NEW);
        INSERT INTO audit_trail (
            table_name, record_id, operation, new_data, executed_at
        ) VALUES (
            TG_TABLE_NAME, NEW.id, TG_OP, new_data, now()
        );
        RETURN NEW;
    END IF;
    
    IF TG_OP = 'UPDATE' THEN
        old_data := to_jsonb(OLD);
        new_data := to_jsonb(NEW);
        
        -- Find changed fields
        SELECT array_agg(key) INTO changed_fields
        FROM jsonb_each(old_data) o
        WHERE o.value IS DISTINCT FROM (new_data->o.key);
        
        INSERT INTO audit_trail (
            table_name, record_id, operation, old_data, new_data, changed_fields, executed_at
        ) VALUES (
            TG_TABLE_NAME, NEW.id, TG_OP, old_data, new_data, changed_fields, now()
        );
        RETURN NEW;
    END IF;
    
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Apply audit triggers to key tables
CREATE TRIGGER organizations_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON organizations
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER organization_members_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON organization_members
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

CREATE TRIGGER identities_audit_trigger
    AFTER INSERT OR UPDATE OR DELETE ON identities
    FOR EACH ROW EXECUTE FUNCTION audit_trigger_function();

-- === SAMPLE DATA FOR TESTING ===

-- Insert sample organizations
INSERT INTO organizations (
    tenant_id, name, display_name, domain, subdomain,
    primary_color, secondary_color,
    require_biometric, enable_ghost_identities,
    subscription_tier, max_users
) VALUES 
('acme', 'Acme Corporation', 'Acme Corp', 'acme.com', 'acme',
 '#E74C3C', '#C0392B', true, true, 'enterprise', 1000),
('startup', 'StartupCo', 'StartupCo', 'startup.co', 'startup',
 '#9B59B6', '#8E44AD', false, false, 'pro', 100),
('individual', 'LogLine ID Individual', 'LogLine ID', NULL, NULL,
 '#007AFF', '#00D4FF', true, true, 'free', 1);

-- Comments for documentation
COMMENT ON TABLE organizations IS 'Multi-tenant organizations/tenants in LogLineOS';
COMMENT ON TABLE organization_members IS 'Users belonging to organizations with roles and permissions';
COMMENT ON TABLE identities IS 'Digital identities with multi-tenant context';
COMMENT ON TABLE federated_nodes IS 'Federated network nodes for cross-organization communication';
COMMENT ON TABLE events_bus IS 'Event streaming system with tenant isolation';
COMMENT ON TABLE audit_trail IS 'Complete audit log of all system changes';
COMMENT ON TABLE universal_objects IS 'Universal object registry with tenant-aware access control';