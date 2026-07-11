use std::collections::HashMap;
use std::env::current_dir;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Once};

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactPayload,
    ArtifactProvider, ArtifactSidecar, ArtifactVersion, DiagnosticAnchor, DiagnosticContext,
    DiagnosticDisplay, DiagnosticError, DirParsed, DirParsedFile, MemoryBlobStore, ToDiagnostic,
};
use destack_compiler::Compiler;
use destack_core::StringPool;
use destack_dir::{Expression, LocalNodeId, NodeParentIndex, ScalarLiteral, Tree};
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, ParserOptions};
use destack_repository::{
    DependencySetResolution, DestackLayoutOverride, Edit, Environment, LintCategory, LintSeverity,
    LinterOptions, Module, Profile, ProviderContext, ProviderError, ProviderResult, Ref,
    Repository, Revision, Settings, open_repository_from_fs,
};
use destack_source::{
    ContentId, DiagnosticCollection, DiagnosticLabel, DiagnosticSeverity, DiffOptions, File,
    FileId, FileSystem, FileType, LanguageType, Loader, ModuleId, OverlayFileSystem, Patch,
    PhysicalFileSystem, PrintOptions, Span, TargetId, Uri, print_diagnostics, print_diff,
};
use parking_lot::Mutex;

use crate::{
    BoxedLintRule, Fixability, LintLevel, LintModuleReport, LintReport, LintRequirement, LintRule,
    LintRunReport, LintRunner,
};

/// Shared memory blob store for linter tests.
static TEST_BLOB_STORE: LazyLock<Arc<MemoryBlobStore>> =
    LazyLock::new(|| Arc::new(MemoryBlobStore::new()));

/// Process wide per lib-set warmers for prelude profile setup.
static PRELUDE_WARMERS: LazyLock<Mutex<HashMap<Vec<String>, Arc<Once>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// One linter test compiler provider attempt.
#[derive(Debug)]
struct TestProviderContext {
    /// The repository that owns the pinned revision.
    repository: Arc<Repository>,
    /// The pinned revision for this attempt.
    revision: Revision,
    /// The artifact key being built.
    artifact_key: ArtifactKey,
    /// The diagnostics produced by this attempt.
    diagnostics: Mutex<DiagnosticCollection>,
    /// The sidecars produced by this attempt.
    sidecars: Mutex<Vec<ArtifactSidecar>>,
}

impl TestProviderContext {
    /// Create one linter test compiler provider attempt.
    fn new(repository: Arc<Repository>, revision: Revision, artifact_key: ArtifactKey) -> Self {
        Self {
            repository,
            revision,
            artifact_key,
            diagnostics: Mutex::new(DiagnosticCollection::new()),
            sidecars: Mutex::new(Vec::new()),
        }
    }

    /// Return diagnostics produced by this attempt.
    fn diagnostics(&self) -> DiagnosticCollection {
        self.diagnostics.lock().clone()
    }

    /// Return sidecars produced by this attempt.
    fn sidecars(&self) -> Vec<ArtifactSidecar> {
        self.sidecars.lock().clone()
    }

    /// Build one invalid diagnostic anchor error.
    fn invalid_anchor(message: impl Into<String>) -> DiagnosticError {
        DiagnosticError::InvalidAnchor {
            message: message.into(),
        }
    }

    /// Return one exact file content id.
    fn file_content_id(&self, file_id: FileId) -> Result<ContentId, DiagnosticError> {
        let content = self
            .repository
            .file_content_id(self.revision, file_id)
            .map_err(|error| {
                Self::invalid_anchor(format!(
                    "failed to read diagnostic file content id: {error}"
                ))
            })?;
        let Some(content) = content else {
            return Err(Self::invalid_anchor(format!(
                "diagnostic file is not tracked in revision: {file_id:?}"
            )));
        };

        Ok(content)
    }
}

/// Collect the source closure for one loader-owned artifact.
fn collect_loader(
    compiler: &Compiler,
    revision: Revision,
    key: ArtifactKey,
) -> ProviderResult<ArtifactDependencySet> {
    let ArtifactKey::DirParsed { module } = key else {
        return Err(ProviderError::internal(format!(
            "unsupported linter test loader artifact key: {key:?}"
        ))
        .into());
    };

    let module = compiler
        .repository
        .module(revision, module)
        .map_err(|error| ProviderError::internal(error.to_string()))?
        .ok_or_else(|| ProviderError::internal(format!("missing module: {module:?}")))?;

    // observe every contributing source file
    let mut dependencies = ArtifactDependencySet::default();
    let file_ids = if module.is_code() {
        module
            .files
            .iter()
            .map(|file| file.file_id)
            .collect::<Vec<_>>()
    } else {
        vec![module.file_id]
    };
    for file_id in file_ids {
        let content = compiler
            .repository
            .file_content_id(revision, file_id)
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or_else(|| {
                ProviderError::internal(format!("missing source content id: {file_id:?}"))
            })?;

        dependencies.observe_file(file_id, content);
    }

    Ok(dependencies)
}

/// Seed one loader-owned artifact for linter compiler tests.
fn provide_loader_artifact(compiler: &Compiler, context: &TestProviderContext) -> ArtifactPayload {
    match context.artifact_key() {
        ArtifactKey::DirParsed { module } => provide_dir_parsed(compiler, module, context),
        ArtifactKey::Data { module } => {
            panic!("data artifact reached linter test loader provider for {module:?}")
        }
        artifact_key => {
            panic!("non loader artifact reached linter test loader provider: {artifact_key:?}")
        }
    }
}

/// Seed one parsed DIR artifact from source.
fn provide_dir_parsed(
    compiler: &Compiler,
    module_id: ModuleId,
    context: &TestProviderContext,
) -> ArtifactPayload {
    let module = compiler
        .repository
        .module(context.revision(), module_id)
        .unwrap_or_else(|error| panic!("failed to load source module: {error}"))
        .unwrap_or_else(|| panic!("missing source module for {module_id:?}"));
    let file = source_file(compiler, context.revision(), module.file_id);
    let dir = match module.loader {
        Loader::Destack => {
            if matches!(file.ty, FileType::Html | FileType::Css) {
                anchor_dir_parsed(module_id, file.as_ref())
            } else {
                parse_code_dir(compiler, file.clone(), context)
            }
        }
        Loader::Json
        | Loader::Toml
        | Loader::Yaml
        | Loader::Text
        | Loader::Base64
        | Loader::Binary
        | Loader::File => anchor_dir_parsed(module_id, file.as_ref()),
    };

    ArtifactPayload::DirParsed(Arc::new(dir))
}

