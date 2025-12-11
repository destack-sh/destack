use std::sync::Arc;

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler};
use destack_source::{FileRegistry, FileSystem, LanguageOptions, MemoryFileSystem};
use destack_workspace::Program;

use crate::harness::{
    RunContext, Runner, Suite, TestCase, TestOptions, TestResult, check_diagnostics,
    discover_test_files, fixtures_dir,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct CompilerSmokeSuite;

impl Suite for CompilerSmokeSuite {
    fn name(&self) -> &'static str {
        "smoke-compiler"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let smoke_directory = fixtures_dir().join("smoke").join("compiler");
        discover_test_files(&smoke_directory, &["ds"], "destack_test::smoke::compiler")
            .expect("failed to discover tests")
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_compiler_case(case)
    }
}

/// Run all compiler smoke tests.
pub fn run_compiler_smoke_tests(options: &TestOptions) -> std::process::ExitCode {
    Runner::run_suite(&CompilerSmokeSuite, options)
}

/// Run a single compiler smoke test.
fn run_compiler_case(test: &TestCase) -> TestResult {
    // read file content from disk
    let content = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };

    // set up program with memory filesystem containing the test file
    let cwd = test.path.parent().unwrap().to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let memory_fs = Arc::new(MemoryFileSystem::new());
    memory_fs
        .add_file(&test.path, content.as_bytes())
        .expect("failed to add test file to memory fs");
    let fs: Arc<dyn FileSystem> = memory_fs;
    let program = Arc::new(Program::new(LanguageOptions::default(), cwd, fs, files));

    // compile the file
    let compiler = Compiler::new(
        program.clone(),
        CompileOptions {
            workers: 1,
            ..Default::default()
        },
    );
    let module_id = match compiler.resolve_path_to_module(&test.path) {
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

    // check for unexpected diagnostics
    check_diagnostics(test, &program.files, &program.diagnostics)
}
