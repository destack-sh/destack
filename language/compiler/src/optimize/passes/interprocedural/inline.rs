use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::CallGraphScc;
use crate::optimize::common::{
    CallsiteHotness, CallsiteHotnessPolicy, ValueTypeMap, block_execution_counts,
    block_hotness_from_counts, build_value_definition_map, callsite_hotness,
    clone_instruction_metadata, constant_for_value, instruction_map_with_locals,
    instruction_substitute_uses_in_tree, remap_instruction_memory_accesses, scaled_profile_count,
    terminator_remap, terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, ModuleAnalyses, ModulePass, PipelineContext};

declare_pass! {
    /// Inline direct calls into their callers when the callee is small.
    ///
    /// This pass clones callee blocks into the caller, rewires returns to a continuation block, and skips recursive SCCs and functions with tail calls.
    ///
    /// ```mir
    /// function callee(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = int.add v0, v0
    ///     return v1
    /// }
    /// function caller(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = call callee(v0)
    ///     v2 = int.add v1, v0
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function caller(v0: int32): int32 {
    /// b0(v0: int32):
    ///     jump b1(v0)
    /// b1(v1: int32):
    ///     v2 = int.add v1, v1
    ///     jump b2(v2)
    /// b2(v3: int32):
    ///     v4 = int.add v3, v0
    ///     return v4
    /// }
    /// ```
    #[pass(id = "inline", requires(call_effects))]
    pub Inline,
    "Inline direct calls"
}

impl ModulePass for Inline {
    /// Run the inline pass over a module.
    fn run(&self, tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> AnalysisPreservation {
        let changed = run_inline(tree, ctx);

        // report analysis preservation based on whether changes occurred
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass display name.
    fn name(&self) -> &'static str {
        "Inline"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "inline"
    }
}

/// Maximum instructions allowed for inlining.
const INLINE_MAX_INSTRUCTIONS: usize = 48;
/// Maximum blocks allowed for inlining.
const INLINE_MAX_BLOCKS: usize = 6;
/// Maximum call sites within the callee for inlining.
const INLINE_MAX_CALLS: usize = 4;
/// Maximum instructions for leaf callees.
const INLINE_MAX_LEAF_INSTRUCTIONS: usize = 96;
/// Maximum function size after inlining.
const INLINE_MAX_FUNCTION_INSTRUCTIONS: usize = 2000;
/// Maximum inline sites per function.
const INLINE_MAX_SITES_PER_FUNCTION: usize = 16;
/// Base inline budget for a caller.
const INLINE_BUDGET_BASE: u64 = 1200;
/// Entry count divisor for inline budget scaling.
const INLINE_BUDGET_ENTRY_DIVISOR: u64 = 50;
/// Maximum inline budget for a caller.
const INLINE_BUDGET_MAX: u64 = 8000;
/// Base inline budget for a module.
const INLINE_MODULE_BUDGET_BASE: u64 = 40000;
/// Entry count divisor for module budget scaling.
const INLINE_MODULE_BUDGET_ENTRY_DIVISOR: u64 = 200;
/// Maximum inline budget for a module.
const INLINE_MODULE_BUDGET_MAX: u64 = 120000;
/// Base inline budget for an SCC.
const INLINE_SCC_BUDGET_BASE: u64 = 4000;
/// Entry count divisor for SCC budget scaling.
const INLINE_SCC_BUDGET_ENTRY_DIVISOR: u64 = 100;
/// Maximum inline budget for an SCC.
const INLINE_SCC_BUDGET_MAX: u64 = 24000;
/// Maximum instructions for cold callsites.
const INLINE_COLD_MAX_INSTRUCTIONS: usize = 8;
/// Maximum blocks for cold callsites.
const INLINE_COLD_MAX_BLOCKS: usize = 2;
/// Maximum calls for cold callsites.
const INLINE_COLD_MAX_CALLS: usize = 0;
/// Maximum instructions for hot callsites.
const INLINE_HOT_MAX_INSTRUCTIONS: usize = 96;
/// Maximum blocks for hot callsites.
const INLINE_HOT_MAX_BLOCKS: usize = 12;
/// Maximum calls for hot callsites.
const INLINE_HOT_MAX_CALLS: usize = 8;
/// Base inline benefit for eliminating a call boundary.
const INLINE_BENEFIT_CALL_OVERHEAD: u64 = 25;
/// Benefit for each constant argument.
const INLINE_BENEFIT_CONST_ARGUMENT: u64 = 6;
/// Benefit for leaf callees.
const INLINE_BENEFIT_LEAF: u64 = 10;
/// Divider for turning callsite counts into benefit multipliers.
const INLINE_BENEFIT_COUNT_DIVISOR: u64 = 50;
/// Maximum multiplier from callsite counts.
const INLINE_BENEFIT_COUNT_MAX_MULTIPLIER: u64 = 8;
/// Extra score bias for hot callsites.
const INLINE_HOT_SCORE_BONUS: i64 = 24;
// cost weights approximate llvm and cranelift heuristics for small inliners
/// Cost for a simple instruction.
const INLINE_COST_SIMPLE: u64 = 1;
/// Cost for a memory access instruction.
const INLINE_COST_MEMORY: u64 = 4;
/// Cost for an allocation instruction.
const INLINE_COST_ALLOC: u64 = 25;
/// Cost for a call instruction.
const INLINE_COST_CALL: u64 = 25;
/// Cost for an indirect call instruction.
const INLINE_COST_CALL_INDIRECT: u64 = 40;
/// Cost per block to account for control flow overhead.
const INLINE_COST_BLOCK: u64 = 3;
/// Always inline when cost is below this threshold.
const INLINE_ALWAYS_INLINE_COST: u64 = 40;

/// Inline pass main entry.
fn run_inline(tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> bool {
    // build analysis summaries for inlining
    let analyses = ModuleAnalyses::new(tree);
    let scc_map = analyses.get::<CallGraphScc>();
    let hotness_policy = ctx.inline_hotness_policy();
    let inline_budget_scale_percent = ctx.inline_budget_scale_percent();
    let mut module_budget = inline_budget_for_module(
        tree,
        ctx.profile(),
        hotness_policy,
        inline_budget_scale_percent,
    );
    let mut scc_budgets = inline_scc_budgets(
        tree,
        &scc_map,
        ctx.profile(),
        hotness_policy,
        inline_budget_scale_percent,
    );

    // collect function ids for stable iteration
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .map(|(id, _)| id)
        .collect();

    // track whether any inlining happened
    let mut changed = false;

    // walk each function for inline opportunities
    for function_id in function_ids {
        // skip functions without bodies
        let function = tree.get(function_id);
        if function.entry.is_none() {
            continue;
        }

        // clone the function for in place edits
        let mut function = function.clone();
        function.recompute_next_value_id(tree);

        // inline until no sites remain or budget is exhausted
        let mut inline_count = 0usize;
        let mut inline_budget = inline_budget_for_function(
            function_id,
            ctx.profile(),
            hotness_policy,
            inline_budget_scale_percent,
        );
        let scc_id = scc_map.scc_id(function_id);
        let mut scc_budget = scc_id
            .and_then(|id| scc_budgets.get(&id).copied())
            .unwrap_or(INLINE_SCC_BUDGET_BASE);
        let block_counts = block_execution_counts(&function, tree, ctx.profile(), hotness_policy);

        // iterate inline sites until the budget is exhausted
        loop {
            // stop when the inline budget is exhausted
            if inline_count >= INLINE_MAX_SITES_PER_FUNCTION {
                break;
            }

            // stop when no inline budget remains
            if inline_budget == 0 || module_budget == 0 || scc_budget == 0 {
                break;
            }

            // build value definitions for constant argument detection
            let value_definitions = build_value_definition_map(&function, tree);
            let available_budget = inline_budget.min(module_budget).min(scc_budget);

            // find the next candidate callsite
            let site = find_inline_site(
                function_id,
                &function,
                tree,
                &scc_map,
                ctx.profile(),
                hotness_policy,
                inline_budget_scale_percent,
                &value_definitions,
                &block_counts,
                available_budget,
            );
            let Some(site) = site else {
                break;
            };

            // attempt to inline the selected callsite
            let did_inline = inline_callsite(&mut function, tree, &site.site);
            if !did_inline {
                break;
            }

            // record a successful inline for this function
            inline_count += 1;
            inline_budget = inline_budget.saturating_sub(site.cost);
            module_budget = module_budget.saturating_sub(site.cost);
            scc_budget = scc_budget.saturating_sub(site.cost);
            changed = true;
        }

        if let Some(scc_id) = scc_id {
            scc_budgets.insert(scc_id, scc_budget);
        }

        // commit the updated function back into the tree
        *tree.get_mut(function_id) = function;
    }

    // record pass activity for downstream diagnostics
    if changed {
        ctx.strings.intern("inline");
    }

    changed
}

/// Inline site information for a call instruction.
#[derive(Debug, Clone)]
struct InlineSite {
    /// The block containing the call.
    block_id: mir::LocalNodeId<mir::Block>,
    /// The index of the call instruction in the block.
    call_index: usize,
    /// The call instruction id.
    call_instruction_id: mir::LocalNodeId<mir::Instruction>,
    /// The resolved callee id.
    callee_id: mir::LocalNodeId<mir::Function>,
    /// The call argument values.
    arguments: Vec<mir::Value>,
    /// The call destination value when present.
    destination: Option<mir::Value>,
}

/// Inline candidate with scoring information.
#[derive(Debug, Clone)]
struct InlineCandidate {
    /// The inline site to apply.
    site: InlineSite,
    /// Inline cost score.
    cost: u64,
    /// Inline benefit score.
    score: i64,
}

/// Find the next inline candidate in a function.
// allow many arguments to keep the inline heuristics explicit
#[allow(clippy::too_many_arguments)]
fn find_inline_site(
    function_id: mir::LocalNodeId<mir::Function>,
    function: &mir::Function,
    tree: &mir::NodeTree,
    scc_map: &CallGraphScc,
    profile: Option<&mir::ProfileTable>,
    hotness_policy: &CallsiteHotnessPolicy,
    inline_budget_scale_percent: u64,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
    inline_budget: u64,
) -> Option<InlineCandidate> {
    let mut best: Option<InlineCandidate> = None;

    // scan blocks in order for candidate callsites
    let block_ids = function.blocks.clone();
    for block_id in block_ids {
        // load the block and clone its instruction list
        let block = tree.get(block_id);
        let instruction_ids = block.instructions.clone();
        for (index, instruction_id) in instruction_ids.iter().enumerate() {
            // resolve a direct inline target for the instruction
            let instruction = tree.get(*instruction_id);
            let Some((callee_id, arguments, destination)) =
                resolve_inline_target(instruction, tree)
            else {
                continue;
            };

            let Some(candidate) = inline_candidate(
                tree,
                function_id,
                callee_id,
                scc_map,
                arguments.len(),
                *instruction_id,
                profile,
                hotness_policy,
                inline_budget_scale_percent,
                value_definitions,
                block_counts,
                InlineSite {
                    block_id,
                    call_index: index,
                    call_instruction_id: *instruction_id,
                    callee_id,
                    arguments,
                    destination,
                },
            ) else {
                continue;
            };

            if candidate.cost > inline_budget {
                continue;
            }

            if let Some(best_candidate) = best.as_ref()
                && candidate.score <= best_candidate.score
            {
                continue;
            }

            best = Some(candidate);
        }
    }

    best
}

/// Evaluate a callsite and return an inline candidate when profitable.
// allow many arguments to keep the inline heuristics explicit
#[allow(clippy::too_many_arguments)]
fn inline_candidate(
    tree: &mir::NodeTree,
    caller_id: mir::LocalNodeId<mir::Function>,
    callee_id: mir::LocalNodeId<mir::Function>,
    scc_map: &CallGraphScc,
    argument_count: usize,
    callsite_id: mir::LocalNodeId<mir::Instruction>,
    profile: Option<&mir::ProfileTable>,
    hotness_policy: &CallsiteHotnessPolicy,
    inline_budget_scale_percent: u64,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
    site: InlineSite,
) -> Option<InlineCandidate> {
    let block_count = block_counts.get(&site.block_id).copied().unwrap_or(0);
    let score = inline_score(
        tree,
        caller_id,
        callee_id,
        scc_map,
        argument_count,
        callsite_id,
        profile,
        hotness_policy,
        inline_budget_scale_percent,
        value_definitions,
        block_count,
        &site.arguments,
    )?;

    Some(InlineCandidate {
        site,
        cost: score.cost,
        score: score.score,
    })
}

/// Inline scoring metadata.
#[derive(Debug, Clone, Copy)]
struct InlineScore {
    /// Inline cost score.
    cost: u64,
    /// Inline benefit score minus cost.
    score: i64,
}

/// Scale inline size limits by an optimization level percent.
fn scale_inline_limit(limit: usize, scale_percent: u64) -> usize {
    if scale_percent == 0 {
        return 0;
    }

    let scaled = (limit as u128)
        .saturating_mul(scale_percent as u128)
        .saturating_add(99)
        .saturating_div(100);
    scaled.min(usize::MAX as u128) as usize
}

/// Compute inline costs and benefits for a callsite.
// allow many arguments to keep the inline heuristics explicit
#[allow(clippy::too_many_arguments)]
fn inline_score(
    tree: &mir::NodeTree,
    caller_id: mir::LocalNodeId<mir::Function>,
    callee_id: mir::LocalNodeId<mir::Function>,
    scc_map: &CallGraphScc,
    argument_count: usize,
    callsite_id: mir::LocalNodeId<mir::Instruction>,
    profile: Option<&mir::ProfileTable>,
    hotness_policy: &CallsiteHotnessPolicy,
    inline_budget_scale_percent: u64,
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    block_count: u64,
    arguments: &[mir::Value],
) -> Option<InlineScore> {
    let callsite_profile = profile.and_then(|profile| profile.callsite_profile(callsite_id));
    let mut hotness = callsite_hotness(profile, caller_id, callsite_id, hotness_policy);
    let entry_count = profile
        .and_then(|profile| {
            profile.function_profile(caller_id).map(|function_profile| {
                scaled_profile_count(
                    function_profile.entry_count,
                    profile.source,
                    &hotness_policy.scaling_policy(),
                )
            })
        })
        .unwrap_or(0);

    if profile.is_some() {
        if callsite_profile.is_none() {
            let block_hotness = block_hotness_from_counts(block_count, entry_count, hotness_policy);
            hotness = match block_hotness {
                CallsiteHotness::Unknown => {
                    if hotness_policy.missing_callsite_is_cold {
                        CallsiteHotness::Cold
                    } else {
                        CallsiteHotness::Unknown
                    }
                }
                _ => block_hotness,
            };
        } else if matches!(hotness, CallsiteHotness::Unknown) {
            hotness = block_hotness_from_counts(block_count, entry_count, hotness_policy);
        }
    }

    // compute size metrics
    let caller = tree.get(caller_id);
    let caller_cost = function_cost_for(tree, caller);
    let callee = tree.get(callee_id);
    let callee_cost = function_cost_for(tree, callee);

    // reject callsites that do not meet heuristic thresholds
    let should_inline = should_inline(
        tree,
        caller_id,
        callee_id,
        scc_map,
        argument_count,
        &callee_cost,
        &caller_cost,
        hotness,
        inline_budget_scale_percent,
    );
    if !should_inline {
        return None;
    }

    if callee_cost.cost <= INLINE_ALWAYS_INLINE_COST {
        return Some(InlineScore {
            cost: callee_cost.cost,
            score: 0,
        });
    }

    // compute benefit score
    let benefit = inline_benefit(
        profile,
        callsite_id,
        hotness_policy,
        hotness,
        arguments,
        value_definitions,
        tree,
        &callee_cost,
        entry_count,
        block_count,
    );
    let mut score = benefit as i64 - callee_cost.cost as i64;

    if matches!(hotness, CallsiteHotness::Hot) {
        score = score.saturating_add(INLINE_HOT_SCORE_BONUS);
    }

    if score < 0 {
        return None;
    }

    Some(InlineScore {
        cost: callee_cost.cost,
        score,
    })
}

/// Resolve a call instruction to a direct inline target.
fn resolve_inline_target(
    instruction: &mir::Instruction,
    tree: &mir::NodeTree,
) -> Option<(
    mir::LocalNodeId<mir::Function>,
    Vec<mir::Value>,
    Option<mir::Value>,
)> {
    // inspect call instruction variants
    match instruction {
        mir::Instruction::Call {
            destination,
            function,
            call,
            ..
        } => {
            // capture call arguments for a direct call
            let args = tree
                .get_arguments(call.arguments)
                .iter()
                .copied()
                .map(|argument| argument.value())
                .collect::<Option<Vec<_>>>()?;
            Some((
                function.function()?,
                args,
                destination.and_then(|value| value.value()),
            ))
        }
        _ => None,
    }
}

/// Check if a call should be inlined.
// allow many arguments to keep the inline heuristics explicit
#[allow(clippy::too_many_arguments)]
fn should_inline(
    tree: &mir::NodeTree,
    caller_id: mir::LocalNodeId<mir::Function>,
    callee_id: mir::LocalNodeId<mir::Function>,
    scc_map: &CallGraphScc,
    argument_count: usize,
    callee_size: &FunctionCost,
    caller_size: &FunctionCost,
    hotness: CallsiteHotness,
    inline_budget_scale_percent: u64,
) -> bool {
    // load the callee metadata
    let callee = tree.get(callee_id);

    // reject callees without bodies or with unsupported forms
    if callee.entry.is_none() {
        return false;
    }
    if callee.suspension.is_some() {
        return false;
    }
    if callee.parameters.len() != argument_count {
        return false;
    }
    if has_tail_calls(tree, callee) {
        return false;
    }
    if is_recursive_call(caller_id, callee_id, scc_map) {
        return false;
    }

    // evaluate inline size thresholds
    let max_function_instructions = scale_inline_limit(
        INLINE_MAX_FUNCTION_INSTRUCTIONS,
        inline_budget_scale_percent,
    );
    let estimated = caller_size.instructions + callee_size.instructions;
    if estimated > max_function_instructions {
        return false;
    }

    // handle cold callsites with strict limits
    if matches!(hotness, CallsiteHotness::Cold) {
        let max_instructions =
            scale_inline_limit(INLINE_COLD_MAX_INSTRUCTIONS, inline_budget_scale_percent);
        let max_blocks = scale_inline_limit(INLINE_COLD_MAX_BLOCKS, inline_budget_scale_percent);
        let max_calls = scale_inline_limit(INLINE_COLD_MAX_CALLS, inline_budget_scale_percent);
        return callee_size.instructions <= max_instructions
            && callee_size.blocks <= max_blocks
            && callee_size.calls <= max_calls;
    }

    // select thresholds for hot or unknown callsites
    let (max_instructions, max_blocks, max_calls) = if matches!(hotness, CallsiteHotness::Hot) {
        (
            scale_inline_limit(INLINE_HOT_MAX_INSTRUCTIONS, inline_budget_scale_percent),
            scale_inline_limit(INLINE_HOT_MAX_BLOCKS, inline_budget_scale_percent),
            scale_inline_limit(INLINE_HOT_MAX_CALLS, inline_budget_scale_percent),
        )
    } else {
        (
            scale_inline_limit(INLINE_MAX_INSTRUCTIONS, inline_budget_scale_percent),
            scale_inline_limit(INLINE_MAX_BLOCKS, inline_budget_scale_percent),
            scale_inline_limit(INLINE_MAX_CALLS, inline_budget_scale_percent),
        )
    };

    if callee_size.calls == 0 {
        let max_leaf =
            scale_inline_limit(INLINE_MAX_LEAF_INSTRUCTIONS, inline_budget_scale_percent);
        let limit = max_leaf.max(max_instructions);
        return callee_size.instructions <= limit;
    }

    callee_size.instructions <= max_instructions
        && callee_size.blocks <= max_blocks
        && callee_size.calls <= max_calls
}

/// Inline a direct callsite into the caller.
fn inline_callsite(
    caller: &mut mir::Function,
    tree: &mut mir::NodeTree,
    site: &InlineSite,
) -> bool {
    // load the callee and entry block
    let callee = tree.get(site.callee_id).clone();
    let Some(entry_block) = callee.entry else {
        return false;
    };

    // reject mismatched return handling
    let Some(return_type) = callee.return_type.ty() else {
        return false;
    };

    if matches!(tree.get(return_type), mir::Type::Void) && site.destination.is_some() {
        return false;
    }

    // validate the call instruction location
    let call_block = tree.get(site.block_id);
    if site.call_index >= call_block.instructions.len() {
        return false;
    }
    if call_block.instructions[site.call_index] != site.call_instruction_id {
        return false;
    }

    // build the parameter to argument mapping
    let mut argument_map = HashMap::new();
    for (param, arg) in callee.parameters.iter().zip(site.arguments.iter()) {
        let Some(param_value) = param.value.value() else {
            return false;
        };

        argument_map.insert(param_value, *arg);
    }

    // ensure entry block parameters are sourced from arguments
    let entry_params = tree.get(entry_block).parameters.clone();
    for param in &entry_params {
        let Some(param_value) = param.value.value() else {
            return false;
        };

        if !argument_map.contains_key(&param_value) {
            return false;
        }
    }

    // clone locals and blocks before rewriting the caller
    let local_map = clone_locals(caller, tree, &callee);
    let (block_map, value_map) = clone_callee_blocks(caller, tree, &callee, &argument_map);

    // split the caller block and jump into the inlined entry
    let inline_entry = block_map[&entry_block];
    let split = split_block_for_inline(
        caller,
        tree,
        site.block_id,
        site.call_index,
        site.call_instruction_id,
        inline_entry,
        return_type,
        site.destination,
        &entry_params,
        &argument_map,
    );
    let Some(split) = split else {
        return false;
    };

    // substitute the call result in the continuation block
    if let (Some(destination), Some(result_value)) = (site.destination, split.result_value) {
        substitute_value_in_function(tree, caller, destination, result_value);
    }

    // remap the inlined blocks and rewrite returns
    let call_provenance = tree.get_provenance(site.call_instruction_id.id);
    remap_inline_blocks(
        tree,
        &callee,
        &block_map,
        &value_map,
        &local_map,
        call_provenance,
    );
    rewrite_inlined_returns(
        tree,
        &block_map,
        split.continuation_id,
        site.destination.is_some(),
    );

    // clean up metadata for the removed call instruction
    tree.metadata
        .memory
        .remove_memory_accesses(site.call_instruction_id);
    tree.metadata
        .debug
        .instruction_locations
        .remove(&site.call_instruction_id);

    true
}

/// Result of splitting a block around a call.
#[derive(Debug, Clone, Copy)]
struct InlineSplit {
    /// The continuation block id.
    continuation_id: mir::LocalNodeId<mir::Block>,
    /// The continuation parameter value when the call returns a value.
    result_value: Option<mir::Value>,
}

/// Clone locals from the callee into the caller.
fn clone_locals(
    caller: &mut mir::Function,
    tree: &mut mir::NodeTree,
    callee: &mir::Function,
) -> HashMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<mir::Local>> {
    // allocate new locals in the caller
    let mut local_map = HashMap::new();
    for local_id in &callee.locals {
        let local = tree.get(*local_id).clone();
        let new_local = tree.insert(local);
        local_map.insert(*local_id, new_local);
        caller.locals.push(new_local);
    }

    local_map
}

/// Clone callee blocks and their value ids into the caller.
fn clone_callee_blocks(
    caller: &mut mir::Function,
    tree: &mut mir::NodeTree,
    callee: &mir::Function,
    argument_map: &HashMap<mir::Value, mir::Value>,
) -> (
    HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    HashMap<mir::Value, mir::Value>,
) {
    // seed value mappings with function arguments
    let mut value_map = argument_map.clone();
    let mut block_map = HashMap::new();

    // build value type lookup for the callee
    let callee_value_types = ValueTypeMap::new(callee, tree);

    // clone each callee block and allocate new values
    for block_id in &callee.blocks {
        // load the original callee block
        let original = tree.get(*block_id);

        // allocate new values for block parameters
        let new_params: Vec<mir::Parameter> = original
            .parameters
            .iter()
            .filter_map(|param| {
                let value = param.value.value()?;
                let ty = param.ty.ty()?;
                let new_value = caller.next_typed_value(ty);
                value_map.insert(value, new_value);
                Some(mir::Parameter {
                    value: new_value.into(),
                    ty: ty.into(),
                })
            })
            .collect();

        // allocate new values for instruction destinations
        for &instruction_id in &original.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction
                .destination()
                .and_then(|destination| destination.value())
            {
                let destination_type = callee_value_types.require_value_type(destination);
                let new_value = caller.next_typed_value(destination_type);
                value_map.insert(destination, new_value);
            }
        }

        // create the empty cloned block
        let new_block = mir::Block {
            name: None,
            parameters: new_params,
            instructions: Vec::new(),
            terminator: original.terminator,
        };
        let new_block_id = tree.insert(new_block);
        block_map.insert(*block_id, new_block_id);
        caller.blocks.push(new_block_id);
    }

    (block_map, value_map)
}

/// Split the caller block around a call and create the continuation block.
#[allow(clippy::too_many_arguments)]
fn split_block_for_inline(
    caller: &mut mir::Function,
    tree: &mut mir::NodeTree,
    block_id: mir::LocalNodeId<mir::Block>,
    call_index: usize,
    call_instruction_id: mir::LocalNodeId<mir::Instruction>,
    inline_entry: mir::LocalNodeId<mir::Block>,
    return_type: mir::LocalNodeId<mir::Type>,
    destination: Option<mir::Value>,
    entry_params: &[mir::Parameter],
    argument_map: &HashMap<mir::Value, mir::Value>,
) -> Option<InlineSplit> {
    // load the call block for editing
    let mut block = tree.get(block_id).clone();
    if call_index >= block.instructions.len() {
        return None;
    }
    if block.instructions[call_index] != call_instruction_id {
        return None;
    }

    // build the continuation block
    let continuation_terminator = tree.insert(mir::Terminator::Trap {
        kind: mir::TrapKind::Abort,
        payload: None,
    });
    let mut continuation_block = mir::Block::new(continuation_terminator);
    let mut result_value = None;

    // allocate a continuation parameter when a value is returned
    if destination.is_some() {
        let new_value = caller.next_typed_value(return_type);
        continuation_block.parameters.push(mir::Parameter {
            value: new_value.into(),
            ty: return_type.into(),
        });
        result_value = Some(new_value);
    }

    // split instructions around the call
    let after_instructions = block.instructions.split_off(call_index + 1);
    let removed = block.instructions.pop();
    if removed != Some(call_instruction_id) {
        return None;
    }

    // preserve the original terminator for the continuation
    let original_terminator = tree.get(block.terminator).clone();

    // build jump arguments for the inlined entry block
    let mut entry_arguments = Vec::new();
    for param in entry_params {
        let param_value = param.value.value()?;
        let argument = argument_map.get(&param_value).copied()?;
        entry_arguments.push(argument.into());
    }

    // replace the call with a jump to the inlined entry
    let jump_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: inline_entry.into(),
            arguments: entry_arguments,
        },
    };
    tree.replace(block.terminator, jump_terminator);
    tree.replace(block_id, block);

    // finish the continuation block
    continuation_block.instructions = after_instructions;
    tree.replace(continuation_terminator, original_terminator);

    // insert the continuation block into the caller
    let continuation_id = tree.insert(continuation_block);
    caller.blocks.push(continuation_id);

    Some(InlineSplit {
        continuation_id,
        result_value,
    })
}

