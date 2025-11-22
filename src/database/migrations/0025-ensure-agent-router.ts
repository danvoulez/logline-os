import { MigrationInterface, QueryRunner } from 'typeorm';
import { Logger } from '@nestjs/common';

export class EnsureAgentRouter1700000000025 implements MigrationInterface {
  private readonly logger = new Logger('Migration:EnsureAgentRouter');

  public async up(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('🔧 Ensuring agent.router exists and is linked...');

    const tenantId = 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11';

    // Ensure Agent Exists
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

    this.logger.log('✅ Agent router ensured');
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    // No-op or revert link
    await queryRunner.query(`
      UPDATE agents 
      SET tenant_id = NULL, owner_logline_id = NULL, active_contract_id = NULL 
      WHERE id = 'agent.router';
    `);
  }
}

