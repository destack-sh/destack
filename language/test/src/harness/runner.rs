use std::process::ExitCode;
use std::sync::mpsc;
use std::time::{Duration, Instant};

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
    fn run_single_case<F>(
        index: usize,
        case: &TestCase,
        context: &RunContext<'_>,
        expected_failures: Option<&HashSet<String>>,
        skip_known_failures: bool,
        skip_ignored: bool,
        run: &F,
    ) -> (usize, TestCase, TestResult, Duration)
    where
        F: Fn(&TestCase, &RunContext<'_>) -> TestResult + Send + Sync,
    {
        let case_start = Instant::now();

        let result = match skip_reason(case, expected_failures, skip_known_failures, skip_ignored) {
            Some(reason) => TestResult::Skipped { reason },
            None => run(case, context),
        };

        let duration = case_start.elapsed();

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

        (index, case.clone(), result, duration)
    }

    fn record_case_result(
        index: usize,
        case: TestCase,
        result: TestResult,
        duration: Duration,
        summary: &mut TestSummary,
        expected_summary: &mut Option<ExpectedFailureSummary>,
        raw_results: &mut Vec<(usize, TestCase, TestResult)>,
        final_results: &mut Vec<(usize, TestCase, TestResult)>,
        verbose: bool,
    ) {
        raw_results.push((index, case.clone(), result.clone()));

        let result = match expected_summary.as_mut() {
            Some(summary) => {
                if is_known_failure_skip(&result) {
                    summary.record_known_failure(&case);
                    result
                } else {
                    summary.update(&case, result)
                }
            }
            None => result,
        };

        summary.record(&result);
        print_result(&case, &result, duration, verbose);
        final_results.push((index, case, result));
    }

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

        let mut summary = TestSummary::new();
        let start = Instant::now();
        let skip_known_failures = !context.options.include_known_failures_effective();
        let skip_ignored = !context.options.include_ignored_effective();
        let track_expected_failures = !context.options.include_known_failures_effective();
        let mut expected_summary =
            track_expected_failures.then(|| ExpectedFailureSummary::new(expected_failures));

        let mut final_results_indexed: Vec<(usize, TestCase, TestResult)> = Vec::new();
        let mut raw_results_indexed: Vec<(usize, TestCase, TestResult)> = Vec::new();
        if context.options.parallel() && context.options.jobs > 1 {
            let jobs = context.options.jobs.max(1);
            let thread_pool = ThreadPoolBuilder::new()
                .num_threads(jobs)
                .build()
                .expect("failed to build rayon thread pool");
            let total_cases = filtered.len();

            let (sender, receiver) = mpsc::channel();
            std::thread::scope(|scope| {
                let worker_sender = sender.clone();
                let worker = scope.spawn(move || {
                    thread_pool.install(|| {
                        filtered.par_iter().enumerate().for_each_with(
                            worker_sender,
                            |sender, (index, case)| {
                                let entry = Self::run_single_case(
                                    index,
                                    case,
                                    context,
                                    expected_failures,
                                    skip_known_failures,
                                    skip_ignored,
                                    &run,
                                );

                                sender
                                    .send(entry)
                                    .expect("failed to send test result from worker");
                            },
                        );
                    });
                });
                drop(sender);

                for _ in 0..total_cases {
                    let (index, case, result, duration) = receiver
                        .recv()
                        .expect("failed to receive test result from worker");
                    Self::record_case_result(
                        index,
                        case,
                        result,
                        duration,
                        &mut summary,
                        &mut expected_summary,
                        &mut raw_results_indexed,
                        &mut final_results_indexed,
                        context.options.verbose,
                    );
                }

                worker.join().expect("worker thread panicked");
            });
        } else {
            for (index, case) in filtered.iter().enumerate() {
                let (index, case, result, duration) = Self::run_single_case(
                    index,
                    case,
                    context,
                    expected_failures,
                    skip_known_failures,
                    skip_ignored,
                    &run,
                );
                Self::record_case_result(
                    index,
                    case,
                    result,
                    duration,
                    &mut summary,
                    &mut expected_summary,
                    &mut raw_results_indexed,
                    &mut final_results_indexed,
                    context.options.verbose,
                );
            }
        }

        let total_duration = start.elapsed();

        final_results_indexed.sort_by_key(|(index, _, _)| *index);
        raw_results_indexed.sort_by_key(|(index, _, _)| *index);

        let final_results: Vec<(TestCase, TestResult)> = final_results_indexed
            .into_iter()
            .map(|(_, case, result)| (case, result))
            .collect();
        let raw_results: Vec<(TestCase, TestResult)> = raw_results_indexed
            .into_iter()
            .map(|(_, case, result)| (case, result))
            .collect();

        print_failures(&final_results);
        print_summary(&summary, total_duration);

        let exit_code = if summary.all_passed() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };

        let expected_summary = if expected_failures.is_some() {
            expected_summary
        } else {
            None
        };
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

/// Compute a skip reason for a case when it should not run.
fn skip_reason(
    case: &TestCase,
    expected_failures: Option<&HashSet<String>>,
    skip_known_failures: bool,
    skip_ignored: bool,
) -> Option<String> {
    if case.is_skipped && skip_ignored {
        return Some("marked as skipped".to_string());
    }

    if skip_known_failures {
        let full_name = case.full_name();
        let is_known_failure = expected_failures
            .is_some_and(|set| set.contains(&full_name) || set.contains(&case.name));
        if is_known_failure {
            return Some("known failure".to_string());
        }
    }

    None
}

/// Check whether a result was skipped because it is a known failure.
fn is_known_failure_skip(result: &TestResult) -> bool {
    matches!(
        result,
        TestResult::Skipped { reason } if reason == "known failure"
    )
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

    /// Record a known failure that was skipped before running.
    fn record_known_failure(&mut self, case: &TestCase) {
        let Some(expected) = self.expected.as_ref() else {
            return;
        };

        let full_name = case.full_name();
        let is_expected = expected.contains(&full_name) || expected.contains(&case.name);
        if is_expected {
            self.known_failed.push(full_name);
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
