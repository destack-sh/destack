use std::time::Duration;

use crate::core::print::color;

use super::{ConformanceResult, ConformanceSuiteResult, ReadmeResults};

/// Print a summary table of all suite results.
pub fn print_summary(results: &[ConformanceSuiteResult], baseline: Option<&ReadmeResults>) {
    if results.len() <= 1 {
        return;
    }

    // totals
    let total_passed: usize = results.iter().map(|suite| suite.result.passed).sum();
    let total_failed: usize = results.iter().map(|suite| suite.result.failed).sum();
    let total_skipped: usize = results.iter().map(|suite| suite.result.skipped).sum();
    let total_timedout: usize = results.iter().map(|suite| suite.result.timedout).sum();
    let total_cases: usize = results.iter().map(|suite| suite.result.total_run()).sum();
    let total_duration: Duration = results.iter().map(|suite| suite.duration).sum();
    let total_regressions: usize = results
        .iter()
        .map(|suite| suite.result.regressions.len())
        .sum();
    let are_known_failures_included = results.iter().all(|suite| suite.ran_known_failures);

    let overall_rate = if total_cases > 0 {
        total_passed as f64 / total_cases as f64 * 100.0
    } else {
        100.0
    };
    let total_cases_with_skipped = total_cases + total_skipped;
    let overall_rate_with_skipped = if total_cases_with_skipped > 0 {
        total_passed as f64 / total_cases_with_skipped as f64 * 100.0
    } else {
        100.0
    };

    // baseline delta
    let old_total_rate = baseline
        .map(|baseline| {
            let passed: usize = baseline.rows.iter().map(|row| row.passed).sum();
            let total: usize = baseline.rows.iter().map(|row| row.total).sum();

            if total > 0 {
                passed as f64 / total as f64 * 100.0
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    // table header
    println!();
    println!("{}", color::bold("CONFORMANCE SUMMARY"));
    println!();
    println!(
        "  {:10}  {:>8}  {:>8}  {:>8}  {:>8}  {:>8}  {:>10}  {:>10}",
        "Suite", "Passed", "Failed", "Skipped", "Total", "Rate", "Incl. Rate", "Δ Rate"
    );
    println!("  {}", "─".repeat(86));

    // suite rows
    for suite in results {
        print_suite_summary_row(suite, baseline);
    }

    println!("  {}", "─".repeat(86));

    // total row
    let total_rate_delta = if baseline.is_some() {
        overall_rate - old_total_rate
    } else {
        0.0
    };

    let total_label = format!("{:10}", "TOTAL");
    let total_passed = format!("{total_passed:>8}");
    let total_failed = format!("{:>8}", total_failed + total_timedout);
    let total_skipped = if total_skipped > 0 {
        format!("{total_skipped:>8}")
    } else {
        format!("{:>8}", "-")
    };

    println!(
        "  {}  {}  {}  {}  {:>8}  {}  {}  {}",
        color::bold(&total_label),
        color::green(&total_passed),
        color::red(&total_failed),
        color::dim(&total_skipped),
        total_cases,
        colorize_rate(format!("{overall_rate:>7.2}%"), overall_rate),
        colorize_rate(
            format!("{overall_rate_with_skipped:>9.2}%"),
            overall_rate_with_skipped
        ),
        format_rate_delta_inline(total_rate_delta)
    );
    println!();
    println!(
        "  {}",
        color::dim(&format!(
            "completed in {:.2}s",
            total_duration.as_secs_f64()
        ))
    );
    println!();

    // final verdict
    let total_fixed: usize = results.iter().map(|suite| suite.result.fixed.len()).sum();
    if total_fixed > 0 {
        println!(
            "  {} {} tests fixed across all suites",
            color::green("FIXED:"),
            total_fixed
        );
    }

    if total_regressions > 0 {
        if are_known_failures_included {
            println!(
                "  {} {} tests failed across all suites",
                color::red("FAILED:"),
                total_regressions
            );
        } else {
            println!(
                "  {} {} tests regressed across all suites",
                color::red("FAILED:"),
                total_regressions
            );
        }
    } else {
        println!(
            "  {} all {} tests accounted for",
            color::green("PASSED:"),
            total_cases
        );
    }

    println!();
}

/// Print one full conformance suite result block.
pub fn print_conformance_result(
    suite_name: &str,
    result: &ConformanceResult,
    duration: Duration,
    runs_known_failures: bool,
) {
    // headline
    let total = result.total();
    let pass_rate = result.pass_rate();

    println!();
    println!(
        "{} conformance: {} ({}/{} tests passing)",
        color::bold(suite_name),
        colorize_rate(format!("{pass_rate:.2}%"), pass_rate),
        result.passed,
        total
    );
    println!();

    // summary counters
    let passed_pct = if total == 0 {
        100.0
    } else {
        result.passed as f64 / total as f64 * 100.0
    };
    let failed_pct = if total == 0 {
        0.0
    } else {
        result.failed as f64 / total as f64 * 100.0
    };

    println!(
        "  {}  {:>5}  ({:.2}%)",
        color::green("passed:"),
        result.passed,
        passed_pct
    );
    println!(
        "  {}  {:>5}  ({:.2}%)",
        color::red("failed:"),
        result.failed,
        failed_pct
    );

    if result.failed > 0 {
        println!(
            "  {}  parse={} output={} idempotence={} read={}",
            color::dim("failure kinds:"),
            result.parse_failed,
            result.output_failed,
            result.idempotence_failed,
            result.read_failed
        );
    }

    if result.skipped > 0 {
        println!("  {} {:>5}", color::yellow("skipped:"), result.skipped);
    }

    // timeout count
    if result.timedout > 0 {
        let timedout_pct = if total == 0 {
            0.0
        } else {
            result.timedout as f64 / total as f64 * 100.0
        };

        println!(
            "  {} {:>5}  ({:.2}%)",
            color::yellow("timeout:"),
            result.timedout,
            timedout_pct
        );
    }

    // elapsed time
    println!(
        "  {}",
        color::dim(&format!("finished in {:.2}s", duration.as_secs_f64()))
    );

    // category breakdown
    if result.categories.len() > 1 {
        let max_name_len = result
            .categories
            .keys()
            .map(|category| category.len())
            .max()
            .unwrap_or(10);
        let max_passed = result
            .categories
            .values()
            .map(|stats| stats.passed)
            .max()
            .unwrap_or(1);
        let max_total = result
            .categories
            .values()
            .map(|stats| stats.total())
            .max()
            .unwrap_or(1);
        let passed_width = max_passed.to_string().len();
        let total_width = max_total.to_string().len();

        println!();
        println!("  {}", color::bold("by category:"));

        for (category, stats) in &result.categories {
            let rate = stats.pass_rate();
            println!(
                "    {:<name_width$}  {:>passed_width$} / {:>total_width$}  {}",
                category,
                stats.passed,
                stats.total(),
                colorize_rate(format!("{rate:>6.2}%"), rate),
                name_width = max_name_len,
                passed_width = passed_width,
                total_width = total_width
            );
        }
    }

    // detail blocks
    println!();
    print_case_list_block(
        "TIMEOUTS",
        &result.timeouts,
        "tests exceeded timeout, likely infinite loops",
    );
    print_case_list_block(
        "PARSE FAILURES",
        &result.parse_failure_tests,
        "tests failed in parser stage",
    );

    if !result.regressions.is_empty() {
        let header = if runs_known_failures {
            color::red("FAILURES")
        } else {
            color::red("REGRESSIONS")
        };
        let description = if runs_known_failures {
            "tests failed"
        } else {
            "tests failed unexpectedly"
        };
        print_named_case_list_block(&header, &result.regressions, description);
    }

    if !result.fixed.is_empty() {
        print_named_case_list_block(
            &color::green("FIXED"),
            &result.fixed,
            "tests now passing, remove from status.json known-fail entries",
        );
    }

    if !result.unskipped.is_empty() {
        print_named_case_list_block(
            &color::green("UNSKIPPED"),
            &result.unskipped,
            "skipped tests now passing, remove from status.json skip entries",
        );
    }

    // final line
    if result.has_regressions() {
        if runs_known_failures {
            println!(
                "test result: {}. {} failures detected",
                color::red("FAILED"),
                result.regressions.len()
            );
        } else {
            println!(
                "test result: {}. {} regressions detected",
                color::red("FAILED"),
                result.regressions.len()
            );
        }
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

/// Print one suite row in the multi-suite summary table.
fn print_suite_summary_row(suite: &ConformanceSuiteResult, baseline: Option<&ReadmeResults>) {
    // rates and delta
    let rate = suite.result.pass_rate();
    let rate_with_skipped = suite.result.pass_rate_with_skipped();
    let rate_delta = baseline
        .and_then(|baseline| baseline.find(&suite.name))
        .map(|old| rate - old.rate)
        .unwrap_or(0.0);

    // formatted cells
    let name = format!("{:10}", suite.name);
    let passed = format!("{:>8}", suite.result.passed);
    let failed = format!("{:>8}", suite.result.failed);
    let skipped = if suite.result.skipped > 0 {
        format!("{:>8}", suite.result.skipped)
    } else {
        format!("{:>8}", "-")
    };

    // output
    println!(
        "  {}  {}  {}  {}  {:>8}  {}  {}  {}",
        color::cyan(&name),
        color::green(&passed),
        color::red(&failed),
        color::dim(&skipped),
        suite.result.total_run(),
        colorize_rate(format!("{rate:>7.2}%"), rate),
        colorize_rate(format!("{rate_with_skipped:>9.2}%"), rate_with_skipped),
        format_rate_delta_inline(rate_delta)
    );
}

/// Print one yellow case list block when the list is non-empty.
fn print_case_list_block(header: &str, tests: &[String], description: &str) {
    if tests.is_empty() {
        return;
    }

    print_named_case_list_block(&color::yellow(header), tests, description);
}

/// Print one named case list block.
fn print_named_case_list_block(header: &str, tests: &[String], description: &str) {
    println!("{header} ({} {description}):", tests.len());

    let show_count = tests.len().min(20);
    for test in &tests[..show_count] {
        println!("  {test}");
    }

    if tests.len() > show_count {
        println!(
            "  {} ... and {} more",
            color::dim(""),
            tests.len() - show_count
        );
    }

    println!();
}

/// Color one preformatted rate string.
fn colorize_rate(rate: String, value: f64) -> String {
    if value >= 90.0 {
        color::green(&rate)
    } else if value >= 50.0 {
        color::yellow(&rate)
    } else {
        color::red(&rate)
    }
}

/// Format one inline percent delta.
fn format_rate_delta_inline(delta: f64) -> String {
    if delta > 0.005 {
        color::green(&format!("{delta:>+8.2}%"))
    } else if delta < -0.005 {
        color::red(&format!("{delta:>+8.2}%"))
    } else {
        color::dim(&format!("{:>9}", "±0.00%"))
    }
}
