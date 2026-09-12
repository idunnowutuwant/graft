use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::trace_ctx::TraceContext;

#[derive(Serialize)]
struct LlmPrompt {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct LlmResponse {
    response: String,
}

pub struct AiIntentMerger {
    endpoint: String,
    model: String,
}

impl AiIntentMerger {
    pub fn new() -> Self {
        let endpoint = std::env::var("GRAFT_AI_ENDPOINT")
            .unwrap_or_else(|_| "http://localhost:11434/api/generate".to_string());
        let model = std::env::var("GRAFT_AI_MODEL")
            .unwrap_or_else(|_| "codellama".to_string());

        Self { endpoint, model }
    }

    pub fn is_available(&self) -> bool {
        ureq::get(&self.endpoint)
            .timeout(Duration::from_millis(300))
            .call()
            .is_ok()
    }

    pub fn resolve_intent(
        &self,
        base: &str,
        ours: &str,
        theirs: &str,
        declaration_key: &str,
    ) -> Option<String> {
        let sliced_base = TraceContext::extract_slice(base, declaration_key, 25);
        let sliced_ours = TraceContext::extract_slice(ours, declaration_key, 25);
        let sliced_theirs = TraceContext::extract_slice(theirs, declaration_key, 25);

        let prompt_text = format!(
            "You are a syntax-aware code merge engine.\n\
             Task: Synthesize and merge divergent modifications of the declaration '{}'.\n\
             Base:\n{}\n\n\
             Ours:\n{}\n\n\
             Theirs:\n{}\n\n\
             Output only the final valid code block without explanations or markdown backticks.",
            declaration_key, sliced_base, sliced_ours, sliced_theirs
        );

        let body = LlmPrompt {
            model: self.model.clone(),
            prompt: prompt_text,
            stream: false,
        };

        let response: LlmResponse = ureq::post(&self.endpoint)
            .timeout(Duration::from_secs(5))
            .send_json(body)
            .ok()?
            .into_json()
            .ok()?;

        let cleaned = response.response
            .trim()
            .trim_start_matches("```typescript")
            .trim_start_matches("```javascript")
            .trim_start_matches("```rust")
            .trim_start_matches("```python")
            .trim_start_matches("```go")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string();

        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    }
}