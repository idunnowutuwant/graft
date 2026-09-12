use crate::ast::DeclarationItem;
use crate::merge::{align_3way_keys, merge_text_block, MergeFailure};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser};

pub fn merge_java(base: &str, ours: &str, theirs: &str) -> Result<String, MergeFailure> {
    let (p_base, i_base, d_base) = parse_java_module(base).ok_or(MergeFailure::SystemError)?;
    let (p_ours, i_ours, d_ours) = parse_java_module(ours).ok_or(MergeFailure::SystemError)?;
    let (p_theirs, i_theirs, d_theirs) = parse_java_module(theirs).ok_or(MergeFailure::SystemError)?;

    let mut has_conflict = false;
    let merged_preamble = merge_text_block(&p_base, &p_ours, &p_theirs, &mut has_conflict);
    let merged_imports = merge_java_imports(&i_base, &i_ours, &i_theirs);
    let merged_declarations = merge_java_declarations(&d_base, &d_ours, &d_theirs, &mut has_conflict);

    let mut output = String::new();
    if !merged_preamble.trim().is_empty() {
        output.push_str(merged_preamble.trim());
        output.push_str("\n\n");
    }
    if !merged_imports.is_empty() {
        output.push_str(&merged_imports);
        output.push_str("\n\n");
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

fn parse_java_module(source: &str) -> Option<(String, BTreeSet<String>, Vec<DeclarationItem>)> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::language()).ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut declarations = Vec::new();
    let mut preamble = String::new();
    let mut imports = BTreeSet::new();

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        let kind = child.kind();

        if kind == "package_declaration" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                preamble = text.to_string();
            }
            continue;
        }

        if kind == "import_declaration" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                imports.insert(text.trim().to_string());
            }
            continue;
        }

        let key = resolve_java_key(&child, source);
        if let Ok(raw_text) = child.utf8_text(source.as_bytes()) {
            declarations.push(DeclarationItem {
                key,
                text: raw_text.to_string(),
            });
        }
    }

    Some((preamble, imports, declarations))
}

fn resolve_java_key(node: &Node, source: &str) -> Option<String> {
    match node.kind() {
        "class_declaration" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("java:class:{}", name))
        }
        "interface_declaration" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("java:interface:{}", name))
        }
        "enum_declaration" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("java:enum:{}", name))
        }
        _ => None,
    }
}

fn merge_java_imports(
    base: &BTreeSet<String>,
    ours: &BTreeSet<String>,
    theirs: &BTreeSet<String>,
) -> String {
    let mut resolved = BTreeSet::new();
    for i in ours {
        resolved.insert(i.clone());
    }
    for i in theirs {
        resolved.insert(i.clone());
    }
    for i in base {
        if !ours.contains(i) && theirs.contains(i) {
            resolved.remove(i);
        } else if !theirs.contains(i) && ours.contains(i) {
            resolved.remove(i);
        }
    }
    resolved.into_iter().collect::<Vec<_>>().join("\n")
}

fn merge_java_declarations(
    base: &[DeclarationItem],
    ours: &[DeclarationItem],
    theirs: &[DeclarationItem],
    has_conflict: &mut bool,
) -> String {
    let mut base_map = BTreeMap::new();
    let mut base_keys = Vec::new();
    for item in base {
        if let Some(k) = &item.key {
            base_map.insert(k.clone(), item.text.clone());
            base_keys.push(k.clone());
        }
    }

    let mut ours_map = BTreeMap::new();
    let mut ours_keys = Vec::new();
    for item in ours {
        if let Some(k) = &item.key {
            ours_map.insert(k.clone(), item.text.clone());
            ours_keys.push(k.clone());
        }
    }

    let mut theirs_map = BTreeMap::new();
    let mut theirs_keys = Vec::new();
    for item in theirs {
        if let Some(k) = &item.key {
            theirs_map.insert(k.clone(), item.text.clone());
            theirs_keys.push(k.clone());
        }
    }

    let sequence = align_3way_keys(&base_keys, &ours_keys, &theirs_keys);
    let mut resolved = Vec::new();

    for key in sequence {
        let b = base_map.get(&key);
        let o = ours_map.get(&key);
        let t = theirs_map.get(&key);

        match (b, o, t) {
            (Some(bv), Some(ov), Some(tv)) => {
                if ov == tv {
                    resolved.push(ov.clone());
                } else if ov == bv {
                    resolved.push(tv.clone());
                } else if tv == bv {
                    resolved.push(ov.clone());
                } else {
                    match diffy::merge(bv, ov, tv) {
                        Ok(res) => resolved.push(res),
                        Err(conf) => {
                            *has_conflict = true;
                            resolved.push(conf);
                        }
                    }
                }
            }
            (None, Some(ov), None) => resolved.push(ov.clone()),
            (None, None, Some(tv)) => resolved.push(tv.clone()),
            (None, Some(ov), Some(tv)) => {
                if ov == tv {
                    resolved.push(ov.clone());
                } else {
                    *has_conflict = true;
                    resolved.push(format!("<<<<<<< OURS\n{}\n=======\n{}\n>>>>>>> THEIRS", ov, tv));
                }
            }
            (Some(bv), Some(ov), None) => {
                if ov != bv {
                    *has_conflict = true;
                    resolved.push(format!("<<<<<<< OURS\n{}\n=======\n// DELETED IN THEIRS\n>>>>>>> THEIRS", ov));
                }
            }
            (Some(bv), None, Some(tv)) => {
                if tv != bv {
                    *has_conflict = true;
                    resolved.push(format!("<<<<<<< OURS\n// DELETED IN OURS\n=======\n{}\n>>>>>>> THEIRS", tv));
                }
            }
            _ => {}
        }
    }
    resolved.join("\n\n")
}