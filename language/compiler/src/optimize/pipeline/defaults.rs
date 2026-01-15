use crate::CompositePipeline;
use crate::optimize::passes::{
    BorrowCheck, BoundsCheckEliminate, CodeHoisting, ConstantFold, CopyPropagate,
    CorrelatedValueProp, DeadCodeEliminate, DeadFunctionEliminate, DeadStoreEliminate, DropInsert,
    FunctionAttrs, GlobalValueNumbering, GuardEliminate, IfConvert, InductionVariableSimplify,
    Inline, InstructionCombine, Licm, LoadStoreForward, LocalCse, LoopBoundsCheckEliminate,
    LoopDelete, LoopIdiomRecognize, LoopPeel, LoopRotate, LoopSimplify, LoopStrengthReduce,
    LoopUnroll, LoopUnswitch, LoopVersioning, Mem2Reg, MemCse, MoveCheck, Narrow,
    PartialRedundancyElim, Reassociate, SimplifyCfg, Sink, SparseConditionalConstantPropagation,
    Sroa, StackCheck, TailCallElim, ValueRangePropagation,
};
use crate::optimize::{FunctionPass, OptimizationLevel};

use super::builder::{PackagePipelineBuilder, PipelineBuilder, ProgramPipelineBuilder};
use super::module::{FunctionPipeline, FunctionToModuleAdaptor};
use super::package::{
    PackageCompositePipeline, PackageLevelProgramPipeline, ProgramCompositePipeline,
};

/// Build the optimization pipeline for the given level.
pub fn default_pipeline(level: OptimizationLevel) -> super::module::CompositePipeline {
    match level {
        OptimizationLevel::O0 => o0_pipeline(),
        OptimizationLevel::O1 => o1_pipeline(),
        OptimizationLevel::O2 => o2_pipeline(),
        OptimizationLevel::O3 => o3_pipeline(),
        OptimizationLevel::O4 => o4_pipeline(),
    }
}

/// Build the package pipeline for the given level.
pub fn default_package_pipeline(level: OptimizationLevel) -> PackageCompositePipeline {
    let module_pipeline = default_pipeline(level);

    PackagePipelineBuilder::new()
        .module_pipeline(module_pipeline)
        .build()
}

/// Build the program pipeline.
pub fn default_program_pipeline() -> ProgramCompositePipeline {
    let pipeline = PackageLevelProgramPipeline::new(
        default_package_pipeline(OptimizationLevel::O0),
        default_package_pipeline(OptimizationLevel::O1),
        default_package_pipeline(OptimizationLevel::O2),
        default_package_pipeline(OptimizationLevel::O3),
        default_package_pipeline(OptimizationLevel::O4),
    );

    ProgramPipelineBuilder::new().pipeline(pipeline).build()
}

// pass bundles: each returns a fresh vec of boxed passes

/// Return verification passes for early pipeline stages.
fn verify() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(MoveCheck),
        Box::new(BorrowCheck),
        Box::new(StackCheck),
    ]
}

/// Return canonicalization passes that normalize MIR shape.
fn canonicalize() -> Vec<Box<dyn FunctionPass>> {
    vec![Box::new(Sroa), Box::new(Mem2Reg), Box::new(DropInsert)]
}

/// Fast simplification passes that benefit from tight iteration.
fn simplify() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(ConstantFold),
        Box::new(InstructionCombine),
        Box::new(SimplifyCfg),
        Box::new(DeadCodeEliminate),
    ]
}

/// Redundancy elimination requiring analysis.
fn eliminate_redundancy() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(SparseConditionalConstantPropagation),
        Box::new(Reassociate),
        Box::new(CorrelatedValueProp),
        Box::new(ValueRangePropagation),
        Box::new(Narrow),
        Box::new(GuardEliminate),
        Box::new(PartialRedundancyElim),
        Box::new(GlobalValueNumbering),
        Box::new(CodeHoisting),
        Box::new(IfConvert),
        Box::new(LocalCse),
        Box::new(CopyPropagate),
    ]
}

/// Lightweight scalar fixed point island.
fn scalar_island_light() -> Vec<Box<dyn FunctionPass>> {
    let mut passes = Vec::new();
    passes.extend(simplify());
    passes.push(Box::new(LocalCse));
    passes.push(Box::new(CopyPropagate));
    passes
}

