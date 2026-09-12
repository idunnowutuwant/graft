pub struct TraceContext;

impl TraceContext {
    pub fn extract_slice(source: &str, target_key: &str, window_size: usize) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let mut match_idx = None;

        let key_id = target_key.split(':').last().unwrap_or(target_key);

        for (idx, line) in lines.iter().enumerate() {
            if line.contains(key_id) {
                match_idx = Some(idx);
                break;
            }
        }

        let center = match_idx.unwrap_or(lines.len() / 2);
        let start = center.saturating_sub(window_size);
        let end = (center + window_size).min(lines.len());

        let sliced = lines[start..end].join("\n");
        Self::sanitize(&sliced)
    }

    fn sanitize(input: &str) -> String {
        let mut result = Vec::new();
        for line in input.lines() {
            if line.contains("ghp_") || line.contains("AKIA") || line.contains("sk-") {
                result.push("[SANITIZED_SECRET]");
            } else {
                result.push(line);
            }
        }
        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_slicing_and_sanitization() {
        let source = "function a() {}\nfunction b() {\n    const token = \"AKIA1234567890123456\";\n}\nfunction c() {}";
        let sliced = TraceContext::extract_slice(source, "b", 2);
        assert!(sliced.contains("function b"));
        assert!(sliced.contains("[SANITIZED_SECRET]"));
        assert!(!sliced.contains("AKIA1234567890123456"));
    }
}