/// Load one tracked source file for a fixed revision.
fn source_file(compiler: &Compiler, revision: Revision, file_id: FileId) -> Arc<File> {
    compiler
        .repository
        .file(revision, file_id)
        .unwrap_or_else(|error| panic!("failed to load source file: {error}"))
        .unwrap_or_else(|| panic!("missing source file for {file_id:?}"))
}

/// Build one stable parsed DIR for non-code source.
fn anchor_dir_parsed(module_id: ModuleId, file: &File) -> DirParsed {
    let mut tree = Tree::new(module_id);
    let anchor_expression = insert_anchor_expression(&mut tree, file.id);

    let file = DirParsedFile {
        file_id: file.id,
        aliases: Vec::new(),
        roots: Vec::new(),
        tokens: Vec::new(),
        side_tokens: Vec::new(),
        anchor_expression,
    };

    DirParsed::new(tree, vec![file], anchor_expression)
}

/// Parse one code module into DIR.
fn parse_code_dir(
    compiler: &Compiler,
    file: Arc<File>,
    context: &TestProviderContext,
) -> DirParsed {
    let language_type = LanguageType::try_from(file.ty)
        .unwrap_or_else(|file_type| panic!("non-code file reached parser: {file_type:?}"));
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language_type,
        ParserOptions::default(),
        Arc::clone(compiler.repository.string_pool()),
    );
    let expressions = parser.parse();
    context.emit_diagnostics(parser.diagnostics());

    let (tokens, side_tokens) = parser.take_tokens();
    let anchor_expression = insert_anchor_expression(&mut parser.tree, file.id);

    let parsed_file = DirParsedFile {
        file_id: file.id,
        aliases: Vec::new(),
        roots: expressions,
        tokens,
        side_tokens,
        anchor_expression,
    };

    DirParsed::new(parser.tree, vec![parsed_file], anchor_expression)
}

/// Insert one synthetic anchor expression at the start of a file.
fn insert_anchor_expression(tree: &mut Tree, file_id: FileId) -> LocalNodeId<Expression> {
    let span = destack_source::Span::empty(file_id);

    tree.insert(
        Expression::ScalarLiteral(ScalarLiteral::Boolean(false)),
        span,
    )
}

impl DiagnosticContext for TestProviderContext {
    /// Resolve one provider diagnostic anchor into a final source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        let span = match anchor {
            DiagnosticAnchor::Span(span) => {
                self.file_content_id(span.file)?;

                *span
            }
            DiagnosticAnchor::File(file) => {
                self.file_content_id(*file)?;

                Span::empty(*file)
            }
            DiagnosticAnchor::Module(module) => {
                let module_id = *module;
                let module = self
                    .repository
                    .module(self.revision, module_id)
                    .expect("linter test provider should read module");
                let Some(module) = module else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic module is not tracked in revision: {module_id:?}"
                    )));
                };
                self.file_content_id(module.file_id)?;

                Span::empty(module.file_id)
            }
            DiagnosticAnchor::Package(package) => {
                let package_id = *package;
                let package = self
                    .repository
                    .package(self.revision, package_id)
                    .expect("linter test provider should read package");
                let Some(package) = package else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic package is not tracked in revision: {package_id:?}"
                    )));
                };
                let Some(file_id) = package.destack_file_id else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic package has no destack config file: {package_id:?}"
                    )));
                };
                self.file_content_id(file_id)?;

                Span::empty(file_id)
            }
        };

        let content = self.file_content_id(span.file)?;

        Ok(DiagnosticLabel {
            content,
            span,
            message,
        })
    }

    /// Display one repository-backed value when the context can resolve it.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        match display {
            DiagnosticDisplay::Module(module_id) => {
                let module = self
                    .repository
                    .module(self.revision, module_id)
                    .expect("linter test provider should read module");
                let Some(module) = module else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic module is not tracked in revision: {module_id:?}"
                    )));
                };

                Ok(module.uri.to_string())
            }
            DiagnosticDisplay::Package(package_id) => {
                let package = self
                    .repository
                    .package(self.revision, package_id)
                    .expect("linter test provider should read package");
                let Some(package) = package else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic package is not tracked in revision: {package_id:?}"
                    )));
                };
                let name = package
                    .name
                    .clone()
                    .or_else(|| package.path.as_ref().map(|path| path.display().to_string()));
                let Some(name) = name else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic package has no display name: {package_id:?}"
                    )));
                };

                Ok(name)
            }
            DiagnosticDisplay::Target(target_id) => {
                let target_name = self
                    .repository
                    .target_display(self.revision, target_id)
                    .expect("linter test provider should read target name");
                let Some(target_name) = target_name else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic target is not tracked in revision: {target_id:?}"
                    )));
                };

                Ok(target_name)
            }
        }
    }
}

impl ProviderContext for TestProviderContext {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey {
        self.artifact_key
    }

    /// Record an already-final diagnostic collection produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: DiagnosticCollection) {
        if diagnostics.is_empty() {
            return;
        }

        self.diagnostics.lock().merge_from(&diagnostics);
    }

    /// Record one sidecar produced by this attempt.
    fn emit_sidecar(&self, sidecar: ArtifactSidecar) {
        self.sidecars.lock().push(sidecar);
    }

    /// Record one diagnostic produced by this attempt.
    fn emit(
        &self,
        diagnostic: &dyn destack_artifact::DiagnosticLike,
    ) -> Result<(), DiagnosticError> {
        let diagnostic = diagnostic
            .to_diagnostic(self)
            .expect("linter test provider should finalize diagnostic");
        let diagnostics = DiagnosticCollection::from_diagnostics(vec![diagnostic]);

        self.emit_diagnostics(diagnostics);

        Ok(())
    }
}

