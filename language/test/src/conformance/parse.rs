use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::Arc;

use destack_compiler::{
    AnalyzeError, AnalyzeTask, BindError, CompileOptions, Compiler, ImportError, ResolveError,
};
use destack_source::{DiagnosticSeverity, FileSystem, FileType, MemoryFileSystem, Uri};
use destack_workspace::Session;

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
    let fs: Arc<dyn FileSystem> = Arc::new(MemoryFileSystem::new());
    let session = Arc::new(Session::new(cwd.clone()).with_fs(fs));
    let program = session.add_root(cwd);

    // register module with the correct file type (important for JSX files with .js extension)
    let uri = Uri::from_path(path);
    let module_id = program.register_inline_module(uri, content.to_string(), file_type);

    // create compiler and compile the module
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompileOptions {
            workers: 1,
            ..Default::default()
        },
    );

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
