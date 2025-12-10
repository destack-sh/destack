use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::parse::{ParseOptions, ParseOutcome, is_module_path, parse_file};
use super::runner::{ConformanceSuite, SuiteResult, TestOutcome, run_conformance_suite};
use crate::harness::{TestOptions, fixtures_dir};

// pinned version of test262-parser-tests
// update this when upgrading the test suite
const TEST262_VERSION: &str = "0.1.0";
const TEST262_COMMIT: &str = "521b4ab"; // short sha

/// Test262 parser conformance suite.
#[derive(Debug, Clone)]
pub struct Test262Suite {
    root: PathBuf,
    conformance_dir: PathBuf,
}

impl Test262Suite {
    /// Create suite with default paths.
    pub fn new() -> Self {
        let conformance_dir = fixtures_dir().join("conformance");
        let root = conformance_dir.join("test262");
        Self {
            root,
            conformance_dir,
        }
    }

    /// Create suite with custom root.
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let conformance_dir = root.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self {
            root,
            conformance_dir,
        }
    }

    fn discover_in_dir(&self, dir: &Path, prefix: &str) -> Vec<String> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "js")
                    && let Some(name) = path.file_stem()
                {
                    tests.push(format!("{prefix}/{}", name.to_string_lossy()));
                }
            }
        }

        tests.sort();
        tests
    }
}

impl Default for Test262Suite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceSuite for Test262Suite {
    fn name(&self) -> &str {
        "test262"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn known_failures_path(&self) -> PathBuf {
        self.conformance_dir.join("test262-known-failures.txt")
    }

    fn discover_tests(&self) -> Vec<String> {
        let mut tests = Vec::new();

        // pass/ directory: files that should parse successfully
        let pass_dir = self.root.join("pass");
        if pass_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_dir, "pass"));
        }

        // fail/ directory: files that should fail to parse
        let fail_dir = self.root.join("fail");
        if fail_dir.exists() {
            tests.extend(self.discover_in_dir(&fail_dir, "fail"));
        }

        // pass-explicit/ directory: files that should parse in module mode
        let pass_explicit_dir = self.root.join("pass-explicit");
        if pass_explicit_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_explicit_dir, "pass-explicit"));
        }

        // early/ directory: files with early errors (parse succeeds, has semantic errors)
        let early_dir = self.root.join("early");
        if early_dir.exists() {
            tests.extend(self.discover_in_dir(&early_dir, "early"));
        }

        tests
    }

    fn run_test(&self, name: &str) -> TestOutcome {
        let parts: Vec<&str> = name.splitn(2, '/').collect();
        if parts.len() != 2 {
            return TestOutcome::Failed;
        }

        let (category, test_name) = (parts[0], parts[1]);
        let path = self.root.join(category).join(format!("{test_name}.js"));

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return TestOutcome::Failed,
        };

        // HTML comments are forbidden in ES modules (files with .module. in name)
        let options = ParseOptions {
            reject_html_comments: is_module_path(&path),
        };
        let parse_outcome = parse_file(&path, &content, FileType::JavaScript, options);

        // early/ tests should also parse successfully (they have semantic errors, not syntax errors)
        let should_pass = matches!(category, "pass" | "pass-explicit" | "early");

        // pass/early tests should parse without errors, fail tests should have errors
        match (should_pass, parse_outcome) {
            (true, ParseOutcome::Ok) => TestOutcome::Passed,
            (true, ParseOutcome::Error) => TestOutcome::Failed,
            (false, ParseOutcome::Ok) => TestOutcome::Failed,
            (false, ParseOutcome::Error) => TestOutcome::Passed,
        }
    }

    fn download_instructions(&self) -> String {
        format!(
            "To download test262 parser tests (version {TEST262_VERSION}, commit {TEST262_COMMIT}):\n\
             \n\
               just language/install-fixtures\n\
             \n\
             Or manually:\n\
               ./language/test/fixtures/conformance/test262-fetch.sh\n"
        )
    }
}

/// Run test262 conformance tests.
pub fn run_test262(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = Test262Suite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