/// Test wrapper for linting.
#[allow(unused)]
pub(crate) struct TestProgram {
    /// The file system.
    fs: Arc<OverlayFileSystem>,
    /// The repository.
    pub repository: Arc<Repository>,
    /// The profile for tests.
    profile: Profile,
    /// The compiler.
    compiler: Arc<Compiler>,
    /// The latest compiler diagnostics for this test harness.
    latest_diagnostics: Mutex<DiagnosticCollection>,
    /// Pending artifact roots for the next compiler run.
    pending_artifact_keys: Mutex<Vec<ArtifactKey>>,
    /// The lint runner.
    runner: LintRunner,
    /// Linter options for tests (all rules enabled by default).
    linter_options: LinterOptions,
    /// Whether library resolution has been enqueued.
    has_enqueued_profile_resolution: AtomicBool,
}

impl std::fmt::Debug for TestProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestProgram")
            .field("rules", &self.runner.rules().len())
            .finish()
    }
}

/// Linter options for tests with all categories enabled.
fn test_linter_options() -> LinterOptions {
    let mut options = LinterOptions::all();
    for category in LintCategory::ALL {
        options = options.with_category(*category, LintSeverity::Warning);
    }
    options
}

/// Return the default libs for linter tests.
fn default_test_libs() -> Vec<String> {
    vec!["es2024".to_string()]
}

/// Extend a lib list with any libs required by lint requirements.
fn extend_libs_from_requirements(libs: &mut Vec<String>, requirements: &[LintRequirement]) {
    for requirement in requirements {
        if let LintRequirement::RequireLibSymbol(_, rule_libs) = requirement {
            // choose a single lib per requirement: this avoids conflicting environment sets
            let Some(lib_name) = rule_libs.first().copied() else {
                continue;
            };
            let lib_name = lib_name.to_string();
            if !libs.contains(&lib_name) {
                libs.push(lib_name);
            }
        }
    }
}

/// Collect required libs from a set of lint rules.
fn collect_required_libs_from_rules(rules: &[BoxedLintRule]) -> Vec<String> {
    let mut libs = default_test_libs();
    for rule in rules {
        let meta = rule.meta();
        extend_libs_from_requirements(&mut libs, meta.requires_all);
        extend_libs_from_requirements(&mut libs, meta.requires_any);
    }
    libs
}

/// Build a module list for multi-module linter tests.
macro_rules! test_modules {
    ($($path:expr => $source:expr),+ $(,)?) => {
        &[
            $(
                ($path, $source),
            )+
        ]
    };
}
pub(crate) use test_modules;

#[allow(dead_code)]
impl TestProgram {
    /// Return the current workspace reference.
    fn current_reference(&self) -> Ref {
        Ref::for_root(self.repository.path())
    }

    /// Return the current published workspace revision.
    fn current_revision(&self) -> Revision {
        self.repository
            .current(&self.current_reference())
            .expect("workspace revision should be tracked")
    }

    /// Resolve one compiler artifact through the repository dependency-set model.
    fn resolve_compiler_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<ArtifactVersion> {
        if let Some(version) = self.terminal_version(revision, key)? {
            return Ok(version);
        }

        let mut set = self.collect_compiler_artifact(revision, key)?;
        loop {
            match self.repository.resolve_dependency_set(revision, set)? {
                DependencySetResolution::Incomplete => {
                    set = self.collect_compiler_artifact(revision, key)?;
                }
                DependencySetResolution::Pending {
                    frontier,
                    pending_set,
                } => {
                    for dependency in frontier {
                        self.resolve_compiler_artifact(revision, dependency)?;
                    }
                    set = if let Some(pending_set) = pending_set {
                        pending_set
                    } else {
                        self.collect_compiler_artifact(revision, key)?
                    };
                }
                DependencySetResolution::Resolved {
                    base,
                    dependencies,
                    failed,
                } => {
                    return self.commit_compiler_artifact(
                        revision,
                        key,
                        base,
                        dependencies,
                        failed,
                    );
                }
            }
        }
    }

