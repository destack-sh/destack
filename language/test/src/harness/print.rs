use std::time::Duration;

use super::{TestCase, TestResult, TestSummary};

/// ANSI color codes.
pub mod color {
    pub const BOLD: &str = "\x1b[1m";
    pub const GREEN: &str = "\x1b[32m";
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const CYAN: &str = "\x1b[36m";
    pub const DIM: &str = "\x1b[2m";
    pub const RESET: &str = "\x1b[0m";

    /// Apply bold green.
    pub fn green(s: &str) -> String {
        format!("{BOLD}{GREEN}{s}{RESET}")
    }

    /// Apply bold red.
    pub fn red(s: &str) -> String {
        format!("{BOLD}{RED}{s}{RESET}")
    }

    /// Apply bold yellow.
    pub fn yellow(s: &str) -> String {
        format!("{BOLD}{YELLOW}{s}{RESET}")
    }

    /// Apply bold cyan.
    pub fn cyan(s: &str) -> String {
        format!("{BOLD}{CYAN}{s}{RESET}")
    }

    /// Apply dim.
    pub fn dim(s: &str) -> String {
        format!("{DIM}{s}{RESET}")
    }

    /// Apply bold.
    pub fn bold(s: &str) -> String {
        format!("{BOLD}{s}{RESET}")
    }
}

/// Print test result with colors.
pub fn print_result(test: &TestCase, result: &TestResult, duration: Duration, verbose: bool) {
    let status = match result {
        TestResult::Passed => color::green("ok"),
        TestResult::Failed { .. } => color::red("FAILED"),
        TestResult::Skipped { .. } => color::yellow("skipped"),
        TestResult::Suite { passed, failed, .. } => {
            if *failed == 0 {
                color::green(&format!("suite ok ({passed} passed)"))
            } else {
                color::yellow(&format!("suite ({passed} passed, {failed} failed)"))
            }
        }
    };

    let duration_str = if duration.as_millis() > 100 {
        color::dim(&format!(" ({:.2}s)", duration.as_secs_f64()))
    } else {
        String::new()
    };

    println!("test {} ... {}{}", test.full_name(), status, duration_str);

    if verbose && let TestResult::Failed { message } = result {
        for line in message.lines() {
            println!("       {line}");
        }
    }
}

/// Print test summary.
pub fn print_summary(summary: &TestSummary, duration: Duration) {
    let passed = summary.passed();
    let failed = summary.failed();
    let skipped = summary.skipped();

    let status = if failed == 0 {
        color::green("ok")
    } else {
        color::red("FAILED")
    };

    // build parts with colors
    let mut parts = Vec::new();
    if passed > 0 {
        parts.push(color::green(&format!("{passed} passed")));
    }
    if failed > 0 {
        parts.push(color::red(&format!("{failed} failed")));
    }
    if skipped > 0 {
        parts.push(color::yellow(&format!("{skipped} skipped")));
    }

    let counts = parts.join("; ");
    let time = color::dim(&format!("finished in {:.2}s", duration.as_secs_f64()));

    println!();
    println!("test result: {status}. {counts}; {time}");
}

/// Print failures summary (list of failed test names).
pub fn print_failures(tests: &[(TestCase, TestResult)]) {
    let failures: Vec<_> = tests.iter().filter(|(_, r)| r.is_failed()).collect();
    if failures.is_empty() {
        return;
    }

    // just list failed test names (diagnostics were already printed inline)
    println!();
    println!("{}:", color::red("failures"));
    for (test, _) in &failures {
        println!("    {}", test.full_name());
    }
}

/// Print test list without running.
pub fn print_test_list(tests: &[TestCase]) {
    for test in tests {
        let suffix = if test.is_skipped {
            format!(": {} {}", color::yellow("test"), color::dim("(skipped)"))
        } else {
            format!(": {}", color::cyan("test"))
        };
        println!("{}{}", test.full_name(), suffix);
    }

    println!();
    let total = color::bold(&format!("{}", tests.len()));
    let skipped_count = tests.iter().filter(|t| t.is_skipped).count();
    if skipped_count > 0 {
        let skipped = color::yellow(&format!("{skipped_count} skipped"));
        println!("{total} tests ({skipped})");
    } else {
        println!("{total} tests");
    }
}
