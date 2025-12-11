use std::sync::Mutex;

use crate::harness::{RunContext, Suite, TestCase, TestOptions, TestResult};

use super::{SuiteResult, print_summary, run_babel, run_biome, run_swc, run_test262};

#[derive(Debug, Clone, Copy, Default)]
pub struct ConformanceSelection {
    pub test262: bool,
    pub babel: bool,
    pub swc: bool,
    pub biome: bool,
}

impl ConformanceSelection {
    pub fn is_all_disabled(&self) -> bool {
        !self.test262 && !self.babel && !self.swc && !self.biome
    }
}

#[derive(Debug)]
pub struct ConformanceHarnessSuite {
    pub selection: ConformanceSelection,
    pub update_known_failures: bool,
    results: Mutex<Vec<SuiteResult>>,
}

impl ConformanceHarnessSuite {
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
        if run_all || self.selection.test262 {
            cases.push(TestCase::directory(
                "test262",
                "fixtures/conformance/test262",
                "destack_test::conformance",
            ));
        }
        if run_all || self.selection.babel {
            cases.push(TestCase::directory(
                "babel",
                "fixtures/conformance/babel",
                "destack_test::conformance",
            ));
        }
        if run_all || self.selection.swc {
            cases.push(TestCase::directory(
                "swc",
                "fixtures/conformance/swc",
                "destack_test::conformance",
            ));
        }
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

    fn report(&self, _results: &[(TestCase, TestResult)], _context: &RunContext<'_>) {
        let Ok(results) = self.results.lock() else {
            return;
        };

        print_summary(&results);
    }
}
