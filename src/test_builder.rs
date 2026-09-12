use std::path::Path;
use crate::ast::DeclarationItem;
use crate::ast::ImportGroup;
use crate::json_merge;
use crate::lang_cpp;
use crate::lang_go;
use crate::lang_java;
use crate::lang_python;
use crate::lang_rust;
use crate::merge;
use crate::policy::PolicyEngine;
use std::collections::BTreeMap;

pub struct TestScenario {
    ext: String,
    base: String,
    ours: String,
    theirs: String,
    expected_clean: bool,
    expected_contains: Vec<String>,
}

impl TestScenario {
    pub fn new(ext: &str) -> Self {
        Self {
            ext: ext.to_string(),
            base: String::new(),
            ours: String::new(),
            theirs: String::new(),
            expected_clean: true,
            expected_contains: Vec::new(),
        }
    }

    pub fn base(mut self, code: &str) -> Self {
        self.base = code.to_string();
        self
    }

    pub fn ours(mut self, code: &str) -> Self {
        self.ours = code.to_string();
        self
    }

    pub fn theirs(mut self, code: &str) -> Self {
        self.theirs = code.to_string();
        self
    }

    pub fn expect_clean(mut self, clean: bool) -> Self {
        self.expected_clean = clean;
        self
    }

    pub fn expect_contains(mut self, text: &str) -> Self {
        self.expected_contains.push(text.to_string());
        self
    }

    pub fn execute(&self) {
        let dummy_path = format!("test.{}", self.ext);
        let path = Path::new(&dummy_path);

        let result = match self.ext.as_str() {
            "ts" | "tsx" | "js" | "jsx" => {
                let is_tsx = self.ext.contains('x');
                merge::merge_module(&self.base, &self.ours, &self.theirs, is_tsx, false, path)
            }
            "py" => lang_python::merge_python(&self.base, &self.ours, &self.theirs),
            "go" => lang_go::merge_go(&self.base, &self.ours, &self.theirs),
            "rs" => lang_rust::merge_rust(&self.base, &self.ours, &self.theirs),
            "java" => lang_java::merge_java(&self.base, &self.ours, &self.theirs),
            "cpp" | "c" => lang_cpp::merge_cpp(&self.base, &self.ours, &self.theirs),
            "json" => match json_merge::merge_json(&self.base, &self.ours, &self.theirs) {
                Some(res) => Ok(res),
                None => Err(merge::MergeFailure::Conflict(String::new())),
            },
            _ => panic!("Unsupported extension in builder: {}", self.ext),
        };

        if self.expected_clean {
            assert!(result.is_ok(), "Expected clean merge for {}, but failed", self.ext);
            let merged = result.unwrap();
            for expected in &self.expected_contains {
                assert!(
                    merged.contains(expected),
                    "Result for {} missing expected content: '{}'\nMerged Output:\n{}",
                    self.ext, expected, merged
                );
            }
        } else {
            assert!(result.is_err(), "Expected conflict for {}, but succeeded", self.ext);
        }
    }
}

pub struct PolicyScenarioBuilder {
    file_path: String,
    imports: BTreeMap<String, ImportGroup>,
    declarations: Vec<DeclarationItem>,
}

impl PolicyScenarioBuilder {
    pub fn new(path: &str) -> Self {
        Self {
            file_path: path.to_string(),
            imports: BTreeMap::new(),
            declarations: Vec::new(),
        }
    }

    pub fn with_declaration(mut self, key: Option<&str>, text: &str) -> Self {
        self.declarations.push(DeclarationItem {
            key: key.map(|k| k.to_string()),
            text: text.to_string(),
        });
        self
    }

    pub fn assert_policy_result(&self, expected_err_substring: Option<&str>) {
        let engine = PolicyEngine::load();
        let path = Path::new(&self.file_path);
        let res = engine.validate(path, &self.imports, &self.declarations);

        match expected_err_substring {
            Some(expected) => {
                assert!(res.is_err());
                let err = res.err().unwrap();
                assert!(err.contains(expected));
            }
            None => {
                assert!(res.is_ok());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_automated_multilang_matrix() {
        let languages = ["ts", "py", "go", "rs", "java", "cpp"];

        for lang in languages {
            let (base, ours, theirs, expected_ours, expected_theirs) = match lang {
                "ts" => (
                    "export function M() {}",
                    "export function OursFn() {}\n\nexport function M() {}",
                    "export function M() {}\n\nexport function TheirsFn() {}",
                    "OursFn",
                    "TheirsFn",
                ),
                "py" => (
                    "def m(): pass",
                    "def ours_fn(): pass\n\ndef m(): pass",
                    "def m(): pass\n\ndef theirs_fn(): pass",
                    "ours_fn",
                    "theirs_fn",
                ),
                "go" => (
                    "package main\n\nfunc M() {}",
                    "package main\n\nfunc OursFn() {}\n\nfunc M() {}",
                    "package main\n\nfunc M() {}\n\nfunc TheirsFn() {}",
                    "OursFn",
                    "TheirsFn",
                ),
                "rs" => (
                    "fn m() {}",
                    "fn ours_fn() {}\n\nfn m() {}",
                    "fn m() {}\n\nfn theirs_fn() {}",
                    "ours_fn",
                    "theirs_fn",
                ),
                "java" => (
                    "class M {}",
                    "class OursClass {}\n\nclass M {}",
                    "class M {}\n\nclass TheirsClass {}",
                    "OursClass",
                    "TheirsClass",
                ),
                "cpp" => (
                    "void m() {}",
                    "void ours_fn() {}\n\nvoid m() {}",
                    "void m() {}\n\nvoid theirs_fn() {}",
                    "ours_fn",
                    "theirs_fn",
                ),
                _ => unreachable!(),
            };

            TestScenario::new(lang)
                .base(base)
                .ours(ours)
                .theirs(theirs)
                .expect_clean(true)
                .expect_contains(expected_ours)
                .expect_contains(expected_theirs)
                .execute();
        }
    }

    #[test]
    fn test_policy_scenario_builder_clean() {
        PolicyScenarioBuilder::new("src/normal.ts")
            .with_declaration(Some("fn:calc"), "function calc() { return 1; }")
            .assert_policy_result(None);
    }
}