    /// Return one terminal artifact binding.
    fn terminal_version(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<Option<ArtifactVersion>> {
        let version = self
            .repository
            .artifact_binding(revision, &key)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let Some(version) = version else {
            return Ok(None);
        };

        Ok(self
            .repository
            .artifact_table()
            .outcome(&version)
            .map(|_| version))
    }

    /// Collect the dependency set for one compiler artifact key.
    fn collect_compiler_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<ArtifactDependencySet> {
        match key.provider() {
            ArtifactProvider::Loader => collect_loader(self.compiler.as_ref(), revision, key),
            ArtifactProvider::Compiler => {
                let context = TestProviderContext::new(self.repository.clone(), revision, key);

                self.compiler.collect(&context)
            }
            ArtifactProvider::Linter | ArtifactProvider::Index => Err(ProviderError::internal(
                format!("unsupported linter test artifact key: {key:?}"),
            )
            .into()),
        }
    }

    /// Commit one frozen compiler artifact dependency set.
    fn commit_compiler_artifact(
        &self,
        revision: Revision,
        key: ArtifactKey,
        base: Option<ArtifactVersion>,
        dependencies: Vec<ArtifactDependency>,
        failed: Option<ArtifactKey>,
    ) -> ProviderResult<ArtifactVersion> {
        let version = ArtifactVersion::new(
            key,
            self.repository.build_fingerprint(),
            base,
            dependencies.iter().cloned(),
        );

        // record poisoned dependencies without running the provider
        if let Some(failed) = failed {
            self.fail_compiler_artifact(
                revision,
                version,
                base,
                dependencies,
                DiagnosticCollection::new(),
                Vec::new(),
                ArtifactFailure::requirement(failed),
            )?;

            return Ok(version);
        }

        // reuse committed memory or disk records before running the provider
        if self.repository.artifact_table().outcome(&version).is_some() {
            self.repository
                .bind_artifact(revision, version)
                .map_err(|error| ProviderError::internal(error.to_string()))?;

            return Ok(version);
        }
        if self
            .repository
            .load_artifact(revision, version)
            .map_err(|error| ProviderError::internal(error.to_string()))?
        {
            return Ok(version);
        }

        // run the local test provider and publish its terminal outcome
        let context = TestProviderContext::new(self.repository.clone(), revision, key);
        match self.provide_compiler_artifact(revision, key, &context) {
            Ok(payload) => {
                self.repository
                    .complete_artifact(
                        revision,
                        version,
                        base,
                        payload,
                        dependencies,
                        context.diagnostics(),
                        context.sidecars(),
                    )
                    .map_err(|error| ProviderError::internal(error.to_string()))?;
            }
            Err(error) => match *error {
                ProviderError::Failed { failure } => {
                    self.fail_compiler_artifact(
                        revision,
                        version,
                        base,
                        dependencies,
                        context.diagnostics(),
                        context.sidecars(),
                        failure,
                    )?;
                }
                ProviderError::RequirementFailed { key } => {
                    self.fail_compiler_artifact(
                        revision,
                        version,
                        base,
                        dependencies,
                        context.diagnostics(),
                        context.sidecars(),
                        ArtifactFailure::requirement(key),
                    )?;
                }
                error => return Err(error.into()),
            },
        }

        Ok(version)
    }

    /// Provide one compiler artifact payload.
    fn provide_compiler_artifact(
        &self,
        _revision: Revision,
        key: ArtifactKey,
        context: &TestProviderContext,
    ) -> ProviderResult<ArtifactPayload> {
        match key.provider() {
            ArtifactProvider::Loader => {
                Ok(provide_loader_artifact(self.compiler.as_ref(), context))
            }
            ArtifactProvider::Compiler => self.compiler.provide(context),
            ArtifactProvider::Linter | ArtifactProvider::Index => Err(ProviderError::internal(
                format!("unsupported linter test artifact key: {key:?}"),
            )
            .into()),
        }
    }

    /// Fail one compiler artifact in the test repository.
    #[allow(clippy::too_many_arguments)]
    fn fail_compiler_artifact(
        &self,
        revision: Revision,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> ProviderResult<()> {
        self.repository
            .fail_artifact(
                revision,
                version,
                base,
                dependencies,
                diagnostics,
                sidecars,
                failure,
            )
            .map_err(|error| ProviderError::internal(error.to_string()).into())
    }

    /// Return the active profile id for tests.
    fn profile_id(&self) -> destack_source::ProfileId {
        self.profile.id()
    }

    /// Return the active package id for tests.
    fn package_id(&self) -> destack_source::PackageId {
        let mut package_ids = self
            .repository
            .package_ids(self.current_revision())
            .expect("workspace package ids should load");
        package_ids.sort_unstable();

        *package_ids
            .first()
            .expect("linter tests should have one active package")
    }

    /// Return one module for the current revision.
    pub(crate) fn repository_module(&self, module_id: ModuleId) -> Arc<Module> {
        self.repository
            .module(self.current_revision(), module_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| panic!("missing module for {module_id:?}"))
    }

    /// Return one source file for the current revision.
    pub(crate) fn repository_file(&self, file_id: FileId) -> Arc<File> {
        self.repository
            .file(self.current_revision(), file_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| panic!("missing file for {file_id:?}"))
    }

    /// Publish repository edits to the current workspace revision.
    fn apply_edits<I>(&self, edits: I)
    where
        I: IntoIterator<Item = Edit>,
    {
        let reference = self.current_reference();
        let revision = self.current_revision();
        let revision = self
            .repository
            .fork_with_edits(revision, edits)
            .expect("failed to fork linter test edits");

        self.repository
            .set_ref(&reference, revision)
            .expect("failed to publish linter test edits");
    }

    /// Create a new test repository with the given rules and options.
    fn new(
        rules: Vec<BoxedLintRule>,
        _inject_prelude: bool,
        explicit_libs: Option<Vec<String>>,
    ) -> Self {
        let cwd = current_dir().unwrap();
        let fs = Arc::new(OverlayFileSystem::with_inner(Arc::new(
            PhysicalFileSystem::new(),
        )));
        let environment = Environment::capture_process();

        let repository = Arc::new(
            open_repository_from_fs(
                cwd.clone(),
                fs.clone(),
                environment.clone(),
                Settings::default(),
                DestackLayoutOverride::default(),
            )
            .expect("failed to import repository from linter test file system")
            .with_blob_store(TEST_BLOB_STORE.clone()),
        );

        // profile
        let _libs = explicit_libs.unwrap_or_else(|| collect_required_libs_from_rules(&rules));
        let reference = Ref::for_root(repository.path());
        let revision = repository
            .current(&reference)
            .expect("linter test repository should publish a workspace revision");
        let mut package_ids = repository
            .package_ids(revision)
            .expect("linter test repository packages should load");
        package_ids.sort_unstable();
        let package_id = *package_ids
            .first()
            .expect("linter tests should have one active package");
        let target_id = TargetId::new(package_id, "js");
        let profile = repository
            .profile_for_target(revision, target_id)
            .expect("linter test target profile should load")
            .as_ref()
            .clone();

        // compiler and runner
        let compiler = Arc::new(Compiler::new(repository.clone()));
        let runner = LintRunner::new(rules);

        Self {
            fs,
            repository,
            profile,
            compiler,
            latest_diagnostics: Mutex::new(DiagnosticCollection::new()),
            pending_artifact_keys: Mutex::new(Vec::new()),
            runner,
            linter_options: test_linter_options(),
            has_enqueued_profile_resolution: AtomicBool::new(false),
        }
    }

    /// Create a test repository without prelude injection.
    pub(crate) fn new_without_prelude(rules: Vec<BoxedLintRule>) -> Self {
        Self::new(rules, false, None)
    }

    /// Create a test repository with prelude injection.
    pub(crate) fn new_with_prelude(rules: Vec<BoxedLintRule>) -> Self {
        Self::new(rules, true, None)
    }

    /// Create a prelude test repository with explicit libs.
    fn new_with_prelude_libs(libs: Vec<String>) -> Self {
        Self::new(Vec::new(), true, Some(libs))
    }

    /// Create a test with a single rule (without prelude).
    pub(crate) fn for_rule_without_prelude<R: LintRule + 'static>(rule: R) -> Self {
        Self::new_without_prelude(vec![crate::boxed(rule)])
    }

    /// Create a test with a single rule (with prelude).
    pub(crate) fn for_rule_with_prelude<R: LintRule + 'static>(rule: R) -> Self {
        let rule = crate::boxed(rule);
        let libs = collect_required_libs_from_rules(std::slice::from_ref(&rule));
        Self::warm_prelude_profile(&libs);
        Self::new(vec![rule], true, Some(libs))
    }

