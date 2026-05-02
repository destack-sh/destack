use std::collections::{HashMap, HashSet};

use crate::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, ConstantPropagation, ControlFlowGraph, DominatorTree, LoopAnalysis,
    MemoryAccessEffect, MemorySSA,
};
use crate::optimize::common::{
    BlockParamForwarding, LoopEffectPolicy, ValueEquivalence, block_is_speculatable_no_reads,
    build_instruction_block_map, build_value_definition_map, clone_instruction_metadata,
    collect_loop_effects, control_instructions_for_latch, effects_may_alias,
    instruction_is_speculatable, instruction_map, loop_guard_branch, loop_preheader,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Fuse adjacent loops with identical bounds and independent bodies.
    ///
    /// This pass merges two sequential loops into one by interleaving their bodies
    /// per iteration when they share the same induction bounds and do not alias.
    ///
    /// ```mir
    /// function before(v0: uint32): void {
    /// b0(v0: uint32):
    ///     v1 = stack.alloc int32 -> ref<int32, raw, space(stack)>
    ///     v2 = stack.alloc int32 -> ref<int32, raw, space(stack)>
    ///     v3 = 0uint32
    ///     v4 = 1uint32
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v0
    ///     branch v6, b2(v5), b3
    /// b2(v7: uint32):
    ///     v8 = element.address v1, v7 -> ref<int32, raw, space(stack)>
    ///     v9 = 1int32
    ///     store v8, v9
    ///     v10 = int.add v7, v4
    ///     jump b1(v10)
    /// b3:
    ///     jump b4(v3)
    /// b4(v11: uint32):
    ///     v12 = int.lt.u v11, v0
    ///     branch v12, b5(v11), b6
    /// b5(v13: uint32):
    ///     v14 = element.address v2, v13 -> ref<int32, raw, space(stack)>
    ///     v15 = 2int32
    ///     store v14, v15
    ///     v16 = int.add v13, v4
    ///     jump b4(v16)
    /// b6:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: uint32): void {
    /// b0(v0: uint32):
    ///     v1 = stack.alloc int32 -> ref<int32, raw, space(stack)>
    ///     v2 = stack.alloc int32 -> ref<int32, raw, space(stack)>
    ///     v3 = 0uint32
    ///     v4 = 1uint32
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v0
    ///     branch v6, b2(v5), b6
    /// b2(v7: uint32):
    ///     v8 = element.address v1, v7 -> ref<int32, raw, space(stack)>
    ///     v9 = 1int32
    ///     store v8, v9
    ///     v14 = element.address v2, v7 -> ref<int32, raw, space(stack)>
    ///     v15 = 2int32
    ///     store v14, v15
    ///     v10 = int.add v7, v4
    ///     jump b1(v10)
    /// b6:
    ///     return
    /// }
    /// ```
    #[pass(id = "loop-fusion")]
    pub LoopFusion,
    "Fuse adjacent loops with identical bounds"
}

impl FunctionPass for LoopFusion {
    /// Run loop fusion on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let loops = analyses.get::<LoopAnalysis>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();
        let memory_ssa = analyses.get::<MemorySSA>();
        let alias = analyses.get::<AliasAnalysis>().clone();
        let constants = analyses.get::<ConstantPropagation>();

        // run loop fusion
        let changed = run_loop_fusion(
            function,
            tree,
            &loops,
            &cfg,
            &domtree,
            memory_ssa.as_ref(),
            &alias,
            constants.as_ref(),
        );

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "LoopFusion"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "loop-fusion"
    }
}

/// Loop fusion candidate data.
struct FusionCandidate {
    /// First loop data.
    first: LoopBody,
    /// Second loop data.
    second: LoopBody,
}

/// Per loop metadata needed for fusion.
struct LoopBody {
    /// Loop header block.
    header: mir::LocalNodeId<mir::Block>,
    /// Loop latch block.
    latch: mir::LocalNodeId<mir::Block>,
    /// Exit block outside the loop.
    exit_block: mir::LocalNodeId<mir::Block>,
    /// Preheader block.
    preheader: mir::LocalNodeId<mir::Block>,
    /// Arguments passed from preheader to header.
    preheader_args: Vec<mir::Value>,
    /// Loop guard information.
    guard: GuardInfo,
    /// Instructions used to compute latch jump arguments.
    control_instructions: HashSet<mir::LocalNodeId<mir::Instruction>>,
    /// Ordered body instructions in the latch.
    body_instructions: Vec<mir::LocalNodeId<mir::Instruction>>,
}

