use crate::ast::{parse_module, DeclarationItem, ImportGroup};
use std::collections::{BTreeMap, BTreeSet};

pub enum MergeFailure {
    Conflict(String),
    SystemError,
}

pub fn merge_module(base: &str, ours: &str, theirs: &str, is_tsx: bool) -> Result<String, MergeFailure> {
    let parsed_base = parse_module(base, is_tsx).ok_or(MergeFailure::SystemError)?;
    let parsed_ours = parse_module(ours, is_tsx).ok_or(MergeFailure::SystemError)?;
    let parsed_theirs = parse_module(theirs, is_tsx).ok_or(MergeFailure::SystemError)?;

    let mut has_conflict = false;

    let merged_preamble = merge_text_block(
        &parsed_base.preamble,
        &parsed_ours.preamble,
        &parsed_theirs.preamble,
        &mut has_conflict,
    );

    let merged_imports = merge_import_maps(
        &parsed_base.imports,
        &parsed_ours.imports,
        &parsed_theirs.imports,
    )?;

    let merged_declarations = merge_declarations(
        &parsed_base.declarations,
        &parsed_ours.declarations,
        &parsed_theirs.declarations,
        &mut has_conflict,
    );

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
        Ok(output)
    }
}

fn merge_text_block(base: &str, ours: &str, theirs: &str, has_conflict: &mut bool) -> String {
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

fn merge_declarations(
    base: &[DeclarationItem],
    ours: &[DeclarationItem],
    theirs: &[DeclarationItem],
    has_conflict: &mut bool,
) -> String {
    let base_has_unkeyed = base.iter().any(|d| d.key.is_none());
    let ours_has_unkeyed = ours.iter().any(|d| d.key.is_none());
    let theirs_has_unkeyed = theirs.iter().any(|d| d.key.is_none());

    if base_has_unkeyed || ours_has_unkeyed || theirs_has_unkeyed {
        let base_raw = join_declarations(base);
        let ours_raw = join_declarations(ours);
        let theirs_raw = join_declarations(theirs);
        return merge_text_block(&base_raw, &ours_raw, &theirs_raw, has_conflict);
    }

    let mut base_map = BTreeMap::new();
    for item in base {
        if let Some(k) = &item.key {
            base_map.insert(k.clone(), item.text.clone());
        }
    }

    let mut ours_map = BTreeMap::new();
    for item in ours {
        if let Some(k) = &item.key {
            ours_map.insert(k.clone(), item.text.clone());
        }
    }

    let mut theirs_map = BTreeMap::new();
    for item in theirs {
        if let Some(k) = &item.key {
            theirs_map.insert(k.clone(), item.text.clone());
        }
    }

    let mut sequence: Vec<String> = Vec::new();
    for item in ours {
        if let Some(k) = &item.key {
            sequence.push(k.clone());
        }
    }
    for item in theirs {
        if let Some(k) = &item.key {
            if !sequence.contains(k) {
                sequence.push(k.clone());
            }
        }
    }

    let mut resolved_chunks = Vec::new();

    for key in sequence {
        let b = base_map.get(&key);
        let o = ours_map.get(&key);
        let t = theirs_map.get(&key);

        match (b, o, t) {
            (Some(bv), Some(ov), Some(tv)) => {
                if ov == tv {
                    resolved_chunks.push(ov.clone());
                } else if ov == bv {
                    resolved_chunks.push(tv.clone());
                } else if tv == bv {
                    resolved_chunks.push(ov.clone());
                } else {
                    match diffy::merge(bv, ov, tv) {
                        Ok(res) => resolved_chunks.push(res),
                        Err(conf) => {
                            *has_conflict = true;
                            resolved_chunks.push(conf);
                        }
                    }
                }
            }
            (None, Some(ov), None) => {
                resolved_chunks.push(ov.clone());
            }
            (None, None, Some(tv)) => {
                resolved_chunks.push(tv.clone());
            }
            (None, Some(ov), Some(tv)) => {
                if ov == tv {
                    resolved_chunks.push(ov.clone());
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

fn join_declarations(decls: &[DeclarationItem]) -> String {
    decls
        .iter()
        .map(|d| d.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn merge_import_maps(
    base: &BTreeMap<String, ImportGroup>,
    ours: &BTreeMap<String, ImportGroup>,
    theirs: &BTreeMap<String, ImportGroup>,
) -> Result<BTreeMap<String, ImportGroup>, MergeFailure> {
    let mut all_keys: BTreeSet<&String> = BTreeSet::new();
    all_keys.extend(base.keys());
    all_keys.extend(ours.keys());
    all_keys.extend(theirs.keys());

    let mut resolved = BTreeMap::new();

    for key in all_keys {
        let b = base.get(key);
        let o = ours.get(key);
        let t = theirs.get(key);

        match (b, o, t) {
            (Some(bv), Some(ov), Some(tv)) => {
                if ov == tv {
                    resolved.insert(key.clone(), ov.clone());
                } else if ov == bv {
                    resolved.insert(key.clone(), tv.clone());
                } else if tv == bv {
                    resolved.insert(key.clone(), ov.clone());
                } else {
                    resolved.insert(key.clone(), merge_single_import(bv, ov, tv)?);
                }
            }
            (None, Some(ov), None) => {
                resolved.insert(key.clone(), ov.clone());
            }
            (None, None, Some(tv)) => {
                resolved.insert(key.clone(), tv.clone());
            }
            (None, Some(ov), Some(tv)) => {
                if ov == tv {
                    resolved.insert(key.clone(), ov.clone());
                } else {
                    let dummy_base = ImportGroup::default();
                    resolved.insert(key.clone(), merge_single_import(&dummy_base, ov, tv)?);
                }
            }
            (Some(bv), None, Some(tv)) => {
                if bv != tv {
                    return Err(MergeFailure::SystemError);
                }
            }
            (Some(bv), Some(ov), None) => {
                if bv != ov {
                    return Err(MergeFailure::SystemError);
                }
            }
            (Some(_), None, None) => {}
            (None, None, None) => {}
        }
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

    let mut named_imports = ours.named_imports.clone();
    for spec in &theirs.named_imports {
        if let Some(existing) = named_imports.iter().find(|s| s.name == spec.name) {
            if existing.alias != spec.alias {
                return Err(MergeFailure::SystemError);
            }
        }
        named_imports.insert(spec.clone());
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