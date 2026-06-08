use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use destack_formatter::format_file_source;
use destack_repository::FormatterOptions;
use destack_source::{File, FileId, Uri};

use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use super::process::{StressWorker, run_stress_child};
use super::{StressCase, StressExpectation, materialize_formatter_cases};

const WORKER_TIMEOUT: Duration = Duration::from_secs(60);
const HARNESS_TIMEOUT: Duration = Duration::from_secs(65);

/// Stress test suite for the formatter.
#[derive(Debug, Clone, Copy, Default)]
pub struct FormatterStressSuite;

impl FormatterStressSuite {
    /// Materialize formatter stress fixtures and return the generated case count.
    pub fn generate() -> Result<usize, String> {
        materialize_formatter_cases().map(|cases| cases.len())
    }

    /// Run one formatter stress fixture inside a worker process.
    pub fn run_worker(path: PathBuf) -> CaseResult {
        let stress_case = match StressCase::from_path(path) {
            Ok(stress_case) => stress_case,
            Err(message) => return CaseResult::Failed { message },
        };

        run_formatter_stress(&stress_case)
    }
}

impl Suite for FormatterStressSuite {
    fn name(&self) -> &'static str {
        "stress-formatter"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        materialize_formatter_cases()
            .unwrap_or_else(|error| {
                eprintln!("{error}");
                Vec::new()
            })
            .into_iter()
            .map(|case| Case::file(case.name, case.path, StressCase::formatter_category()))
            .collect()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        let stress_case = match StressCase::from_case(case) {
            Ok(stress_case) => stress_case,
            Err(message) => return CaseResult::Failed { message },
        };

        run_stress_child(StressWorker::Formatter, &stress_case, WORKER_TIMEOUT)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(HARNESS_TIMEOUT)
    }
}

fn run_formatter_stress(test: &StressCase) -> CaseResult {
    let source = match test.source() {
        Ok(source) => source,
        Err(message) => return CaseResult::Failed { message },
    };
    let default_options = FormatterOptions::default();
    let narrow_options = FormatterOptions::default().with_line_width(60);
    let start = std::time::Instant::now();

    // damaged and bounded fixtures may fail, but must not crash
    if test.expectation != StressExpectation::Valid {
        check_format_fallible(test, &source, default_options, start);

        return CaseResult::Passed;
    }

    let source_size = source.len();
    let line_count = source.lines().count();
    let mut stats = FormatterStressStats::new(source_size, line_count);

    // check the default profile first
    let default_stats = match check_format_idempotence(test, &source, "default", default_options) {
        Ok(stats) => stats,
        Err(message) => return CaseResult::Failed { message },
    };
    stats.add_profile(default_stats);

    // force additional line breaking through a narrow profile
    let narrow_stats = match check_format_idempotence(test, &source, "narrow", narrow_options) {
        Ok(stats) => stats,
        Err(message) => return CaseResult::Failed { message },
    };
    stats.add_profile(narrow_stats);

    eprintln!("{}", stats.format(start.elapsed()));

    CaseResult::Passed
}

/// Check one fallible formatter case for graceful success or failure.
fn check_format_fallible(
    test: &StressCase,
    source: &str,
    options: FormatterOptions,
    start: std::time::Instant,
) {
    let file = stress_file(test, source);
    let source_size = source.len();
    let line_count = source.lines().count();

    match format_file_source(&file, source, options) {
        Ok(formatted) => {
            let formatted_size = formatted.len();
            eprintln!(
                "fallible ok: {source_size} bytes, {line_count} lines -> {formatted_size} bytes in {:?}",
                start.elapsed()
            );
        }
        Err(error) => {
            eprintln!(
                "fallible error: {source_size} bytes, {line_count} lines in {:?}: {}",
                start.elapsed(),
                error.message
            );
        }
    }
}

fn check_format_idempotence(
    test: &StressCase,
    source: &str,
    profile: &str,
    options: FormatterOptions,
) -> Result<FormatterStressProfileStats, String> {
    let file = stress_file(test, source);

    // parse and format the source once
    let first = format_file_source(&file, source, options)
        .map_err(|error| format!("first format failed: {}", error.message))?;
    let file = stress_file(test, &first);

    // parse and format the formatted source again
    let second = format_file_source(&file, &first, options)
        .map_err(|error| format!("second format failed: {}", error.message))?;

    if first != second {
        return Err(format!(
            "formatter stress case was not idempotent under {profile} profile"
        ));
    }

    write_formatted_output(test, profile, &first)?;

    Ok(FormatterStressProfileStats::new(source, &first))
}

fn write_formatted_output(test: &StressCase, profile: &str, output: &str) -> Result<(), String> {
    let Some(stem) = test.path.file_stem().and_then(|stem| stem.to_str()) else {
        return Err(format!(
            "stress case has no utf-8 file stem: {}",
            test.path.display()
        ));
    };
    let Some(extension) = test
        .path
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return Err(format!(
            "stress case has no utf-8 extension: {}",
            test.path.display()
        ));
    };

    let output_name = format!("{stem}.{profile}.formatted.{extension}");
    let output_path = test.path.with_file_name(output_name);

    fs::write(&output_path, output)
        .map_err(|error| format!("failed to write {}: {error}", output_path.display()))
}

fn stress_file(test: &StressCase, source: &str) -> File {
    let file_name = test.file_name();
    let file_id = FileId::from_logical_path(&test.logical_path());

    File::from_text(
        file_id,
        file_name.clone(),
        Uri::from_string(&file_name),
        None,
        test.file_type,
        source.to_string(),
    )
}

/// Formatter stress throughput accounting.
#[derive(Debug, Clone, Copy)]
struct FormatterStressStats {
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

impl FormatterStressStats {
    /// Create formatter stress accounting for one source.
    const fn new(input_bytes: usize, input_lines: usize) -> Self {
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
    fn add_profile(&mut self, profile: FormatterStressProfileStats) {
        self.checked_bytes += profile.checked_bytes;
        self.checked_lines += profile.checked_lines;
        self.output_bytes += profile.output_bytes;
        self.output_lines += profile.output_lines;
    }

    /// Format this accounting for terminal output.
    fn format(self, elapsed: Duration) -> String {
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

/// Formatter stress accounting for one profile.
#[derive(Debug, Clone, Copy)]
struct FormatterStressProfileStats {
    /// The source bytes formatted for this profile.
    checked_bytes: usize,
    /// The source lines formatted for this profile.
    checked_lines: usize,
    /// The formatted output bytes produced for this profile.
    output_bytes: usize,
    /// The formatted output lines produced for this profile.
    output_lines: usize,
}

impl FormatterStressProfileStats {
    /// Create profile accounting from the original and formatted source.
    fn new(source: &str, formatted: &str) -> Self {
        Self {
            checked_bytes: source.len() + formatted.len(),
            checked_lines: source.lines().count() + formatted.lines().count(),
            output_bytes: formatted.len(),
            output_lines: formatted.lines().count(),
        }
    }
}
