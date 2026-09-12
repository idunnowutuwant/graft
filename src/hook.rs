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

        Self::write_hook(
            &hooks_dir.join("post-merge"),
            "#!/bin/sh\ngraft log\n",
        )?;

        Self::write_hook(
            &hooks_dir.join("pre-commit"),
            "#!/bin/sh\ngraft radar HEAD\n",
        )?;

        println!("✔ Installed Git hooks: post-merge, pre-commit");
        Ok(())
    }

    pub fn uninstall_hooks() -> Result<()> {
        let hooks_dir = Path::new(".git").join("hooks");
        let post_merge = hooks_dir.join("post-merge");
        let pre_commit = hooks_dir.join("pre-commit");

        if post_merge.exists() {
            let _ = fs::remove_file(post_merge);
        }
        if pre_commit.exists() {
            let _ = fs::remove_file(pre_commit);
        }

        println!("✔ Removed graft Git hooks");
        Ok(())
    }

    fn write_hook(target: &Path, content: &str) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(target)?;

        file.write_all(content.as_bytes())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(target)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(target, perms)?;
        }

        Ok(())
    }
}