/// Substitute a value inside a single block.
fn substitute_value_in_function(
    tree: &mut mir::NodeTree,
    function: &mir::Function,
    from: mir::Value,
    to: mir::Value,
) {
    // build substitution map for a single replacement
    let mut substitutions = HashMap::new();
    substitutions.insert(from, to);

    // update instructions and terminators across all blocks
    for block_id in &function.blocks {
        let block = tree.get(*block_id).clone();
        for instruction_id in &block.instructions {
            let instruction = tree.get(*instruction_id).clone();
            let updated = instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);
            tree.replace(*instruction_id, updated);
            remap_instruction_memory_accesses(tree, *instruction_id, &substitutions);
        }

        let terminator = tree.get(block.terminator).clone();
        let updated_terminator = terminator_substitute_uses(&terminator, &substitutions);
        if updated_terminator != terminator {
            tree.replace(block.terminator, updated_terminator);
        }
    }
}

/// Remap values and locals in inlined blocks.
fn remap_inline_blocks(
    tree: &mut mir::NodeTree,
    callee: &mir::Function,
    block_map: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    value_map: &HashMap<mir::Value, mir::Value>,
    local_map: &HashMap<mir::LocalNodeId<mir::Local>, mir::LocalNodeId<mir::Local>>,
    call_provenance: Option<mir::ProvenanceId>,
) {
    // clone instruction bodies and remap terminators for each block
    for block_id in &callee.blocks {
        // load the original and cloned blocks
        let new_block_id = block_map[block_id];
        let original_block = tree.get(*block_id);
        let original_instructions = original_block.instructions.clone();
        let original_terminator = tree.get(original_block.terminator).clone();
        let mut new_block = tree.get(new_block_id).clone();

        // clone instructions with remapped values
        let mut new_instructions = Vec::with_capacity(original_instructions.len());
        for instruction_id in original_instructions {
            // remap the instruction operands and destination
            let instruction = tree.get(instruction_id).clone();
            let remapped = instruction_map_with_locals(&instruction, value_map, local_map, tree);
            let new_id = tree.insert(remapped);

            // clone memory access metadata onto the new instruction
            clone_instruction_metadata(tree, instruction_id, new_id, value_map);

            // inlined instruction provenance
            if let (Some(call_provenance), Some(instruction_provenance)) =
                (call_provenance, tree.get_provenance(instruction_id.id))
            {
                let provenance_id = tree.metadata.provenance.create(
                    mir::ProvenanceAnchor::Mir(call_provenance),
                    None,
                    vec![
                        mir::ProvenanceKey::Mir(call_provenance),
                        mir::ProvenanceKey::Mir(instruction_provenance),
                    ],
                    Some(mir::ProvenanceReason::Inlined),
                );
                tree.set_provenance(new_id.id, provenance_id);
            }

            // clone debug locations onto the new instruction
            if let Some(location) = tree
                .metadata
                .debug
                .instruction_locations
                .get(&instruction_id)
                .cloned()
            {
                tree.metadata
                    .debug
                    .instruction_locations
                    .insert(new_id, location);
            }
            new_instructions.push(new_id);
        }

        // remap the terminator and commit the new block body
        new_block.instructions = new_instructions;
        tree.replace(new_block.terminator, original_terminator);

        let mut remapped_terminator = tree.get(new_block.terminator).clone();
        terminator_remap(&mut remapped_terminator, block_map, value_map);
        tree.replace(new_block.terminator, remapped_terminator);

        tree.replace(new_block_id, new_block);
    }
}

