//! Parser smoke tests.

use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{
    File, FileId, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};
use destack_workspace::Program;

use crate::harness::{
    TestCase, TestOptions, TestResult, check_diagnostics, discover_test_files, fixtures_dir,
    run_tests,
};

/// Run all parser smoke tests.
pub fn run_parser_smoke_tests(options: &TestOptions) -> std::process::ExitCode {
    let smoke_dir = fixtures_dir().join("smoke").join("parser");
    let tests = discover_test_files(&smoke_dir, &["ds"], "destack_test::smoke::parser")
        .expect("failed to discover tests");
    run_tests(tests, options, run_parser_test)
}

/// Run a single parser smoke test.
fn run_parser_test(test: &TestCase) -> TestResult {
    // set up a minimal program for diagnostics
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

    // parse the file
    let mut parser = Parser::lex_file(file, program.language);
    let _expressions = parser.parse();
    program.diagnostics.merge_from(&parser.diagnostics);

    // check for unexpected diagnostics
    check_diagnostics(test, &program.files, &program.diagnostics)
}
