use std::collections::HashMap;
use std::env::current_dir;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Once};

use destack_ast::NodeParentIndex;
use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, ImportTask, ResolveTask};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions, statement_list};
use destack_parser::Parser;
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, DiffOptions, Edit, File, FileId, FileType,
    LanguageType, MemoryFileSystem, ModuleId, ModuleStamp, PrintOptions, ProfileStamp, Uri,
    print_diagnostics, print_diff,
};
use destack_workspace::{
    EnvSnapshot, LintCategory, LintSeverity, LinterOptions, MemoryCacheStore, OutputFormat,
    Platform, ProfileFlags, ProfileId, ProfileKey, Program, Runtime, Session,
};
use parking_lot::Mutex;

use crate::{
    BoxedLintRule, Fixability, LintDiagnostic, LintLevel, LintModuleReport, LintRequirement,
    LintRunReport, LintRunner,
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
    fs: Arc<MemoryFileSystem>,
    /// The session.
    session: Arc<Session>,
    /// The program.
    pub program: Arc<Program>,
    /// The profile id for tests.
    profile_id: ProfileId,
    /// The compiler.
    compiler: Arc<Compiler>,
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
    /// Create a new test program with the given rules and options.
    fn new(
        rules: Vec<BoxedLintRule>,
        inject_prelude: bool,
        explicit_libs: Option<Vec<String>>,
    ) -> Self {
        let fs = Arc::new(MemoryFileSystem::new());
        let cwd = current_dir().unwrap();

        let session = Arc::new(
            Session::new(cwd.clone())
                .with_fs(fs.clone())
                .with_cache_store(TEST_CACHE_STORE.clone()),
        );
        let program = session.add_root(cwd);

        // libs
        let libs = explicit_libs.unwrap_or_else(|| collect_required_libs_from_rules(&rules));

        // profile
        let profile_key = ProfileKey::new(
            OutputFormat::Js,
            Runtime::Browser,
            Platform::Web,
            None,
            None,
            None,
            libs,
            false,
            false,
            EnvSnapshot::from_env_all(),
            ProfileFlags::default(),
        );
        let profile_id = program.profiles.get_or_create(profile_key);

        // compiler / runner
        let compiler = Arc::new(Compiler::new(
            session.clone(),
            program.clone(),
            CompilerOptions {
                workers: 1,
                inject_prelude,
                ..Default::default()
            },
        ));
        let runner = LintRunner::new(rules);

        Self {
            fs,
            session,
            program,
            profile_id,
            compiler,
            runner,
            linter_options: test_linter_options(),
            has_enqueued_profile_resolution: AtomicBool::new(false),
        }
    }

    /// Create a test program without prelude injection.
    pub(crate) fn new_without_prelude(rules: Vec<BoxedLintRule>) -> Self {
        Self::new(rules, false, None)
    }

    /// Create a test program with prelude injection.
    pub(crate) fn new_with_prelude(rules: Vec<BoxedLintRule>) -> Self {
        Self::new(rules, true, None)
    }

    /// Create a prelude test program with explicit libs.
    fn new_with_prelude_libs(libs: Vec<String>) -> Self {
        Self::new(Vec::new(), true, Some(libs))
    }

    /// Create a test with a single rule (without prelude).
    pub(crate) fn for_rule_without_prelude<R: crate::LintRule + 'static>(rule: R) -> Self {
        Self::new_without_prelude(vec![crate::boxed(rule)])
    }

    /// Create a test with a single rule (with prelude).
    pub(crate) fn for_rule_with_prelude<R: crate::LintRule + 'static>(rule: R) -> Self {
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

    /// Add a file to the filesystem.
    pub(crate) fn add_file(&self, path: &str, content: &str) {
        self.fs.add_file(path, content.as_bytes()).unwrap();
    }

    /// Add a module and return its id.
    pub(crate) fn add_module(&self, path: &str, content: &str) -> ModuleId {
        self.add_file(path, content);
        self.compiler
            .resolve_path_to_module(&std::path::PathBuf::from(path))
            .unwrap()
    }

    /// Import a module.
    pub(crate) fn import_module(&self, module: ModuleId) {
        let module_stamp =
            ModuleStamp::new(module, self.program.modules.get(module).read().version);
        self.compiler.enqueue(ImportTask::ImportModule {
            module: module_stamp,
        });
    }

    /// Resolve a module.
    pub(crate) fn resolve_module(&self, module: ModuleId) {
        let module_stamp =
            ModuleStamp::new(module, self.program.modules.get(module).read().version);
        let profile_version = self
            .program
            .profiles
            .get(self.profile_id)
            .unwrap_or_else(|| panic!("missing profile data for {:?}", self.profile_id))
            .version;
        let profile_stamp = ProfileStamp::new(self.profile_id, profile_version);
        let graph_stamp = self.compiler.module_graph_stamp(self.profile_id);
        self.compiler.enqueue(ResolveTask::ResolveModuleCanonical {
            module: module_stamp,
            profile: profile_stamp,
            graph: graph_stamp,
        });
    }

    /// Resolve builtin language items for the current profile.
    pub(crate) fn resolve_builtins(&self) {
        let profile_version = self
            .program
            .profiles
            .get(self.profile_id)
            .unwrap_or_else(|| panic!("missing profile data for {:?}", self.profile_id))
            .version;
        let profile_stamp = ProfileStamp::new(self.profile_id, profile_version);
        self.compiler.enqueue(ResolveTask::ResolveBuiltins {
            profile: profile_stamp,
        });
    }

    /// Resolve builtin libs for the current profile.
    pub(crate) fn resolve_libs(&self) {
        let profile_version = self
            .program
            .profiles
            .get(self.profile_id)
            .unwrap_or_else(|| panic!("missing profile data for {:?}", self.profile_id))
            .version;
        let profile_stamp = ProfileStamp::new(self.profile_id, profile_version);
        self.compiler.enqueue(ResolveTask::ResolveLibs {
            profile: profile_stamp,
        });
    }

    /// Enqueue builtin and lib resolution once for this test program.
    pub(crate) fn enqueue_profile_resolution_once(&self) {
        let has_enqueued = self
            .has_enqueued_profile_resolution
            .swap(true, Ordering::Relaxed);
        if has_enqueued {
            return;
        }

        self.resolve_builtins();
        self.resolve_libs();
    }

    /// Analyze a module.
    pub(crate) fn analyze_module(&self, module: ModuleId) {
        let module_stamp =
            ModuleStamp::new(module, self.program.modules.get(module).read().version);
        let profile_version = self
            .program
            .profiles
            .get(self.profile_id)
            .unwrap_or_else(|| panic!("missing profile data for {:?}", self.profile_id))
            .version;
        let profile_stamp = ProfileStamp::new(self.profile_id, profile_version);
        self.compiler.enqueue(AnalyzeTask::AnalyzeModule {
            module: module_stamp,
            profile: profile_stamp,
        });
    }

    /// Run all queued tasks.
    pub(crate) fn compile(&self) {
        self.compiler.compile();
    }

    /// Lint a module at the given level.
    pub(crate) fn lint_module(&self, module: ModuleId, level: LintLevel) -> Vec<LintDiagnostic> {
        let module = self.program.modules.get(module);
        let profile = self.profile_id;
        self.runner.lint_module(
            self.program.clone(),
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
        let module = self.program.modules.get(module);
        let profile = self.profile_id;
        self.runner.lint_module_profiled(
            self.program.clone(),
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

    /// Add modules, analyze them at DIR level, and lint program-scope DIR rules.
    pub(crate) fn lint_program_dir_with_modules(
        &self,
        modules: &[(&str, &str)],
    ) -> Vec<LintDiagnostic> {
        self.prepare_dir_modules(modules);
        self.lint_program_dir()
    }

    /// Lint the full program with program-scope rules.
    pub(crate) fn lint_program_ast(&self) -> Vec<LintDiagnostic> {
        self.runner
            .lint_program_ast(self.program.clone(), &self.linter_options)
    }

    /// Lint the full program with AST program-scope rules and collect performance data.
    pub(crate) fn lint_program_ast_profiled(&self) -> LintRunReport {
        self.runner
            .lint_program_ast_profiled(self.program.clone(), &self.linter_options)
    }

    /// Lint the full program with DIR program-scope rules.
    pub(crate) fn lint_program_dir(&self) -> Vec<LintDiagnostic> {
        self.runner
            .lint_program_dir(self.program.clone(), self.profile_id, &self.linter_options)
    }

    /// Lint the full program with DIR program-scope rules and collect performance data.
    pub(crate) fn lint_program_dir_profiled(&self) -> LintRunReport {
        self.runner.lint_program_dir_profiled(
            self.program.clone(),
            self.profile_id,
            &self.linter_options,
        )
    }

    /// Lint the full program with program-scope rules.
    pub(crate) fn lint_program(&self) -> Vec<LintDiagnostic> {
        let mut diagnostics = self.lint_program_ast();
        diagnostics.extend(self.lint_program_dir());
        diagnostics
    }

    /// Lint the full program with program-scope rules and collect performance data.
    pub(crate) fn lint_program_profiled(&self) -> LintRunReport {
        let mut ast_report = self.lint_program_ast_profiled();
        let dir_report = self.lint_program_dir_profiled();
        ast_report.performance.merge(&dir_report.performance);
        ast_report.diagnostics.extend(dir_report.diagnostics);
        ast_report
    }

    /// Check no compiler diagnostics at or above the given severity.
    #[track_caller]
    pub(crate) fn check_no_diagnostic(&self, min_severity: DiagnosticSeverity) {
        let diagnostics = self.program.diagnostics.collect();
        let highest = diagnostics.highest_severity();
        if let Some(highest) = highest
            && highest >= min_severity
        {
            let options = PrintOptions::new()
                .with_line_width(self.program.formatter.line_width as u32)
                .with_module_count(self.program.modules.len());
            print_diagnostics(&self.program.files, &diagnostics, options);
            let severity_name = min_severity.family_name().to_ascii_lowercase();
            panic!(
                "program has {} unexpected {severity_name}s",
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
        LintResult::new(diagnostics, &self.program)
    }
}

/// Result of linting that can be asserted on.
pub(crate) struct LintResult<'a> {
    diagnostics: Vec<LintDiagnostic>,
    program: &'a Program,
}

#[allow(dead_code)]
impl<'a> LintResult<'a> {
    /// Create a new lint result.
    pub(crate) fn new(diagnostics: Vec<LintDiagnostic>, program: &'a Program) -> Self {
        Self {
            diagnostics,
            program,
        }
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
        let options = PrintOptions::new()
            .with_line_width(self.program.formatter.line_width as u32)
            .with_module_count(self.program.modules.len());
        print_diagnostics(&self.program.files, &collection, options);
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
            let file = self.program.files.get(diagnostic.file_id);
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
                let file = self.program.files.get(d.file_id);
                file.get_position(d.span.start)
                    .map(|(line_index, _)| line_index + 1)
            })
            .collect();
        self.print_diagnostics(&self.diagnostics);
        panic!("expected lint '{rule_id}' at line {line} but found at lines: {lines:?}");
    }

    /// Apply edits to source code and return the result.
    pub(crate) fn apply_edits(&self, edits: Vec<&Edit>) -> String {
        // return original source if no edits
        if edits.is_empty() {
            if let Some(d) = self.diagnostics.first() {
                return self.program.files.get(d.file_id).text().to_string();
            }
            return String::new();
        }

        // sort by span start, descending (apply from end to preserve offsets)
        let mut sorted_edits = edits;
        sorted_edits.sort_by_key(|b| std::cmp::Reverse(b.span.start));

        // apply all edits to the source
        let file_id = sorted_edits[0].span.file;
        let file = self.program.files.get(file_id);
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
        let edits: Vec<&Edit> = self
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
        parser.finish();

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
