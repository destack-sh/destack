use std::path::{Path, PathBuf};

use super::parse::parse_file;
use crate::conformance::{
    Case, CaseOutcome, ConformanceDriver, ConformanceSuiteResult, run_conformance_driver,
    suite_fixtures_dir, suite_tests_dir,
};
use crate::core::RunOptions;

// pinned version of test262-parser-tests
// update this when upgrading the test suite
const TEST262_VERSION: &str = "2026-01-29";
const TEST262_COMMIT: &str = "0e808c7"; // short sha

/// Test262 parser conformance suite.
#[derive(Debug, Clone)]
pub struct Test262Suite {
    tests_dir: PathBuf,
    suite_dir: PathBuf,
}

impl Test262Suite {
    /// Create suite with default paths.
    pub fn new() -> Self {
        let suite_dir = suite_fixtures_dir("test262");
        let tests_dir = suite_tests_dir("test262");
        Self {
            tests_dir,
            suite_dir,
        }
    }

    /// Create suite with custom tests_dir.
    pub fn with_tests_dir(tests_dir: impl Into<PathBuf>) -> Self {
        let tests_dir = tests_dir.into();
        let suite_dir = suite_fixtures_dir("test262");
        Self {
            tests_dir,
            suite_dir,
        }
    }

    fn discover_in_dir(&self, dir: &Path, prefix: &str) -> Vec<Case> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "js")
                    && let Some(stem) = path.file_stem()
                {
                    let name = format!("{prefix}/{}", stem.to_string_lossy());
                    tests.push(Case::valid(name));
                }
            }
        }

        tests.sort_by(|a, b| a.name.cmp(&b.name));
        tests
    }
}

impl Default for Test262Suite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceDriver for Test262Suite {
    fn name(&self) -> &str {
        "test262"
    }

    fn suite_dir(&self) -> &Path {
        &self.suite_dir
    }

    fn tests_dir(&self) -> &Path {
        &self.tests_dir
    }

    fn allows_undiscovered_status(&self, case_name: &str) -> bool {
        case_name.starts_with("early/") || case_name.starts_with("fail/")
    }

    fn discover_cases(&self) -> Vec<Case> {
        let mut tests = Vec::new();

        // pass/ directory: files that should parse successfully
        let pass_dir = self.tests_dir.join("pass");
        if pass_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_dir, "pass"));
        }

        // pass-explicit/ directory: files that should parse in module mode
        let pass_explicit_dir = self.tests_dir.join("pass-explicit");
        if pass_explicit_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_explicit_dir, "pass-explicit"));
        }

        tests
    }

    fn run(&self, test: &Case, show_diff: bool) -> CaseOutcome {
        let parts: Vec<&str> = test.name.splitn(2, '/').collect();
        if parts.len() != 2 {
            return CaseOutcome::FailedRead;
        }

        let (category, test_name) = (parts[0], parts[1]);
        let path = self
            .tests_dir
            .join(category)
            .join(format!("{test_name}.js"));

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return CaseOutcome::FailedRead,
        };

        let parse_outcome = parse_file(&path, &content, test.file_type, show_diff);

        parse_outcome.case_outcome(test.source_validity)
    }

    fn fetch_instructions(&self) -> String {
        format!(
            "To refresh test262 parser tests (version {TEST262_VERSION}, commit {TEST262_COMMIT}):\n\
             \n\
               python3 ./language/test/fixtures/conformance/fetch-suite.py ./language/test/fixtures/conformance/test262\n"
        )
    }
}

/// Run test262 conformance tests.
pub fn run_test262(
    options: &RunOptions,
    update_known_failures: bool,
) -> Option<ConformanceSuiteResult> {
    let suite = Test262Suite::new();
    run_conformance_driver(&suite, options, update_known_failures)
}
