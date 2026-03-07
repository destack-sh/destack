use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::harness::{
    RunContext, Suite, TestCase, TestOptions, TestResult, fixtures_dir, save_expected_failures,
};
use crate::lsp::runner;
use crate::mdtest::{
    MdTestCase, discover_md_files, load_mdtest_expected_failures, parse_mdtest_file, slug,
};

const LSP_TEST_CATEGORY: &str = "destack_test::lsp";

/// One discovered markdown LSP suite.
#[derive(Debug, Default)]
pub struct LspSuite {
    /// The parsed markdown cases keyed by full test name.
    tests: HashMap<String, MdTestCase>,
    /// The discovered fixture cases.
    cases: Vec<TestCase>,
    /// Known failing tests for baseline tracking.
    expected_failures: HashSet<String>,
    /// Location of the known failures file.
    expected_failures_path: PathBuf,
}

impl LspSuite {
    /// Load all applied LSP fixtures from `fixtures/lsp`.
    pub fn load() -> Self {
        let fixtures = fixtures_dir();
        let lsp_dir = fixtures.join("lsp");
        let mut suite = Self::default();
        let md_paths = match discover_md_files(&lsp_dir) {
            Ok(md_paths) => md_paths,
            Err(error) => {
                panic!("failed to discover {}: {error}", lsp_dir.display());
            }
        };

        for md_path in md_paths {
            suite.add_file(&lsp_dir, &md_path);
        }

        suite.expected_failures = load_mdtest_expected_failures(&lsp_dir);
        suite.expected_failures_path = lsp_dir.join("known-failures.txt");
        suite
            .cases
            .sort_by(|left, right| left.name.cmp(&right.name));
        suite
    }

    /// Add one markdown fixture file to the suite.
    fn add_file(&mut self, base_dir: &Path, md_path: &Path) {
        let cases = match parse_mdtest_file(md_path) {
            Ok(cases) => cases,
            Err(error) => {
                panic!("failed to parse {}: {error}", md_path.display());
            }
        };

        let relative_path = match md_path.strip_prefix(base_dir) {
            Ok(relative_path) => relative_path,
            Err(error) => {
                panic!(
                    "failed to relativize {} against {}: {error}",
                    md_path.display(),
                    base_dir.display()
                );
            }
        };
        let relative_name = relative_path.to_string_lossy();

        for lsp_case in cases {
            let name = format!(
                "{relative_name}/{}/{}",
                slug(&lsp_case.section),
                slug(&lsp_case.name)
            );
            let test_case = TestCase::file(name, PathBuf::from(md_path), LSP_TEST_CATEGORY)
                .with_skipped(lsp_case.skip);

            self.tests.insert(test_case.full_name(), lsp_case);
            self.cases.push(test_case);
        }
    }
}

impl Suite for LspSuite {
    fn name(&self) -> &'static str {
        "lsp"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.cases.clone()
    }

    fn expected_failures(&self, _options: &TestOptions) -> Option<&HashSet<String>> {
        if self.expected_failures.is_empty() {
            None
        } else {
            Some(&self.expected_failures)
        }
    }

    fn report(&self, results: &[(TestCase, TestResult)], context: &RunContext<'_>) {
        if !context.options.update_known_failures {
            return;
        }

        let mut failures = HashSet::new();
        for (case, result) in results {
            if result.is_failed() {
                failures.insert(case.full_name());
            }
        }

        if let Err(error) = save_expected_failures(&self.expected_failures_path, &failures) {
            eprintln!(
                "failed to update {}: {error}",
                self.expected_failures_path.display()
            );
            return;
        }

        println!(
            "  {} updated with {} failures",
            self.expected_failures_path.display(),
            failures.len()
        );
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        let Some(lsp_case) = self.tests.get(&case.full_name()) else {
            return TestResult::Failed {
                message: "test not found".to_string(),
            };
        };

        runner::run_mdtest_case(&case.path, lsp_case)
    }
}
