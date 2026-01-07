use std::fmt;

use destack_mir as mir;

use super::context::OptimizationContext;
use super::pass::{BoxedFunctionPass, FunctionPass, OptimizationLevel};
use crate::optimize::passes::{ConstantFold, DeadCodeEliminate};

/// Optimization pipeline that runs passes in sequence.
///
/// The pipeline manages the order of passes and handles analysis invalidation
/// between passes. It can be configured with different optimization levels
/// to control which passes run.
pub struct Pipeline {
    /// Function passes to run on each function.
    function_passes: Vec<BoxedFunctionPass>,
    /// Optimization level.
    level: OptimizationLevel,
}

impl fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pipeline")
            .field("level", &self.level)
            .field("function_pass_count", &self.function_passes.len())
            .finish()
    }
}

impl Pipeline {
    /// Create a new empty pipeline.
    pub fn new(level: OptimizationLevel) -> Self {
        Self {
            function_passes: Vec::new(),
            level,
        }
    }

    /// Get the optimization level.
    pub fn level(&self) -> OptimizationLevel {
        self.level
    }

    /// Add a function pass to the pipeline.
    pub fn add_function_pass<P: FunctionPass + 'static>(&mut self, pass: P) {
        self.function_passes.push(Box::new(pass));
    }

    /// Run the pipeline on a single function.
    pub fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) {
        for pass in &self.function_passes {
            let preserved = pass.run_on_function(function, tree, context);
            context.invalidate(&preserved);
        }
    }

    /// Run the pipeline on all functions in the tree.
    pub fn run_on_module(&self, tree: &mut mir::NodeTree, context: &OptimizationContext<'_>) {
        // collect function IDs first to avoid borrow issues
        let function_ids: Vec<_> = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        for function_id in function_ids {
            let mut function = tree.get(function_id).clone();

            // skip imported functions (no body)
            if function.entry.is_none() {
                continue;
            }

            // clear analyses between functions
            context.clear_analyses();

            // run all passes on this function
            self.run_on_function(&mut function, tree, context);

            // write function back
            *tree.get_mut(function_id) = function;
        }
    }

    /// Get the number of function passes.
    pub fn function_pass_count(&self) -> usize {
        self.function_passes.len()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new(OptimizationLevel::default())
    }
}

/// Build a default pipeline for the given optimization level.
pub fn default_pipeline(level: OptimizationLevel) -> Pipeline {
    let mut pipeline = Pipeline::new(level);

    match level {
        OptimizationLevel::O0 => {
            // no optimizations
        }
        OptimizationLevel::O1 | OptimizationLevel::O2 | OptimizationLevel::O3 => {
            // basic passes for all optimization levels
            pipeline.add_function_pass(ConstantFold);
            pipeline.add_function_pass(DeadCodeEliminate);
        }
    }

    pipeline
}
