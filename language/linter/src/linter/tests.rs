use std::collections::HashMap;
use std::env::current_dir;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Once};

use destack_artifact::{
    ArtifactKey, EmitFormat, EnvironmentStamp, MemoryCacheStore, Platform, ProfileFlags,
    ProfileKey, Runtime,
};
use destack_ast::NodeParentIndex;
use destack_compiler::{Compiler, CompilerOptions};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::Parser;
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, Edit as SourceEdit, File, FileId,
    FileSystem, FileType, LanguageType, ModuleId, OverlayFileSystem, PhysicalFileSystem, Uri,
    print_diff,
};
use destack_workspace::{
    AmbientSnapshot, Change, Edit as RepositoryEdit, LintCategory, LintSeverity, LinterOptions,
    Profile, Ref, Repository, Revision,
};
use parking_lot::Mutex;

use crate::{
    BoxedLintRule, Fixability, LintDiagnostic, LintLevel, LintModuleReport, LintRequirement,
    LintRule, LintRunReport, LintRunner,
};

/// Shared memory cache store for linter tests.
static TEST_CACHE_STORE: LazyLock<Arc<MemoryCacheStore>> =
    LazyLock::new(|| Arc::new(MemoryCacheStore::new()));

/// Process wide per lib-set warmers for prelude profile setup.
static PRELUDE_WARMERS: LazyLock<Mutex<HashMap<Vec<String>, Arc<Once>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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
    /// Whether builtin and lib resolution has been enqueued.
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
            // choose a single lib per requirement: this avoids conflicting ambient sets
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
            .workspace_package_ids(self.current_revision())
            .expect("workspace package ids should load");
        package_ids.sort_unstable();

        *package_ids
            .first()
            .expect("linter tests should have one active package")
    }

    /// Return one module snapshot for the current revision.
    pub(crate) fn repository_module(&self, module_id: ModuleId) -> Arc<destack_workspace::Module> {
        self.repository
            .module(self.current_revision(), module_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| panic!("missing module snapshot for {module_id:?}"))
    }

    /// Return one file snapshot for the current revision.
    pub(crate) fn repository_file(&self, file_id: FileId) -> Arc<File> {
        self.repository
            .file(self.current_revision(), file_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| panic!("missing file snapshot for {file_id:?}"))
    }

    /// Publish one source change to the current workspace revision.
    fn apply_change(&self, change: Change) {
        self.repository
            .apply(&self.current_reference(), change)
            .expect("failed to apply linter test change");
    }

    /// Create a new test repository with the given rules and options.
    fn new(
        rules: Vec<BoxedLintRule>,
        inject_prelude: bool,
        explicit_libs: Option<Vec<String>>,
    ) -> Self {
        let cwd = current_dir().unwrap();
        let fs = Arc::new(OverlayFileSystem::with_inner(Arc::new(
            PhysicalFileSystem::new(),
        )));
        let ambient = AmbientSnapshot::capture_process();

        let repository = Arc::new(
            Repository::open_root_from_fs(cwd.clone(), fs.clone(), ambient.clone())
                .expect("failed to import repository from linter test file system")
                .with_cache(TEST_CACHE_STORE.clone()),
        );

        // libs
        let libs = explicit_libs.unwrap_or_else(|| collect_required_libs_from_rules(&rules));

        // profile
        let profile_key = ProfileKey::new(
            EmitFormat::Js,
            Runtime::Browser,
            Platform::Web,
            None,
            None,
            None,
            libs,
            false,
            false,
            false,
            EnvironmentStamp::from_env_all(),
            ProfileFlags::default(),
        );
        let profile = Profile::from_key(profile_key, &ambient.environment);

        // compiler / runner
        let compiler = Arc::new(Compiler::new(
            repository.clone(),
            CompilerOptions {
                workers: 1,
                inject_prelude,
                ..Default::default()
            },
        ));
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

        self.apply_change(Change::single(RepositoryEdit::set_text(path, content)));
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
            .push(ArtifactKey::DirBase { module });
    }

    /// Resolve a module.
    pub(crate) fn resolve_module(&self, module: ModuleId) {
        self.pending_artifact_keys
            .lock()
            .push(ArtifactKey::DirResolved {
                module,
                profile: self.profile_id(),
            });
    }

    /// Resolve the language environment for the current profile.
    pub(crate) fn resolve_language_environment(&self) {
        self.compiler
            .provide(
                self.current_revision(),
                ArtifactKey::language_environment(self.profile_id()),
            )
            .unwrap_or_else(|error| panic!("failed to resolve language environment: {error:?}"));

        // publish diagnostics from this compiler operation into the test harness
        self.compiler.flush_diagnostics();
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
    }

    /// Resolve builtin libs for the current profile.
    pub(crate) fn resolve_libs(&self) {
        self.compiler
            .provide(
                self.current_revision(),
                ArtifactKey::library_environment(self.profile_id()),
            )
            .unwrap_or_else(|error| panic!("failed to resolve libs: {error:?}"));

        // publish diagnostics from this compiler operation into the test harness
        self.compiler.flush_diagnostics();
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
    }

    /// Enqueue builtin and lib resolution once for this test repository.
    pub(crate) fn enqueue_profile_resolution_once(&self) {
        let has_enqueued = self
            .has_enqueued_profile_resolution
            .swap(true, Ordering::Relaxed);
        if has_enqueued {
            return;
        }

        self.resolve_language_environment();
        self.resolve_libs();
    }

    /// Analyze a module.
    pub(crate) fn analyze_module(&self, module: ModuleId) {
        self.pending_artifact_keys
            .lock()
            .push(ArtifactKey::DirAnalyzed {
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

        for artifact_key in artifact_keys {
            self.compiler
                .provide(self.current_revision(), artifact_key)
                .unwrap_or_else(|error| {
                    panic!("failed to provide linter test artifact: {error:?}")
                });
        }

        // publish diagnostics from this compiler run into the test harness
        self.compiler.flush_diagnostics();
        self.replace_latest_diagnostics(self.current_workspace_diagnostics());
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
            .workspace_module_ids(revision)
            .unwrap_or_else(|error| panic!("failed to read workspace modules: {error}"));
        let mut diagnostics = DiagnosticCollection::new();

        // current workspace families
        for module_id in module_ids {
            diagnostics.merge_from(&self.repository.module_artifact_diagnostics(
                revision,
                module_id,
                self.profile_id(),
            ));
        }

        diagnostics
    }

    /// Lint a module at the given level.
    pub(crate) fn lint_module(&self, module: ModuleId, level: LintLevel) -> Vec<LintDiagnostic> {
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
    pub(crate) fn lint_dir(&self, path: &str, content: &str) -> Vec<LintDiagnostic> {
        let module = self.add_module(path, content);
        self.import_module(module);
        self.enqueue_profile_resolution_once();
        self.analyze_module(module);
        self.compile();
        self.lint_module(module, LintLevel::Dir)
    }

    /// Add module, import only (parse), and lint at AST level.
    pub(crate) fn lint_ast(&self, path: &str, content: &str) -> Vec<LintDiagnostic> {
        let module = self.add_module(path, content);
        self.import_module(module);
        self.compile();
        self.lint_module(module, LintLevel::Ast)
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
    ) -> Vec<LintDiagnostic> {
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
    ) -> Vec<LintDiagnostic> {
        self.prepare_dir_modules(modules);
        self.lint_package_dir()
    }

    /// Add modules, analyze them at DIR level, and lint workspace-scope DIR rules.
    pub(crate) fn lint_workspace_dir_with_modules(
        &self,
        modules: &[(&str, &str)],
    ) -> Vec<LintDiagnostic> {
        self.prepare_dir_modules(modules);
        self.lint_workspace_dir()
    }

    /// Lint the full workspace with workspace-scope rules.
    pub(crate) fn lint_workspace_ast(&self) -> Vec<LintDiagnostic> {
        let revision = self.current_revision();
        self.runner
            .lint_workspace_ast(self.repository.clone(), revision, &self.linter_options)
    }

    /// Lint the full workspace with AST workspace-scope rules and collect performance data.
    pub(crate) fn lint_workspace_ast_profiled(&self) -> LintRunReport {
        let revision = self.current_revision();
        self.runner.lint_workspace_ast_profiled(
            self.repository.clone(),
            revision,
            &self.linter_options,
        )
    }

    /// Lint the active package with package-scope AST rules.
    pub(crate) fn lint_package_ast(&self) -> Vec<LintDiagnostic> {
        let revision = self.current_revision();
        self.runner.lint_package_ast(
            self.repository.clone(),
            revision,
            self.package_id(),
            &self.linter_options,
        )
    }

    /// Lint the active package with package-scope AST rules and collect performance data.
    pub(crate) fn lint_package_ast_profiled(&self) -> LintRunReport {
        let revision = self.current_revision();
        self.runner.lint_package_ast_profiled(
            self.repository.clone(),
            revision,
            self.package_id(),
            &self.linter_options,
        )
    }

    /// Lint the active package with package-scope DIR rules.
    pub(crate) fn lint_package_dir(&self) -> Vec<LintDiagnostic> {
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
    pub(crate) fn lint_workspace_dir(&self) -> Vec<LintDiagnostic> {
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
    pub(crate) fn lint_workspace(&self) -> Vec<LintDiagnostic> {
        let mut diagnostics = self.lint_workspace_ast();
        diagnostics.extend(self.lint_package_ast());
        diagnostics.extend(self.lint_workspace_dir());
        diagnostics.extend(self.lint_package_dir());
        diagnostics
    }

    /// Lint the active workspace and package scope rules and collect performance data.
    pub(crate) fn lint_workspace_profiled(&self) -> LintRunReport {
        let mut ast_report = self.lint_workspace_ast_profiled();
        let package_ast_report = self.lint_package_ast_profiled();
        let workspace_dir_report = self.lint_workspace_dir_profiled();
        let package_dir_report = self.lint_package_dir_profiled();
        ast_report
            .performance
            .merge(&package_ast_report.performance);
        ast_report
            .diagnostics
            .extend(package_ast_report.diagnostics);
        ast_report
            .performance
            .merge(&workspace_dir_report.performance);
        ast_report
            .diagnostics
            .extend(workspace_dir_report.diagnostics);
        ast_report
            .performance
            .merge(&package_dir_report.performance);
        ast_report
            .diagnostics
            .extend(package_dir_report.diagnostics);
        ast_report
    }

    /// Check no compiler diagnostics at or above the given severity.
    #[track_caller]
    pub(crate) fn check_no_diagnostic(&self, min_severity: DiagnosticSeverity) {
        let diagnostics = self.diagnostics();
        let highest = diagnostics.highest_severity();
        if let Some(highest) = highest
            && highest >= min_severity
        {
            self.repository
                .print_diagnostics(self.current_revision(), &diagnostics, 120);
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
    pub(crate) fn result(&self, diagnostics: Vec<LintDiagnostic>) -> LintResult<'_> {
        LintResult::new(
            diagnostics,
            self.repository.as_ref(),
            self.current_revision(),
        )
    }
}

/// Result of linting that can be asserted on.
pub(crate) struct LintResult<'a> {
    diagnostics: Vec<LintDiagnostic>,
    repository: &'a Repository,
    revision: Revision,
}

#[allow(dead_code)]
impl<'a> LintResult<'a> {
    /// Create a new lint result.
    pub(crate) fn new(
        diagnostics: Vec<LintDiagnostic>,
        repository: &'a Repository,
        revision: Revision,
    ) -> Self {
        Self {
            diagnostics,
            repository,
            revision,
        }
    }

    /// Return one file snapshot for one diagnostic file id.
    fn repository_file(&self, file_id: FileId) -> Arc<File> {
        self.repository
            .file(self.revision, file_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| panic!("missing file snapshot for {file_id:?}"))
    }

    /// Get the diagnostics.
    pub(crate) fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }

    /// Print lint diagnostics using the standard diagnostic printer.
    fn print_diagnostics(&self, diagnostics: &[LintDiagnostic]) {
        let mut collection = DiagnosticCollection::new();
        for d in diagnostics {
            collection.insert(d.clone().into_diagnostic());
        }
        self.repository
            .print_diagnostics(self.revision, &collection, 120);
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
            let file = self.repository_file(diagnostic.file_id);
            if let Some((line_index, _)) = file.get_position(diagnostic.span.start) {
                let diagnostic_line = line_index + 1;
                if diagnostic_line == line {
                    return self;
                }
            }
        }

        let lines: Vec<_> = matching
            .iter()
            .filter_map(|d| {
                let file = self.repository_file(d.file_id);
                file.get_position(d.span.start)
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
                let file = self.repository_file(d.file_id);
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
        let mut parser = Parser::lex_file(file.clone(), language_type);
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
        let strings = parser.strings.into_immutable();
        let format_options = DestackFormatOptions::default();
        let context = DestackFormatContext::new(
            format_options,
            file.as_ref(),
            &parser.tree,
            &tokens,
            &side_tokens,
            &side_span,
            &strings,
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
