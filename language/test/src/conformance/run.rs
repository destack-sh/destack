use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use crate::conformance::{CaseStatus, StatusEntry, StatusSet, compress_exact_selectors};
use crate::core::RunOptions;
use crate::core::print::color;

use super::{
    Case, CaseOutcome, CategoryStats, ConformanceDriver, ConformanceResult, ConformanceSuiteResult,
    print_conformance_result,
};

enum CaseRunResult {
    Completed(CaseOutcome),
    TimedOut,
}

/// Run one conformance driver with the shared harness.
pub fn run_conformance_driver<S: ConformanceDriver + 'static>(
    suite: &S,
    options: &RunOptions,
    update_known_failures: bool,
) -> Option<ConformanceSuiteResult> {
    // suite input
    if !suite.tests_dir().exists() {
        eprintln!(
            "{}: {} not found at {}",
            color::red("error"),
            suite.name(),
            suite.tests_dir().display()
        );
        eprintln!();
        eprintln!("{}", suite.fetch_instructions());
        return None;
    }

    // discovery and filtering
    let all_cases = suite.discover_cases();
    let discovered_case_names: HashSet<String> =
        all_cases.iter().map(|case| case.name.clone()).collect();
    let total_discovered = all_cases.len();
    let cases: Vec<_> = if let Some(filter) = &options.filter {
        all_cases
            .into_iter()
            .filter(|case| case.name.contains(filter))
            .collect()
    } else {
        all_cases
    };
    let total_selected = cases.len();
    let show_diff = options.verbose || options.filter.is_some();

    if options.list {
        println!(
            "{} cases in {}:",
            color::bold(&cases.len().to_string()),
            color::cyan(suite.name())
        );
        for case in &cases {
            println!("  {}", case.name);
        }
        return None;
    }

    // status loading
    let status_path = suite.status_path();
    let statuses = match load_suite_statuses(suite) {
        Ok(statuses) => statuses,
        Err(error) => {
            eprintln!("{}: {error}", color::red("error"));
            return None;
        }
    };
    let runs_known_failures = options.runs_known_failures();
    let runs_ignored = options.runs_ignored();
    let stale_ignored = stale_skipped_patterns(suite, &statuses, &discovered_case_names);

    let known_failure_count = cases
        .iter()
        .filter(|case| {
            status_for_case(suite, &statuses, &case.name).is_some_and(is_known_failure_status)
        })
        .count();

    let skipped_cases = cases
        .iter()
        .filter(|case| {
            status_for_case(suite, &statuses, &case.name).is_some_and(is_skipped_status)
                && !runs_ignored
        })
        .collect::<Vec<_>>();
    let skipped_count = skipped_cases.len();
    let runnable_cases = cases
        .iter()
        .filter(|case| {
            !status_for_case(suite, &statuses, &case.name).is_some_and(is_skipped_status)
                || runs_ignored
        })
        .map(|case| {
            let status = status_for_case(suite, &statuses, &case.name);
            if status == Some(CaseStatus::KnownFailIdempotence) && !runs_known_failures {
                case.clone().without_idempotence()
            } else {
                case.clone()
            }
        })
        .collect::<Vec<_>>();

    println!();
    println!(
        "running {} {} cases",
        color::bold(&cases.len().to_string()),
        color::cyan(suite.name())
    );

    if known_failure_count > 0 {
        println!(
            "  {} known failures loaded from {}",
            color::yellow(&known_failure_count.to_string()),
            status_path.display()
        );
    }

    if skipped_count > 0 {
        println!(
            "  {} skipped cases loaded from {}",
            color::dim(&skipped_count.to_string()),
            status_path.display()
        );
    }

    if !stale_ignored.is_empty() {
        println!(
            "  {} stale skipped entries in {}",
            color::yellow(&stale_ignored.len().to_string()),
            status_path.display()
        );

        let show_count = stale_ignored.len().min(20);
        for name in stale_ignored.iter().take(show_count) {
            println!("    - {name}");
        }

        if stale_ignored.len() > show_count {
            println!("    ... and {} more", stale_ignored.len() - show_count);
        }
    }

    println!();

    // execution
    let start = Instant::now();
    let progress_counter = AtomicUsize::new(0);
    let total_cases = runnable_cases.len();
    let verbose = options.verbose;
    let abort_on_timeout = options.aborts_on_timeout();

    let results: Vec<_> = if options.runs_in_parallel() {
        let jobs = options.jobs.max(1);
        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build()
            .expect("failed to build rayon thread pool");

        thread_pool.install(|| {
            runnable_cases
                .par_iter()
                .map(|case| {
                    let timeout = suite.timeout_for_case(case, options);
                    let outcome =
                        run_case_with_timeout(suite, case, timeout, abort_on_timeout, show_diff);

                    if verbose {
                        let completed = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        if completed.is_multiple_of(100) {
                            eprintln!(
                                "  progress: ~{}/{} ({:.0}%)",
                                completed,
                                total_cases,
                                completed as f64 / total_cases as f64 * 100.0
                            );
                        }
                    }

                    (case.name.clone(), outcome)
                })
                .collect()
        })
    } else {
        runnable_cases
            .iter()
            .map(|case| {
                let timeout = suite.timeout_for_case(case, options);
                let outcome =
                    run_case_with_timeout(suite, case, timeout, abort_on_timeout, show_diff);
                (case.name.clone(), outcome)
            })
            .collect()
    };

    // aggregation
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut timedout = 0;
    let mut regressions = Vec::new();
    let mut fixed = Vec::new();
    let mut unskipped = Vec::new();
    let mut timeouts = Vec::new();
    let mut current_failures = HashSet::new();
    let mut parse_failed = 0;
    let mut parse_failure_tests = Vec::new();
    let mut output_failed = 0;
    let mut idempotence_failed = 0;
    let mut read_failed = 0;
    let mut categories: BTreeMap<String, CategoryStats> = BTreeMap::new();
    let are_ignored_failures_strict = suite.ignored_failures_are_strict();

    // pre-count skipped cases
    for case in skipped_cases {
        let category = suite.category_for_case(&case.name);
        let stats = categories.entry(category).or_default();
        skipped += 1;
        stats.skipped += 1;
    }

    // aggregate executed cases
    for (name, outcome) in results {
        let status = status_for_case(suite, &statuses, &name);
        let is_skipped = status.is_some_and(is_skipped_status) && !runs_ignored;
        let is_known_failure = status.is_some_and(is_known_failure_status) && !runs_known_failures;
        let category = suite.category_for_case(&name);
        let stats = categories.entry(category).or_default();

        if is_skipped && !are_ignored_failures_strict {
            skipped += 1;
            stats.skipped += 1;
            continue;
        }

        match outcome {
            CaseRunResult::Completed(CaseOutcome::Passed) => {
                if is_skipped {
                    unskipped.push(name);
                    skipped += 1;
                    stats.skipped += 1;
                } else {
                    passed += 1;
                    stats.passed += 1;

                    let is_idempotence_check_suppressed =
                        status == Some(CaseStatus::KnownFailIdempotence) && !runs_known_failures;
                    if is_known_failure && !is_idempotence_check_suppressed {
                        fixed.push(name);
                    }
                }
            }

            CaseRunResult::Completed(
                failure_kind @ (CaseOutcome::FailedParse
                | CaseOutcome::FailedOutput
                | CaseOutcome::FailedIdempotence
                | CaseOutcome::FailedRead),
            ) => {
                if is_skipped {
                    skipped += 1;
                    stats.skipped += 1;
                } else {
                    failed += 1;
                    stats.failed += 1;

                    match failure_kind {
                        CaseOutcome::FailedParse => {
                            parse_failed += 1;
                            parse_failure_tests.push(name.clone());
                        }
                        CaseOutcome::FailedOutput => output_failed += 1,
                        CaseOutcome::FailedIdempotence => idempotence_failed += 1,
                        CaseOutcome::FailedRead => read_failed += 1,
                        CaseOutcome::Passed => unreachable!("passed outcome handled earlier"),
                    }

                    let is_expected_failure = status
                        .is_some_and(|status| status_matches_failure(status, failure_kind))
                        && !runs_known_failures;

                    if !is_expected_failure {
                        regressions.push(name.clone());
                    }

                    current_failures.insert(name);
                }
            }

            CaseRunResult::TimedOut => {
                if is_skipped {
                    skipped += 1;
                    stats.skipped += 1;
                } else {
                    timedout += 1;
                    stats.failed += 1;
                    timeouts.push(name.clone());

                    let is_expected_timeout = status.is_some_and(|status| {
                        matches!(status, CaseStatus::KnownFail | CaseStatus::Flaky)
                    }) && !runs_known_failures;

                    if !is_expected_timeout {
                        regressions.push(name.clone());
                    }

                    current_failures.insert(name);
                }
            }
        }
    }

    // final bookkeeping
    let duration = start.elapsed();

    if update_known_failures {
        if let Err(error) =
            save_known_failure_statuses(suite, &statuses, &current_failures, &discovered_case_names)
        {
            eprintln!("{}: failed to update status: {error}", color::red("error"));
        } else {
            println!(
                "  {} updated with {} known failures",
                status_path.display(),
                current_failures.len()
            );
        }

        regressions.clear();
    }

    let result = ConformanceResult {
        total_discovered,
        total_selected,
        passed,
        failed,
        skipped,
        timedout,
        parse_failed,
        parse_failure_tests,
        output_failed,
        idempotence_failed,
        read_failed,
        regressions,
        fixed,
        unskipped,
        timeouts,
        categories,
    };

    print_conformance_result(suite.name(), &result, duration, runs_known_failures);

    Some(ConformanceSuiteResult {
        name: suite.name().to_string(),
        result,
        duration,
        ran_known_failures: runs_known_failures,
    })
}

