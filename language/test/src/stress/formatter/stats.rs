use std::time::Duration;

use super::timing::{FormatterTiming, format_duration};

/// Formatter stress stats for one run.
#[derive(Debug, Clone, Copy)]
pub(super) struct FormatterRunStats {
    /// The original input byte count.
    input_bytes: usize,
    /// The original input line count.
    input_lines: usize,
    /// The source bytes formatted across all profile passes.
    checked_bytes: usize,
    /// The source lines formatted across all profile passes.
    checked_lines: usize,
    /// The formatted output bytes produced across all profiles.
    output_bytes: usize,
    /// The formatted output lines produced across all profiles.
    output_lines: usize,
}

impl FormatterRunStats {
    /// Create formatter stress stats for one source.
    pub(super) const fn new(input_bytes: usize, input_lines: usize) -> Self {
        Self {
            input_bytes,
            input_lines,
            checked_bytes: 0,
            checked_lines: 0,
            output_bytes: 0,
            output_lines: 0,
        }
    }

    /// Add one formatter profile pass.
    pub(super) fn add_profile(&mut self, profile: FormatterProfileStats) {
        self.checked_bytes += profile.checked_bytes;
        self.checked_lines += profile.checked_lines;
        self.output_bytes += profile.output_bytes;
        self.output_lines += profile.output_lines;
    }

    /// Format these stats for terminal output.
    pub(super) fn format(self, elapsed: Duration) -> String {
        let seconds = elapsed.as_secs_f64().max(f64::EPSILON);
        let checked_megabytes = self.checked_bytes as f64 / 1_000_000.0;
        let checked_megabytes_per_second = checked_megabytes / seconds;
        let output_megabytes = self.output_bytes as f64 / 1_000_000.0;
        let output_megabytes_per_second = output_megabytes / seconds;
        let checked_lines_per_second = self.checked_lines as f64 / seconds;
        let output_lines_per_second = self.output_lines as f64 / seconds;

        format!(
            "formatted: input {} bytes, {} lines; checked {} bytes, {} lines; output {} bytes, {} lines; {:.3}s, {:.2} checked MB/s, {:.0} checked lines/s, {:.2} output MB/s, {:.0} output lines/s",
            self.input_bytes,
            self.input_lines,
            self.checked_bytes,
            self.checked_lines,
            self.output_bytes,
            self.output_lines,
            seconds,
            checked_megabytes_per_second,
            checked_lines_per_second,
            output_megabytes_per_second,
            output_lines_per_second,
        )
    }
}

/// Formatter stress stats for one profile.
#[derive(Debug, Clone, Copy)]
pub(super) struct FormatterProfileStats {
    /// The source bytes formatted for this profile.
    checked_bytes: usize,
    /// The source lines formatted for this profile.
    checked_lines: usize,
    /// The formatted output bytes produced for this profile.
    output_bytes: usize,
    /// The formatted output lines produced for this profile.
    output_lines: usize,
    /// The first formatting pass timing.
    first: FormatterTiming,
    /// The second formatting pass timing.
    second: FormatterTiming,
    /// The formatted output write time.
    write: Duration,
}

impl FormatterProfileStats {
    /// Create profile stats from the original and formatted source.
    pub(super) fn new(
        source: &str,
        formatted: &str,
        first: FormatterTiming,
        second: FormatterTiming,
    ) -> Self {
        Self {
            checked_bytes: source.len() + formatted.len(),
            checked_lines: source.lines().count() + formatted.lines().count(),
            output_bytes: formatted.len(),
            output_lines: formatted.lines().count(),
            first,
            second,
            write: Duration::ZERO,
        }
    }

    /// Set the formatted output write time.
    pub(super) fn with_write_elapsed(mut self, elapsed: Duration) -> Self {
        self.write = elapsed;

        self
    }

    /// Format this profile's phase timings.
    pub(super) fn format_phases(self) -> String {
        format!(
            "first [{}]; second [{}]; write {}",
            self.first.format(),
            self.second.format(),
            format_duration(self.write)
        )
    }
}
