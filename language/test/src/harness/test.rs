use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_source::DiagnosticSeverity;

use super::{RunContext, Runner, TestOptions};

/// Result of running a single test.
#[derive(Debug, Clone)]
pub enum TestResult {
    /// Test passed.
    Passed,
    /// Test failed with a message.
    Failed { message: String },
    /// Test was skipped.
    Skipped { reason: String },
    /// Suite of tests (for conformance tracking).
    Suite {
        passed: usize,
        failed: usize,
        skipped: usize,
    },
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

    /// Whether this is a suite result.
    pub fn is_suite(&self) -> bool {
        matches!(self, Self::Suite { .. })
    }

    /// Get pass rate as a percentage (for Suite results).
    pub fn pass_rate(&self) -> Option<f64> {
        match self {
            Self::Suite {
                passed,
                failed,
                skipped,
            } => {
                let total = passed + failed + skipped;
                if total == 0 {
                    Some(100.0)
                } else {
                    Some(*passed as f64 / total as f64 * 100.0)
                }
            }
            _ => None,
        }
    }
}

/// Kind of test case (file-based or directory-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestKind {
    /// Test based on a single file.
    File,
    /// Test based on a directory.
    Directory,
}

/// A single test case.
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Name of the test (usually the file stem or directory name).
    pub name: String,
    /// Path to the test file or directory.
    pub path: PathBuf,
    /// Category/group of the test.
    pub category: String,
    /// Kind of test (file or directory).
    pub kind: TestKind,
    /// Minimum severity that causes test failure (default: Warning).
    pub min_fail_severity: DiagnosticSeverity,
    /// Whether this test is marked as skipped (e.g., prefixed with `_`).
    pub is_skipped: bool,
}

impl TestCase {
    /// Create a new file-based test case with default settings.
    pub fn file(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            category: category.into(),
            kind: TestKind::File,
            min_fail_severity: DiagnosticSeverity::Warning,
            is_skipped: false,
        }
    }

    /// Create a new directory-based test case with default settings.
    pub fn directory(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            category: category.into(),
            kind: TestKind::Directory,
            min_fail_severity: DiagnosticSeverity::Warning,
            is_skipped: false,
        }
    }

    /// Mark this test as skipped.
    pub fn with_skipped(mut self, skipped: bool) -> Self {
        self.is_skipped = skipped;
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
            TestResult::Passed => {
                self.passed.fetch_add(1, Ordering::Relaxed);
            }
            TestResult::Failed { .. } => {
                self.failed.fetch_add(1, Ordering::Relaxed);
            }
            TestResult::Skipped { .. } => {
                self.skipped.fetch_add(1, Ordering::Relaxed);
            }
            TestResult::Suite {
                passed,
                failed,
                skipped,
            } => {
                // suite results aggregate their counts into the summary
                self.passed.fetch_add(*passed, Ordering::Relaxed);
                self.failed.fetch_add(*failed, Ordering::Relaxed);
                self.skipped.fetch_add(*skipped, Ordering::Relaxed);
            }
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
    let context = RunContext::new(options);
    Runner::run_cases(tests, &context, |test, _context| runner(test))
}