/// Return the first matching status for one case.
fn status_for_case<S: ConformanceDriver>(
    suite: &S,
    statuses: &StatusSet,
    case_name: &str,
) -> Option<CaseStatus> {
    statuses
        .match_entry(
            case_name,
            Some(suite.capability()),
            Some(suite.environment()),
        )
        .map(|entry| entry.status)
}

/// Return whether one status skips execution by default.
fn is_skipped_status(status: CaseStatus) -> bool {
    matches!(
        status,
        CaseStatus::Ignore | CaseStatus::EnvBlocked | CaseStatus::Manual
    )
}

/// Return whether one status counts as a known failure.
fn is_known_failure_status(status: CaseStatus) -> bool {
    matches!(
        status,
        CaseStatus::KnownFail | CaseStatus::KnownFailIdempotence | CaseStatus::Flaky
    )
}

/// Return whether one failure outcome is expected by a status.
fn status_matches_failure(status: CaseStatus, outcome: CaseOutcome) -> bool {
    match status {
        CaseStatus::KnownFail | CaseStatus::Flaky => true,
        CaseStatus::KnownFailIdempotence => outcome == CaseOutcome::FailedIdempotence,
        _ => false,
    }
}

/// Return the exact selectors declared by one status entry.
fn exact_selectors(entry: &StatusEntry) -> Vec<String> {
    entry
        .selectors()
        .into_iter()
        .filter(|selector| !selector.contains('*'))
        .collect()
}

