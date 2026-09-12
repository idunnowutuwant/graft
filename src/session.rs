use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProjectMergeSession {
    pub renamed_exports: HashMap<String, String>,
    pub modified_signatures: HashMap<String, String>,
}

pub struct SessionManager {
    session_path: PathBuf,
}

impl SessionManager {
    pub fn new() -> Self {
        let git_dir = Path::new(".git");
        let session_path = if git_dir.exists() {
            git_dir.join("graft_session.json")
        } else {
            PathBuf::from(".graft_session.json")
        };
        Self { session_path }
    }

    pub fn load(&self) -> ProjectMergeSession {
        if !self.session_path.exists() {
            return ProjectMergeSession::default();
        }
        fs::read_to_string(&self.session_path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub fn record_rename(&self, old_name: &str, new_name: &str) {
        let mut session = self.load();
        session.renamed_exports.insert(old_name.to_string(), new_name.to_string());
        if let Ok(serialized) = serde_json::to_string_pretty(&session) {
            let _ = fs::write(&self.session_path, serialized);
        }
    }

    pub fn resolve_cross_file_symbol(&self, symbol: &str) -> Option<String> {
        let session = self.load();
        session.renamed_exports.get(symbol).cloned()
    }

    pub fn cleanup(&self) {
        if self.session_path.exists() {
            let _ = fs::remove_file(&self.session_path);
        }
    }
}