    /// Warm a prelude profile for a specific lib-set once per process.
    fn warm_prelude_profile(libs: &[String]) {
        // get or create a once gate for this lib set
        let libs = libs.to_vec();
        let warmer = {
            let mut warmers = PRELUDE_WARMERS.lock();
            warmers
                .entry(libs.clone())
                .or_insert_with(|| Arc::new(Once::new()))
                .clone()
        };

        // warm this lib set exactly once per process
        warmer.call_once(|| {
            let warm_program = Self::new_with_prelude_libs(libs);
            warm_program.enqueue_profile_resolution_once();
            warm_program.compile();
        });
    }

    /// Modify linter options.
    pub(crate) fn with_options(mut self, f: impl FnOnce(&mut LinterOptions)) -> Self {
        f(&mut self.linter_options);
        self
    }

    /// Set one source file in the filesystem and current workspace revision.
    pub(crate) fn add_file(&self, path: &str, content: &str) {
        let overlay_path = self.repository.path().join(path);
        self.fs.set_overlay(&overlay_path, content.to_string());

        self.apply_edits([Edit::set_text(path, content)]);
    }

    /// Set one root target config with explicit entry paths.
    pub(crate) fn set_root_target_entries(&self, name: &str, entry_paths: &[&str]) {
        let entries = entry_paths
            .iter()
            .map(|entry| format!("\"{entry}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let config = format!(
            r#"{{
    "targets": {{
        "{name}": {{
            "emit": "js",
            "entry": [{entries}]
        }}
    }}
}}
"#
        );

