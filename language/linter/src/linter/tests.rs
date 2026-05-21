use std::collections::HashMap;
use std::env::current_dir;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Once};

use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactOutcome, ArtifactPathState,
    ArtifactPayload, ArtifactProvider, ArtifactVersion, DiagnosticAnchor, DiagnosticContext,
    DiagnosticDisplay, DiagnosticError, DirParsed, DirParsedFile, EmitFormat, Host,
    MemoryCacheStore, Platform, ProfileFlags, ProfileKey, Runtime, ToDiagnostic,
};
use destack_compiler::Compiler;
use destack_core::StringPool;
use destack_dir as dir;
use destack_dir::NodeParentIndex;
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::{Parser, ParserOptions};
use destack_session::open_repository_from_fs;
use destack_source::{
    DiagnosticCollection, DiagnosticLabel, DiagnosticSeverity, DiffOptions, Edit as SourceEdit,
    File, FileContentId, FileId, FileSystem, FileType, LanguageType, Loader, ModuleId,
    OverlayFileSystem, PackageId, PhysicalFileSystem, PrintOptions, Span, Uri, print_diagnostics,
    print_diff,
};
use destack_workspace::{
    Edit as RepositoryEdit, Environment, LintCategory, LintSeverity, LinterOptions, Module,
    Profile, ProviderContext, ProviderError, Ref, Repository, Revision,
};
use parking_lot::Mutex;

use crate::{
    BoxedLintRule, Fixability, LintLevel, LintModuleReport, LintReport, LintRequirement, LintRule,
    LintRunReport, LintRunner,
};

/// Shared memory cache store for linter tests.
static TEST_CACHE_STORE: LazyLock<Arc<MemoryCacheStore>> =
    LazyLock::new(|| Arc::new(MemoryCacheStore::new()));

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
    /// The exact dependencies read by this attempt.
    dependencies: Mutex<Vec<ArtifactDependency>>,
    /// The diagnostics produced by this attempt.
    diagnostics: Mutex<DiagnosticCollection>,
}

impl TestProviderContext {
    /// Create one linter test compiler provider attempt.
    fn new(repository: Arc<Repository>, revision: Revision, artifact_key: ArtifactKey) -> Self {
        Self {
            repository,
            revision,
            artifact_key,
            dependencies: Mutex::new(Vec::new()),
            diagnostics: Mutex::new(DiagnosticCollection::new()),
        }
    }

    /// Publish the recorded payload as one exact artifact record.
    fn publish(&self, payload: ArtifactPayload) {
        let dependencies = self.dependencies.lock().clone();
        let diagnostics = self.diagnostics.lock().clone();
        let version = ArtifactVersion::new(self.artifact_key, dependencies.iter().cloned());

        self.repository
            .complete_artifact(self.revision, version, payload, dependencies, diagnostics)
            .expect("linter test provider should record artifact version");
    }

    /// Fail the test attempt.
    fn fail(&self, failure: ArtifactFailure) {
        let dependencies = self.dependencies.lock().clone();
        let diagnostics = self.diagnostics.lock().clone();
        let version = ArtifactVersion::new(self.artifact_key, dependencies.iter().cloned());

        self.repository
            .fail_artifact(self.revision, version, dependencies, diagnostics, failure)
            .expect("linter test provider should record failed artifact version");
    }

    /// Build one invalid diagnostic anchor error.
    fn invalid_anchor(message: impl Into<String>) -> DiagnosticError {
        DiagnosticError::InvalidAnchor {
            message: message.into(),
        }
    }