/// Full scalar fixed point island.
fn scalar_island_full(aggressive: bool) -> Vec<Box<dyn FunctionPass>> {
    let mut passes = Vec::new();
    passes.extend(eliminate_redundancy());
    passes.extend(simplify());
    if aggressive {
        passes.push(Box::new(InstructionCombine));
        passes.push(Box::new(SimplifyCfg));
        passes.push(Box::new(DeadCodeEliminate));
    }
    passes
}

/// Return memory optimization passes.
fn optimize_memory() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(LoadStoreForward),
        Box::new(MemCse),
        Box::new(DeadStoreEliminate),
    ]
}

/// Return loop optimization passes.
fn optimize_loops(aggressive: bool) -> Vec<Box<dyn FunctionPass>> {
    let mut passes: Vec<Box<dyn FunctionPass>> = vec![
        Box::new(LoopSimplify),
        Box::new(LoopRotate),
        Box::new(LoopPeel),
        Box::new(InductionVariableSimplify),
        Box::new(LoopStrengthReduce),
        Box::new(LoopVersioning),
        Box::new(LoopIdiomRecognize),
        Box::new(Licm),
    ];
    if aggressive {
        passes.push(Box::new(LoopUnswitch));
        passes.push(Box::new(LoopUnroll));
    }
    passes.push(Box::new(LoopDelete));
    passes
}

/// Return type and bounds check optimizations.
fn optimize_types() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(LoopBoundsCheckEliminate),
        Box::new(BoundsCheckEliminate),
    ]
}

/// Return late cleanup passes.
fn cleanup() -> Vec<Box<dyn FunctionPass>> {
    vec![Box::new(SimplifyCfg), Box::new(DeadCodeEliminate)]
}

/// O0: Verification and correctness only.
fn o0_pipeline() -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        .build()
}

/// O1: Fast compilation with essential optimizations.
fn o1_pipeline() -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        .function_passes(scalar_island_light())
        .function_passes(optimize_types())
        .function_passes(cleanup())
        .build()
}

/// O2: Release builds with comprehensive optimization.
///
/// Structure: verify -> canonicalize -> [simplify <-> optimize]* -> cleanup
/// Each major phase is followed by simplification to expose new opportunities.
fn o2_pipeline() -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        // early scalar fixed point island
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(false))),
        )
        // interprocedural inlining and attribute inference
        .module_pass(Inline)
        .module_pass(FunctionAttrs)
        .module_pass(DeadFunctionEliminate)
        // memory optimization
        .function_passes(optimize_memory())
        .function_passes(scalar_island_full(false))
        // loop optimization
        .function_passes(optimize_loops(false))
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(false))),
        )
        // type optimization
        .function_passes(optimize_types())
        .function_passes(scalar_island_full(false))
        // late scalar
        .module_pass(TailCallElim)
        .function_passes(vec![Box::new(Sink)])
        .function_passes(cleanup())
        .build()
}

/// O3: aggressive optimization.
///
/// More iterations, aggressive loop transforms, extra cleanup rounds.
fn o3_pipeline() -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        // early scalar fixed point island (more iterations)
        .repeat(
            3,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // interprocedural inlining and attribute inference
        .module_pass(Inline)
        .module_pass(FunctionAttrs)
        .module_pass(DeadFunctionEliminate)
        // memory optimization
        .function_passes(optimize_memory())
        .function_passes(scalar_island_full(true))
        // loop optimization (aggressive)
        .function_passes(optimize_loops(true))
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // type optimization
        .function_passes(optimize_types())
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // late scalar
        .module_pass(TailCallElim)
        .function_passes(vec![Box::new(Sink)])
        .function_passes(cleanup())
        .build()
}

/// O4: maximal single module optimization.
///
/// This adds more fixed point iterations to expose secondary effects.
fn o4_pipeline() -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        // early scalar fixed point island (extra iterations)
        .repeat(
            4,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // interprocedural inlining and attribute inference
        .module_pass(Inline)
        .module_pass(FunctionAttrs)
        .module_pass(DeadFunctionEliminate)
        // memory optimization
        .function_passes(optimize_memory())
        .function_passes(scalar_island_full(true))
        // loop optimization (aggressive)
        .function_passes(optimize_loops(true))
        .repeat(
            3,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // type optimization
        .function_passes(optimize_types())
        .repeat(
            3,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // late scalar
        .module_pass(TailCallElim)
        .function_passes(vec![Box::new(Sink)])
        .function_passes(cleanup())
        .build()
}
