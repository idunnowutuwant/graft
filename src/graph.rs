use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct DependencyGraph {
    pub forward_deps: HashMap<PathBuf, HashSet<PathBuf>>,
    pub reverse_deps: HashMap<PathBuf, HashSet<PathBuf>>,
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
                    if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "rs" | "py" | "go") {
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
                for token in trimmed.split(|c: char| c == '"' || c == '\'' || c.is_whitespace()) {
                    if token.starts_with("./") || token.starts_with("../") {
                        let candidate = parent.join(token);
                        for ext in &["ts", "tsx", "js", "jsx", "rs", "py", "go"] {
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
}