/// Rewrite return terminators in inlined blocks to jump to the continuation.
fn rewrite_inlined_returns(
    tree: &mut mir::NodeTree,
    block_map: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    continuation: mir::LocalNodeId<mir::Block>,
    expects_value: bool,
) {
    // rewrite return terminators to jump to the continuation
    for &new_block_id in block_map.values() {
        // skip blocks that do not return
        let block = tree.get(new_block_id).clone();
        let terminator = tree.get(block.terminator).clone();
        let mir::Terminator::Return { value } = terminator else {
            continue;
        };

        // forward return values when a result is expected
        let mut arguments = Vec::new();
        if expects_value && let Some(value) = value {
            arguments.push(value);
        }

        // replace the return with a jump to the continuation
        let new_terminator = mir::Terminator::Jump {
            target: mir::BlockTarget {
                block: continuation.into(),
                arguments: arguments.into_iter().map(Into::into).collect(),
            },
        };
        tree.replace(block.terminator, new_terminator);
        tree.replace(new_block_id, block);
    }
}

/// Check whether a function contains tail call terminators.
fn has_tail_calls(tree: &mir::NodeTree, function: &mir::Function) -> bool {
    // scan terminators for tail call forms
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        if matches!(
            terminator,
            mir::Terminator::TailCall { .. } | mir::Terminator::TailCallIndirect { .. }
        ) {
            return true;
        }
    }

    false
}

