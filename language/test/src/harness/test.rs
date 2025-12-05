//! Test case, result, and runner types.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use destack_source::DiagnosticSeverity;

use super::{filter_tests, print_failures, print_result, print_summary, print_test_list, TestOptions};

/// Result of running a single test.
#[derive(Debug, Clone)]
pub enum TestResult {
    /// Test passed.
    Passed,
    /// Test failed with a message.
    Failed { message: String },
    /// Test was skipped.
    Skipped { reason: String },
}

impl TestResult {
    /// Whether the test passed.
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }

    /// Whether the test failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Whether the test was skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }
}

/// A single test case.
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Name of the test (usually the file stem).
    pub name: String,
    /// Path to the test file or directory.
    pub path: PathBuf,
    /// Category/group of the test.
    pub category: String,
    /// Minimum severity that causes test failure (default: Warning).
    pub min_fail_severity: DiagnosticSeverity,
    /// Whether this test is marked as skipped (e.g., prefixed with `_`).
    pub skipped: bool,
}

impl TestCase {
    /// Create a new test case with default settings.
    pub fn new(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            category: category.into(),
            min_fail_severity: DiagnosticSeverity::Warning,
            skipped: false,
        }
    }

    /// Mark this test as skipped.
    pub fn with_skipped(mut self, skipped: bool) -> Self {
        self.skipped = skipped;
        self
    }

    /// Set the minimum severity that causes test failure.
    pub fn with_min_fail_severity(mut self, severity: DiagnosticSeverity) -> Self {
        self.min_fail_severity = severity;
        self
    }

    /// Full name including category.
    pub fn full_name(&self) -> String {
        format!("{}::{}", self.category, self.name)
    }
}

/// Summary of test run results.
#[derive(Debug, Default)]
pub struct TestSummary {
    passed: AtomicUsize,
    failed: AtomicUsize,
    skipped: AtomicUsize,
}

impl TestSummary {
    /// Create a new empty summary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a test result.
    pub fn record(&self, result: &TestResult) {
        match result {
            TestResult::Passed => self.passed.fetch_add(1, Ordering::Relaxed),
            TestResult::Failed { .. } => self.failed.fetch_add(1, Ordering::Relaxed),
            TestResult::Skipped { .. } => self.skipped.fetch_add(1, Ordering::Relaxed),
        };
    }

    /// Number of passed tests.
    pub fn passed(&self) -> usize {
        self.passed.load(Ordering::Relaxed)
    }

    /// Number of failed tests.
    pub fn failed(&self) -> usize {
        self.failed.load(Ordering::Relaxed)
    }

    /// Number of skipped tests.
    pub fn skipped(&self) -> usize {
        self.skipped.load(Ordering::Relaxed)
    }

    /// Total number of tests.
    pub fn total(&self) -> usize {
        self.passed() + self.failed() + self.skipped()
    }

    /// Whether all tests passed.
    pub fn all_passed(&self) -> bool {
        self.failed() == 0
    }
}

/// Run tests and return exit code.
pub fn run_tests<F>(tests: Vec<TestCase>, options: &TestOptions, runner: F) -> ExitCode
where
    F: Fn(&TestCase) -> TestResult + Send + Sync,
{
    let filtered = filter_tests(tests, options.filter.as_deref());

    if filtered.is_empty() {
        println!("no tests to run");
        return ExitCode::SUCCESS;
    }

    // list mode: just print tests and exit
    if options.list {
        print_test_list(&filtered);
        return ExitCode::SUCCESS;
    }

    println!();
    println!("running {} tests", filtered.len());

    let summary = TestSummary::new();
    let start = Instant::now();
    let mut results: Vec<(TestCase, TestResult)> = Vec::new();

    // run tests sequentially (parallel can be added later with rayon)
    for test in &filtered {
        let test_start = Instant::now();

        // handle pre-skipped tests
        let result = if test.skipped {
            TestResult::Skipped {
                reason: "marked as skipped".to_string(),
            }
        } else {
            runner(test)
        };

        let duration = test_start.elapsed();
        summary.record(&result);
        print_result(test, &result, duration, options.verbose);
        results.push((test.clone(), result));
    }

    let total_duration = start.elapsed();

    // print failures
    print_failures(&results);

    // print summary
    print_summary(&summary, total_duration);

    if summary.all_passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
