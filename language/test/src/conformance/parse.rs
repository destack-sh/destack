use std::path::Path;
use std::sync::Arc;

use destack_compiler::{BindTask, CompileOptions, Compiler};
use destack_source::{
    DiagnosticSeverity, FileRegistry, FileSystem, FileType, LanguageOptions, LanguageType,
    MemoryFileSystem,
};
use destack_workspace::Program;

/// Outcome of checking a file for conformance testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParseOutcome {
    /// File parsed and bound without errors.
    Ok,
    /// File had errors (parse or bind).
    Error,
}

/// Options for parsing in conformance tests.
#[derive(Debug, Clone, Default)]
pub(super) struct ParseOptions {}

/// Parse and bind a file, return the outcome.
pub(super) fn parse_file(
    path: &Path,
    content: &str,
    file_type: FileType,
    _options: ParseOptions,
) -> ParseOutcome {
    let cwd = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let files = Arc::new(FileRegistry::new());
    let memory_fs = Arc::new(MemoryFileSystem::new());
    memory_fs
        .add_file(path, content.as_bytes())
        .expect("failed to add file to memory fs");
    let fs: Arc<dyn FileSystem> = memory_fs;

    // set language type based on file type for proper compatibility mode
    let language_type = LanguageType::from(file_type);
    let language = LanguageOptions::default().with_type(language_type);
    let program = Arc::new(Program::new(language, cwd, fs, files));

    // create compiler and resolve module
    let compiler = Compiler::new(
        program.clone(),
        CompileOptions {
            workers: 1,
            ..Default::default()
        },
    );

    let module_id = match compiler.resolve_path_to_module(&path.to_path_buf()) {
        Ok(id) => id,
        Err(_) => return ParseOutcome::Error,
    };

    // run import (parse) and bind phases (including validation)
    compiler.enqueue(BindTask::BindModuleValidate { module: module_id });
    compiler.compile();

    // check for any errors (parse or bind)
    if program.diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
        return ParseOutcome::Error;
    }

    ParseOutcome::Ok
}
