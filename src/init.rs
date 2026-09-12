use anyhow::{bail, Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn run(global: bool) -> Result<()> {
    if global {
        configure_git_global()?;
        println!("✔ graft: configured globally in ~/.gitconfig");
        return Ok(());
    }

    if !is_inside_work_tree() {
        eprintln!("graft: not a git repository. Run 'git init' first or use 'graft init --global'");
        bail!("not in git directory");
    }

    configure_git_local()?;
    configure_gitattributes()?;
    println!("✔ graft: initialized in current repository");
    Ok(())
}

pub fn doctor() -> Result<()> {
    println!("🔍 Graft Diagnostic (Doctor)\n==============================");

    let global_driver = Command::new("git")
        .args(["config", "--global", "merge.graft.driver"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let local_driver = Command::new("git")
        .args(["config", "merge.graft.driver"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if !local_driver.is_empty() {
        println!("✔ Local Git Merge Driver: {}", local_driver);
    } else if !global_driver.is_empty() {
        println!("✔ Global Git Merge Driver: {}", global_driver);
    } else {
        println!("✖ No Git Merge Driver configured. Run 'graft init' or 'graft init --global'");
    }

    let attr_path = Path::new(".gitattributes");
    if attr_path.exists() {
        let content = fs::read_to_string(attr_path).unwrap_or_default();
        let matches: Vec<_> = content
            .lines()
            .filter(|l| l.contains("merge=graft"))
            .collect();

        if !matches.is_empty() {
            println!("✔ .gitattributes found with {} configured rule(s):", matches.len());
            for m in matches {
                println!("    {}", m.trim());
            }
        } else {
            println!("⚠ .gitattributes exists, but contains no 'merge=graft' rules.");
        }
    } else {
        println!("⚠ No local .gitattributes found in current directory.");
    }

    println!("\nSupported filetypes: TS, TSX, JS, JSX, PY, GO, RS, JSON");
    println!("Graft status: READY");
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
    let content = "\n*.ts merge=graft\n*.tsx merge=graft\n*.js merge=graft\n*.jsx merge=graft\n*.py merge=graft\n*.go merge=graft\n*.rs merge=graft\n*.json merge=graft\n";

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