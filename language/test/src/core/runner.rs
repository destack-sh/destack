use std::collections::HashSet;
use std::process::ExitCode;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use super::print::color;
use super::{
    Case, CaseResult, RunContext, RunOptions, RunSummary, Suite, filter_cases, print_failures,
    print_result, print_summary, print_test_list,
};

/// Shared runner implementation for all `destack_test` suites.
#[derive(Debug)]
pub struct Runner;

/// Result tuple from running suite cases.
type RunCasesResult = (
    ExitCode,
    Vec<(Case, CaseResult)>,
    Vec<(Case, CaseResult)>,
    Option<ExpectedFailureSummary>,
);

impl Runner {
    /// Run one case and capture its elapsed time.
    fn run_single_case<F>(
        index: usize,
        case: &Case,
        context: &RunContext<'_>,
        expected_failures: Option<&HashSet<String>>,
        skip_known_failures: bool,
        skip_skipped: bool,
        run: &Arc<F>,
    ) -> (usize, Case, CaseResult, Duration)
    where
        F: Fn(&Case, &RunContext<'_>) -> CaseResult + Send + Sync + 'static,
    {
        // measure the full case duration
        let case_start = Instant::now();

        // resolve skipping before running the case body
        let result = match skip_reason(case, expected_failures, skip_known_failures, skip_skipped) {
            Some(reason) => CaseResult::Skipped { reason },
            None => run_case_with_timeout(case, context, run),
        };

        // return the indexed result tuple
        let duration = case_start.elapsed();
        (index, case.clone(), result, duration)
    }

    /// Record one case result into the shared summaries.
    #[allow(clippy::too_many_arguments)]
    fn record_case_result(
        index: usize,
        case: Case,
        result: CaseResult,
        duration: Duration,
        summary: &mut RunSummary,
        expected_summary: &mut Option<ExpectedFailureSummary>,
        raw_results: &mut Vec<(usize, Case, CaseResult)>,
        final_results: &mut Vec<(usize, Case, CaseResult)>,
        verbose: bool,
    ) {
        // keep the raw pre-baseline result for suite specific reporting
        raw_results.push((index, case.clone(), result.clone()));

        // rewrite the result through known-failure tracking when enabled
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

        // update summaries and terminal output
        summary.record(&result);
        print_result(&case, &result, duration, verbose);

        // preserve stable output ordering
        final_results.push((index, case, result));
    }

    /// Run one suite through the shared harness.
    pub fn run_suite<S: Suite + 'static>(suite: S, options: &RunOptions) -> ExitCode {
        // build the shared suite context
        let suite = Arc::new(suite);
        let context = RunContext {
            options,
            timeout: suite.timeout(),
        };
        let case_noun = suite.case_noun();

        // discover cases and adapt the suite into one runnable callback
        let cases = suite.discover(options);
        let expected_failures = suite.expected_failures(options);
        let suite_for_run = suite.clone();
        let run = Arc::new(move |case: &Case, ctx: &RunContext<'_>| suite_for_run.run(case, ctx));

        // execute the suite and collect both raw and rewritten results
        let (exit_code, results, raw_results, expected_summary) = Self::run_cases_and_collect(
            cases,
            expected_failures,
            &context,
            case_noun,
            suite.runs_in_parallel(),
            run,
        );

        // report raw results when baselines are being updated
        let report_results = if context.options.update_known_failures {
            &raw_results
        } else {
            &results
        };
        suite.report(report_results, &context);

        // print the expected-failure summary after the suite report
        if let Some(summary) = expected_summary {
            Self::print_expected_failure_summary(&summary);
        }

        exit_code
    }

    /// Run plain cases without suite specific reporting.
    pub fn run_cases<F>(cases: Vec<Case>, context: &RunContext<'_>, run: F) -> ExitCode
    where
        F: Fn(&Case, &RunContext<'_>) -> CaseResult + Send + Sync + 'static,
    {
        let run = Arc::new(run);
        let (exit_code, _results, _raw_results, _summary) =
            Self::run_cases_and_collect(cases, None, context, "tests", true, run);
        exit_code
    }

    /// Run a case list and collect both raw and rewritten results.
    fn run_cases_and_collect<F>(
        cases: Vec<Case>,
        expected_failures: Option<&HashSet<String>>,
        context: &RunContext<'_>,
        case_noun: &str,
        suite_runs_in_parallel: bool,
        run: Arc<F>,
    ) -> RunCasesResult
    where
        F: Fn(&Case, &RunContext<'_>) -> CaseResult + Send + Sync + 'static,
    {
        // filter the discovered cases first
        let filtered = filter_cases(cases, context.options.filter.as_deref());

        // handle empty or list-only execution early
        if filtered.is_empty() {
            println!("no tests to run");
            return (ExitCode::SUCCESS, Vec::new(), Vec::new(), None);
        }

        if context.options.list {
            print_test_list(&filtered);
            return (ExitCode::SUCCESS, Vec::new(), Vec::new(), None);
        }

        println!();
        println!("running {} {case_noun}", filtered.len());

        // initialize shared execution state
        let mut summary = RunSummary::new();
        let start = Instant::now();
        let skip_known_failures = !context.options.runs_known_failures();
        let skip_skipped = !context.options.runs_skipped();
        let track_expected_failures = !context.options.runs_known_failures();
        let mut expected_summary =
            track_expected_failures.then(|| ExpectedFailureSummary::new(expected_failures));
        let abort_on_timeout = context.options.aborts_on_timeout();
        let should_run_parallel = should_run_parallel(
            context.options,
            context.timeout,
            abort_on_timeout,
            suite_runs_in_parallel,
        );

        let mut final_results_indexed: Vec<(usize, Case, CaseResult)> = Vec::new();
        let mut raw_results_indexed: Vec<(usize, Case, CaseResult)> = Vec::new();

        // run in parallel when the suite and options both allow it
        if should_run_parallel {
            let jobs = context.options.jobs.max(1);
            let thread_pool = ThreadPoolBuilder::new()
                .num_threads(jobs)
                .build()
                .expect("failed to build rayon thread pool");
            let total_cases = filtered.len();

            // stream worker results back to the main thread
            let (sender, receiver): (
                mpsc::Sender<(usize, Case, CaseResult, Duration)>,
                mpsc::Receiver<(usize, Case, CaseResult, Duration)>,
            ) = mpsc::channel();
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
                                    skip_skipped,
                                    &run,
                                );

                                sender
                                    .send(entry)
                                    .expect("failed to send case result from worker");
                            },
                        );
                    });
                });
                drop(sender);

                for _ in 0..total_cases {
                    let (index, case, result, duration) = receiver
                        .recv()
                        .expect("failed to receive case result from worker");
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
            // keep sequential mode simple when timeout aborts are active
            for (index, case) in filtered.iter().enumerate() {
                let (index, case, result, duration) = Self::run_single_case(
                    index,
                    case,
                    context,
                    expected_failures,
                    skip_known_failures,
                    skip_skipped,
                    &run,
                );
                let is_timeout = is_timeout_failure(&result);
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

                // stop early on timeout when requested
                if abort_on_timeout && is_timeout {
                    break;
                }
            }
        }

        // normalize result ordering before reporting
        let total_duration = start.elapsed();
        final_results_indexed.sort_by_key(|(index, _, _)| *index);
        raw_results_indexed.sort_by_key(|(index, _, _)| *index);

        // erase the internal ordering keys
        let final_results: Vec<(Case, CaseResult)> = final_results_indexed
            .into_iter()
            .map(|(_, case, result)| (case, result))
            .collect();
        let raw_results: Vec<(Case, CaseResult)> = raw_results_indexed
            .into_iter()
            .map(|(_, case, result)| (case, result))
            .collect();

        // print the shared terminal summary
        print_failures(&final_results);
        print_summary(&summary, total_duration);

        // compute the final exit code from the rewritten summary
        let exit_code = if summary.all_passed() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };

        // only return expected-failure details when tracking was enabled
        let expected_summary = if expected_failures.is_some() {
            expected_summary
        } else {
            None
        };

        (exit_code, final_results, raw_results, expected_summary)
    }

    /// Print the shared expected-failure summary block.
    fn print_expected_failure_summary(summary: &ExpectedFailureSummary) {
        let regressions = summary.regressions.len();
        let fixed = summary.fixed.len();
        let known_failed = summary.known_failed.len();

        // skip the block when nothing interesting happened
        if regressions == 0 && fixed == 0 && known_failed == 0 {
            return;
        }

        // report newly fixed cases first
        if fixed > 0 {
            println!("{} {} tests fixed", color::green("FIXED:"), fixed);
            for name in summary.fixed.iter().take(20) {
                println!("  {name}");
            }
            if fixed > 20 {
                println!("  ...and {} more", fixed - 20);
            }
        }

        // report regressions next
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

        // report the remaining known failures last
        if known_failed > 0 {
            println!(
                "{} {} known failures",
                color::yellow("KNOWN FAILURES:"),
                known_failed
            );
        }
    }
}

