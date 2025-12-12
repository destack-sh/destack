use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::parse::{ParseOptions, ParseOutcome, TestArea, parse_file};
use super::runner::{ConformanceSuite, SuiteResult, Test, TestOutcome, run_conformance_suite};
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

    fn discover_in_dir(&self, dir: &Path, prefix: &str, expect_error: bool) -> Vec<Test> {
        let mut tests = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "js")
                    && let Some(stem) = path.file_stem()
                {
                    let name = format!("{prefix}/{}", stem.to_string_lossy());
                    tests.push(Test {
                        name,
                        file_type: FileType::JavaScript,
                        expect_error,
                    });
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

    fn discover(&self) -> Vec<Test> {
        let mut tests = Vec::new();

        // pass/ directory: files that should parse successfully
        let pass_dir = self.root.join("pass");
        if pass_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_dir, "pass", false));
        }

        // fail/ directory: files that should fail to parse
        let fail_dir = self.root.join("fail");
        if fail_dir.exists() {
            tests.extend(self.discover_in_dir(&fail_dir, "fail", true));
        }

        // pass-explicit/ directory: files that should parse in module mode
        let pass_explicit_dir = self.root.join("pass-explicit");
        if pass_explicit_dir.exists() {
            tests.extend(self.discover_in_dir(&pass_explicit_dir, "pass-explicit", false));
        }

        // early/ directory: files with early errors (should be detected as errors)
        let early_dir = self.root.join("early");
        if early_dir.exists() {
            tests.extend(self.discover_in_dir(&early_dir, "early", true));
        }

        tests
    }

    fn run(&self, test: &Test) -> TestOutcome {
        let parts: Vec<&str> = test.name.splitn(2, '/').collect();
        if parts.len() != 2 {
            return TestOutcome::Failed;
        }

        let (category, test_name) = (parts[0], parts[1]);
        let path = self.root.join(category).join(format!("{test_name}.js"));

        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return TestOutcome::Failed,
        };

        // early/ tests check for early errors (bind + flow), others only check parse errors
        let area = if category == "early" {
            TestArea::Early
        } else {
            TestArea::Parse
        };

        let parse_outcome = parse_file(&path, &content, test.file_type, ParseOptions { area });

        match (test.expect_error, parse_outcome) {
            (true, ParseOutcome::Error) => TestOutcome::Passed,
            (true, ParseOutcome::Ok) => TestOutcome::Failed,
            (false, ParseOutcome::Ok) => TestOutcome::Passed,
            (false, ParseOutcome::Error) => TestOutcome::Failed,
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
