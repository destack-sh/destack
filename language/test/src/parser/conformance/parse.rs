use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_compiler::{BuildKey, Compiler, CompilerOptions, ImportError, ResolveMode};
use destack_parser::{Parser, ParserSettings};
use destack_source::{
    DiagnosticSeverity, File, FileId, FileType, LanguageType, MemoryFileSystem, ModuleId, Uri,
};
use destack_workspace::{
    ArtifactKey, Destack, DestackOptions, MemoryCacheStore, OutputFormat, Program, Session,
    TargetId, TargetOptions,
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
        let cwd = PathBuf::from("/test/parser/conformance");
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
    let language = LanguageType::from(file_type);
    let mut parser = Parser::lex_file_with_settings(
        file.clone(),
        language,
        ParserSettings {
            disallow_ambiguous_tree_literal: options.disallow_ambiguous_tree_literal,
            ..ParserSettings::default()
        },
    );
    let _ = parser.parse();

    // collect parse errors for relevance checks
    let errors: Vec<_> = parser
        .diagnostics
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

/// Apply a default config for conformance runs.
fn apply_default_destack_config(program: &Program, module_id: ModuleId, root: &Path) {
    // build a default config to enable early checks for JS and TS
    let file_id = program.files.next_id();
    let mut options = DestackOptions::default();
    options.compiler.check_ts = true;
    options.compiler.check_js = true;
    let target = TargetOptions {
        output: OutputFormat::Js,
        ..Default::default()
    };
    options.targets.insert("default".to_string(), target);
    options.default_target = Some("default".to_string());

    // attach the config to the owning package
    let destack_config_path = root.join("destack.json");
    let config = Destack {
        file_id,
        path: destack_config_path,
        directory: root.to_path_buf(),
        options,
        content: Default::default(),
    };
    let package_id = {
        let module = program.modules.get(module_id);
        let module = module.read();
        module.package_id
    };
    let package = program.packages.get(package_id);
    let mut package = package.write();
    package.config = Some(config.clone());
    package.targets.clear();
    for (name, options) in config.options.targets.iter() {
        let target = options.to_target(name);
        let target_id = TargetId::new(package_id, name);
        package.targets.insert(target_id, target);
    }
}

fn parse_file_with_compiler(
    path: &Path,
    content: &str,
    file_type: FileType,
    options: ParseOptions,
) -> ParseOutcome {
    let (session, program, root, file_path) = SHARED_CONFORMANCE_ENV.with(|env| {
        // allocate a fresh test root and file path
        let root = env.root_for(path);
        let file_path = env.file_for(&root, path);
        let session = env.session.clone();
        let program = session.add_root(root.clone());

        env.fs
            .add_file(&file_path, content.as_bytes())
            .expect("failed to add test file");

        (session, program, root, file_path)
    });

    // register module with the correct file type (important for JSX files with .js extension)
    let uri = Uri::from_path(&file_path);
    let module_id = program.register_inline_module(uri, content.to_string(), file_type);

    // ensure conformance runs check JS/TS analyze errors
    apply_default_destack_config(&program, module_id, &root);

    // create compiler and compile the module
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            follow_imports: false,
            resolve_mode: ResolveMode::Lenient,
            disallow_ambiguous_tree_literal: options.disallow_ambiguous_tree_literal,
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
    compiler.enqueue(BuildKey::Artifact(ArtifactKey::DirAnalyzed {
        module: module_id,
        profile,
    }));
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        compiler.compile();
    }));

    // panics during compilation are treated as errors
    if result.is_err() {
        return ParseOutcome::Error;
    }

    // collect compiler errors for relevance checks
    let errors: Vec<_> = program
        .diagnostics
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
