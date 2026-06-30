use std::time::Instant;

use destack_repository::FormatterOptions;

use crate::core::CaseResult;
use crate::stress::{StressExpectation, StressFixture};

use super::output::write_formatted_output;
use super::pass::format_pass;
use super::stats::{FormatterProfileStats, FormatterRunStats};

/// Run one formatter stress fixture.
pub(super) fn run_formatter_stress(fixture: &StressFixture) -> CaseResult {
    let source = match fixture.source() {
        Ok(source) => source,
        Err(message) => return CaseResult::Failed { message },
    };
    let default_options = FormatterOptions::default();
    let narrow_options = FormatterOptions::default().with_line_width(60);
    let start = Instant::now();

    // damaged and bounded fixtures may fail, but must not crash
    if fixture.expectation != StressExpectation::Valid {
        check_format_fallible(fixture, &source, default_options, start);

        return CaseResult::Passed;
    }

    let source_size = source.len();
    let line_count = source.lines().count();
    let mut stats = FormatterRunStats::new(source_size, line_count);

    // check the default profile first
    let default_stats = match check_format_idempotence(fixture, &source, "default", default_options)
    {
        Ok(stats) => stats,
        Err(message) => return CaseResult::Failed { message },
    };
    stats.add_profile(default_stats);

    // force additional line breaking through a narrow profile
    let narrow_stats = match check_format_idempotence(fixture, &source, "narrow", narrow_options) {
        Ok(stats) => stats,
        Err(message) => return CaseResult::Failed { message },
    };
    stats.add_profile(narrow_stats);

    eprintln!("{}", stats.format(start.elapsed()));

    CaseResult::Passed
}

/// Check one fallible formatter fixture for graceful success or failure.
fn check_format_fallible(
    fixture: &StressFixture,
    source: &str,
    options: FormatterOptions,
    start: Instant,
) {
    let source_size = source.len();
    let line_count = source.lines().count();

    match format_pass(fixture, source, options) {
        Ok(formatted) => {
            let formatted_size = formatted.output.len();
            eprintln!(
                "fallible ok: {source_size} bytes, {line_count} lines -> {formatted_size} bytes in {:?}; {}",
                start.elapsed(),
                formatted.timing.format()
            );
        }
        Err(error) => {
            eprintln!(
                "fallible error: {source_size} bytes, {line_count} lines in {:?}: {}",
                start.elapsed(),
                error
            );
        }
    }
}

/// Check one valid formatter fixture for idempotence under one profile.
fn check_format_idempotence(
    fixture: &StressFixture,
    source: &str,
    profile: &str,
    options: FormatterOptions,
) -> Result<FormatterProfileStats, String> {
    // parse and format the source once
    let first = format_pass(fixture, source, options)
        .map_err(|error| format!("first format failed: {error}"))?;

    // parse and format the formatted source again
    let second = format_pass(fixture, &first.output, options)
        .map_err(|error| format!("second format failed: {error}"))?;

    if first.output != second.output {
        return Err(format!(
            "formatter stress fixture was not idempotent under {profile} profile"
        ));
    }

    let write_start = Instant::now();
    write_formatted_output(fixture, profile, &first.output)?;
    let write_elapsed = write_start.elapsed();
    let profile_stats =
        FormatterProfileStats::new(source, &first.output, first.timing, second.timing)
            .with_write_elapsed(write_elapsed);

    eprintln!("profile {profile}: {}", profile_stats.format_phases());

    Ok(profile_stats)
}
