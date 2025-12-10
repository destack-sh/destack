use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::{fs, thread};

use rayon::prelude::*;

use crate::harness::TestOptions;
use crate::harness::print::color;

/// Per-test timeout in seconds.
const TEST_TIMEOUT_SECS: u64 = 5;

/// A conformance test suite (e.g., test262, typescript).
pub trait ConformanceSuite: Send + Sync + Clone {
    /// Name of the suite (for display).
    fn name(&self) -> &str;

    /// Root directory of the suite.
    fn root(&self) -> &Path;

    /// Path to known-failures file.
    fn known_failures_path(&self) -> PathBuf;

    /// Discover all test names in this suite.
    fn discover_tests(&self) -> Vec<String>;

    /// Run a single test, return true if passed.
    fn run_test(&self, name: &str) -> bool;

    /// Instructions for downloading the suite.
    fn download_instructions(&self) -> String;
}

/// Result of running a single test with timeout.
enum TestOutcome {
    Passed,
    Failed,
    TimedOut,
}

/// Run a single test with a timeout.
fn run_test_with_timeout<S: ConformanceSuite + 'static>(
    suite: &S,
    name: &str,
    timeout: Duration,
) -> TestOutcome {
    let suite = suite.clone();
    let name = name.to_string();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = suite.run_test(&name);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(true) => TestOutcome::Passed,
        Ok(false) => TestOutcome::Failed,
        Err(mpsc::RecvTimeoutError::Timeout) => TestOutcome::TimedOut,
        Err(mpsc::RecvTimeoutError::Disconnected) => TestOutcome::Failed,
    }
}

/// Result of a conformance test run.
#[derive(Debug)]
pub struct ConformanceResult {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub timedout: usize,
    /// tests that failed unexpectedly (not in known-failures)
    pub regressions: Vec<String>,
    /// tests that passed but were in known-failures (progress!)
    pub fixed: Vec<String>,
    /// tests that timed out (likely infinite loops)
    pub timeouts: Vec<String>,
}

impl ConformanceResult {
    pub fn has_regressions(&self) -> bool {
        !self.regressions.is_empty()
    }

    pub fn pass_rate(&self) -> f64 {
        let total = self.passed + self.failed + self.skipped;
        if total == 0 {
            100.0
        } else {
            self.passed as f64 / total as f64 * 100.0
        }
    }

    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped + self.timedout
    }

    pub fn has_timeouts(&self) -> bool {
        !self.timeouts.is_empty()
    }
}

/// Load known failures from a file (one test name per line).
fn load_known_failures(path: &Path) -> HashSet<String> {
    match fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(String::from)
            .collect(),
        Err(_) => HashSet::new(),
    }
}

/// Save known failures to a file.
fn save_known_failures(path: &Path, failures: &HashSet<String>) -> std::io::Result<()> {
    let mut sorted: Vec<_> = failures.iter().cloned().collect();
    sorted.sort();
    let content = sorted.join("\n") + "\n";
    fs::write(path, content)
}

/// Result of running a suite, for aggregation.
#[derive(Debug)]
pub struct SuiteResult {
    pub name: String,
    pub result: ConformanceResult,
    pub duration: Duration,
}