/// Guard metadata for a loop.
struct GuardInfo {
    /// Induction parameter index.
    induction_index: usize,
    /// Bound value.
    bound: mir::Value,
    /// Guard operator.
    operator: mir::BinaryOperator,
    /// Constant step for the induction variable.
    step: i64,
    /// True when the guard is signed.
    is_signed: bool,
}

/// Run loop fusion and return true when changes are made.
#[allow(clippy::too_many_arguments)]
fn run_loop_fusion(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    loops: &LoopAnalysis,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    constants: &ConstantPropagation,
) -> bool {
    // build definition maps
    let definitions = build_value_definition_map(function, tree);
    let instruction_blocks = build_instruction_block_map(function, tree);
    let forwarding = BlockParamForwarding::build(function, tree, cfg);

    // locate a fusion candidate
    let mut equivalence =
        ValueEquivalence::new_with_constants(tree, &definitions, constants, &instruction_blocks);

    let candidate = loops.loops().iter().enumerate().find_map(|(index, lp)| {
        build_fusion_candidate(
            index,
            lp,
            loops,
            tree,
            cfg,
            domtree,
            memory_ssa,
            alias,
            constants,
            &forwarding,
            &definitions,
            &instruction_blocks,
            &mut equivalence,
        )
    });
    let Some(candidate) = candidate else {
        return false;
    };

    // apply the fusion
    apply_fusion(
        function,
        tree,
        cfg,
        &candidate,
        &definitions,
        &instruction_blocks,
    )
}

/// Build a fusion candidate from a loop.
#[allow(clippy::too_many_arguments)]
fn build_fusion_candidate(
    loop_index: usize,
    lp: &crate::optimize::analyses::Loop,
    loops: &LoopAnalysis,
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    constants: &ConstantPropagation,
    forwarding: &BlockParamForwarding,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
    equivalence: &mut ValueEquivalence<'_>,
) -> Option<FusionCandidate> {
    // require a single latch and exit
    if !lp.has_single_latch() || !lp.has_single_exit() {
        return None;
    }

    // require header and latch form
    if lp.blocks.len() != 2 {
        return None;
    }

    // resolve the latch block
    let latch = *lp.latches.first()?;
    if latch == lp.header {
        return None;
    }

    // resolve preheader
    let (preheader, preheader_args) = loop_preheader(lp.header, &lp.blocks, cfg, domtree, tree)?;

    // resolve guard and exit block
    let guard = loop_guard_branch(lp.header, latch, &lp.blocks, tree)?;
    if !guard.exit_arguments.is_empty() {
        return None;
    }

    // require empty exit parameters
    if !tree.get(guard.exit_block).parameters.is_empty() {
        return None;
    }

    // resolve guard info
    let loop_guard = guard_info(lp.header, latch, tree, constants, forwarding, definitions)?;

    // reject non speculatable instructions in the latch
    if !latch_is_speculatable(latch, tree, memory_ssa) {
        return None;
    }

    // collect loop body data
    let control_instructions =
        control_instructions_for_latch(lp.header, latch, tree, definitions, instruction_blocks);
    let body_instructions = latch_body_instructions(latch, tree, &control_instructions);
    let body_effects =
        collect_loop_effects(&lp.blocks, tree, memory_ssa, LoopEffectPolicy::ReadWrite)?;

    // record the first loop metadata
    let first = LoopBody {
        header: lp.header,
        latch,
        exit_block: guard.exit_block,
        preheader,
        preheader_args,
        guard: loop_guard,
        control_instructions,
        body_instructions,
    };

    // locate the next loop by preheader
    let next_loop = loops.loops().iter().enumerate().find(|(index, other)| {
        if *index == loop_index {
            return false;
        }

        let Some((other_preheader, _)) =
            loop_preheader(other.header, &other.blocks, cfg, domtree, tree)
        else {
            return false;
        };

        other_preheader == guard.exit_block
    })?;

    let (second_index, second_loop) = next_loop;

    // require the candidate loop to be distinct
    if second_index == loop_index {
        return None;
    }

    // require the next loop to be canonical
    if !second_loop.has_single_latch() || !second_loop.has_single_exit() {
        return None;
    }

    // require header and latch form
    if second_loop.blocks.len() != 2 {
        return None;
    }

    // build second loop body
    let (second_preheader, second_preheader_args) =
        loop_preheader(second_loop.header, &second_loop.blocks, cfg, domtree, tree)?;
    if second_preheader != guard.exit_block {
        return None;
    }

    // require the second preheader to be empty
    let preheader_block = tree.get(second_preheader);
    if !preheader_block.instructions.is_empty() {
        return None;
    }

    // resolve the second guard
    let second_guard = loop_guard_branch(
        second_loop.header,
        *second_loop.latches.first()?,
        &second_loop.blocks,
        tree,
    )?;
    if !second_guard.exit_arguments.is_empty() {
        return None;
    }

    // require empty exit parameters
    if !tree.get(second_guard.exit_block).parameters.is_empty() {
        return None;
    }

    // require droppable header instructions
    if !block_is_speculatable_no_reads(second_loop.header, tree, memory_ssa) {
        return None;
    }

    // resolve the second guard info
    let second_guard_info = guard_info(
        second_loop.header,
        *second_loop.latches.first()?,
        tree,
        constants,
        forwarding,
        definitions,
    )?;

    // reject non speculatable instructions in the latch
    if !latch_is_speculatable(*second_loop.latches.first()?, tree, memory_ssa) {
        return None;
    }

    // collect second loop body data
    let second_control = control_instructions_for_latch(
        second_loop.header,
        *second_loop.latches.first()?,
        tree,
        definitions,
        instruction_blocks,
    );
    let second_body = latch_body_instructions(*second_loop.latches.first()?, tree, &second_control);
    let second_effects = collect_loop_effects(
        &second_loop.blocks,
        tree,
        memory_ssa,
        LoopEffectPolicy::ReadWrite,
    )?;

    // check bounds compatibility
    if !guards_compatible(
        &first,
        &second_guard_info,
        &second_preheader_args,
        equivalence,
    ) {
        return None;
    }

    // ensure loop bodies are independent
    if !effects_are_independent(&body_effects, &second_effects, tree, alias) {
        return None;
    }

    // require matching preheader arguments
    if !preheader_args_match(&first.preheader_args, &second_preheader_args, equivalence) {
        return None;
    }

    // record the second loop metadata
    let second = LoopBody {
        header: second_loop.header,
        latch: *second_loop.latches.first()?,
        exit_block: second_guard.exit_block,
        preheader: second_preheader,
        preheader_args: second_preheader_args,
        guard: second_guard_info,
        control_instructions: second_control,
        body_instructions: second_body,
    };

    Some(FusionCandidate { first, second })
}

