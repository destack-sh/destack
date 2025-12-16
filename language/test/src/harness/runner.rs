use std::process::ExitCode;
use std::time::Instant;

use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use super::{
    RunContext, Suite, TestCase, TestOptions, TestResult, TestSummary, filter_tests,
    print_failures, print_result, print_summary, print_test_list,
};

/// Shared runner implementation for all `destack_test` suites.
#[derive(Debug)]
pub struct Runner;


impl Runner {
    pub fn run_suite<S: Suite>(suite: &S, options: &TestOptions) -> ExitCode {
        let context = RunContext {
            options,
            timeout: suite.timeout(),
        };

        let cases = suite.discover(options);
        let (exit_code, results) =
            Self::run_cases_and_collect(cases, &context, |case, ctx| suite.run(case, ctx));

        suite.report(&results, &context);

        exit_code
    }

    pub fn run_cases<F>(cases: Vec<TestCase>, context: &RunContext<'_>, run: F) -> ExitCode
    where
        F: Fn(&TestCase, &RunContext<'_>) -> TestResult + Send + Sync,
    {
        let (exit_code, _results) = Self::run_cases_and_collect(cases, context, run);
        exit_code
    }

    fn run_cases_and_collect<F>(
        cases: Vec<TestCase>,
        context: &RunContext<'_>,
        run: F,
    ) -> (ExitCode, Vec<(TestCase, TestResult)>)
    where
        F: Fn(&TestCase, &RunContext<'_>) -> TestResult + Send + Sync,
    {
        let filtered = filter_tests(cases, context.options.filter.as_deref());

        if filtered.is_empty() {
            println!("no tests to run");
            return (ExitCode::SUCCESS, Vec::new());
        }

        if context.options.list {
            print_test_list(&filtered);
            return (ExitCode::SUCCESS, Vec::new());
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
                                            "timeout: took {:?}, limit {:?}",
                                            duration, timeout
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
                                        "timeout: took {:?}, limit {:?}",
                                        duration, timeout
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
        for (case, result, duration) in results {
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

        (exit_code, final_results)
    }
}
