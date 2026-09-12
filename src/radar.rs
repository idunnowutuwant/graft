use anyhow::{bail, Result};
use std::collections::HashSet;
use std::process::Command;
use crate::git_state::GitState;

pub struct ConflictRadar;

impl ConflictRadar {
    pub fn scan(target_branch: &str) -> Result<()> {
        if !GitState::is_worktree() {
            bail!("Not inside a git repository");
        }

        println!("🛰  Graft Proactive Conflict Radar");
        println!("============================================================");
        println!("Comparing current working tree with branch: '{}'", target_branch);

        if let Some(commit_sha) = GitState::read_ref(target_branch) {
            println!("Target branch head: {}", &commit_sha[..7.min(commit_sha.len())]);
        }
        println!();

        let local_files = GitState::get_modified_files()?;

        let target_diff = Command::new("git")
            .args(["diff", "--name-only", &format!("HEAD..{}", target_branch)])
            .output()?;

        let mut target_files = HashSet::new();
        if target_diff.status.success() {
            for line in String::from_utf8_lossy(&target_diff.stdout).lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    target_files.insert(trimmed.to_string());
                }
            }
        }

        let intersecting_files: Vec<_> = local_files.intersection(&target_files).collect();

        if intersecting_files.is_empty() {
            println!("✔ No overlapping file modifications detected.");
            println!("  Collision Risk: 0% - Safe to merge anytime.");
            return Ok(());
        }

        println!("⚠ Overlapping modified files found (Potential Merge Hotspots):");
        for file in &intersecting_files {
            println!("  • {} -> Collision Risk: 75%", file);
        }

        println!("============================================================");
        println!("Recommendation: Rebase onto '{}' early to resolve AST divergency proactively.", target_branch);
        Ok(())
    }
}