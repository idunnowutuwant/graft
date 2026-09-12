use crate::ai::AiIntentMerger;
use crate::ast::{parse_module, DeclarationItem, ImportGroup, ImportSpecifier};
use crate::explainer::{MergeLedger, MergeResolutionKind};
use crate::policy::PolicyEngine;
use crate::semantic::SemanticAnalyzer;
use crate::session::SessionManager;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeFailure {
    Conflict(String),
    SystemError,
}

pub fn merge_module(
    base: &str,
    ours: &str,
    theirs: &str,
    is_tsx: bool,
    enable_ai: bool,
    file_path: &Path,
) -> Result<String, MergeFailure> {
    let parsed_base = parse_module(base, is_tsx).ok_or(MergeFailure::SystemError)?;
    let parsed_ours = parse_module(ours, is_tsx).ok_or(MergeFailure::SystemError)?;
    let parsed_theirs = parse_module(theirs, is_tsx).ok_or(MergeFailure::SystemError)?;

    let mut has_conflict = false;
    let mut ledger = MergeLedger::new();
    let policy_engine = PolicyEngine::load();

    let merged_preamble = merge_text_block(
        &parsed_base.preamble,
        &parsed_ours.preamble,
        &parsed_theirs.preamble,
        &mut has_conflict,
    );

    let session = SessionManager::new();

    let merged_imports = merge_import_maps(
        &parsed_base.imports,
        &parsed_ours.imports,
        &parsed_theirs.imports,
        &session,
        &mut ledger,
    )?;

    let merged_declarations = merge_declarations(
        &parsed_base.declarations,
        &parsed_ours.declarations,
        &parsed_theirs.declarations,
        &session,
        enable_ai,
        &mut ledger,
        &mut has_conflict,
    );

    if let Err(policy_err) = policy_engine.validate(file_path, &merged_imports, &parsed_ours.declarations) {
        return Err(MergeFailure::Conflict(format!(
            "<<<<<<< OURS\n/* {} */\n=======\n>>>>>>> THEIRS\n{}",
            policy_err, ours
        )));
    }

    let rendered_imports = render_imports(&merged_imports);

    let mut output = String::new();

    if !merged_preamble.trim().is_empty() {
        output.push_str(merged_preamble.trim());
        output.push_str("\n\n");
    }

    if !rendered_imports.is_empty() {
        output.push_str(&rendered_imports);
        output.push('\n');
    }

    if !merged_declarations.is_empty() {
        output.push_str(&merged_declarations);
        output.push('\n');
    }

    if has_conflict {
        Err(MergeFailure::Conflict(output))
    } else {
        ledger.print_briefing(file_path);
        Ok(output)
    }
}

pub fn merge_text_block(base: &str, ours: &str, theirs: &str, has_conflict: &mut bool) -> String {
    if ours == theirs {
        return ours.to_string();
    }
    if ours == base {
        return theirs.to_string();
    }
    if theirs == base {
        return ours.to_string();
    }

    match diffy::merge(base, ours, theirs) {
        Ok(merged) => merged,
        Err(conflict) => {
            *has_conflict = true;
            conflict
        }
    }
}

pub fn align_3way_keys(base: &[String], ours: &[String], theirs: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    let mut bi = 0;
    let mut oi = 0;
    let mut ti = 0;

    for b_key in base {
        while oi < ours.len() && ours[oi] != *b_key && !base[bi..].contains(&ours[oi]) {
            if seen.insert(ours[oi].clone()) {
                result.push(ours[oi].clone());
            }
            oi += 1;
        }

        while ti < theirs.len() && theirs[ti] != *b_key && !base[bi..].contains(&theirs[ti]) {
            if seen.insert(theirs[ti].clone()) {
                result.push(theirs[ti].clone());
            }
            ti += 1;
        }

        if seen.insert(b_key.clone()) {
            result.push(b_key.clone());
        }

        if oi < ours.len() && ours[oi] == *b_key {
            oi += 1;
        }
        if ti < theirs.len() && theirs[ti] == *b_key {
            ti += 1;
        }
        bi += 1;
    }

    while oi < ours.len() {
        if seen.insert(ours[oi].clone()) {
            result.push(ours[oi].clone());
        }
        oi += 1;
    }

    while ti < theirs.len() {
        if seen.insert(theirs[ti].clone()) {
            result.push(theirs[ti].clone());
        }
        ti += 1;
    }

    result
}

