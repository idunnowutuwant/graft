mod ai;
mod ast;
mod benchmark;
mod chronicle;
mod explainer;
mod git_state;
mod graph;
mod hook;
mod init;
mod json_merge;
mod lang_cpp;
mod lang_go;
mod lang_java;
mod lang_python;
mod lang_rust;
mod leakguard;
mod merge;
mod mergetool;
mod policy;
mod radar;
mod repro;
mod semantic;
mod session;
mod test_builder;
mod trace_ctx;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "graft", version, about = "AST-aware cognitive merge platform for Git")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(value_name = "BASE")]
    base: Option<PathBuf>,

    #[arg(value_name = "OURS")]
    ours: Option<PathBuf>,

    #[arg(value_name = "THEIRS")]
    theirs: Option<PathBuf>,

    #[arg(short, long, value_name = "PATH")]
    path: Option<PathBuf>,

    #[arg(long)]
    ai: bool,

    #[arg(long)]
    repro: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init {
        #[arg(short, long)]
        global: bool,
    },
    Doctor,
    Mergetool {
        #[arg(value_name = "BASE")]
        base: PathBuf,
        #[arg(value_name = "OURS")]
        ours: PathBuf,
        #[arg(value_name = "THEIRS")]
        theirs: PathBuf,
        #[arg(short, long, value_name = "OUTPUT")]
        output: PathBuf,
    },
    Radar {
        #[arg(value_name = "TARGET_BRANCH", default_value = "main")]
        target: String,
    },
    Impact {
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    Hook {
        #[arg(value_name = "ACTION")]
        action: String,
    },
    Undo {
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
    Log,
    Bench,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Init { global } => {
                return match init::run(global) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(_) => ExitCode::from(1),
                };
            }
            Commands::Doctor => {
                return match init::doctor() {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(_) => ExitCode::from(1),
                };
            }
            Commands::Mergetool { base, ours, theirs, output } => {
                return match mergetool::run_interactive(&base, &ours, &theirs, &output) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(_) => ExitCode::from(1),
                };
            }
            Commands::Radar { target } => {
                return match radar::ConflictRadar::scan(&target) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(_) => ExitCode::from(1),
                };
            }
            Commands::Impact { file } => {
                let graph = graph::DependencyGraph::build(Path::new("."));
                let (count, affected) = graph.calculate_blast_radius(&file);
                println!("Blast Radius for {}: {} downstream files affected", file.display(), count);
                for aff in affected {
                    println!("  ↳ {}", aff.display());
                }
                return ExitCode::SUCCESS;
            }
            Commands::Hook { action } => {
                if action == "install" {
                    return match hook::HookManager::install_hooks() {
                        Ok(_) => ExitCode::SUCCESS,
                        Err(_) => ExitCode::from(1),
                    };
                } else if action == "uninstall" {
                    return match hook::HookManager::uninstall_hooks() {
                        Ok(_) => ExitCode::SUCCESS,
                        Err(_) => ExitCode::from(1),
                    };
                } else {
                    eprintln!("Unknown hook action: {}. Use 'install' or 'uninstall'", action);
                    return ExitCode::from(1);
                }
            }
            Commands::Undo { path } => {
                let chronicle = chronicle::Chronicle::new();
                return match chronicle.rollback_latest(&path) {
                    Ok(_) => {
                        println!("✔ Rolled back {} to previous merge snapshot", path.display());
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("✖ Rollback failed: {}", e);
                        ExitCode::from(1)
                    }
                };
            }
            Commands::Log => {
                chronicle::Chronicle::new().list_history();
                return ExitCode::SUCCESS;
            }
                Commands::Bench => {
                benchmark::BenchmarkSuite::run();
                return ExitCode::SUCCESS;
            }
        }
    }

    let (base_path, ours_path, theirs_path) = match (cli.base, cli.ours, cli.theirs) {
        (Some(b), Some(o), Some(t)) => (b, o, t),
        _ => return ExitCode::from(1),
    };

    let ours_raw = match fs::read_to_string(&ours_path) {
        Ok(c) => c,
        Err(_) => return ExitCode::from(1),
    };
    let base_raw = match fs::read_to_string(&base_path) {
        Ok(c) => c,
        Err(_) => return ExitCode::from(1),
    };
    let theirs_raw = match fs::read_to_string(&theirs_path) {
        Ok(c) => c,
        Err(_) => return ExitCode::from(1),
    };

    let uses_crlf = ours_raw.contains("\r\n");

    let base_content = normalize_newlines(&base_raw);
    let ours_content = normalize_newlines(&ours_raw);
    let theirs_content = normalize_newlines(&theirs_raw);

    let ext = cli
        .path
        .as_ref()
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .unwrap_or("ts");

    if ext == "json" {
        if let Some(merged_json) = json_merge::merge_json(&base_content, &ours_content, &theirs_content) {
            let final_output = restore_newlines(merged_json, uses_crlf);
            return if atomic_write(&ours_path, &final_output) {
                session::SessionManager::new().cleanup();
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            };
        }
        return fallback_diffy_merge(&base_content, &ours_content, &theirs_content, &ours_path, uses_crlf, cli.repro, cli.path.as_deref());
    }

    if ext == "py" {
        return handle_merge_result(
            lang_python::merge_python(&base_content, &ours_content, &theirs_content),
            &base_content,
            &ours_content,
            &theirs_content,
            &ours_path,
            uses_crlf,
            cli.repro,
            cli.path.as_deref(),
        );
    }

    if ext == "go" {
        return handle_merge_result(
            lang_go::merge_go(&base_content, &ours_content, &theirs_content),
            &base_content,
            &ours_content,
            &theirs_content,
            &ours_path,
            uses_crlf,
            cli.repro,
            cli.path.as_deref(),
        );
    }

    if ext == "rs" {
        return handle_merge_result(
            lang_rust::merge_rust(&base_content, &ours_content, &theirs_content),
            &base_content,
            &ours_content,
            &theirs_content,
            &ours_path,
            uses_crlf,
            cli.repro,
            cli.path.as_deref(),
        );
    }

    if ext == "java" {
        return handle_merge_result(
            lang_java::merge_java(&base_content, &ours_content, &theirs_content),
            &base_content,
            &ours_content,
            &theirs_content,
            &ours_path,
            uses_crlf,
            cli.repro,
            cli.path.as_deref(),
        );
    }

    if matches!(ext, "cpp" | "cc" | "cxx" | "c" | "h" | "hpp") {
        return handle_merge_result(
            lang_cpp::merge_cpp(&base_content, &ours_content, &theirs_content),
            &base_content,
            &ours_content,
            &theirs_content,
            &ours_path,
            uses_crlf,
            cli.repro,
            cli.path.as_deref(),
        );
    }

    let is_supported = matches!(ext, "ts" | "tsx" | "js" | "jsx");
    if !is_supported {
        return fallback_diffy_merge(&base_content, &ours_content, &theirs_content, &ours_path, uses_crlf, cli.repro, cli.path.as_deref());
    }

    let is_tsx = matches!(ext, "tsx" | "jsx");
    let fallback_path = PathBuf::from("current_file.ts");
    let target_path = cli.path.as_deref().unwrap_or(&fallback_path);

    handle_merge_result(
        merge::merge_module(&base_content, &ours_content, &theirs_content, is_tsx, cli.ai, target_path),
        &base_content,
        &ours_content,
        &theirs_content,
        &ours_path,
        uses_crlf,
        cli.repro,
        cli.path.as_deref(),
    )
}

