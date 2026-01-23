use std::process::ExitCode;
use std::time::Instant;

use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use super::print::color;
use super::{
    RunContext, Suite, TestCase, TestOptions, TestResult, TestSummary, filter_tests,
    print_failures, print_result, print_summary, print_test_list,
};
use std::collections::HashSet;

/// Shared runner implementation for all `destack_test` suites.
#[derive(Debug)]
pub struct Runner;

/// Result tuple from running test cases.
type RunCasesResult = (
    ExitCode,
    Vec<(TestCase, TestResult)>,
    Vec<(TestCase, TestResult)>,
    Option<ExpectedFailureSummary>,
);

impl Runner {
    pub fn run_suite<S: Suite>(suite: &S, options: &TestOptions) -> ExitCode {
        let context = RunContext {
            options,
            timeout: suite.timeout(),
        };

        let cases = suite.discover(options);
        let expected_failures = suite.expected_failures(options);
        let (exit_code, results, raw_results, expected_summary) =
            Self::run_cases_and_collect(cases, expected_failures, &context, |case, ctx| {
                suite.run(case, ctx)
            });

        let report_results = if context.options.update_known_failures {
            &raw_results
        } else {
            &results
        };
        suite.report(report_results, &context);
        if let Some(summary) = expected_summary {
            Self::print_expected_failure_summary(&summary);
        }

        exit_code
    }

    pub fn run_cases<F>(cases: Vec<TestCase>, context: &RunContext<'_>, run: F) -> ExitCode
    where
        F: Fn(&TestCase, &RunContext<'_>) -> TestResult + Send + Sync,
    {
        let (exit_code, _results, _raw_results, _summary) =
            Self::run_cases_and_collect(cases, None, context, run);
        exit_code
    }

    fn run_cases_and_collect<F>(
        cases: Vec<TestCase>,
        expected_failures: Option<&HashSet<String>>,
        context: &RunContext<'_>,
        run: F,
    ) -> RunCasesResult
    where
        F: Fn(&TestCase, &RunContext<'_>) -> TestResult + Send + Sync,
    {
        let filtered = filter_tests(cases, context.options.filter.as_deref());

        if filtered.is_empty() {
            println!("no tests to run");
            return (ExitCode::SUCCESS, Vec::new(), Vec::new(), None);
        }

        if context.options.list {
            print_test_list(&filtered);
            return (ExitCode::SUCCESS, Vec::new(), Vec::new(), None);
        }

        println!();
        println!("running {} tests", filtered.len());

        let summary = TestSummary::new();
        let start = Instant::now();

        let results: Vec<(TestCase, TestResult, std::time::Duration)> =
            if context.options.parallel() {
                let jobs = context.options.jobs.max(1);
                let thread_pool = ThreadPoolBuilder::new()
                    .num_threads(jobs)
                    .build()
                    .expect("failed to build rayon thread pool");

                thread_pool.install(|| {
                    filtered
                        .par_iter()
                        .map(|case| {
                            let case_start = Instant::now();

                            let result = if case.is_skipped {
                                TestResult::Skipped {
                                    reason: "marked as skipped".to_string(),
                                }
                            } else {
                                run(case, context)
                            };

                            let duration = case_start.elapsed();

                            // check timeout after test completes
                            let result = if let Some(timeout) = context.timeout {
                                if duration > timeout {
                                    TestResult::Failed {
                                        message: format!(
                                            "timeout: took {:.2}s, limit {:.2}s",
                                            duration.as_secs_f64(),
                                            timeout.as_secs_f64()
                                        ),
                                    }
                                } else {
                                    result
                                }
                            } else {
                                result
                            };

                            (case.clone(), result, duration)
                        })
                        .collect()
                })
            } else {
                filtered
                    .iter()
                    .map(|case| {
                        let case_start = Instant::now();

                        let result = if case.is_skipped {
                            TestResult::Skipped {
                                reason: "marked as skipped".to_string(),
                            }
                        } else {
                            run(case, context)
                        };

                        let duration = case_start.elapsed();

                        // check timeout after test completes
                        let result = if let Some(timeout) = context.timeout {
                            if duration > timeout {
                                TestResult::Failed {
                                    message: format!(
                                        "timeout: took {:.2}s, limit {:.2}s",
                                        duration.as_secs_f64(),
                                        timeout.as_secs_f64()
                                    ),
                                }
                            } else {
                                result
                            }
                        } else {
                            result
                        };

                        (case.clone(), result, duration)
                    })
                    .collect()
            };

        let total_duration = start.elapsed();

        let mut final_results: Vec<(TestCase, TestResult)> = Vec::new();
        let mut raw_results: Vec<(TestCase, TestResult)> = Vec::new();
        let mut expected_summary = ExpectedFailureSummary::new(expected_failures);
        for (case, result, duration) in results {
            raw_results.push((case.clone(), result.clone()));
            let result = expected_summary.update(&case, result);
            summary.record(&result);
            print_result(&case, &result, duration, context.options.verbose);
            final_results.push((case, result));
        }

        print_failures(&final_results);
        print_summary(&summary, total_duration);

        let exit_code = if summary.all_passed() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };

        let expected_summary = expected_failures.map(|_| expected_summary);
        (exit_code, final_results, raw_results, expected_summary)
    }

    fn print_expected_failure_summary(summary: &ExpectedFailureSummary) {
        let regressions = summary.regressions.len();
        let fixed = summary.fixed.len();
        let known_failed = summary.known_failed.len();

        if regressions == 0 && fixed == 0 && known_failed == 0 {
            return;
        }

        if fixed > 0 {
            println!("{} {} tests fixed", color::green("FIXED:"), fixed);
            for name in summary.fixed.iter().take(20) {
                println!("  {name}");
            }
            if fixed > 20 {
                println!("  ...and {} more", fixed - 20);
            }
        }

        if regressions > 0 {
            println!(
                "{} {} regressions detected",
                color::red("REGRESSIONS:"),
                regressions
            );
            for name in summary.regressions.iter().take(20) {
                println!("  {name}");
            }
            if regressions > 20 {
                println!("  ...and {} more", regressions - 20);
            }
        }

        if known_failed > 0 {
            println!(
                "{} {} known failures",
                color::yellow("KNOWN FAILURES:"),
                known_failed
            );
        }
    }
}

#[derive(Debug, Default)]
struct ExpectedFailureSummary {
    /// Set of known failures.
    expected: Option<HashSet<String>>,
    /// Tests that failed but were expected to pass.
    regressions: Vec<String>,
    /// Tests that passed but were expected to fail.
    fixed: Vec<String>,
    /// Tests that failed and are in the expected list.
    known_failed: Vec<String>,
}

impl ExpectedFailureSummary {
    fn new(expected: Option<&HashSet<String>>) -> Self {
        let expected = expected.map(|set| set.iter().cloned().collect());
        Self {
            expected,
            regressions: Vec::new(),
            fixed: Vec::new(),
            known_failed: Vec::new(),
        }
    }

    fn update(&mut self, case: &TestCase, result: TestResult) -> TestResult {
        let Some(expected) = self.expected.as_ref() else {
            return result;
        };

        if result.is_suite() || result.is_skipped() {
            return result;
        }

        let full_name = case.full_name();
        let name = &case.name;
        let is_expected = expected.contains(&full_name) || expected.contains(name);

        if result.is_failed() {
            if is_expected {
                self.known_failed.push(full_name);
                return TestResult::Skipped {
                    reason: "known failure".to_string(),
                };
            }
            self.regressions.push(full_name);
            return result;
        }

        if result.is_passed() && is_expected {
            self.fixed.push(full_name);
        }

        result
    }
}
