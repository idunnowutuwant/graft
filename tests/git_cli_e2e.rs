use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn run_cmd(dir: &PathBuf, args: &[&str]) -> bool {
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn test_real_git_merge_e2e() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("graft_e2e_{}", nanos));
    let _ = fs::create_dir_all(&temp_dir);

    assert!(run_cmd(&temp_dir, &["init"]));
    assert!(run_cmd(&temp_dir, &["config", "user.name", "Graft Tester"]));
    assert!(run_cmd(&temp_dir, &["config", "user.email", "test@graft.dev"]));

    let cargo_bin = env!("CARGO_BIN_EXE_graft");
    let driver_cmd = format!("\"{}\" %O %A %B -p %P", cargo_bin.replace('\\', "/"));
    assert!(run_cmd(&temp_dir, &["config", "merge.graft.name", "graft"]));
    assert!(run_cmd(&temp_dir, &["config", "merge.graft.driver", &driver_cmd]));

    let gitattributes = temp_dir.join(".gitattributes");
    let _ = fs::write(&gitattributes, "*.ts merge=graft\n");
    assert!(run_cmd(&temp_dir, &["add", ".gitattributes"]));
    assert!(run_cmd(&temp_dir, &["commit", "-m", "init gitattributes"]));

    let target_file = temp_dir.join("app.ts");
    let base_code = "import { useState } from 'react';\n\nexport function A() {}\n";
    let _ = fs::write(&target_file, base_code);
    assert!(run_cmd(&temp_dir, &["add", "app.ts"]));
    assert!(run_cmd(&temp_dir, &["commit", "-m", "base commit"]));

    assert!(run_cmd(&temp_dir, &["checkout", "-b", "feature-b"]));
    let theirs_code = "import { useState, useMemo } from 'react';\n\nexport function A() {}\n\nexport function C() {}\n";
    let _ = fs::write(&target_file, theirs_code);
    assert!(run_cmd(&temp_dir, &["commit", "-am", "theirs addition"]));

    let back = run_cmd(&temp_dir, &["checkout", "-"])
        || run_cmd(&temp_dir, &["checkout", "main"])
        || run_cmd(&temp_dir, &["checkout", "master"]);
    assert!(back);

    let ours_code = "import { useState, useEffect } from 'react';\n\nexport function A() {}\n\nexport function B() {}\n";
    let _ = fs::write(&target_file, ours_code);
    assert!(run_cmd(&temp_dir, &["commit", "-am", "ours addition"]));

    let merge_status = Command::new("git")
        .current_dir(&temp_dir)
        .args(["merge", "feature-b"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    assert!(merge_status);

    let merged_content = fs::read_to_string(&target_file).unwrap();
    assert!(merged_content.contains("useEffect, useMemo, useState"));
    assert!(merged_content.contains("export function B() {}"));
    assert!(merged_content.contains("export function C() {}"));

    let _ = fs::remove_dir_all(&temp_dir);
}