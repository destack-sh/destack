use std::collections::BTreeMap;
use std::time::Duration;

/// Per-category statistics.
#[derive(Debug, Clone, Default)]
pub struct CategoryStats {
    /// The passed case count.
    pub passed: usize,
    /// The failed case count.
    pub failed: usize,
    /// The skipped case count.
    pub skipped: usize,
}

impl CategoryStats {
    /// Return the run case count without skipped cases.
    pub fn total(&self) -> usize {
        self.passed + self.failed
    }

    /// Return the total case count including skipped cases.
    pub fn total_with_skipped(&self) -> usize {
        self.passed + self.failed + self.skipped
    }

    /// Return the pass rate without skipped cases.
    pub fn pass_rate(&self) -> f64 {
        let total = self.total();

        if total == 0 {
            100.0
        } else {
            self.passed as f64 / total as f64 * 100.0
        }
    }

    /// Return the pass rate including skipped cases.
    pub fn pass_rate_with_skipped(&self) -> f64 {
        let total = self.total_with_skipped();

        if total == 0 {
            100.0
        } else {
            self.passed as f64 / total as f64 * 100.0
        }
    }
}

/// Result of one conformance suite run.
#[derive(Debug)]
pub struct ConformanceResult {
    /// The discovered case count before filtering.
    pub total_discovered: usize,
    /// The selected case count after filtering.
    pub total_selected: usize,
    /// The passed case count.
    pub passed: usize,
    /// The failed case count.
    pub failed: usize,
    /// The skipped case count.
    pub skipped: usize,
    /// The timed out case count.
    pub timedout: usize,
    /// Cases that failed in parser stage.
    pub parse_failed: usize,
    /// Case names that failed in parser stage.
    pub parse_failure_tests: Vec<String>,
    /// Cases that failed due to output mismatch.
    pub output_failed: usize,
    /// Cases that failed due to idempotence mismatch.
    pub idempotence_failed: usize,
    /// Cases that failed due to read errors.
    pub read_failed: usize,
    /// Cases counted as regressions.
    pub regressions: Vec<String>,
    /// Cases that were known failures and now pass.
    pub fixed: Vec<String>,
    /// Cases that were skipped and now pass.
    pub unskipped: Vec<String>,
    /// Cases that timed out.
    pub timeouts: Vec<String>,
    /// Per-category statistics.
    pub categories: BTreeMap<String, CategoryStats>,
}

impl ConformanceResult {
    /// Return whether the suite was filtered.
    pub fn is_filtered(&self) -> bool {
        self.total_selected != self.total_discovered
    }

    /// Return whether the suite has regressions.
    pub fn has_regressions(&self) -> bool {
        !self.regressions.is_empty()
    }

    /// Return the pass rate without skipped cases.
    pub fn pass_rate(&self) -> f64 {
        let total = self.total_run();

        if total == 0 {
            100.0
        } else {
            self.passed as f64 / total as f64 * 100.0
        }
    }

    /// Return the pass rate including skipped cases.
    pub fn pass_rate_with_skipped(&self) -> f64 {
        let total = self.total();

        if total == 0 {
            100.0
        } else {
            self.passed as f64 / total as f64 * 100.0
        }
    }

    /// Return the total run case count without skipped cases.
    pub fn total_run(&self) -> usize {
        self.passed + self.failed + self.timedout
    }

    /// Return the total case count including skipped cases.
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped + self.timedout
    }

    /// Return whether any case timed out.
    pub fn has_timeouts(&self) -> bool {
        !self.timeouts.is_empty()
    }
}

/// Result of running one suite, for aggregation.
#[derive(Debug)]
pub struct ConformanceSuiteResult {
    /// The suite name.
    pub name: String,
    /// The suite result payload.
    pub result: ConformanceResult,
    /// The elapsed suite duration.
    pub duration: Duration,
    /// Whether known failures were included as normal failures.
    pub include_known_failures: bool,
}