    /// Return one exact file content id.
    fn file_content_id(&self, file_id: FileId) -> Result<FileContentId, DiagnosticError> {
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
    let file = source_file(compiler, context.revision(), module.file_id, context);
    let dir = match module.loader {
        Loader::Destack | Loader::TypeScript | Loader::JavaScript => {
            if matches!(file.ty, FileType::Html | FileType::Css) {
                anchor_dir_parsed(module_id, file.as_ref())
            } else {
                parse_code_dir(compiler, file.clone(), module.package_id, context)
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

    ArtifactPayload::DirParsed(dir)
}

/// Load one source file and record its exact content dependency.
fn source_file(
    compiler: &Compiler,
    revision: Revision,
    file_id: FileId,
    context: &TestProviderContext,
) -> Arc<File> {
    let content_id = compiler
        .repository
        .file_content_id(revision, file_id)
        .unwrap_or_else(|error| panic!("failed to load source content id: {error}"))
        .unwrap_or_else(|| panic!("missing source content id for {file_id:?}"));
    let file = compiler
        .repository
        .file(revision, file_id)
        .unwrap_or_else(|error| panic!("failed to load source file: {error}"))
        .unwrap_or_else(|| panic!("missing source file for {file_id:?}"));

    context.track(ArtifactDependency::path_state(
        file_id,
        ArtifactPathState::File,
    ));
    context.track(ArtifactDependency::file_content(file_id, content_id));

    file
}

/// Build one stable parsed DIR for non-code source.
fn anchor_dir_parsed(module_id: ModuleId, file: &File) -> DirParsed {
    let mut tree = dir::Tree::new(module_id);
    let anchor_expression = insert_anchor_expression(&mut tree, file.id);

    let file = DirParsedFile {
        file_id: file.id,
        aliases: Vec::new(),
        roots: Vec::new(),
        token_range: 0..0,
        side_token_range: 0..0,
        anchor_expression,
    };

    DirParsed::new(tree, vec![file], Vec::new(), Vec::new(), anchor_expression)
}

/// Parse one code module into DIR.
fn parse_code_dir(
    compiler: &Compiler,
    file: Arc<File>,
    package_id: PackageId,
    context: &TestProviderContext,
) -> DirParsed {
    let language_type = language_type_for_code_file(compiler, file.ty, package_id, context);
    let mut parser = Parser::lex_file_with_options(
        file.clone(),
        language_type,
        ParserOptions::default(),
        Arc::new(StringPool::new()),
    );
    let expressions = parser.parse();
    context.emit_collection(parser.diagnostics());

    let (tokens, side_tokens) = parser.take_tokens();
    let anchor_expression = insert_anchor_expression(&mut parser.tree, file.id);

    let parsed_file = DirParsedFile {
        file_id: file.id,
        aliases: Vec::new(),
        roots: expressions,
        token_range: 0..tokens.len() as u32,
        side_token_range: 0..side_tokens.len() as u32,
        anchor_expression,
    };

    DirParsed::new(
        parser.tree,
        vec![parsed_file],
        tokens,
        side_tokens,
        anchor_expression,
    )
}

/// Insert one synthetic anchor expression at the start of a file.
fn insert_anchor_expression(
    tree: &mut dir::Tree,
    file_id: FileId,
) -> dir::LocalNodeId<dir::Expression> {
    let span = destack_source::Span::empty(file_id);

    tree.insert(
        dir::Expression::ScalarLiteral(dir::ScalarLiteral::Boolean(false)),
        span,
    )
}

/// Resolve parser language type for one code file.
fn language_type_for_code_file(
    compiler: &Compiler,
    file_type: FileType,
    package_id: PackageId,
    context: &TestProviderContext,
) -> LanguageType {
    assert!(
        file_type.is_code(),
        "non-code file type reached code parser: {file_type:?}",
    );

    if file_type != FileType::JavaScript {
        return LanguageType::try_from(file_type)
            .unwrap_or_else(|_| panic!("code file type has no parser language: {file_type:?}"));
    }

    let Some(package) = compiler
        .repository
        .package(context.revision(), package_id)
        .unwrap_or_else(|error| panic!("failed to load source package: {error}"))
    else {
        return LanguageType::JavaScript;
    };
    if let Some(file_id) = package.destack_file_id {
        let content_id = compiler
            .repository
            .file_content_id(context.revision(), file_id)
            .unwrap_or_else(|error| panic!("failed to load source content id: {error}"))
            .unwrap_or_else(|| panic!("missing source content id for {file_id:?}"));
        context.track(ArtifactDependency::path_state(
            file_id,
            ArtifactPathState::File,
        ));
        context.track(ArtifactDependency::file_content(file_id, content_id));
    }

    LanguageType::JavaScript
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

    /// Require one artifact and return its exact version when ready.
    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        if key == self.artifact_key {
            return Err(ProviderError::RequirementFailed { key });
        }

        let Some(version) = self
            .repository
            .artifact_version(self.revision, &key)
            .expect("linter test provider should read artifact version")
        else {
            return Err(ProviderError::blocked(key));
        };

        match self.repository.artifact_store().outcome(&version) {
            Some(ArtifactOutcome::Ok) => {}
            Some(ArtifactOutcome::Failed(_)) => {
                let mut dependencies = self.dependencies.lock();
                let dependency = ArtifactDependency::artifact(version);
                if !dependencies.iter().any(|existing| *existing == dependency) {
                    dependencies.push(dependency);
                }

                return Err(ProviderError::RequirementFailed { key });
            }
            None => return Err(ProviderError::blocked(key)),
        }

        let mut dependencies = self.dependencies.lock();
        let dependency = ArtifactDependency::artifact(version);
        if !dependencies.iter().any(|existing| *existing == dependency) {
            dependencies.push(dependency);
        }

        Ok(version)
    }

    /// Require many artifacts and return their exact versions when ready.
    fn require_all(&self, keys: &[ArtifactKey]) -> Result<Vec<ArtifactVersion>, ProviderError> {
        let mut blocked = Vec::new();
        let mut versions = Vec::with_capacity(keys.len());

        // scan the full dependency batch
        for key in keys {
            // reject direct cycles
            if *key == self.artifact_key {
                return Err(ProviderError::RequirementFailed { key: *key });
            }

            // read the version bound to this revision
            let Some(version) = self
                .repository
                .artifact_version(self.revision, key)
                .expect("linter test provider should read artifact version")
            else {
                blocked.push(*key);
                continue;
            };

            // inspect the required artifact outcome
            match self.repository.artifact_store().outcome(&version) {
                Some(ArtifactOutcome::Ok) => {
                    let mut dependencies = self.dependencies.lock();
                    let dependency = ArtifactDependency::artifact(version);
                    if !dependencies.iter().any(|existing| *existing == dependency) {
                        dependencies.push(dependency);
                    }

                    versions.push(version);
                }
                Some(ArtifactOutcome::Failed(_)) => {
                    let mut dependencies = self.dependencies.lock();
                    let dependency = ArtifactDependency::artifact(version);
                    if !dependencies.iter().any(|existing| *existing == dependency) {
                        dependencies.push(dependency);
                    }

                    return Err(ProviderError::RequirementFailed { key: *key });
                }
                None => {
                    blocked.push(*key);
                }
            }
        }

        // return the full blocked set together
        if !blocked.is_empty() {
            blocked.sort();
            blocked.dedup();

            return Err(ProviderError::blocked_many(blocked));
        }

        Ok(versions)
    }

    /// Record one exact dependency read by this attempt.
    fn track(&self, dependency: ArtifactDependency) {
        let mut dependencies = self.dependencies.lock();
        if !dependencies.iter().any(|existing| existing == &dependency) {
            dependencies.push(dependency);
        }
    }

    /// Record an already-final diagnostic collection produced by this attempt.
    fn emit_collection(&self, diagnostics: DiagnosticCollection) {
        if diagnostics.is_empty() {
            return;
        }

        self.diagnostics.lock().merge_from(&diagnostics);
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

        self.emit_collection(diagnostics);

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
        Ref::for_workspace_root(self.repository.workspace_root())
    }

    /// Return the current published workspace revision.
    fn current_revision(&self) -> Revision {
        self.repository
            .current(&self.current_reference())
            .expect("workspace revision should be tracked")
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
        I: IntoIterator<Item = RepositoryEdit>,
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
            open_repository_from_fs(cwd.clone(), fs.clone(), environment.clone())
                .expect("failed to import repository from linter test file system")
                .with_cache(TEST_CACHE_STORE.clone()),
        );

        // libs
        let libs = explicit_libs.unwrap_or_else(|| collect_required_libs_from_rules(&rules));

        // profile
        let profile_key = ProfileKey::new(
            EmitFormat::Js,
            Runtime::Js,
            Platform::Unknown,
            Host::Browser,
            None,
            None,
            None,
            libs,
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
            None,
            environment.key_all(),
            ProfileFlags::default(),
        );
        let profile = Profile::from_key(profile_key);

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
        let overlay_path = self.repository.workspace_root().join(path);
        self.fs.set_overlay(&overlay_path, content.to_string());

        self.apply_edits([RepositoryEdit::set_text(path, content)]);
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

    /// Provide compiler artifacts through the session scheduler.
    fn provide_compiler_artifacts(&self, artifact_keys: &[ArtifactKey]) {
        let revision = self.current_revision();
        let mut pending_artifact_keys = artifact_keys.to_vec();

        while let Some(artifact_key) = pending_artifact_keys.pop() {
            if let Some(version) = self
                .repository
                .artifact_version(revision, &artifact_key)
                .expect("linter test provider should read artifact version")
                && self.repository.artifact_store().outcome(&version).is_some()
            {
                continue;
            }

            let context = Arc::new(TestProviderContext::new(
                self.repository.clone(),
                revision,
                artifact_key,
            ));

            match artifact_key.provider() {
                ArtifactProvider::Loader => {
                    let payload = provide_loader_artifact(self.compiler.as_ref(), context.as_ref());
                    context.publish(payload);

                    continue;
                }
                ArtifactProvider::Compiler | ArtifactProvider::Linter | ArtifactProvider::Query => {
                }
            }

            match self.compiler.provide(context.as_ref()) {
                Ok(payload) => context.publish(payload),
                Err(error) => match *error {
                    ProviderError::Blocked { keys } => {
                        pending_artifact_keys.push(artifact_key);
                        pending_artifact_keys.extend(keys);
                    }
                    ProviderError::RequirementFailed { key } => {
                        context.fail(ArtifactFailure::requirement(key));
                    }
                    ProviderError::Corrupt { version } => {
                        panic!(
                            "failed to provide linter test artifact {artifact_key:?}: corrupt artifact {version:?}"
                        );
                    }
                    ProviderError::Failed { failure } => context.fail(failure),
                    ProviderError::Internal { message } => {
                        panic!(
                            "failed to provide linter test artifact {artifact_key:?}: {message}"
                        );
                    }
                },
            }
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

    /// Apply edits to source code and return the result.
    pub(crate) fn apply_edits(&self, edits: Vec<&SourceEdit>) -> String {
        // return original source if no edits
        if edits.is_empty() {
            if let Some(d) = self.diagnostics.first() {
                let file = self.repository_file(d.primary.file);
                return file.text().to_string();
            }
            return String::new();
        }

        // sort by span start, descending (apply from end to preserve offsets)
        let mut sorted_edits = edits;
        sorted_edits.sort_by_key(|b| std::cmp::Reverse(b.span.start));

        // apply all edits to the source
        let file_id = sorted_edits[0].span.file;
        let file = self.repository_file(file_id);
        let mut source = file.text().to_string();
        for edit in sorted_edits {
            let start = edit.span.start as usize;
            let end = edit.span.end as usize;
            source.replace_range(start..end, &edit.new_text);
        }

        source
    }

    /// Apply fixes from diagnostics with the given applicability and return the fixed source.
    pub(crate) fn apply_fixes(&self, applicability: Option<Fixability>) -> String {
        // collect all edits from fixes with matching applicability
        let edits: Vec<&SourceEdit> = self
            .diagnostics
            .iter()
            .flat_map(|d| &d.fixes)
            .filter(|f| applicability.map(|a| f.applicability == a).unwrap_or(true))
            .flat_map(|f| &f.edits)
            .collect();

        let fixed = self.apply_edits(edits);
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
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            return source.to_string();
        }

        // format context
        let side_span = parser.compute_side_span();
        let parents = NodeParentIndex::from_tree(&parser.tree);
        let (tokens, side_tokens) = parser.take_tokens();
        let strings = parser.strings.as_ref();
        let format_options = DestackFormatOptions::default();
        let context = DestackFormatContext::new(
            format_options,
            file.as_ref(),
            &parser.tree,
            &tokens,
            &side_tokens,
            &side_span,
            strings,
            parents,
        );

        // format
        let mut result = if expressions.is_empty() {
            String::new()
        } else {
            let formatted = fir_format!(context.clone(), [statement_list(&expressions)]).unwrap();
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
