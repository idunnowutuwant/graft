use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct GitState;

impl GitState {
    pub fn is_worktree() -> bool {
        Path::new(".git").exists()
    }

    pub fn get_modified_files() -> Result<HashSet<String>> {
        let output = Command::new("git")
            .args(["diff", "--name-only", "HEAD"])
            .output()?;

        let mut files = HashSet::new();
        if output.status.success() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    files.insert(trimmed.to_string());
                }
            }
        }
        Ok(files)
    }

    pub fn read_ref(ref_name: &str) -> Option<String> {
        let ref_path = Path::new(".git").join("refs").join("heads").join(ref_name);
        if ref_path.exists() {
            fs::read_to_string(ref_path).ok().map(|s| s.trim().to_string())
        } else {
            None
        }
    }
}