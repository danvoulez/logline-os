use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::RuleError;
use crate::rule::Rule;

pub fn load_rules(path: impl AsRef<Path>) -> Result<Vec<Rule>, RuleError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(RuleError::MissingPath(path.display().to_string()));
    }

    let mut rules = if path.is_dir() {
        load_from_directory(path)?
    } else {
        load_from_file(path)?
    };

    deduplicate(&mut rules)?;
    rules.sort_by(|a, b| a.priority.cmp(&b.priority).then(a.id.cmp(&b.id)));

    Ok(rules)
}

fn load_from_directory(path: &Path) -> Result<Vec<Rule>, RuleError> {
    let mut rules = Vec::new();
    for entry in fs::read_dir(path).map_err(|err| RuleError::from_io(path, err))? {
        let entry = entry.map_err(|err| RuleError::from_io(path, err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| RuleError::from_io(entry.path(), err))?;
        if file_type.is_dir() {
            continue;
        }

        if let Some(ext) = entry.path().extension().and_then(|value| value.to_str()) {
            if matches!(ext, "json" | "yaml" | "yml") {
                let mut file_rules = load_from_file(&entry.path())?;
                rules.append(&mut file_rules);
            }
        }
    }

    Ok(rules)
}

fn load_from_file(path: &Path) -> Result<Vec<Rule>, RuleError> {
    let raw = fs::read_to_string(path).map_err(|err| RuleError::from_io(path, err))?;
    parse_rules(&raw, path)
}

fn parse_rules(raw: &str, path: &Path) -> Result<Vec<Rule>, RuleError> {
    let mut attempts = Vec::new();

    if let Ok(doc) = serde_yaml::from_str::<RuleDocument>(raw) {
        return Ok(doc.rules);
    }

    attempts.push("rules document".to_string());

    if let Ok(list) = serde_yaml::from_str::<Vec<Rule>>(raw) {
        return Ok(list);
    }

    attempts.push("list".to_string());

    if let Ok(rule) = serde_yaml::from_str::<Rule>(raw) {
        return Ok(vec![rule]);
    }

    attempts.push("single".to_string());

    let message = format!("unable to parse rules file using {:?} formats", attempts);
    Err(RuleError::parse_error(path.to_path_buf(), message))
}

fn deduplicate(rules: &mut [Rule]) -> Result<(), RuleError> {
    let mut seen = HashSet::new();
    for rule in rules.iter() {
        if !seen.insert(rule.id.clone()) {
            return Err(RuleError::DuplicateRule {
                id: rule.id.clone(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct RuleDocument {
    rules: Vec<Rule>,
}