/// Run a conformance suite with the standard test harness.
/// Returns Some(SuiteResult) on success, None if suite not found.
pub fn run_conformance_suite<S: ConformanceSuite + 'static>(
    suite: &S,
    options: &TestOptions,
    update_known_failures: bool,
) -> Option<SuiteResult> {
    // check suite exists
    if !suite.root().exists() {
        eprintln!(
            "{}: {} not found at {}",
            color::red("error"),
            suite.name(),
            suite.root().display()
        );
        eprintln!();
        eprintln!("{}", suite.download_instructions());
        return None;
    }

    // discover tests
    let all_tests = suite.discover_tests();
    let tests: Vec<_> = if let Some(filter) = &options.filter {
        all_tests
            .into_iter()
            .filter(|t| t.contains(filter))
            .collect()
    } else {
        all_tests
    };

    // list mode
    if options.list {
        println!(
            "{} tests in {}:",
            color::bold(&tests.len().to_string()),
            color::cyan(suite.name())
        );
        for test in &tests {
            println!("  {test}");
        }
        return None; // list mode doesn't return results
    }

    // load known failures
    let known_failures_path = suite.known_failures_path();
    let known_failures = load_known_failures(&known_failures_path);

    println!();
    println!(
        "running {} {} tests",
        color::bold(&tests.len().to_string()),
        color::cyan(suite.name())
    );
    if !known_failures.is_empty() {
        println!(
            "  {} known failures loaded from {}",
            color::yellow(&known_failures.len().to_string()),
            known_failures_path.display()
        );
    }
    println!();

    // run tests in parallel
    let start = Instant::now();
    let timeout = Duration::from_secs(TEST_TIMEOUT_SECS);

    // atomic counters for progress reporting
    let progress_counter = AtomicUsize::new(0);
    let total_tests = tests.len();
    let verbose = options.verbose;

    // run tests in parallel and collect results
    let results: Vec<_> = tests
        .par_iter()
        .map(|name| {
            let outcome = run_test_with_timeout(suite, name, timeout);

            // progress reporting (approximate due to parallelism)
            if verbose {
                let completed = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;
                if completed.is_multiple_of(100) {
                    eprintln!(
                        "  progress: ~{}/{} ({:.0}%)",
                        completed,
                        total_tests,
                        completed as f64 / total_tests as f64 * 100.0
                    );
                }
            }

            (name.clone(), outcome)
        })
        .collect();

    // aggregate results 
    let mut passed = 0;
    let mut failed = 0;
    let mut timedout = 0;
    let mut regressions = Vec::new();
    let mut fixed = Vec::new();
    let mut timeouts = Vec::new();
    let mut current_failures = HashSet::new();

    for (name, outcome) in results {
        match outcome {
            TestOutcome::Passed => {
                passed += 1;
                if known_failures.contains(&name) {
                    fixed.push(name);
                }
            }
            TestOutcome::Failed => {
                failed += 1;
                if !known_failures.contains(&name) {
                    regressions.push(name.clone());
                }
                current_failures.insert(name);
            }
            TestOutcome::TimedOut => {
                timedout += 1;
                timeouts.push(name.clone());
                if !known_failures.contains(&name) {
                    regressions.push(name.clone());
                }
                current_failures.insert(name);
            }
        }
    }

    let duration = start.elapsed();

    // update known-failures if requested
    if update_known_failures {
        if let Err(err) = save_known_failures(&known_failures_path, &current_failures) {
            eprintln!(
                "{}: failed to update known-failures: {err}",
                color::red("error")
            );
        } else {
            println!(
                "  {} updated with {} failures",
                known_failures_path.display(),
                current_failures.len()
            );
        }
    }

    let result = ConformanceResult {
        passed,
        failed,
        skipped: 0,
        timedout,
        regressions,
        fixed,
        timeouts,
    };

    // print results
    print_conformance_result(suite.name(), &result, duration);

    Some(SuiteResult {
        name: suite.name().to_string(),
        result,
        duration,
    })
}

/// Print a summary table of all suite results.
pub fn print_summary(results: &[SuiteResult]) {
    if results.len() <= 1 {
        return; // no summary needed for single suite
    }

    let total_passed: usize = results.iter().map(|r| r.result.passed).sum();
    let total_failed: usize = results.iter().map(|r| r.result.failed).sum();
    let total_timedout: usize = results.iter().map(|r| r.result.timedout).sum();
    let total_tests: usize = results.iter().map(|r| r.result.total()).sum();
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    let total_regressions: usize = results.iter().map(|r| r.result.regressions.len()).sum();

    let overall_rate = if total_tests > 0 {
        total_passed as f64 / total_tests as f64 * 100.0
    } else {
        100.0
    };

    println!();
    println!("{}", color::bold("═══════════════════════════════════════════════════════════════"));
    println!("{}", color::bold("                      CONFORMANCE SUMMARY"));
    println!("{}", color::bold("═══════════════════════════════════════════════════════════════"));
    println!();

    // header
    println!(
        "  {:10}  {:>8}  {:>8}  {:>8}  {:>8}",
        "Suite", "Passed", "Failed", "Total", "Rate"
    );
    println!("  {}", "─".repeat(52));

    // rows - pad values before coloring to maintain alignment
    for r in results {
        let rate = r.result.pass_rate();
        let rate_str = format!("{:>6.1}%", rate);
        let rate_colored = if rate >= 90.0 {
            color::green(&rate_str)
        } else if rate >= 50.0 {
            color::yellow(&rate_str)
        } else {
            color::red(&rate_str)
        };

        let name = format!("{:10}", r.name);
        let passed = format!("{:>8}", r.result.passed);
        let failed = format!("{:>8}", r.result.failed);

        println!(
            "  {}  {}  {}  {:>8}  {}",
            color::cyan(&name),
            color::green(&passed),
            color::red(&failed),
            r.result.total(),
            rate_colored
        );
    }

    // total row
    println!("  {}", "─".repeat(52));
    let total_rate_str = format!("{:>6.1}%", overall_rate);
    let total_rate_colored = if overall_rate >= 90.0 {
        color::green(&total_rate_str)
    } else if overall_rate >= 50.0 {
        color::yellow(&total_rate_str)
    } else {
        color::red(&total_rate_str)
    };

    let total_label = format!("{:10}", "TOTAL");
    let total_passed_str = format!("{:>8}", total_passed);
    let total_failed_str = format!("{:>8}", total_failed + total_timedout);

    println!(
        "  {}  {}  {}  {:>8}  {}",
        color::bold(&total_label),
        color::green(&total_passed_str),
        color::red(&total_failed_str),
        total_tests,
        total_rate_colored
    );
    println!();

    // timing
    println!(
        "  {}",
        color::dim(&format!("completed in {:.2}s", total_duration.as_secs_f64()))
    );

    // final verdict
    println!();
    if total_regressions > 0 {
        println!(
            "  {} {} regressions across all suites",
            color::red("FAILED:"),
            total_regressions
        );
    } else {
        println!(
            "  {} all {} tests accounted for",
            color::green("PASSED:"),
            total_tests
        );
    }
    println!();
}

