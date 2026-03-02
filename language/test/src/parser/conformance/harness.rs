use std::sync::Mutex;

use crate::harness::{RunContext, Suite, TestCase, TestOptions, TestResult};

use super::{
    SuiteResult, load_readme_baseline, print_summary, run_babel, run_biome, run_swc, run_test262,
    update_readme,
};

/// Selection of parser conformance suites to run.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParserConformanceSelection {
    /// Run test262 suite.
    pub test262: bool,
    /// Run Babel parser suite.
    pub babel: bool,
    /// Run SWC parser suite.
    pub swc: bool,
    /// Run Biome parser suite.
    pub biome: bool,
}

impl ParserConformanceSelection {
    pub fn is_all_disabled(&self) -> bool {
        !self.test262 && !self.babel && !self.swc && !self.biome
    }
}

/// A parser conformance test suite.
#[derive(Debug)]
pub struct ParserConformanceHarnessSuite {
    /// Selection of parser conformance suites to run.
    pub selection: ParserConformanceSelection,
    /// Update known-failures file with current failures.
    pub update_known_failures: bool,
    /// Filter tests inside a selected suite.
    pub suite_filter: Option<String>,
    /// Results of the parser conformance tests.
    results: Mutex<Vec<SuiteResult>>,
}

impl ParserConformanceHarnessSuite {
    /// Create a new parser conformance test suite.
    pub fn new(
        selection: ParserConformanceSelection,
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

impl Suite for ParserConformanceHarnessSuite {
    fn name(&self) -> &'static str {
        "parser-conformance"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let run_all = self.selection.is_all_disabled();

        let mut cases = Vec::new();

        // test262
        if run_all || self.selection.test262 {
            cases.push(TestCase::directory(
                "test262",
                "fixtures/parser/conformance/test262",
                "destack_test::parser::conformance",
            ));
        }

        // babel
        if run_all || self.selection.babel {
            cases.push(TestCase::directory(
                "babel",
                "fixtures/parser/conformance/babel",
                "destack_test::parser::conformance",
            ));
        }

        // swc
        if run_all || self.selection.swc {
            cases.push(TestCase::directory(
                "swc",
                "fixtures/parser/conformance/swc",
                "destack_test::parser::conformance",
            ));
        }

        // biome
        if run_all || self.selection.biome {
            cases.push(TestCase::directory(
                "biome",
                "fixtures/parser/conformance/biome",
                "destack_test::parser::conformance",
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
            "test262" => run_test262(&suite_options, self.update_known_failures),
            "babel" => run_babel(&suite_options, self.update_known_failures),
            "swc" => run_swc(&suite_options, self.update_known_failures),
            "biome" => run_biome(&suite_options, self.update_known_failures),
            other => {
                return TestResult::Failed {
                    message: format!("unknown parser conformance suite: {other}"),
                };
            }
        };

        let Some(suite_result) = suite_result else {
            return TestResult::Failed {
                message: format!(
                    "parser conformance fixture setup failed for suite '{}'",
                    case.name
                ),
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

        // determine if this is a partial run (not all suites or filtered suites)
        let run_all = self.selection.is_all_disabled();
        let is_partial = !run_all || has_filtered_suites;

        update_readme(&results, is_partial);
    }
}
