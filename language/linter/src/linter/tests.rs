use std::env::current_dir;
use std::sync::Arc;

use destack_ast::NodeParentIndex;
use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, ImportTask};
use destack_fir::format as fir_format;
use destack_formatter::{DestackFormatContext, DestackFormatOptions};
use destack_parser::Parser;
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, Edit, File, FileId, FileType, LanguageType,
    MemoryFileSystem, ModuleId, PrintOptions, Uri, print_diagnostics,
};
use destack_workspace::{LintCategory, LintSeverity, LinterOptions, Program, Session};

use crate::{
    BoxedLintRule, FixApplicability, LintDiagnostic, LintLevel, LintRunner, all_rules, print_diff,
};

/// Test wrapper for linting.
pub(crate) struct TestProgram {
    /// The file system.
    fs: Arc<MemoryFileSystem>,
    /// The session.
    session: Arc<Session>,
    /// The program.
    pub program: Arc<Program>,
    /// The compiler.
    compiler: Arc<Compiler>,
    /// The lint runner.
    runner: LintRunner,
    /// Linter options for tests (all rules enabled by default).
    linter_options: LinterOptions,
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

#[allow(dead_code)]
impl TestProgram {
    /// Create a new test program with the given rules and options.
    fn new(rules: Vec<BoxedLintRule>, inject_prelude: bool) -> Self {
        let fs = Arc::new(MemoryFileSystem::new());
        let cwd = current_dir().unwrap();

        let session = Arc::new(Session::new(cwd.clone()).with_fs(fs.clone()));
        let program = session.add_root(cwd);

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
            compiler,
            runner,
            linter_options: test_linter_options(),
        }
    }

    /// Create a test program without prelude injection.
    pub(crate) fn new_without_builtins(rules: Vec<BoxedLintRule>) -> Self {
        Self::new(rules, false)
    }

    /// Create a test program with prelude injection.
    pub(crate) fn new_with_builtins(rules: Vec<BoxedLintRule>) -> Self {
        Self::new(rules, true)
    }

    /// Create a test with a single rule (without prelude).
    pub(crate) fn for_rule<R: crate::LintRule + 'static>(rule: R) -> Self {
        Self::new_without_builtins(vec![crate::boxed(rule)])
    }

    /// Create a test with all rules (without prelude).
    pub(crate) fn with_all_rules() -> Self {
        Self::new_without_builtins(all_rules())
    }

    /// Load a lib module set (builder pattern).
    pub(crate) fn with_lib(self, name: &str) -> Self {
        self.session.load_lib(name);
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

    /// Import a module (parse).
    pub(crate) fn import_module(&self, module: ModuleId) {
        self.compiler.enqueue(ImportTask::ImportModule { module });
    }

    /// Analyze a module (bind, resolve, type check).
    pub(crate) fn analyze_module(&self, module: ModuleId) {
        self.compiler.enqueue(AnalyzeTask::AnalyzeModule { module });
    }

    /// Run all queued tasks.
    pub(crate) fn compile(&self) {
        self.compiler.compile();
    }

    /// Lint a module at the given level.
    pub(crate) fn lint_module(&self, module: ModuleId, level: LintLevel) -> Vec<LintDiagnostic> {
        let module = self.program.modules.get(module);
        self.runner
            .lint_module(self.program.clone(), module, &self.linter_options, level)
    }

    /// Add module, compile through analysis, and lint.
    pub(crate) fn lint(&self, path: &str, content: &str, level: LintLevel) -> Vec<LintDiagnostic> {
        let module = self.add_module(path, content);
        self.import_module(module);
        self.analyze_module(module);
        self.compile();
        self.lint_module(module, level)
    }

    /// Add module, import only (parse), and lint at AST level.
    pub(crate) fn lint_ast(&self, path: &str, content: &str) -> Vec<LintDiagnostic> {
        let module = self.add_module(path, content);
        self.import_module(module);
        self.compile();
        self.lint_module(module, LintLevel::Ast)
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
        sorted_edits.sort_by(|a, b| b.span.start.cmp(&a.span.start));

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
    pub(crate) fn apply_fixes(&self, applicability: Option<FixApplicability>) -> String {
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
        let fixed = self.apply_fixes(Some(FixApplicability::Safe));
        let fixed = fixed.trim();
        let expected = expected.trim();
        if fixed != expected {
            print_diff(expected, fixed);
            panic!("fixed code mismatch");
        }
        self
    }

    /// Assert the unsafely fixed code matches expected.
    #[track_caller]
    pub(crate) fn assert_unsafe_fixed(&self, expected: &str) -> &Self {
        let fixed = self.apply_fixes(Some(FixApplicability::Unsafe));
        let fixed = fixed.trim();
        let expected = expected.trim();
        if fixed != expected {
            print_diff(expected, fixed);
            panic!("fixed code mismatch");
        }
        self
    }

    /// Assert the fully fixed code matches expected.
    #[track_caller]
    pub(crate) fn assert_any_fixed(&self, expected: &str) -> &Self {
        let fixed = self.apply_fixes(None);
        let fixed = fixed.trim();
        let expected = expected.trim();
        if fixed != expected {
            print_diff(expected, fixed);
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
        let strings = parser.strings.into_immutable();
        let parents = NodeParentIndex::from_tree(&parser.tree);
        let format_options = DestackFormatOptions::default();
        let context = DestackFormatContext {
            options: format_options,
            file: file.as_ref(),
            tree: &parser.tree,
            source_map: &parser.tree.source_map,
            parents,
            tokens: &parser.tokens,
            side_tokens: &parser.side_tokens,
            side_span: &side_span,
            strings: &strings,
        };

        // format
        let mut result = String::new();
        for (i, expr) in expressions.iter().enumerate() {
            let formatted = fir_format!(context.clone(), [expr]).unwrap();
            let printed = formatted.print().unwrap();
            result.push_str(printed.as_str());
            if i < expressions.len() - 1 {
                result.push('\n');
            }
        }

        // ensure trailing newline
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }

        result
    }
}
