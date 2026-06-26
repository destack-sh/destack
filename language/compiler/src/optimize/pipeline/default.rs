use crate::CompositePipeline;
use crate::optimize::passes::{
    CombineInstructions, ConvertBranches, DistributeLoops, EliminateBoundsChecks,
    EliminateDeadArguments, EliminateDeadCode, EliminateDeadFunctions, EliminateDeadLoops,
    EliminateDeadStores, EliminateGuards, EliminateInterproceduralDeadCode,
    EliminateLocalCommonSubexpressions, EliminateLoopBoundsChecks, EliminatePartialRedundancy,
    EliminatePartialRedundantLoads, EliminatePartialRedundantStores, EliminateRedundantExpressions,
    EliminateRedundantMemory, EliminateTailCalls, FoldConstants, ForwardStoredValues, FuseLoops,
    HoistInstructions, HoistLoopInvariants, InlineFunctions, InterchangeLoops, NarrowValues,
    OptimizeGlobals, OrderBlocks, PeelLoops, PromoteMemoryToRegisters, PropagateCopies,
    PropagateCorrelatedValues, PropagateInterproceduralConstants,
    PropagateInterproceduralSparseConstants, PropagateSparseConstants, PropagateValueRanges,
    ReassociateExpressions, RecognizeLoopIdioms, ReduceLoopStrength, RotateLoops,
    SimplifyControlFlow, SimplifyInductionVariables, SimplifyLoops, SinkInstructions, SinkStores,
    SpecializeArguments, SplitAggregates, UnrollAndJamLoops, UnrollLoops, UnswitchLoops,
    VersionLoops,
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
    let mut passes: Vec<Box<dyn FunctionPass>> = vec![Box::new(SplitAggregates)];
    if is_native_target {
        passes.push(Box::new(PromoteMemoryToRegisters));
    }
    passes
}

/// Fast simplification passes that benefit from tight iteration.
fn simplify() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(FoldConstants),
        Box::new(CombineInstructions),
        Box::new(SimplifyControlFlow),
        Box::new(EliminateDeadCode),
    ]
}

/// Redundancy elimination requiring analysis.
fn eliminate_redundancy() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(PropagateSparseConstants),
        Box::new(ReassociateExpressions),
        Box::new(PropagateCorrelatedValues),
        Box::new(PropagateValueRanges),
        Box::new(NarrowValues),
        Box::new(EliminateGuards),
        Box::new(EliminatePartialRedundancy),
        Box::new(EliminateRedundantExpressions),
        Box::new(HoistInstructions),
        Box::new(ConvertBranches),
        Box::new(EliminateLocalCommonSubexpressions),
        Box::new(PropagateCopies),
    ]
}

/// Return lightweight scalar simplification passes.
fn scalar_passes_light() -> Vec<Box<dyn FunctionPass>> {
    let mut passes = Vec::new();
    passes.extend(simplify());
    passes.push(Box::new(EliminateLocalCommonSubexpressions));
    passes.push(Box::new(PropagateCopies));
    passes
}

/// Return full scalar simplification passes.
fn scalar_passes_full(aggressive: bool) -> Vec<Box<dyn FunctionPass>> {
    let mut passes = Vec::new();
    passes.extend(eliminate_redundancy());
    passes.extend(simplify());
    if aggressive {
        passes.push(Box::new(CombineInstructions));
        passes.push(Box::new(SimplifyControlFlow));
        passes.push(Box::new(EliminateDeadCode));
    }
    passes
}

/// Return memory optimization passes.
fn optimize_memory() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(EliminatePartialRedundantLoads),
        Box::new(EliminatePartialRedundantStores),
        Box::new(ForwardStoredValues),
        Box::new(EliminateRedundantMemory),
        Box::new(SinkStores),
        Box::new(EliminateDeadStores),
    ]
}

/// Return loop optimization passes before fusion.
fn optimize_loops_pre_fusion(aggressive: bool) -> Vec<Box<dyn FunctionPass>> {
    let mut passes: Vec<Box<dyn FunctionPass>> = vec![
        Box::new(SimplifyLoops),
        Box::new(RotateLoops),
        Box::new(PeelLoops),
        Box::new(SimplifyInductionVariables),
        Box::new(ReduceLoopStrength),
        Box::new(InterchangeLoops),
        Box::new(DistributeLoops),
        Box::new(VersionLoops),
        Box::new(RecognizeLoopIdioms),
        Box::new(HoistLoopInvariants),
    ];
    if aggressive {
        passes.push(Box::new(UnswitchLoops));
        passes.push(Box::new(UnrollLoops));
        passes.push(Box::new(UnrollAndJamLoops));
    }
    passes.push(Box::new(EliminateDeadLoops));
    passes
}

/// Return loop optimization passes for fusion cleanup.
fn optimize_loops_post_fusion() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(SimplifyLoops),
        Box::new(FuseLoops),
        Box::new(EliminateDeadLoops),
    ]
}

/// Return the memory optimization pipeline.
fn memory_pipeline() -> FunctionPipeline {
    FunctionPipeline::new(optimize_memory())
}

/// Return the loop optimization pipeline before fusion.
fn loop_pipeline_pre_fusion(aggressive: bool) -> FunctionPipeline {
    FunctionPipeline::new(optimize_loops_pre_fusion(aggressive))
}

/// Return the loop fusion pipeline.
fn loop_pipeline_post_fusion() -> FunctionPipeline {
    FunctionPipeline::new(optimize_loops_post_fusion())
}

/// Return type and bounds check optimizations.
fn optimize_types() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(EliminateLoopBoundsChecks),
        Box::new(EliminateBoundsChecks),
    ]
}

