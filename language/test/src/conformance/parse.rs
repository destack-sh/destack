use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::Arc;

use destack_compiler::{
    AnalyzeError, AnalyzeTask, BindError, CompileOptions, Compiler, ImportError, ResolveError,
};
use destack_source::{
    DiagnosticSeverity, FileRegistry, FileSystem, FileType, LanguageType, MemoryFileSystem,
};
use destack_workspace::{LanguageOptions, Program};

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
    /// Pure grammar-level conformance.
    Parse,
    /// "Early" error semi-semantic conformance.
    #[default]
    Early,
}

/// Early analysis error codes relevant for conformance testing.
const EARLY_ANALYZE_CODES: &[&str] = &[
    AnalyzeError::ALL_CODES[20], // EA020: InvalidLineage
    AnalyzeError::ALL_CODES[21], // EA021: InvalidBreak
    AnalyzeError::ALL_CODES[22], // EA022: InvalidContinue
    AnalyzeError::ALL_CODES[23], // EA023: InvalidAwait
    AnalyzeError::ALL_CODES[24], // EA024: InvalidYield
    AnalyzeError::ALL_CODES[25], // EA025: InvalidReturn
    AnalyzeError::ALL_CODES[26], // EA026: InvalidConstructor
    AnalyzeError::ALL_CODES[27], // EA027: InvalidInterface
    AnalyzeError::ALL_CODES[28], // EA028: InvalidFunction
    AnalyzeError::ALL_CODES[29], // EA029: InvalidMethod
    AnalyzeError::ALL_CODES[30], // EA030: InvalidMemberModifier
    AnalyzeError::ALL_CODES[31], // EA031: InvalidParameterProperty
    AnalyzeError::ALL_CODES[32], // EA032: InvalidStaticBlockModifier
];

/// Early resolve error codes relevant for conformance testing.
const EARLY_RESOLVE_CODES: &[&str] = &[
    ResolveError::ALL_CODES[10], // ER010: MissingTarget
    ResolveError::ALL_CODES[11], // ER011: InvalidTarget
];

impl TestArea {
    /// Check if an error code is relevant for this test area.
    pub(super) fn is_relevant_error(&self, code: &str) -> bool {
        match self {
            // parse errors are EP (parser) and EI (import, includes file loading)
            TestArea::Parse => code.starts_with("EP") || ImportError::is_valid_code(code),
            // early errors include parse + bind + specific resolve/analyze codes
            TestArea::Early => {
                code.starts_with("EP")
                    || ImportError::is_valid_code(code)
                    || BindError::is_valid_code(code)
                    || EARLY_RESOLVE_CODES.contains(&code)
                    || EARLY_ANALYZE_CODES.contains(&code)
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
    let program = Arc::new(Program::new(
        language,
        FormatterOptions::default(),
        LinterOptions::default(),
        cwd,
        fs,
        files,
    ));

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

    // run up to analyze
    compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate { module: module_id });
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
