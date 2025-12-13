use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use rayon::ThreadPoolBuilder;
use rayon::prelude::*;

use destack_source::FileType;

use crate::harness::print::color;
use crate::harness::{TestOptions, load_expected_failures, save_expected_failures};

/// Per-test timeout in seconds.
const TEST_TIMEOUT_SECONDS: u64 = 1;

/// Result of running a single conformance test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOutcome {
    /// Test passed (actual result matched expected).
    Passed,
    /// Test failed (actual result did not match expected).
    Failed,
}

/// A discovered conformance test with metadata.
#[derive(Debug, Clone)]
pub struct Test {
    /// Test name (relative path within suite).
    pub name: String,
    /// File type for parsing.
    pub file_type: FileType,
    /// Whether the test is expected to have parse errors.
    pub expect_error: bool,
}

impl Test {
    /// Create a test that should pass (no parse errors expected).
    pub fn pass(name: impl Into<String>, file_type: FileType) -> Self {
        Self {
            name: name.into(),
            file_type,
            expect_error: false,
        }
    }

    /// Create a test that should fail (parse errors expected).
    pub fn fail(name: impl Into<String>, file_type: FileType) -> Self {
        Self {
            name: name.into(),
            file_type,
            expect_error: true,
        }
    }

    /// Infer file type from extension.
    pub fn file_type_from_name(name: &str) -> FileType {
        if name.ends_with(".tsx") {
            FileType::TypeScriptXml
        } else if name.ends_with(".ts") {
            FileType::TypeScript
        } else if name.ends_with(".jsx") {
            FileType::JavaScriptXml
        } else {
            FileType::JavaScript
        }
    }
}

/// A conformance test suite (e.g., test262, typescript).
pub trait ConformanceSuite: Send + Sync + Clone {
    /// Name of the suite (for display).
    fn name(&self) -> &str;

    /// Root directory of the suite.
    fn root(&self) -> &Path;

    /// Path to known-failures file (tests that fail but we want to fix).
    fn known_failures_path(&self) -> PathBuf;

    /// Path to expected-failures file (tests we consciously skip).
    /// These are tests we don't expect to pass due to intentional language differences.
    fn expected_failures_path(&self) -> PathBuf {
        // Default: same directory as known-failures, with -expected-failures.txt suffix
        let known = self.known_failures_path();
        let stem = known
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let name = stem.strip_suffix("-known-failures").unwrap_or(stem);
        known.with_file_name(format!("{name}-expected-failures.txt"))
    }

    /// Discover all tests in this suite.
    fn discover(&self) -> Vec<Test>;

    /// Run a single test, return whether it passed or failed.
    fn run(&self, test: &Test) -> TestOutcome;

    /// Instructions for downloading the suite.
    fn download_instructions(&self) -> String;

    /// Extract category from test name (default: first path segment).
    fn category_for_test(&self, test_name: &str) -> String {
        test_name.split('/').next().unwrap_or("unknown").to_string()
    }
}

/// Internal result including timeout state.
enum TestResult {
    Passed,
    Failed,
    TimedOut,
}

/// Run a single test with a timeout.
fn run_test_with_timeout<S: ConformanceSuite + 'static>(
    suite: &S,
    test: &Test,
    timeout: Duration,
) -> TestResult {
    let suite = suite.clone();
    let test = test.clone();
    let thread_name = format!("{}::{}", suite.name(), test.name);

    let (tx, rx) = mpsc::channel();

    thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let result = suite.run(&test);
            let _ = tx.send(result);
        })
        .expect("failed to spawn test thread");

    match rx.recv_timeout(timeout) {
        Ok(TestOutcome::Passed) => TestResult::Passed,
        Ok(TestOutcome::Failed) => TestResult::Failed,
        Err(mpsc::RecvTimeoutError::Timeout) => TestResult::TimedOut,
        Err(mpsc::RecvTimeoutError::Disconnected) => TestResult::Failed,
    }
}