/// Return the interprocedural cleanup pipeline.
fn interprocedural_cleanup_pipeline() -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(cleanup())
        .module_pass(EliminateInterproceduralDeadCode)
        .build()
}

/// Return late cleanup passes.
fn cleanup() -> Vec<Box<dyn FunctionPass>> {
    vec![Box::new(SimplifyControlFlow), Box::new(EliminateDeadCode)]
}

/// O0: debug builds with canonicalization only.
fn o0_pipeline(is_native_target: bool) -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        .build()
}

/// O1: fast local optimization.
fn o1_pipeline(is_native_target: bool) -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        .function_passes(scalar_passes_light())
        .function_passes(optimize_types())
        .function_passes(cleanup())
        // drop functions no root reaches (module scope at this level)
        .module_pass(EliminateDeadFunctions)
        .build()
}

/// O2: standard release optimization.
///
/// Structure: verify -> canonicalize -> [simplify <-> optimize]* => cleanup
/// Each major phase is followed by simplification to expose new opportunities.
fn o2_pipeline(is_native_target: bool) -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        // run early scalar cleanup
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_passes_full(false))),
        )
        // drop functions no root reaches at program scope
        .module_pass(EliminateDeadFunctions)
        // propagate interprocedural constants before inlining
        .module_pass(PropagateInterproceduralConstants)
        .module_pass(PropagateInterproceduralSparseConstants)
        .module_pass(EliminateDeadArguments)
        .module_pass(InlineFunctions)
        .module_pass(OptimizeGlobals)
        .repeat(2, interprocedural_cleanup_pipeline())
        // memory optimization
        .repeat(2, FunctionToModuleAdaptor::new(memory_pipeline()))
        .function_passes(scalar_passes_full(false))
        // loop optimization
        .repeat(
            2,
            FunctionToModuleAdaptor::new(loop_pipeline_pre_fusion(false)),
        )
        .repeat(2, FunctionToModuleAdaptor::new(loop_pipeline_post_fusion()))
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_passes_full(false))),
        )
        // type optimization
        .function_passes(optimize_types())
        .function_passes(scalar_passes_full(false))
        // late scalar
        .module_pass(EliminateTailCalls)
        .function_passes(vec![Box::new(SinkInstructions)])
        .function_passes(cleanup())
        .function_passes(vec![Box::new(OrderBlocks)])
        .build()
}

/// O3: aggressive release optimization.
///
/// More iterations, aggressive loop transforms, extra cleanup rounds.
fn o3_pipeline(is_native_target: bool) -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        // run early scalar cleanup with more iterations
        .repeat(
            3,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_passes_full(true))),
        )
        // drop functions no root reaches at program scope
        .module_pass(EliminateDeadFunctions)
        // propagate interprocedural constants before inlining
        .module_pass(PropagateInterproceduralConstants)
        .module_pass(PropagateInterproceduralSparseConstants)
        .module_pass(SpecializeArguments)
        .module_pass(EliminateDeadArguments)
        .module_pass(InlineFunctions)
        .module_pass(OptimizeGlobals)
        .repeat(3, interprocedural_cleanup_pipeline())
        // memory optimization
        .repeat(3, FunctionToModuleAdaptor::new(memory_pipeline()))
        .function_passes(scalar_passes_full(true))
        // loop optimization (aggressive)
        .repeat(
            2,
            FunctionToModuleAdaptor::new(loop_pipeline_pre_fusion(true)),
        )
        .repeat(2, FunctionToModuleAdaptor::new(loop_pipeline_post_fusion()))
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_passes_full(true))),
        )
        // type optimization
        .function_passes(optimize_types())
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_passes_full(true))),
        )
        // late scalar
        .module_pass(EliminateTailCalls)
        .function_passes(vec![Box::new(SinkInstructions)])
        .function_passes(cleanup())
        .function_passes(vec![Box::new(OrderBlocks)])
        .build()
}

/// O4: maximum program optimization.
///
/// This adds more fixed point iterations to expose secondary effects.
fn o4_pipeline(is_native_target: bool) -> super::module::CompositePipeline {
    PipelineBuilder::new()
        .function_passes(canonicalize(is_native_target))
        // run early scalar cleanup with extra iterations
        .repeat(
            4,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_passes_full(true))),
        )
        // drop functions no root reaches at program scope
        .module_pass(EliminateDeadFunctions)
        // propagate interprocedural constants before inlining
        .module_pass(PropagateInterproceduralConstants)
        .module_pass(PropagateInterproceduralSparseConstants)
        .module_pass(SpecializeArguments)
        .module_pass(EliminateDeadArguments)
        .module_pass(InlineFunctions)
        .module_pass(OptimizeGlobals)
        .repeat(4, interprocedural_cleanup_pipeline())
        // memory optimization
        .repeat(4, FunctionToModuleAdaptor::new(memory_pipeline()))
        .function_passes(scalar_passes_full(true))
        // loop optimization (aggressive)
        .repeat(
            3,
            FunctionToModuleAdaptor::new(loop_pipeline_pre_fusion(true)),
        )
        .repeat(3, FunctionToModuleAdaptor::new(loop_pipeline_post_fusion()))
        .repeat(
            3,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_passes_full(true))),
        )
        // type optimization
        .function_passes(optimize_types())
        .repeat(
            3,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_passes_full(true))),
        )
        // late scalar
        .module_pass(EliminateTailCalls)
        .function_passes(vec![Box::new(SinkInstructions)])
        .function_passes(cleanup())
        .function_passes(vec![Box::new(OrderBlocks)])
        .build()
}
