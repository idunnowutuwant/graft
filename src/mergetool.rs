use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn run_interactive(base: &Path, ours: &Path, theirs: &Path, output: &Path) -> Result<()> {
    let base_text = fs::read_to_string(base).unwrap_or_default();
    let ours_text = fs::read_to_string(ours).unwrap_or_default();
    let theirs_text = fs::read_to_string(theirs).unwrap_or_default();

    println!("\n=== Graft Interactive 3-Way Resolver ===");
    println!("File: {}", output.display());

    match diffy::merge(&base_text, &ours_text, &theirs_text) {
        Ok(clean) => {
            fs::write(output, clean)?;
            println!("✔ Clean merge applied successfully.");
            return Ok(());
        }
        Err(conflict) => {
            println!("Automatic AST merge reached an ambiguous conflict block.");
            println!("--------------------------------------------------");
            println!("{}", conflict);
            println!("--------------------------------------------------");
            print!("Choose version to keep: [o]urs, [t]heirs, [e]dit manually: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match input.trim().to_lowercase().as_str() {
                "o" | "ours" => {
                    fs::write(output, ours_text)?;
                    println!("✔ Applied OURS.");
                }
                "t" | "theirs" => {
                    fs::write(output, theirs_text)?;
                    println!("✔ Applied THEIRS.");
                }
                _ => {
                    fs::write(output, conflict)?;
                    println!("⚠ Conflict markers retained in destination.");
                }
            }
        }
    }
    Ok(())
}