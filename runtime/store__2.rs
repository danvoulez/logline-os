use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Rule, RuleEngine, RuleError};

/// Versioned history entry for a stored rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleHistoryEntry {
    pub version: u32,
    pub rule: Rule,
    pub created_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

impl RuleHistoryEntry {
    fn new(version: u32, rule: Rule, updated_by: Option<String>) -> Self {
        Self {
            version,
            rule,
            created_at: Utc::now(),
            updated_by,
        }
    }
}

#[derive(Default)]
struct TenantRules {
    rules: HashMap<String, Vec<RuleHistoryEntry>>,
}

/// In-memory multi-tenant rule store with version tracking.
#[derive(Default, Clone)]
pub struct RuleStore {
    inner: Arc<RwLock<HashMap<String, TenantRules>>>,
}

impl RuleStore {
    /// Creates a new empty rule store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the list of tenants currently tracked.
    pub fn tenants(&self) -> Vec<String> {
        let inner = self.inner.read();
        inner.keys().cloned().collect()
    }

    /// Returns the latest rule versions for the provided tenant.
    pub fn list_rules(&self, tenant: &str) -> Vec<RuleHistoryEntry> {
        let inner = self.inner.read();
        inner
            .get(tenant)
            .map(|rules| {
                rules
                    .rules
                    .values()
                    .filter_map(|versions| versions.last().cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns the full history for a specific rule.
    pub fn rule_history(&self, tenant: &str, rule_id: &str) -> Vec<RuleHistoryEntry> {
        let inner = self.inner.read();
        inner
            .get(tenant)
            .and_then(|rules| rules.rules.get(rule_id).cloned())
            .unwrap_or_default()
    }

    /// Returns the latest version of a rule, if available.
    pub fn latest_rule(&self, tenant: &str, rule_id: &str) -> Option<RuleHistoryEntry> {
        let inner = self.inner.read();
        inner
            .get(tenant)
            .and_then(|rules| rules.rules.get(rule_id))
            .and_then(|versions| versions.last().cloned())
    }

    /// Inserts or updates a rule. Returning the new history entry.
    pub fn put_rule(
        &self,
        tenant: &str,
        mut rule: Rule,
        updated_by: Option<String>,
    ) -> RuleHistoryEntry {
        let mut inner = self.inner.write();
        let tenant_rules = inner.entry(tenant.to_string()).or_default();

        // Ensure the rule id is set. If blank, generate a random id.
        if rule.id.trim().is_empty() {
            rule.id = format!("rule-{}", Uuid::new_v4());
        }

        let entry = tenant_rules.rules.entry(rule.id.clone()).or_default();

        let version = entry.last().map(|last| last.version + 1).unwrap_or(1);
        let history_entry = RuleHistoryEntry::new(version, rule, updated_by);
        entry.push(history_entry.clone());
        history_entry
    }

    /// Disables a rule for the tenant by appending a new version with `enabled = false`.
    pub fn disable_rule(
        &self,
        tenant: &str,
        rule_id: &str,
        updated_by: Option<String>,
    ) -> Result<RuleHistoryEntry, RuleError> {
        let mut inner = self.inner.write();
        let tenant_rules = inner
            .get_mut(tenant)
            .ok_or_else(|| RuleError::NotFound(rule_id.to_string()))?;

        let history = tenant_rules
            .rules
            .get_mut(rule_id)
            .ok_or_else(|| RuleError::NotFound(rule_id.to_string()))?;

        let latest = history
            .last()
            .cloned()
            .ok_or_else(|| RuleError::NotFound(rule_id.to_string()))?;

        if !latest.rule.enabled {
            return Ok(latest);
        }

        let mut disabled_rule = latest.rule.clone();
        disabled_rule.enabled = false;
        let version = latest.version + 1;
        let entry = RuleHistoryEntry::new(version, disabled_rule, updated_by);
        history.push(entry.clone());
        Ok(entry)
    }

    /// Builds a rule engine using the latest active rules for a tenant.
    pub fn engine_for(&self, tenant: &str) -> RuleEngine {
        let rules = self
            .list_rules(tenant)
            .into_iter()
            .filter(|entry| entry.rule.enabled)
            .map(|entry| entry.rule)
            .collect();
        RuleEngine::new(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuleCondition;

    fn sample_rule(id: &str) -> Rule {
        Rule {
            id: id.to_string(),
            description: Some("demo".into()),
            priority: 10,
            enabled: true,
            labels: vec!["demo".into()],
            condition: RuleCondition::Always,
            actions: vec![],
        }
    }

    #[test]
    fn versioning_is_tracked() {
        let store = RuleStore::new();
        let entry1 = store.put_rule("tenant-a", sample_rule("allow"), None);
        assert_eq!(entry1.version, 1);

        let mut updated_rule = entry1.rule.clone();
        updated_rule.description = Some("updated".into());
        let entry2 = store.put_rule("tenant-a", updated_rule, Some("alice".into()));
        assert_eq!(entry2.version, 2);
        assert_eq!(entry2.updated_by.as_deref(), Some("alice"));

        let history = store.rule_history("tenant-a", "allow");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 1);
        assert_eq!(history[1].version, 2);
    }

    #[test]
    fn disabling_rule_creates_new_version() {
        let store = RuleStore::new();
        let entry = store.put_rule("tenant-a", sample_rule("deny"), None);
        assert!(entry.rule.enabled);

        let disabled = store
            .disable_rule("tenant-a", "deny", Some("system".into()))
            .expect("disable rule");
        assert!(!disabled.rule.enabled);
        assert_eq!(disabled.version, entry.version + 1);

        let engine = store.engine_for("tenant-a");
        assert!(engine.is_empty(), "disabled rules should be skipped");
    }
}
