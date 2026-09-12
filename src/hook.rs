use anyhow::{bail, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub struct HookManager;

impl HookManager {
    pub fn install_hooks() -> Result<()> {
        let hooks_dir = Path::new(".git").join("hooks");
        if !hooks_dir.exists() {
            bail!("Not inside a git repository");
        }

        Self::inject_hook_chain(
            &hooks_dir.join("post-merge"),
            "graft log >/dev/null 2>&1 || true",
        )?;

        Self::inject_hook_chain(
            &hooks_dir.join("pre-commit"),
            "graft radar HEAD >/dev/null 2>&1 || true",
        )?;

        println!("✔ Non-destructively chained Git hooks (post-merge, pre-commit) with GUI PATH guards");
        Ok(())
    }

    pub fn uninstall_hooks() -> Result<()> {
        let hooks_dir = Path::new(".git").join("hooks");
        Self::remove_hook_chain(&hooks_dir.join("post-merge"))?;
        Self::remove_hook_chain(&hooks_dir.join("pre-commit"))?;

        println!("✔ Safely removed graft hook chains while preserving original user hooks");
        Ok(())
    }

    fn inject_hook_chain(target: &Path, graft_cmd: &str) -> Result<()> {
        let start_marker = "### GRAFT HOOK START ###";
        let end_marker = "### GRAFT HOOK END ###";

        let path_guard = "export PATH=\"$PATH:$HOME/.cargo/bin:/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin\"\n";

        let payload = format!(
            "\n{}\n{}\ncommand -v graft >/dev/null 2>&1 && {}\n{}\n",
            start_marker, path_guard, graft_cmd, end_marker
        );

        if target.exists() {
            let current = fs::read_to_string(target).unwrap_or_default();
            if current.contains(start_marker) {
                return Ok(());
            }
            let mut file = OpenOptions::new().append(true).open(target)?;
            file.write_all(payload.as_bytes())?;
        } else {
            let header = "#!/bin/sh\n";
            let full_content = format!("{}{}", header, payload);
            fs::write(target, full_content)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(target) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(target, perms);
            }
        }

        Ok(())
    }

    fn remove_hook_chain(target: &Path) -> Result<()> {
        let start_marker = "### GRAFT HOOK START ###";
        let end_marker = "### GRAFT HOOK END ###";

        if !target.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(target)?;
        if let Some(start_idx) = content.find(start_marker) {
            if let Some(end_idx) = content.find(end_marker) {
                let after_end = end_idx + end_marker.len();
                let mut sanitized = String::new();
                sanitized.push_str(&content[..start_idx]);
                if after_end < content.len() {
                    sanitized.push_str(&content[after_end..]);
                }

                let trimmed = sanitized.trim();
                if trimmed == "#!/bin/sh" || trimmed.is_empty() {
                    let _ = fs::remove_file(target);
                } else {
                    fs::write(target, sanitized)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_chain_injection_and_removal() {
        let temp_file = std::env::temp_dir().join(format!("test_hook_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let original_hook = "#!/bin/sh\necho 'husky original hook'\n";
        fs::write(&temp_file, original_hook).unwrap();

        HookManager::inject_hook_chain(&temp_file, "graft radar").unwrap();
        let modified = fs::read_to_string(&temp_file).unwrap();
        assert!(modified.contains("husky original hook"));
        assert!(modified.contains("### GRAFT HOOK START ###"));
        assert!(modified.contains("graft radar"));

        HookManager::remove_hook_chain(&temp_file).unwrap();
        let restored = fs::read_to_string(&temp_file).unwrap();
        assert!(restored.contains("husky original hook"));
        assert!(!restored.contains("### GRAFT HOOK START ###"));

        let _ = fs::remove_file(temp_file);
    }
}