/// Print conformance test results with colors.
fn print_conformance_result(
    suite_name: &str,
    result: &ConformanceResult,
    duration: std::time::Duration,
) {
    let total = result.total();
    let pass_rate = result.pass_rate();

    // header with pass rate
    let rate_color = if pass_rate >= 90.0 {
        color::green(&format!("{pass_rate:.1}%"))
    } else if pass_rate >= 50.0 {
        color::yellow(&format!("{pass_rate:.1}%"))
    } else {
        color::red(&format!("{pass_rate:.1}%"))
    };

    println!();
    println!(
        "{} conformance: {} ({}/{} tests passing)",
        color::bold(suite_name),
        rate_color,
        result.passed,
        total
    );
    println!();

    // breakdown
    println!(
        "  {}  {:>5}  ({:.1}%)",
        color::green("passed:"),
        result.passed,
        result.passed as f64 / total as f64 * 100.0
    );
    println!(
        "  {}  {:>5}  ({:.1}%)",
        color::red("failed:"),
        result.failed,
        result.failed as f64 / total as f64 * 100.0
    );
    if result.skipped > 0 {
        println!("  {} {:>5}", color::yellow("skipped:"), result.skipped);
    }
    if result.timedout > 0 {
        println!(
            "  {} {:>5}  ({:.1}%)",
            color::yellow("timeout:"),
            result.timedout,
            result.timedout as f64 / total as f64 * 100.0
        );
    }
    println!(
        "  {}",
        color::dim(&format!("finished in {:.2}s", duration.as_secs_f64()))
    );
    println!();

    // timeouts (separate from regressions for visibility)
    if result.has_timeouts() {
        println!(
            "{} ({} tests exceeded {}s timeout, likely infinite loops):",
            color::yellow("TIMEOUTS"),
            result.timeouts.len(),
            TEST_TIMEOUT_SECS
        );
        let show_count = result.timeouts.len().min(20);
        for test in &result.timeouts[..show_count] {
            println!("  {test}");
        }
        if result.timeouts.len() > show_count {
            println!(
                "  {} ... and {} more",
                color::dim(""),
                result.timeouts.len() - show_count
            );
        }
        println!();
    }

    // regressions
    if !result.regressions.is_empty() {
        println!(
            "{} ({} tests failed unexpectedly):",
            color::red("REGRESSIONS"),
            result.regressions.len()
        );
        let show_count = result.regressions.len().min(20);
        for test in &result.regressions[..show_count] {
            println!("  {test}");
        }
        if result.regressions.len() > show_count {
            println!(
                "  {} ... and {} more",
                color::dim(""),
                result.regressions.len() - show_count
            );
        }
        println!();
    }

    // fixed tests
    if !result.fixed.is_empty() {
        println!(
            "{} ({} tests now passing, remove from known-failures.txt):",
            color::green("FIXED"),
            result.fixed.len()
        );
        let show_count = result.fixed.len().min(20);
        for test in &result.fixed[..show_count] {
            println!("  {test}");
        }
        if result.fixed.len() > show_count {
            println!(
                "  {} ... and {} more",
                color::dim(""),
                result.fixed.len() - show_count
            );
        }
        println!();
    }

    // final status
    if result.has_regressions() {
        println!(
            "test result: {}. {} regressions detected",
            color::red("FAILED"),
            result.regressions.len()
        );
    } else if !result.fixed.is_empty() {
        println!(
            "test result: {}. {} tests newly passing",
            color::green("ok"),
            result.fixed.len()
        );
    } else {
        println!("test result: {}.", color::green("ok"));
    }
}
