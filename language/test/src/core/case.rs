use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_source::DiagnosticSeverity;

use super::{RunContext, RunOptions, Runner};

/// Result of running one case.
#[derive(Debug, Clone)]
pub enum CaseResult {
    /// The case passed.
    Passed,
    /// The case failed with a message.
    Failed { message: String },
    /// The case was skipped.
    Skipped { reason: String },
    /// Aggregated suite counts.
    Suite {
        /// The passed case count.
        passed: usize,
        /// The failed case count.
        failed: usize,
        /// The skipped case count.
        skipped: usize,
    },
}

impl CaseResult {
    /// Return whether the case passed.
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }

    /// Return whether the case failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Return whether the case was skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }

    /// Return whether this is a suite summary result.
    pub fn is_suite(&self) -> bool {
        matches!(self, Self::Suite { .. })
    }

    /// Return the pass rate for suite summary results.
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

/// Kind of case fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseKind {
    /// One file is the case fixture.
    File,
    /// One directory is the case fixture.
    Directory,
}

/// One discovered suite case.
#[derive(Debug, Clone)]
pub struct Case {
    /// The case name.
    pub name: String,
    /// The case file or directory path.
    pub path: PathBuf,
    /// The case category.
    pub category: String,
    /// The case fixture kind.
    pub kind: CaseKind,
    /// The minimum severity that fails the case.
    pub min_fail_severity: DiagnosticSeverity,
    /// Whether this case is marked as skipped.
    pub is_skipped: bool,
}

impl Case {
    /// Create one file case with default settings.
    pub fn file(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            category: category.into(),
            kind: CaseKind::File,
            min_fail_severity: DiagnosticSeverity::Warning,
            is_skipped: false,
        }
    }

    /// Create one directory case with default settings.
    pub fn directory(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            category: category.into(),
            kind: CaseKind::Directory,
            min_fail_severity: DiagnosticSeverity::Warning,
            is_skipped: false,
        }
    }

    /// Mark this case as skipped.
    pub fn with_skipped(mut self, is_skipped: bool) -> Self {
        self.is_skipped = is_skipped;
        self
    }

    /// Set the minimum severity that fails this case.
    pub fn with_min_fail_severity(mut self, severity: DiagnosticSeverity) -> Self {
        self.min_fail_severity = severity;
        self
    }

    /// Return the full case name including its category.
    pub fn full_name(&self) -> String {
        format!("{}::{}", self.category, self.name)
    }
}

/// Summary of one run across many cases.
#[derive(Debug, Default)]
pub struct RunSummary {
    /// The passed case count.
    passed: AtomicUsize,
    /// The failed case count.
    failed: AtomicUsize,
    /// The skipped case count.
    skipped: AtomicUsize,
}

impl RunSummary {
    /// Create one empty run summary.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one case result.
    pub fn record(&self, result: &CaseResult) {
        match result {
            CaseResult::Passed => {
                self.passed.fetch_add(1, Ordering::Relaxed);
            }
            CaseResult::Failed { .. } => {
                self.failed.fetch_add(1, Ordering::Relaxed);
            }
            CaseResult::Skipped { .. } => {
                self.skipped.fetch_add(1, Ordering::Relaxed);
            }
            CaseResult::Suite {
                passed,
                failed,
                skipped,
            } => {
                // suite results aggregate directly into the outer summary
                self.passed.fetch_add(*passed, Ordering::Relaxed);
                self.failed.fetch_add(*failed, Ordering::Relaxed);
                self.skipped.fetch_add(*skipped, Ordering::Relaxed);
            }
        };
    }

    /// Return the number of passed cases.
    pub fn passed(&self) -> usize {
        self.passed.load(Ordering::Relaxed)
    }

    /// Return the number of failed cases.
    pub fn failed(&self) -> usize {
        self.failed.load(Ordering::Relaxed)
    }

    /// Return the number of skipped cases.
    pub fn skipped(&self) -> usize {
        self.skipped.load(Ordering::Relaxed)
    }

    /// Return the total number of cases.
    pub fn total(&self) -> usize {
        self.passed() + self.failed() + self.skipped()
    }

    /// Return whether all cases passed.
    pub fn all_passed(&self) -> bool {
        self.failed() == 0
    }
}

/// Run cases and return the process exit code.
pub fn run_cases<F>(cases: Vec<Case>, options: &RunOptions, runner: F) -> ExitCode
where
    F: Fn(&Case) -> CaseResult + Send + Sync + 'static,
{
    let context = RunContext::new(options);
    Runner::run_cases(cases, &context, move |case, _context| runner(case))
}
