use std::fmt;

use destack_mir as mir;

use super::context::OptimizationContext;
use super::pass::{
    BoxedFunctionPass, BoxedModulePass, FunctionPass, ModulePass, OptimizationLevel,
};
use crate::optimize::passes::{
    ConstantFold, CopyPropagate, DeadCodeEliminate, GlobalValueNumbering, InstructionCombine, Licm,
    LocalCse, LoopDelete, LoopRotate, LoopSimplify, LoopUnswitch, Mem2Reg, SimplifyCfg, Sink,
};

/// Optimization pipeline that runs passes in sequence.
///
/// The pipeline manages the order of passes and handles analysis invalidation
/// between passes. It can be configured with different optimization levels
/// to control which passes run.
pub struct Pipeline {
    /// Module passes to run on the entire module.
    module_passes: Vec<BoxedModulePass>,
    /// Function passes to run on each function.
    function_passes: Vec<BoxedFunctionPass>,
    /// Optimization level.
    level: OptimizationLevel,
}

impl fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pipeline")
            .field("level", &self.level)
            .field("module_passes", &self.module_passes.len())
            .field("function_passes", &self.function_passes.len())
            .finish()
    }
}

impl Pipeline {
    /// Create a new empty pipeline.
    pub fn new(level: OptimizationLevel) -> Self {
        Self {
            module_passes: Vec::new(),
            function_passes: Vec::new(),
            level,
        }
    }

    /// Get the optimization level.
    pub fn level(&self) -> OptimizationLevel {
        self.level
    }

    /// Add a module pass to the pipeline.
    pub fn add_module_pass<P: ModulePass + 'static>(&mut self, pass: P) {
        self.module_passes.push(Box::new(pass));
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
        // run module passes first
        for pass in &self.module_passes {
            let preserved = pass.run_on_module(tree, context);
            context.invalidate(&preserved);
        }

        // then run function passes on each function
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

    /// Get the number of module passes.
    pub fn module_pass_count(&self) -> usize {
        self.module_passes.len()
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
            // phase 1: initial simplifications
            pipeline.add_function_pass(ConstantFold);
            pipeline.add_function_pass(InstructionCombine);

            // phase 2: promote to SSA (enables better optimizations)
            pipeline.add_function_pass(Mem2Reg);

            // phase 3: redundancy elimination
            pipeline.add_function_pass(LocalCse);
            pipeline.add_function_pass(GlobalValueNumbering);
            pipeline.add_function_pass(CopyPropagate);

            // phase 4: loop optimizations
            pipeline.add_function_pass(LoopSimplify);
            pipeline.add_function_pass(Licm);
            pipeline.add_function_pass(LoopRotate);
            pipeline.add_function_pass(LoopUnswitch);
            pipeline.add_function_pass(LoopDelete);

            // phase 5: code placement
            pipeline.add_function_pass(Sink);

            // phase 6: cleanup
            pipeline.add_function_pass(SimplifyCfg);
            pipeline.add_function_pass(DeadCodeEliminate);
        }
    }

    pipeline
}
