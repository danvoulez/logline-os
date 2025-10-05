use logline_protocol::timeline::Span;
use serde_json::Value;
use tracing::debug;

use crate::action::RuleAction;
use crate::error::RuleError;
use crate::loader::load_rules;
use crate::outcome::{Decision, EnforcementOutcome};
use crate::rule::Rule;

#[cfg(test)]
use crate::condition::RuleCondition;

/// Runtime executor that evaluates spans against a set of rules.
#[derive(Debug, Default, Clone)]
pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    /// Construct an engine from the provided rules, sorting them by priority.
    pub fn new(mut rules: Vec<Rule>) -> Self {
        rules.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));
        Self { rules }
    }

    /// Loads rules from the given path (file or directory).
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, RuleError> {
        let rules = load_rules(path)?;
        Ok(Self::new(rules))
    }

    /// Borrow the underlying rule set.
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Whether the engine contains no rules.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Evaluate a span and mutate it according to any triggered actions.
    pub fn apply(&self, span: &mut Span) -> EnforcementOutcome {
        let mut outcome = EnforcementOutcome::new();

        for rule in &self.rules {
            if !rule.is_enabled() {
                continue;
            }

            let snapshot = serde_json::to_value(&span).unwrap_or(Value::Null);
            if !rule.condition.evaluate(span, &snapshot) {
                continue;
            }

            debug!(rule_id = %rule.id, "rule matched span");
            outcome.record_rule(rule.id.clone());
            if let Some(description) = &rule.description {
                outcome.push_note(description.clone());
            }

            for action in &rule.actions {
                apply_action(span, action, &mut outcome);
                if outcome.is_reject() {
                    debug!(rule_id = %rule.id, "rule rejected span");
                    return outcome;
                }
            }
        }

        outcome
    }

    /// Evaluate a span without mutating it, returning the outcome.
    pub fn evaluate(&self, span: &Span) -> EnforcementOutcome {
        let mut clone = span.clone();
        self.apply(&mut clone)
    }
}

fn apply_action(span: &mut Span, action: &RuleAction, outcome: &mut EnforcementOutcome) {
    match action {
        RuleAction::Allow => outcome.update_decision(Decision::Allow),
        RuleAction::Reject { reason } => {
            outcome.update_decision(Decision::Reject {
                reason: reason.clone(),
            });
        }
        RuleAction::Simulate { note } => {
            outcome.update_decision(Decision::Simulate { note: note.clone() });
            if let Some(note) = note {
                outcome.push_note(note.clone());
            }
        }
        RuleAction::AddTag { tag } => {
            span.add_tag(tag.clone());
            outcome.push_tag(tag.clone());
        }
        RuleAction::SetMetadata { key, value } => {
            span.add_metadata(key.clone(), value.clone());
            outcome.push_metadata(key.clone(), value.clone());
        }
        RuleAction::MarkProcessed => {
            span.mark_processed();
        }
        RuleAction::Note { message } => {
            outcome.push_note(message.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logline_protocol::timeline::{SpanBuilder, SpanStatus};
    use serde_json::json;

    fn build_span() -> Span {
        SpanBuilder::new("node", "example span")
            .status(SpanStatus::Executed)
            .build()
    }

    #[test]
    fn applies_matching_rule() {
        let rule = Rule {
            id: "allow".into(),
            description: Some("allow processed spans".into()),
            priority: 10,
            enabled: true,
            labels: vec![],
            condition: RuleCondition::Always,
            actions: vec![RuleAction::MarkProcessed],
        };
        let engine = RuleEngine::new(vec![rule]);
        let mut span = build_span();

        let outcome = engine.apply(&mut span);
        assert!(span.processed);
        assert_eq!(outcome.applied_rules, vec!["allow".to_string()]);
    }

    #[test]
    fn rejects_span_when_condition_matches() {
        let rule = Rule {
            id: "deny".into(),
            description: None,
            priority: 1,
            enabled: true,
            labels: vec![],
            condition: RuleCondition::Equals {
                field: "title".into(),
                value: json!("example span"),
            },
            actions: vec![RuleAction::Reject {
                reason: "blocked by rule".into(),
            }],
        };

        let engine = RuleEngine::new(vec![rule]);
        let mut span = build_span();
        let outcome = engine.apply(&mut span);

        assert!(matches!(outcome.decision, Decision::Reject { .. }));
        assert!(outcome.is_reject());
    }
}
