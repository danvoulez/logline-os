import { MigrationInterface, QueryRunner } from 'typeorm';
import { Logger } from '@nestjs/common';

export class SeedMiniConstitution1700000000028 implements MigrationInterface {
  private readonly logger = new Logger('Migration:SeedMiniConstitution');

  public async up(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('📜 Seeding Mini Constitution...');

    const constitutionContent = `
law system_invariant:1.0.0: mini_constitution:
  if invalid_contract then deny
  if agent_balance < 0 then revoke
  if contract_expired and not_delivered then penalize
    `.trim();

    const standardTenantLaw = `
law tenant_standard:1.0.0: tenant:
  if contract_value > 10000 and approvers_count < 2 then hold(approval_pending)
    `.trim();

    await queryRunner.query(`
      INSERT INTO "registry_laws" (
        "id", "scope", "target_id", "name", "description", "content", "is_active", "version"
      ) VALUES 
      (
        'law.const.001', 
        'mini_constitution', 
        NULL, 
        'System Invariants', 
        'Fundamental invariants that govern the entire LogLine system. Cannot be overridden.', 
        '${constitutionContent}', 
        true, 
        1
      ),
      (
        'law.tenant.default',
        'tenant',
        NULL, -- Default for all tenants (or specific if needed)
        'Standard Tenant Policy',
        'Standard financial controls for tenants.',
        '${standardTenantLaw}',
        true,
        1
      );
    `);

    this.logger.log('✅ Mini Constitution seeded');
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('🗑️ Removing Mini Constitution...');
    await queryRunner.query(`DELETE FROM "registry_laws" WHERE id IN ('law.const.001', 'law.tenant.default')`);
    this.logger.log('✅ Mini Constitution removed');
  }
}

