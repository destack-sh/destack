use std::time::Duration;

/// Throughput for one stress case.
#[derive(Debug, Clone, Copy)]
pub(super) struct StressMetric {
    /// The processed byte count.
    bytes: usize,
    /// The processed line count.
    lines: usize,
    /// The elapsed processing time.
    elapsed: Duration,
}

impl StressMetric {
    /// Create throughput metrics for one stress case.
    pub(super) const fn new(bytes: usize, lines: usize, elapsed: Duration) -> Self {
        Self {
            bytes,
            lines,
            elapsed,
        }
    }

    /// Format the metric for terminal output.
    pub(super) fn format(self, action: &str) -> String {
        let seconds = self.elapsed.as_secs_f64().max(f64::EPSILON);
        let megabytes = self.bytes as f64 / 1_000_000.0;
        let megabytes_per_second = megabytes / seconds;
        let lines_per_second = self.lines as f64 / seconds;

        format!(
            "{action}: {} bytes, {} lines, {:.3}s, {:.2} MB/s, {:.0} lines/s",
            self.bytes, self.lines, seconds, megabytes_per_second, lines_per_second,
        )
    }
}
