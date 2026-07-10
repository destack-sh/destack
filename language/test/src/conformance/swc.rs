use std::path::{Path, PathBuf};

use super::parse::parse_file;
use crate::conformance::{
    Case, CaseOutcome, ConformanceDriver, ConformanceSuiteResult, run_conformance_driver,
    suite_fixtures_dir, suite_tests_dir,
};
use crate::core::RunOptions;

// pinned version of SWC parser tests
const SWC_VERSION: &str = "1.x";
const SWC_COMMIT: &str = "5b9d77c"; // short sha

/// SWC parser conformance suite.
#[derive(Debug, Clone)]
pub struct SwcSuite {
    tests_dir: PathBuf,
    suite_dir: PathBuf,
}

impl SwcSuite {
    /// Create one SWC conformance suite.
    pub fn new() -> Self {
        let suite_dir = suite_fixtures_dir("swc");
        let tests_dir = suite_tests_dir("swc");
        Self {
            tests_dir,
            suite_dir,
        }
    }

    fn discover_recursive(&self, dir: &Path, prefix: &str) -> Vec<Case> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy();

                if path.is_dir() {
                    let new_prefix = if prefix.is_empty() {
                        file_name.to_string()
                    } else {
                        format!("{prefix}/{file_name}")
                    };
                    tests.extend(self.discover_recursive(&path, &new_prefix));
                } else if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy();
                    if matches!(ext.as_ref(), "ts" | "tsx" | "js" | "jsx") {
                        // skip .d.ts files
                        if file_name.ends_with(".d.ts") {
                            continue;
                        }
                        let stem = path.file_stem().unwrap().to_string_lossy();
                        let name = if prefix.is_empty() {
                            format!("{stem}.{ext}")
                        } else {
                            format!("{prefix}/{stem}.{ext}")
                        };

                        if name.contains("/errors/") || name.contains("typescript-errors") {
                            continue;
                        }

                        tests.push(Case::valid(name));
                    }
                }
            }
        }

        tests.sort_by(|a, b| a.name.cmp(&b.name));
        tests
    }
}

impl Default for SwcSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceDriver for SwcSuite {
    fn name(&self) -> &str {
        "swc"
    }

    fn suite_dir(&self) -> &Path {
        &self.suite_dir
    }

    fn tests_dir(&self) -> &Path {
        &self.tests_dir
    }

    fn allows_undiscovered_status(&self, case_name: &str) -> bool {
        case_name.contains("/errors/") || case_name.contains("typescript-errors")
    }

    fn discover_cases(&self) -> Vec<Case> {
        let mut tests = Vec::new();

        // discover tests from typescript, jsx, and js directories
        for category in &["typescript", "jsx", "js"] {
            let category_dir = self.tests_dir.join(category);
            if category_dir.exists() {
                tests.extend(self.discover_recursive(&category_dir, category));
            }
        }

        tests
    }

    fn run(&self, test: &Case, show_diff: bool) -> CaseOutcome {
        let path = self.tests_dir.join(&test.name);

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return CaseOutcome::FailedRead,
        };

        let parse_outcome = parse_file(&path, &content, test.file_type, show_diff);

        parse_outcome.case_outcome(test.source_validity)
    }

    fn fetch_instructions(&self) -> String {
        format!(
            "To refresh SWC parser tests (version {SWC_VERSION}, commit {SWC_COMMIT}):\n\
             \n\
               python3 ./language/test/fixtures/conformance/fetch-suite.py ./language/test/fixtures/conformance/swc\n"
        )
    }

    fn category_for_case(&self, test_name: &str) -> String {
        // swc tests are typescript/subcategory/... or jsx/subcategory/... or js/subcategory/...
        // use the second segment as the category
        let parts: Vec<&str> = test_name.split('/').collect();
        let category = if parts.len() >= 2 {
            parts[1]
        } else {
            parts.first().unwrap_or(&"unknown")
        };

        // combine issue-* and deno-* into single categories
        if category.starts_with("issue-") || category.starts_with("jssue-") {
            "issue".to_string()
        } else if category.starts_with("deno-") {
            "deno".to_string()
        } else {
            category.to_string()
        }
    }
}

/// Run SWC conformance tests.
pub fn run_swc(
    options: &RunOptions,
    update_known_failures: bool,
) -> Option<ConformanceSuiteResult> {
    let suite = SwcSuite::new();
    run_conformance_driver(&suite, options, update_known_failures)
}
