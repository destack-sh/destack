/// Static metadata about a MIR pass.
#[derive(Debug, Clone, Copy)]
pub struct PassMetadata {
    /// Pass ID like "fold-constants".
    pub id: &'static str,
    /// Pass name like "FoldConstants".
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
}

/// Base trait for MIR passes.
pub trait Pass: Send + Sync {
    /// Return the static metadata for this pass.
    fn metadata(&self) -> &'static PassMetadata;
}

/// Declare one MIR pass and its static metadata.
macro_rules! declare_pass {
    (
        $(#[doc = $doc:literal])*
        #[pass(id = $id:literal)]
        $visibility:vis $name:ident,
        $description:literal $(,)?
    ) => {
        $(#[doc = $doc])*
        #[derive(Debug, Clone, Copy)]
        $visibility struct $name;

        impl $crate::optimize::Pass for $name {
            fn metadata(&self) -> &'static $crate::optimize::PassMetadata {
                Self::metadata()
            }
        }

        impl $name {
            /// Static metadata for this pass.
            $visibility const METADATA: $crate::optimize::PassMetadata =
                $crate::optimize::PassMetadata {
                    id: $id,
                    name: stringify!($name),
                    description: $description,
            };

            /// Return the pass metadata.
            $visibility const fn metadata() -> &'static $crate::optimize::PassMetadata {
                &Self::METADATA
            }
        }
    };

}

pub(crate) use declare_pass;

use destack_mir as mir;

use crate::optimize::{
    MirOptimized, PackagePipelineContext, PackageWorkset, PipelineContext, ProgramPipelineContext,
    ProgramWorkset,
};
use destack_mir::{FunctionAnalyses, ModuleAnalyses, Mutation};

/// Trait for optimization passes that operate on individual functions.
pub trait FunctionPass: Pass + Send + Sync {
    /// Run the pass on a function.
    ///
    /// Analyses persist across this function's pass sequence.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        context: &PipelineContext<'_>,
        analyses: &mut FunctionAnalyses,
    ) -> Mutation;

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
    fn run(
        &self,
        optimized: &mut MirOptimized,
        context: &PipelineContext<'_>,
        analyses: &mut ModuleAnalyses,
    ) -> Mutation;

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
    fn run(&self, workset: &mut PackageWorkset, context: &PackagePipelineContext) -> Mutation;

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
    fn run(&self, workset: &mut ProgramWorkset, context: &ProgramPipelineContext) -> Mutation;

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
    optimized: &mut MirOptimized,
    context: &PipelineContext<'_>,
    passes: &[&dyn FunctionPass],
) -> bool {
    let mut function = optimized.tree.get(function_id).clone();
    if function.entry().is_none() {
        return false;
    }

    // retain analyses across the function pass sequence
    let mut analyses = FunctionAnalyses::with_options(context.options.analysis);

    // seal the value counter once on entry; passes maintain it via next_value
    function.recompute_next_value_id(&optimized.tree);
    function.rebuild_instruction_index(&optimized.tree);

    let mut changed = false;
    for pass in passes {
        let mutation = pass.run(&mut function, optimized, context, &mut analyses);

        // drop the analyses this pass's mutation invalidates
        analyses.invalidate(mutation);
        if !mutation.is_none() {
            function.rebuild_instruction_index(&optimized.tree);
            changed = true;
        }
    }

    if changed {
        *optimized.tree.get_mut(function_id) = function;
    }

    changed
}

/// Run function passes on one cloned function and write it back.
pub fn run_function_passes_always(
    function_id: mir::LocalNodeId<mir::Function>,
    optimized: &mut MirOptimized,
    context: &PipelineContext<'_>,
    passes: &[&dyn FunctionPass],
) {
    let mut function = optimized.tree.get(function_id).clone();
    if function.entry().is_none() {
        return;
    }

    // retain analyses across the function pass sequence
    let mut analyses = FunctionAnalyses::with_options(context.options.analysis);

    // seal the value counter once on entry; passes maintain it via next_value
    function.recompute_next_value_id(&optimized.tree);
    function.rebuild_instruction_index(&optimized.tree);

    for pass in passes {
        let mutation = pass.run(&mut function, optimized, context, &mut analyses);
        analyses.invalidate(mutation);
        if !mutation.is_none() {
            function.rebuild_instruction_index(&optimized.tree);
        }
    }

    *optimized.tree.get_mut(function_id) = function;
}
