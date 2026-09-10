use anyhow::{bail, Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn run(global: bool) -> Result<()> {
    if global {
        configure_git_global()?;
        println!("graft: configured globally");
        return Ok(());
    }

    if !is_inside_work_tree() {
        eprintln!("graft: not a git repository. Run 'git init' first or use 'graft init --global'");
        bail!("not in git directory");
    }

    configure_git_local()?;
    configure_gitattributes()?;
    println!("graft: initialized in current repository");
    Ok(())
}

fn is_inside_work_tree() -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn configure_git_local() -> Result<()> {
    run_command(&["config", "merge.graft.name", "graft"])?;
    run_command(&["config", "merge.graft.driver", "graft %O %A %B -p %P"])?;
    Ok(())
}

fn configure_git_global() -> Result<()> {
    run_command(&["config", "--global", "merge.graft.name", "graft"])?;
    run_command(&["config", "--global", "merge.graft.driver", "graft %O %A %B -p %P"])?;
    Ok(())
}

fn configure_gitattributes() -> Result<()> {
    let path = Path::new(".gitattributes");
    let content = "\n*.ts merge=graft\n*.tsx merge=graft\n*.js merge=graft\n*.jsx merge=graft\n";

    if path.exists() {
        let current = fs::read_to_string(path).context("Failed to read .gitattributes")?;
        if !current.contains("merge=graft") {
            let mut file = OpenOptions::new()
                .append(true)
                .open(path)
                .context("Failed to open .gitattributes")?;
            file.write_all(content.as_bytes())
                .context("Failed to append to .gitattributes")?;
        }
    } else {
        fs::write(path, content.trim_start()).context("Failed to write .gitattributes")?;
    }
    Ok(())
}

fn run_command(args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .status()
        .context("Failed to execute git command")?;

    if !status.success() {
        bail!("git command failed: {:?}", args);
    }
    Ok(())
}