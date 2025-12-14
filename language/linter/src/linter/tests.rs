use std::env::current_dir;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, CompileOptions, Compiler, ImportTask};
use destack_source::{
    DiagnosticCollection, DiagnosticSeverity, FileRegistry, MemoryFileSystem, ModuleId,
    PrintOptions, print_diagnostics,
};
use destack_workspace::{LinterOptions, Program};

use crate::{BoxedLintRule, LintDiagnostic, LintLevel, LintRunner};

/// Test wrapper for linting.
pub(crate) struct TestProgram {
    /// The file system.
    fs: Arc<MemoryFileSystem>,
    /// The program.
    pub program: Arc<Program>,
    /// The compiler.
    compiler: Arc<Compiler>,
    /// The lint runner.
    runner: LintRunner,
}

impl std::fmt::Debug for TestProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestProgram")
            .field("rules", &self.runner.rules().len())
            .finish()
    }
}

#[allow(dead_code)]
impl TestProgram {
    /// Create a new test program with the given rules.
    pub(crate) fn memory_sequential(rules: Vec<BoxedLintRule>) -> Self {
        let fs = Arc::new(MemoryFileSystem::new());
        let program = Arc::new(Program::new(
            Default::default(),
            LinterOptions::default(),
            current_dir().unwrap(),
            fs.clone(),
            Arc::new(FileRegistry::new()),
        ));
        let compiler = Arc::new(Compiler::new(
            program.clone(),
            CompileOptions {
                workers: 1,
                ..Default::default()
            },
        ));
        let runner = LintRunner::new(rules);

        Self {
            fs,
            program,
            compiler,
            runner,
        }
    }

    /// Create a test with a single rule.
    pub(crate) fn for_rule<R: crate::LintRule + 'static>(rule: R) -> Self {
        Self::memory_sequential(vec![crate::boxed(rule)])
    }

    /// Create a test with all rules.
    pub(crate) fn with_all_rules() -> Self {
        Self::memory_sequential(crate::all_rules())
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
            .lint_module(self.program.clone(), module, &self.program.linter, level)
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
    pub(crate) fn assert_lint(&self, rule_id: &str) {
        let found = self.diagnostics.iter().any(|d| d.rule_id == rule_id);
        if !found {
            let ids: Vec<_> = self.diagnostics.iter().map(|d| d.rule_id).collect();
            self.print_diagnostics(&self.diagnostics);
            panic!("expected lint '{rule_id}' but found: {ids:?}");
        }
    }

    /// Assert diagnostics do not contain a lint with the given rule id.
    #[track_caller]
    pub(crate) fn assert_no_lint(&self, rule_id: &str) {
        let found = self.diagnostics.iter().find(|d| d.rule_id == rule_id);
        if let Some(d) = found {
            panic!("unexpected lint '{rule_id}': {}", d.message);
        }
    }

    /// Assert diagnostics contain exactly n lints with the given rule id.
    #[track_caller]
    pub(crate) fn assert_lint_count(&self, rule_id: &str, expected: usize) {
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
    }

    /// Assert a lint exists at the given line (1-indexed).
    #[track_caller]
    pub(crate) fn assert_lint_at_line(&self, rule_id: &str, line: u32) {
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
                    return;
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
}
