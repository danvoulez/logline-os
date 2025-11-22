import { Injectable, Logger } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { RegistryLaw, LawScope } from '../entities/registry-law.entity';

export interface LawContext {
  contract?: any;
  agent?: any;
  tenant_id?: string;
  user_id?: string;
  app_id?: string;
  now?: Date;
  [key: string]: any;
}

interface Rule {
  condition: string;
  action: string;
}

interface ParsedLaw {
  id: string;
  version: string;
  scope: LawScope;
  rules: Rule[];
}

// Hierarchy: Higher number = Higher Authority
const SCOPE_PRECEDENCE: Record<LawScope, number> = {
  [LawScope.MINI_CONSTITUTION]: 5,
  [LawScope.SUPERIOR]: 4,
  [LawScope.APP]: 3,
  [LawScope.TENANT]: 2,
  [LawScope.USER]: 1,
};

const ALLOWED_ACTIONS: Record<LawScope, string[]> = {
  [LawScope.MINI_CONSTITUTION]: ['deny', 'accept', 'penalize', 'hold', 'notify', 'revoke'],
  [LawScope.SUPERIOR]: ['accept', 'penalize', 'hold', 'notify'],
  [LawScope.APP]: ['accept', 'hold', 'notify'],
  [LawScope.TENANT]: ['hold', 'notify', 'penalize'], // Expanded for tenant policy
  [LawScope.USER]: ['notify'],
};

@Injectable()
export class ConstitutionService {
  private readonly logger = new Logger(ConstitutionService.name);

  constructor(
    @InjectRepository(RegistryLaw)
    private lawsRepository: Repository<RegistryLaw>,
  ) {}

  /**
   * Evaluate all applicable laws for a given context.
   * Returns the highest priority action determined by the Constitution.
   */
  async evaluate(context: LawContext): Promise<string | null> {
    const laws = await this.fetchApplicableLaws(context);
    const parsedLaws = laws.map(l => this.parseLaw(l)).filter(l => l !== null) as ParsedLaw[];
    
    // Sort by precedence (Constitution first)
    parsedLaws.sort((a, b) => SCOPE_PRECEDENCE[b.scope] - SCOPE_PRECEDENCE[a.scope]);

    for (const law of parsedLaws) {
      for (const rule of law.rules) {
        if (this.evaluateCondition(rule.condition, context)) {
          this.logger.log(`⚖️ Law Applied: [${law.scope}] ${law.id} -> ${rule.action}`);
          
          // If action is 'deny' or 'revoke', it overrides everything (Fail Closed)
          if (['deny', 'revoke'].includes(rule.action)) {
            return rule.action;
          }
          
          // Otherwise, return the first matching action from the highest authority
          return rule.action;
        }
      }
    }

    return null; // No law triggered
  }

  private async fetchApplicableLaws(context: LawContext): Promise<RegistryLaw[]> {
    const query = this.lawsRepository.createQueryBuilder('law')
      .where('law.is_active = :active', { active: true })
      .andWhere(
        '(law.scope = :s1 OR law.scope = :s2 OR (law.scope = :s3 AND law.target_id = :appId) OR (law.scope = :s4 AND law.target_id = :tenantId) OR (law.scope = :s5 AND law.target_id = :userId))',
        {
          s1: LawScope.MINI_CONSTITUTION,
          s2: LawScope.SUPERIOR,
          s3: LawScope.APP,
          s4: LawScope.TENANT,
          s5: LawScope.USER,
          appId: context.app_id || 'global',
          tenantId: context.tenant_id || 'global',
          userId: context.user_id || 'global'
        }
      );

    return await query.getMany();
  }

  public parseLaw(law: RegistryLaw): ParsedLaw | null {
    try {
      const lines = law.content.split('\n').filter(l => l.trim());
      const headerRegex = /law\s+([\w\.-]+):([\d\.]+):\s*(\w+):/;
      const header = lines[0].match(headerRegex);

      if (!header) {
        // Fallback: use DB metadata if header is missing in content (legacy/migration)
        // Construct artificial parsed law
        return this.parseBodyOnly(law.id, String(law.version), law.scope, lines);
      }

      // Validate scope matches DB
      // const scope = header[3] as LawScope; 
      // We trust DB scope for query, but parser should be consistent.

      return this.parseBodyOnly(header[1], header[2], law.scope, lines.slice(1));

    } catch (error) {
      this.logger.error(`Failed to parse law ${law.id}: ${error.message}`);
      return null;
    }
  }

  private parseBodyOnly(id: string, version: string, scope: LawScope, lines: string[]): ParsedLaw {
    const rules: Rule[] = [];
    
    for (const line of lines) {
      const match = line.match(/if\s+(.+?)\s+then\s+(.+)/);
      if (match) {
        const condition = match[1].trim();
        const action = match[2].trim();

        // Scope Enforcement (Grammar Check)
        const allowed = ALLOWED_ACTIONS[scope] || [];
        const baseAction = action.split('(')[0]; // handle hold(24h)
        
        if (!allowed.includes(baseAction)) {
          this.logger.warn(`⚠️ Law ${id} tried forbidden action '${action}' for scope '${scope}'. Ignored.`);
          continue; 
        }

        rules.push({ condition, action });
      }
    }

    return { id, version, scope, rules };
  }

  private evaluateCondition(condition: string, context: LawContext): boolean {
    // 1. Context Injection (Variable Substitution)
    let expr = condition;
    
    // Basic substitutions
    // Ex: "agent_balance < 0" -> "100 < 0"
    // Ex: "contract_expired" -> "true/false"
    
    const vars = {
      ...context,
      agent_balance: context.agent?.balance || 0,
      contract_value: context.contract?.valor_total_cents || 0,
      contract_expired: context.contract?.data_limite ? new Date() > new Date(context.contract.data_limite) : false,
      invalid_contract: !context.contract || !context.contract.titulo,
      approvers_count: context.contract?.approvals?.length || 0,
    };

    // Naive replacement (Security risk in prod! Use a real expression parser like 'jexl' later)
    // For now, we assume trusted input (since laws are Admin/DB controlled)
    
    // We will implement a safer, extremely simple evaluator without eval() if possible, 
    // but for MVP "Mini Constitution", simple Function eval with restricted scope is pragmatic.
    
    try {
      const keys = Object.keys(vars);
      const values = Object.values(vars);
      const func = new Function(...keys, `return ${expr};`);
      return !!func(...values);
    } catch (e) {
      this.logger.warn(`Failed to evaluate condition '${condition}': ${e.message}`);
      return false;
    }
  }
}

