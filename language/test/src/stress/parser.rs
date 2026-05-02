use std::sync::Arc;
use std::time::Duration;

use destack_parser::Parser;
use destack_source::{File, FileId, FileSystem, FileType, LanguageType, MemoryFileSystem, Uri};
use destack_workspace::{FormatterOptions, LinterOptions};

use crate::core::{
    Case, CaseResult, RunContext, RunOptions, Suite, discover_file_cases, fixtures_dir,
    open_repository_with_options,
};

/// Stress test suite for the parser.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParserStressSuite;

impl Suite for ParserStressSuite {
    fn name(&self) -> &'static str {
        "stress-parser"
    }

    fn discover(&self, _options: &RunOptions) -> Vec<Case> {
        let stress_dir = fixtures_dir().join("stress").join("parser");
        if !stress_dir.exists() {
            eprintln!("stress fixtures not found, run `just generate-stress`");
            return vec![];
        }

        let extensions = &["ds", "ts", "tsx", "js", "jsx"];
        discover_file_cases(&stress_dir, extensions, "destack_test::stress::parser")
            .unwrap_or_default()
    }

    fn run(&self, case: &Case, _context: &RunContext<'_>) -> CaseResult {
        run_parser_stress(case)
    }

    fn timeout(&self) -> Option<Duration> {
        Some(Duration::from_secs(30))
    }
}

/// Run parser-only stress test.
fn run_parser_stress(test: &Case) -> CaseResult {
    // determine file type
    let path_str = test.path.to_string_lossy();
    let file_type = if path_str.ends_with(".d.ds") {
        FileType::DestackDeclaration
    } else if path_str.ends_with(".d.ts") {
        FileType::TypeScriptDeclaration
    } else {
        let extension = test
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        FileType::from_extension(extension).unwrap_or(FileType::Destack)
    };

    // set up minimal context
    let cwd = test.path.parent().unwrap().to_path_buf();
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let _repository = open_repository_with_options(
        cwd,
        fs,
        FormatterOptions::default(),
        LinterOptions::default(),
    );

    // load test file
    let uri = Uri::from_path(&test.path);
    let content = match std::fs::read_to_string(&test.path) {
        Ok(content) => content,
        Err(error) => {
            return CaseResult::Failed {
                message: format!("failed to read: {error}"),
            };
        }
    };
    let file_size = content.len();
    let line_count = content.lines().count();
    let name = test.path.file_name().unwrap().to_string_lossy().to_string();
    let file_id = FileId::from_logical_path(&test.path);
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(test.path.clone()),
        file_type,
        content,
    );
    let file = Arc::new(file);

    // parse only
    let start = std::time::Instant::now();
    let language_type = LanguageType::try_from(file.ty).expect("file type has no parser language");
    let mut parser = Parser::lex_file(file.clone(), language_type);
    let _ast = parser.parse();
    let elapsed = start.elapsed();

    eprintln!("  {file_size} bytes, {line_count} lines, parsed in {elapsed:?}");
    CaseResult::Passed
}
