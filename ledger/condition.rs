use logline_protocol::timeline::Span;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON pointer-like field path used to inspect attributes on a [`Span`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct FieldPath(String);

impl FieldPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('.').filter(|segment| !segment.is_empty())
    }

    fn locate<'a>(&self, root: &'a Value) -> Option<&'a Value> {
        let mut current = root;
        for segment in self.segments() {
            match current {
                Value::Object(map) => match map.get(segment) {
                    Some(value) => current = value,
                    None => return None,
                },
                Value::Array(items) => {
                    let index: usize = segment.parse().ok()?;
                    current = items.get(index)?;
                }
                _ => return None,
            }
        }
        Some(current)
    }
}

impl From<&str> for FieldPath {
    fn from(value: &str) -> Self {
        FieldPath::new(value)
    }
}

impl From<String> for FieldPath {
    fn from(value: String) -> Self {
        FieldPath::new(value)
    }
}

/// Conditional expression that determines when a rule should trigger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleCondition {
    /// Matches all spans.
    Always,
    /// All nested conditions must return true.
    All { conditions: Vec<RuleCondition> },
    /// Any of the nested conditions must return true.
    Any { conditions: Vec<RuleCondition> },
    /// Negate the outcome of the nested condition.
    Not { condition: Box<RuleCondition> },
    /// Compare the value at `field` for equality.
    Equals { field: FieldPath, value: Value },
    /// Compare the value at `field` for inequality.
    NotEquals { field: FieldPath, value: Value },
    /// Ensure the field exists within the serialized span.
    Exists { field: FieldPath },
    /// Ensure the field is missing from the serialized span.
    Missing { field: FieldPath },
    /// Check if the given field contains a textual snippet.
    ContainsText { field: FieldPath, text: String },
    /// Whether the span already has the provided tag.
    ContainsTag { tag: String },
    /// Whether the numerical value at `field` is greater than the provided value.
    GreaterThan { field: FieldPath, value: f64 },
    /// Whether the numerical value at `field` is less than the provided value.
    LessThan { field: FieldPath, value: f64 },
}

impl RuleCondition {
    pub fn always() -> Self {
        RuleCondition::Always
    }

    pub fn evaluate(&self, span: &Span, snapshot: &Value) -> bool {
        match self {
            RuleCondition::Always => true,
            RuleCondition::All { conditions } => conditions
                .iter()
                .all(|condition| condition.evaluate(span, snapshot)),
            RuleCondition::Any { conditions } => conditions
                .iter()
                .any(|condition| condition.evaluate(span, snapshot)),
            RuleCondition::Not { condition } => !condition.evaluate(span, snapshot),
            RuleCondition::Equals { field, value } => field
                .locate(snapshot)
                .map(|actual| values_equal(actual, value))
                .unwrap_or(false),
            RuleCondition::NotEquals { field, value } => !field
                .locate(snapshot)
                .map(|actual| values_equal(actual, value))
                .unwrap_or(false),
            RuleCondition::Exists { field } => field.locate(snapshot).is_some(),
            RuleCondition::Missing { field } => field.locate(snapshot).is_none(),
            RuleCondition::ContainsText { field, text } => field
                .locate(snapshot)
                .and_then(|value| value.as_str())
                .map(|candidate| candidate.contains(text))
                .unwrap_or(false),
            RuleCondition::ContainsTag { tag } => span.tags.iter().any(|existing| existing == tag),
            RuleCondition::GreaterThan { field, value } => field
                .locate(snapshot)
                .and_then(Value::as_f64)
                .map(|candidate| candidate > *value)
                .unwrap_or(false),
            RuleCondition::LessThan { field, value } => field
                .locate(snapshot)
                .and_then(Value::as_f64)
                .map(|candidate| candidate < *value)
                .unwrap_or(false),
        }
    }
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(lhs), Value::Number(rhs)) => match (lhs.as_f64(), rhs.as_f64()) {
            (Some(l), Some(r)) => (l - r).abs() < f64::EPSILON,
            _ => lhs == rhs,
        },
        _ => left == right,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_nested_fields() {
        let path = FieldPath::from("metadata.author.name");
        let value = json!({
            "metadata": {"author": {"name": "Ada"}}
        });

        assert_eq!(path.locate(&value).and_then(Value::as_str), Some("Ada"));
    }
}