/// Per-category statistics.
#[derive(Debug, Clone, Default)]
pub struct CategoryStats {
    pub passed: usize,
    pub failed: usize,
}

impl CategoryStats {
    pub fn total(&self) -> usize {
        self.passed + self.failed
    }

    pub fn pass_rate(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            100.0
        } else {
            self.passed as f64 / total as f64 * 100.0
        }
    }
}

/// Result of a conformance test run.
#[derive(Debug)]
pub struct ConformanceResult {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub timedout: usize,
    /// tests that failed unexpectedly (not in known-failures or skipped-failures)
    pub regressions: Vec<String>,
    /// tests that passed but were in known-failures (progress!)
    pub fixed: Vec<String>,
    /// tests in skipped-failures that now pass (remove from skipped!)
    pub unskipped: Vec<String>,
    /// tests that timed out (likely infinite loops)
    pub timeouts: Vec<String>,
    /// per-category breakdown (category name -> stats)
    pub categories: BTreeMap<String, CategoryStats>,
}

impl ConformanceResult {
    pub fn has_regressions(&self) -> bool {
        !self.regressions.is_empty()
    }

    /// Pass rate excludes skipped tests (they weren't run).
    pub fn pass_rate(&self) -> f64 {
        let run = self.passed + self.failed;
        if run == 0 {
            100.0
        } else {
            self.passed as f64 / run as f64 * 100.0
        }
    }

    /// Total tests run (excludes skipped).
    pub fn total_run(&self) -> usize {
        self.passed + self.failed + self.timedout
    }

