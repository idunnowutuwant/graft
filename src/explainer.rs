use std::path::Path;

#[derive(Debug, Clone)]
pub enum MergeResolutionKind {
    ImportUnion(usize),
    OursDeclarationKept(String),
    TheirsDeclarationKept(String),
    SynthesizedBoth(String),
    AiSynthesized(String),
}

#[derive(Debug, Default)]
pub struct MergeLedger {
    pub actions: Vec<MergeResolutionKind>,
    pub ours_lines: usize,
    pub theirs_lines: usize,
    pub synthesized_lines: usize,
}

impl MergeLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, action: MergeResolutionKind) {
        match &action {
            MergeResolutionKind::OursDeclarationKept(text) => {
                self.ours_lines += text.lines().count();
            }
            MergeResolutionKind::TheirsDeclarationKept(text) => {
                self.theirs_lines += text.lines().count();
            }
            MergeResolutionKind::SynthesizedBoth(text) | MergeResolutionKind::AiSynthesized(text) => {
                self.synthesized_lines += text.lines().count();
            }
            MergeResolutionKind::ImportUnion(count) => {
                self.synthesized_lines += count;
            }
        }
        self.actions.push(action);
    }

    pub fn print_briefing(&self, path: &Path) {
        let total = self.ours_lines + self.theirs_lines + self.synthesized_lines;
        if total == 0 {
            return;
        }

        let ours_pct = (self.ours_lines * 100) / total;
        let theirs_pct = (self.theirs_lines * 100) / total;
        let syn_pct = 100 - (ours_pct + theirs_pct);

        println!("\n[graft explainer] Merge Briefing for {}", path.display());
        println!("------------------------------------------------------------");
        for act in &self.actions {
            match act {
                MergeResolutionKind::ImportUnion(count) => {
                    println!("  ✔ Unified {} import specifiers into seamless set", count);
                }
                MergeResolutionKind::OursDeclarationKept(key) => {
                    println!("  ✔ Retained local declaration: {}", key);
                }
                MergeResolutionKind::TheirsDeclarationKept(key) => {
                    println!("  ✔ Accepted remote declaration: {}", key);
                }
                MergeResolutionKind::SynthesizedBoth(key) => {
                    println!("  ⚡ Interleaved independent additions in: {}", key);
                }
                MergeResolutionKind::AiSynthesized(key) => {
                    println!("  🤖 AI synthesized divergent logic in: {}", key);
                }
            }
        }
        println!("------------------------------------------------------------");
        println!(
            "  Code Provenance: Ours: {}% | Theirs: {}% | Synthesized: {}%\n",
            ours_pct, theirs_pct, syn_pct
        );
    }
}