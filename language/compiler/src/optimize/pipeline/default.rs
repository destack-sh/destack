use crate::CompositePipeline;
use crate::optimize::passes::{
    ArgumentSpecialize, BoundsCheckEliminate, CfgLayout, CodeHoisting, ConstantFold, CopyPropagate,
    CorrelatedValueProp, DeadArgEliminate, DeadCodeEliminate, DeadStoreEliminate, FunctionAttrs,
    GlobalOpt, GlobalValueNumbering, GuardEliminate, IfConvert, InductionVariableSimplify, Inline,
    InstructionCombine, InterproceduralConstantPropagation, InterproceduralDceCleanup,
    InterproceduralSccp, Licm, LoadPre, LoadStoreForward, LocalCse, LoopBoundsCheckEliminate,
    LoopDelete, LoopDistribute, LoopFusion, LoopIdiomRecognize, LoopInterchange, LoopPeel,
    LoopRotate, LoopSimplify, LoopStrengthReduce, LoopUnroll, LoopUnrollAndJam, LoopUnswitch,
    LoopVersioning, Mem2Reg, MemCse, Narrow, PartialRedundancyElim, Reassociate, SimplifyCfg, Sink,
    SparseConditionalConstantPropagation, Sroa, StorePre, StoreSink, TailCallElim,
    ValueRangePropagation,
};
use crate::optimize::{FunctionPass, OptimizationLevel};

use super::builder::{PackagePipelineBuilder, PipelineBuilder, ProgramPipelineBuilder};
use super::module::{FunctionPipeline, FunctionToModuleAdaptor};
use super::package::{
    PackageCompositePipeline, PackageLevelProgramPipeline, ProgramCompositePipeline,
};

/// Build the optimization pipeline for the given level and target family.
pub fn default_pipeline(
    level: OptimizationLevel,
    is_native_target: bool,
) -> super::module::CompositePipeline {
    match level {
        OptimizationLevel::O0 => o0_pipeline(is_native_target),
        OptimizationLevel::O1 => o1_pipeline(is_native_target),
        OptimizationLevel::O2 => o2_pipeline(is_native_target),
        OptimizationLevel::O3 => o3_pipeline(is_native_target),
        OptimizationLevel::O4 => o4_pipeline(is_native_target),
    }
}

/// Build the package pipeline for the given level.
pub fn default_package_pipeline(
    level: OptimizationLevel,
    is_native_target: bool,
) -> PackageCompositePipeline {
    let module_pipeline = default_pipeline(level, is_native_target);

    PackagePipelineBuilder::new()
        .module_pipeline(module_pipeline)
        .build()
}

/// Build the program pipeline.
pub fn default_program_pipeline(is_native_target: bool) -> ProgramCompositePipeline {
    let pipeline = PackageLevelProgramPipeline::new(
        default_package_pipeline(OptimizationLevel::O0, is_native_target),
        default_package_pipeline(OptimizationLevel::O1, is_native_target),
        default_package_pipeline(OptimizationLevel::O2, is_native_target),
        default_package_pipeline(OptimizationLevel::O3, is_native_target),
        default_package_pipeline(OptimizationLevel::O4, is_native_target),
    );

    ProgramPipelineBuilder::new().pipeline(pipeline).build()
}

/// Return canonicalization passes that normalize MIR shape.
fn canonicalize(is_native_target: bool) -> Vec<Box<dyn FunctionPass>> {
    let mut passes: Vec<Box<dyn FunctionPass>> = vec![Box::new(Sroa)];
    if is_native_target {
        passes.push(Box::new(Mem2Reg));
    }
    passes
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
        Box::new(LoadPre),
        Box::new(StorePre),
        Box::new(LoadStoreForward),
        Box::new(MemCse),
        Box::new(StoreSink),
        Box::new(DeadStoreEliminate),
    ]
}

/// Return loop optimization passes before fusion.
fn optimize_loops_pre_fusion(aggressive: bool) -> Vec<Box<dyn FunctionPass>> {
    let mut passes: Vec<Box<dyn FunctionPass>> = vec![
        Box::new(LoopSimplify),
        Box::new(LoopRotate),
        Box::new(LoopPeel),
        Box::new(InductionVariableSimplify),
        Box::new(LoopStrengthReduce),
        Box::new(LoopInterchange),
        Box::new(LoopDistribute),
        Box::new(LoopVersioning),
        Box::new(LoopIdiomRecognize),
        Box::new(Licm),
    ];
    if aggressive {
        passes.push(Box::new(LoopUnswitch));
        passes.push(Box::new(LoopUnroll));
        passes.push(Box::new(LoopUnrollAndJam));
    }
    passes.push(Box::new(LoopDelete));
    passes
}

/// Return loop optimization passes for fusion cleanup.
fn optimize_loops_post_fusion() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(LoopSimplify),
        Box::new(LoopFusion),
        Box::new(LoopDelete),
    ]
}

