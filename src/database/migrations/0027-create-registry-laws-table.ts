import { MigrationInterface, QueryRunner } from 'typeorm';
import { Logger } from '@nestjs/common';

export class CreateRegistryLawsTable1700000000027 implements MigrationInterface {
  private readonly logger = new Logger('Migration:CreateRegistryLawsTable');

  public async up(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('⚖️ Creating registry_laws table...');

    await queryRunner.query(`
      CREATE TYPE "registry_law_scope_enum" AS ENUM (
        'mini_constitution',
        'superior',
        'app',
        'tenant',
        'user'
      );

      CREATE TABLE "registry_laws" (
        "id" character varying NOT NULL,
        "scope" "registry_law_scope_enum" NOT NULL DEFAULT 'tenant',
        "target_id" character varying,
        "name" text NOT NULL,
        "description" text NOT NULL,
        "content" text NOT NULL,
        "is_active" boolean NOT NULL DEFAULT true,
        "version" integer NOT NULL DEFAULT 1,
        "created_at" TIMESTAMP NOT NULL DEFAULT now(),
        "updated_at" TIMESTAMP NOT NULL DEFAULT now(),
        CONSTRAINT "PK_registry_laws" PRIMARY KEY ("id")
      );

      CREATE INDEX "IDX_registry_laws_scope_target" ON "registry_laws" ("scope", "target_id");
      CREATE INDEX "IDX_registry_laws_is_active" ON "registry_laws" ("is_active");
    `);

    this.logger.log('✅ registry_laws table created');
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    this.logger.log('🗑️ Dropping registry_laws table...');
    await queryRunner.query(`DROP TABLE "registry_laws"`);
    await queryRunner.query(`DROP TYPE "registry_law_scope_enum"`);
    this.logger.log('✅ registry_laws table dropped');
  }
}