        self.add_file("destack.json", &config);
    }

    /// Add a module and return its id.
    pub(crate) fn add_module(&self, path: &str, content: &str) -> ModuleId {
        self.add_file(path, content);
        self.repository
            .module_id_for_path(self.current_revision(), Path::new(path))
            .unwrap()
            .unwrap_or_else(|| panic!("missing module id for {path}"))
    }

    /// Import a module.
    pub(crate) fn import_module(&self, module: ModuleId) {
        self.pending_artifact_keys
            .lock()
            .push(ArtifactKey::DirBound {
                module,
                profile: self.profile_id(),
            });
    }

    /// Resolve a module.
    pub(crate) fn resolve_module(&self, module: ModuleId) {
        self.pending_artifact_keys
            .lock()
            .push(ArtifactKey::DirExported {
                module,
                profile: self.profile_id(),
            });
    }

    /// Resolve the global environment for the current profile.
    pub(crate) fn resolve_global_environment(&self) {
        self.provide_compiler_artifacts(&[ArtifactKey::global_environment(self.profile_id())]);
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
    }

    /// Resolve library packages for the current profile.
    pub(crate) fn resolve_libs(&self) {
        self.provide_compiler_artifacts(&[ArtifactKey::global_environment(self.profile_id())]);
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
    }

    /// Enqueue library resolution once for this test repository.
    pub(crate) fn enqueue_profile_resolution_once(&self) {
        let has_enqueued = self
            .has_enqueued_profile_resolution
            .swap(true, Ordering::Relaxed);
        if has_enqueued {
            return;
        }

        self.resolve_global_environment();
        self.resolve_libs();
    }

    /// Analyze a module.
    pub(crate) fn analyze_module(&self, module: ModuleId) {
        self.pending_artifact_keys
            .lock()
            .push(ArtifactKey::DirChecked {
                module,
                profile: self.profile_id(),
            });
    }

    /// Run all queued tasks.
    pub(crate) fn compile(&self) {
        let artifact_keys = {
            let mut pending_artifact_keys = self.pending_artifact_keys.lock();
            std::mem::take(&mut *pending_artifact_keys)
        };

        self.provide_compiler_artifacts(&artifact_keys);
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
    }

    /// Build compiler artifacts through the shared resolution engine.
    fn provide_compiler_artifacts(&self, artifact_keys: &[ArtifactKey]) {
        let revision = self.current_revision();

        for artifact_key in artifact_keys {
            self.resolve_compiler_artifact(revision, *artifact_key)
                .unwrap_or_else(|error| {
                    panic!("failed to build linter test artifact {artifact_key:?}: {error}")
                });
        }
    }

    /// Replace the latest compiler diagnostics for this test harness.
    fn replace_latest_diagnostics(&self, diagnostics: DiagnosticCollection) {
        let mut latest_diagnostics = self.latest_diagnostics.lock();

        *latest_diagnostics = diagnostics;
    }

    /// Return the latest compiler diagnostics for this test harness.
    fn diagnostics(&self) -> DiagnosticCollection {
        self.latest_diagnostics.lock().clone()
    }

    /// Collect the current workspace diagnostics from module artifact families.
    fn current_workspace_diagnostics(&self) -> DiagnosticCollection {
        let revision = self.current_revision();
        let module_ids = self
            .repository
            .module_ids(revision)
            .unwrap_or_else(|error| panic!("failed to read workspace modules: {error}"));
        let profile_id = self.profile_id();
        let mut diagnostics = DiagnosticCollection::new();

        // current workspace artifacts
        for module_id in module_ids {
            let artifact_keys = [
                ArtifactKey::dir_parsed(module_id),
                ArtifactKey::dir_bound(module_id, profile_id),
                ArtifactKey::dir_imported(module_id, profile_id),
                ArtifactKey::dir_expanded(module_id, profile_id),
                ArtifactKey::dir_exported(module_id, profile_id),
                ArtifactKey::dir_resolved(module_id, profile_id),
                ArtifactKey::dir_checked(module_id, profile_id),
            ];

            for artifact_key in artifact_keys {
                let artifact_diagnostics = self
                    .repository
                    .diagnostics(revision, Some(artifact_key))
                    .unwrap_or_else(|error| {
                        panic!("failed to read diagnostics for {artifact_key:?}: {error}")
                    });
                diagnostics.merge_from(&artifact_diagnostics);
            }
        }

        diagnostics
    }

    /// Lint a module at the given level.
    pub(crate) fn lint_module(&self, module: ModuleId, level: LintLevel) -> Vec<LintReport> {
        let revision = self.current_revision();
        let module = self.repository_module(module);
        let profile = self.profile.clone();
        self.runner.lint_module(
            self.repository.clone(),
            revision,
            module,
            profile,
            &self.linter_options,
            level,
        )
    }

    /// Lint a module at the given level and collect performance data.
    pub(crate) fn lint_module_profiled(
        &self,
        module: ModuleId,
        level: LintLevel,
    ) -> LintModuleReport {
        let revision = self.current_revision();
        let module = self.repository_module(module);
        let profile = self.profile.clone();
        self.runner.lint_module_profiled(
            self.repository.clone(),
            revision,
            module,
            profile,
            &self.linter_options,
            level,
        )
    }

    /// Add module, compile through analysis, and lint at DIR level.
    pub(crate) fn lint_dir(&self, path: &str, content: &str) -> Vec<LintReport> {
        let module = self.add_module(path, content);
        self.import_module(module);
        self.enqueue_profile_resolution_once();
        self.analyze_module(module);
        self.compile();
        self.lint_module(module, LintLevel::Dir)
    }

    /// Add module, compile through analysis, and lint.
    pub(crate) fn lint(&self, path: &str, content: &str) -> Vec<LintReport> {
        let module = self.add_module(path, content);
        self.import_module(module);
        self.enqueue_profile_resolution_once();
        self.analyze_module(module);
        self.compile();
        self.lint_module(module, LintLevel::Dir)
    }

    /// Add modules, analyze them at DIR level, and return module ids by path.
    fn prepare_dir_modules(&self, modules: &[(&str, &str)]) -> Vec<(String, ModuleId)> {
        let mut module_entries = Vec::new();

        // add modules and track ids
        for (path, source) in modules {
            let module_id = self.add_module(path, source);
            module_entries.push(((*path).to_string(), module_id));
        }

        // import all modules
        for (_, module_id) in &module_entries {
            self.import_module(*module_id);
        }

        // resolve profile symbols once
        self.enqueue_profile_resolution_once();

        // analyze all modules
        for (_, module_id) in &module_entries {
            self.analyze_module(*module_id);
        }

        // compile all queued tasks
        self.compile();

        module_entries
    }

    /// Add modules, analyze them at DIR level, and lint one target module.
    pub(crate) fn lint_module_dir_with_modules(
        &self,
        modules: &[(&str, &str)],
        target_path: &str,
    ) -> Vec<LintReport> {
        let module_entries = self.prepare_dir_modules(modules);

        // resolve target module id from path
        let target_module_id = module_entries
            .iter()
            .find_map(|(path, module_id)| (path == target_path).then_some(*module_id))
            .unwrap_or_else(|| panic!("missing target module path {target_path}"));

        self.lint_module(target_module_id, LintLevel::Dir)
    }

    /// Add modules, analyze them at DIR level, and lint package-scope DIR rules.
    pub(crate) fn lint_package_dir_with_modules(
        &self,
        modules: &[(&str, &str)],
    ) -> Vec<LintReport> {
        self.prepare_dir_modules(modules);
        self.lint_package_dir()
    }

    /// Add modules, analyze them at DIR level, and lint workspace-scope DIR rules.
    pub(crate) fn lint_workspace_dir_with_modules(
        &self,
        modules: &[(&str, &str)],
    ) -> Vec<LintReport> {
        self.prepare_dir_modules(modules);
        self.lint_workspace_dir()
    }

    /// Lint the active package with package-scope DIR rules.
    pub(crate) fn lint_package_dir(&self) -> Vec<LintReport> {
        let revision = self.current_revision();
        self.runner.lint_package_dir(
            self.repository.clone(),
            revision,
            self.package_id(),
            self.profile_id(),
            &self.linter_options,
        )
    }

    /// Lint the active package with package-scope DIR rules and collect performance data.
    pub(crate) fn lint_package_dir_profiled(&self) -> LintRunReport {
        let revision = self.current_revision();
        self.runner.lint_package_dir_profiled(
            self.repository.clone(),
            revision,
            self.package_id(),
            self.profile_id(),
            &self.linter_options,
        )
    }

    /// Lint the active workspace with workspace-scope DIR rules.
    pub(crate) fn lint_workspace_dir(&self) -> Vec<LintReport> {
        let revision = self.current_revision();
        self.runner.lint_workspace_dir(
            self.repository.clone(),
            revision,
            self.profile_id(),
            &self.linter_options,
        )
    }

    /// Lint the active workspace with workspace-scope DIR rules and collect performance data.
    pub(crate) fn lint_workspace_dir_profiled(&self) -> LintRunReport {
        let revision = self.current_revision();
        self.runner.lint_workspace_dir_profiled(
            self.repository.clone(),
            revision,
            self.profile_id(),
            &self.linter_options,
        )
    }

    /// Lint the active workspace and package scope rules.
    pub(crate) fn lint_workspace(&self) -> Vec<LintReport> {
        let mut diagnostics = self.lint_workspace_dir();
        diagnostics.extend(self.lint_package_dir());
        diagnostics
    }

    /// Lint the active workspace and package scope rules and collect performance data.
    pub(crate) fn lint_workspace_profiled(&self) -> LintRunReport {
        let mut report = self.lint_workspace_dir_profiled();
        let package_dir_report = self.lint_package_dir_profiled();
        report.performance.merge(&package_dir_report.performance);
        report.diagnostics.extend(package_dir_report.diagnostics);
        report
    }

    /// Check no compiler diagnostics at or above the given severity.
    #[track_caller]
    pub(crate) fn check_no_diagnostic(&self, min_severity: DiagnosticSeverity) {
        let diagnostics = self.diagnostics();
        let highest = diagnostics.highest_severity();
        if let Some(highest) = highest
            && highest >= min_severity
        {
            let revision = self.current_revision();
            print_diagnostics(
                &|file_id| self.repository.file(revision, file_id).ok().flatten(),
                &diagnostics,
                PrintOptions::new().with_line_width(120),
            )
            .expect("compiler diagnostics should print");
            let severity_name = min_severity.family_name().to_ascii_lowercase();
            panic!(
                "repository has {} unexpected {severity_name}s",
                diagnostics.len()
            );
        }
    }

    /// Check no compiler errors.
    #[track_caller]
    pub(crate) fn check_clean(&self) {
        self.check_no_diagnostic(DiagnosticSeverity::Error);
    }

    /// Wrap diagnostics in a LintResult for assertion methods.
    pub(crate) fn result(&self, diagnostics: Vec<LintReport>) -> LintResult<'_> {
        LintResult::new(
            diagnostics,
            self.repository.as_ref(),
            self.current_revision(),
        )
    }
}

