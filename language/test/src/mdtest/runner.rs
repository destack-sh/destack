use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::{io, thread};

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler};
use destack_source::{FileRegistry, FileSystem, LanguageOptions, MemoryFileSystem};
use destack_workspace::Program;

use crate::harness::{
    RunContext, Runner, Suite, TestCase, TestOptions, TestResult, discover_test_files, fixtures_dir,
};

use super::parser::{MdTestCase, parse_mdtest_file};

/// Per-test timeout in seconds.
const TEST_TIMEOUT_SECONDS: u64 = 1;

#[derive(Debug, Default)]
pub struct MdtestSuite {
    tests: HashMap<String, MdTestCase>,
    cases: Vec<TestCase>,
}

impl MdtestSuite {
    pub fn load() -> Self {
        let mdtest_directory = fixtures_dir().join("mdtest");

        let md_files = discover_md_files(&mdtest_directory).unwrap_or_default();

        let mut suite = Self::default();
        for md_path in md_files {
            suite.add_file(&mdtest_directory, &md_path);
        }

        suite
    }

    fn add_file(&mut self, mdtest_directory: &Path, md_path: &Path) {
        let cases = match parse_mdtest_file(md_path) {
            Ok(cases) => cases,
            Err(error) => {
                eprintln!("failed to parse {}: {error}", md_path.display());
                return;
            }
        };

        let relative_path = md_path.strip_prefix(mdtest_directory).unwrap_or(md_path);
        let relative_name = relative_path.to_string_lossy();

        for case in cases {
            let name = format!(
                "{relative_name}/{}/{}",
                slug(&case.section),
                slug(&case.name),
            );
            let test_case = TestCase::file(name, md_path.to_path_buf(), "destack_test::mdtest");

            self.tests.insert(test_case.full_name(), case);
            self.cases.push(test_case);
        }
    }
}

impl Suite for MdtestSuite {
    fn name(&self) -> &'static str {
        "mdtest"
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

        run_mdtest_with_timeout(md_test, context.timeout)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(TEST_TIMEOUT_SECONDS))
    }
}

pub fn run_mdtests(options: &TestOptions) -> ExitCode {
    let suite = MdtestSuite::load();
    Runner::run_suite(&suite, options)
}

/// Run a single markdown test case with a timeout.
fn run_mdtest_with_timeout(test: &MdTestCase, timeout: Option<Duration>) -> TestResult {
    let timeout = timeout.unwrap_or(Duration::from_secs(TEST_TIMEOUT_SECONDS));
    let test = test.clone();

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = run_mdtest(&test);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => TestResult::Failed {
            message: format!(
                "test timed out after {TEST_TIMEOUT_SECONDS}s (likely deadlock or infinite loop)"
            ),
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => TestResult::Failed {
            message: "test thread panicked".to_string(),
        },
    }
}

/// Run a single markdown test case.
fn run_mdtest(test: &MdTestCase) -> TestResult {
    // set up an in-memory program with the test files
    let files = Arc::new(FileRegistry::new());
    let memory_fs = Arc::new(MemoryFileSystem::new());
    let cwd = PathBuf::from("/test");

    // create virtual files for all test files
    let mut main_path: Option<PathBuf> = None;

    for file in &test.files {
        let file_path = cwd.join(&file.path);
        memory_fs
            .add_file(&file_path, file.content.as_bytes())
            .expect("failed to add test file");

        // track which file is the main entry point:
        // - if named "main.ds", it's the main file
        // - otherwise, use the last file
        if file.path == "main.ds" || main_path.is_none() {
            main_path = Some(file_path);
        }
    }

    let main_path = main_path.expect("test should have at least one file");

    let fs: Arc<dyn FileSystem> = memory_fs;
    let program = Arc::new(Program::new(LanguageOptions::default(), cwd, fs, files));

    // compile the main file (this will pull in imports)
    let compiler = Compiler::new(
        program.clone(),
        CompileOptions {
            workers: 1,
            ..Default::default()
        },
    );

    let module_id = match compiler.resolve_path_to_module(&main_path) {
        Ok(id) => id,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to resolve module: {e:?}"),
            };
        }
    };

    compiler.enqueue(AnalyzeTask::AnalyzeModuleCheck { module: module_id });
    compiler.compile();
    drop(compiler);

    // collect actual error messages
    let diagnostics = program.diagnostics.collect();
    let actual_errors: Vec<String> = diagnostics
        .iter()
        .into_iter()
        .filter(|d| d.severity == destack_source::DiagnosticSeverity::Error)
        .map(|d| d.message.clone())
        .collect();

    // compare against expected errors
    compare_errors(&test.expected_errors, &actual_errors)
}

/// Compare expected and actual errors.
fn compare_errors(expected: &[String], actual: &[String]) -> TestResult {
    let expected_patterns: Vec<ExpectedError> =
        expected.iter().map(|s| ExpectedError::parse(s)).collect();
    let actual_normalized: Vec<String> = actual.iter().map(|s| normalize_error(s)).collect();

    // check for missing expected errors
    let mut missing: Vec<&str> = Vec::new();
    for expected in &expected_patterns {
        if !actual_normalized.iter().any(|a| expected.matches(a)) {
            missing.push(expected.as_str());
        }
    }

    // check for unexpected errors
    let mut unexpected: Vec<&str> = Vec::new();
    for act in &actual_normalized {
        if !expected_patterns.iter().any(|e| e.matches(act)) {
            unexpected.push(act.as_str());
        }
    }

    if missing.is_empty() && unexpected.is_empty() {
        return TestResult::Passed;
    }

    // build failure message
    let mut message = String::new();

    if !missing.is_empty() {
        message.push_str("missing expected errors:\n");
        for err in &missing {
            message.push_str(&format!("  - {err}\n"));
        }
    }

    if !unexpected.is_empty() {
        if !message.is_empty() {
            message.push('\n');
        }
        message.push_str("unexpected errors:\n");
        for err in &unexpected {
            message.push_str(&format!("  + {err}\n"));
        }
    }

    TestResult::Failed { message }
}

/// Normalize an error message for comparison.
fn normalize_error(s: &str) -> String {
    let s = s.trim().to_lowercase();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, Clone)]
enum ExpectedError {
    Exact(String),
    Contains(String),
}

impl ExpectedError {
    fn parse(raw: &str) -> Self {
        let raw = raw.trim();

        let lower = raw.to_lowercase();
        if let Some(rest) = lower.strip_prefix("contains:") {
            return Self::Contains(normalize_error(rest));
        }

        Self::Exact(normalize_error(raw))
    }

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

/// Discover markdown files recursively in a directory.
fn discover_md_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() {
        return Ok(files);
    }

    // first, get direct .md files (except README.md)
    let direct_files = discover_test_files(dir, &["md"], "mdtest")?;
    for test in direct_files {
        // skip README files
        if test.name.to_lowercase() == "readme" {
            continue;
        }
        files.push(test.path);
    }

    // then recurse into subdirectories
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // skip hidden directories, staging, and common non-test directories
            if !name.starts_with('.') && name != "node_modules" && name != "staging" {
                files.extend(discover_md_files(&path)?);
            }
        }
    }

    Ok(files)
}

/// Convert a test name to a slug (lowercase, spaces to hyphens).
fn slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
