use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_artifact::{ArtifactKey, MemoryCacheStore};
use destack_compiler::{Compiler, ImportError};
use destack_parser::{Parser, ParserOptions};
use destack_source::{
    DiagnosticSeverity, File, FileContent, FileId, FileSystem, FileType, LanguageType,
    MemoryFileSystem, ModuleId, Uri,
};
use destack_workspace::{HostEnvironment, Repository, Revision};

use crate::core::{
    default_profile_id_for_module, module_artifact_diagnostics, module_id_for_path,
    provide_workspace_artifacts, write_workspace_file, write_workspace_text_file,
};

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
    "EA500", // InvalidLineage
    "EA214", // ReservedIdentifier
    "EA215", // ObjectLiteralDefault
    "EA216", // ObjectPatternMultipleSpreads
    "EA217", // ObjectPatternSpreadNotLast
    "EA235", // ObjectPatternRestNotIdentifier
    "EA238", // DuplicateDefaultExport
    "EA239", // InvalidPrivateIdentifier
    "EA240", // InvalidExponentLeftUnary
    "EA241", // EmptyParenthesizedExpression
    "EA242", // InvalidOptionalChainTemplate
    "EA243", // DuplicateLabel
    "EA244", // InvalidSuperCall
    "EA245", // InvalidSuperOptionalChain
    "EA246", // InvalidCatchAnnotationType
    "EA247", // InvalidTypeImportTarget
    "EA248", // InvalidNewTarget
    "EA249", // UnsupportedObjectPrototypeSetter
    "EA250", // InvalidNewOptionalChain
    "EA218", // ExportNamespaceOutsideDeclaration
    "EA219", // InvalidTypeParameterModifier
    "EA220", // InvalidReadonlyType
    "EA221", // InvalidTupleElementOrder
    "EA222", // InvalidIntrinsicTypeIndex
    "EA223", // MissingDestructuringInitializer
    "EA224", // InvalidDeclareInitializer
    "EA225", // InvalidTypeOnlyImportBindings
    "EA226", // InvalidAssignmentTarget
    "EA228", // TypeScriptSyntaxInJavaScript
    "EA229", // InvalidTypeOnlyImportAlias
    "EA230", // MissingConstInitializer
    "EA231", // InvalidAmbientConstInitializer
    "EA232", // InvalidDefiniteAssignmentDeclarator
    "EA233", // InvalidImportAliasTarget
    "EA234", // InvalidInstantiationAccess
    "EA403", // InvalidPatternNamedField
    "EA300", // InvalidBreak
    "EA301", // InvalidContinue
    "EA302", // InvalidAwait
    "EA303", // InvalidYield
    "EA304", // InvalidReturn
    "EA307", // IncompleteTry
    "EA322", // InvalidForOfBinding
    "EA323", // InvalidCatchBinding
    "EA501", // InvalidConstructor
    "EA502", // InvalidInterface
    "EA503", // InvalidFunction
    "EA504", // InvalidMethod
    "EA505", // InvalidMemberModifier
    "EA506", // InvalidParameterProperty
    "EA507", // InvalidStaticBlockModifier
    "EA512", // InvalidOptionalPatternParameter
    "EA513", // InvalidOptionalRestParameter
    "EA701", // InvalidDecoratorStaticArguments
    "EA821", // InvalidStrictDelete
];

/// Early resolve error codes relevant for conformance testing.
const EARLY_RESOLVE_CODES: &[&str] = &[
    "ER201", // MissingTarget
    "ER202", // InvalidTarget
    "ER104", // MissingExportBinding
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
    /// Whether to disallow ambiguous tree literal syntax.
    pub disallow_ambiguous_tree_literal: bool,
}

#[derive(Debug)]
struct SharedConformanceEnvironment {
    /// The shared test repository.
    repository: Arc<Repository>,
    /// The shared in-memory file system.
    fs: Arc<MemoryFileSystem>,
    /// The next unique test id.
    next_id: AtomicUsize,
}

