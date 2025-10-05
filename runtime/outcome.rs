use serde_json::Value;

/// Final decision from applying all matched rules.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    Allow,
    Reject { reason: String },
    Simulate { note: Option<String> },
}

impl Decision {
    pub fn merge(self, other: Decision) -> Decision {
        use Decision::*;
        match (self, other) {
            (Reject { .. }, Reject { reason }) => Reject { reason },
            (Reject { reason }, _) => Reject { reason },
            (_, Reject { reason }) => Reject { reason },
            (Simulate { note }, Simulate { note: other }) => Simulate {
                note: other.or(note),
            },
            (Simulate { note }, Allow) => Simulate { note },
            (Allow, Simulate { note }) => Simulate { note },
            (Allow, Allow) => Allow,
        }
    }
}

impl Default for Decision {
    fn default() -> Self {
        Decision::Allow
    }
}

/// Aggregated view of how rules affected the span.
#[derive(Debug, Clone, PartialEq)]
pub struct EnforcementOutcome {
    pub decision: Decision,
    pub applied_rules: Vec<String>,
    pub added_tags: Vec<String>,
    pub metadata_updates: Vec<(String, Value)>,
    pub notes: Vec<String>,
}

impl EnforcementOutcome {
    pub fn new() -> Self {
        Self {
            decision: Decision::Allow,
            applied_rules: Vec::new(),
            added_tags: Vec::new(),
            metadata_updates: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn record_rule(&mut self, id: impl Into<String>) {
        self.applied_rules.push(id.into());
    }

    pub fn push_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.added_tags.contains(&tag) {
            self.added_tags.push(tag);
        }
    }

    pub fn push_metadata(&mut self, key: impl Into<String>, value: Value) {
        let key = key.into();
        if let Some(existing) = self
            .metadata_updates
            .iter_mut()
            .find(|(existing_key, _)| existing_key == &key)
        {
            existing.1 = value;
        } else {
            self.metadata_updates.push((key, value));
        }
    }

    pub fn push_note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    pub fn update_decision(&mut self, new_decision: Decision) {
        let current = std::mem::take(&mut self.decision);
        self.decision = current.merge(new_decision);
    }

    pub fn is_reject(&self) -> bool {
        matches!(self.decision, Decision::Reject { .. })
    }
}

impl Default for EnforcementOutcome {
    fn default() -> Self {
        Self::new()
    }
}
