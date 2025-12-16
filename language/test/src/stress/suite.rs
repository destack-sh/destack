use std::sync::Arc;
use std::time::Duration;

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler, ResolveTask};
use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageType, MemoryFileSystem, Uri,
};
use destack_workspace::{FormatterOptions, LinterOptions, Program, Session};

use crate::harness::{
    RunContext, Suite, TestCase, TestOptions, TestResult, discover_test_files, fixtures_dir,
};

// --- parser stress suite ---

/// Stress test suite for the parser (lexer + AST construction).
#[derive(Debug, Clone, Copy, Default)]
pub struct ParserStressSuite;

impl Suite for ParserStressSuite {
    fn name(&self) -> &'static str {
        "stress-parser"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let stress_dir = fixtures_dir().join("stress").join("parser");
        if !stress_dir.exists() {
            eprintln!("stress fixtures not found, run `just generate-stress`");
            return vec![];
        }

        let extensions = &["ds", "ts", "tsx", "js", "jsx"];
        discover_test_files(&stress_dir, extensions, "destack_test::stress::parser")
            .unwrap_or_default()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_parser_stress(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}

/// Run parser-only stress test.
fn run_parser_stress(test: &TestCase) -> TestResult {
    // determine file type
    let path_str = test.path.to_string_lossy();
    let file_type = if path_str.ends_with(".d.ds") {
        FileType::DestackDeclaration
    } else if path_str.ends_with(".d.ts") {
        FileType::TypeScriptDeclaration
    } else {
        let ext = test.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        FileType::from_extension(ext).unwrap_or(FileType::Destack)
    };

    // set up minimal context
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
        Ok(c) => c,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read: {e}"),
            };
        }
    };
    let file_size = content.len();
    let line_count = content.lines().count();
    let file_id = program.files.next_id();
    let name = test.path.file_name().unwrap().to_string_lossy().to_string();
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(test.path.clone()),
        file_type,
        content,
    );
    program.files.insert(file);
    let file = program.files.get(file_id);

    // parse only
    let start = std::time::Instant::now();
    let language_type = LanguageType::from(file.ty);
    let mut parser = Parser::lex_file(file, language_type);
    let _ast = parser.parse();
    let elapsed = start.elapsed();

    eprintln!("  {file_size} bytes, {line_count} lines, parsed in {elapsed:?}",);
    TestResult::Passed
}

// --- resolver stress suite ---

/// Stress test suite for the resolver (module resolution + name binding).
#[derive(Debug, Clone, Copy, Default)]
pub struct ResolverStressSuite;

impl Suite for ResolverStressSuite {
    fn name(&self) -> &'static str {
        "stress-resolver"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let stress_dir = fixtures_dir().join("stress").join("resolver");
        if !stress_dir.exists() {
            eprintln!("stress fixtures not found, run `just generate-stress`");
            return vec![];
        }

        // resolver tests are project-based (look for index.ds entry points)
        discover_project_entry_points(&stress_dir, "destack_test::stress::resolver")
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_resolver_stress(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}

/// Discover project entry points (index.ds files) in subdirectories.
fn discover_project_entry_points(dir: &std::path::Path, category: &str) -> Vec<TestCase> {
    let mut cases = vec![];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let index = path.join("index.ds");
                if index.exists() {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    cases.push(TestCase::file(name, index, category));
                }
            }
        }
    }
    cases
}

/// Run resolver stress test (parse + bind + resolve).
fn run_resolver_stress(test: &TestCase) -> TestResult {
    let project_dir = test.path.parent().unwrap();
    let start = std::time::Instant::now();

    // count files in project
    let file_count = std::fs::read_dir(project_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "ds"))
                .count()
        })
        .unwrap_or(0);

    // set up compiler with physical file system access to the project
    let session = Arc::new(Session::new(project_dir.to_path_buf()));
    let program = session.add_root(project_dir.to_path_buf());
    let compiler = Arc::new(Compiler::new(
        session.clone(),
        program.clone(),
        CompileOptions::default(),
    ));

    // resolve entry module
    let module_id = match compiler.resolve_path_to_module(&test.path) {
        Ok(id) => id,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to resolve module: {e:?}"),
            };
        }
    };

    // resolve schedules import + bind automatically
    compiler.enqueue(ResolveTask::ResolveModule { module: module_id });
    compiler.compile();

    let elapsed = start.elapsed();
    eprintln!("  {file_count} files, resolved in {elapsed:?}",);
    TestResult::Passed
}

// --- checker stress suite ---

/// Stress test suite for the type checker (full semantic analysis).
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckerStressSuite;

impl Suite for CheckerStressSuite {
    fn name(&self) -> &'static str {
        "stress-checker"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let stress_dir = fixtures_dir().join("stress").join("checker");
        if !stress_dir.exists() {
            eprintln!("stress fixtures not found, run `just generate-stress`");
            return vec![];
        }

        let extensions = &["ds"];
        discover_test_files(&stress_dir, extensions, "destack_test::stress::checker")
            .unwrap_or_default()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_checker_stress(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}

/// Run checker stress test (full compile through type checking).
fn run_checker_stress(test: &TestCase) -> TestResult {
    let start = std::time::Instant::now();

    // read file content for stats
    let content = match std::fs::read_to_string(&test.path) {
        Ok(c) => c,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read: {e}"),
            };
        }
    };
    let file_size = content.len();
    let line_count = content.lines().count();

    // set up compiler
    let cwd = test.path.parent().unwrap().to_path_buf();
    let session = Arc::new(Session::new(cwd.clone()));
    let program = session.add_root(cwd);
    let compiler = Arc::new(Compiler::new(
        session.clone(),
        program.clone(),
        CompileOptions::default(),
    ));

    // resolve module
    let module_id = match compiler.resolve_path_to_module(&test.path) {
        Ok(id) => id,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to resolve module: {e:?}"),
            };
        }
    };

    // analyze schedules import + bind + resolve automatically
    compiler.enqueue(AnalyzeTask::AnalyzeModule { module: module_id });
    compiler.compile();

    let elapsed = start.elapsed();
    eprintln!("  {file_size} bytes, {line_count} lines, checked in {elapsed:?}",);
    TestResult::Passed
}
