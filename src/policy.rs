use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use crate::ast::{DeclarationItem, ImportGroup};
use crate::leakguard::LeakGuard;

#[derive(Debug, Deserialize)]
pub struct GraftPolicy {
    #[serde(default)]
    pub rules: PolicyRules,
}

#[derive(Debug, Deserialize)]
pub struct PolicyRules {
    #[serde(default)]
    pub forbid_cross_import: Vec<CrossImportRule>,
    #[serde(default)]
    pub max_function_lines: Option<usize>,
    #[serde(default = "default_block_secrets")]
    pub block_leaked_secrets: bool,
}

fn default_block_secrets() -> bool {
    true
}

impl Default for PolicyRules {
    fn default() -> Self {
        Self {
            forbid_cross_import: Vec::new(),
            max_function_lines: None,
            block_leaked_secrets: true,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CrossImportRule {
    pub from: String,
    pub to: String,
}

pub struct PolicyEngine {
    policy: Option<GraftPolicy>,
}

impl PolicyEngine {
    pub fn load() -> Self {
        let path = Path::new("graft.policy.toml");
        if !path.exists() {
            return Self { policy: None };
        }

        let policy = fs::read_to_string(path)
            .ok()
            .and_then(|data| toml::from_str::<GraftPolicy>(&data).ok());

        Self { policy }
    }

    pub fn validate(
        &self,
        file_path: &Path,
        imports: &BTreeMap<String, ImportGroup>,
        declarations: &[DeclarationItem],
    ) -> Result<(), String> {
        let should_block_secrets = self.policy.as_ref().map(|p| p.rules.block_leaked_secrets).unwrap_or(true);

        if should_block_secrets {
            for decl in declarations {
                let leaks = LeakGuard::scan(&decl.text);
                if !leaks.is_empty() {
                    return Err(format!(
                        "Security Violation (LeakGuard): High-entropy secret detected in '{}': {}",
                        decl.key.as_deref().unwrap_or("unknown"),
                        leaks.join(", ")
                    ));
                }
            }
        }

        let policy = match &self.policy {
            Some(p) => p,
            None => return Ok(()),
        };

        let path_str = file_path.to_string_lossy().replace('\\', "/");

        for rule in &policy.rules.forbid_cross_import {
            if path_str.contains(&rule.from.replace("/**", "")) {
                for target_import in imports.keys() {
                    if target_import.contains(&rule.to.replace("/**", "")) {
                        return Err(format!(
                            "Architectural Violation: '{}' is prohibited from importing '{}' by policy rule",
                            rule.from, target_import
                        ));
                    }
                }
            }
        }

        if let Some(max_lines) = policy.rules.max_function_lines {
            for decl in declarations {
                let lines = decl.text.lines().count();
                if lines > max_lines {
                    return Err(format!(
                        "Architectural Violation: Declaration '{}' has {} lines, exceeding policy limit of {}",
                        decl.key.as_deref().unwrap_or("unknown"),
                        lines,
                        max_lines
                    ));
                }
            }
        }

        Ok(())
    }
}