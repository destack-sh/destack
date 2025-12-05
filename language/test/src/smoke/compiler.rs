//! Compiler smoke tests.

use std::sync::Arc;

use destack_compiler::{CompileOptions, Compiler, ImportTask};
use destack_dir::Program;
use destack_source::{
    File, FileId, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};

use crate::harness::{
    check_diagnostics, discover_test_files, fixtures_dir, run_tests, TestCase, TestOptions, TestResult,
};

/// Run all compiler smoke tests.
pub fn run_compiler_smoke_tests(options: &TestOptions) -> std::process::ExitCode {
    let smoke_dir = fixtures_dir().join("smoke").join("compiler");
    let tests = discover_test_files(&smoke_dir, &["ds"], "destack_test::smoke::compiler")
        .expect("failed to discover tests");
    run_tests(tests, options, run_compiler_test)
}

/// Run a single compiler smoke test.
fn run_compiler_test(test: &TestCase) -> TestResult {
    // set up a program
    let cwd = test.path.parent().unwrap().to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::new(LanguageOptions::default(), cwd, fs, files));

    // load the file
    let uri = Uri::from_path(&test.path);
    let file_type = if test.path.to_string_lossy().ends_with(".d.ds") {
        FileType::DestackDeclaration
    } else {
        FileType::Destack
    };
    let content = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };
    let file_id = FileId::new(0);
    let name = test.path.file_name().unwrap().to_string_lossy().to_string();
    let path = Some(test.path.clone());
    let file = File::from_text(file_id, name, uri, path, file_type, content);
    program.files.insert(file);
    let file = program.files.get(file_id);

    // compile the file
    let compiler = Compiler::new(
        program.clone(),
        CompileOptions {
            workers: 1,
            ..Default::default()
        },
    );
    compiler.enqueue(ImportTask::ImportModuleFromFile { file: file.id });
    compiler.compile();
    drop(compiler);

    // check for unexpected diagnostics
    check_diagnostics(test, &program.files, &program.diagnostics)
}
