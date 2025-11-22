import { MigrationInterface, QueryRunner } from 'typeorm';
import { Logger } from '@nestjs/common';

export class InstallAppIdeaCrafter1700000000026 implements MigrationInterface {
  private readonly logger = new Logger('Migration:InstallAppIdeaCrafter');

  public async up(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('📦 Installing App Zero: Idea Crafter...');

    const tenantId = 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11'; // Demo Tenant

    // 1. Create the Agent (The Brain)
    await queryRunner.query(`
      INSERT INTO agents (
        id, name, description, model_profile, 
        instructions,
        logline_agent_id, onboarding_status, 
        tenant_id,
        created_at, updated_at
      ) VALUES (
        'agent.idea.crafter', 
        'Idea Crafter', 
        'Specialist in expanding and refining raw ideas.', 
        '{"model": "gpt-4o", "provider": "openai", "temperature": 0.7}', 
        'You are the Idea Crafter. Your goal is to take vague user inputs and help expand them into concrete, actionable concepts. Ask clarifying questions, suggest features, and help structure the idea. Be enthusiastic and creative.',
        'LL-AGENT-2024-IDEA-001', 
        'certified',
        '${tenantId}',
        NOW(), NOW()
      )
      ON CONFLICT (id) DO UPDATE SET
        instructions = EXCLUDED.instructions,
        model_profile = EXCLUDED.model_profile;
    `);

    // 2. Create the App Manifest (The Package)
    await queryRunner.query(`
      INSERT INTO apps (
        id, name, description, 
        owner,
        visibility,
        created_at, updated_at
      ) VALUES (
        'app.idea.crafter',
        'Idea Crafter',
        'Bring any idea to life with AI-guided refinement.',
        'LL-BR-2024-000000001-CEO',
        'public',
        NOW(), NOW()
      )
      ON CONFLICT (id) DO NOTHING;
    `);

    // 3. Grant Permissions (Scopes)
    // Clean up first to avoid dupes (since no unique constraint on value)
    await queryRunner.query(`DELETE FROM app_scopes WHERE app_id = 'app.idea.crafter'`);
    
    await queryRunner.query(`
      INSERT INTO app_scopes (
        app_id, scope_type, scope_value
      ) VALUES (
        'app.idea.crafter',
        'external',
        'agent.idea.crafter'
      );
    `);

    this.logger.log('✅ App Idea Crafter installed successfully');
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`DELETE FROM app_scopes WHERE app_id = 'app.idea.crafter'`);
    await queryRunner.query(`DELETE FROM apps WHERE id = 'app.idea.crafter'`);
    await queryRunner.query(`DELETE FROM agents WHERE id = 'agent.idea.crafter'`);
  }
}
