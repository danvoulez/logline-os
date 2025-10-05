use crate::action::RuleAction;
use crate::condition::RuleCondition;
use serde::{Deserialize, Serialize};

/// Declarative rule definition applied to spans on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    /// Unique identifier for the rule. Used for reporting and deduplication.
    pub id: String,
    /// Optional human readable description.
    #[serde(default)]
    pub description: Option<String>,
    /// Ordering priority. Lower numbers are evaluated first.
    #[serde(default = "Rule::default_priority")]
    pub priority: u32,
    /// Whether the rule is active.
    #[serde(default = "Rule::default_enabled")]
    pub enabled: bool,
    /// Additional labels for reporting / filtering.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Matching condition for the rule.
    #[serde(default = "RuleCondition::always")]
    pub condition: RuleCondition,
    /// Actions executed when the condition matches.
    #[serde(default)]
    pub actions: Vec<RuleAction>,
}

impl Rule {
    pub fn default_priority() -> u32 {
        100
    }

    pub fn default_enabled() -> bool {
        true
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