/// Check whether a call is recursive via SCC membership.
fn is_recursive_call(
    caller: mir::LocalNodeId<mir::Function>,
    callee: mir::LocalNodeId<mir::Function>,
    scc_map: &CallGraphScc,
) -> bool {
    // load the caller scc id
    let Some(caller_scc) = scc_map.scc_id(caller) else {
        return false;
    };

    // load the callee scc id
    let Some(callee_scc) = scc_map.scc_id(callee) else {
        return false;
    };

    // short circuit when the functions are in different sccs
    if caller_scc != callee_scc {
        return false;
    }

    // report recursion if the scc is marked recursive
    scc_map.is_recursive_scc(caller_scc)
}

/// Summary of function cost for inlining decisions.
#[derive(Debug, Clone, Copy, Default)]
struct FunctionCost {
    /// Instruction count for the function.
    instructions: usize,
    /// Block count for the function.
    blocks: usize,
    /// Call count for the function.
    calls: usize,
    /// Weighted inline cost.
    cost: u64,
}

/// Compute the cost summary for a single function.
fn function_cost_for(tree: &mir::NodeTree, function: &mir::Function) -> FunctionCost {
    // count blocks, instructions, and callsites
    let mut cost = FunctionCost {
        blocks: function.blocks.len(),
        cost: function.blocks.len() as u64 * INLINE_COST_BLOCK,
        ..FunctionCost::default()
    };

    // scan all blocks for instruction and call counts
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        cost.instructions += block.instructions.len();

        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            cost.cost = cost
                .cost
                .saturating_add(instruction_cost(instruction, tree));

            if matches!(
                instruction,
                mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. }
            ) {
                cost.calls += 1;
            }
        }

        let terminator = tree.get(block.terminator);
        cost.cost = cost.cost.saturating_add(terminator_cost(terminator));

        if matches!(
            terminator,
            mir::Terminator::TailCall { .. } | mir::Terminator::TailCallIndirect { .. }
        ) {
            cost.calls += 1;
        }
    }

    cost
}