fn merge_declarations(
    base: &[DeclarationItem],
    ours: &[DeclarationItem],
    theirs: &[DeclarationItem],
    session: &SessionManager,
    enable_ai: bool,
    ledger: &mut MergeLedger,
    has_conflict: &mut bool,
) -> String {
    let mut base_map = BTreeMap::new();
    let mut base_keys = Vec::new();
    let mut base_hash_map = HashMap::new();
    for item in base {
        if let Some(k) = &item.key {
            base_map.insert(k.clone(), item.text.clone());
            base_keys.push(k.clone());
            base_hash_map.insert(k.clone(), item.text.clone());
        }
    }

    let mut ours_map = BTreeMap::new();
    let mut ours_keys = Vec::new();
    let mut ours_hash_map = HashMap::new();
    for item in ours {
        if let Some(k) = &item.key {
            ours_map.insert(k.clone(), item.text.clone());
            ours_keys.push(k.clone());
            ours_hash_map.insert(k.clone(), item.text.clone());
        }
    }

    let mut theirs_map = BTreeMap::new();
    let mut theirs_keys = Vec::new();
    let mut theirs_hash_map = HashMap::new();
    for item in theirs {
        if let Some(k) = &item.key {
            theirs_map.insert(k.clone(), item.text.clone());
            theirs_keys.push(k.clone());
            theirs_hash_map.insert(k.clone(), item.text.clone());
        }
    }

    let renames_ours = SemanticAnalyzer::detect_renames(
        &base_keys,
        &ours_keys,
        &base_hash_map,
        &ours_hash_map,
    );
    for r in &renames_ours {
        session.record_rename(&r.old_name, &r.new_name);
    }

    let renames_theirs = SemanticAnalyzer::detect_renames(
        &base_keys,
        &theirs_keys,
        &base_hash_map,
        &theirs_hash_map,
    );
    for r in &renames_theirs {
        session.record_rename(&r.old_name, &r.new_name);
    }

    let sequence = align_3way_keys(&base_keys, &ours_keys, &theirs_keys);
    let mut resolved_chunks = Vec::new();
    let ai_engine = if enable_ai {
        let engine = AiIntentMerger::new();
        if engine.is_available() {
            Some(engine)
        } else {
            None
        }
    } else {
        None
    };

    for key in sequence {
        let b = base_map.get(&key);
        let o = ours_map.get(&key);
        let t = theirs_map.get(&key);

        match (b, o, t) {
            (Some(bv), Some(ov), Some(tv)) => {
                if ov == tv {
                    resolved_chunks.push(ov.clone());
                    ledger.record(MergeResolutionKind::OursDeclarationKept(key.clone()));
                } else if ov == bv {
                    resolved_chunks.push(tv.clone());
                    ledger.record(MergeResolutionKind::TheirsDeclarationKept(key.clone()));
                } else if tv == bv {
                    resolved_chunks.push(ov.clone());
                    ledger.record(MergeResolutionKind::OursDeclarationKept(key.clone()));
                } else {
                    let mut handled_by_ai = false;
                    if let Some(ref ai) = ai_engine {
                        if let Some(ai_code) = ai.resolve_intent(bv, ov, tv, &key) {
                            resolved_chunks.push(ai_code.clone());
                            ledger.record(MergeResolutionKind::AiSynthesized(key.clone()));
                            handled_by_ai = true;
                        }
                    }

                    if !handled_by_ai {
                        match diffy::merge(bv, ov, tv) {
                            Ok(res) => {
                                resolved_chunks.push(res.clone());
                                ledger.record(MergeResolutionKind::SynthesizedBoth(key.clone()));
                            }
                            Err(conf) => {
                                *has_conflict = true;
                                resolved_chunks.push(conf);
                            }
                        }
                    }
                }
            }
            (None, Some(ov), None) => {
                let dangling = SemanticAnalyzer::check_dangling_references(&renames_theirs, ov);
                if !dangling.is_empty() {
                    *has_conflict = true;
                    resolved_chunks.push(format!("/* {} */\n{}", dangling.join("\n"), ov));
                } else {
                    resolved_chunks.push(ov.clone());
                    ledger.record(MergeResolutionKind::OursDeclarationKept(key.clone()));
                }
            }
            (None, None, Some(tv)) => {
                let dangling = SemanticAnalyzer::check_dangling_references(&renames_ours, tv);
                if !dangling.is_empty() {
                    *has_conflict = true;
                    resolved_chunks.push(format!("/* {} */\n{}", dangling.join("\n"), tv));
                } else {
                    resolved_chunks.push(tv.clone());
                    ledger.record(MergeResolutionKind::TheirsDeclarationKept(key.clone()));
                }
            }
            (None, Some(ov), Some(tv)) => {
                if ov == tv {
                    resolved_chunks.push(ov.clone());
                    ledger.record(MergeResolutionKind::OursDeclarationKept(key.clone()));
                } else {
                    *has_conflict = true;
                    resolved_chunks.push(format!(
                        "<<<<<<< OURS\n{}\n=======\n{}\n>>>>>>> THEIRS",
                        ov, tv
                    ));
                }
            }
            (Some(bv), Some(ov), None) => {
                if ov != bv {
                    *has_conflict = true;
                    resolved_chunks.push(format!(
                        "<<<<<<< OURS\n{}\n=======\n/* DELETED IN THEIRS */\n>>>>>>> THEIRS",
                        ov
                    ));
                }
            }
            (Some(bv), None, Some(tv)) => {
                if tv != bv {
                    *has_conflict = true;
                    resolved_chunks.push(format!(
                        "<<<<<<< OURS\n/* DELETED IN OURS */\n=======\n{}\n>>>>>>> THEIRS",
                        tv
                    ));
                }
            }
            (Some(_), None, None) => {}
            (None, None, None) => {}
        }
    }

    resolved_chunks.join("\n\n")
}

