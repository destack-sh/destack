use destack_base::{ImmutableStringPool, StringPool};
use destack_mir as mir;
use destack_source::{DiffOptions, ModuleId, PackageId, print_diff};
use destack_workspace::TargetId;

use crate::optimize::{FunctionPass, ModulePass, OptimizationContext, OptimizeOptions};
use crate::{OptimizeError, OptimizeWarning};

/// Placeholder module id for tests.
fn test_module_id() -> ModuleId {
    ModuleId::new(PackageId::new(0), 0)
}

/// Placeholder target id for tests.
fn test_target_id() -> TargetId {
    TargetId::new(PackageId::new(0), "test")
}

/// Test program for optimization passes.
///
/// Parses MIR from text, applies passes, and formats the result back to text.
pub(crate) struct TestProgram {
    /// The MIR node tree.
    pub(crate) tree: mir::NodeTree,
    /// String pool for identifiers (immutable, from parser).
    strings: ImmutableStringPool,
    /// Thread-safe string pool for optimization context.
    strings_pool: StringPool,
    /// Errors collected from the last pass run.
    errors: Vec<OptimizeError>,
    /// Warnings collected from the last pass run.
    warnings: Vec<OptimizeWarning>,
}

impl TestProgram {
    /// Create a new test program from MIR source text.
    pub(crate) fn new(source: &str) -> Self {
        let (tree, strings) = mir::parse::Parser::parse(source).expect("failed to parse MIR");
        let strings_pool = StringPool::new();

        Self {
            tree,
            strings,
            strings_pool,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Apply a function pass to all functions in the program.
    pub(crate) fn run_pass<P: FunctionPass + ?Sized>(&mut self, pass: &P) {
        self.run_pass_with_options_impl(pass, OptimizeOptions::default());
    }

    /// Apply a function pass with custom options.
    pub(crate) fn run_pass_with_options<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
        options: OptimizeOptions,
    ) {
        self.run_pass_with_options_impl(pass, options);
    }

    /// Internal implementation that handles the borrow correctly.
    fn run_pass_with_options_impl<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
        options: OptimizeOptions,
    ) {
        let context = OptimizationContext::new(
            &self.strings_pool,
            options,
            test_module_id(),
            test_target_id(),
        );

        // collect function ids
        let function_ids: Vec<_> = self
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        // run pass on each function
        for function_id in function_ids {
            let mut function = self.tree.get(function_id).clone();

            // skip imported functions (no body)
            if function.entry.is_none() {
                continue;
            }

            // clear analysis cache between functions (analyses are per-function)
            context.analyses.clear();

            // recompute next_value_id so passes can allocate fresh values
            function.recompute_next_value_id(&self.tree);
            pass.run_on_function(&mut function, &mut self.tree, &context);
            *self.tree.get_mut(function_id) = function;
        }

        // collect diagnostics after pass completes
        self.errors = context.take_errors();
        self.warnings = context.take_warnings();
    }

    /// Apply a module pass to the program.
    pub(crate) fn run_module_pass<P: ModulePass + ?Sized>(&mut self, pass: &P) {
        let context = OptimizationContext::new(
            &self.strings_pool,
            OptimizeOptions::default(),
            test_module_id(),
            test_target_id(),
        );

        pass.run_on_module(&mut self.tree, &context);

        // collect diagnostics after pass completes
        self.errors = context.take_errors();
        self.warnings = context.take_warnings();
    }

    /// Format the MIR back to text.
    pub(crate) fn format(&self) -> String {
        mir::format_mir(&self.tree, &self.strings, mir::MirFormatOptions::default())
    }

    /// Create an optimization context for analysis tests.
    pub(crate) fn context(&self) -> OptimizationContext<'_> {
        OptimizationContext::new(
            &self.strings_pool,
            OptimizeOptions::default(),
            test_module_id(),
            test_target_id(),
        )
    }

    /// Get string by id from the string pool.
    pub(crate) fn get_string(&self, id: destack_base::StringId) -> &str {
        self.strings.get(id)
    }

    /// Get errors from the last pass run.
    #[allow(dead_code)]
    pub(crate) fn errors(&self) -> &[OptimizeError] {
        &self.errors
    }

    /// Get warnings from the last pass run.
    #[allow(dead_code)]
    pub(crate) fn warnings(&self) -> &[OptimizeWarning] {
        &self.warnings
    }

    // ================================================================================
    // assertions
    // ================================================================================

    /// Assert that the current MIR matches the expected output.
    #[track_caller]
    pub(crate) fn assert_output(&self, expected: &str) {
        let actual = self.format();
        let expected = expected.trim();
        let actual = actual.trim();

        if actual != expected {
            print_diff(expected, actual, &DiffOptions::new());
            panic!("optimization output mismatch");
        }
    }

    /// Assert that the MIR is unchanged from the original source.
    #[track_caller]
    pub(crate) fn assert_unchanged(&self, original: &str) {
        self.assert_output(original);
    }

    /// Assert that no errors were emitted.
    #[track_caller]
    pub(crate) fn assert_no_errors(&self) {
        if !self.errors.is_empty() {
            panic!("expected no errors, got: {:?}", self.errors);
        }
    }

    /// Assert that no warnings were emitted.
    #[allow(dead_code)]
    #[track_caller]
    pub(crate) fn assert_no_warnings(&self) {
        if !self.warnings.is_empty() {
            panic!("expected no warnings, got: {:?}", self.warnings);
        }
    }

    /// Assert that at least one error matches the predicate.
    #[track_caller]
    pub(crate) fn assert_error<F>(&self, predicate: F)
    where
        F: Fn(&OptimizeError) -> bool,
    {
        let has_match = self.errors.iter().any(|e| predicate(e));

        if !has_match {
            panic!(
                "expected an error matching predicate, got: {:?}",
                self.errors
            );
        }
    }

    /// Assert that at least one warning matches the predicate.
    #[allow(dead_code)]
    #[track_caller]
    pub(crate) fn assert_warning<F>(&self, predicate: F)
    where
        F: Fn(&OptimizeWarning) -> bool,
    {
        let has_match = self.warnings.iter().any(|w| predicate(w));

        if !has_match {
            panic!(
                "expected a warning matching predicate, got: {:?}",
                self.warnings
            );
        }
    }

    // ================================================================================
    // legacy compatibility
    // ================================================================================

    /// Apply a function pass and return any errors emitted.
    ///
    /// Prefer using `run_pass` followed by `assert_*` methods instead.
    #[allow(dead_code)]
    pub(crate) fn run_pass_collecting_errors<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
    ) -> Vec<OptimizeError> {
        self.run_pass(pass);
        self.errors.clone()
    }
}
