use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions};
use destack_parser::source_colorizer;
use destack_source::PrintOptions;

use crate::harness::print::color;
use crate::harness::{
    RunContext, Suite, TestCase, TestOptions, TestResult, fixtures_dir, format_diagnostics,
};
use crate::mdtest::{
    MdTestCase, TEST_TIMEOUT_SECONDS, discover_md_files, parse_mdtest_file, run_with_timeout,
    setup_test_environment, slug,
};

/// Test suite for type checking specification tests.
#[derive(Debug, Default)]
pub struct SpecificationSuite {
    /// Map from test full name to the parsed test case.
    tests: HashMap<String, MdTestCase>,
    /// List of discovered test cases.
    cases: Vec<TestCase>,
}

impl SpecificationSuite {
    /// Load all specification tests from the fixtures/specification directory.
    pub fn load() -> Self {
        let fixtures = fixtures_dir();
        let mut suite = Self::default();

        let spec_dir = fixtures.join("specification");
        for md_path in discover_md_files(&spec_dir).unwrap_or_default() {
            suite.add_file(&spec_dir, &md_path);
        }

        suite
    }

    fn add_file(&mut self, base_dir: &Path, md_path: &Path) {
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
            let test_case =
                TestCase::file(name, md_path.to_path_buf(), "destack_test::specification")
                    .with_skipped(case.skip);

            self.tests.insert(test_case.full_name(), case);
            self.cases.push(test_case);
        }
    }
}

impl Suite for SpecificationSuite {
    fn name(&self) -> &'static str {
        "specification"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.cases.clone()
    }

    fn run(&self, case: &TestCase, context: &RunContext<'_>) -> TestResult {
        let Some(md_test) = self.tests.get(&case.full_name()) else {
            return TestResult::Failed {
                message: "test not found".to_string(),
            };
        };

        let timeout = context
            .timeout
            .unwrap_or(Duration::from_secs(TEST_TIMEOUT_SECONDS));
        run_with_timeout(md_test.clone(), timeout, run_specification_test)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(TEST_TIMEOUT_SECONDS))
    }
}

/// Run a single spec test: compile the code and compare errors against expectations.
fn run_specification_test(test: &MdTestCase) -> TestResult {
    let (session, program, _main_path) = setup_test_environment(test);

    // compile with single worker for deterministic results
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );

    // find the main file to compile
    let main_path = PathBuf::from("/test").join(
        test.files
            .iter()
            .find(|f| f.path == "main.ds")
            .map(|f| &f.path)
            .unwrap_or(&test.files[0].path),
    );

    let module_id = match compiler.resolve_path_to_module(&main_path) {
        Ok(id) => id,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to resolve module: {e:?}"),
            };
        }
    };

    // run analysis
    compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate { module: module_id });
    compiler.compile();
    drop(compiler);

    // collect actual errors
    let diagnostics = program.diagnostics.collect();
    let diagnostics_vec = diagnostics.iter();
    let actual_errors: Vec<String> = diagnostics_vec
        .iter()
        .filter(|d| d.severity == destack_source::DiagnosticSeverity::Error)
        .map(|d| d.message.clone())
        .collect();

    // compare against expected errors (bullet items in markdown)
    let result = compare_errors(&test.bullet_items, &actual_errors);

    // on failure, append rendered diagnostics for context
    match result {
        TestResult::Failed { mut message } => {
            let options = PrintOptions::new().with_colorizer(source_colorizer());
            let rendered = format_diagnostics(&program.files, &diagnostics, options);
            if !rendered.is_empty() {
                if !message.is_empty() {
                    message.push('\n');
                    message.push('\n');
                }
                message.push_str(&rendered);
            }
            TestResult::Failed { message }
        }
        other => other,
    }
}

/// Compare expected errors against actual errors.
fn compare_errors(expected: &[String], actual: &[String]) -> TestResult {
    let expected_patterns: Vec<ExpectedError> =
        expected.iter().map(|s| ExpectedError::parse(s)).collect();
    let actual_normalized: Vec<String> = actual.iter().map(|s| normalize_error(s)).collect();

    // find expected errors that didn't occur
    let mut missing: Vec<&str> = Vec::new();
    for expected in &expected_patterns {
        if !actual_normalized.iter().any(|a| expected.matches(a)) {
            missing.push(expected.as_str());
        }
    }

    // find actual errors that weren't expected
    let mut unexpected: Vec<&str> = Vec::new();
    for act in &actual_normalized {
        if !expected_patterns.iter().any(|e| e.matches(act)) {
            unexpected.push(act.as_str());
        }
    }

    // if no errors are missing or unexpected, return passed
    if missing.is_empty() && unexpected.is_empty() {
        return TestResult::Passed;
    }

    // build failure message
    let mut message = String::new();
    if !missing.is_empty() {
        message.push_str(&format!("{}\n", color::red("missing expected errors:")));
        for err in &missing {
            message.push_str(&format!("  {}\n", color::red(&format!("- {err}"))));
        }
    }
    if !unexpected.is_empty() {
        if !message.is_empty() {
            message.push('\n');
        }
        message.push_str(&format!(
            "{}\n",
            color::green("additional unexpected errors:")
        ));
        for err in &unexpected {
            message.push_str(&format!("  {}\n", color::green(&format!("+ {err}"))));
        }
    }

    TestResult::Failed { message }
}

/// Normalize an error message for fuzzy comparison.
fn normalize_error(s: &str) -> String {
    let s = s.trim().to_lowercase();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// An expected error pattern: either exact match or substring match.
#[derive(Debug, Clone)]
enum ExpectedError {
    /// Exact match against normalized error message.
    Exact(String),
    /// Substring match (use "contains: foo" in test).
    Contains(String),
}

impl ExpectedError {
    /// Parse an expected error string.
    /// Use "contains: foo" prefix for substring matching.
    fn parse(raw: &str) -> Self {
        let raw = raw.trim();
        let lower = raw.to_lowercase();
        if let Some(rest) = lower.strip_prefix("contains:") {
            return Self::Contains(normalize_error(rest));
        }
        Self::Exact(normalize_error(raw))
    }

    /// Check if an actual error matches this expectation.
    fn matches(&self, actual: &str) -> bool {
        match self {
            ExpectedError::Exact(expected) => actual == expected,
            ExpectedError::Contains(expected) => actual.contains(expected),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            ExpectedError::Exact(s) => s,
            ExpectedError::Contains(s) => s,
        }
    }
}
