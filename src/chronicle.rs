use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChronicleEntry {
    pub timestamp: u128,
    pub path: String,
    pub pre_merge_content: String,
    pub post_merge_content: String,
}

pub struct Chronicle {
    journal_path: PathBuf,
}

impl Chronicle {
    pub fn new() -> Self {
        let dir = Path::new(".git").join("graft");
        let _ = fs::create_dir_all(&dir);
        Self {
            journal_path: dir.join("chronicle.journal"),
        }
    }

    pub fn record(&self, path: &Path, pre_merge: &str, post_merge: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let entry = ChronicleEntry {
            timestamp,
            path: path.to_string_lossy().to_string(),
            pre_merge_content: pre_merge.to_string(),
            post_merge_content: post_merge.to_string(),
        };

        if let Ok(serialized) = serde_json::to_string(&entry) {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.journal_path)
            {
                let _ = writeln!(file, "{}", serialized);
            }
        }
    }

    pub fn rollback_latest(&self, target_path: &Path) -> Result<bool, String> {
        if !self.journal_path.exists() {
            return Err("Chronicle journal is empty".to_string());
        }

        let file = fs::File::open(&self.journal_path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);

        let mut matching_entry = None;
        let target_str = target_path.to_string_lossy();

        for line in reader.lines().flatten() {
            if let Ok(entry) = serde_json::from_str::<ChronicleEntry>(&line) {
                if entry.path == target_str {
                    matching_entry = Some(entry);
                }
            }
        }

        if let Some(entry) = matching_entry {
            fs::write(target_path, entry.pre_merge_content).map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Err("No previous merge state found for this file".to_string())
        }
    }

    pub fn list_history(&self) {
        if !self.journal_path.exists() {
            println!("Chronicle: No recorded merge history.");
            return;
        }

        let file = match fs::File::open(&self.journal_path) {
            Ok(f) => f,
            Err(_) => return,
        };

        let reader = BufReader::new(file);
        println!("\nChronicle Time-Travel History\n============================================================");

        for line in reader.lines().flatten() {
            if let Ok(entry) = serde_json::from_str::<ChronicleEntry>(&line) {
                println!(
                    "[{}] File: {} (Rollback point available)",
                    entry.timestamp, entry.path
                );
            }
        }
        println!("============================================================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chronicle_record_and_rollback() {
        let temp_dir = std::env::temp_dir().join(format!("chronicle_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("test_file.txt");
        let journal_file = temp_dir.join("test.journal");

        let _ = fs::write(&test_file, "post_merge_content");

        let chronicle = Chronicle {
            journal_path: journal_file,
        };

        chronicle.record(&test_file, "pre_merge_content", "post_merge_content");
        let res = chronicle.rollback_latest(&test_file);
        assert!(res.is_ok());

        let restored = fs::read_to_string(&test_file).unwrap();
        assert_eq!(restored, "pre_merge_content");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}