/// Compute the inline budget for a caller.
fn scale_inline_budget(budget: u64, scale_percent: u64, max_budget: u64) -> u64 {
    if scale_percent == 0 {
        return 0;
    }

    let scaled = (budget as u128)
        .saturating_mul(scale_percent as u128)
        .saturating_add(99)
        .saturating_div(100);
    let scaled_max = (max_budget as u128)
        .saturating_mul(scale_percent as u128)
        .saturating_add(99)
        .saturating_div(100);
    let scaled = scaled.min(scaled_max);
    scaled.min(u64::MAX as u128) as u64
}

/// Compute the inline budget for a caller.
fn inline_budget_for_function(
    function_id: mir::LocalNodeId<mir::Function>,
    profile: Option<&mir::ProfileTable>,
    policy: &CallsiteHotnessPolicy,
    inline_budget_scale_percent: u64,
) -> u64 {
    let Some(profile) = profile else {
        return scale_inline_budget(
            INLINE_BUDGET_BASE,
            inline_budget_scale_percent,
            INLINE_BUDGET_MAX,
        );
    };
    let Some(function_profile) = profile.function_profile(function_id) else {
        return scale_inline_budget(
            INLINE_BUDGET_BASE,
            inline_budget_scale_percent,
            INLINE_BUDGET_MAX,
        );
    };

    let entry_count = scaled_profile_count(
        function_profile.entry_count,
        profile.source,
        &policy.scaling_policy(),
    );
    if entry_count == 0 {
        return scale_inline_budget(
            INLINE_BUDGET_BASE,
            inline_budget_scale_percent,
            INLINE_BUDGET_MAX,
        );
    }

    let bonus = entry_count / INLINE_BUDGET_ENTRY_DIVISOR;
    let budget = INLINE_BUDGET_BASE
        .saturating_add(bonus)
        .min(INLINE_BUDGET_MAX);
    scale_inline_budget(budget, inline_budget_scale_percent, INLINE_BUDGET_MAX)
}