/// Build guard metadata from a header and latch.
fn guard_info(
    header: mir::LocalNodeId<mir::Block>,
    latch: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    constants: &ConstantPropagation,
    forwarding: &BlockParamForwarding,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Option<GuardInfo> {
    // resolve the guard condition
    let header_block = tree.get(header);
    let header_terminator = tree.get(header_block.terminator);
    let condition = match header_terminator {
        mir::Terminator::Branch { condition, .. } => condition.value()?,
        _ => return None,
    };

    // resolve the comparison instruction
    let condition_def = definitions.get(&condition)?;
    let instruction = tree.get(*condition_def);
    let mir::Instruction::Binary {
        operator,
        left,
        right,
        ..
    } = instruction
    else {
        return None;
    };

    // require strict comparisons
    let (is_signed, is_strict) = match operator {
        mir::BinaryOperator::SignedLessThan | mir::BinaryOperator::SignedGreaterThan => {
            (true, true)
        }
        mir::BinaryOperator::UnsignedLessThan | mir::BinaryOperator::UnsignedGreaterThan => {
            (false, true)
        }
        _ => return None,
    };

    // resolve the induction parameter
    let induction_index = header_block
        .parameters
        .iter()
        .position(|param| param.value.value() == left.value())?;

    // resolve the induction step
    let step = induction_step(
        header,
        latch,
        induction_index,
        tree,
        constants,
        forwarding,
        definitions,
    )?;
    if step == 0 {
        return None;
    }

    // validate the step direction
    let operator = *operator;
    let bound = right.value()?;

    // reject less than guards with non positive steps
    if matches!(
        operator,
        mir::BinaryOperator::SignedLessThan | mir::BinaryOperator::UnsignedLessThan
    ) && step <= 0
    {
        return None;
    }

    // reject greater than guards with non negative steps
    if matches!(
        operator,
        mir::BinaryOperator::SignedGreaterThan | mir::BinaryOperator::UnsignedGreaterThan
    ) && step >= 0
    {
        return None;
    }

    // build guard info for strict comparisons
    if is_strict {
        Some(GuardInfo {
            induction_index,
            bound,
            operator,
            step,
            is_signed,
        })
    } else {
        None
    }
}

/// Resolve the induction step for a header parameter.
fn induction_step(
    header: mir::LocalNodeId<mir::Block>,
    latch: mir::LocalNodeId<mir::Block>,
    induction_index: usize,
    tree: &mir::Tree,
    constants: &ConstantPropagation,
    forwarding: &BlockParamForwarding,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Option<i64> {
    // locate the latch jump argument
    let latch_block = tree.get(latch);
    let latch_terminator = tree.get(latch_block.terminator);
    let mir::Terminator::Jump { target } = latch_terminator else {
        return None;
    };

    // resolve the induction argument
    let arg = target.arguments.get(induction_index).copied()?.value()?;
    let header_param = tree
        .get(header)
        .parameters
        .get(induction_index)?
        .value
        .value()?;

    // reject unchanged induction values
    if forwarding.resolve(arg) == header_param {
        return None;
    }

    // resolve the step from the latch argument
    let definition = definitions.get(&arg)?;
    let instruction = tree.get(*definition);
    let mir::Instruction::Binary {
        operator,
        left,
        right,
        ..
    } = instruction
    else {
        return None;
    };

    // compute the constant step
    let step = match operator {
        mir::BinaryOperator::Add
            if left
                .value()
                .is_some_and(|left| forwarding.resolve(left) == header_param) =>
        {
            const_i64(right.value()?, latch, constants)?
        }
        mir::BinaryOperator::Subtract
            if left
                .value()
                .is_some_and(|left| forwarding.resolve(left) == header_param) =>
        {
            -const_i64(right.value()?, latch, constants)?
        }
        _ => return None,
    };

    Some(step)
}

/// Check whether a latch contains only speculatable instructions.
fn latch_is_speculatable(
    latch: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> bool {
    // scan latch instructions
    let block = tree.get(latch);
    for instruction_id in &block.instructions {
        let instruction = tree.get(*instruction_id);

        // accept speculatable instructions
        if instruction_is_speculatable(instruction, tree) {
            continue;
        }

        // accept explicit memory operations
        if matches!(
            instruction,
            mir::Instruction::Store { .. }
                | mir::Instruction::LocalSet { .. }
                | mir::Instruction::Load { .. }
        ) {
            continue;
        }

        // reject remaining instructions
        return false;
    }

    // check memory effects for volatility
    let mut blocks = HashSet::new();
    blocks.insert(latch);
    collect_loop_effects(&blocks, tree, memory_ssa, LoopEffectPolicy::ReadWrite).is_some()
}

/// Collect ordered body instructions for a latch.
fn latch_body_instructions(
    latch: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    control: &HashSet<mir::LocalNodeId<mir::Instruction>>,
) -> Vec<mir::LocalNodeId<mir::Instruction>> {
    // return instructions that are not part of control
    let block = tree.get(latch);
    block
        .instructions
        .iter()
        .filter(|id| !control.contains(id))
        .copied()
        .collect()
}

/// Check whether two loop effect sets are independent.
fn effects_are_independent(
    first: &[MemoryAccessEffect],
    second: &[MemoryAccessEffect],
    tree: &mir::Tree,
    alias: &AliasAnalysis,
) -> bool {
    // compare each pair
    for effect in first {
        for other in second {
            if !(effect.writes || other.writes) {
                continue;
            }

            if effects_may_alias(tree, alias, effect, other) {
                return false;
            }
        }
    }

    true
}

/// Compare guard compatibility between loops.
fn guards_compatible(
    first: &LoopBody,
    second_guard: &GuardInfo,
    second_preheader_args: &[mir::Value],
    equivalence: &mut ValueEquivalence<'_>,
) -> bool {
    // require matching induction indices
    if first.guard.induction_index != second_guard.induction_index {
        return false;
    }

    // require matching operators
    if first.guard.operator != second_guard.operator {
        return false;
    }

    // require matching signedness
    if first.guard.is_signed != second_guard.is_signed {
        return false;
    }

    // require matching step
    if first.guard.step != second_guard.step {
        return false;
    }

    // require equivalent bounds
    let first_bound = first.guard.bound;
    let second_bound = second_guard.bound;
    if !equivalence.equivalent(first_bound, second_bound) {
        return false;
    }

    // ensure preheader args are compatible for the induction param
    let Some(first_start) = first.preheader_args.get(first.guard.induction_index) else {
        return false;
    };
    let Some(second_start) = second_preheader_args.get(second_guard.induction_index) else {
        return false;
    };
    equivalence.equivalent(*first_start, *second_start)
}

/// Compare two preheader argument lists.
fn preheader_args_match(
    first: &[mir::Value],
    second: &[mir::Value],
    equivalence: &mut ValueEquivalence<'_>,
) -> bool {
    // require equal arity
    if first.len() != second.len() {
        return false;
    }

    // compare each argument pair
    for (left, right) in first.iter().zip(second.iter()) {
        if !equivalence.equivalent(*left, *right) {
            return false;
        }
    }

    true
}

/// Resolve a constant integer value when possible.
fn const_i64(
    value: mir::Value,
    block: mir::LocalNodeId<mir::Block>,
    constants: &ConstantPropagation,
) -> Option<i64> {
    // resolve the constant for this block
    let constant = constants.exit(block).get(value)?;
    match constant {
        mir::Constant::Int { value, .. } => i64::try_from(*value).ok(),
        mir::Constant::UInt { value, .. } => i64::try_from(*value).ok(),
        _ => None,
    }
}

/// Apply loop fusion to a candidate.
fn apply_fusion(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    cfg: &ControlFlowGraph,
    candidate: &FusionCandidate,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
) -> bool {
    // prepare new values
    function.recompute_next_value_id(tree);

    // map loop2 parameters to loop1 parameters
    let first_header = tree.get(candidate.first.header);
    let second_header = tree.get(candidate.second.header);
    if first_header.parameters.len() != second_header.parameters.len() {
        return false;
    }

    // build the value remap for parameters
    let mut value_map: HashMap<mir::Value, mir::Value> = HashMap::new();
    for (first_param, second_param) in first_header
        .parameters
        .iter()
        .zip(second_header.parameters.iter())
    {
        let Some(first_param) = first_param.typed_value() else {
            return false;
        };
        let Some(second_param) = second_param.typed_value() else {
            return false;
        };

        if first_param.ty != second_param.ty {
            return false;
        }

        value_map.insert(second_param.value, first_param.value);
    }

    // map latch parameters between the loops
    let first_latch = tree.get(candidate.first.latch);
    let second_latch = tree.get(candidate.second.latch);
    if first_latch.parameters.len() != second_latch.parameters.len() {
        return false;
    }

    for (first_param, second_param) in first_latch
        .parameters
        .iter()
        .zip(second_latch.parameters.iter())
    {
        let Some(first_param) = first_param.typed_value() else {
            return false;
        };
        let Some(second_param) = second_param.typed_value() else {
            return false;
        };

        // require matching parameter types
        if first_param.ty != second_param.ty {
            return false;
        }

        // record latch parameter mapping
        value_map.insert(second_param.value, first_param.value);
    }

    // map loop2 body definitions
    for &instruction_id in &candidate.second.body_instructions {
        let instruction = tree.get(instruction_id);
        if let Some(destination) = instruction.destination().and_then(|value| value.value()) {
            let new_value = function.next_typed_value_like(destination);
            value_map.insert(destination, new_value);
        }
    }

    // ensure loop2 body does not depend on header local values
    for &instruction_id in &candidate.second.body_instructions {
        let instruction = tree.get(instruction_id);
        for value in instruction
            .uses()
            .into_iter()
            .filter_map(|value| value.value())
        {
            if value_map.contains_key(&value) {
                continue;
            }

            let Some(definition) = definitions.get(&value) else {
                continue;
            };

            if instruction_blocks.get(definition) == Some(&candidate.second.header) {
                return false;
            }
        }
    }

    // locate the insertion point in the latch
    let mut latch_block = tree.get(candidate.first.latch).clone();
    let insertion_index = latch_block
        .instructions
        .iter()
        .position(|id| candidate.first.control_instructions.contains(id))
        .unwrap_or(latch_block.instructions.len());

    // build the new instruction list prefix
    let mut new_instructions = Vec::new();
    for &instruction_id in latch_block.instructions.iter().take(insertion_index) {
        new_instructions.push(instruction_id);
    }

    // clone loop2 body instructions into the latch
    for &instruction_id in &candidate.second.body_instructions {
        let original = tree.get(instruction_id).clone();
        let new_instruction = instruction_map(&original, &value_map, tree);
        let new_instruction_id = tree.insert(new_instruction);
        clone_instruction_metadata(tree, instruction_id, new_instruction_id, &value_map);
        new_instructions.push(new_instruction_id);
    }

    // append the original control instructions
    for &instruction_id in latch_block.instructions.iter().skip(insertion_index) {
        new_instructions.push(instruction_id);
    }

    // commit the rewritten latch
    latch_block.instructions = new_instructions;
    tree.replace(candidate.first.latch, latch_block);

    // update loop1 header exit to loop2 exit
    let header_block = tree.get(candidate.first.header).clone();
    let header_terminator = tree.get(header_block.terminator).clone();
    let mir::Terminator::Branch {
        condition,
        then_target,
        else_target,
    } = &header_terminator
    else {
        return false;
    };

    // rewrite the exit target
    let in_loop_is_then = then_target.block.block() == Some(candidate.first.latch);
    let new_terminator = if in_loop_is_then {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: then_target.clone(),
            else_target: mir::BlockTarget {
                block: candidate.second.exit_block.into(),
                arguments: Vec::new(),
            },
        }
    } else {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: mir::BlockTarget {
                block: candidate.second.exit_block.into(),
                arguments: Vec::new(),
            },
            else_target: else_target.clone(),
        }
    };
    tree.replace(candidate.first.header, header_block);
    tree.replace(tree.get(candidate.first.header).terminator, new_terminator);

    // drop loop2 blocks from the function
    let mut to_remove = HashSet::new();
    to_remove.insert(candidate.second.header);
    to_remove.insert(candidate.second.latch);
    if cfg.predecessors(candidate.second.preheader).len() == 1 {
        to_remove.insert(candidate.second.preheader);
    }

    // prune removed blocks from the function
    function
        .blocks
        .retain(|block_id| !to_remove.contains(block_id));

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Adjacent loops with identical bounds are fused.
    #[test]
    fn test_loop_fusion_merges_adjacent_loops() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    jump b1(v3)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v0
    branch v6, b2(v5), b3
b2(v7: uint32):
    v8: ref<int32, raw, space(stack)> = element.address v1, v7
    v9: int32 = 1int32
    store v8, v9
    v10: uint32 = int.add v7, v4
    jump b1(v10)
b3:
    jump b4(v3)
b4(v11: uint32):
    v12: boolean = int.lt.u v11, v0
    branch v12, b5(v11), b6
b5(v13: uint32):
    v14: ref<int32, raw, space(stack)> = element.address v2, v13
    v15: int32 = 2int32
    store v14, v15
    v16: uint32 = int.add v13, v4
    jump b4(v16)
b6:
    return
}"#;

        let expected = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    jump b1(v3)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v0
    branch v6, b2(v5), b3
b2(v7: uint32):
    v8: ref<int32, raw, space(stack)> = element.address v1, v7
    v9: int32 = 1int32
    store v8, v9
    v10: ref<int32, raw, space(stack)> = element.address v2, v7
    v11: int32 = 2int32
    store v10, v11
    v12: uint32 = int.add v7, v4
    jump b1(v12)
b3:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(expected);
    }

    /// Loops with matching carry arguments are fused.
    #[test]
    fn test_loop_fusion_merges_with_carry_args() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    v5: uint32 = 7uint32
    jump b1(v3, v5)
