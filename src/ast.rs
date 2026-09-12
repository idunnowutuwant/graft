use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportSpecifier {
    pub name: String,
    pub alias: Option<String>,
    pub is_type: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportGroup {
    pub is_type_only: bool,
    pub default_import: Option<String>,
    pub namespace_import: Option<String>,
    pub named_imports: BTreeSet<ImportSpecifier>,
    pub is_side_effect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationItem {
    pub key: Option<String>,
    pub text: String,
}

#[derive(Debug)]
pub struct ParsedModule {
    pub preamble: String,
    pub imports: BTreeMap<String, ImportGroup>,
    pub declarations: Vec<DeclarationItem>,
}

fn hash_content(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

pub fn parse_module(source: &str, is_tsx: bool) -> Option<ParsedModule> {
    let mut parser = Parser::new();
    let language: Language = if is_tsx {
        tree_sitter_typescript::language_tsx()
    } else {
        tree_sitter_typescript::language_typescript()
    };

    parser.set_language(&language).ok()?;
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut imports = BTreeMap::new();
    let mut declarations: Vec<DeclarationItem> = Vec::new();
    let mut in_preamble = true;
    let mut preamble_end_byte = 0;
    let mut pending_comments: Vec<String> = Vec::new();

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        let kind = child.kind();

        if in_preamble {
            if is_directive_or_comment(&child, source) {
                preamble_end_byte = child.end_byte();
                continue;
            } else {
                in_preamble = false;
            }
        }

        if kind == "comment" {
            if let Ok(comment_text) = child.utf8_text(source.as_bytes()) {
                pending_comments.push(comment_text.to_string());
            }
            continue;
        }

        if kind == "import_statement" {
            if let Some((module, group)) = extract_import(&child, source) {
                imports.insert(module, group);
            }
            pending_comments.clear();
            continue;
        }

        let mut key = resolve_declaration_key(&child, source);
        if let Ok(raw_text) = child.utf8_text(source.as_bytes()) {
            let mut full_text = String::new();
            if !pending_comments.is_empty() {
                full_text.push_str(&pending_comments.join("\n"));
                full_text.push('\n');
                pending_comments.clear();
            }
            full_text.push_str(raw_text);

            if key.is_none() {
                key = Some(format!("unkeyed:{:016x}", hash_content(full_text.trim())));
            }

            if let Some(ref k) = key {
                if let Some(last) = declarations.last_mut() {
                    if last.key.as_ref() == Some(k) {
                        last.text.push_str("\n\n");
                        last.text.push_str(&full_text);
                        continue;
                    }
                }
            }

            declarations.push(DeclarationItem {
                key,
                text: full_text,
            });
        }
    }

    if !pending_comments.is_empty() {
        let comment_text = pending_comments.join("\n");
        let fallback_key = Some(format!("comment:{:016x}", hash_content(&comment_text)));
        if let Some(last) = declarations.last_mut() {
            last.text.push_str("\n\n");
            last.text.push_str(&comment_text);
        } else {
            declarations.push(DeclarationItem {
                key: fallback_key,
                text: comment_text,
            });
        }
    }

    let preamble = if preamble_end_byte > 0 {
        source[..preamble_end_byte].to_string()
    } else {
        String::new()
    };

    Some(ParsedModule {
        preamble,
        imports,
        declarations,
    })
}

fn is_directive_or_comment(node: &Node, source: &str) -> bool {
    match node.kind() {
        "comment" | "hash_bang_line" => true,
        "expression_statement" => {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                let trimmed = text.trim().trim_end_matches(';');
                trimmed == "\"use client\""
                    || trimmed == "'use client'"
                    || trimmed == "\"use server\""
                    || trimmed == "'use server'"
                    || trimmed == "\"use strict\""
                    || trimmed == "'use strict'"
            } else {
                false
            }
        }
        _ => false,
    }
}

fn resolve_declaration_key(node: &Node, source: &str) -> Option<String> {
    let mut target = *node;

    if target.kind() == "export_statement" {
        let mut is_default = false;
        let mut cursor = target.walk();
        for child in target.children(&mut cursor) {
            if child.kind() == "default" {
                is_default = true;
            }
            match child.kind() {
                "function_declaration"
                | "class_declaration"
                | "interface_declaration"
                | "type_alias_declaration"
                | "enum_declaration"
                | "lexical_declaration" => {
                    target = child;
                    break;
                }
                _ => {}
            }
        }
        if is_default && target.kind() == "export_statement" {
            return Some("export:default".to_string());
        }
    }

    let kind = target.kind();
    match kind {
        "function_declaration" => {
            let name = extract_name(&target, source)?;
            Some(format!("fn:{}", name))
        }
        "class_declaration" => {
            let name = extract_name(&target, source)?;
            Some(format!("class:{}", name))
        }
        "interface_declaration" => {
            let name = extract_name(&target, source)?;
            Some(format!("interface:{}", name))
        }
        "type_alias_declaration" => {
            let name = extract_name(&target, source)?;
            Some(format!("type:{}", name))
        }
        "enum_declaration" => {
            let name = extract_name(&target, source)?;
            Some(format!("enum:{}", name))
        }
        "lexical_declaration" => {
            let mut cursor = target.walk();
            for child in target.children(&mut cursor) {
                if child.kind() == "variable_declarator" {
                    let name = extract_name(&child, source)?;
                    return Some(format!("var:{}", name));
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_name(node: &Node, source: &str) -> Option<String> {
    if let Some(name_node) = node.child_by_field_name("name") {
        return name_node.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" || child.kind() == "type_identifier" {
            return child.utf8_text(source.as_bytes()).ok().map(|s| s.to_string());
        }
    }
    None
}

fn extract_import(node: &Node, source: &str) -> Option<(String, ImportGroup)> {
    let source_node = node.child_by_field_name("source")?;
    let raw_source = source_node.utf8_text(source.as_bytes()).ok()?;
    let module_path = raw_source.trim_matches(|c| c == '\'' || c == '"').to_string();

    let mut group = ImportGroup::default();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "type" {
            group.is_type_only = true;
        }
    }

    let clause_node = match node.children(&mut cursor).find(|c| c.kind() == "import_clause") {
        Some(clause) => clause,
        None => {
            group.is_side_effect = true;
            return Some((module_path, group));
        }
    };

    let mut clause_cursor = clause_node.walk();
    for child in clause_node.children(&mut clause_cursor) {
        match child.kind() {
            "type" => {
                group.is_type_only = true;
            }
            "identifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    group.default_import = Some(text.to_string());
                }
            }
            "namespace_import" => {
                if let Some(id) = child.children(&mut child.walk()).find(|c| c.kind() == "identifier") {
                    if let Ok(text) = id.utf8_text(source.as_bytes()) {
                        group.namespace_import = Some(text.to_string());
                    }
                }
            }
            "named_imports" => {
                let mut named_cursor = child.walk();
                for spec in child.children(&mut named_cursor) {
                    if spec.kind() == "import_specifier" {
                        if let Some(specifier) = parse_specifier(&spec, source) {
                            group.named_imports.insert(specifier);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Some((module_path, group))
}

fn parse_specifier(node: &Node, source: &str) -> Option<ImportSpecifier> {
    let mut is_type = false;
    let mut names = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "type" => is_type = true,
            "identifier" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    names.push(text.to_string());
                }
            }
            _ => {}
        }
    }

    match names.len() {
        1 => Some(ImportSpecifier {
            name: names[0].clone(),
            alias: None,
            is_type,
        }),
        2 => Some(ImportSpecifier {
            name: names[0].clone(),
            alias: Some(names[1].clone()),
            is_type,
        }),
        _ => None,
    }
}