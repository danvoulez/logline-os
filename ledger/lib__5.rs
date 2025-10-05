//! Rule evaluation engine for the LogLine ecosystem.
//!
//! This crate exposes a declarative rule system used by services such as the
//! timeline microservice to enforce policies before spans are persisted. Rules
//! are expressed as YAML/JSON documents that define matching conditions and
//! actions to perform when a span satisfies those conditions.

mod action;
mod condition;
mod engine;
mod error;
mod loader;
mod outcome;
mod rule;
mod service;
mod store;
mod ws_client;

pub use action::RuleAction;
pub use condition::{FieldPath, RuleCondition};
pub use engine::RuleEngine;
pub use error::RuleError;
pub use outcome::{Decision, EnforcementOutcome};
pub use rule::Rule;
pub use service::{RuleApiBuilder, RuleServiceConfig};
pub use store::{RuleHistoryEntry, RuleStore};

#[cfg(test)]
mod tests {
    use super::*;
    use logline_protocol::timeline::SpanBuilder;
    use serde_json::json;

    #[test]
    fn evaluates_simple_rule() {
        let rule = Rule {
            id: "allow".into(),
            description: None,
            priority: 1,
            enabled: true,
            labels: vec![],
            condition: RuleCondition::Equals {
                field: FieldPath::from("title"),
                value: json!("demo"),
            },
            actions: vec![RuleAction::AddTag {
                tag: "matched".into(),
            }],
        };

        let engine = RuleEngine::new(vec![rule]);
        let mut span = SpanBuilder::new("node", "demo").build();
        let outcome = engine.apply(&mut span);

        assert_eq!(outcome.applied_rules, vec!["allow".to_string()]);
        assert!(span.tags.iter().any(|tag| tag == "matched"));
    }
}
