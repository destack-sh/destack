/// Requirements for running a MIR pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassRequirements {
    /// Bitmask storing requirement flags.
    bits: u32,
}

impl PassRequirements {
    /// No special requirements.
    pub const NONE: Self = Self { bits: 0 };
    /// Call instructions must carry call effects metadata.
    pub const CALL_EFFECTS: Self = Self { bits: 1 << 0 };
    /// Memory access instructions must carry memory access metadata.
    pub const MEMORY_ACCESS_METADATA: Self = Self { bits: 1 << 1 };
    /// Profile data must be present in the pipeline context.
    pub const PROFILE_DATA: Self = Self { bits: 1 << 2 };
    /// Aggregate types must carry layout metadata.
    pub const TYPE_LAYOUTS: Self = Self { bits: 1 << 3 };

    /// Return true when no requirements are set.
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// Return true when all bits in other are present.
    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }

    /// Return the union of two requirement sets.
    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }
}

impl std::ops::BitOr for PassRequirements {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            bits: self.bits | rhs.bits,
        }
    }
}

impl std::ops::BitOrAssign for PassRequirements {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

/// Static metadata about a MIR pass.
#[derive(Debug, Clone, Copy)]
pub struct PassMetadata {
    /// Pass ID like "constant-fold".
    pub id: &'static str,
    /// Pass name like "ConstantFold".
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Metadata required before this pass can run.
    pub requirements: PassRequirements,
}

/// Base trait for MIR passes.
pub trait Pass: Send + Sync {
    /// Return the static metadata for this pass.
    fn metadata(&self) -> &'static PassMetadata;
}

/// Declare one MIR pass and its static metadata.
#[macro_export]
macro_rules! declare_mir_pass {
    (
        $(#[doc = $doc:literal])*
        #[pass(id = $id:literal $(, requires($($requirement:ident),* $(,)?))?)]
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
                    requirements: $crate::declare_mir_pass!(@requirements $($($requirement),*)?),
            };

            /// Return the pass metadata.
            $visibility const fn metadata() -> &'static $crate::optimize::PassMetadata {
                &Self::METADATA
            }
        }
    };

    (@requirements) => {
        $crate::optimize::PassRequirements::NONE
    };

    (@requirements $first:ident $(, $rest:ident)*) => {
        $crate::declare_mir_pass!(@requirement $first)$(.union($crate::declare_mir_pass!(@requirement $rest)))*
    };

    (@requirement call_effects) => {
        $crate::optimize::PassRequirements::CALL_EFFECTS
    };

    (@requirement memory_access_metadata) => {
        $crate::optimize::PassRequirements::MEMORY_ACCESS_METADATA
    };

    (@requirement profile_data) => {
        $crate::optimize::PassRequirements::PROFILE_DATA
    };

    (@requirement type_layouts) => {
        $crate::optimize::PassRequirements::TYPE_LAYOUTS
    };
}

use destack_mir as mir;

use crate::optimize::{
    PackagePipelineContext, PackageWorkset, PipelineContext, ProgramPipelineContext, ProgramWorkset,
};
use destack_mir::{AnalysisPreservation, FunctionAnalyses, ModuleAnalyses};

/// Trait for optimization passes that operate on individual functions.
pub trait FunctionPass: Pass + Send + Sync {
    /// Run the pass on a function.
    ///
    /// The analysis cache persists across this function's pass sequence; the
    /// pass queries it for the analyses it needs and reports which it preserves.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        context: &PipelineContext<'_>,
        analyses: &FunctionAnalyses,
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
    fn run(
        &self,
        tree: &mut mir::Tree,
        context: &PipelineContext<'_>,
        analyses: &ModuleAnalyses,
    ) -> AnalysisPreservation;

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

    // one analysis cache lives across this function's whole pass sequence
    let analyses = context.new_function_analyses();

    // seal the value counter once on entry; passes maintain it via next_value
    function.recompute_next_value_id(tree);

    let mut changed = false;
    for pass in passes {
        let preservation = pass.run(&mut function, tree, context, &analyses);

        // invalidate whatever this pass did not preserve
        analyses.apply_preservation(&preservation);
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

    // one analysis cache lives across this function's whole pass sequence
    let analyses = context.new_function_analyses();

    // seal the value counter once on entry; passes maintain it via next_value
    function.recompute_next_value_id(tree);

    for pass in passes {
        let preservation = pass.run(&mut function, tree, context, &analyses);
        analyses.apply_preservation(&preservation);
    }

    *tree.get_mut(function_id) = function;
}
