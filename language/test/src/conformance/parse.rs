use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_compiler::{
    AnalyzeError, AnalyzeTask, BindError, Compiler, CompilerOptions, ImportError, ResolveError,
};
use destack_source::{DiagnosticSeverity, FileType, MemoryFileSystem, Uri};
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

#[derive(Debug)]
struct SharedConformanceEnvironment {
    /// The shared test session.
    session: Arc<Session>,
    /// The shared in-memory file system.
    fs: Arc<MemoryFileSystem>,
    /// The next unique test id.
    next_id: AtomicUsize,
}

impl SharedConformanceEnvironment {
    /// Create a new shared environment for conformance tests.
    fn new() -> Self {
        let fs = Arc::new(MemoryFileSystem::new());
        let cwd = PathBuf::from("/test/conformance");
        let session = Arc::new(Session::new(cwd.clone()).with_fs(fs.clone()));
        Self {
            session,
            fs,
            next_id: AtomicUsize::new(0),
        }
    }

    /// Allocate a unique root directory for a test case.
    fn root_for(&self, path: &Path) -> PathBuf {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("case");
        PathBuf::from("/test/conformance").join(format!("{stem}-{id}"))
    }

    /// Build a synthetic file path for a test case.
    fn file_for(&self, root: &Path, path: &Path) -> PathBuf {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("case.js");
        root.join(name)
    }
}

thread_local! {
    static SHARED_CONFORMANCE_ENV: SharedConformanceEnvironment =
        SharedConformanceEnvironment::new();
}

/// Parse and bind a file, return the outcome.
pub(super) fn parse_file(
    path: &Path,
    content: &str,
    file_type: FileType,
    options: ParseOptions,
) -> ParseOutcome {
    let (session, program, file_path) = SHARED_CONFORMANCE_ENV.with(|env| {
        let root = env.root_for(path);
        let file_path = env.file_for(&root, path);
        let session = env.session.clone();
        let program = session.add_root(root);

        env.fs
            .add_file(&file_path, content.as_bytes())
            .expect("failed to add test file");

        (session, program, file_path)
    });

    // register module with the correct file type (important for JSX files with .js extension)
    let uri = Uri::from_path(&file_path);
    let module_id = program.register_inline_module(uri, content.to_string(), file_type);

    // create compiler and compile the module
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );

    // run up to analyze
    let profile = program.default_profile_id_for_module(module_id);
    compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate {
        module: module_id,
        profile,
    });
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
