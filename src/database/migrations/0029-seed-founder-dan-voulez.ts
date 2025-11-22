import { MigrationInterface, QueryRunner } from 'typeorm';
import { Logger } from '@nestjs/common';

export class SeedFounderDanVoulez1700000000029 implements MigrationInterface {
  private readonly logger = new Logger('Migration:SeedFounderDanVoulez');

  public async up(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('👤 Seeding Founder Dan Voulez...');

    const tenantId = 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11';
    const loglineId = 'LL-BR-2024-000000000-FDR';
    const email = 'dan@danvoulez.com';
    const name = 'Dan Voulez';

    // 1. Create Core Person
    await queryRunner.query(`
      INSERT INTO core_people (logline_id, name, email_primary, created_at, updated_at)
      VALUES ('${loglineId}', '${name}', '${email}', NOW(), NOW())
      ON CONFLICT (logline_id) DO UPDATE SET
        name = EXCLUDED.name,
        email_primary = EXCLUDED.email_primary;
    `);

    // 2. Link to Tenant as Founder
    // Note: role 'founder' is allowed as TEXT in the database, though Typescript enum might need update
    await queryRunner.query(`
      INSERT INTO tenant_people_relationships (logline_id, tenant_id, role, permissions, created_at, updated_at)
      VALUES 
      ('${loglineId}', '${tenantId}', 'founder', '{"admin": true, "founder": true, "superuser": true}', NOW(), NOW())
      ON CONFLICT (logline_id, tenant_id) DO UPDATE SET
        role = 'founder',
        permissions = '{"admin": true, "founder": true, "superuser": true}';
    `);

    // 3. Create User Login (Admin Role)
    await queryRunner.query(`
      INSERT INTO users (email, name, role, tenant_id, logline_id, password_hash, created_at, updated_at)
      VALUES 
      ('${email}', '${name}', 'admin', '${tenantId}', '${loglineId}', '$2b$10$EpWk.zF.a.K.g.h.j.k.l.m.n.o.p.q.r.s.t.u.v.w.x.y.z', NOW(), NOW()) -- Hash needs to be valid bcrypt, this is a placeholder or use a known one
      ON CONFLICT (email) DO UPDATE SET
        role = 'admin',
        logline_id = '${loglineId}',
        tenant_id = '${tenantId}';
    `);

    this.logger.log('✅ Founder Dan Voulez seeded');
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('🗑️ Removing Founder Dan Voulez...');
    const loglineId = 'LL-BR-2024-000000000-FDR';
    const email = 'dan@danvoulez.com';

    await queryRunner.query(`DELETE FROM users WHERE email = '${email}';`);
    await queryRunner.query(`DELETE FROM tenant_people_relationships WHERE logline_id = '${loglineId}';`);
    await queryRunner.query(`DELETE FROM core_people WHERE logline_id = '${loglineId}';`);
    
    this.logger.log('✅ Founder removed');
  }
}

