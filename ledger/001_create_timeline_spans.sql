-- Migration 001: Create timeline_spans table
-- Append-only table for LogLine spans with full auditability

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE timeline_spans (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    logline_id TEXT NOT NULL,
    author TEXT NOT NULL,
    title TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}',
    contract_id TEXT,
    workflow_id TEXT,
    flow_id TEXT,
    caused_by UUID REFERENCES timeline_spans(id),
    signature TEXT,
    status TEXT NOT NULL DEFAULT 'executed' CHECK (status IN ('executed', 'simulated', 'reverted', 'ghost')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Índices para performance e queries computáveis
CREATE INDEX idx_timeline_spans_logline_id ON timeline_spans(logline_id);
CREATE INDEX idx_timeline_spans_contract_id ON timeline_spans(contract_id);
CREATE INDEX idx_timeline_spans_timestamp ON timeline_spans(timestamp DESC);
CREATE INDEX idx_timeline_spans_workflow ON timeline_spans(workflow_id);
CREATE INDEX idx_timeline_spans_caused_by ON timeline_spans(caused_by);
CREATE INDEX idx_timeline_spans_status ON timeline_spans(status);

-- Índice para busca full-text no payload
CREATE INDEX idx_timeline_spans_payload_search ON timeline_spans USING GIN(to_tsvector('portuguese', payload::text));
CREATE INDEX idx_timeline_spans_title_search ON timeline_spans USING GIN(to_tsvector('portuguese', title));

-- Função para garantir append-only (sem UPDATE/DELETE)
CREATE OR REPLACE FUNCTION prevent_timeline_modification()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'Timeline spans are append-only. Updates not allowed.';
    END IF;
    
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'Timeline spans are append-only. Deletions not allowed.';
    END IF;
    
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Apply append-only constraint
CREATE TRIGGER timeline_spans_append_only
    BEFORE UPDATE OR DELETE ON timeline_spans
    FOR EACH ROW
    EXECUTE FUNCTION prevent_timeline_modification();

-- View para consultas common
CREATE VIEW timeline_view AS
SELECT 
    id,
    timestamp,
    logline_id,
    author,
    title,
    contract_id,
    workflow_id,
    flow_id,
    status,
    CASE 
        WHEN caused_by IS NOT NULL THEN 
            (SELECT title FROM timeline_spans WHERE id = t.caused_by)
        ELSE NULL 
    END as caused_by_title,
    created_at
FROM timeline_spans t
ORDER BY timestamp DESC;