b1(v6: uint32, v7: uint32):
    v8: boolean = int.lt.u v6, v0
    branch v8, b2(v6, v7), b3
b2(v9: uint32, v10: uint32):
    v11: ref<int32, raw, space(stack)> = element.address v1, v9
    store v11, v10
    v12: uint32 = int.add v9, v4
    jump b1(v12, v10)
b3:
    jump b4(v3, v5)
b4(v13: uint32, v14: uint32):
    v15: boolean = int.lt.u v13, v0
    branch v15, b5(v13, v14), b6
b5(v16: uint32, v17: uint32):
    v18: ref<int32, raw, space(stack)> = element.address v2, v16
    store v18, v17
    v19: uint32 = int.add v16, v4
    jump b4(v19, v17)
b6:
    return
}"#;

        let expected = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    v5: uint32 = 7uint32
    jump b1(v3, v5)
b1(v6: uint32, v7: uint32):
    v8: boolean = int.lt.u v6, v0
    branch v8, b2(v6, v7), b3
b2(v9: uint32, v10: uint32):
    v11: ref<int32, raw, space(stack)> = element.address v1, v9
    store v11, v10
    v12: ref<int32, raw, space(stack)> = element.address v2, v9
    store v12, v10
    v13: uint32 = int.add v9, v4
    jump b1(v13, v10)
