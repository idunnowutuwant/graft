use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct DependencyGraph {
    pub forward_deps: HashMap<PathBuf, HashSet<PathBuf>>,
    pub reverse_deps: HashMap<PathBuf, HashSet<PathBuf>>,
    pub endpoint_registry: HashMap<String, HashSet<PathBuf>>,
}

impl DependencyGraph {
    pub fn build(root_dir: &Path) -> Self {
        let mut graph = Self::default();
        let mut files = Vec::new();
        Self::collect_source_files(root_dir, &mut files);

        for file in &files {
            if let Ok(content) = fs::read_to_string(file) {
                let imported_paths = Self::extract_import_paths(&content, file, root_dir);
                for imp in imported_paths {
                    graph.forward_deps.entry(file.clone()).or_default().insert(imp.clone());
                    graph.reverse_deps.entry(imp).or_default().insert(file.clone());
                }

                let endpoints = Self::extract_api_endpoints(&content);
                for ep in endpoints {
                    graph.endpoint_registry.entry(ep).or_default().insert(file.clone());
                }
            }
        }

        for files_sharing_endpoints in graph.endpoint_registry.values() {
            let file_list: Vec<_> = files_sharing_endpoints.iter().cloned().collect();
            for i in 0..file_list.len() {
                for j in (i + 1)..file_list.len() {
                    let f1 = &file_list[i];
                    let f2 = &file_list[j];
                    if f1.extension() != f2.extension() {
                        graph.reverse_deps.entry(f1.clone()).or_default().insert(f2.clone());
                        graph.reverse_deps.entry(f2.clone()).or_default().insert(f1.clone());
                    }
                }
            }
        }

        graph
    }

    pub fn calculate_blast_radius(&self, changed_file: &Path) -> (usize, Vec<PathBuf>) {
        let mut visited = HashSet::new();
        let mut queue = vec![changed_file.to_path_buf()];

        while let Some(current) = queue.pop() {
            if let Some(dependents) = self.reverse_deps.get(&current) {
                for dep in dependents {
                    if visited.insert(dep.clone()) {
                        queue.push(dep.clone());
                    }
                }
            }
        }

        let count = visited.len();
        let mut affected: Vec<_> = visited.into_iter().collect();
        affected.sort();
        (count, affected)
    }

    fn collect_source_files(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name();
                    if name != ".git" && name != "node_modules" && name != "target" {
                        Self::collect_source_files(&path, files);
                    }
                } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "rs" | "py" | "go" | "java" | "proto" | "graphql") {
                        files.push(path);
                    }
                }
            }
        }
    }

    fn extract_import_paths(content: &str, current_file: &Path, root_dir: &Path) -> Vec<PathBuf> {
        let mut results = Vec::new();
        let parent = current_file.parent().unwrap_or(root_dir);

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") || trimmed.starts_with("use ") {
                for token in trimmed.split(|c: char| c == '"' || c == '\'' || c == ';' || c.is_whitespace()) {
                    if token.starts_with("./") || token.starts_with("../") {
                        let candidate = parent.join(token);
                        for ext in &["ts", "tsx", "js", "jsx", "rs", "py", "go", "java", "proto", "graphql"] {
                            let resolved = candidate.with_extension(ext);
                            if resolved.exists() {
                                results.push(resolved);
                                break;
                            }
                        }
                    }
                }
            }
        }
        results
    }

    fn extract_api_endpoints(content: &str) -> HashSet<String> {
        let mut endpoints = HashSet::new();
        for line in content.lines() {
            for token in line.split(|c: char| c == '"' || c == '\'' || c == '`') {
                let token = token.trim();
                if token.starts_with("/api/") || token.starts_with("/v1/") || token.starts_with("/v2/") {
                    let clean_endpoint = token.split('?').next().unwrap_or(token);
                    if clean_endpoint.len() > 5 {
                        endpoints.insert(clean_endpoint.to_string());
                    }
                }
            }
        }
        endpoints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blast_radius_calculation() {
        let mut graph = DependencyGraph::default();
        let a = PathBuf::from("a.ts");
        let b = PathBuf::from("b.ts");
        let c = PathBuf::from("c.ts");

        graph.reverse_deps.entry(a.clone()).or_default().insert(b.clone());
        graph.reverse_deps.entry(b.clone()).or_default().insert(c.clone());

        let (count, affected) = graph.calculate_blast_radius(&a);
        assert_eq!(count, 2);
        assert!(affected.contains(&b));
        assert!(affected.contains(&c));
    }

    #[test]
    fn test_cross_language_endpoint_linking() {
        let py_code = "app.add_route('/api/v1/orders', OrderHandler)";
        let ts_code = "fetch('/api/v1/orders').then(res => res.json())";

        let py_endpoints = DependencyGraph::extract_api_endpoints(py_code);
        let ts_endpoints = DependencyGraph::extract_api_endpoints(ts_code);

        assert!(py_endpoints.contains("/api/v1/orders"));
        assert!(ts_endpoints.contains("/api/v1/orders"));
    }
}