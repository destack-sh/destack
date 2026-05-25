use std::sync::Mutex;

use crate::conformance::{suite_case, update_catalog_report_target};
use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use super::{ConformanceSuiteResult, print_summary, run_babel, run_biome, run_swc, run_test262};

/// Selection of ECMA conformance suites to run.
#[derive(Debug, Clone, Copy, Default)]
pub struct EcmaConformanceSelection {
    /// Run test262 suite.
    pub test262: bool,
    /// Run Babel parser suite.
    pub babel: bool,
    /// Run SWC parser suite.
    pub swc: bool,
    /// Run Biome parser suite.
    pub biome: bool,
}

impl EcmaConformanceSelection {
    /// Return whether no explicit suite was selected.
    pub fn is_all_disabled(&self) -> bool {
        !self.test262 && !self.babel && !self.swc && !self.biome
    }
}

/// An ECMA conformance test suite.
#[derive(Debug)]
pub struct EcmaConformanceSuite {
    /// Selection of ECMA conformance suites to run.
    pub selection: EcmaConformanceSelection,
    /// Update known-failures file with current failures.
    pub update_known_failures: bool,
    /// Filter tests inside a selected suite.
    pub suite_filter: Option<String>,
    /// Results of the ECMA conformance suites.
    results: Mutex<Vec<ConformanceSuiteResult>>,
}

impl EcmaConformanceSuite {
    /// Create a new ECMA conformance test suite.
    pub fn new(
        selection: EcmaConformanceSelection,
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

impl Suite for EcmaConformanceSuite {
    fn name(&self) -> &'static str {
        "conformance-ecma"
    }

    fn case_noun(&self) -> &'static str {
        "suites"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let run_all = self.selection.is_all_disabled();

        let mut cases = Vec::new();

        // test262
        if run_all || self.selection.test262 {
            cases.push(suite_case("ecma", "test262"));
        }

        // babel
        if run_all || self.selection.babel {
            cases.push(suite_case("ecma", "babel"));
        }

        // swc
        if run_all || self.selection.swc {
            cases.push(suite_case("ecma", "swc"));
        }

        // biome
        if run_all || self.selection.biome {
            cases.push(suite_case("ecma", "biome"));
        }

        cases
    }

    fn run(&self, case: &Case, context: &RunContext<'_>) -> CaseResult {
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
                return CaseResult::Failed {
                    message: format!("unknown ECMA conformance suite: {other}"),
                };
            }
        };

        let Some(suite_result) = suite_result else {
            return CaseResult::Failed {
                message: format!(
                    "ECMA conformance fixture setup failed for suite '{}'",
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
            return CaseResult::Failed {
                message: format!("{regressions_len} regressions"),
            };
        }

        CaseResult::Passed
    }

    fn report(&self, _results: &[(Case, CaseResult)], _context: &RunContext<'_>) {
        let Ok(results) = self.results.lock() else {
            return;
        };

        print_summary(&results, None);

        if let Err(error) = update_catalog_report_target() {
            eprintln!("failed to update conformance catalog: {error}");
        }
    }
}