fn merge_import_maps(
    base: &BTreeMap<String, ImportGroup>,
    ours: &BTreeMap<String, ImportGroup>,
    theirs: &BTreeMap<String, ImportGroup>,
    session: &SessionManager,
    ledger: &mut MergeLedger,
) -> Result<BTreeMap<String, ImportGroup>, MergeFailure> {
    let mut all_keys: BTreeSet<&String> = BTreeSet::new();
    all_keys.extend(base.keys());
    all_keys.extend(ours.keys());
    all_keys.extend(theirs.keys());

    let mut resolved = BTreeMap::new();
    let mut unified_count = 0;

    for key in all_keys {
        let b = base.get(key);
        let o = ours.get(key);
        let t = theirs.get(key);

        let mut group = match (b, o, t) {
            (Some(bv), Some(ov), Some(tv)) => {
                if ov == tv {
                    ov.clone()
                } else if ov == bv {
                    tv.clone()
                } else if tv == bv {
                    ov.clone()
                } else {
                    unified_count += 1;
                    merge_single_import(bv, ov, tv)?
                }
            }
            (None, Some(ov), None) => ov.clone(),
            (None, None, Some(tv)) => tv.clone(),
            (None, Some(ov), Some(tv)) => {
                if ov == tv {
                    ov.clone()
                } else {
                    unified_count += 1;
                    let dummy_base = ImportGroup::default();
                    merge_single_import(&dummy_base, ov, tv)?
                }
            }
            (Some(bv), None, Some(tv)) => {
                if bv != tv {
                    return Err(MergeFailure::SystemError);
                }
                continue;
            }
            (Some(bv), Some(ov), None) => {
                if bv != ov {
                    return Err(MergeFailure::SystemError);
                }
                continue;
            }
            (Some(_), None, None) => continue,
            (None, None, None) => continue,
        };

        let mut remapped_specs = BTreeSet::new();
        for spec in group.named_imports {
            if let Some(renamed) = session.resolve_cross_file_symbol(&spec.name) {
                remapped_specs.insert(ImportSpecifier {
                    name: renamed,
                    alias: spec.alias,
                    is_type: spec.is_type,
                });
            } else {
                remapped_specs.insert(spec);
            }
        }
        group.named_imports = remapped_specs;

        resolved.insert(key.clone(), group);
    }

    if unified_count > 0 {
        ledger.record(MergeResolutionKind::ImportUnion(unified_count));
    }

    Ok(resolved)
}

