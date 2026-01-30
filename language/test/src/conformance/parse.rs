use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, ImportError, ResolveError};
use destack_parser::Parser;
use destack_source::{
    DiagnosticSeverity, File, FileId, FileType, LanguageType, MemoryFileSystem, ModuleStamp,
    ProfileStamp, Uri,
};
use destack_workspace::{MemoryCacheStore, Session};

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
    #[default]
    Parse,
    /// Grammar + early syntax validation (analysis-backed, no resolve errors).
    EarlySyntax,
    /// "Early" error semi-semantic conformance.
    Early,
}

/// Early analysis error codes relevant for syntax-level conformance.
const EARLY_SYNTAX_ANALYZE_CODES: &[&str] = &[
    "EA214", // ReservedIdentifier
    "EA300", // InvalidBreak
    "EA301", // InvalidContinue
    "EA302", // InvalidAwait
    "EA303", // InvalidYield
    "EA304", // InvalidReturn
    "EA501", // InvalidConstructor
    "EA502", // InvalidInterface
    "EA503", // InvalidFunction
    "EA504", // InvalidMethod
    "EA505", // InvalidMemberModifier
    "EA506", // InvalidParameterProperty
    "EA507", // InvalidStaticBlockModifier
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
            // early syntax errors include parse + import + targeted analyze codes
            TestArea::EarlySyntax => {
                code.starts_with("EP")
                    || ImportError::is_valid_code(code)
                    || EARLY_SYNTAX_ANALYZE_CODES.contains(&code)
            }
            // early errors include parse + import + specific resolve/analyze codes
            TestArea::Early => {
                code.starts_with("EP")
                    || ImportError::is_valid_code(code)
                    || EARLY_RESOLVE_CODES.contains(&code)
                    || EARLY_SYNTAX_ANALYZE_CODES.contains(&code)
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
        let session = Arc::new(
            Session::new(cwd.clone())
                .with_fs(fs.clone())
                .with_cache_store(Arc::new(MemoryCacheStore::new())),
        );
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
    // select the parsing pipeline for this conformance area
    match options.area {
        // parse only tests should not pay the cost of full compiler analysis
        TestArea::Parse => parse_file_with_parser(path, content, file_type, options.area),
        // early error tests still need compiler checks
        TestArea::EarlySyntax | TestArea::Early => {
            parse_file_with_compiler(path, content, file_type, options.area)
        }
    }
}

fn parse_file_with_parser(
    path: &Path,
    content: &str,
    file_type: FileType,
    area: TestArea,
) -> ParseOutcome {
    // build a synthetic file for parser only diagnostics
    let uri = Uri::from_path(path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("case.js")
        .to_string();
    let file = Arc::new(File::from_text(
        FileId::new(0),
        name,
        uri,
        Some(path.to_path_buf()),
        file_type,
        content.to_string(),
    ));

    // parse and collect diagnostics
    let language = LanguageType::from(file_type);
    let mut parser = Parser::lex_file(file, language);
    let _ = parser.parse();

    // check for errors relevant to the test area
    let has_relevant_error = parser
        .diagnostics
        .iter()
        .into_iter()
        .any(|d| d.severity == DiagnosticSeverity::Error && area.is_relevant_error(&d.code));

    if has_relevant_error {
        ParseOutcome::Error
    } else {
        ParseOutcome::Ok
    }
}

fn parse_file_with_compiler(
    path: &Path,
    content: &str,
    file_type: FileType,
    area: TestArea,
) -> ParseOutcome {
    let (session, program, file_path) = SHARED_CONFORMANCE_ENV.with(|env| {
        // allocate a fresh test root and file path
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
            follow_imports: false,
            inject_prelude: false,
            load_libs: false,
            source_map: false,
            elaborate_with_ternary: false,
            elaborate_split_declarators: false,
            elaborate_explicit_return: false,
            emit_overwrite: false,
            emit_create_dirs: false,
            emit_dry_run: true,
            ..Default::default()
        },
    );

    // run up to analyze
    let profile = program.default_profile_id_for_module(module_id);
    let module_version = program.modules.get(module_id).read().version;
    let profile_version = program
        .profiles
        .get(profile)
        .unwrap_or_else(|| panic!("missing profile data for {profile:?}"))
        .version;
    compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate {
        module: ModuleStamp::new(module_id, module_version),
        profile: ProfileStamp::new(profile, profile_version),
    });
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        compiler.compile();
    }));

    // panics during compilation are treated as errors
    if result.is_err() {
        return ParseOutcome::Error;
    }

    // check for errors relevant to the test area
    let has_relevant_error = program
        .diagnostics
        .iter()
        .into_iter()
        .any(|d| d.severity == DiagnosticSeverity::Error && area.is_relevant_error(&d.code));

    if has_relevant_error {
        ParseOutcome::Error
    } else {
        ParseOutcome::Ok
    }
}
