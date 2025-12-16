use std::sync::Arc;
use std::time::Duration;

use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageType, MemoryFileSystem, Uri,
};
use destack_workspace::{FormatterOptions, LinterOptions, Program};

use crate::harness::{
    RunContext, Suite, TestCase, TestOptions, TestResult, discover_test_files, fixtures_dir,
};

/// Stress test suite for large files.
#[derive(Debug, Clone, Copy, Default)]
pub struct LargeFilesStressSuite;

impl Suite for LargeFilesStressSuite {
    fn name(&self) -> &'static str {
        "stress-large-files"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let stress_dir = fixtures_dir().join("stress").join("large_files");

        if !stress_dir.exists() {
            eprintln!(
                "Stress fixtures not found. Run `just generate-stress` to generate them."
            );
            return vec![];
        }

        let extensions = &["ds", "ts", "tsx", "js", "jsx"];
        discover_test_files(&stress_dir, extensions, "destack_test::stress::large_files")
            .unwrap_or_default()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_stress_case(case)
    }

    fn timeout(&self) -> Option<Duration> {
        // 5 minutes per test
        Some(Duration::from_secs(300))
    }
}

/// Stress test suite for large projects (many files).
#[derive(Debug, Clone, Copy, Default)]
pub struct LargeProjectsStressSuite;

impl Suite for LargeProjectsStressSuite {
    fn name(&self) -> &'static str {
        "stress-large-projects"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let stress_dir = fixtures_dir().join("stress").join("large_projects");

        if !stress_dir.exists() {
            eprintln!(
                "Stress fixtures not found. Run `just generate-stress` to generate them."
            );
            return vec![];
        }

        // test entry points (index.ds files)
        discover_project_entry_points(&stress_dir)
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_stress_case(case)
    }

    fn timeout(&self) -> Option<Duration> {
        // 10 minutes for multi-file projects
        Some(Duration::from_secs(600))
    }
}

/// Stress test suite for memory pressure scenarios.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStressSuite;

impl Suite for MemoryStressSuite {
    fn name(&self) -> &'static str {
        "stress-memory"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let stress_dir = fixtures_dir().join("stress").join("memory");

        if !stress_dir.exists() {
            eprintln!(
                "Stress fixtures not found. Run `just generate-stress` to generate them."
            );
            return vec![];
        }

        let extensions = &["ds", "ts", "tsx", "js", "jsx"];
        discover_test_files(&stress_dir, extensions, "destack_test::stress::memory")
            .unwrap_or_default()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_stress_case(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(300))
    }
}

/// Stress test suite for edge cases.
#[derive(Debug, Clone, Copy, Default)]
pub struct EdgeCasesStressSuite;

impl Suite for EdgeCasesStressSuite {
    fn name(&self) -> &'static str {
        "stress-edge-cases"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let stress_dir = fixtures_dir().join("stress").join("edge_cases");

        if !stress_dir.exists() {
            eprintln!(
                "Stress fixtures not found. Run `just generate-stress` to generate them."
            );
            return vec![];
        }

        let extensions = &["ds", "ts", "tsx", "js", "jsx"];
        discover_test_files(&stress_dir, extensions, "destack_test::stress::edge_cases")
            .unwrap_or_default()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_stress_case(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(60))
    }
}

/// Stress test suite for pathological inputs (adversarial, malformed).
#[derive(Debug, Clone, Copy, Default)]
pub struct PathologicalStressSuite;

impl Suite for PathologicalStressSuite {
    fn name(&self) -> &'static str {
        "stress-pathological"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let stress_dir = fixtures_dir().join("stress").join("pathological");

        if !stress_dir.exists() {
            eprintln!(
                "Stress fixtures not found. Run `just generate-stress` to generate them."
            );
            return vec![];
        }

        let extensions = &["ds", "ts", "tsx", "js", "jsx"];
        discover_test_files(&stress_dir, extensions, "destack_test::stress::pathological")
            .unwrap_or_default()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_stress_case(case)
    }

    fn timeout(&self) -> Option<Duration> {
        // longer timeout for pathological cases that may be slow to parse
        Some(Duration::from_secs(120))
    }
}

/// Discover project entry points (index.ds files) in subdirectories.
fn discover_project_entry_points(dir: &std::path::Path) -> Vec<TestCase> {
    let mut cases = vec![];

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let index = path.join("index.ds");
                if index.exists() {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    cases.push(TestCase::file(
                        name,
                        index,
                        "destack_test::stress::large_projects",
                    ));
                }
            }
        }
    }

    cases
}

/// Run a single stress test case.
///
/// Stress tests verify:
/// 1. The toolchain doesn't crash or panic
/// 2. Memory usage stays bounded (checked externally)
/// 3. Completion within timeout (enforced by harness)
fn run_stress_case(test: &TestCase) -> TestResult {
    // determine file type from extension
    let path_str = test.path.to_string_lossy();
    let file_type = if path_str.ends_with(".d.ds") {
        FileType::DestackDeclaration
    } else if path_str.ends_with(".d.ts") {
        FileType::TypeScriptDeclaration
    } else {
        let ext = test.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        FileType::from_extension(ext).unwrap_or(FileType::Destack)
    };

    // set up program context
    let cwd = test.path.parent().unwrap().to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::from_options(
        FormatterOptions::default(),
        LinterOptions::default(),
        cwd,
        fs,
        files,
    ));

    // load test file
    let uri = Uri::from_path(&test.path);
    let content = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };
    let file_size = content.len();
    let line_count = content.lines().count();
    let file_id = program.files.next_id();
    let name = test.path.file_name().unwrap().to_string_lossy().to_string();
    let path = Some(test.path.clone());
    let file = File::from_text(file_id, name, uri, path, file_type, content);
    program.files.insert(file);
    let file = program.files.get(file_id);

    // parse: the main stress test (just verify completion without crash)
    let start = std::time::Instant::now();
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file, language_type);
    let _expressions = parser.parse();
    let elapsed = start.elapsed();

    eprintln!(
        "  {} bytes, {} lines, parsed in {:?}",
        file_size, line_count, elapsed
    );
    TestResult::Passed
}
