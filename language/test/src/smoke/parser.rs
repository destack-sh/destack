use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{File, FileId, FileSystem, FileType, LanguageType, MemoryFileSystem, Uri};
use destack_workspace::{FormatterOptions, LinterOptions};

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Runner, Suite, check_diagnostics,
    discover_file_cases, fixtures_dir, open_repository_with_options,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ParserSmokeSuite;

impl Suite for ParserSmokeSuite {
    fn name(&self) -> &'static str {
        "smoke-parser"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let smoke_directory = fixtures_dir().join("smoke").join("parser");

        // support all language file extensions
        let extensions = &["ds", ".d.ds", "ts", ".d.ts", "tsx", "js", "jsx"];

        discover_file_cases(&smoke_directory, extensions, "destack_test::smoke::parser")
            .expect("failed to discover tests")
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        run_parser_case(case)
    }
}

/// Run all parser smoke tests.
pub fn run_parser_smoke_tests(options: &RunOptions) -> std::process::ExitCode {
    Runner::run_suite(ParserSmokeSuite, options)
}

/// Run a single parser smoke test.
fn run_parser_case(test: &Case) -> CaseResult {
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

    // set up one minimal repository context for path normalization
    let cwd = test.path.parent().unwrap().to_path_buf();
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let _repository = open_repository_with_options(
        cwd,
        fs,
        FormatterOptions::default(),
        LinterOptions::default(),
    );

    // load the file
    let uri = Uri::from_path(&test.path);
    let content = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(e) => {
            return CaseResult::Failed {
                message: format!("failed to read file: {e}"),
            };
        }
    };
    let name = test.path.file_name().unwrap().to_string_lossy().to_string();
    let path = Some(test.path.clone());
    let file_id = FileId::from_logical_path(&test.path);
    let file = Arc::new(File::from_text(
        file_id, name, uri, path, file_type, content,
    ));
    let file_for_id = |current_file_id| {
        if current_file_id == file_id {
            Some(file.clone())
        } else {
            None
        }
    };

    // parse the file
    let language_type = LanguageType::try_from(file.ty).expect("file type has no parser language");
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let _expressions = parser.parse();

    // check for unexpected diagnostics
    check_diagnostics(test, &file_for_id, &parser.diagnostics)
}