/// Load the structured status file for one suite.
fn load_suite_statuses<S: ConformanceDriver>(suite: &S) -> Result<StatusSet, String> {
    StatusSet::load(&suite.status_path())
}

/// Return stale skipped selectors that no longer resolve to discovered cases.
fn stale_skipped_patterns<S: ConformanceDriver>(
    suite: &S,
    statuses: &StatusSet,
    discovered_case_names: &HashSet<String>,
) -> Vec<String> {
    let mut stale = Vec::new();

    // skipped entries
    for entry in &statuses.entries {
        if !is_skipped_status(entry.status) {
            continue;
        }

        // exact selectors only
        for selector in exact_selectors(entry) {
            let matches = status_for_case(suite, statuses, &selector);
            if matches.is_some() && !discovered_case_names.contains(&selector) {
                stale.push(selector);
            }
        }
    }

    stale.sort();
    stale
}

/// Save the current exact known-failure set back into `status.json`.
fn save_known_failure_statuses<S: ConformanceDriver>(
    suite: &S,
    statuses: &StatusSet,
    current_failures: &HashSet<String>,
    discovered_case_names: &HashSet<String>,
) -> Result<(), String> {
    let status_path = suite.status_path();

    // preserve non known-fail entries
    let mut entries = statuses
        .entries
        .iter()
        .filter(|entry| !matches!(entry.status, CaseStatus::KnownFail))
        .cloned()
        .collect::<Vec<_>>();

    // remove manually classified failure modes from auto generated known failures
    let current_failures = current_failures
        .iter()
        .filter(|selector| {
            status_for_case(suite, statuses, selector) != Some(CaseStatus::KnownFailIdempotence)
        })
        .cloned()
        .collect::<HashSet<_>>();

    // group failures by suite category
    let failures = compress_exact_selectors(&current_failures, discovered_case_names);
    let mut grouped_failures = BTreeMap::<String, Vec<String>>::new();

    for selector in failures {
        let category = suite.category_for_case(&selector);
        grouped_failures.entry(category).or_default().push(selector);
    }

    // regenerate deterministic known-fail entries
    for selectors in grouped_failures.into_values() {
        entries.push(StatusEntry {
            patterns: selectors,
            source_file: String::new(),
            source_kind: None,
            source_key: String::new(),
            source_hash: String::new(),
            source_ordinal: None,
            target_file: String::new(),
            target_subcase: String::new(),
            target_ordinal: None,
            status: CaseStatus::KnownFail,
            reason: format!("auto-updated {} known failure", suite.name()),
            note: String::new(),
            capabilities: Vec::new(),
            environments: Vec::new(),
        });
    }

    let updated = StatusSet { entries }.normalized();
    updated.save(&status_path)
}

/// Run one case on a worker thread and enforce its timeout.
fn run_case_with_timeout<S: ConformanceDriver + 'static>(
    suite: &S,
    case: &Case,
    timeout: Duration,
    abort_on_timeout: bool,
    show_diff: bool,
) -> CaseRunResult {
    // owned worker state
    let suite = suite.clone();
    let case = case.clone();
    let suite_name = suite.name().to_string();
    let case_name = case.name.clone();
    let thread_name = format!("{suite_name}::{case_name}");
    let (sender, receiver) = mpsc::channel();

    // worker thread
    thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let result = suite.run(&case, show_diff);
            let _ = sender.send(result);
        })
        .expect("failed to spawn case thread");

    // timeout outcome
    let outcome = match receiver.recv_timeout(timeout) {
        Ok(outcome) => CaseRunResult::Completed(outcome),
        Err(mpsc::RecvTimeoutError::Timeout) => CaseRunResult::TimedOut,
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            CaseRunResult::Completed(CaseOutcome::FailedRead)
        }
    };

    // hard abort
    if abort_on_timeout && matches!(outcome, CaseRunResult::TimedOut) {
        eprintln!("timeout in {suite_name}::{case_name} (aborting to avoid runaway threads)");
        std::process::exit(2);
    }

    outcome
}