    /// Total tests including skipped.
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped + self.timedout
    }

    pub fn has_timeouts(&self) -> bool {
        !self.timeouts.is_empty()
    }
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
    let all_tests = suite.discover();
    let tests: Vec<_> = if let Some(filter) = &options.filter {
        all_tests
            .into_iter()
            .filter(|t| t.name.contains(filter))
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
            println!("  {}", test.name);
        }
        return None; // list mode doesn't return results
    }

    // load known failures and skipped failures
    let known_failures_path = suite.known_failures_path();
    let known_failures = load_expected_failures(&known_failures_path);

    let expected_failures_path = suite.expected_failures_path();
    let skipped_failures = load_expected_failures(&expected_failures_path);

    // count how many tests are in skipped list (for display)
    let skipped_count = tests.iter().filter(|t| skipped_failures.contains(&t.name)).count();

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
    if skipped_count > 0 {
        println!(
            "  {} expected failures loaded from {}",
            color::dim(&skipped_count.to_string()),
            expected_failures_path.display()
        );
    }
    println!();

    // run tests in parallel
    let start = Instant::now();
    let timeout = Duration::from_secs(TEST_TIMEOUT_SECONDS);

    // atomic counters for progress reporting
    let progress_counter = AtomicUsize::new(0);
    let total_tests = tests.len();
    let verbose = options.verbose;

    // run tests in parallel and collect results
    let results: Vec<_> = if options.parallel() {
        let jobs = options.jobs.max(1);
        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build()
            .expect("failed to build rayon thread pool");

        thread_pool.install(|| {
            tests
                .par_iter()
                .map(|test| {
                    let outcome = run_test_with_timeout(suite, test, timeout);

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

                    (test.name.clone(), outcome)
                })
                .collect()
        })
    } else {
        tests
            .iter()
            .map(|test| {
                let outcome = run_test_with_timeout(suite, test, timeout);
                (test.name.clone(), outcome)
            })
            .collect()
    };

    // aggregate results
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut timedout = 0;
    let mut regressions = Vec::new();
    let mut fixed = Vec::new();
    let mut unskipped = Vec::new();
    let mut timeouts = Vec::new();
    let mut current_failures = HashSet::new();
    let mut categories: BTreeMap<String, CategoryStats> = BTreeMap::new();

    for (name, outcome) in results {
        let is_skipped = skipped_failures.contains(&name);
        let is_known_failure = known_failures.contains(&name);

        // extract category from test name (suite-specific)
        let category = suite.category_for_test(&name);
        let cat_stats = categories.entry(category).or_default();

        match outcome {
            TestResult::Passed => {
                if is_skipped {
                    // Skipped test now passes - report so we can remove from skipped list
                    unskipped.push(name);
                    skipped += 1;
                    // Don't count in category stats (it's skipped)
                } else {
                    passed += 1;
                    cat_stats.passed += 1;
                    if is_known_failure {
                        fixed.push(name);
                    }
                }
            }
            TestResult::Failed => {
                if is_skipped {
                    // Expected - skipped tests should fail
                    skipped += 1;
                    // Don't count in category stats (it's skipped)
                } else {
                    failed += 1;
                    cat_stats.failed += 1;
                    if !is_known_failure {
                        regressions.push(name.clone());
                    }
                    current_failures.insert(name);
                }
            }
            TestResult::TimedOut => {
                if is_skipped {
                    // Skipped test timed out - still counts as skipped
                    skipped += 1;
                } else {
                    timedout += 1;
                    cat_stats.failed += 1; // count timeouts as failures in category
                    timeouts.push(name.clone());
                    if !is_known_failure {
                        regressions.push(name.clone());
                    }
                    current_failures.insert(name);
                }
            }
        }
    }

    let duration = start.elapsed();

    // update known-failures if requested
    if update_known_failures {
        if let Err(err) = save_expected_failures(&known_failures_path, &current_failures) {
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
        skipped,
        timedout,
        regressions,
        fixed,
        unskipped,
        timeouts,
        categories,
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
pub fn print_summary(results: &[SuiteResult], baseline: Option<&ReadmeResults>) {
    if results.len() <= 1 {
        return; // no summary needed for single suite
    }

    let total_passed: usize = results.iter().map(|r| r.result.passed).sum();
    let total_failed: usize = results.iter().map(|r| r.result.failed).sum();
    let total_skipped: usize = results.iter().map(|r| r.result.skipped).sum();
    let total_timedout: usize = results.iter().map(|r| r.result.timedout).sum();
    let total_tests: usize = results.iter().map(|r| r.result.total_run()).sum();
    let total_duration: Duration = results.iter().map(|r| r.duration).sum();
    let total_regressions: usize = results.iter().map(|r| r.result.regressions.len()).sum();

    let overall_rate = if total_tests > 0 {
        total_passed as f64 / total_tests as f64 * 100.0
    } else {
        100.0
    };

    // compute old totals for delta
    let old_total_rate = baseline
        .map(|b| {
            let passed: usize = b.rows.iter().map(|r| r.passed).sum();
            let total: usize = b.rows.iter().map(|r| r.total).sum();
            if total > 0 {
                passed as f64 / total as f64 * 100.0
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    println!();
    println!(
        "{}",
        color::bold("═══════════════════════════════════════════════════════════════════════")
    );
    println!(
        "{}",
        color::bold("                          CONFORMANCE SUMMARY")
    );
    println!(
        "{}",
        color::bold("═══════════════════════════════════════════════════════════════════════")
    );
    println!();

    // header
    println!(
        "  {:10}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}  {:>10}",
        "Suite", "Passed", "Failed", "Skipped", "Total", "Rate", "Δ Rate"
    );
    println!("  {}", "─".repeat(72));

    // rows - pad values before coloring to maintain alignment
    for r in results {
        let rate = r.result.pass_rate();
        let rate_str = format!("{rate:>7.2}%");
        let rate_colored = if rate >= 90.0 {
            color::green(&rate_str)
        } else if rate >= 50.0 {
            color::yellow(&rate_str)
        } else {
            color::red(&rate_str)
        };

        // compute delta from baseline
        let rate_delta = baseline
            .and_then(|b| b.find(&r.name))
            .map(|old| rate - old.rate)
            .unwrap_or(0.0);
        let delta_str = format_rate_delta_inline(rate_delta);

        let name = format!("{:10}", r.name);
        let passed = format!("{:>8}", r.result.passed);
        let failed = format!("{:>8}", r.result.failed);
        let skipped = if r.result.skipped > 0 {
            format!("{:>8}", r.result.skipped)
        } else {
            format!("{:>8}", "-")
        };

        println!(
            "  {}  {}  {}  {}  {:>8}  {}  {}",
            color::cyan(&name),
            color::green(&passed),
            color::red(&failed),
            color::dim(&skipped),
            r.result.total_run(),
            rate_colored,
            delta_str
        );
    }

    // total row
    println!("  {}", "─".repeat(72));
    let total_rate_str = format!("{overall_rate:>7.2}%");
    let total_rate_colored = if overall_rate >= 90.0 {
        color::green(&total_rate_str)
    } else if overall_rate >= 50.0 {
        color::yellow(&total_rate_str)
    } else {
        color::red(&total_rate_str)
    };

    let total_rate_delta = if baseline.is_some() {
        overall_rate - old_total_rate
    } else {
        0.0
    };
    let total_delta_str = format_rate_delta_inline(total_rate_delta);

    let total_label = format!("{:10}", "TOTAL");
    let total_passed_str = format!("{total_passed:>8}");
    let total_failed_str = format!("{:>8}", total_failed + total_timedout);
    let total_skipped_str = if total_skipped > 0 {
        format!("{total_skipped:>8}")
    } else {
        format!("{:>8}", "-")
    };

    println!(
        "  {}  {}  {}  {}  {:>8}  {}  {}",
        color::bold(&total_label),
        color::green(&total_passed_str),
        color::red(&total_failed_str),
        color::dim(&total_skipped_str),
        total_tests,
        total_rate_colored,
        total_delta_str
    );
    println!();

    // timing
    println!(
        "  {}",
        color::dim(&format!(
            "completed in {:.2}s",
            total_duration.as_secs_f64()
        ))
    );

    // final verdict
    println!();
    let total_fixed: usize = results.iter().map(|r| r.result.fixed.len()).sum();
    if total_fixed > 0 {
        println!(
            "  {} {} tests fixed across all suites",
            color::green("FIXED:"),
            total_fixed
        );
    }
    if total_regressions > 0 {
        println!(
            "  {} {} tests regressed across all suites",
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

fn format_rate_delta_inline(delta: f64) -> String {
    if delta > 0.005 {
        color::green(&format!("{delta:>+8.2}%"))
    } else if delta < -0.005 {
        color::red(&format!("{delta:>+8.2}%"))
    } else {
        color::dim(&format!("{:>9}", "±0.00%"))
    }
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
        color::green(&format!("{pass_rate:.2}%"))
    } else if pass_rate >= 50.0 {
        color::yellow(&format!("{pass_rate:.2}%"))
    } else {
        color::red(&format!("{pass_rate:.2}%"))
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
        "  {}  {:>5}  ({:.2}%)",
        color::green("passed:"),
        result.passed,
        result.passed as f64 / total as f64 * 100.0
    );
    println!(
        "  {}  {:>5}  ({:.2}%)",
        color::red("failed:"),
        result.failed,
        result.failed as f64 / total as f64 * 100.0
    );
    if result.skipped > 0 {
        println!("  {} {:>5}", color::yellow("skipped:"), result.skipped);
    }
    if result.timedout > 0 {
        println!(
            "  {} {:>5}  ({:.2}%)",
            color::yellow("timeout:"),
            result.timedout,
            result.timedout as f64 / total as f64 * 100.0
        );
    }
    println!(
        "  {}",
        color::dim(&format!("finished in {:.2}s", duration.as_secs_f64()))
    );

    // category breakdown (if more than one category)
    if result.categories.len() > 1 {
        // compute column widths
        let max_name_len = result
            .categories
            .keys()
            .map(|k| k.len())
            .max()
            .unwrap_or(10);
        let max_passed = result
            .categories
            .values()
            .map(|s| s.passed)
            .max()
            .unwrap_or(1);
        let max_total = result
            .categories
            .values()
            .map(|s| s.total())
            .max()
            .unwrap_or(1);
        let passed_width = max_passed.to_string().len();
        let total_width = max_total.to_string().len();

        println!();
        println!("  {}", color::bold("by category:"));
        for (category, stats) in &result.categories {
            let rate = stats.pass_rate();
            let rate_str = format!("{rate:>6.2}%");
            let rate_colored = if rate >= 90.0 {
                color::green(&rate_str)
            } else if rate >= 50.0 {
                color::yellow(&rate_str)
            } else {
                color::red(&rate_str)
            };
            println!(
                "    {:<name_width$}  {:>passed_width$} / {:>total_width$}  {}",
                category,
                stats.passed,
                stats.total(),
                rate_colored,
                name_width = max_name_len,
                passed_width = passed_width,
                total_width = total_width
            );
        }
    }
    println!();

    // timeouts (separate from regressions for visibility)
    if result.has_timeouts() {
        println!(
            "{} ({} tests exceeded {}s timeout, likely infinite loops):",
            color::yellow("TIMEOUTS"),
            result.timeouts.len(),
            TEST_TIMEOUT_SECONDS
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

    // fixed tests (were in known-failures, now pass)
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

    // unskipped tests (were in skipped-failures, now pass)
    if !result.unskipped.is_empty() {
        println!(
            "{} ({} skipped tests now passing, remove from skipped-failures.txt):",
            color::green("UNSKIPPED"),
            result.unskipped.len()
        );
        let show_count = result.unskipped.len().min(20);
        for test in &result.unskipped[..show_count] {
            println!("  {test}");
        }
        if result.unskipped.len() > show_count {
            println!(
                "  {} ... and {} more",
                color::dim(""),
                result.unskipped.len() - show_count
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
    } else if !result.fixed.is_empty() || !result.unskipped.is_empty() {
        let mut parts = Vec::new();
        if !result.fixed.is_empty() {
            parts.push(format!("{} fixed", result.fixed.len()));
        }
        if !result.unskipped.is_empty() {
            parts.push(format!("{} unskipped", result.unskipped.len()));
        }
        println!(
            "test result: {}. {} tests newly passing",
            color::green("ok"),
            parts.join(", ")
        );
    } else {
        println!("test result: {}.", color::green("ok"));
    }
}

// ============================================================================
// README.md auto-update functionality
// ============================================================================

/// Row data for a suite in the results table.
#[derive(Debug, Clone, PartialEq)]
struct ReadmeRow {
    name: String,
    passed: usize,
    failed: usize,
    skipped: usize,
    total: usize,
    rate: f64,
}

impl ReadmeRow {
    fn from_suite_result(result: &SuiteResult) -> Self {
        Self {
            name: result.name.clone(),
            passed: result.result.passed,
            failed: result.result.failed + result.result.timedout,
            skipped: result.result.skipped,
            total: result.result.total_run(),
            rate: result.result.pass_rate(),
        }
    }

    /// Format a single row with proper spacing.
    fn format(&self) -> String {
        if self.skipped > 0 {
            format!(
                "| {:<8} | {:>5}  | {:>5}  | {:>5}  | {:>5} | {:>6.2}% |",
                self.name, self.passed, self.failed, self.skipped, self.total, self.rate
            )
        } else {
            format!(
                "| {:<8} | {:>5}  | {:>5}  | {:>5}  | {:>5} | {:>6.2}% |",
                self.name, self.passed, self.failed, "-", self.total, self.rate
            )
        }
    }
}

/// Parsed results from README.md.
#[derive(Debug, Clone)]
pub struct ReadmeResults {
    rows: Vec<ReadmeRow>,
}

impl ReadmeResults {
    /// Parse a row from the table (returns None for separator/header rows).
    /// Handles both old format (5 columns) and new format (6 columns with skipped).
    fn parse_row(line: &str) -> Option<ReadmeRow> {
        let line = line.trim();
        if !line.starts_with('|') || !line.ends_with('|') {
            return None;
        }

        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();

        // Handle both formats:
        // Old: ["", "name", "passed", "failed", "total", "rate", ""] - 7 parts
        // New: ["", "name", "passed", "failed", "skipped", "total", "rate", ""] - 8 parts
        let (name, passed, failed, skipped, total, rate) = match parts.len() {
            7 => {
                // Old format without skipped column
                let name = parts[1].to_lowercase();
                let passed: usize = parts[2].parse().ok()?;
                let failed: usize = parts[3].parse().ok()?;
                let total: usize = parts[4].parse().ok()?;
                let rate: f64 = parts[5].trim_end_matches('%').parse().ok()?;
                (name, passed, failed, 0, total, rate)
            }
            8 => {
                // New format with skipped column
                let name = parts[1].to_lowercase();
                let passed: usize = parts[2].parse().ok()?;
                let failed: usize = parts[3].parse().ok()?;
                let skipped: usize = parts[4].parse().unwrap_or(0); // "-" parses as 0
                let total: usize = parts[5].parse().ok()?;
                let rate: f64 = parts[6].trim_end_matches('%').parse().ok()?;
                (name, passed, failed, skipped, total, rate)
            }
            _ => return None,
        };

        // skip header row and separator
        if name.is_empty()
            || name == "suite"
            || name.starts_with(':')
            || name.starts_with('-')
            || name == "total"
        {
            return None;
        }

        Some(ReadmeRow {
            name,
            passed,
            failed,
            skipped,
            total,
            rate,
        })
    }

    /// Parse results from README content.
    fn parse(content: &str) -> Option<Self> {
        let begin_marker = "<!-- begin:summary-results -->";
        let end_marker = "<!-- end:summary-results -->";

        let begin_idx = content.find(begin_marker)?;
        let end_idx = content.find(end_marker)?;
        let section = &content[begin_idx + begin_marker.len()..end_idx];

        let rows: Vec<ReadmeRow> = section.lines().filter_map(Self::parse_row).collect();

        Some(Self { rows })
    }

    /// Find a row by suite name.
    fn find(&self, name: &str) -> Option<&ReadmeRow> {
        self.rows.iter().find(|r| r.name == name.to_lowercase())
    }
}

/// Load the baseline results from README.md.
pub fn load_readme_baseline() -> Option<ReadmeResults> {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("conformance")
        .join("README.md");

    let content = std::fs::read_to_string(&readme_path).ok()?;
    ReadmeResults::parse(&content)
}

/// Delta between old and new results.
#[derive(Debug)]
struct ResultDelta {
    name: String,
    passed_delta: i64,
    failed_delta: i64,
    total_delta: i64,
    rate_delta: f64,
}

impl ResultDelta {
    fn new(name: &str, old: Option<&ReadmeRow>, new: &ReadmeRow) -> Self {
        match old {
            Some(old) => Self {
                name: name.to_string(),
                passed_delta: new.passed as i64 - old.passed as i64,
                failed_delta: new.failed as i64 - old.failed as i64,
                total_delta: new.total as i64 - old.total as i64,
                rate_delta: new.rate - old.rate,
            },
            None => Self {
                name: name.to_string(),
                passed_delta: new.passed as i64,
                failed_delta: new.failed as i64,
                total_delta: new.total as i64,
                rate_delta: new.rate,
            },
        }
    }

    fn has_changes(&self) -> bool {
        self.passed_delta != 0 || self.failed_delta != 0 || self.total_delta != 0
    }
}

/// Replace content between markers in a string.
/// Returns the new content, or None if markers not found.
fn replace_section(content: &str, section_name: &str, new_section: &str) -> Option<String> {
    let begin_marker = format!("<!-- begin:{section_name} -->");
    let end_marker = format!("<!-- end:{section_name} -->");

    let begin_idx = content.find(&begin_marker)?;
    let end_idx = content.find(&end_marker)?;

    Some(format!(
        "{}\n{}\n{}{}",
        &content[..begin_idx + begin_marker.len()],
        new_section,
        end_marker,
        &content[end_idx + end_marker.len()..]
    ))
}

/// Format a category breakdown table for a suite.
fn format_category_section(categories: &BTreeMap<String, CategoryStats>) -> String {
    let total_passed: usize = categories.values().map(|s| s.passed).sum();
    let total_failed: usize = categories.values().map(|s| s.failed).sum();
    let total_total: usize = categories.values().map(|s| s.total()).sum();
    let total_rate = if total_total > 0 {
        total_passed as f64 / total_total as f64 * 100.0
    } else {
        0.0
    };

    let mut lines = Vec::new();
    lines.push("| Category             | Passed | Failed | Total |  Rate   |".to_string());
    lines.push("|:---------------------|-------:|-------:|------:|--------:|".to_string());

    for (category, stats) in categories {
        lines.push(format!(
            "| {:<20} | {:>5}  | {:>5}  | {:>5} | {:>6.2}% |",
            category,
            stats.passed,
            stats.failed,
            stats.total(),
            stats.pass_rate()
        ));
    }

    lines.push("|----------------------|--------|--------|-------|---------|".to_string());
    lines.push(format!(
        "| {:<20} | {:>5}  | {:>5}  | {:>5} | {:>6.2}% |",
        "total", total_passed, total_failed, total_total, total_rate
    ));

    lines.join("\n")
}

/// Format the complete results section.
fn format_results_section(rows: &[ReadmeRow]) -> String {
    // compute totals
    let total_passed: usize = rows.iter().map(|r| r.passed).sum();
    let total_failed: usize = rows.iter().map(|r| r.failed).sum();
    let total_skipped: usize = rows.iter().map(|r| r.skipped).sum();
    let total_total: usize = rows.iter().map(|r| r.total).sum();
    let total_rate = if total_total > 0 {
        total_passed as f64 / total_total as f64 * 100.0
    } else {
        0.0
    };

    let mut lines = Vec::new();
    lines.push("| Suite    | Passed | Failed | Skipped | Total |  Rate   |".to_string());
    lines.push("|:---------|-------:|-------:|--------:|------:|--------:|".to_string());

    for row in rows {
        lines.push(row.format());
    }

    lines.push("|----------|--------|--------|---------|-------|---------|".to_string());
    let skipped_str = if total_skipped > 0 {
        format!("{total_skipped:>5}")
    } else {
        "-".to_string()
    };
    lines.push(format!(
        "| {:<8} | {:>5}  | {:>5}  | {:>6}  | {:>5} | {:>6.2}% |",
        "total", total_passed, total_failed, skipped_str, total_total, total_rate
    ));
    lines.push(String::new());
    lines.push(format!("Total Blended Pass Rate: **{total_rate:.2}%**"));

    lines.join("\n")
}

/// Update the README.md with new results.
///
/// # Arguments
/// * `results` - The suite results to update
/// * `is_partial` - If true, only update rows for suites that were run
///
/// Returns true if changes were made.
pub fn update_readme(results: &[SuiteResult], is_partial: bool) -> bool {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("conformance")
        .join("README.md");

    let content = match std::fs::read_to_string(&readme_path) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("{}: failed to read README.md: {err}", color::red("error"));
            return false;
        }
    };

    // parse existing results
    let old_results = ReadmeResults::parse(&content);

    // build new rows
    let new_rows: Vec<ReadmeRow> = results.iter().map(ReadmeRow::from_suite_result).collect();

    // merge with existing if partial update
    let mut final_rows: Vec<ReadmeRow> = if is_partial {
        let Some(old) = &old_results else {
            eprintln!(
                "{}: cannot do partial update - no existing results in README.md",
                color::yellow("warning")
            );
            return false;
        };

        // collect all suite names from both old and new
        let mut all_names: Vec<&str> = old.rows.iter().map(|r| r.name.as_str()).collect();
        for row in &new_rows {
            if !all_names.contains(&row.name.as_str()) {
                all_names.push(&row.name);
            }
        }

        // use new row if we have it, otherwise keep old
        all_names
            .into_iter()
            .filter_map(|name| {
                if let Some(new_row) = new_rows.iter().find(|r| r.name == name) {
                    Some(new_row.clone())
                } else {
                    old.find(name).cloned()
                }
            })
            .collect()
    } else {
        new_rows.clone()
    };

    // sort alphabetically
    final_rows.sort_by(|a, b| a.name.cmp(&b.name));

    // compute deltas for reporting
    let deltas: Vec<ResultDelta> = new_rows
        .iter()
        .map(|new| {
            let old_row = old_results.as_ref().and_then(|o| o.find(&new.name));
            ResultDelta::new(&new.name, old_row, new)
        })
        .collect();

    let any_changes = deltas.iter().any(|d| d.has_changes());

    // format and replace summary section
    let summary_section = format_results_section(&final_rows);
    let Some(mut new_content) = replace_section(&content, "summary-results", &summary_section)
    else {
        eprintln!(
            "{}: README.md missing summary-results markers",
            color::red("error")
        );
        return false;
    };

    // update per-suite category sections (only for suites that were run)
    for suite_result in results {
        if suite_result.result.categories.len() > 1 {
            let section_name = format!("{}-results", suite_result.name);
            let category_section = format_category_section(&suite_result.result.categories);
            if let Some(updated) = replace_section(&new_content, &section_name, &category_section) {
                new_content = updated;
            }
            // if section doesn't exist, that's fine, just skip it
        }
    }

    // check if content actually changed
    if new_content == content {
        // compute totals for display
        let total_passed: usize = final_rows.iter().map(|r| r.passed).sum();
        let total_total: usize = final_rows.iter().map(|r| r.total).sum();
        let total_rate = if total_total > 0 {
            total_passed as f64 / total_total as f64 * 100.0
        } else {
            0.0
        };

        println!();
        println!(
            "  {} README.md unchanged — {:.2}% ({}/{})",
            color::dim("(no changes)"),
            total_rate,
            total_passed,
            total_total
        );
        return false;
    }

    // write updated content
    if let Err(err) = std::fs::write(&readme_path, &new_content) {
        eprintln!("{}: failed to write README.md: {err}", color::red("error"));
        return false;
    }

    // report changes
    println!();
    if any_changes {
        println!("  {} README.md updated:", color::green("UPDATED"));
        for delta in &deltas {
            if delta.has_changes() {
                let passed_str = format_delta(delta.passed_delta);
                let failed_str = format_delta(delta.failed_delta);
                let rate_str = format_rate_delta(delta.rate_delta);
                println!(
                    "    {}: passed {} | failed {} | rate {}",
                    color::cyan(&delta.name),
                    passed_str,
                    failed_str,
                    rate_str
                );
            }
        }
    } else {
        println!(
            "  {} README.md updated (formatting only)",
            color::dim("UPDATED")
        );
    }

    true
}

fn format_delta(delta: i64) -> String {
    if delta > 0 {
        color::green(&format!("+{delta}"))
    } else if delta < 0 {
        color::red(&format!("{delta}"))
    } else {
        color::dim("±0")
    }
}

fn format_rate_delta(delta: f64) -> String {
    if delta > 0.005 {
        color::green(&format!("+{delta:.2}%"))
    } else if delta < -0.005 {
        color::red(&format!("{delta:.2}%"))
    } else {
        color::dim("±0.00%")
    }
}
