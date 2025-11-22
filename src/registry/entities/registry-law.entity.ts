import { Entity, PrimaryColumn, Column, CreateDateColumn, UpdateDateColumn, Index } from 'typeorm';

export enum LawScope {
  MINI_CONSTITUTION = 'mini_constitution', // Suprema (Invariantes do Sistema)
  SUPERIOR = 'superior',                   // Regulamentação (Compliance Externo)
  APP = 'app',                             // Regras de Negócio do App
  TENANT = 'tenant',                       // Políticas da Empresa (Cliente)
  USER = 'user',                           // Preferências Individuais
}

@Entity('registry_laws')
@Index(['scope', 'target_id'])
@Index(['is_active'])
export class RegistryLaw {
  @PrimaryColumn('varchar')
  id: string; // ex: 'law.const.001'

  @Column({
    type: 'enum',
    enum: LawScope,
    default: LawScope.TENANT
  })
  scope: LawScope;

  @Column('varchar', { nullable: true })
  target_id: string | null; // ID do App, Tenant ou User. NULL para escopos globais (Constitution/Superior)

  @Column('text')
  name: string; // Nome legível (ex: "Lei da Solvência")

  @Column('text')
  description: string; // Explicação humana

  @Column('text')
  content: string; // A gramática da lei ("if agent_balance < 0 then revoke")

  @Column('boolean', { default: true })
  is_active: boolean;

  @Column('int', { default: 1 })
  version: number;

  @CreateDateColumn()
  created_at: Date;

  @UpdateDateColumn()
  updated_at: Date;
}

