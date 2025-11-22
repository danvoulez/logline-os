import { MigrationInterface, QueryRunner } from 'typeorm';
import { Logger } from '@nestjs/common';

export class SeedDemoData1700000000024 implements MigrationInterface {
  private readonly logger = new Logger('Migration:SeedDemoData');

  public async up(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('🌱 Seeding demo data...');

    // 1. Create Demo Tenant
    // Using a fixed UUID for reproducibility
    const tenantId = 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11';
    
    // 2. Create Core People (Identity)
    // Admin / CEO
    await queryRunner.query(`
      INSERT INTO core_people (logline_id, name, email_primary, created_at, updated_at)
      VALUES ('LL-BR-2024-000000001-CEO', 'Alice CEO', 'alice@demo.corp', NOW(), NOW())
      ON CONFLICT (logline_id) DO NOTHING;
    `);

    // Developer / CTO
    await queryRunner.query(`
      INSERT INTO core_people (logline_id, name, email_primary, created_at, updated_at)
      VALUES ('LL-BR-2024-000000002-CTO', 'Bob Dev', 'bob@demo.corp', NOW(), NOW())
      ON CONFLICT (logline_id) DO NOTHING;
    `);

    // Manager / PM
    await queryRunner.query(`
      INSERT INTO core_people (logline_id, name, email_primary, created_at, updated_at)
      VALUES ('LL-BR-2024-000000003-PM', 'Charlie PM', 'charlie@demo.corp', NOW(), NOW())
      ON CONFLICT (logline_id) DO NOTHING;
    `);

    // 3. Link People to Tenant
    await queryRunner.query(`
      INSERT INTO tenant_people_relationships (logline_id, tenant_id, role, permissions, created_at, updated_at)
      VALUES 
      ('LL-BR-2024-000000001-CEO', '${tenantId}', 'owner', '{"admin": true}', NOW(), NOW()),
      ('LL-BR-2024-000000002-CTO', '${tenantId}', 'admin', '{"dev": true}', NOW(), NOW()),
      ('LL-BR-2024-000000003-PM', '${tenantId}', 'member', '{"manage": true}', NOW(), NOW())
      ON CONFLICT (logline_id, tenant_id) DO NOTHING;
    `);

    // 4. Create Users (Auth)
    await queryRunner.query(`
      INSERT INTO users (email, name, role, tenant_id, logline_id, password_hash, created_at, updated_at)
      VALUES 
      ('alice@demo.corp', 'Alice CEO', 'admin', '${tenantId}', 'LL-BR-2024-000000001-CEO', 'hashed_pass_123', NOW(), NOW()),
      ('bob@demo.corp', 'Bob Dev', 'developer', '${tenantId}', 'LL-BR-2024-000000002-CTO', 'hashed_pass_123', NOW(), NOW()),
      ('charlie@demo.corp', 'Charlie PM', 'user', '${tenantId}', 'LL-BR-2024-000000003-PM', 'hashed_pass_123', NOW(), NOW())
      ON CONFLICT (email) DO NOTHING;
    `);

    // 5. Create Ideas (Budget Democracy)
    // Idea 1: AI HR System (Approved)
    await queryRunner.query(`
      INSERT INTO registry_ideas (
        id, tenant_id, titulo, descricao, autor_logline_id, 
        prioridade_autor, prioridade_consensual, custo_estimado_cents, 
        status, periodo_votacao_dias, data_submissao, created_at, updated_at
      )
      VALUES (
        'b1eebc99-9c0b-4ef8-bb6d-6bb9bd380b22', 
        '${tenantId}', 
        'AI-Powered HR System', 
        'Automate recruitment and onboarding using LLM agents.', 
        'LL-BR-2024-000000001-CEO',
        10, 9.5, 15000000, -- R$ 150,000.00
        'aprovada', 7, NOW() - INTERVAL '10 days', NOW(), NOW()
      )
      ON CONFLICT (id) DO NOTHING;
    `);

    // Idea 2: Backend Refactor (Voting)
    await queryRunner.query(`
      INSERT INTO registry_ideas (
        id, tenant_id, titulo, descricao, autor_logline_id, 
        prioridade_autor, prioridade_consensual, custo_estimado_cents, 
        status, periodo_votacao_dias, data_submissao, created_at, updated_at
      )
      VALUES (
        'c1eebc99-9c0b-4ef8-bb6d-6bb9bd380c33', 
        '${tenantId}', 
        'Backend Refactoring', 
        'Migrate to new architecture for better scale.', 
        'LL-BR-2024-000000002-CTO',
        9, 8.0, 500000, -- R$ 5,000.00
        'em_votacao', 7, NOW() - INTERVAL '2 days', NOW(), NOW()
      )
      ON CONFLICT (id) DO NOTHING;
    `);

    // 6. Create Votes
    await queryRunner.query(`
      INSERT INTO registry_idea_votes (idea_id, voter_logline_id, prioridade, comentario, peso, created_at, updated_at)
      VALUES 
      ('b1eebc99-9c0b-4ef8-bb6d-6bb9bd380b22', 'LL-BR-2024-000000002-CTO', 9, 'Technical implementation is feasible', 1.0, NOW(), NOW()),
      ('b1eebc99-9c0b-4ef8-bb6d-6bb9bd380b22', 'LL-BR-2024-000000003-PM', 10, 'Crucial for growth', 1.0, NOW(), NOW()),
      ('c1eebc99-9c0b-4ef8-bb6d-6bb9bd380c33', 'LL-BR-2024-000000001-CEO', 7, 'Important but expensive', 1.5, NOW(), NOW())
      ON CONFLICT (idea_id, voter_logline_id) DO NOTHING;
    `);

    // 7. Create Contracts
    // Contract 1: Active Dev Contract
    await queryRunner.query(`
      INSERT INTO registry_contracts (
        id, tenant_id, tipo, titulo, descricao, 
        autor_logline_id, contraparte_logline_id, testemunha_logline_id,
        valor_total_cents, moeda, estado_atual, 
        data_inicio, prazo_dias, data_limite,
        idea_id, created_at, updated_at
      )
      VALUES (
        'd1eebc99-9c0b-4ef8-bb6d-6bb9bd380d44',
        '${tenantId}',
        'desenvolvimento',
        'Contract: AI HR System Implementation',
        'Implementation of the approved AI HR System idea.',
        'LL-BR-2024-000000001-CEO',
        'LL-BR-2024-000000002-CTO',
        'LL-BR-2024-000000003-PM',
        15000000, 'BRL', 'VIGENTE',
        NOW(), 90, NOW() + INTERVAL '90 days',
        'b1eebc99-9c0b-4ef8-bb6d-6bb9bd380b22',
        NOW(), NOW()
      )
      ON CONFLICT (id) DO NOTHING;
    `);

    // Contract 2: Draft Service Agreement
    await queryRunner.query(`
      INSERT INTO registry_contracts (
        id, tenant_id, tipo, titulo, descricao, 
        autor_logline_id, contraparte_logline_id,
        valor_total_cents, moeda, estado_atual, 
        created_at, updated_at
      )
      VALUES (
        'e1eebc99-9c0b-4ef8-bb6d-6bb9bd380e55',
        '${tenantId}',
        'servico',
        'Draft: External Security Audit',
        'Pending vendor selection.',
        'LL-BR-2024-000000003-PM',
        'LL-BR-2024-000000001-CEO',
        2500000, 'BRL', 'RASCUNHO',
        NOW(), NOW()
      )
      ON CONFLICT (id) DO NOTHING;
    `);

    // 8. Create Objects
    await queryRunner.query(`
      INSERT INTO registry_objects (
        id, object_type, name, description, tenant_id, 
        owner_logline_id, current_custodian_logline_id, location, 
        created_at, updated_at
      )
      VALUES 
      (
        'f1eebc99-9c0b-4ef8-bb6d-6bb9bd380f66',
        'merchandise',
        'Dev MacBook Pro 16',
        'Asset tag: MB-001',
        '${tenantId}',
        'LL-BR-2024-000000001-CEO',
        'LL-BR-2024-000000002-CTO',
        'Remote - Bob Home',
        NOW(), NOW()
      ),
      (
        'a1eebc99-9c0b-4ef8-bb6d-6bb9bd380a77',
        'document',
        'HR System Architecture Doc',
        'Version 1.0 PDF',
        '${tenantId}',
        'LL-BR-2024-000000002-CTO',
        'LL-BR-2024-000000002-CTO',
        'Google Drive / Tech / Docs',
        NOW(), NOW()
      )
      ON CONFLICT (id) DO NOTHING;
    `);

    // 9. Update Agents (Link to Context)
    // Ensure Agent Exists first (for clean environments)
    await queryRunner.query(`
      INSERT INTO agents (
        id, name, description, model_profile, 
        logline_agent_id, onboarding_status, 
        created_at, updated_at
      ) VALUES (
        'agent.router', 'System Router', 'Main routing agent', 
        '{"model": "gpt-4o", "provider": "openai"}', 
        'LL-AGENT-2024-000', 'certified',
        NOW(), NOW()
      )
      ON CONFLICT (id) DO NOTHING;
    `);

    // Link Router Agent to Tenant and Contract
    await queryRunner.query(`
      UPDATE agents 
      SET 
        tenant_id = '${tenantId}',
        owner_logline_id = 'LL-BR-2024-000000002-CTO',
        active_contract_id = 'd1eebc99-9c0b-4ef8-bb6d-6bb9bd380d44',
        total_runs = 42,
        avg_cost_per_run_cents = 15
      WHERE id = 'agent.router';
    `);

    this.logger.log('✅ Demo data seeded successfully');
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    // Delete in reverse order
    const tenantId = 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11';

    await queryRunner.query(`DELETE FROM registry_objects WHERE tenant_id = '${tenantId}';`);
    await queryRunner.query(`DELETE FROM registry_contracts WHERE tenant_id = '${tenantId}';`);
    await queryRunner.query(`DELETE FROM registry_idea_votes WHERE voter_logline_id IN ('LL-BR-2024-000000001-CEO', 'LL-BR-2024-000000002-CTO', 'LL-BR-2024-000000003-PM');`);
    await queryRunner.query(`DELETE FROM registry_ideas WHERE tenant_id = '${tenantId}';`);
    await queryRunner.query(`DELETE FROM users WHERE tenant_id = '${tenantId}';`);
    await queryRunner.query(`DELETE FROM tenant_people_relationships WHERE tenant_id = '${tenantId}';`);
    await queryRunner.query(`DELETE FROM core_people WHERE logline_id IN ('LL-BR-2024-000000001-CEO', 'LL-BR-2024-000000002-CTO', 'LL-BR-2024-000000003-PM');`);
    
    // Reset agents
    await queryRunner.query(`
      UPDATE agents 
      SET tenant_id = NULL, owner_logline_id = NULL, active_contract_id = NULL 
      WHERE id = 'agent.router';
    `);

    this.logger.log('✅ Demo data removed');
  }
}

