use std::sync::Mutex;

use crate::harness::{RunContext, Suite, TestCase, TestOptions, TestResult};

use super::{
    SuiteResult, load_readme_baseline, print_summary, run_babel, run_biome, run_swc, run_test262,
    update_readme,
};

/// Selection of conformance suites to run.
#[derive(Debug, Clone, Copy, Default)]
pub struct ConformanceSelection {
    /// Run test262 suite.
    pub test262: bool,
    /// Run Babel parser suite.
    pub babel: bool,
    /// Run SWC parser suite.
    pub swc: bool,
    /// Run Biome parser suite.
    pub biome: bool,
}

impl ConformanceSelection {
    pub fn is_all_disabled(&self) -> bool {
        !self.test262 && !self.babel && !self.swc && !self.biome
    }
}

/// A conformance test suite.
#[derive(Debug)]
pub struct ConformanceHarnessSuite {
    /// Selection of conformance suites to run.
    pub selection: ConformanceSelection,
    /// Update known-failures file with current failures.
    pub update_known_failures: bool,
    /// Results of the conformance tests.
    results: Mutex<Vec<SuiteResult>>,
}

impl ConformanceHarnessSuite {
    /// Create a new conformance test suite.
    pub fn new(selection: ConformanceSelection, update_known_failures: bool) -> Self {
        Self {
            selection,
            update_known_failures,
            results: Mutex::new(Vec::new()),
        }
    }
}

impl Suite for ConformanceHarnessSuite {
    fn name(&self) -> &'static str {
        "conformance"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let run_all = self.selection.is_all_disabled();

        let mut cases = Vec::new();

        // test262
        if run_all || self.selection.test262 {
            cases.push(TestCase::directory(
                "test262",
                "fixtures/conformance/test262",
                "destack_test::conformance",
            ));
        }

        // babel
        if run_all || self.selection.babel {
            cases.push(TestCase::directory(
                "babel",
                "fixtures/conformance/babel",
                "destack_test::conformance",
            ));
        }

        // swc
        if run_all || self.selection.swc {
            cases.push(TestCase::directory(
                "swc",
                "fixtures/conformance/swc",
                "destack_test::conformance",
            ));
        }

        // biome
        if run_all || self.selection.biome {
            cases.push(TestCase::directory(
                "biome",
                "fixtures/conformance/biome",
                "destack_test::conformance",
            ));
        }

        cases
    }

    fn run(&self, case: &TestCase, context: &RunContext<'_>) -> TestResult {
        let suite_result = match case.name.as_str() {
            "test262" => run_test262(context.options, self.update_known_failures),
            "babel" => run_babel(context.options, self.update_known_failures),
            "swc" => run_swc(context.options, self.update_known_failures),
            "biome" => run_biome(context.options, self.update_known_failures),
            other => {
                return TestResult::Failed {
                    message: format!("unknown conformance suite: {other}"),
                };
            }
        };

        let Some(suite_result) = suite_result else {
            return TestResult::Skipped {
                reason: "suite not found (run: just language/install-fixtures)".to_string(),
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

    fn report(&self, _results: &[(TestCase, TestResult)], context: &RunContext<'_>) {
        let Ok(results) = self.results.lock() else {
            return;
        };

        // load baseline for delta display
        let baseline = load_readme_baseline();
        print_summary(&results, baseline.as_ref());

        // update README.md with results (skip if filtering is applied)
        if context.options.filter.is_some() {
            return;
        }

        // determine if this is a partial run (not all suites)
        let run_all = self.selection.is_all_disabled();
        let is_partial = !run_all;

        update_readme(&results, is_partial);
    }
}
