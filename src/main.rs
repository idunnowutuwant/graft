mod ast;
mod init;
mod json_merge;
mod lang_go;
mod lang_python;
mod merge;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "graft", version)]
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
}

#[derive(Subcommand, Debug)]
enum Commands {
    Init {
        #[arg(short, long)]
        global: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Some(Commands::Init { global }) = cli.command {
        return match init::run(global) {
            Ok(_) => ExitCode::SUCCESS,
            Err(_) => ExitCode::from(1),
        };
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
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            };
        }
        return fallback_diffy_merge(&base_content, &ours_content, &theirs_content, &ours_path, uses_crlf);
    }

    if ext == "py" {
        return match lang_python::merge_python(&base_content, &ours_content, &theirs_content) {
            Ok(merged) => {
                let final_output = restore_newlines(merged, uses_crlf);
                if atomic_write(&ours_path, &final_output) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(merge::MergeFailure::Conflict(content)) => {
                let final_output = restore_newlines(content, uses_crlf);
                let _ = atomic_write(&ours_path, &final_output);
                ExitCode::from(1)
            }
            Err(merge::MergeFailure::SystemError) => {
                fallback_diffy_merge(&base_content, &ours_content, &theirs_content, &ours_path, uses_crlf)
            }
        };
    }

    if ext == "go" {
        return match lang_go::merge_go(&base_content, &ours_content, &theirs_content) {
            Ok(merged) => {
                let final_output = restore_newlines(merged, uses_crlf);
                if atomic_write(&ours_path, &final_output) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(merge::MergeFailure::Conflict(content)) => {
                let final_output = restore_newlines(content, uses_crlf);
                let _ = atomic_write(&ours_path, &final_output);
                ExitCode::from(1)
            }
            Err(merge::MergeFailure::SystemError) => {
                fallback_diffy_merge(&base_content, &ours_content, &theirs_content, &ours_path, uses_crlf)
            }
        };
    }

    let is_supported = matches!(ext, "ts" | "tsx" | "js" | "jsx");
    if !is_supported {
        return fallback_diffy_merge(&base_content, &ours_content, &theirs_content, &ours_path, uses_crlf);
    }

    let is_tsx = matches!(ext, "tsx" | "jsx");

    match merge::merge_module(&base_content, &ours_content, &theirs_content, is_tsx) {
        Ok(merged) => {
            let final_output = restore_newlines(merged, uses_crlf);
            if atomic_write(&ours_path, &final_output) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(merge::MergeFailure::Conflict(content)) => {
            let final_output = restore_newlines(content, uses_crlf);
            let _ = atomic_write(&ours_path, &final_output);
            ExitCode::from(1)
        }
        Err(merge::MergeFailure::SystemError) => {
            fallback_diffy_merge(&base_content, &ours_content, &theirs_content, &ours_path, uses_crlf)
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
) -> ExitCode {
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