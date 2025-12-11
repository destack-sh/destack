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
    TestCase, TestOptions, TestResult, discover_test_files, fixtures_dir, run_tests,
};

use super::parser::{MdTestCase, parse_mdtest_file};

/// Per-test timeout in seconds.
const TEST_TIMEOUT_SECONDS: u64 = 1;

/// Run all markdown tests.
pub fn run_mdtests(options: &TestOptions) -> ExitCode {
    let mdtest_dir = fixtures_dir().join("mdtest");

    // discover all .md files in the mdtest directory (recursively)
    let md_files = discover_md_files(&mdtest_dir).expect("failed to discover mdtest files");
    if md_files.is_empty() {
        println!("no mdtest files found in {}", mdtest_dir.display());
        return ExitCode::SUCCESS;
    }

    // parse all markdown files and extract test cases
    let mut all_tests = Vec::new();
    for md_path in &md_files {
        let tests = match parse_mdtest_file(md_path) {
            Ok(tests) => tests,
            Err(e) => {
                eprintln!("failed to parse {}: {}", md_path.display(), e);
                continue;
            }
        };

        // convert MdTestCase to TestCase for the harness
        let file_stem = md_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        for md_test in tests {
            let test_name = format!("{}::{}", file_stem, slug(&md_test.name));
            let category = format!("destack_test::mdtest::{file_stem}");
            let case = MdTestCaseWrapper {
                test_case: TestCase::file(&test_name, md_path.clone(), &category),
                md_test,
            };
            all_tests.push(case);
        }
    }

    if all_tests.is_empty() {
        println!("no tests found in mdtest files");
        return ExitCode::SUCCESS;
    }

    // convert to plain TestCases for filtering/listing
    let test_cases: Vec<TestCase> = all_tests.iter().map(|w| w.test_case.clone()).collect();

    // create a lookup for the actual test data
    let test_map: HashMap<String, MdTestCase> = all_tests
        .into_iter()
        .map(|w| (w.test_case.full_name(), w.md_test))
        .collect();

    run_tests(test_cases, options, |test| {
        let md_test = test_map.get(&test.full_name()).expect("test not found");
        run_mdtest_with_timeout(md_test)
    })
}

/// Run a single markdown test case with a timeout.
fn run_mdtest_with_timeout(test: &MdTestCase) -> TestResult {
    let timeout = Duration::from_secs(TEST_TIMEOUT_SECONDS);
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

/// Wrapper to hold both TestCase and MdTestCase.
struct MdTestCaseWrapper {
    test_case: TestCase,
    md_test: MdTestCase,
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
    // normalize and compare
    let expected_normalized: Vec<String> = expected.iter().map(|s| normalize_error(s)).collect();
    let actual_normalized: Vec<String> = actual.iter().map(|s| normalize_error(s)).collect();

    // check for missing expected errors
    let mut missing: Vec<&str> = Vec::new();
    for exp in &expected_normalized {
        if !actual_normalized.iter().any(|a| error_matches(exp, a)) {
            missing.push(exp);
        }
    }

    // check for unexpected errors
    let mut unexpected: Vec<&str> = Vec::new();
    for act in &actual_normalized {
        if !expected_normalized.iter().any(|e| error_matches(e, act)) {
            unexpected.push(act);
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
    s.trim().to_lowercase()
}

/// Check if an expected error matches an actual error.
/// Uses substring matching to be flexible with error message formatting.
fn error_matches(expected: &str, actual: &str) -> bool {
    // exact match
    if expected == actual {
        return true;
    }

    // check if actual contains the expected message (substring match)
    if actual.contains(expected) {
        return true;
    }

    // check if expected contains key parts of actual:
    // this handles cases where expected is more specific
    if expected.contains(actual) {
        return true;
    }

    // check for common patterns like "type X is not assignable to type Y"
    // where X and Y might be formatted differently
    if expected.contains("not assignable") && actual.contains("not assignable") {
        // extract the types being compared and check if they match
        return types_match_in_assignability_error(expected, actual);
    }

    false
}

/// Check if two assignability error messages are about the same types.
fn types_match_in_assignability_error(expected: &str, actual: &str) -> bool {
    // simple heuristic: check if key type names appear in both
    // expected format: "type X is not assignable to type Y"
    // actual format: "type X is not assignable to type Y"

    // extract words that look like type names (capitalized or quoted)
    let expected_words: Vec<&str> = expected.split_whitespace().collect();
    let actual_words: Vec<&str> = actual.split_whitespace().collect();

    // find the type names (words after "type")
    let mut expected_types: Vec<&str> = Vec::new();
    let mut actual_types: Vec<&str> = Vec::new();

    for (i, word) in expected_words.iter().enumerate() {
        if *word == "type" && i + 1 < expected_words.len() {
            expected_types.push(expected_words[i + 1]);
        }
    }

    for (i, word) in actual_words.iter().enumerate() {
        if *word == "type" && i + 1 < actual_words.len() {
            actual_types.push(actual_words[i + 1]);
        }
    }

    // check if at least the key types match
    if expected_types.len() >= 2 && actual_types.len() >= 2 {
        return expected_types[0] == actual_types[0] && expected_types[1] == actual_types[1];
    }

    false
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