b3:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(expected);
    }

    /// Loops with aliasing stores are not fused.
    #[test]
    fn test_loop_fusion_skips_aliasing() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: uint32 = 0uint32
    v3: uint32 = 1uint32
    jump b1(v2)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v0
    branch v5, b2(v4), b3
b2(v6: uint32):
    v7: ref<int32, raw, space(stack)> = element.address v1, v6
    v8: int32 = 1int32
    store v7, v8
    v9: uint32 = int.add v6, v3
    jump b1(v9)
b3:
    jump b4(v2)
b4(v10: uint32):
    v11: boolean = int.lt.u v10, v0
    branch v11, b5(v10), b6
b5(v12: uint32):
    v13: ref<int32, raw, space(stack)> = element.address v1, v12
    v14: int32 = 2int32
    store v13, v14
    v15: uint32 = int.add v12, v3
    jump b4(v15)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }

    /// Non empty preheaders prevent fusion.
    #[test]
    fn test_loop_fusion_skips_non_empty_preheader() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    jump b1(v3)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v0
    branch v6, b2(v5), b3
b2(v7: uint32):
    v8: ref<int32, raw, space(stack)> = element.address v1, v7
    v9: int32 = 1int32
    store v8, v9
    v10: uint32 = int.add v7, v4
    jump b1(v10)