/// Compute the inline benefit for a callsite.
// allow many arguments to keep the inline heuristics explicit
#[allow(clippy::too_many_arguments)]
fn inline_benefit(
    profile: Option<&mir::ProfileTable>,
    callsite_id: mir::LocalNodeId<mir::Instruction>,
    policy: &CallsiteHotnessPolicy,
    hotness: CallsiteHotness,
    arguments: &[mir::Value],
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
    callee_cost: &FunctionCost,
    entry_count: u64,
    block_count: u64,
) -> u64 {
    let mut benefit = INLINE_BENEFIT_CALL_OVERHEAD;

    let mut const_count = 0u64;
    for argument in arguments {
        if constant_for_value(*argument, value_definitions, tree).is_some() {
            const_count += 1;
        }
    }

    benefit = benefit.saturating_add(const_count * INLINE_BENEFIT_CONST_ARGUMENT);

    if callee_cost.calls == 0 {
        benefit = benefit.saturating_add(INLINE_BENEFIT_LEAF);
    }

    let callsite_count = profile
        .and_then(|profile| {
            profile
                .callsite_profile(callsite_id)
                .map(|callsite_profile| {
                    scaled_profile_count(
                        callsite_profile.total_count,
                        profile.source,
                        &policy.scaling_policy(),
                    )
                })
        })
        .unwrap_or(0);
    let callsite_count = if callsite_count == 0 {
        block_count
    } else {
        callsite_count
    };

    let count_multiplier = if callsite_count == 0 {
        1
    } else {
        let raw = callsite_count / INLINE_BENEFIT_COUNT_DIVISOR;
        raw.clamp(1, INLINE_BENEFIT_COUNT_MAX_MULTIPLIER)
    };
    let ratio_multiplier = if entry_count == 0 {
        1
    } else {
        let ratio_percent = callsite_count.saturating_mul(100) / entry_count.max(1);
        let raw = ratio_percent / 10;
        raw.clamp(1, INLINE_BENEFIT_COUNT_MAX_MULTIPLIER)
    };
    let mut multiplier = count_multiplier.max(ratio_multiplier);

    if matches!(hotness, CallsiteHotness::Hot) {
        multiplier = (multiplier * 2).min(INLINE_BENEFIT_COUNT_MAX_MULTIPLIER);
    }

    benefit.saturating_mul(multiplier)
}

/// Compute the inline budget for a module.
fn inline_budget_for_module(
    tree: &mir::NodeTree,
    profile: Option<&mir::ProfileTable>,
    policy: &CallsiteHotnessPolicy,
    inline_budget_scale_percent: u64,
) -> u64 {
    let Some(profile) = profile else {
        return scale_inline_budget(
            INLINE_MODULE_BUDGET_BASE,
            inline_budget_scale_percent,
            INLINE_MODULE_BUDGET_MAX,
        );
    };

    let mut total_entry = 0u64;
    for (function_id, _) in tree.iter_nodes::<mir::Function>() {
        let Some(function_profile) = profile.function_profile(function_id) else {
            continue;
        };
        total_entry = total_entry.saturating_add(scaled_profile_count(
            function_profile.entry_count,
            profile.source,
            &policy.scaling_policy(),
        ));
    }

    let bonus = total_entry / INLINE_MODULE_BUDGET_ENTRY_DIVISOR;
    let budget = INLINE_MODULE_BUDGET_BASE
        .saturating_add(bonus)
        .min(INLINE_MODULE_BUDGET_MAX);
    scale_inline_budget(
        budget,
        inline_budget_scale_percent,
        INLINE_MODULE_BUDGET_MAX,
    )
}

/// Compute inline budgets per SCC.
fn inline_scc_budgets(
    tree: &mir::NodeTree,
    scc_map: &CallGraphScc,
    profile: Option<&mir::ProfileTable>,
    policy: &CallsiteHotnessPolicy,
    inline_budget_scale_percent: u64,
) -> HashMap<usize, u64> {
    let mut scc_entry_counts: HashMap<usize, u64> = HashMap::new();
    let mut scc_ids = HashSet::new();

    for (function_id, _) in tree.iter_nodes::<mir::Function>() {
        let Some(scc_id) = scc_map.scc_id(function_id) else {
            continue;
        };
        scc_ids.insert(scc_id);

        let entry = profile
            .and_then(|profile| {
                profile
                    .function_profile(function_id)
                    .map(|function_profile| {
                        scaled_profile_count(
                            function_profile.entry_count,
                            profile.source,
                            &policy.scaling_policy(),
                        )
                    })
            })
            .unwrap_or(0);
        let total = scc_entry_counts.entry(scc_id).or_insert(0);
        *total = total.saturating_add(entry);
    }

    let mut budgets = HashMap::new();
    for scc_id in scc_ids {
        let entry_count = scc_entry_counts.get(&scc_id).copied().unwrap_or(0);
        let bonus = entry_count / INLINE_SCC_BUDGET_ENTRY_DIVISOR;
        let budget = INLINE_SCC_BUDGET_BASE
            .saturating_add(bonus)
            .min(INLINE_SCC_BUDGET_MAX);
        let budget =
            scale_inline_budget(budget, inline_budget_scale_percent, INLINE_SCC_BUDGET_MAX);
        budgets.insert(scc_id, budget);
    }

    budgets
}

