use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ReproGenerator;

impl ReproGenerator {
    pub fn generate_fixture(
        target_path: &Path,
        base: &str,
        ours: &str,
        theirs: &str,
    ) -> Option<PathBuf> {
        let repro_dir = Path::new(".graft").join("repro");
        let _ = fs::create_dir_all(&repro_dir);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_millis();

        let file_stem = target_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("conflict");

        let fixture_file = repro_dir.join(format!("{}_{}_repro.json", file_stem, timestamp));

        let fixture_data = serde_json::json!({
            "target": target_path.to_string_lossy(),
            "timestamp": timestamp,
            "base": base,
            "ours": ours,
            "theirs": theirs
        });

        if fs::write(&fixture_file, serde_json::to_string_pretty(&fixture_data).ok()?).is_ok() {
            Some(fixture_file)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repro_generator_creates_fixture() {
        let target = Path::new("test_target.ts");
        let path = ReproGenerator::generate_fixture(target, "base", "ours", "theirs");
        assert!(path.is_some());
        let fixture_path = path.unwrap();
        assert!(fixture_path.exists());
        let content = fs::read_to_string(&fixture_path).unwrap();
        assert!(content.contains("\"base\": \"base\""));
        let _ = fs::remove_file(fixture_path);
    }
}

