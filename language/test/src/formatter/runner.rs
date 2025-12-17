use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::harness::{
    RunContext, Runner, Suite, TestCase, TestOptions, TestResult, discover_test_files, fixtures_dir,
};
use crate::mdtest::{MdTestCase, discover_md_files, parse_mdtest_file, slug};

use super::{roundtrip, transform};

/// Test suite combining roundtrip and transform formatter tests.
#[derive(Debug, Default)]
pub struct FormatterSuite {
    mdtests: HashMap<String, MdTestCase>,
    cases: Vec<TestCase>,
}

impl FormatterSuite {
    /// Load all formatter tests from the fixtures/formatter directory.
    pub fn load() -> Self {
        let fixtures = fixtures_dir();
        let mut suite = Self::default();
        let formatter_dir = fixtures.join("formatter");

        suite.discover_roundtrip_tests(&formatter_dir);
        suite.discover_mdtest_tests(&formatter_dir);

        suite
    }

    fn discover_roundtrip_tests(&mut self, base_dir: &Path) {
        let ds_tests = discover_test_files(
            base_dir,
            &["ds", ".d.ds"],
            "destack_test::formatter::roundtrip",
        )
        .unwrap_or_default();

        for test in ds_tests {
            self.cases.push(test);
        }
    }

    fn discover_mdtest_tests(&mut self, base_dir: &Path) {
        for md_path in discover_md_files(base_dir).unwrap_or_default() {
            self.add_mdtest_file(base_dir, &md_path);
        }
    }

    fn add_mdtest_file(&mut self, base_dir: &Path, md_path: &Path) {
        let cases = match parse_mdtest_file(md_path) {
            Ok(cases) => cases,
            Err(error) => {
                eprintln!("failed to parse {}: {error}", md_path.display());
                return;
            }
        };

        let relative_path = md_path.strip_prefix(base_dir).unwrap_or(md_path);
        let relative_name = relative_path.to_string_lossy();

        for case in cases {
            let name = format!(
                "{relative_name}/{}/{}",
                slug(&case.section),
                slug(&case.name)
            );
            let test_case = TestCase::file(
                name,
                md_path.to_path_buf(),
                "destack_test::formatter::transform",
            )
            .with_skipped(case.skip);

            self.mdtests.insert(test_case.full_name(), case);
            self.cases.push(test_case);
        }
    }
}

impl Suite for FormatterSuite {
    fn name(&self) -> &'static str {
        "formatter"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.cases.clone()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        if let Some(md_test) = self.mdtests.get(&case.full_name()) {
            transform::run(md_test)
        } else {
            roundtrip::run(case)
        }
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(10))
    }
}

/// Run all formatter tests.
pub fn run_formatter_tests(options: &TestOptions) -> std::process::ExitCode {
    let suite = FormatterSuite::load();
    Runner::run_suite(&suite, options)
}
