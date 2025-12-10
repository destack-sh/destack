//! Shared parsing utilities for conformance tests.

use std::path::Path;
use std::sync::Arc;

use destack_ast::TokenType;
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
pub(super) struct ParseOptions {
    /// Whether to treat HTML comments as errors (for ES modules).
    pub reject_html_comments: bool,
}

/// Parse a file and return the outcome.
pub(super) fn parse_file(
    path: &Path,
    content: &str,
    file_type: FileType,
    options: ParseOptions,
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

    // Check for HTML comments if requested (ES modules forbid them) #ModuleTypeHandling
    if options.reject_html_comments {
        let has_html_comment = parser
            .side_tokens
            .iter()
            .any(|t| t.token.ty == TokenType::HtmlComment);
        if has_html_comment {
            return ParseOutcome::Error;
        }
    }

    ParseOutcome::Ok
}

/// Determine the file type from a path's extension and directory context.
pub(super) fn file_type_from_path(path: &Path) -> FileType {
    let path_str = path.to_string_lossy();

    // check if test is in a tsx or jsx directory (some suites enable JSX based on directory)
    // nocheckin TODO #Suspicious: language type should be determined per-suite?
    let in_tsx_dir = path_str.contains("/tsx/") || path_str.contains("/tsx-");
    let in_jsx_dir = path_str.contains("/jsx/") || path_str.contains("/jsx-");

    if path_str.ends_with(".tsx") {
        FileType::TypeScriptXml
    } else if path_str.ends_with(".ts") {
        if in_tsx_dir {
            FileType::TypeScriptXml
        } else {
            FileType::TypeScript
        }
    } else if path_str.ends_with(".jsx") || in_jsx_dir {
        FileType::JavaScriptXml
    } else {
        FileType::JavaScript
    }
}

/// Check if a path indicates an ES module (by naming convention).
pub(super) fn is_module_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.contains(".module."))
}
