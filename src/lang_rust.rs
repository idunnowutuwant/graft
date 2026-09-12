use crate::ast::DeclarationItem;
use crate::merge::{align_3way_keys, merge_text_block, MergeFailure};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser};

pub fn merge_rust(base: &str, ours: &str, theirs: &str) -> Result<String, MergeFailure> {
    let (p_base, u_base, d_base) = parse_rust_module(base).ok_or(MergeFailure::SystemError)?;
    let (p_ours, u_ours, d_ours) = parse_rust_module(ours).ok_or(MergeFailure::SystemError)?;
    let (p_theirs, u_theirs, d_theirs) = parse_rust_module(theirs).ok_or(MergeFailure::SystemError)?;

    let mut has_conflict = false;
    let merged_preamble = merge_text_block(&p_base, &p_ours, &p_theirs, &mut has_conflict);
    let merged_uses = merge_rust_uses(&u_base, &u_ours, &u_theirs);
    let merged_declarations = merge_rust_declarations(&d_base, &d_ours, &d_theirs, &mut has_conflict);

    let mut output = String::new();
    if !merged_preamble.trim().is_empty() {
        output.push_str(merged_preamble.trim());
        output.push_str("\n\n");
    }
    if !merged_uses.is_empty() {
        output.push_str(&merged_uses);
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

fn parse_rust_module(source: &str) -> Option<(String, BTreeSet<String>, Vec<DeclarationItem>)> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::language()).ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut preamble_end_byte = 0;
    let mut in_preamble = true;
    let mut uses = BTreeSet::new();
    let mut declarations = Vec::new();
    let mut pending_comments = Vec::new();

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        let kind = child.kind();

        if in_preamble {
            if kind == "inner_attribute_item" || kind == "comment" {
                preamble_end_byte = child.end_byte();
                continue;
            } else {
                in_preamble = false;
            }
        }

        if kind == "comment" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                pending_comments.push(text.to_string());
            }
            continue;
        }

        if kind == "use_declaration" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                uses.insert(text.trim().to_string());
            }
            pending_comments.clear();
            continue;
        }

        let key = resolve_rust_key(&child, source);
        if let Ok(raw_text) = child.utf8_text(source.as_bytes()) {
            let mut full_text = String::new();
            if !pending_comments.is_empty() {
                full_text.push_str(&pending_comments.join("\n"));
                full_text.push('\n');
                pending_comments.clear();
            }
            full_text.push_str(raw_text);

            declarations.push(DeclarationItem {
                key,
                text: full_text,
            });
        }
    }

    let preamble = if preamble_end_byte > 0 {
        source[..preamble_end_byte].to_string()
    } else {
        String::new()
    };

    Some((preamble, uses, declarations))
}

fn resolve_rust_key(node: &Node, source: &str) -> Option<String> {
    match node.kind() {
        "function_item" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("rs:fn:{}", name))
        }
        "struct_item" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("rs:struct:{}", name))
        }
        "enum_item" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("rs:enum:{}", name))
        }
        "trait_item" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("rs:trait:{}", name))
        }
        "impl_item" => {
            let type_node = node.child_by_field_name("type")?;
            let type_name = type_node.utf8_text(source.as_bytes()).ok()?;
            if let Some(trait_node) = node.child_by_field_name("trait") {
                let trait_name = trait_node.utf8_text(source.as_bytes()).ok()?;
                Some(format!("rs:impl:{}:for:{}", trait_name, type_name))
            } else {
                Some(format!("rs:impl:{}", type_name))
            }
        }
        "mod_item" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("rs:mod:{}", name))
        }
        "type_item" => {
            let name = node.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("rs:type:{}", name))
        }
        _ => None,
    }
}

fn merge_rust_uses(
    base: &BTreeSet<String>,
    ours: &BTreeSet<String>,
    theirs: &BTreeSet<String>,
) -> String {
    let mut resolved = BTreeSet::new();

    for u in ours {
        resolved.insert(u.clone());
    }
    for u in theirs {
        resolved.insert(u.clone());
    }
    for u in base {
        if !ours.contains(u) && theirs.contains(u) {
            resolved.remove(u);
        } else if !theirs.contains(u) && ours.contains(u) {
            resolved.remove(u);
        }
    }

    resolved.into_iter().collect::<Vec<_>>().join("\n")
}

fn merge_rust_declarations(
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