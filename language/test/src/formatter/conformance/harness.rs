use std::sync::Mutex;

use crate::harness::{RunContext, Suite, TestCase, TestOptions, TestResult};

use super::{
    SuiteResult, load_readme_baseline, print_summary, run_biome, run_oxfmt, run_prettier,
    update_readme,
};

/// Selection of formatter conformance suites to run.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConformanceSelection {
    /// Run Biome formatter suite.
    pub biome: bool,
    /// Run Prettier formatter suite.
    pub prettier: bool,
    /// Run oxfmt formatter suite.
    pub oxfmt: bool,
}

impl ConformanceSelection {
    pub fn is_all_disabled(&self) -> bool {
        !self.biome && !self.prettier && !self.oxfmt
    }
}

/// A formatter conformance test suite.
#[derive(Debug)]
pub struct ConformanceHarnessSuite {
    /// Selection of conformance suites to run.
    pub selection: ConformanceSelection,
    /// Update known-failures file with current failures.
    pub update_known_failures: bool,
    /// Filter tests inside a selected conformance suite.
    pub suite_filter: Option<String>,
    /// Results of the conformance tests.
    results: Mutex<Vec<SuiteResult>>,
}

impl ConformanceHarnessSuite {
    /// Create a new formatter conformance test suite.
    pub fn new(
        selection: ConformanceSelection,
        update_known_failures: bool,
        suite_filter: Option<String>,
    ) -> Self {
        Self {
            selection,
            update_known_failures,
            suite_filter,
            results: Mutex::new(Vec::new()),
        }
    }
}

impl Suite for ConformanceHarnessSuite {
    fn name(&self) -> &'static str {
        "formatter-conformance"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let run_all = self.selection.is_all_disabled();

        let mut cases = Vec::new();

        // biome
        if run_all || self.selection.biome {
            cases.push(TestCase::directory(
                "biome",
                "fixtures/formatter/conformance/staging/biome",
                "destack_test::formatter::conformance",
            ));
        }

        // prettier
        if run_all || self.selection.prettier {
            cases.push(TestCase::directory(
                "prettier",
                "fixtures/formatter/conformance/staging/prettier",
                "destack_test::formatter::conformance",
            ));
        }

        // oxfmt
        if run_all || self.selection.oxfmt {
            cases.push(TestCase::directory(
                "oxfmt",
                "fixtures/formatter/conformance/staging/oxfmt",
                "destack_test::formatter::conformance",
            ));
        }

        cases
    }

    fn run(&self, case: &TestCase, context: &RunContext<'_>) -> TestResult {
        let mut suite_options = context.options.clone();
        if let Some(filter) = &self.suite_filter {
            suite_options.filter = Some(filter.clone());
        }

        let suite_result = match case.name.as_str() {
            "biome" => run_biome(&suite_options, self.update_known_failures),
            "prettier" => run_prettier(&suite_options, self.update_known_failures),
            "oxfmt" => run_oxfmt(&suite_options, self.update_known_failures),
            other => {
                return TestResult::Failed {
                    message: format!("unknown formatter conformance suite: {other}"),
                };
            }
        };

        let Some(suite_result) = suite_result else {
            return TestResult::Skipped {
                reason: "suite not found (run: just language/install-formatter-conformance)"
                    .to_string(),
            };
        };

        let regressions_len = suite_result.result.regressions.len();
        let has_regressions = suite_result.result.has_regressions();

        if let Ok(mut results) = self.results.lock() {
            results.push(suite_result);
        }

        if has_regressions {
            return TestResult::Failed {
                message: format!("{regressions_len} regressions"),
            };
        }

        TestResult::Passed
    }

    fn report(&self, _results: &[(TestCase, TestResult)], _context: &RunContext<'_>) {
        let Ok(results) = self.results.lock() else {
            return;
        };

        // load baseline for delta display
        let baseline = load_readme_baseline();
        print_summary(&results, baseline.as_ref());

        let has_filtered_suites = results.iter().any(|suite| suite.result.is_filtered());

        // determine if this is a partial run: not all suites or filtered suites
        let run_all = self.selection.is_all_disabled();
        let is_partial = !run_all || has_filtered_suites;

        update_readme(&results, is_partial);
    }
}