impl SharedConformanceEnvironment {
    /// Create a new shared environment for conformance tests.
    fn new() -> Self {
        let fs = Arc::new(MemoryFileSystem::new());
        let cwd = PathBuf::from("/test/parser/conformance");
        fs.create_dir_all(&cwd)
            .expect("failed to create conformance workspace root");
        let repository = Arc::new(Repository::new(
            cwd.clone(),
            Arc::new(MemoryCacheStore::new()),
            fs.clone(),
            HostEnvironment::capture_process(),
        ));
        Self {
            repository,
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
        PathBuf::from("/test/parser/conformance").join(format!("{stem}-{id}"))
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
        TestArea::Parse => parse_file_with_parser(path, content, file_type, options),
        // early error tests still need compiler checks
        TestArea::EarlySyntax | TestArea::Early => {
            parse_file_with_compiler(path, content, file_type, options)
        }
    }
}

fn parse_file_with_parser(
    path: &Path,
    content: &str,
    file_type: FileType,
    options: ParseOptions,
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
    let language = LanguageType::try_from(file_type).expect("file type has no parser language");
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language,
        ParserOptions {
            disallow_ambiguous_tree_literal: options.disallow_ambiguous_tree_literal,
            ..ParserOptions::default()
        },
    );
    let _ = parser.parse();

    // collect parse errors for relevance checks
    let errors: Vec<_> = parser
        .diagnostics
        .to_vec()
        .into_iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();
    let has_relevant_error = errors
        .iter()
        .any(|d| options.area.is_relevant_error(&d.code));

    if has_relevant_error {
        ParseOutcome::Error
    } else {
        ParseOutcome::Ok
    }
}

/// Apply a default config for conformance runs.
fn apply_default_destack_config(
    program: &Repository,
    root: &Path,
    main_path: &Path,
) -> (Revision, ModuleId) {
    let destack_config_path = root.join("destack.json");
    let content = serde_json::to_string_pretty(&serde_json::json!({
        "compiler": {
            "checkTs": true,
            "checkJs": true,
        },
        "targets": {
            "default": {
                "emit": "js",
            }
        },
        "defaultTarget": "default",
    }))
    .expect("default conformance config serialization should succeed");

    program
        .file_system()
        .write_string(&destack_config_path, &content)
        .expect("failed to write conformance destack.json");
    let revision = write_workspace_text_file(program, &destack_config_path, &content);
    let module_id = module_id_for_path(program, revision, main_path);

    (revision, module_id)
}

fn parse_file_with_compiler(
    path: &Path,
    content: &str,
    file_type: FileType,
    options: ParseOptions,
) -> ParseOutcome {
    let (repository, program, root, file_path) = SHARED_CONFORMANCE_ENV.with(|env| {
        // allocate a fresh test root and file path
        let root = env.root_for(path);
        let file_path = env.file_for(&root, path);
        let repository = env.repository.clone();
        let program = repository.clone();

        env.fs
            .add_file(&file_path, content.as_bytes())
            .expect("failed to add test file");

        (repository, program, root, file_path)
    });

    // materialize the source file with the requested file type
    let file_path = conformance_module_path_for_type(&file_path, file_type);
    let content = FileContent::Text {
        content: content.to_string(),
    };
    let _ = write_workspace_file(&program, &file_path, content);

    // ensure conformance runs check JS/TS analyze errors
    let (revision, module_id) = apply_default_destack_config(&program, &root, &file_path);

    // create compiler and compile the module
    let compiler = Arc::new(Compiler::new(repository.clone()));

    // run up to check
    let profile = default_profile_id_for_module(&program, revision, module_id);
    let artifact_keys = vec![ArtifactKey::DirChecked {
        module: module_id,
        profile,
    }];
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _revision =
            provide_workspace_artifacts(repository.clone(), compiler.clone(), &artifact_keys);
    }));

    // panics during compilation are treated as errors
    if result.is_err() {
        return ParseOutcome::Error;
    }

    // collect compiler errors for relevance checks
    let diagnostics = module_artifact_diagnostics(&program, revision, module_id, profile);
    let errors: Vec<_> = diagnostics
        .iter()
        .into_iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();
    let has_relevant_error = errors
        .iter()
        .any(|d| options.area.is_relevant_error(&d.code));

    if has_relevant_error {
        ParseOutcome::Error
    } else {
        ParseOutcome::Ok
    }
}

/// Return one workspace path whose extension matches the requested file type.
fn conformance_module_path_for_type(path: &Path, file_type: FileType) -> PathBuf {
    let Some(extension) = file_type.extension() else {
        return path.to_path_buf();
    };

    path.with_extension(extension)
}
