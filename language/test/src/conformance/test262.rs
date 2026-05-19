use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::parse::{ParseOptions, ParseOutcome, TestArea, parse_file};
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

    fn discover_in_dir(&self, dir: &Path, prefix: &str, expect_error: bool) -> Vec<Case> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "js")
                    && let Some(stem) = path.file_stem()
                {
                    let name = format!("{prefix}/{}", stem.to_string_lossy());
                    let test = if expect_error {
                        Case::fail(name, FileType::JavaScript)
                    } else {
                        Case::pass(name, FileType::JavaScript)
                    };
                    tests.push(test);
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

    fn discover_cases(&self) -> Vec<Case> {
        let mut tests = Vec::new();

        // pass/ directory: files that should parse successfully
        let pass_dir = self.tests_dir.join("pass");
        if pass_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_dir, "pass", false));
        }

        // fail/ directory: files that should fail to parse
        let fail_dir = self.tests_dir.join("fail");
        if fail_dir.exists() {
            tests.extend(self.discover_in_dir(&fail_dir, "fail", true));
        }

        // pass-explicit/ directory: files that should parse in module mode
        let pass_explicit_dir = self.tests_dir.join("pass-explicit");
        if pass_explicit_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_explicit_dir, "pass-explicit", false));
        }

        // early/ directory: files with early errors (should be detected as errors)
        let early_dir = self.tests_dir.join("early");
        if early_dir.exists() {
            tests.extend(self.discover_in_dir(&early_dir, "early", true));
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

        // early tests use the full early pipeline
        // fail fixtures include many early syntax errors we intentionally enforce in analyze
        let area = if category == "early" {
            TestArea::Early
        } else if category == "fail" {
            TestArea::EarlySyntax
        } else {
            TestArea::Parse
        };

        let parse_outcome = parse_file(
            &path,
            &content,
            test.file_type,
            ParseOptions {
                area,
                disallow_ambiguous_tree_literal: false,
                should_print_diagnostics: show_diff,
            },
        );

        match (test.expect_error, parse_outcome) {
            (true, ParseOutcome::Error) => CaseOutcome::Passed,
            (true, ParseOutcome::Ok) => CaseOutcome::FailedParse,
            (false, ParseOutcome::Ok) => CaseOutcome::Passed,
            (false, ParseOutcome::Error) => CaseOutcome::FailedParse,
        }
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
