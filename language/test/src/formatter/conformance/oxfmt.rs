use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::expected::load_expected_output;
use super::fixtures::{
    expect_error_from_path, is_formattable_file_type, should_skip_directory,
    should_skip_fixture_file, sibling_with_suffix,
};
use super::format::run_formatter_case;
use super::runner::{
    ConformanceSuite, ExpectedOutput, SuiteResult, Test, TestOutcome, run_conformance_suite,
};
use crate::harness::{TestOptions, fixtures_dir};

// pinned version of oxfmt fixtures
const OXFMT_VERSION: &str = "oxc-main";
const OXFMT_REF: &str = "main";

/// oxfmt formatter conformance suite.
#[derive(Debug, Clone)]
pub struct OxfmtSuite {
    root: PathBuf,
    conformance_dir: PathBuf,
}

impl OxfmtSuite {
    pub fn new() -> Self {
        let conformance_dir = fixtures_dir().join("formatter").join("conformance");
        let root = conformance_dir
            .join("staging")
            .join("oxfmt")
            .join("crates")
            .join("oxc_formatter")
            .join("tests")
            .join("fixtures");
        Self {
            root,
            conformance_dir,
        }
    }

    fn discover_in_dir(&self, dir: &Path, tests: &mut Vec<Test>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                let directory_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                if should_skip_directory(directory_name) {
                    continue;
                }
                self.discover_in_dir(&path, tests);
                continue;
            }
            if !path.is_file() || should_skip_fixture_file(&path) {
                continue;
            }

            let Some(file_type) = FileType::from_path(&path) else {
                continue;
            };
            if !is_formattable_file_type(file_type) {
                continue;
            }

            let relative = path.strip_prefix(&self.root).unwrap_or(&path);
            let test_name = relative.to_string_lossy().replace('\\', "/");
            let mut test = if expect_error_from_path(&test_name) {
                Test::fail(test_name, file_type)
            } else {
                Test::pass(test_name, file_type)
            };

            if let Some(expected_path) = sibling_with_suffix(&path, ".snap")
                && expected_path.is_file()
                && let Ok(relative_expected) = expected_path.strip_prefix(&self.root)
            {
                test = test.with_expected_output(ExpectedOutput::OxfmtSnapshot(
                    relative_expected.to_path_buf(),
                ));
            }

            tests.push(test);
        }
    }
}

impl Default for OxfmtSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl ConformanceSuite for OxfmtSuite {
    fn name(&self) -> &str {
        "oxfmt"
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn known_failures_path(&self) -> PathBuf {
        self.conformance_dir.join("oxfmt-known-failures.txt")
    }

    fn discover(&self) -> Vec<Test> {
        let mut tests = Vec::new();
        self.discover_in_dir(&self.root, &mut tests);
        tests
    }

    fn run(&self, test: &Test, show_diff: bool) -> TestOutcome {
        let path = self.root.join(&test.name);
        let expected_output = load_expected_output(self.root(), &test.expected_output);

        run_formatter_case(
            &path,
            test.file_type,
            expected_output.as_deref(),
            test.expect_error,
            show_diff,
        )
    }

    fn download_instructions(&self) -> String {
        format!(
            "To download oxfmt fixtures (version {OXFMT_VERSION}, ref {OXFMT_REF}):\n\
             \n\
               just language/install-formatter-conformance\n\
             \n\
             Or manually:\n\
               ./language/test/fixtures/formatter/conformance/oxfmt-fetch.sh\n"
        )
    }
}

/// Run oxfmt formatter conformance tests.
pub fn run_oxfmt(options: &TestOptions, update_known_failures: bool) -> Option<SuiteResult> {
    let suite = OxfmtSuite::new();
    run_conformance_suite(&suite, options, update_known_failures)
}
