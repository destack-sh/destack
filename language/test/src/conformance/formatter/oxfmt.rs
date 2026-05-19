use std::path::{Path, PathBuf};

use destack_source::FileType;

use super::expected::{load_expected_output, load_oxfmt_expected_case};
use super::fixtures::{
    expect_error_from_path, is_excluded_directory, is_excluded_fixture_file,
    is_formattable_file_type, sibling_with_suffix,
};
use super::format::{default_conformance_formatter_options, run_formatter_case};
use crate::conformance::{
    Case, CaseOutcome, ConformanceCapability, ConformanceDriver, ConformanceSuiteResult,
    ExpectedOutput, category_key_for_case_name, formatter, run_conformance_driver,
};
use crate::core::RunOptions;

// pinned version of oxfmt fixtures
const OXFMT_VERSION: &str = "oxc-main";
const OXFMT_REF: &str = "main";

/// oxfmt formatter conformance suite.
#[derive(Debug, Clone)]
pub struct OxfmtSuite {
    tests_dir: PathBuf,
    suite_dir: PathBuf,
}

impl OxfmtSuite {
    /// Create one oxfmt conformance suite.
    pub fn new() -> Self {
        let suite_dir = formatter::suite_fixtures_dir("oxfmt");
        let tests_dir = formatter::suite_tests_dir("oxfmt");
        Self {
            tests_dir,
            suite_dir,
        }
    }

    fn discover_in_dir(&self, dir: &Path, tests: &mut Vec<Case>) {
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
                if is_excluded_directory(directory_name) {
                    continue;
                }
                self.discover_in_dir(&path, tests);
                continue;
            }
            if !path.is_file() || is_excluded_fixture_file(&path) {
                continue;
            }

            let Some(file_type) = FileType::from_path(&path) else {
                continue;
            };
            if !is_formattable_file_type(file_type) {
                continue;
            }

            let relative = path.strip_prefix(&self.tests_dir).unwrap_or(&path);
            let test_name = relative.to_string_lossy().replace('\\', "/");
            let mut test = if expect_error_from_path(&test_name) {
                Case::fail(test_name, file_type)
            } else {
                Case::pass(test_name, file_type)
            };

            if let Some(expected_path) = sibling_with_suffix(&path, ".snap")
                && expected_path.is_file()
                && let Ok(relative_expected) = expected_path.strip_prefix(&self.tests_dir)
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

impl ConformanceDriver for OxfmtSuite {
    fn name(&self) -> &str {
        "oxfmt"
    }

    fn suite_dir(&self) -> &Path {
        &self.suite_dir
    }

    fn tests_dir(&self) -> &Path {
        &self.tests_dir
    }

    fn capability(&self) -> ConformanceCapability {
        ConformanceCapability::Format
    }

    fn category_for_case(&self, test_name: &str) -> String {
        category_key_for_case_name(test_name)
    }

    fn discover_cases(&self) -> Vec<Case> {
        let mut tests = Vec::new();
        self.discover_in_dir(&self.tests_dir, &mut tests);
        tests
    }

    fn run(&self, test: &Case, show_diff: bool) -> CaseOutcome {
        let path = self.tests_dir.join(&test.name);
        let mut formatter_options = default_conformance_formatter_options();
        let expected_output = match &test.expected_output {
            ExpectedOutput::OxfmtSnapshot(path) => {
                if let Some(expected_case) =
                    load_oxfmt_expected_case(self.tests_dir(), path, formatter_options)
                {
                    formatter_options = expected_case.formatter_options;
                    Some(expected_case.output)
                } else {
                    None
                }
            }
            _ => load_expected_output(self.tests_dir(), &test.expected_output),
        };

        run_formatter_case(
            &path,
            test.file_type,
            expected_output.as_deref(),
            formatter_options,
            test.expect_error,
            test.check_idempotence,
            show_diff,
        )
    }

    fn fetch_instructions(&self) -> String {
        format!(
            "To refresh oxfmt fixtures (version {OXFMT_VERSION}, ref {OXFMT_REF}):\n\
             \n\
               python3 ./language/test/fixtures/conformance/fetch-suite.py ./language/test/fixtures/conformance/oxfmt\n"
        )
    }
}

/// Run oxfmt formatter conformance tests.
pub fn run_oxfmt(
    options: &RunOptions,
    update_known_failures: bool,
) -> Option<ConformanceSuiteResult> {
    let suite = OxfmtSuite::new();
    run_conformance_driver(&suite, options, update_known_failures)
}
