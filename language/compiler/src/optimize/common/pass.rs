use destack_mir as mir;

use crate::common::mir::{AnalysisPreservation, Pass};
use crate::optimize::{
    PackagePipelineContext, PackageWorkset, PipelineContext, ProgramPipelineContext, ProgramWorkset,
};

/// Trait for optimization passes that operate on individual functions.
pub trait FunctionPass: Pass + Send + Sync {
    /// Run the pass on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        context: &PipelineContext<'_>,
    ) -> AnalysisPreservation;

    /// Return the pass name.
    fn name(&self) -> &'static str;

    /// Return the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for optimization passes that operate on one module.
pub trait ModulePass: Pass + Send + Sync {
    /// Run the pass on a module.
    fn run(&self, tree: &mut mir::Tree, context: &PipelineContext<'_>) -> AnalysisPreservation;

    /// Return the pass name.
    fn name(&self) -> &'static str;

    /// Return the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for optimization passes that operate on one package.
pub trait PackagePass: Pass + Send + Sync {
    /// Run the pass on a package workset.
    fn run(
        &self,
        workset: &mut PackageWorkset,
        context: &PackagePipelineContext,
    ) -> AnalysisPreservation;

    /// Return the pass name.
    fn name(&self) -> &'static str;

    /// Return the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for optimization passes that operate on one program.
pub trait ProgramPass: Pass + Send + Sync {
    /// Run the pass on a program workset.
    fn run(
        &self,
        workset: &mut ProgramWorkset,
        context: &ProgramPipelineContext,
    ) -> AnalysisPreservation;

    /// Return the pass name.
    fn name(&self) -> &'static str;

    /// Return the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Run function passes on one cloned function.
pub fn run_function_passes(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::Tree,
    context: &PipelineContext<'_>,
    passes: &[&dyn FunctionPass],
) -> bool {
    let mut function = tree.get(function_id).clone();
    if function.entry.is_none() {
        return false;
    }

    let mut changed = false;
    for pass in passes {
        function.recompute_next_value_id(tree);
        let preservation = pass.run(&mut function, tree, context);
        if !preservation.preserves_all() {
            changed = true;
        }
    }

    if changed {
        *tree.get_mut(function_id) = function;
    }

    changed
}

/// Run function passes on one cloned function and write it back.
pub fn run_function_passes_always(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::Tree,
    context: &PipelineContext<'_>,
    passes: &[&dyn FunctionPass],
) {
    let mut function = tree.get(function_id).clone();
    if function.entry.is_none() {
        return;
    }

    for pass in passes {
        function.recompute_next_value_id(tree);
        pass.run(&mut function, tree, context);
    }

    *tree.get_mut(function_id) = function;
}
