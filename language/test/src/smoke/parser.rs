//! Parser smoke tests.

use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, MemoryFileSystem, Uri,
};
use destack_workspace::Program;

use crate::harness::{
    TestCase, TestOptions, TestResult, check_diagnostics, discover_test_files, fixtures_dir,
    run_tests,
};

/// Run all parser smoke tests.
pub fn run_parser_smoke_tests(options: &TestOptions) -> std::process::ExitCode {
    let smoke_dir = fixtures_dir().join("smoke").join("parser");
    // support all language file extensions
    let extensions = &["ds", ".d.ds", "ts", ".d.ts", "tsx", "js", "jsx"];
    let tests = discover_test_files(&smoke_dir, extensions, "destack_test::smoke::parser")
        .expect("failed to discover tests");
    run_tests(tests, options, run_parser_test)
}

/// Run a single parser smoke test.
fn run_parser_test(test: &TestCase) -> TestResult {
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

    // set up language options based on file type
    let language_type = destack_source::LanguageType::from(file_type);
    let language = LanguageOptions::default().with_type(language_type);

    // set up a minimal program for diagnostics
    let cwd = test.path.parent().unwrap().to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let program = Arc::new(Program::new(language, cwd, fs, files));

    // load the file
    let uri = Uri::from_path(&test.path);
    let content = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };
    let file_id = program.files.next_id();
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
