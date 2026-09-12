use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRename {
    pub old_name: String,
    pub new_name: String,
    pub symbol_type: String,
}

pub struct SemanticAnalyzer;

impl SemanticAnalyzer {
    pub fn detect_renames(
        base_keys: &[String],
        target_keys: &[String],
        base_map: &HashMap<String, String>,
        target_map: &HashMap<String, String>,
    ) -> Vec<SymbolRename> {
        let mut renames = Vec::new();
        let base_set: HashSet<_> = base_keys.iter().collect();
        let target_set: HashSet<_> = target_keys.iter().collect();

        let removed: Vec<_> = base_keys.iter().filter(|k| !target_set.contains(k)).collect();
        let added: Vec<_> = target_keys.iter().filter(|k| !base_set.contains(k)).collect();

        for rem in &removed {
            let rem_parts: Vec<&str> = rem.split(':').collect();
            if rem_parts.len() < 2 {
                continue;
            }
            let rem_type = rem_parts[0];
            let rem_name = rem_parts[1];

            let rem_body = match base_map.get(*rem) {
                Some(b) => b,
                None => continue,
            };

            for add in &added {
                let add_parts: Vec<&str> = add.split(':').collect();
                if add_parts.len() < 2 {
                    continue;
                }
                let add_type = add_parts[0];
                let add_name = add_parts[1];

                if rem_type == add_type && rem_name != add_name {
                    if let Some(add_body) = target_map.get(*add) {
                        if Self::calculate_similarity(rem_body, add_body) > 0.65 {
                            renames.push(SymbolRename {
                                old_name: rem_name.to_string(),
                                new_name: add_name.to_string(),
                                symbol_type: rem_type.to_string(),
                            });
                        }
                    }
                }
            }
        }
        renames
    }

    pub fn check_dangling_references(
        renames: &[SymbolRename],
        incoming_code: &str,
    ) -> Vec<String> {
        let mut conflicts = Vec::new();
        for rename in renames {
            let pattern = format!("{}(", rename.old_name);
            let direct_ref = format!(" {}", rename.old_name);
            if incoming_code.contains(&pattern) || incoming_code.contains(&direct_ref) {
                conflicts.push(format!(
                    "Semantic Conflict: '{}' was renamed to '{}', but referencing code was added",
                    rename.old_name, rename.new_name
                ));
            }
        }
        conflicts
    }

    fn calculate_similarity(s1: &str, s2: &str) -> f64 {
        let words1: HashSet<&str> = s1.split_whitespace().collect();
        let words2: HashSet<&str> = s2.split_whitespace().collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            1.0
        } else {
            intersection as f64 / union as f64
        }
    }
}