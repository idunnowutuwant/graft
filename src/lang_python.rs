use crate::ast::DeclarationItem;
use crate::merge::MergeFailure;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::{Node, Parser};

pub fn merge_python(base: &str, ours: &str, theirs: &str) -> Result<String, MergeFailure> {
    let (p_base, d_base) = parse_python_module(base).ok_or(MergeFailure::SystemError)?;
    let (p_ours, d_ours) = parse_python_module(ours).ok_or(MergeFailure::SystemError)?;
    let (p_theirs, d_theirs) = parse_python_module(theirs).ok_or(MergeFailure::SystemError)?;

    let mut has_conflict = false;
    let merged_preamble = if p_ours == p_theirs {
        p_ours
    } else {
        match diffy::merge(&p_base, &p_ours, &p_theirs) {
            Ok(m) => m,
            Err(c) => {
                has_conflict = true;
                c
            }
        }
    };

    let merged_declarations = merge_python_declarations(&d_base, &d_ours, &d_theirs, &mut has_conflict);

    let mut output = String::new();
    if !merged_preamble.trim().is_empty() {
        output.push_str(merged_preamble.trim());
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

fn parse_python_module(source: &str) -> Option<(String, Vec<DeclarationItem>)> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::language()).ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut declarations = Vec::new();
    let mut preamble_end_byte = 0;
    let mut in_preamble = true;
    let mut pending_comments = Vec::new();

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        let kind = child.kind();

        if in_preamble {
            if kind == "comment" {
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

        let key = resolve_python_key(&child, source);
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

    Some((preamble, declarations))
}

fn resolve_python_key(node: &Node, source: &str) -> Option<String> {
    let mut target = *node;
    if target.kind() == "decorated_definition" {
        let mut cursor = target.walk();
        for child in target.children(&mut cursor) {
            if child.kind() == "function_definition" || child.kind() == "class_definition" {
                target = child;
                break;
            }
        }
    }

    match target.kind() {
        "function_definition" => {
            let name = target.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("py:fn:{}", name))
        }
        "class_definition" => {
            let name = target.child_by_field_name("name")?.utf8_text(source.as_bytes()).ok()?;
            Some(format!("py:class:{}", name))
        }
        "import_statement" | "import_from_statement" => {
            let text = target.utf8_text(source.as_bytes()).ok()?;
            Some(format!("py:import:{}", text.trim()))
        }
        _ => None,
    }
}

fn merge_python_declarations(
    base: &[DeclarationItem],
    ours: &[DeclarationItem],
    theirs: &[DeclarationItem],
    has_conflict: &mut bool,
) -> String {
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

    let mut sequence = Vec::new();
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
                    resolved.push(format!("<<<<<<< OURS\n{}\n=======\n# DELETED IN THEIRS\n>>>>>>> THEIRS", ov));
                }
            }
            (Some(bv), None, Some(tv)) => {
                if tv != bv {
                    *has_conflict = true;
                    resolved.push(format!("<<<<<<< OURS\n# DELETED IN OURS\n=======\n{}\n>>>>>>> THEIRS", tv));
                }
            }
            _ => {}
        }
    }
    resolved.join("\n\n")
}