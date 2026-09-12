use std::collections::HashMap;

pub struct LeakGuard;

impl LeakGuard {
    pub fn scan(content: &str) -> Vec<String> {
        let mut violations = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }

            for raw_token in trimmed.split(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '=' || c == ':') {
                let token = raw_token.trim_matches(|c: char| c == ';' || c == ',' || c == ')' || c == '(' || c.is_whitespace());
                if token.len() >= 16 && token.len() <= 100 {
                    let entropy = Self::shannon_entropy(token);
                    let is_known = Self::is_known_secret_prefix(token);
                    let is_suspicious = Self::is_suspicious_pattern(token);

                    if is_known || (entropy > 3.8 && is_suspicious) {
                        violations.push(format!(
                            "Line {}: High-entropy secret detected (entropy: {:.2}, token: {}...)",
                            idx + 1,
                            entropy,
                            &token[..8.min(token.len())]
                        ));
                    }
                }
            }
        }

        violations
    }

    fn shannon_entropy(s: &str) -> f64 {
        let mut map = HashMap::new();
        for ch in s.chars() {
            *map.entry(ch).or_insert(0usize) += 1;
        }

        let len = s.len() as f64;
        let mut entropy = 0.0;

        for count in map.values() {
            let p = *count as f64 / len;
            entropy -= p * p.log2();
        }

        entropy
    }

    fn is_known_secret_prefix(token: &str) -> bool {
        token.starts_with("ghp_")
            || token.starts_with("AKIA")
            || token.starts_with("sk-")
            || token.starts_with("gho_")
            || token.starts_with("glpat-")
    }

    fn is_suspicious_pattern(token: &str) -> bool {
        let has_upper = token.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = token.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = token.chars().any(|c| c.is_ascii_digit());

        has_upper && has_lower && has_digit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leakguard_detects_high_entropy_secret() {
        let code = "const key = \"AKIAIOSFODNN7EXAMPLE1234567890\";\nconst normal = \"hello world\";";
        let violations = LeakGuard::scan(code);
        assert!(!violations.is_empty());
        assert!(violations[0].contains("High-entropy secret"));
    }

    #[test]
    fn test_leakguard_ignores_safe_strings() {
        let code = "const normal = \"user_profile_container\";\nconst count = 42;";
        let violations = LeakGuard::scan(code);
        assert!(violations.is_empty());
    }
}