b3:
    v11: uint32 = 0uint32
    jump b4(v3)
b4(v12: uint32):
    v13: boolean = int.lt.u v12, v0
    branch v13, b5(v12), b6
b5(v14: uint32):
    v15: ref<int32, raw, space(stack)> = element.address v2, v14
    v16: int32 = 2int32
    store v15, v16
    v17: uint32 = int.add v14, v4
    jump b4(v17)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }

    /// Loops with mismatched bounds are not fused.
    #[test]
    fn test_loop_fusion_skips_mismatched_bounds() {
        let input = r#"
function test(v0: uint32, v1: uint32): void {
b0(v0: uint32, v1: uint32):
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: ref<int32, raw, space(stack)> = stack.alloc int32
    v4: uint32 = 0uint32
    v5: uint32 = 1uint32
    jump b1(v4)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v0
    branch v7, b2(v6), b3
b2(v8: uint32):
    v9: ref<int32, raw, space(stack)> = element.address v2, v8
    v10: int32 = 1int32
    store v9, v10
    v11: uint32 = int.add v8, v5
    jump b1(v11)
b3:
    jump b4(v4)
b4(v12: uint32):
    v13: boolean = int.lt.u v12, v1
    branch v13, b5(v12), b6
b5(v14: uint32):
    v15: ref<int32, raw, space(stack)> = element.address v3, v14
    v16: int32 = 2int32
    store v15, v16
    v17: uint32 = int.add v14, v5
    jump b4(v17)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }

    /// Header reads in the second loop prevent fusion.
    #[test]
    fn test_loop_fusion_skips_header_reads() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    jump b1(v3)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v0
    branch v6, b2(v5), b3
b2(v7: uint32):
    v8: ref<int32, raw, space(stack)> = element.address v1, v7
    v9: int32 = 1int32
    store v8, v9
    v10: uint32 = int.add v7, v4
    jump b1(v10)
b3:
    jump b4(v3)
b4(v11: uint32):
    v12: int32 = load v2
    v13: boolean = int.lt.u v11, v0
    branch v13, b5(v11), b6
b5(v14: uint32):
    v15: ref<int32, raw, space(stack)> = element.address v2, v14
    v16: int32 = 2int32
    store v15, v16
    v17: uint32 = int.add v14, v4
    jump b4(v17)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }

    /// Header defined values used in the second latch prevent fusion.
    #[test]
    fn test_loop_fusion_skips_header_latch_dependency() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    jump b1(v3)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v0
    branch v6, b2(v5), b3
