use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use destack_formatter::format_file_source;
use destack_source::{File, FileId, Uri};
use destack_workspace::FormatterOptions;

use crate::core::{Case, CaseResult, RunContext, RunOptions, Suite};

use super::metric::StressMetric;
use super::process::{StressWorker, run_stress_child};
use super::{StressCase, materialize_formatter_cases};

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
    let source_size = source.len();
    let line_count = source.lines().count();
    let default_options = FormatterOptions::default();
    let narrow_options = FormatterOptions::default().with_line_width(60);
    let start = std::time::Instant::now();

    // check the default profile first
    if let Err(message) = check_format_idempotence(test, &source, "default", default_options) {
        return CaseResult::Failed { message };
    }

    // force additional line breaking through a narrow profile
    if let Err(message) = check_format_idempotence(test, &source, "narrow", narrow_options) {
        return CaseResult::Failed { message };
    }

    let metric = StressMetric::new(source_size, line_count, start.elapsed());
    eprintln!("{}", metric.format("formatted"));

    CaseResult::Passed
}

fn check_format_idempotence(
    test: &StressCase,
    source: &str,
    profile: &str,
    options: FormatterOptions,
) -> Result<(), String> {
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

    Ok(())
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