fn handle_merge_result(
    result: Result<String, merge::MergeFailure>,
    base: &str,
    ours: &str,
    theirs: &str,
    target: &PathBuf,
    uses_crlf: bool,
    enable_repro: bool,
    relative_path: Option<&Path>,
) -> ExitCode {
    match result {
        Ok(merged) => {
            let final_output = restore_newlines(merged, uses_crlf);
            if atomic_write(target, &final_output) {
                let chronicle = chronicle::Chronicle::new();
                chronicle.record(target, ours, &final_output);
                session::SessionManager::new().cleanup();
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(merge::MergeFailure::Conflict(content)) => {
            if enable_repro {
                if let Some(rel) = relative_path {
                    if let Some(fixture) = repro::ReproGenerator::generate_fixture(rel, base, ours, theirs) {
                        eprintln!("[graft repro] Generated isolated conflict fixture at: {}", fixture.display());
                    }
                }
            }
            let final_output = restore_newlines(content, uses_crlf);
            let _ = atomic_write(target, &final_output);
            ExitCode::from(1)
        }
        Err(merge::MergeFailure::SystemError) => {
            fallback_diffy_merge(base, ours, theirs, target, uses_crlf, enable_repro, relative_path)
        }
    }
}

fn atomic_write(target: &Path, content: &str) -> bool {
    let tmp_file = target.with_extension("tmp_graft");
    if fs::write(&tmp_file, content).is_err() {
        return false;
    }
    if fs::rename(&tmp_file, target).is_err() {
        let _ = fs::remove_file(tmp_file);
        return false;
    }
    true
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn restore_newlines(s: String, uses_crlf: bool) -> String {
    if uses_crlf {
        s.replace('\n', "\r\n")
    } else {
        s
    }
}

fn fallback_diffy_merge(
    base: &str,
    ours: &str,
    theirs: &str,
    target: &PathBuf,
    uses_crlf: bool,
    enable_repro: bool,
    relative_path: Option<&Path>,
) -> ExitCode {
    if enable_repro {
        if let Some(rel) = relative_path {
            if let Some(fixture) = repro::ReproGenerator::generate_fixture(rel, base, ours, theirs) {
                eprintln!("[graft repro] Generated isolated conflict fixture at: {}", fixture.display());
            }
        }
    }

    match diffy::merge(base, ours, theirs) {
        Ok(clean) => {
            let output = restore_newlines(clean, uses_crlf);
            if atomic_write(target, &output) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(conflict) => {
            let output = restore_newlines(conflict, uses_crlf);
            let _ = atomic_write(target, &output);
            ExitCode::from(1)
        }
    }
}