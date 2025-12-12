use std::path::Path;
use std::sync::Arc;

use destack_parser::Parser;
use destack_source::{
    File, FileRegistry, FileSystem, FileType, LanguageOptions, LanguageType, MemoryFileSystem, Uri,
};
use destack_workspace::Program;

/// Outcome of parsing a file for conformance testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParseOutcome {
    /// File parsed without errors.
    Ok,
    /// File had parse errors.
    Error,
}

/// Options for parsing in conformance tests.
#[derive(Debug, Clone, Default)]
pub(super) struct ParseOptions {}

/// Parse a file and return the outcome.
pub(super) fn parse_file(
    path: &Path,
    content: &str,
    file_type: FileType,
    _options: ParseOptions,
) -> ParseOutcome {
    let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());

    // set language type based on file type for proper compatibility mode
    let language_type = LanguageType::from(file_type);
    let language = LanguageOptions::default().with_type(language_type);
    let program = Arc::new(Program::new(language, cwd, fs, files));

    let uri = Uri::from_path(path);
    let file_id = program.files.next_id();
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let file = File::from_text(
        file_id,
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        content.to_string(),
    );
    program.files.insert(file);
    let file = program.files.get(file_id);

    let mut parser = Parser::lex_file(file, program.language);
    let _ = parser.parse();

    // Check for parse errors
    if !parser.diagnostics.is_empty() {
        return ParseOutcome::Error;
    }

    ParseOutcome::Ok
}