/// Return a fixed point island for memory optimizations.
fn memory_island() -> FunctionPipeline {
    FunctionPipeline::new(optimize_memory())
}

/// Return a fixed point island for loop optimizations before fusion.
fn loop_island_pre_fusion(aggressive: bool) -> FunctionPipeline {
    FunctionPipeline::new(optimize_loops_pre_fusion(aggressive))
}

/// Return a fixed point island for loop fusion.
fn loop_island_post_fusion() -> FunctionPipeline {
    FunctionPipeline::new(optimize_loops_post_fusion())
}

/// Return type and bounds check optimizations.
fn optimize_types() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(LoopBoundsCheckEliminate),
        Box::new(BoundsCheckEliminate),
    ]
}

/// Return the interprocedural cleanup pipeline.
fn interprocedural_cleanup_pipeline() -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(cleanup())
        .module_pass(InterproceduralDceCleanup)
        .build()
}

/// Return late cleanup passes.
fn cleanup() -> Vec<Box<dyn FunctionPass>> {
    vec![Box::new(SimplifyCfg), Box::new(DeadCodeEliminate)]
}

/// O0: Verification and correctness only.
fn o0_pipeline(is_native_target: bool) -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        .build()
}

/// O1: Fast compilation with essential optimizations.
fn o1_pipeline(is_native_target: bool) -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        .function_passes(scalar_island_light())
        .function_passes(optimize_types())
        .function_passes(cleanup())
        .build()
}

/// O2: Release builds with comprehensive optimization.
///
/// Structure: verify -> canonicalize -> [simplify <-> optimize]* -> cleanup
/// Each major phase is followed by simplification to expose new opportunities.
fn o2_pipeline(is_native_target: bool) -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        // early scalar fixed point island
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(false))),
        )
        // interprocedural inlining and attribute inference
        .module_pass(FunctionAttrs)
        .module_pass(InterproceduralConstantPropagation)
        .module_pass(InterproceduralSccp)
        .module_pass(DeadArgEliminate)
        .module_pass(Inline)
        .module_pass(GlobalOpt)
        .repeat(2, interprocedural_cleanup_pipeline())
        // memory optimization
        .repeat(2, FunctionToModuleAdaptor::new(memory_island()))
        .function_passes(scalar_island_full(false))
        // loop optimization
        .repeat(
            2,
            FunctionToModuleAdaptor::new(loop_island_pre_fusion(false)),
        )
        .repeat(2, FunctionToModuleAdaptor::new(loop_island_post_fusion()))
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
        .function_passes(vec![Box::new(CfgLayout)])
        .build()
}

/// O3: aggressive optimization.
///
/// More iterations, aggressive loop transforms, extra cleanup rounds.
fn o3_pipeline(is_native_target: bool) -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        // early scalar fixed point island (more iterations)
        .repeat(
            3,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // interprocedural inlining and attribute inference
        .module_pass(FunctionAttrs)
        .module_pass(InterproceduralConstantPropagation)
        .module_pass(InterproceduralSccp)
        .module_pass(ArgumentSpecialize)
        .module_pass(DeadArgEliminate)
        .module_pass(Inline)
        .module_pass(GlobalOpt)
        .repeat(3, interprocedural_cleanup_pipeline())
        // memory optimization
        .repeat(3, FunctionToModuleAdaptor::new(memory_island()))
        .function_passes(scalar_island_full(true))
        // loop optimization (aggressive)
        .repeat(
            2,
            FunctionToModuleAdaptor::new(loop_island_pre_fusion(true)),
        )
        .repeat(2, FunctionToModuleAdaptor::new(loop_island_post_fusion()))
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
        .function_passes(vec![Box::new(CfgLayout)])
        .build()
}

/// O4: maximal single module optimization.
///
/// This adds more fixed point iterations to expose secondary effects.
fn o4_pipeline(is_native_target: bool) -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        // early scalar fixed point island (extra iterations)
        .repeat(
            4,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // interprocedural inlining and attribute inference
        .module_pass(FunctionAttrs)
        .module_pass(InterproceduralConstantPropagation)
        .module_pass(InterproceduralSccp)
        .module_pass(ArgumentSpecialize)
        .module_pass(DeadArgEliminate)
        .module_pass(Inline)
        .module_pass(GlobalOpt)
        .repeat(4, interprocedural_cleanup_pipeline())
        // memory optimization
        .repeat(4, FunctionToModuleAdaptor::new(memory_island()))
        .function_passes(scalar_island_full(true))
        // loop optimization (aggressive)
        .repeat(
            3,
            FunctionToModuleAdaptor::new(loop_island_pre_fusion(true)),
        )
        .repeat(3, FunctionToModuleAdaptor::new(loop_island_post_fusion()))
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
        .function_passes(vec![Box::new(CfgLayout)])
        .build()
}
