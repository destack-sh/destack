use std::sync::Mutex;

use crate::conformance::{suite_case, update_catalog_report_targets};
use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use super::{ConformanceSuiteResult, print_summary, run_oxfmt};

/// Selection of formatter conformance suites to run.
#[derive(Debug, Clone, Copy, Default)]
pub struct FormatterConformanceSelection {
    /// Run oxfmt formatter suite.
    pub oxfmt: bool,
}

impl FormatterConformanceSelection {
    /// Return whether no explicit suite was selected.
    pub fn is_all_disabled(&self) -> bool {
        !self.oxfmt
    }
}

/// A formatter conformance test suite.
#[derive(Debug)]
pub struct FormatterConformanceSuite {
    /// Selection of conformance suites to run.
    pub selection: FormatterConformanceSelection,
    /// Update known-failures file with current failures.
    pub update_known_failures: bool,
    /// Filter tests inside a selected conformance suite.
    pub suite_filter: Option<String>,
    /// Results of the conformance tests.
    results: Mutex<Vec<ConformanceSuiteResult>>,
}

impl FormatterConformanceSuite {
    /// Create a new formatter conformance test suite.
    pub fn new(
        selection: FormatterConformanceSelection,
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

impl Suite for FormatterConformanceSuite {
    fn name(&self) -> &'static str {
        "conformance-formatter"
    }

    fn case_noun(&self) -> &'static str {
        "suites"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let run_all = self.selection.is_all_disabled();

        let mut cases = Vec::new();

        // oxfmt
        if run_all || self.selection.oxfmt {
            cases.push(suite_case("formatter", "oxfmt"));
        }

        cases
    }

    fn run(&self, case: &Case, context: &RunContext<'_>) -> CaseResult {
        let mut suite_options = context.options.clone();
        if let Some(filter) = &self.suite_filter {
            suite_options.filter = Some(filter.clone());
        }

        let suite_result = match case.name.as_str() {
            "oxfmt" => run_oxfmt(&suite_options, self.update_known_failures),
            other => {
                return CaseResult::Failed {
                    message: format!("unknown formatter conformance suite: {other}"),
                };
            }
        };

        let Some(suite_result) = suite_result else {
            return CaseResult::Skipped {
                reason: "suite not found (run: just language/install-conformance-formatter)"
                    .to_string(),
            };
        };

        let regressions_len = suite_result.result.regressions.len();
        let has_regressions = suite_result.result.has_regressions();
        let runs_known_failures = suite_options.runs_known_failures();

        if let Ok(mut results) = self.results.lock() {
            results.push(suite_result);
        }

        if has_regressions {
            return CaseResult::Failed {
                message: if runs_known_failures {
                    format!("{regressions_len} failures")
                } else {
                    format!("{regressions_len} regressions")
                },
            };
        }

        CaseResult::Passed
    }

    fn report(&self, _results: &[(Case, CaseResult)], _context: &RunContext<'_>) {
        let Ok(results) = self.results.lock() else {
            return;
        };

        print_summary(&results, None);

        if let Err(error) = update_catalog_report_targets() {
            eprintln!("failed to update conformance catalog: {error}");
        }
    }
}