fn merge_single_import(
    _base: &ImportGroup,
    ours: &ImportGroup,
    theirs: &ImportGroup,
) -> Result<ImportGroup, MergeFailure> {
    if ours.is_side_effect || theirs.is_side_effect {
        if ours.is_side_effect && theirs.is_side_effect {
            return Ok(ours.clone());
        }
        return Err(MergeFailure::SystemError);
    }

    let default_import = match (&ours.default_import, &theirs.default_import) {
        (Some(a), Some(b)) if a == b => Some(a.clone()),
        (Some(_), Some(_)) => return Err(MergeFailure::SystemError),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
    };

    let namespace_import = match (&ours.namespace_import, &theirs.namespace_import) {
        (Some(a), Some(b)) if a == b => Some(a.clone()),
        (Some(_), Some(_)) => return Err(MergeFailure::SystemError),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
    };

    let mut named_map: BTreeMap<String, (Option<String>, bool)> = BTreeMap::new();

    for spec in &ours.named_imports {
        named_map.insert(spec.name.clone(), (spec.alias.clone(), spec.is_type));
    }

    for spec in &theirs.named_imports {
        if let Some((existing_alias, existing_is_type)) = named_map.get_mut(&spec.name) {
            if *existing_alias != spec.alias {
                return Err(MergeFailure::SystemError);
            }
            *existing_is_type = *existing_is_type && spec.is_type;
        } else {
            named_map.insert(spec.name.clone(), (spec.alias.clone(), spec.is_type));
        }
    }

    let mut named_imports = BTreeSet::new();
    for (name, (alias, is_type)) in named_map {
        named_imports.insert(ImportSpecifier {
            name,
            alias,
            is_type,
        });
    }

    Ok(ImportGroup {
        is_type_only: ours.is_type_only && theirs.is_type_only,
        default_import,
        namespace_import,
        named_imports,
        is_side_effect: false,
    })
}