b2(v7: uint32):
    v8: ref<int32, raw, space(stack)> = element.address v1, v7
    v9: int32 = 1int32
    store v8, v9
    v10: uint32 = int.add v7, v4
    jump b1(v10)
b3:
    jump b4(v3)
b4(v11: uint32):
    v12: uint32 = int.add v11, v4
    v13: boolean = int.lt.u v11, v0
    branch v13, b5(v11), b6
b5(v14: uint32):
    v15: ref<int32, raw, space(stack)> = element.address v2, v14
    v16: int32 = 2int32
    store v15, v16
    v17: uint32 = int.add v12, v4
    jump b4(v17)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }

    /// Non speculatable latch instructions prevent fusion.
    #[test]
    fn test_loop_fusion_skips_side_effects() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    jump b1(v3)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v0
    branch v6, b2(v5), b3
b2(v7: uint32):
    v8: ref<int32, raw, space(stack)> = element.address v1, v7
    v9: int32 = 1int32
    store v8, v9
    v10: uint32 = int.add v7, v4
    jump b1(v10)
b3:
    jump b4(v3)
b4(v11: uint32):
    v12: boolean = int.lt.u v11, v0
    branch v12, b5(v11), b6
b5(v13: uint32):
    call touch(v13): (uint32) -> void
    v14: ref<int32, raw, space(stack)> = element.address v2, v13
    v15: int32 = 2int32
    store v14, v15
    v16: uint32 = int.add v13, v4
    jump b4(v16)
