use destack_mir as mir;

use crate::optimize::{
    AnalysisPreservation, PackagePipelineContext, PackageWorkset, PipelineContext,
    ProgramPipelineContext, ProgramWorkset,
};

/// Requirements for running a pass in optimized pipelines.
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

/// Optimization level.
///
/// Controls which passes run and how aggressive they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizationLevel {
    /// Debug: no optimizations.
    #[default]
    O0,
    /// Comptime/dev: fast local passes (constant fold, DCE, simplify CFG).
    O1,
    /// Release: full suite (inlining, escape analysis, devirtualization).
    O2,
    /// Hot paths: aggressive thresholds, loop unrolling.
    O3,
    /// Maximal optimization with extra fixed point rounds and optional LTO.
    O4,
}

impl std::str::FromStr for OptimizationLevel {
    type Err = ();

    /// Parse from string (e.g., "O2" or "2").
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "O0" | "0" => Ok(Self::O0),
            "O1" | "1" => Ok(Self::O1),
            "O2" | "2" => Ok(Self::O2),
            "O3" | "3" => Ok(Self::O3),
            "O4" | "4" => Ok(Self::O4),
            _ => Err(()),
        }
    }
}

/// Static metadata about an optimization pass.
#[derive(Debug, Clone, Copy)]
pub struct PassMetadata {
    /// Pass ID like "constant-fold".
    pub id: &'static str,
    /// Name like "ConstantFold".
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Metadata requirements for optimized pipelines.
    pub requirements: PassRequirements,
}

/// Base trait for all optimization passes.
///
/// Provides access to pass metadata.
pub trait Pass: Send + Sync {
    /// Get the static metadata for this pass.
    fn metadata(&self) -> &'static PassMetadata;
}

/// Trait for passes that operate on individual functions.
///
/// Returns AnalysisPreservation to indicate what analyses are still valid.
pub trait FunctionPass: Pass + Send + Sync {
    /// Run the pass on a function.
    ///
    /// The context provides access to strings, options, and diagnostic emission.
    /// Passes that need analyses can create `FunctionAnalyses::new(func, tree)`.
    fn run(
        &self,
        func: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation;

    /// Get the pass name.
    fn name(&self) -> &'static str;

    /// Get the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for passes that operate on entire modules.
///
/// Returns AnalysisPreservation to indicate what analyses are still valid.
pub trait ModulePass: Pass + Send + Sync {
    /// Run the pass on a module.
    ///
    /// The context provides access to strings, options, analyses, and diagnostics.
    fn run(&self, tree: &mut mir::Tree, ctx: &PipelineContext<'_>) -> AnalysisPreservation;

    /// Get the pass name.
    fn name(&self) -> &'static str;

    /// Get the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for passes that operate on packages.
///
/// Returns AnalysisPreservation to indicate what analyses are still valid.
pub trait PackagePass: Pass + Send + Sync {
    /// Run the pass on a package workset.
    fn run(
        &self,
        workset: &mut PackageWorkset,
        ctx: &PackagePipelineContext,
    ) -> AnalysisPreservation;

    /// Get the pass name.
    fn name(&self) -> &'static str;

    /// Get the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for passes that operate on programs.
///
/// Returns AnalysisPreservation to indicate what analyses are still valid.
pub trait ProgramPass: Pass + Send + Sync {
    /// Run the pass on a program workset.
    fn run(
        &self,
        workset: &mut ProgramWorkset,
        ctx: &ProgramPipelineContext,
    ) -> AnalysisPreservation;

    /// Get the pass name.
    fn name(&self) -> &'static str;

    /// Get the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Run a sequence of function passes on a cloned function and write it back when changed.
pub fn run_function_passes(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::Tree,
    ctx: &PipelineContext<'_>,
    passes: &[&dyn FunctionPass],
) -> bool {
    // clone the function for mutation
    let mut function = tree.get(function_id).clone();

    // skip extern functions
    if function.entry.is_none() {
        return false;
    }

    // run passes and track changes
    let mut changed = false;
    for pass in passes {
        function.recompute_next_value_id(tree);
        let preservation = pass.run(&mut function, tree, ctx);
        if !preservation.preserves_all() {
            changed = true;
        }
    }

    // write back when changes occurred
    if changed {
        *tree.get_mut(function_id) = function;
    }

    changed
}

/// Run a sequence of function passes on a cloned function and always write it back.
pub fn run_function_passes_always(
    function_id: mir::LocalNodeId<mir::Function>,
    tree: &mut mir::Tree,
    ctx: &PipelineContext<'_>,
    passes: &[&dyn FunctionPass],
) {
    // clone the function for mutation
    let mut function = tree.get(function_id).clone();

    // skip extern functions
    if function.entry.is_none() {
        return;
    }

    // run passes
    for pass in passes {
        function.recompute_next_value_id(tree);
        pass.run(&mut function, tree, ctx);
    }

    // write back the updated function
    *tree.get_mut(function_id) = function;
}