fn render_imports(imports: &BTreeMap<String, ImportGroup>) -> String {
    let mut lines = Vec::new();

    for (path, group) in imports {
        if group.is_side_effect {
            lines.push(format!("import \"{}\";", path));
            continue;
        }

        let mut parts = Vec::new();

        if let Some(def) = &group.default_import {
            parts.push(def.clone());
        }

        if let Some(ns) = &group.namespace_import {
            parts.push(format!("* as {}", ns));
        }

        if !group.named_imports.is_empty() {
            let formatted: Vec<String> = group
                .named_imports
                .iter()
                .map(|spec| {
                    let mut s = String::new();
                    if spec.is_type && !group.is_type_only {
                        s.push_str("type ");
                    }
                    s.push_str(&spec.name);
                    if let Some(alias) = &spec.alias {
                        s.push_str(&format!(" as {}", alias));
                    }
                    s
                })
                .collect();
            parts.push(format!("{{ {} }}", formatted.join(", ")));
        }

        let type_prefix = if group.is_type_only { "type " } else { "" };
        if parts.is_empty() {
            lines.push(format!("import \"{}\";", path));
        } else {
            lines.push(format!(
                "import {}{} from \"{}\";",
                type_prefix,
                parts.join(", "),
                path
            ));
        }
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_ast_merge_imports_and_declarations() {
        let base = "import { useState } from \"react\";\n\nexport function A() {}";
        let ours = "import { useState, useEffect } from \"react\";\n\nexport function A() {}\n\nexport function B() {}";
        let theirs = "import { useState, useMemo } from \"react\";\n\nexport function A() {}\n\nexport function C() {}";
        let path = Path::new("test.ts");
        let res = merge_module(base, ours, theirs, false, false, path).unwrap();
        assert!(res.contains("useEffect, useMemo, useState"));
        assert!(res.contains("export function B() {}"));
        assert!(res.contains("export function C() {}"));
    }

    #[test]
    fn test_type_promotion_deduplication() {
        let base = "import { type User } from \"./types\";\n\nexport const x = 1;";
        let ours = "import { type User, type Post } from \"./types\";\n\nexport const x = 1;";
        let theirs = "import { User } from \"./types\";\n\nexport const x = 1;";
        let path = Path::new("test.ts");
        let res = merge_module(base, ours, theirs, false, false, path).unwrap();
        assert!(res.contains("import { type Post, User } from \"./types\";"));
        assert!(!res.contains("type User, User"));
    }

    #[test]
    fn test_order_preservation_with_top_addition() {
        let base = "export function M() {}";
        let ours = "export function TopOurs() {}\n\nexport function M() {}";
        let theirs = "export function TopTheirs() {}\n\nexport function M() {}";
        let path = Path::new("test.ts");
        let res = merge_module(base, ours, theirs, false, false, path).unwrap();
        let top_ours_pos = res.find("TopOurs").unwrap();
        let top_theirs_pos = res.find("TopTheirs").unwrap();
        let m_pos = res.find("function M").unwrap();
        assert!(top_ours_pos < m_pos);
        assert!(top_theirs_pos < m_pos);
    }

    #[test]
    fn test_preamble_and_directives_preservation() {
        let base = "\"use client\";\n// comment\nimport { A } from \"mod\";";
        let ours = "\"use client\";\n// comment\nimport { A, B } from \"mod\";";
        let theirs = "\"use client\";\n// comment\nimport { A, C } from \"mod\";";
        let path = Path::new("test.ts");
        let res = merge_module(base, ours, theirs, true, false, path).unwrap();
        assert!(res.starts_with("\"use client\";\n// comment"));
        assert!(res.contains("import { A, B, C } from \"mod\";"));
    }

    #[test]
    fn test_enterprise_scale_stress_benchmark() {
        let mut base = String::new();
        let mut ours = String::new();
        let mut theirs = String::new();

        for i in 0..200 {
            base.push_str(&format!("import {{ sym{} }} from \"pkg{}\";\n", i, i));
            ours.push_str(&format!("import {{ sym{} }} from \"pkg{}\";\n", i, i));
            theirs.push_str(&format!("import {{ sym{} }} from \"pkg{}\";\n", i, i));
        }

        for i in 0..500 {
            let func = format!("\nexport function fn_{}() {{\n    const x = {};\n    return x * 2;\n}}\n", i, i);
            base.push_str(&func);
            ours.push_str(&func);
            theirs.push_str(&func);
        }

        for i in 500..550 {
            ours.push_str(&format!("\nexport function ours_added_{}() {{\n    return {};\n}}\n", i, i));
        }

        for i in 550..600 {
            theirs.push_str(&format!("\nexport function theirs_added_{}() {{\n    return {};\n}}\n", i, i));
        }

        let path = Path::new("test.ts");
        let start = Instant::now();
        let result = merge_module(&base, &ours, &theirs, false, false, path);
        let duration = start.elapsed();

        assert!(result.is_ok());
        let merged = result.unwrap();

        assert!(merged.contains("ours_added_500"));
        assert!(merged.contains("theirs_added_550"));
        assert!(duration.as_millis() < 500);
    }
}