/// Run one case, enforcing a timeout when the suite declares one.
fn run_case_with_timeout<F>(case: &Case, context: &RunContext<'_>, run: &Arc<F>) -> CaseResult
where
    F: Fn(&Case, &RunContext<'_>) -> CaseResult + Send + Sync + 'static,
{
    // run inline when the suite does not declare a timeout
    let Some(timeout) = context.timeout else {
        return run(case, context);
    };

    // clone the shared state needed by the timeout worker
    let case = case.clone();
    let options = context.options.clone();
    let (sender, receiver) = mpsc::channel();
    let run = Arc::clone(run);

    // run the case on a detached worker and wait for the result
    std::thread::spawn(move || {
        let context = RunContext {
            options: &options,
            timeout: Some(timeout),
        };
        let result = run(&case, &context);
        let _ = sender.send(result);
    });

    // map timeout outcomes into explicit case failures
    match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => CaseResult::Failed {
            message: format!("timeout: exceeded {:.2}s limit", timeout.as_secs_f64()),
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => CaseResult::Failed {
            message: "test worker disconnected unexpectedly".to_string(),
        },
    }
}

/// Return whether one case failed because of a timeout.
fn is_timeout_failure(result: &CaseResult) -> bool {
    match result {
        CaseResult::Failed { message } => message.starts_with("timeout:"),
        _ => false,
    }
}

/// Return whether this run should use parallel execution.
fn should_run_parallel(
    options: &RunOptions,
    timeout: Option<Duration>,
    abort_on_timeout: bool,
    suite_runs_in_parallel: bool,
) -> bool {
    suite_runs_in_parallel
        && options.runs_in_parallel()
        && options.jobs > 1
        && !(timeout.is_some() && abort_on_timeout)
}

/// Compute a skip reason for a case when it should not run.
fn skip_reason(
    case: &Case,
    expected_failures: Option<&HashSet<String>>,
    skip_known_failures: bool,
    skip_skipped: bool,
) -> Option<String> {
    // skip explicitly skipped cases first
    if case.is_skipped && skip_skipped {
        return Some("marked as skipped".to_string());
    }

    // skip known failures when the run configuration requests it
    if skip_known_failures {
        let full_name = case.full_name();
        let is_known_failure = expected_failures.is_some_and(|set| set.contains(&full_name));
        if is_known_failure {
            return Some("known failure".to_string());
        }
    }

    None
}

/// Check whether a result was skipped because it is a known failure.
fn is_known_failure_skip(result: &CaseResult) -> bool {
    matches!(
        result,
        CaseResult::Skipped { reason } if reason == "known failure"
    )
}

#[derive(Debug, Default)]
struct ExpectedFailureSummary {
    /// Set of known failures.
    expected: Option<HashSet<String>>,
    /// Cases that failed but were expected to pass.
    regressions: Vec<String>,
    /// Cases that passed but were expected to fail.
    fixed: Vec<String>,
    /// Cases that failed and are in the expected list.
    known_failed: Vec<String>,
}

impl ExpectedFailureSummary {
    /// Build one expected-failure summary from the current baseline set.
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
    fn record_known_failure(&mut self, case: &Case) {
        let Some(expected) = self.expected.as_ref() else {
            return;
        };

        let full_name = case.full_name();
        let is_expected = expected.contains(&full_name);
        if is_expected {
            self.known_failed.push(full_name);
        }
    }

    fn update(&mut self, case: &Case, result: CaseResult) -> CaseResult {
        // skip bookkeeping when there is no expected-failure baseline
        let Some(expected) = self.expected.as_ref() else {
            return result;
        };

        // ignore suite summaries and already skipped cases
        if result.is_suite() || result.is_skipped() {
            return result;
        }

        // classify the case against the expected-failure set
        let full_name = case.full_name();
        let is_expected = expected.contains(&full_name);

        // rewrite failing expected cases into skipped known failures
        if result.is_failed() {
            if is_expected {
                self.known_failed.push(full_name);
                return CaseResult::Skipped {
                    reason: "known failure".to_string(),
                };
            }
            self.regressions.push(full_name);
            return result;
        }

        // track newly fixed cases without rewriting the result
        if result.is_passed() && is_expected {
            self.fixed.push(full_name);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{is_timeout_failure, should_run_parallel};
    use crate::core::{CaseResult, RunOptions};

    #[test]
    fn test_is_timeout_failure_matches_timeout_prefix() {
        let timeout = CaseResult::Failed {
            message: "timeout: exceeded 1.00s limit".to_string(),
        };
        let other = CaseResult::Failed {
            message: "panic: boom".to_string(),
        };

        assert!(is_timeout_failure(&timeout));
        assert!(!is_timeout_failure(&other));
        assert!(!is_timeout_failure(&CaseResult::Passed));
    }

    #[test]
    fn test_should_run_parallel_disables_parallel_abortable_timeouts() {
        let options = RunOptions {
            jobs: 8,
            ..RunOptions::default()
        };

        assert!(should_run_parallel(&options, None, true, true));
        assert!(should_run_parallel(
            &options,
            Some(Duration::from_secs(1)),
            false,
            true,
        ));
        assert!(!should_run_parallel(
            &options,
            Some(Duration::from_secs(1)),
            true,
            true,
        ));
    }

    #[test]
    fn test_should_run_parallel_respects_suite_capability() {
        let options = RunOptions {
            jobs: 8,
            ..RunOptions::default()
        };

        assert!(!should_run_parallel(&options, None, false, false));
    }
}
