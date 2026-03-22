use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::core::{
    Case, CaseResult, MarkdownSuiteIndex, RunContext, RunOptions, Suite, discover_markdown_suite,
    expected_failures_view, fixtures_dir, update_failure_baseline,
};
use crate::lsp::runner;
use crate::mdtest::MdTestCase;

const LSP_TEST_CATEGORY: &str = "destack_test::lsp";

/// One discovered markdown LSP suite.
#[derive(Debug, Default)]
pub struct LspSuite {
    /// The parsed markdown cases keyed by full test name.
    tests: HashMap<String, MdTestCase>,
    /// The discovered fixture cases.
    cases: Vec<Case>,
    /// Known failing tests for baseline tracking.
    expected_failures: HashSet<String>,
    /// Location of the known failures file.
    expected_failures_path: PathBuf,
}

impl LspSuite {
    /// Load all applied LSP fixtures from `fixtures/lsp`.
    pub fn load() -> Result<Self, String> {
        let fixtures = fixtures_dir();
        let lsp_dir = fixtures.join("lsp");
        let MarkdownSuiteIndex {
            mut cases,
            entries,
            expected_failures,
            expected_failures_path,
        } = discover_markdown_suite(&lsp_dir, LSP_TEST_CATEGORY, Some)?;

        cases.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(Self {
            tests: entries,
            cases,
            expected_failures,
            expected_failures_path,
        })
    }
}

impl Suite for LspSuite {
    fn name(&self) -> &'static str {
        "lsp"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn expected_failures(&self, _options: &RunOptions) -> Option<&HashSet<String>> {
        expected_failures_view(&self.expected_failures)
    }

    fn report(&self, results: &[(Case, CaseResult)], context: &RunContext<'_>) {
        if !context.options.update_known_failures {
            return;
        }

        let failure_count = match update_failure_baseline(&self.expected_failures_path, results) {
            Ok(failure_count) => failure_count,
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        };

        println!(
            "  {} updated with {} failures",
            self.expected_failures_path.display(),
            failure_count
        );
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        let Some(lsp_case) = self.tests.get(&case.full_name()) else {
            return CaseResult::Failed {
                message: "test not found".to_string(),
            };
        };

        runner::run_mdtest_case(&case.path, lsp_case)
    }
}