b6:
    return
}
function touch(v0: uint32): void {
b0(v0: uint32):
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }

    /// Non adjacent loops are not fused.
    #[test]
    fn test_loop_fusion_skips_non_adjacent_loops() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    jump b1(v3)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v0
    branch v6, b2(v5), b3
b2(v7: uint32):
    v8: ref<int32, raw, space(stack)> = element.address v1, v7
    v9: int32 = 1int32
    store v8, v9
    v10: uint32 = int.add v7, v4
    jump b1(v10)
b3:
    jump b4(v3)
b4(v11: uint32):
    jump b5(v11)
b5(v12: uint32):
    v13: boolean = int.lt.u v12, v0
    branch v13, b6(v12), b7
b6(v14: uint32):
    v15: ref<int32, raw, space(stack)> = element.address v2, v14
    v16: int32 = 2int32
    store v15, v16
    v17: uint32 = int.add v14, v4
    jump b5(v17)
b7:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }

    /// Mismatched loop carried arguments prevent fusion.
    #[test]
    fn test_loop_fusion_skips_mismatched_carry_args() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    v5: uint32 = 10uint32
    v6: uint32 = 20uint32
    jump b1(v3, v5)
b1(v7: uint32, v8: uint32):
    v9: boolean = int.lt.u v7, v0
    branch v9, b2(v7, v8), b3
b2(v10: uint32, v11: uint32):
    v12: ref<int32, raw, space(stack)> = element.address v1, v10
    v13: int32 = 1int32
    store v12, v13
    v14: uint32 = int.add v10, v4
    jump b1(v14, v11)
b3:
    jump b4(v3, v6)
b4(v15: uint32, v16: uint32):
    v17: boolean = int.lt.u v15, v0
    branch v17, b5(v15, v16), b6
b5(v18: uint32, v19: uint32):
    v20: ref<int32, raw, space(stack)> = element.address v2, v18
    v21: int32 = 2int32
    store v20, v21
    v22: uint32 = int.add v18, v4
    jump b4(v22, v19)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }

    /// Mismatched steps prevent fusion.
    #[test]
    fn test_loop_fusion_skips_step_mismatch() {
        let input = r#"
function test(v0: uint32): void {
b0(v0: uint32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: uint32 = 0uint32
    v4: uint32 = 1uint32
    v5: uint32 = 2uint32
    jump b1(v3)
b1(v6: uint32):
    v7: boolean = int.lt.u v6, v0
    branch v7, b2(v6), b3
b2(v8: uint32):
    v9: ref<int32, raw, space(stack)> = element.address v1, v8
    v10: int32 = 1int32
    store v9, v10
    v11: uint32 = int.add v8, v4
    jump b1(v11)
b3:
    jump b4(v3)
b4(v12: uint32):
    v13: boolean = int.lt.u v12, v0
    branch v13, b5(v12), b6
b5(v14: uint32):
    v15: ref<int32, raw, space(stack)> = element.address v2, v14
    v16: int32 = 2int32
    store v15, v16
    v17: uint32 = int.add v14, v5
    jump b4(v17)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopFusion);
        test.assert_output(input);
    }
}