/// Result of linting that can be asserted on.
pub(crate) struct LintResult<'a> {
    diagnostics: Vec<LintReport>,
    repository: &'a Repository,
    revision: Revision,
}

#[allow(dead_code)]
impl<'a> LintResult<'a> {
    /// Create a new lint result.
    pub(crate) fn new(
        diagnostics: Vec<LintReport>,
        repository: &'a Repository,
        revision: Revision,
    ) -> Self {
        Self {
            diagnostics,
            repository,
            revision,
        }
    }

    /// Return one source file for one diagnostic file id.
    fn repository_file(&self, file_id: FileId) -> Arc<File> {
        self.repository
            .file(self.revision, file_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| panic!("missing file for {file_id:?}"))
    }

    /// Get the diagnostics.
    pub(crate) fn diagnostics(&self) -> &[LintReport] {
        &self.diagnostics
    }

    /// Print lint diagnostics using the standard diagnostic printer.
    fn print_diagnostics(&self, diagnostics: &[LintReport]) {
        let mut collection = DiagnosticCollection::new();
        for diagnostic in diagnostics {
            let diagnostic = diagnostic
                .to_diagnostic(self)
                .expect("lint test diagnostic should resolve");
            collection.insert(diagnostic);
        }

        print_diagnostics(
            &|file_id| self.repository.file(self.revision, file_id).ok().flatten(),
            &collection,
            PrintOptions::new().with_line_width(120),
        )
        .expect("lint diagnostics should print");
    }

    /// Assert diagnostics contain a lint with the given rule id.
    #[track_caller]
    pub(crate) fn assert_lint(&self, rule_id: &str) -> &Self {
        let found = self.diagnostics.iter().any(|d| d.rule_id == rule_id);
        if !found {
            let ids: Vec<_> = self.diagnostics.iter().map(|d| d.rule_id).collect();
            self.print_diagnostics(&self.diagnostics);
            panic!("expected lint '{rule_id}' but found: {ids:?}");
        }
        self
    }

    /// Assert diagnostics do not contain a lint with the given rule id.
    #[track_caller]
    pub(crate) fn assert_no_lint(&self, rule_id: &str) -> &Self {
        let found = self.diagnostics.iter().find(|d| d.rule_id == rule_id);
        if let Some(d) = found {
            panic!("unexpected lint '{rule_id}': {}", d.message);
        }
        self
    }

    /// Assert diagnostics contain exactly n lints with the given rule id.
    #[track_caller]
    pub(crate) fn assert_lint_count(&self, rule_id: &str, expected: usize) -> &Self {
        let matching: Vec<_> = self
            .diagnostics
            .iter()
            .filter(|d| d.rule_id == rule_id)
            .cloned()
            .collect();
        let count = matching.len();
        if count != expected {
            self.print_diagnostics(&matching);
            panic!("expected {expected} '{rule_id}' lints but found {count}");
        }
        self
    }

    /// Assert a lint exists at the given line (1-indexed).
    #[track_caller]
    pub(crate) fn assert_lint_at_line(&self, rule_id: &str, line: u32) -> &Self {
        let matching: Vec<_> = self
            .diagnostics
            .iter()
            .filter(|d| d.rule_id == rule_id)
            .collect();
        if matching.is_empty() {
            self.print_diagnostics(&self.diagnostics);
            panic!("expected lint '{rule_id}' at line {line} but found none");
        }

        for diagnostic in &matching {
            let span = diagnostic.primary;
            let file = self.repository_file(span.file);
            if let Some((line_index, _)) = file.get_position(span.start) {
                let diagnostic_line = line_index + 1;
                if diagnostic_line == line {
                    return self;
                }
            }
        }

        let lines: Vec<_> = matching
            .iter()
            .filter_map(|d| {
                let file = self.repository_file(d.primary.file);
                file.get_position(d.primary.start)
                    .map(|(line_index, _)| line_index + 1)
            })
            .collect();
        self.print_diagnostics(&self.diagnostics);
        panic!("expected lint '{rule_id}' at line {line} but found at lines: {lines:?}");
    }

    /// Apply patches to source code and return the result.
    pub(crate) fn apply_patches(&self, patches: Vec<&Patch>) -> String {
        // return original source if no patches
        if patches.is_empty() {
            if let Some(d) = self.diagnostics.first() {
                let file = self.repository_file(d.primary.file);
                return file.text().to_string();
            }
            return String::new();
        }

        // sort by span start, descending
        let mut patches = patches;
        patches.sort_by_key(|patch| std::cmp::Reverse(patch.span.start));

        // apply patches from the end so byte offsets stay stable
        let file_id = patches[0].span.file;
        let file = self.repository_file(file_id);
        let mut source = file.text().to_string();
        for patch in patches {
            let start = patch.span.start as usize;
            let end = patch.span.end as usize;
            source.replace_range(start..end, &patch.new_text);
        }

        source
    }

    /// Apply fixes from diagnostics with the given applicability and return the fixed source.
    pub(crate) fn apply_fixes(&self, applicability: Option<Fixability>) -> String {
        // collect all patches from fixes with matching applicability
        let patches: Vec<&Patch> = self
            .diagnostics
            .iter()
            .flat_map(|d| &d.fixes)
            .filter(|f| applicability.map(|a| f.applicability == a).unwrap_or(true))
            .flat_map(|f| &f.patches)
            .collect();

        let fixed = self.apply_patches(patches);
        self.format_source(&fixed)
    }

    /// Assert the safely fixed code matches expected.
    #[track_caller]
    pub(crate) fn assert_safe_fixed(&self, expected: &str) -> &Self {
        let fixed = self.apply_fixes(Some(Fixability::Safe));
        let fixed = fixed.trim();
        let expected = self.format_source(expected);
        let expected = expected.trim();
        if fixed != expected {
            print_diff(expected, fixed, &DiffOptions::new().with_whitespace());
            panic!("fixed code mismatch");
        }
        self
    }