/// Compute the cost for a single instruction.
fn instruction_cost(instruction: &mir::Instruction, tree: &mir::NodeTree) -> u64 {
    match instruction {
        mir::Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }
        mir::Instruction::Const { .. }
        | mir::Instruction::Binary { .. }
        | mir::Instruction::Unary { .. }
        | mir::Instruction::Cast { .. }
        | mir::Instruction::Select { .. }
        | mir::Instruction::LocalGet { .. }
        | mir::Instruction::LocalSet { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::FunctionAddr { .. }
        | mir::Instruction::FunctionBind { .. }
        | mir::Instruction::FunctionEnvironment { .. }
        | mir::Instruction::LocalAddr { .. }
        | mir::Instruction::GlobalConst { .. }
        | mir::Instruction::Assume { .. } => INLINE_COST_SIMPLE,
        mir::Instruction::VectorSplat { .. }
        | mir::Instruction::VectorExtract { .. }
        | mir::Instruction::VectorInsert { .. }
        | mir::Instruction::VectorShuffle { .. }
        | mir::Instruction::VectorSelect { .. }
        | mir::Instruction::VectorReduce { .. }
        | mir::Instruction::VectorCompare { .. }
        | mir::Instruction::VectorConvert { .. }
        | mir::Instruction::TensorReshape { .. }
        | mir::Instruction::TensorBroadcast { .. }
        | mir::Instruction::TensorTranspose { .. }
        | mir::Instruction::TensorCast { .. }
        | mir::Instruction::TensorView { .. }
        | mir::Instruction::TensorSlice { .. }
        | mir::Instruction::TensorPad { .. }
        | mir::Instruction::TensorConcat { .. }
        | mir::Instruction::TensorReduce { .. }
        | mir::Instruction::TensorDot { .. }
        | mir::Instruction::TensorConvolution { .. }
        | mir::Instruction::TensorGather { .. }
        | mir::Instruction::TensorScatter { .. }
        | mir::Instruction::TensorCompare { .. }
        | mir::Instruction::TensorSelect { .. }
        | mir::Instruction::TensorConvert { .. } => INLINE_COST_SIMPLE,
        mir::Instruction::TensorLoad { .. }
        | mir::Instruction::TensorStore { .. }
        | mir::Instruction::TensorFill { .. }
        | mir::Instruction::TensorCopy { .. } => INLINE_COST_MEMORY,
        mir::Instruction::Load { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::AtomicLoad { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicCompareExchange { .. }
        | mir::Instruction::AtomicRmw { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::Barrier { .. } => INLINE_COST_MEMORY,
        mir::Instruction::FieldGet { .. }
        | mir::Instruction::FieldAddr { .. }
        | mir::Instruction::FieldSet { .. }
        | mir::Instruction::ElementGet { .. }
        | mir::Instruction::ElementAddr { .. }
        | mir::Instruction::ElementSet { .. } => INLINE_COST_SIMPLE + 1,
        mir::Instruction::Struct { fields, .. } => {
            INLINE_COST_SIMPLE + tree.get_arguments(*fields).len() as u64
        }
        mir::Instruction::Tuple { elements, .. } => {
            INLINE_COST_SIMPLE + tree.get_arguments(*elements).len() as u64
        }
        mir::Instruction::Array { elements, .. } => {
            INLINE_COST_SIMPLE + tree.get_arguments(*elements).len() as u64
        }
        mir::Instruction::Call { .. } => INLINE_COST_CALL,
        mir::Instruction::CallVirtual { .. } | mir::Instruction::CallInterface { .. } => {
            INLINE_COST_CALL_INDIRECT
        }
        mir::Instruction::CallIndirect { .. } => INLINE_COST_CALL_INDIRECT,
        mir::Instruction::ManagedAlloc { .. }
        | mir::Instruction::ManagedAllocArray { .. }
        | mir::Instruction::RawAlloc { .. }
        | mir::Instruction::RawFree { .. }
        | mir::Instruction::Dispose { .. }
        | mir::Instruction::AsyncDispose { .. }
        | mir::Instruction::Drop { .. }
        | mir::Instruction::AsyncDrop { .. }
        | mir::Instruction::StackAlloc { .. } => INLINE_COST_ALLOC,
        mir::Instruction::Intrinsic { intrinsic, .. } => {
            if intrinsic.has_memory_effects() {
                INLINE_COST_MEMORY + 2
            } else {
                INLINE_COST_SIMPLE + 1
            }
        }
    }
}

/// Compute the cost of a terminator.
fn terminator_cost(terminator: &mir::Terminator) -> u64 {
    match terminator {
        mir::Terminator::Error => {
            panic!("recovered MIR terminator reached optimizer");
        }
        mir::Terminator::Return { .. } => INLINE_COST_SIMPLE,
        mir::Terminator::Throw { .. } => INLINE_COST_SIMPLE + 1,
        mir::Terminator::Trap { .. } => INLINE_COST_SIMPLE + 1,
        mir::Terminator::Jump { .. } => INLINE_COST_SIMPLE,
        mir::Terminator::Branch { .. }
        | mir::Terminator::Check { .. }
        | mir::Terminator::Switch { .. }
        | mir::Terminator::Yield { .. } => INLINE_COST_SIMPLE + 1,
        mir::Terminator::Invoke { .. } => INLINE_COST_CALL + 1,
        mir::Terminator::InvokeIndirect { .. }
        | mir::Terminator::InvokeVirtual { .. }
        | mir::Terminator::InvokeInterface { .. } => INLINE_COST_CALL_INDIRECT + 1,
        mir::Terminator::Unreachable => 0,
        mir::Terminator::TailCall { .. } => INLINE_COST_CALL,
        mir::Terminator::TailCallVirtual { .. } | mir::Terminator::TailCallInterface { .. } => {
            INLINE_COST_CALL_INDIRECT
        }
        mir::Terminator::TailCallIndirect { .. } => INLINE_COST_CALL_INDIRECT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Simple direct calls are inlined.
    #[test]
    fn test_inline_basic_call() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    v2: int32 = int.add v1, v0
    return v2
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: int32): int32 {
b0(v0: int32):
    jump b1(v0)
b1(v1: int32):
    v2: int32 = int.add v1, v1
    jump b2(v2)
b2(v3: int32):
    v4: int32 = int.add v3, v0
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&Inline);
        test.assert_output(expected);
    }

    /// Recursive calls are not inlined.
    #[test]
    fn test_inline_skips_recursive_call() {
        let input = r#"
function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call caller(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&Inline);
        test.assert_output(input);
    }

    /// Tail call callees are not inlined.
    #[test]
    fn test_inline_skips_tailcall_callee() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    tailCall callee(v0): (int32) -> int32
}
function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&Inline);
        test.assert_output(input);
    }

    /// Locals are cloned during inlining.
    #[test]
    fn test_inline_clones_locals() {
        let input = r#"
function callee(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    v1: int32 = local.get local0
    v2: int32 = int.add v1, v0
    return v2
}
function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    v1: int32 = local.get local0
    v2: int32 = int.add v1, v0
    return v2
}
function caller(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    jump b1(v0)
b1(v1: int32):
    v2: int32 = local.get local0
    v3: int32 = int.add v2, v1
    jump b2(v3)
b2(v4: int32):
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&Inline);
        test.assert_output(expected);
    }

    /// Inlined memory access metadata remaps pointer targets.
    #[test]
    fn test_inline_remaps_memory_access_metadata() {
        let input = r#"
function callee(): int32 {
    local local0: int32, owned
b0:
    v0: ref<int32, borrowed, space(frame)> = local.address local0
    v1: int32 = load v0
    return v1
}
function caller(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#;

        let mut test = TestProgram::new(input);

        let callee_id = test.function_id_by_name("callee");
        let callee = test.tree.get(callee_id);
        let mut callee_load = None;
        let mut callee_pointer = None;
        for block_id in &callee.blocks {
            let block = test.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::Load { pointer, .. } = test.tree.get(*instruction_id) {
                    callee_load = Some(*instruction_id);
                    callee_pointer = Some(*pointer);
                    break;
                }
            }
            if callee_load.is_some() {
                break;
            }
        }

        let callee_load = callee_load.expect("missing callee load");
        let callee_pointer = callee_pointer
            .expect("missing callee pointer")
            .value()
            .expect("callee pointer should be concrete");
        test.insert_pointer_access(
            callee_load,
            mir::MemoryAccessKind::Read,
            callee_pointer,
            None,
            Vec::new(),
            Vec::new(),
            None,
        );

        test.run_module_pass(&Inline);

        let caller_id = test.function_id_by_name("caller");
        let caller = test.tree.get(caller_id);
        let mut inlined_load = None;
        let mut inlined_pointer = None;
        for block_id in &caller.blocks {
            let block = test.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if let mir::Instruction::Load { pointer, .. } = test.tree.get(*instruction_id) {
                    inlined_load = Some(*instruction_id);
                    inlined_pointer = Some(*pointer);
                    break;
                }
            }
            if inlined_load.is_some() {
                break;
            }
        }

        let inlined_load = inlined_load.expect("missing inlined load");
        let inlined_pointer = inlined_pointer
            .expect("missing inlined pointer")
            .value()
            .expect("inlined pointer should be concrete");
        let accesses = test
            .tree
            .metadata
            .memory
            .memory_accesses(inlined_load)
            .expect("missing inlined access metadata");
        assert_eq!(accesses.len(), 1);
        match accesses[0].target {
            mir::MemoryAccessTarget::Pointer(value) => {
                assert_eq!(value, inlined_pointer);
            }
            _ => panic!("unexpected access target"),
        }
    }

    /// Large callees are not inlined.
    #[test]
    fn test_inline_skips_large_callee() {
        let mut input = String::from("function callee(v0: int32): int32 {\n");
        input.push_str("b0(v0: int32):\n");
        input.push_str("    v1: int32 = int.add v0, v0\n");
        for index in 2..=97 {
            input.push_str(&format!(
                "    v{index}: int32 = int.add v{}, v0\n",
                index - 1
            ));
        }
        input.push_str("    return v97\n");
        input.push_str("}\n");
        input.push_str("function caller(v0: int32): int32 {\n");
        input.push_str("b0(v0: int32):\n");
        input.push_str("    v1: int32 = call callee(v0): (int32) -> int32\n");
        input.push_str("    return v1\n");
        input.push_str("}\n");

        let mut test = TestProgram::new(&input);
        test.run_module_pass(&Inline);
        test.assert_output(&input);
    }

    /// Calls with unused return values inline without continuation arguments.
    #[test]
    fn test_inline_unused_return() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: int32): void {
b0(v0: int32):
    call callee(v0): (int32) -> int32
    return
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: int32): void {
b0(v0: int32):
    jump b1(v0)
b1(v1: int32):
    v2: int32 = int.add v1, v1
    jump b2
b2:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&Inline);
        test.assert_output(expected);
    }

    /// Cold callsites avoid inlining under profile guidance.
    #[test]
    fn test_inline_skips_cold_callsite() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    v2: int32 = int.add v1, v0
    v3: int32 = int.add v2, v0
    v4: int32 = int.add v3, v0
    v5: int32 = int.add v4, v0
    v6: int32 = int.add v5, v0
    v7: int32 = int.add v6, v0
    v8: int32 = int.add v7, v0
    v9: int32 = int.add v8, v0
    return v9
}
function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        let caller_id = test.function_id_by_name("caller");
        let (call_id, _) = test.first_call_in_entry(caller_id);

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_function_profile(&mut profile, caller_id, 100);
        test.record_callsite_profile(&mut profile, call_id, 5);

        test.run_module_pass_with_profile(&Inline, profile);
        test.assert_output(input);
    }

    /// Hot callsites enable larger inlines under profile guidance.
    #[test]
    fn test_inline_uses_hot_callsite() {
        let input = r#"
function helper(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    v2: int32 = int.add v1, v0
    v3: int32 = int.add v2, v0
    v4: int32 = int.add v3, v0
    v5: int32 = int.add v4, v0
    v6: int32 = int.add v5, v0
    v7: int32 = int.add v6, v0
    v8: int32 = int.add v7, v0
    v9: int32 = int.add v8, v0
    return v9
}
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call helper(v0): (int32) -> int32
    v2: int32 = call helper(v1): (int32) -> int32
    v3: int32 = call helper(v2): (int32) -> int32
    v4: int32 = call helper(v3): (int32) -> int32
    v5: int32 = call helper(v4): (int32) -> int32
    return v5
}
function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let expected = r#"
function helper(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    v2: int32 = int.add v1, v0
    v3: int32 = int.add v2, v0
    v4: int32 = int.add v3, v0
    v5: int32 = int.add v4, v0
    v6: int32 = int.add v5, v0
    v7: int32 = int.add v6, v0
    v8: int32 = int.add v7, v0
    v9: int32 = int.add v8, v0
    return v9
}
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call helper(v0): (int32) -> int32
    v2: int32 = call helper(v1): (int32) -> int32
    v3: int32 = call helper(v2): (int32) -> int32
    v4: int32 = call helper(v3): (int32) -> int32
    v5: int32 = call helper(v4): (int32) -> int32
    return v5
}
function caller(v0: int32): int32 {
b0(v0: int32):
    jump b1(v0)
b1(v1: int32):
    v2: int32 = call helper(v1): (int32) -> int32
    v3: int32 = call helper(v2): (int32) -> int32
    v4: int32 = call helper(v3): (int32) -> int32
    v5: int32 = call helper(v4): (int32) -> int32
    v6: int32 = call helper(v5): (int32) -> int32
    jump b2(v6)
b2(v7: int32):
    return v7
}"#;

        let mut test = TestProgram::new(input);
        let caller_id = test.function_id_by_name("caller");
        let callee_id = test.function_id_by_name("callee");
        let (call_id, _) = test.first_call_in_entry(caller_id);

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_function_profile(&mut profile, caller_id, 100);
        test.record_callsite_profile(&mut profile, call_id, 200);
        for call_id in test.call_instructions_in_function(callee_id) {
            test.record_callsite_profile(&mut profile, call_id, 1);
        }

        test.run_module_pass_with_profile(&Inline, profile);
        test.assert_output(expected);
    }

    /// Inline budgets scale with profile entry counts.
    #[test]
    fn test_inline_budget_scales_with_profile() {
        let policy = CallsiteHotnessPolicy::inline_default();
        let function_id = mir::LocalNodeId::<mir::Function>::new(1);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        profile.functions.insert(
            function_id,
            mir::FunctionProfile {
                entry_count: mir::ProfileCount::new(500, mir::ProfileConfidence::Precise),
            },
        );

        let base = inline_budget_for_function(function_id, None, &policy, 100);
        let scaled = inline_budget_for_function(function_id, Some(&profile), &policy, 100);

        assert!(scaled > base);
        assert!(scaled <= INLINE_BUDGET_MAX);
    }

    /// Block profiles can classify hotness for missing callsite data.
    #[test]
    fn test_inline_hotness_from_block_profile() {
        let policy = CallsiteHotnessPolicy::inline_default();
        let hot = block_hotness_from_counts(100, 100, &policy);
        let cold = block_hotness_from_counts(1, 100, &policy);

        assert_eq!(hot, CallsiteHotness::Hot);
        assert_eq!(cold, CallsiteHotness::Cold);
    }

    /// Block execution counts fall back to edge profiles when missing.
    #[test]
    fn test_inline_block_counts_use_edges() {
        let policy = CallsiteHotnessPolicy::inline_default();
        let input = r#"
function test(): void {
b0:
    jump b1
b1:
    return
}"#;

        let test = TestProgram::new(input);
        let function_id = test.function_id_by_name("test");
        let block0 = test.entry_block_id(function_id);
        let block1 = {
            let function = test.tree.get(function_id);
            *function.blocks.get(1).expect("missing block1")
        };

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_jump_edge_profile(&mut profile, block0, block1, 42);

        let function = test.tree.get(function_id);
        let counts = block_execution_counts(function, &test.tree, Some(&profile), &policy);

        assert_eq!(counts.get(&block1), Some(&42));
    }

    /// Inline replaces multiple returns with a continuation.
    #[test]
    fn test_inline_multiple_returns() {
        let input = r#"
function callee(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    return v3
b2:
    v4: int32 = int.sub v0, v1
    return v4
}
function caller(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = call callee(v0, v1, v2): (int32, int32, boolean) -> int32
    return v3
}"#;

        let expected = r#"
function callee(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    return v3
b2:
    v4: int32 = int.sub v0, v1
    return v4
}
function caller(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    jump b1(v0, v1, v2)
b1(v3: int32, v4: int32, v5: boolean):
    branch v5, b2, b3
b2:
    v6: int32 = int.add v3, v4
    jump b4(v6)
b3:
    v7: int32 = int.sub v3, v4
    jump b4(v7)
b4(v8: int32):
    return v8
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&Inline);
        test.assert_output(expected);
    }

    /// Inline forwards call results into continuation terminators.
    #[test]
    fn test_inline_continuation_argument() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call callee(v0): (int32) -> int32
    jump b1(v1)
b1(v2: int32):
    return v2
}"#;

        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: int32): int32 {
b0(v0: int32):
    jump b2(v0)
b1(v1: int32):
    return v1
b2(v2: int32):
    v3: int32 = int.add v2, v2
    jump b3(v3)
b3(v4: int32):
    jump b1(v4)
}"#;

        let mut test = TestProgram::new(input);
        test.run_module_pass(&Inline);
        test.assert_output(expected);
    }

    /// Call indirect sites do not inline without a direct target.
    #[test]
    fn test_inline_skips_indirect_call() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}
function caller(v0: fn(int32) -> int32, v1: int32): int32  {
b0(v0: fn(int32) -> int32, v1: int32) -> v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);

        test.run_module_pass(&Inline);
        test.assert_output(input);
    }
}
