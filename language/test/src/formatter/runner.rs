use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::core::{
    Case, CaseResult, MarkdownSuiteIndex, RunContext, RunOptions, Runner, Suite,
    discover_file_cases, discover_markdown_suite, expected_failures_view, fixtures_dir,
    update_failure_baseline,
};
use crate::mdtest::MdTestCase;

use super::{roundtrip, transform};

/// Test suite combining roundtrip and transform formatter tests.
#[derive(Debug, Default)]
pub struct FormatterSuite {
    mdtests: HashMap<String, MdTestCase>,
    cases: Vec<Case>,
    expected_failures: HashSet<String>,
    expected_failures_path: PathBuf,
}

impl FormatterSuite {
    /// Load all formatter tests from the fixtures/formatter directory.
    pub fn load() -> Result<Self, String> {
        let fixtures = fixtures_dir();
        let formatter_dir = fixtures.join("formatter");
        let MarkdownSuiteIndex {
            cases,
            entries,
            expected_failures,
            expected_failures_path,
        } = discover_markdown_suite(&formatter_dir, "destack_test::formatter::transform", Some)?;
        let mut suite = Self {
            mdtests: entries,
            cases,
            expected_failures,
            expected_failures_path,
        };

        suite.discover_roundtrip_tests(&formatter_dir)?;

        Ok(suite)
    }

    fn discover_roundtrip_tests(&mut self, base_dir: &Path) -> Result<(), String> {
        let roundtrip_dir = base_dir.join("roundtrip");
        let roundtrip_tests = discover_file_cases(
            &roundtrip_dir,
            &["ds", ".d.ds", "js", "jsx", "ts", "tsx", ".d.ts"],
            "destack_test::formatter::roundtrip",
        )
        .map_err(|error| {
            format!(
                "failed to discover formatter roundtrip fixtures in {}: {error}",
                roundtrip_dir.display()
            )
        })?;

        for test in roundtrip_tests {
            self.cases.push(test);
        }

        Ok(())
    }
}

impl Suite for FormatterSuite {
    fn name(&self) -> &'static str {
        "formatter"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        self.cases.clone()
    }

    fn run(&self, case: &Case, context: &RunContext<'_>) -> CaseResult {
        if let Some(md_test) = self.mdtests.get(&case.full_name()) {
            transform::run(md_test)
        } else {
            roundtrip::run(case, context.options)
        }
    }

    fn expected_failures(&self, _options: &RunOptions) -> Option<&HashSet<String>> {
        expected_failures_view(&self.expected_failures)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(10))
    }

    fn report(&self, results: &[(Case, CaseResult)], context: &RunContext<'_>) {
        if !context.options.update_known_failures {
            return;
        }

        let failure_count = match update_failure_baseline(&self.expected_failures_path, results) {
            Ok(failure_count) => failure_count,
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        };

        println!(
            "  {} updated with {} failures",
            self.expected_failures_path.display(),
            failure_count
        );
    }
}

/// Run all formatter tests.
pub fn run_formatter_tests(options: &RunOptions) -> std::process::ExitCode {
    let suite = match FormatterSuite::load() {
        Ok(suite) => suite,
        Err(error) => {
            eprintln!("{error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    Runner::run_suite(suite, options)
}