    /// Assert the unsafely fixed code matches expected.
    #[track_caller]
    pub(crate) fn assert_unsafe_fixed(&self, expected: &str) -> &Self {
        let fixed = self.apply_fixes(Some(Fixability::Unsafe));
        let fixed = fixed.trim();
        let expected = self.format_source(expected);
        let expected = expected.trim();
        if fixed != expected {
            print_diff(expected, fixed, &DiffOptions::new().with_whitespace());
            panic!("fixed code mismatch");
        }
        self
    }

    /// Assert the suggested fixed code matches expected.
    #[track_caller]
    pub(crate) fn assert_suggested_fixed(&self, expected: &str) -> &Self {
        let fixed = self.apply_fixes(Some(Fixability::Suggestion));
        let fixed = fixed.trim();
        let expected = self.format_source(expected);
        let expected = expected.trim();
        if fixed != expected {
            print_diff(expected, fixed, &DiffOptions::new().with_whitespace());
            panic!("fixed code mismatch");
        }
        self
    }

    /// Assert that a fix exists for the given rule.
    #[track_caller]
    pub(crate) fn assert_has_fix(&self, rule_id: &str) -> &Self {
        let has_fix = self
            .diagnostics
            .iter()
            .filter(|d| d.rule_id == rule_id)
            .any(|d| !d.fixes.is_empty());
        assert!(has_fix, "expected fix for '{rule_id}' but none found");
        self
    }

    /// Assert that no fix exists for the given rule.
    #[track_caller]
    pub(crate) fn assert_has_no_fix(&self, rule_id: &str) -> &Self {
        let has_fix = self
            .diagnostics
            .iter()
            .filter(|d| d.rule_id == rule_id)
            .any(|d| !d.fixes.is_empty());
        assert!(!has_fix, "expected no fix for '{rule_id}' but found one");
        self
    }

    /// Format source code to normalize whitespace.
    pub(crate) fn format_source(&self, source: &str) -> String {
        let file_id = FileId::new(0);
        let file = Arc::new(File::from_text(
            file_id,
            "<test>".to_string(),
            Uri::from_string("<test>"),
            None,
            FileType::Destack,
            source.to_string(),
        ));

        // parse file
        let language_type = LanguageType::Destack;
        let mut parser = Parser::lex_file(file.clone(), language_type, Arc::new(StringPool::new()));
        let expressions = parser.parse();

        // if parsing fails, return original source
        if parser
            .diagnostics()
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            return source.to_string();
        }

        // format context
        let side_span = parser.tree.decorator_span();
        let parents = NodeParentIndex::from_tree(&parser.tree);
        let (tokens, side_tokens) = parser.take_token_spans();
        let strings = parser.publish_strings();
        let format_options = DestackFormatOptions::default();
        let context = DestackFormatContext::new(
            format_options,
            file.as_ref(),
            &parser.tree,
            &tokens,
            &side_tokens,
            &side_span,
            strings,
            &parents,
        );

        // format
        let mut result = if expressions.is_empty() {
            String::new()
        } else {
            let formatted = destack_fir::format!(context, [statement_list(&expressions)]).unwrap();
            let printed = formatted.print().unwrap();
            printed.as_str().to_string()
        };

        // ensure trailing newline
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }

        result
    }
}

impl DiagnosticContext for LintResult<'_> {
    /// Resolve one lint diagnostic anchor into a final source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        let span = match anchor {
            DiagnosticAnchor::Span(span) => *span,
            DiagnosticAnchor::File(file) => Span::empty(*file),
            DiagnosticAnchor::Module(module) => {
                let module = self
                    .repository
                    .module(self.revision, *module)
                    .map_err(|error| DiagnosticError::InvalidAnchor {
                        message: format!("failed to read diagnostic module: {error}"),
                    })?;
                let Some(module) = module else {
                    return Err(DiagnosticError::InvalidAnchor {
                        message: format!("diagnostic module is not tracked: {module:?}"),
                    });
                };

                Span::empty(module.file_id)
            }
            DiagnosticAnchor::Package(package) => {
                let package =
                    self.repository
                        .package(self.revision, *package)
                        .map_err(|error| DiagnosticError::InvalidAnchor {
                            message: format!("failed to read diagnostic package: {error}"),
                        })?;
                let Some(package) = package else {
                    return Err(DiagnosticError::InvalidAnchor {
                        message: format!("diagnostic package is not tracked: {package:?}"),
                    });
                };
                let Some(file_id) = package.destack_file_id else {
                    return Err(DiagnosticError::InvalidAnchor {
                        message: "diagnostic package has no destack config file".to_string(),
                    });
                };

                Span::empty(file_id)
            }
        };
        let content = self
            .repository
            .file_content_id(self.revision, span.file)
            .map_err(|error| DiagnosticError::InvalidAnchor {
                message: format!("failed to read diagnostic file content id: {error}"),
            })?;
        let Some(content) = content else {
            return Err(DiagnosticError::InvalidAnchor {
                message: format!("diagnostic file is not tracked: {:?}", span.file),
            });
        };

        Ok(DiagnosticLabel {
            content,
            span,
            message,
        })
    }

    /// Display one repository-backed value when the context can resolve it.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        match display {
            DiagnosticDisplay::Module(module) => self
                .repository
                .module_display(self.revision, module)
                .map_err(|error| DiagnosticError::InvalidAnchor {
                    message: format!("failed to format diagnostic module: {error}"),
                })?
                .ok_or_else(|| DiagnosticError::InvalidAnchor {
                    message: format!("diagnostic module is not tracked: {module:?}"),
                }),
            DiagnosticDisplay::Package(package) => self
                .repository
                .package_display(self.revision, package)
                .map_err(|error| DiagnosticError::InvalidAnchor {
                    message: format!("failed to format diagnostic package: {error}"),
                })?
                .ok_or_else(|| DiagnosticError::InvalidAnchor {
                    message: format!("diagnostic package has no display name: {package:?}"),
                }),
            DiagnosticDisplay::Target(target) => self
                .repository
                .target_display(self.revision, target)
                .map_err(|error| DiagnosticError::InvalidAnchor {
                    message: format!("failed to format diagnostic target: {error}"),
                })?
                .ok_or_else(|| DiagnosticError::InvalidAnchor {
                    message: format!("diagnostic target is not tracked: {target:?}"),
                }),
        }
    }
}
