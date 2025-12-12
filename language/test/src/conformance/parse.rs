use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler};
use destack_source::{
    DiagnosticSeverity, FileRegistry, FileSystem, FileType, LanguageOptions, LanguageType,
    MemoryFileSystem,
};
use destack_workspace::Program;

/// Outcome of checking a file for conformance testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ParseOutcome {
    /// File parsed without relevant errors.
    Ok,
    /// File had relevant errors.
    Error,
}

/// What category of errors a test cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum TestArea {
    /// Only parse/import errors (EI*) - pure grammar conformance.
    Parse,
    /// Parse + bind + flow errors (EI*, EB*, EA021-EA023) - early error conformance.
    #[default]
    Early,
}

impl TestArea {
    /// Check if an error code is relevant for this test area.
    ///
    /// Error code prefixes:
    /// - EP*: Parse errors (from parser)
    /// - EI*: Import errors (module resolution, parse wrapper)
    /// - EB*: Bind errors (structural/declaration conflicts)
    /// - EA*: Analyze errors (type inference, flow validation)
    pub(super) fn is_relevant_error(&self, code: &str) -> bool {
        match self {
            // Parse-only: grammar errors
            TestArea::Parse => code.starts_with("EP") || code.starts_with("EI"),
            // Early: grammar + early errors (bind, flow)
            TestArea::Early => {
                code.starts_with("EP")
                    || code.starts_with("EI")
                    || code.starts_with("EB")
                    || code == "EA021"
                    || code == "EA022"
                    || code == "EA023"
            }
        }
    }
}

/// Options for parsing in conformance tests.
#[derive(Debug, Clone, Default)]
pub(super) struct ParseOptions {
    /// What category of errors to check for.
    pub area: TestArea,
}

/// Parse and bind a file, return the outcome.
pub(super) fn parse_file(
    path: &Path,
    content: &str,
    file_type: FileType,
    options: ParseOptions,
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

    // run import, bind, resolve, and analyze (infer) phases
    // flow validation (break/continue) is done in the analyze infer phase
    // catch panics to treat them as errors (some malformed code causes panics)
    compiler.enqueue(AnalyzeTask::AnalyzeModuleInfer { module: module_id });
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        compiler.compile();
    }));

    // panics during compilation are treated as errors
    if result.is_err() {
        return ParseOutcome::Error;
    }

    // check for errors relevant to the test area
    let has_relevant_error = program.diagnostics.iter().into_iter().any(|d| {
        d.severity == DiagnosticSeverity::Error && options.area.is_relevant_error(&d.code)
    });

    if has_relevant_error {
        ParseOutcome::Error
    } else {
        ParseOutcome::Ok
    }
}
