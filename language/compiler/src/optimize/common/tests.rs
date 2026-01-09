//! Test utilities for optimization passes.

use destack_base::{ImmutableStringPool, StringPool};
use destack_mir as mir;
use destack_source::{DiffOptions, print_diff};

use crate::optimize::{FunctionPass, OptimizationContext, OptimizeOptions};

/// Test program for optimization passes.
///
/// Parses MIR from text, applies passes, and formats the result back to text.
pub(crate) struct TestProgram {
    /// The MIR node tree.
    pub(crate) tree: mir::NodeTree,
    /// The string pool for identifiers (immutable, from parser).
    strings: ImmutableStringPool,
    /// Thread-safe string pool for optimization context.
    strings_pool: StringPool,
}

impl TestProgram {
    /// Create a new test program from MIR source text.
    pub(crate) fn new(source: &str) -> Self {
        let (tree, strings) = mir::parse::Parser::parse(source).expect("failed to parse MIR");
        // Create a separate StringPool for the optimization context.
        // The optimizer doesn't currently use strings, but the context requires one.
        let strings_pool = StringPool::new();
        Self {
            tree,
            strings,
            strings_pool,
        }
    }

    /// Apply a function pass to all functions in the program.
    pub(crate) fn run_pass<P: FunctionPass + ?Sized>(&mut self, pass: &P) {
        let context = OptimizationContext::new(&self.strings_pool, OptimizeOptions::default());

        // collect function IDs
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

            // recompute next_value_id after parsing so passes can allocate fresh values
            function.recompute_next_value_id(&self.tree);

            pass.run_on_function(&mut function, &mut self.tree, &context);

            // write function back
            *self.tree.get_mut(function_id) = function;
        }
    }

    /// Format the MIR back to text.
    pub(crate) fn format(&self) -> String {
        mir::format_mir(&self.tree, &self.strings, mir::MirFormatOptions::default())
    }

    /// Assert that the current MIR matches the expected output after formatting.
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

    /// Create an optimization context for analysis tests.
    pub(crate) fn context(&self) -> OptimizationContext<'_> {
        OptimizationContext::new(&self.strings_pool, OptimizeOptions::default())
    }
}
