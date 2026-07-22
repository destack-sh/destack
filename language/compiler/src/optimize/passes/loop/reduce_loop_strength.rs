use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    ControlFlowGraph, DominatorTree, Loop, LoopAnalysis, Mutation, RangeAnalysis, ScalarEvolution,
    Scev, TargetLayout, ValueRange, ValueTypes, clone_instruction_tables, constant_is_zero,
    instruction_is_speculatable, instruction_map, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, resolve_substitution_chains, terminator_substitute_uses,
};

declare_pass! {
    /// Reduce strength of loop expressions derived from induction variables.
    ///
    /// Rewrites loop values with linear recurrences into explicit header
    /// parameters updated by simple additions in the latch.
    ///
    /// ```mir
    /// function before(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = 0int32
    ///     v2 = 4int32
    ///     v3 = 1int32
    ///     jump b1(v1)
    /// b1(v4: int32):
    ///     v5 = int.lt.s v4, v0
    ///     branch v5, b2, b3
    /// b2:
    ///     v6 = int.mul v4, v2
    ///     v7 = int.add v6, v3
    ///     v8 = int.add v4, v3
    ///     jump b1(v8)
    /// b3:
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = 0int32
    ///     v2 = 4int32
    ///     v3 = 1int32
    ///     jump b1(v1, v1)
    /// b1(v4: int32, v9: int32):
    ///     v5 = int.lt.s v4, v0
    ///     branch v5, b2, b3
    /// b2:
    ///     v6 = int.mul v4, v2
    ///     v7 = int.add v9, v3
    ///     v8 = int.add v4, v3
    ///     v10 = int.add v9, v2
    ///     jump b1(v8, v10)
    /// b3:
    ///     return v4
    /// }
    /// ```
    #[pass(id = "reduce-loop-strength")]
    pub ReduceLoopStrength,
    "Reduce strength of loop derived computations"
}

impl FunctionPass for ReduceLoopStrength {
    /// Run the loop strength reduction pass.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let memory = &mut optimized.memory;

        // skip imported functions
        if function.entry().is_none() {
            return Mutation::NONE;
        }

        // gather analyses
        let loops = analyses.get::<LoopAnalysis>(function, tree).clone();
        let cfg = analyses.get::<ControlFlowGraph>(function, tree).clone();
        let domtree = analyses.get::<DominatorTree>(function, tree).clone();
        let scev = analyses.get::<ScalarEvolution>(function, tree).clone();
        let ranges = analyses.get::<RangeAnalysis>(function, tree).clone();
        let value_types = analyses.get::<ValueTypes>(function, tree);

        // skip when no loops are present
        if loops.num_loops() == 0 {
            return Mutation::NONE;
        }

        // run the strength reduction pass
        let context = StrengthReduceContext {
            loops: &loops,
            cfg: &cfg,
            domtree: &domtree,
            scev: &scev,
            value_types: &value_types,
            ranges: &ranges,
            target_layout: ctx.target_layout(),
        };
        let changed = run_reduce_loop_strength(function, tree, memory, &context);
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "ReduceLoopStrength"
    }

    /// Return the stable id for this pass.
    fn id(&self) -> &'static str {
        "reduce-loop-strength"
    }
}

/// Candidate for strength reduction in a loop.
#[derive(Debug, Clone)]
struct StrengthReductionCandidate {
    /// Loop header block.
    header: mir::LocalNodeId<mir::Block>,
    /// Loop preheader block.
    preheader: mir::LocalNodeId<mir::Block>,
    /// Loop latch block.
    latch: mir::LocalNodeId<mir::Block>,
    /// Value to replace.
    value: mir::Value,
    /// Recurrence start expression.
    start: Scev,
    /// Recurrence step expression.
    step: Scev,
    /// Type of the value being replaced.
    value_type: mir::LocalNodeId<mir::Type>,
    /// Blocks inside the loop.
    loop_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
}

/// Plan item for rewriting a loop value.
#[derive(Debug, Clone)]
struct StrengthReductionPlanItem {
    /// Original value to replace.
    original: mir::Value,
    /// New header parameter.
    new_param: mir::TypedValue,
    /// Start value provided by the preheader.
    start_value: mir::Value,
    /// Step value added each iteration.
    step_value: mir::Value,
    /// Next value computed in the latch.
    next_value: mir::Value,
}

/// Definition kind for a value.
#[derive(Debug, Clone, Copy)]
enum ValueDefinitionKind {
    /// Block parameter definition.
    Parameter,
    /// Instruction definition.
    Instruction {
        /// Instruction that defines the value.
        instruction: mir::LocalNodeId<mir::Instruction>,
    },
}

/// Definition tables for a value.
#[derive(Debug, Clone, Copy)]
struct ValueDefinition {
    /// Block where the value is defined.
    block: mir::LocalNodeId<mir::Block>,
    /// Definition kind.
    kind: ValueDefinitionKind,
}

/// Map of values to their definitions.
#[derive(Debug)]
struct ValueDefinitions {
    /// Definitions keyed by value.
    definitions: HashMap<mir::Value, ValueDefinition>,
}

impl ValueDefinitions {
    /// Build a definition map for a function.
    fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        // collect parameter and instruction definitions
        let mut definitions = HashMap::new();

        // scan blocks for definitions
        for &block_id in function.blocks() {
            // record block parameters
            let block = tree.get(block_id);
            for param in block.parameters.iter() {
                let value = param.value;

                definitions.insert(
                    value,
                    ValueDefinition {
                        block: block_id,
                        kind: ValueDefinitionKind::Parameter,
                    },
                );
            }

            // record instruction destinations
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    definitions.insert(
                        destination,
                        ValueDefinition {
                            block: block_id,
                            kind: ValueDefinitionKind::Instruction {
                                instruction: instruction_id,
                            },
                        },
                    );
                }
            }
        }

        Self { definitions }
    }

    /// Get the definition for a value.
    fn definition_for(&self, value: mir::Value) -> Option<ValueDefinition> {
        self.definitions.get(&value).copied()
    }
}

/// Map of values to the blocks where they are used.
#[derive(Debug)]
struct ValueUses {
    /// Use sites keyed by value.
    uses: HashMap<mir::Value, HashSet<mir::LocalNodeId<mir::Block>>>,
}

impl ValueUses {
    /// Build a use map for a function.
    fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        // collect value uses per block
        let mut uses: HashMap<mir::Value, HashSet<mir::LocalNodeId<mir::Block>>> = HashMap::new();

        // scan blocks for uses
        for &block_id in function.blocks() {
            let block = tree.get(block_id);

            // scan instructions for uses
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                for value in instruction.uses() {
                    uses.entry(value).or_default().insert(block_id);
                }

                // scan external argument slices
                if let Some(args) = instruction.argument_slice() {
                    for &value in tree.get_values(args) {
                        uses.entry(value).or_default().insert(block_id);
                    }
                }
            }

            // scan terminator uses
            let terminator = tree.get(block.terminator);
            for value in terminator.uses(tree) {
                uses.entry(value).or_default().insert(block_id);
            }
        }

        Self { uses }
    }

    /// Get the blocks where a value is used.
    fn blocks_for(&self, value: mir::Value) -> Option<&HashSet<mir::LocalNodeId<mir::Block>>> {
        self.uses.get(&value)
    }
}

/// Shared context for strength reduction.
struct StrengthReduceContext<'a> {
    /// Loop analysis results.
    loops: &'a LoopAnalysis,
    /// Control flow graph for the function.
    cfg: &'a ControlFlowGraph,
    /// Dominator tree for the function.
    domtree: &'a DominatorTree,
    /// Scalar evolution analysis.
    scev: &'a ScalarEvolution,
    /// Value type lookup for the function.
    value_types: &'a ValueTypes,
    /// Range analysis for loop invariants.
    ranges: &'a RangeAnalysis,
    /// Type context for layout sensitive operations.
    target_layout: TargetLayout,
}

/// Shared context for collecting strength reduction candidates.
struct CandidateContext<'a> {
    /// tree for the function.
    tree: &'a mir::Tree,
    /// Loop analysis results.
    loops: &'a LoopAnalysis,
    /// Control flow graph for the function.
    cfg: &'a ControlFlowGraph,
    /// Dominator tree for the function.
    domtree: &'a DominatorTree,
    /// Scalar evolution analysis.
    scev: &'a ScalarEvolution,
    /// Value type lookup for the function.
    value_types: &'a ValueTypes,
    /// Value definition tables.
    definitions: &'a ValueDefinitions,
    /// Value use tables.
    uses: &'a ValueUses,
    /// Range analysis for safety checks.
    ranges: &'a RangeAnalysis,
    /// Type context for layout sensitive operations.
    target_layout: TargetLayout,
}

impl<'a> CandidateContext<'a> {
    /// Collect strength reduction candidates from all loops.
    fn collect_candidates(&self) -> Vec<StrengthReductionCandidate> {
        // collect candidate values
        let mut candidates = Vec::new();

        // scan each loop for reducible values
        for (loop_index, lp) in self.loops.loops().iter().enumerate() {
            // require a single latch
            if !lp.has_single_latch() {
                continue;
            }

            // resolve the canonical preheader and latch
            let Some(preheader) = find_preheader(lp, self.cfg, self.domtree) else {
                continue;
            };
            let latch = lp.latches[0];

            // ensure preheader and latch reach the header
            let preheader_block = self.tree.get(preheader);
            let preheader_terminator = self.tree.get(preheader_block.terminator);
            if !terminator_has_successor(self.tree, preheader_terminator, lp.header) {
                continue;
            }

            // ensure the latch has a back edge to the header
            let latch_block = self.tree.get(latch);
            let latch_terminator = self.tree.get(latch_block.terminator);
            if !terminator_has_successor(self.tree, latch_terminator, lp.header) {
                continue;
            }

            // scan blocks inside the loop
            for &block_id in &lp.blocks {
                // skip blocks owned by nested loops
                let Some(inner_loop) = self.loops.innermost_loop(block_id) else {
                    continue;
                };
                if inner_loop.header != lp.header {
                    continue;
                }

                // scan instructions in the block
                let block = self.tree.get(block_id);
                for &instruction_id in &block.instructions {
                    // read the instruction and its destination
                    let instruction = self.tree.get(instruction_id);
                    let Some(destination) = instruction.destination() else {
                        continue;
                    };

                    // require a profitable strength reduction candidate
                    if !instruction_is_candidate(instruction) {
                        continue;
                    }

                    // skip potentially trapping divisions and remainders
                    if let mir::Instruction::Binary {
                        operator,
                        left,
                        right,
                        ..
                    } = instruction
                        && !division_is_safe(
                            *operator,
                            *left,
                            *right,
                            block_id,
                            self.ranges,
                            self.value_types,
                            self.target_layout.pointer_bits(),
                            self.tree,
                        )
                    {
                        continue;
                    }

                    // require an integer type for the value
                    let value_type = self.value_types.expect_value_type(destination);
                    if !type_is_integer(value_type, self.target_layout.pointer_bits(), self.tree) {
                        continue;
                    }

                    // require a loop recurrence
                    let Some(scev_value) = self.scev.value_scev(loop_index, destination) else {
                        continue;
                    };
                    let scev_value = scev_value.clone();

                    // require a recurrence anchored on the loop header
                    let Scev::AddRec {
                        start,
                        step,
                        loop_header,
                    } = scev_value
                    else {
                        continue;
                    };

                    // skip recurrences from other headers
                    if loop_header != lp.header {
                        continue;
                    }

                    // require invariant start and step
                    if !start.is_loop_invariant(lp.header) || !step.is_loop_invariant(lp.header) {
                        continue;
                    }

                    // skip recurrences with zero step
                    if scev_is_zero(&step) {
                        continue;
                    }

                    // require loop local definition
                    let Some(definition) = self.definitions.definition_for(destination) else {
                        continue;
                    };
                    if !lp.blocks.contains(&definition.block) {
                        continue;
                    }

                    // skip parameters that are already induction variables
                    if matches!(definition.kind, ValueDefinitionKind::Parameter) {
                        continue;
                    }

                    // require uses dominated by the header
                    if !uses_within_loop(destination, &lp.blocks, self.uses) {
                        continue;
                    }

                    // record the candidate for transformation
                    candidates.push(StrengthReductionCandidate {
                        header: lp.header,
                        preheader,
                        latch,
                        value: destination,
                        start: *start,
                        step: *step,
                        value_type,
                        loop_blocks: lp.blocks.clone(),
                    });
                }
            }
        }

        candidates
    }
}

/// Build strength reduction candidates and apply transformations.
fn run_reduce_loop_strength(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    context: &StrengthReduceContext<'_>,
) -> bool {
    // build definition and use tables
    let definitions = ValueDefinitions::build(function, tree);
    let uses = ValueUses::build(function, tree);

    // collect candidates across loops
    let candidates_context = CandidateContext {
        tree,
        loops: context.loops,
        cfg: context.cfg,
        domtree: context.domtree,
        scev: context.scev,
        value_types: context.value_types,
        definitions: &definitions,
        uses: &uses,
        ranges: context.ranges,
        target_layout: context.target_layout,
    };
    let candidates = candidates_context.collect_candidates();

    // skip when no candidates were found
    if candidates.is_empty() {
        return false;
    }

    // refresh value allocation state
    function.recompute_next_value_id(tree);

    // group candidates by loop header
    let mut candidates_by_header: HashMap<
        mir::LocalNodeId<mir::Block>,
        Vec<StrengthReductionCandidate>,
    > = HashMap::new();
    for candidate in candidates {
        candidates_by_header
            .entry(candidate.header)
            .or_default()
            .push(candidate);
    }

    // apply transformations and collect substitutions
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut headers: Vec<_> = candidates_by_header.keys().copied().collect();
    headers.sort();

    // process candidates header by header
    for header in headers {
        // gather candidates for the header
        let mut loop_candidates = candidates_by_header.remove(&header).unwrap_or_default();
        loop_candidates.sort_by_key(|candidate| candidate.value);

        // apply loop specific rewrites
        let loop_substitutions = apply_candidates_for_loop(
            function,
            tree,
            memory,
            &loop_candidates,
            &definitions,
            context.value_types,
            context.ranges,
            context.domtree,
            context.target_layout,
        );

        // merge substitutions into the global map
        for (old_value, new_value) in loop_substitutions {
            substitutions.insert(old_value, new_value);
        }
    }

    // skip if no substitutions were applied
    if substitutions.is_empty() {
        return false;
    }

    // resolve substitution chains
    let substitutions = resolve_substitution_chains(substitutions);

    // apply substitutions to instructions
    for &block_id in function.blocks() {
        // collect instruction ids to avoid borrow issues
        let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();

        // rewrite instruction operands
        for instruction_id in instruction_ids {
            let instruction = tree.get(instruction_id).clone();
            let new_instruction =
                instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);

            // replace instructions that changed
            if new_instruction != instruction {
                tree.set(instruction_id, new_instruction);
                remap_instruction_memory_accesses(memory, instruction_id, &substitutions);
            }
        }
    }

    // apply substitutions to terminators
    for &block_id in function.blocks() {
        // read the current block
        let terminator_id = tree.get(block_id).terminator;
        let terminator = tree.get(terminator_id).clone();

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(tree, &terminator, &substitutions);

        // replace blocks that changed
        if new_terminator != terminator {
            tree.set(terminator_id, new_terminator);
        }
    }

    true
}

/// Apply candidates for a single loop and return substitutions.
fn apply_candidates_for_loop(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    candidates: &[StrengthReductionCandidate],
    definitions: &ValueDefinitions,
    value_types: &ValueTypes,
    ranges: &RangeAnalysis,
    domtree: &DominatorTree,
    target_layout: TargetLayout,
) -> Vec<(mir::Value, mir::Value)> {
    // skip empty candidate lists
    if candidates.is_empty() {
        return Vec::new();
    }

    // use the first candidate to get loop tables
    let header = candidates[0].header;
    let preheader = candidates[0].preheader;
    let latch = candidates[0].latch;
    let loop_blocks = &candidates[0].loop_blocks;

    // build a materializer for the preheader
    let mut materializer = ScevMaterializer::new(
        tree,
        memory,
        preheader,
        loop_blocks,
        definitions,
        value_types,
        ranges,
        domtree,
        target_layout,
    );

    // build plan items
    let mut plan_items: Vec<StrengthReductionPlanItem> = Vec::new();
    for candidate in candidates {
        // materialize the start value
        let Some(start_value) = materializer.materialize(function, &candidate.start) else {
            continue;
        };

        // materialize the step value
        let Some(step_value) = materializer.materialize(function, &candidate.step) else {
            continue;
        };

        // allocate the new header parameter value
        let new_param_value = function.next_typed_value(candidate.value_type);
        let new_param = mir::TypedValue::new(new_param_value, candidate.value_type);

        // allocate the next value for the latch update
        let next_value = function.next_typed_value(candidate.value_type);

        // record the planned rewrite
        plan_items.push(StrengthReductionPlanItem {
            original: candidate.value,
            new_param,
            start_value,
            step_value,
            next_value,
        });
    }

    // skip when nothing could be materialized
    if plan_items.is_empty() {
        return Vec::new();
    }

    // compute updated latch terminator
    let latch_block = tree.get(latch).clone();
    let latch_current_terminator = tree.get(latch_block.terminator).clone();

    // collect latch arguments in header parameter order
    let latch_args: Vec<_> = plan_items.iter().map(|item| item.next_value).collect();
    let Some(latch_terminator) =
        append_successor_arguments(tree, &latch_current_terminator, header, &latch_args)
    else {
        return Vec::new();
    };

    // compute updated preheader terminator
    let preheader_block = tree.get(preheader).clone();
    let preheader_current_terminator = tree.get(preheader_block.terminator).clone();

    // collect preheader arguments in header parameter order
    let preheader_args: Vec<_> = plan_items.iter().map(|item| item.start_value).collect();
    let Some(preheader_terminator) =
        append_successor_arguments(tree, &preheader_current_terminator, header, &preheader_args)
    else {
        return Vec::new();
    };

    // update header parameters
    let mut header_block = tree.get(header).clone();

    // append new parameters in header order
    for item in &plan_items {
        header_block.parameters.push(mir::BlockParameter {
            value: item.new_param.value,
            ty: item.new_param.ty,
        });
    }
    tree.set(header, header_block);

    // insert recurrence updates into the latch
    let mut latch_instructions = latch_block.instructions.clone();
    for item in &plan_items {
        let instruction = mir::Instruction::Binary {
            destination: item.next_value,
            operator: mir::BinaryOperator::Add,
            left: item.new_param.value,
            right: item.step_value,
        };
        let instruction_id = tree.insert(instruction);
        latch_instructions.push(instruction_id);
    }
    function.replace_block_instructions(latch, latch_instructions, tree);

    // install latch terminator
    tree.set(latch_block.terminator, latch_terminator);

    // install preheader terminator
    if preheader_terminator != preheader_current_terminator {
        tree.set(preheader_block.terminator, preheader_terminator);
    }

    // return substitutions
    plan_items
        .iter()
        .map(|item| (item.original, item.new_param.value))
        .collect()
}

/// Check if an instruction is a strength reduction candidate.
fn instruction_is_candidate(instruction: &mir::Instruction) -> bool {
    // check supported instruction kinds
    match instruction {
        mir::Instruction::Binary { operator, .. } => matches!(
            operator,
            mir::BinaryOperator::Multiply
                | mir::BinaryOperator::SignedDivide
                | mir::BinaryOperator::UnsignedDivide
                | mir::BinaryOperator::SignedRemainder
                | mir::BinaryOperator::UnsignedRemainder
        ),
        _ => false,
    }
}

/// Check if a SCEV expression is a constant zero.
fn scev_is_zero(scev: &Scev) -> bool {
    match scev {
        Scev::Constant(constant) => constant_is_zero(Some(constant)),
        _ => false,
    }
}

/// Check if a type is an integer.
fn type_is_integer(
    ty: mir::LocalNodeId<mir::Type>,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> bool {
    // inspect the referenced type
    tree.get(ty)
        .int_info_with_pointer_width(pointer_width_bits)
        .is_some()
}

/// Check whether a division or remainder is safe to eliminate.
fn division_is_safe(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
    value_types: &ValueTypes,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> bool {
    match operator {
        mir::BinaryOperator::SignedDivide | mir::BinaryOperator::SignedRemainder => {
            signed_division_is_safe(
                left,
                right,
                block_id,
                ranges,
                value_types,
                pointer_width_bits,
                tree,
            )
        }
        mir::BinaryOperator::UnsignedDivide | mir::BinaryOperator::UnsignedRemainder => {
            unsigned_division_is_safe(right, block_id, ranges)
        }
        _ => true,
    }
}

/// Check whether a signed division or remainder is proven safe.
fn signed_division_is_safe(
    left: mir::Value,
    right: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
    value_types: &ValueTypes,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> bool {
    // require signed operand ranges
    let Some(right_range) = signed_integer_range(right, block_id, ranges) else {
        return false;
    };

    // divisor must be non zero
    if !integer_range_excludes_zero(&right_range) {
        return false;
    }

    // when divisor may be -1, require dividend to exclude min value
    if !integer_range_excludes_minus_one(&right_range) {
        let Some(left_range) = signed_integer_range(left, block_id, ranges) else {
            return false;
        };

        let Some(min_value) = signed_min_for_value(left, value_types, pointer_width_bits, tree)
            .or_else(|| signed_min_from_range(&left_range))
        else {
            return false;
        };

        if left_range.min <= min_value {
            return false;
        }
    }

    true
}

/// Check whether an unsigned division or remainder is proven safe.
fn unsigned_division_is_safe(
    right: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
) -> bool {
    let Some(range) = unsigned_integer_range(right, block_id, ranges) else {
        return false;
    };

    range.min > 0
}

/// Extract a signed integer range for a value.
fn signed_integer_range(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
) -> Option<IntegerRange> {
    let range = ranges.exit(block_id).get(value)?;
    let ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    } = range
    else {
        return None;
    };

    if !*is_signed {
        return None;
    }

    Some(IntegerRange {
        min: *min,
        max: *max,
        width: *width,
        is_signed: *is_signed,
    })
}

/// Extract an unsigned integer range for a value.
fn unsigned_integer_range(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
) -> Option<IntegerRange> {
    let range = ranges.exit(block_id).get(value)?;
    let ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    } = range
    else {
        return None;
    };

    if *is_signed {
        return None;
    }

    Some(IntegerRange {
        min: *min,
        max: *max,
        width: *width,
        is_signed: *is_signed,
    })
}

/// Integer range snapshot.
#[derive(Debug, Clone, Copy)]
struct IntegerRange {
    /// Minimum value.
    min: i128,
    /// Maximum value.
    max: i128,
    /// Bit width.
    width: u16,
    /// Signedness.
    is_signed: bool,
}

/// Check whether an integer range excludes zero.
fn integer_range_excludes_zero(range: &IntegerRange) -> bool {
    range.min > 0 || range.max < 0
}

/// Check whether an integer range excludes -1.
fn integer_range_excludes_minus_one(range: &IntegerRange) -> bool {
    range.min > -1 || range.max < -1
}

/// Extract the signed minimum for a value type.
fn signed_min_for_value(
    value: mir::Value,
    value_types: &ValueTypes,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> Option<i128> {
    let ty = value_types.expect_value_type(value);
    let (width, is_signed) = tree
        .get(ty)
        .int_info_with_pointer_width(pointer_width_bits)?;
    if !is_signed {
        return None;
    }

    signed_min_for_width(width)
}

/// Extract the signed minimum for a range when a type is unavailable.
fn signed_min_from_range(range: &IntegerRange) -> Option<i128> {
    if !range.is_signed {
        return None;
    }

    signed_min_for_width(range.width)
}

/// Compute the signed minimum value for a bit width.
fn signed_min_for_width(width: u16) -> Option<i128> {
    if width == 0 || width > 127 {
        return None;
    }

    let shift = (width - 1) as u32;
    Some(-(1_i128 << shift))
}

/// Check if all uses are dominated by the loop header.
fn uses_within_loop(
    value: mir::Value,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    uses: &ValueUses,
) -> bool {
    // skip values with no uses
    let Some(use_blocks) = uses.blocks_for(value) else {
        return false;
    };

    // ensure all uses stay inside the loop body
    for &block_id in use_blocks {
        if !loop_blocks.contains(&block_id) {
            return false;
        }
    }

    true
}

/// Find the loop preheader from dominance information.
fn find_preheader(
    lp: &Loop,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
) -> Option<mir::LocalNodeId<mir::Block>> {
    // require immediate dominator outside the loop
    let preheader = domtree.immediate_dominator(lp.header)?;
    if lp.blocks.contains(&preheader) {
        return None;
    }

    // require a single outside predecessor
    let outside_preds: Vec<_> = cfg
        .predecessors(lp.header)
        .iter()
        .copied()
        .filter(|pred| !lp.blocks.contains(pred))
        .collect();
    if outside_preds.len() != 1 || outside_preds[0] != preheader {
        return None;
    }

    Some(preheader)
}

/// Check if a terminator has an edge to a successor.
fn terminator_has_successor(
    tree: &mir::Tree,
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
) -> bool {
    // check successor list
    terminator.successors(tree).contains(&successor)
}

/// Append arguments for a successor edge.
fn append_successor_arguments(
    tree: &mut mir::Tree,
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
    new_args: &[mir::Value],
) -> Option<mir::Terminator> {
    // skip when nothing to append
    if new_args.is_empty() {
        return Some(terminator.clone());
    }

    // match terminator kinds with successor edges
    match terminator {
        mir::Terminator::Jump { target } => {
            // ensure the jump targets the successor
            if target.block != successor {
                return None;
            }

            // append arguments for the jump
            let updated_args = appended_arguments(tree, target.arguments, new_args);
            Some(mir::Terminator::Jump {
                target: mir::BlockTarget::new(target.block, updated_args),
            })
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            // update branch arguments for matching edges
            let mut updated_then = then_target.arguments;
            let mut updated_else = else_target.arguments;
            let mut touched = false;

            if then_target.block == successor {
                updated_then = appended_arguments(tree, then_target.arguments, new_args);
                touched = true;
            }

            // update else arguments when needed
            if else_target.block == successor {
                updated_else = appended_arguments(tree, else_target.arguments, new_args);
                touched = true;
            }

            // ensure the successor was updated
            if !touched {
                return None;
            }

            Some(mir::Terminator::Branch {
                condition: *condition,
                then_target: mir::BlockTarget::new(then_target.block, updated_then),
                else_target: mir::BlockTarget::new(else_target.block, updated_else),
            })
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            // update check target arguments for matching edges
            let mut updated_success = success.arguments;
            let mut updated_failure = failure.arguments;
            let mut touched = false;

            if success.block == successor {
                updated_success = appended_arguments(tree, success.arguments, new_args);
                touched = true;
            }

            // update failure arguments when needed
            if failure.block == successor {
                updated_failure = appended_arguments(tree, failure.arguments, new_args);
                touched = true;
            }

            // ensure the successor was updated
            if !touched {
                return None;
            }

            Some(mir::Terminator::Check {
                constraint: constraint.clone(),
                success: mir::BlockTarget::new(success.block, updated_success),
                failure: mir::BlockTarget::new(failure.block, updated_failure),
            })
        }
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            // update switch case arguments for matching edges
            let mut updated_cases = Vec::new();
            let mut updated_default = default.arguments;
            let mut touched = false;

            // update default arguments when needed
            if default.block == successor {
                updated_default = appended_arguments(tree, default.arguments, new_args);
                touched = true;
            }

            // update case arguments when needed
            let cases = tree.get_switch_cases(*cases).to_vec();
            for case in cases {
                let mut updated_case_args = case.target.arguments;
                if case.target.block == successor {
                    updated_case_args = appended_arguments(tree, case.target.arguments, new_args);
                    touched = true;
                }

                updated_cases.push(mir::SwitchCase {
                    value: case.value,
                    target: mir::BlockTarget::new(case.target.block, updated_case_args),
                });
            }

            // ensure the successor was updated
            if !touched {
                return None;
            }

            Some(mir::Terminator::Switch {
                value: *value,
                default: mir::BlockTarget::new(default.block, updated_default),
                cases: tree.add_switch_cases(&updated_cases),
            })
        }
        mir::Terminator::Yield {
            value,
            resume,
            unwind,
        } => {
            let successor = mir::BlockId::from(successor);
            let mut touched = false;

            // append resume arguments
            let mut updated_resume = resume.clone();
            if resume.block == successor {
                updated_resume.arguments = appended_arguments(tree, resume.arguments, new_args);
                touched = true;
            }

            // append unwind arguments
            let mut updated_unwind = unwind.clone();
            if let Some(unwind) = &mut updated_unwind
                && unwind.block == successor
            {
                unwind.arguments = appended_arguments(tree, unwind.arguments, new_args);
                touched = true;
            }

            if !touched {
                return None;
            }

            Some(mir::Terminator::Yield {
                value: *value,
                resume: updated_resume,
                unwind: updated_unwind,
            })
        }
        _ => None,
    }
}

/// Append values to one compact argument slice.
fn appended_arguments(
    tree: &mut mir::Tree,
    arguments: mir::ValueSlice,
    new_args: &[mir::Value],
) -> mir::ValueSlice {
    let mut arguments = tree.get_values(arguments).to_vec();
    arguments.extend_from_slice(new_args);

    tree.add_values(&arguments)
}

/// Helper for materializing SCEV expressions in the preheader.
struct ScevMaterializer<'a> {
    /// Mutable tree reference.
    tree: &'a mut mir::Tree,
    /// Mutable memory metadata reference.
    memory: &'a mut mir::MemoryTable,
    /// Preheader block id.
    preheader: mir::LocalNodeId<mir::Block>,
    /// Blocks inside the loop.
    loop_blocks: &'a HashSet<mir::LocalNodeId<mir::Block>>,
    /// Value definitions for the function.
    definitions: &'a ValueDefinitions,
    /// Value type lookup for the function.
    value_types: &'a ValueTypes,
    /// Range analysis for invariant checks.
    ranges: &'a RangeAnalysis,
    /// Dominator tree for availability checks.
    domtree: &'a DominatorTree,
    /// Cached constants in the preheader.
    constant_cache: Vec<(mir::Constant, mir::Value)>,
    /// Cached scev to value mappings.
    scev_cache: Vec<(Scev, mir::Value)>,
    /// Cached value mappings for cloned invariants.
    value_cache: HashMap<mir::Value, mir::Value>,
    /// Values currently being materialized.
    value_in_progress: HashSet<mir::Value>,
    /// Cached integer types by width and signedness.
    type_cache: HashMap<(u16, bool), mir::LocalNodeId<mir::Type>>,
    /// Type context for layout sensitive operations.
    target_layout: TargetLayout,
}

impl<'a> ScevMaterializer<'a> {
    /// Create a new materializer for the preheader.
    fn new(
        tree: &'a mut mir::Tree,
        memory: &'a mut mir::MemoryTable,
        preheader: mir::LocalNodeId<mir::Block>,
        loop_blocks: &'a HashSet<mir::LocalNodeId<mir::Block>>,
        definitions: &'a ValueDefinitions,
        value_types: &'a ValueTypes,
        ranges: &'a RangeAnalysis,
        domtree: &'a DominatorTree,
        target_layout: TargetLayout,
    ) -> Self {
        // collect constants already in the preheader
        let mut constant_cache = Vec::new();
        let preheader_block = tree.get(preheader);

        // scan preheader instructions for constants
        for &instruction_id in &preheader_block.instructions {
            let instruction = tree.get(instruction_id);
            let mir::Instruction::Const { destination, value } = instruction else {
                continue;
            };

            let destination = *destination;

            constant_cache.push((value.clone(), destination));
        }

        Self {
            tree,
            memory,
            preheader,
            loop_blocks,
            definitions,
            value_types,
            ranges,
            domtree,
            constant_cache,
            scev_cache: Vec::new(),
            value_cache: HashMap::new(),
            value_in_progress: HashSet::new(),
            type_cache: HashMap::new(),
            target_layout,
        }
    }

    /// Materialize a SCEV expression into the preheader.
    fn materialize(&mut self, function: &mut mir::Function, scev: &Scev) -> Option<mir::Value> {
        // check cached results
        if let Some(value) = self.cached_scev_value(scev) {
            return Some(value);
        }

        // materialize based on scev kind
        let value = match scev {
            Scev::Constant(constant) => self.materialize_constant(function, constant)?,
            Scev::Unknown(value) => self.materialize_unknown(function, *value)?,
            Scev::Neg(inner) => {
                let argument = self.materialize(function, inner)?;
                self.insert_unary(function, mir::UnaryOperator::Negate, argument)
            }
            Scev::Add(left, right) => {
                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(function, mir::BinaryOperator::Add, left_value, right_value)
            }
            Scev::Mul(left, right) => {
                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(
                    function,
                    mir::BinaryOperator::Multiply,
                    left_value,
                    right_value,
                )
            }
            Scev::SignedDivide(left, right) => {
                // require a non zero divisor for speculative execution
                if !self.scev_non_zero(right) || !self.scev_signed_division_safe(left, right) {
                    return None;
                }

                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(
                    function,
                    mir::BinaryOperator::SignedDivide,
                    left_value,
                    right_value,
                )
            }
            Scev::UnsignedDivide(left, right) => {
                // require a non zero divisor for speculative execution
                if !self.scev_non_zero(right) {
                    return None;
                }

                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(
                    function,
                    mir::BinaryOperator::UnsignedDivide,
                    left_value,
                    right_value,
                )
            }
            Scev::SignedRemainder(left, right) => {
                // require a non zero divisor for speculative execution
                if !self.scev_non_zero(right) || !self.scev_signed_division_safe(left, right) {
                    return None;
                }

                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(
                    function,
                    mir::BinaryOperator::SignedRemainder,
                    left_value,
                    right_value,
                )
            }
            Scev::UnsignedRemainder(left, right) => {
                // require a non zero divisor for speculative execution
                if !self.scev_non_zero(right) {
                    return None;
                }

                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(
                    function,
                    mir::BinaryOperator::UnsignedRemainder,
                    left_value,
                    right_value,
                )
            }
            Scev::ShiftLeft(left, right) => {
                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(
                    function,
                    mir::BinaryOperator::ShiftLeft,
                    left_value,
                    right_value,
                )
            }
            Scev::ArithmeticShiftRight(left, right) => {
                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(
                    function,
                    mir::BinaryOperator::ArithmeticShiftRight,
                    left_value,
                    right_value,
                )
            }
            Scev::LogicalShiftRight(left, right) => {
                let left_value = self.materialize(function, left)?;
                let right_value = self.materialize(function, right)?;
                self.insert_binary(
                    function,
                    mir::BinaryOperator::LogicalShiftRight,
                    left_value,
                    right_value,
                )
            }
            Scev::ZeroExtend { value, width } => {
                let argument = self.materialize(function, value)?;
                let to_type = self.int_type(*width, false)?;
                self.insert_cast(function, mir::CastOperator::ZeroExtend, argument, to_type)
            }
            Scev::SignExtend { value, width } => {
                let argument = self.materialize(function, value)?;
                let to_type = self.int_type(*width, true)?;
                self.insert_cast(function, mir::CastOperator::SignExtend, argument, to_type)
            }
            Scev::Truncate { value, width } => {
                let argument = self.materialize(function, value)?;
                let signed = self.truncate_signedness(argument)?;
                let to_type = self.int_type(*width, signed)?;
                self.insert_cast(function, mir::CastOperator::Truncate, argument, to_type)
            }
            Scev::AddRec { .. } => return None,
        };

        // cache the materialized value
        self.scev_cache.push((scev.clone(), value));
        Some(value)
    }

    /// Find a cached value for a SCEV expression.
    fn cached_scev_value(&self, scev: &Scev) -> Option<mir::Value> {
        self.scev_cache
            .iter()
            .find(|(cached, _)| cached == scev)
            .map(|(_, value)| *value)
    }

    /// Check if a SCEV is proven non zero.
    fn scev_non_zero(&self, scev: &Scev) -> bool {
        match scev {
            Scev::Constant(constant) => constant_non_zero(constant).unwrap_or(false),
            Scev::Unknown(value) => self.value_non_zero(*value),
            Scev::Neg(inner) => self.scev_non_zero(inner),
            _ => false,
        }
    }

    /// Check if a value is proven non zero at the preheader.
    fn value_non_zero(&self, value: mir::Value) -> bool {
        let Some(range) = self.range_for_value_at_preheader(value) else {
            return false;
        };

        range_excludes_zero(range)
    }

    /// Get the range for a value at the preheader.
    fn range_for_value_at_preheader(&self, value: mir::Value) -> Option<&ValueRange> {
        let definition = self.definitions.definition_for(value)?;

        let ranges = match definition.kind {
            ValueDefinitionKind::Parameter => self.ranges.entry(self.preheader),
            ValueDefinitionKind::Instruction { .. } => {
                if definition.block == self.preheader {
                    self.ranges.exit(self.preheader)
                } else {
                    self.ranges.entry(self.preheader)
                }
            }
        };

        ranges.get(value)
    }

    /// Materialize a constant in the preheader.
    fn materialize_constant(
        &mut self,
        function: &mut mir::Function,
        constant: &mir::Constant,
    ) -> Option<mir::Value> {
        // reuse existing constants when possible
        if let Some((_, value)) = self
            .constant_cache
            .iter()
            .find(|(cached, _)| cached == constant)
        {
            return Some(*value);
        }

        // resolve the constant type
        let type_id = match constant {
            mir::Constant::Null
            | mir::Constant::Undefined
            | mir::Constant::Uninit
            | mir::Constant::Zeroed => return None,
            mir::Constant::Boolean { .. } => self.tree.boolean_type(),
            mir::Constant::Int {
                width, is_signed, ..
            } => self.int_type(*width, *is_signed)?,
            mir::Constant::UInt { width, .. } => self.int_type(*width, false)?,
            mir::Constant::Float { format, .. } => self.tree.float_type(*format),
            mir::Constant::Char { .. } => self.int_type(32, false)?,
        };

        // allocate a new constant instruction
        let destination = function.next_typed_value(type_id);
        let instruction = mir::Instruction::Const {
            destination,
            value: constant.clone(),
        };
        self.insert_instruction(function, instruction);
        self.constant_cache.push((constant.clone(), destination));

        Some(destination)
    }

    /// Materialize an unknown value if it can be made available in the preheader.
    fn materialize_unknown(
        &mut self,
        function: &mut mir::Function,
        value: mir::Value,
    ) -> Option<mir::Value> {
        // delegate to value materialization
        self.materialize_value(function, value)
    }

    /// Materialize a loop invariant value in the preheader.
    fn materialize_value(
        &mut self,
        function: &mut mir::Function,
        value: mir::Value,
    ) -> Option<mir::Value> {
        // reuse cached materializations
        if let Some(mapped) = self.value_cache.get(&value) {
            return Some(*mapped);
        }

        // accept values already available in the preheader
        if self.value_available_in_preheader(value) {
            self.value_cache.insert(value, value);
            return Some(value);
        }

        // avoid cycles when cloning invariant instructions
        if self.value_in_progress.contains(&value) {
            return None;
        }
        self.value_in_progress.insert(value);

        // require a definition for the value
        let definition = self.definitions.definition_for(value);
        let Some(definition) = definition else {
            self.value_in_progress.remove(&value);
            return None;
        };

        // handle instruction defined values inside the loop
        let new_value = match definition.kind {
            ValueDefinitionKind::Parameter => None,
            ValueDefinitionKind::Instruction { instruction } => {
                if !self.loop_blocks.contains(&definition.block) {
                    None
                } else {
                    let instruction_data = self.tree.get(instruction).clone();
                    if !instruction_is_speculatable(&instruction_data, self.tree) {
                        None
                    } else {
                        self.clone_speculatable_instruction(
                            function,
                            instruction,
                            &instruction_data,
                            value,
                        )
                    }
                }
            }
        };

        // record completion for recursion tracking
        self.value_in_progress.remove(&value);

        // cache successful clones
        if let Some(materialized) = new_value {
            self.value_cache.insert(value, materialized);
        }

        new_value
    }

    /// Clone a speculatable instruction into the preheader.
    fn clone_speculatable_instruction(
        &mut self,
        function: &mut mir::Function,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        original: mir::Value,
    ) -> Option<mir::Value> {
        // allocate a destination for the cloned instruction
        let destination = function.next_typed_value_like(original);

        // map the destination and operands to preheader values
        let mut value_map = HashMap::new();
        value_map.insert(original, destination);

        // materialize inline operands
        for operand in instruction.uses() {
            let mapped = self.materialize_value(function, operand)?;
            value_map.insert(operand, mapped);
        }

        // materialize externalized operands when present
        if let Some(args_slice) = instruction.argument_slice() {
            let arguments = self.tree.get_values(args_slice).to_vec();
            for operand in arguments {
                let mapped = self.materialize_value(function, operand)?;
                value_map.insert(operand, mapped);
            }
        }

        // build the cloned instruction with remapped values
        let cloned = instruction_map(instruction, &value_map, self.tree);

        // insert the cloned instruction in the preheader
        let cloned_id = self.insert_instruction(function, cloned);
        clone_instruction_tables(
            self.tree,
            self.memory,
            instruction_id,
            cloned_id,
            &value_map,
        );

        Some(destination)
    }

    /// Insert a binary instruction into the preheader.
    fn insert_binary(
        &mut self,
        function: &mut mir::Function,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> mir::Value {
        // allocate a destination value
        let destination = function.next_typed_value_like(left);
        let instruction = mir::Instruction::Binary {
            destination,
            operator,
            left,
            right,
        };
        self.insert_instruction(function, instruction);
        destination
    }

    /// Insert a unary instruction into the preheader.
    fn insert_unary(
        &mut self,
        function: &mut mir::Function,
        operator: mir::UnaryOperator,
        argument: mir::Value,
    ) -> mir::Value {
        // allocate a destination value
        let destination = function.next_typed_value_like(argument);
        let instruction = mir::Instruction::Unary {
            destination,
            operator,
            argument,
        };
        self.insert_instruction(function, instruction);
        destination
    }

    /// Insert a cast instruction into the preheader.
    fn insert_cast(
        &mut self,
        function: &mut mir::Function,
        operator: mir::CastOperator,
        argument: mir::Value,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> mir::Value {
        // allocate a destination value
        let destination = function.next_typed_value(to_type);
        let instruction = mir::Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        };
        self.insert_instruction(function, instruction);
        destination
    }

    /// Insert an instruction into the preheader block.
    fn insert_instruction(
        &mut self,
        function: &mut mir::Function,
        instruction: mir::Instruction,
    ) -> mir::LocalNodeId<mir::Instruction> {
        // append the instruction to the preheader
        let instruction_id = self.tree.insert(instruction);
        let mut instructions = self.tree.get(self.preheader).instructions.clone();
        instructions.push(instruction_id);
        function.replace_block_instructions(self.preheader, instructions, self.tree);
        instruction_id
    }

    /// Check if a value is available in the preheader.
    fn value_available_in_preheader(&self, value: mir::Value) -> bool {
        // check definition tables first
        let Some(definition) = self.definitions.definition_for(value) else {
            return false;
        };

        // reject values defined inside the loop
        if self.loop_blocks.contains(&definition.block) {
            return false;
        }

        self.domtree.dominates(definition.block, self.preheader)
    }

    /// Get or create an integer type.
    fn int_type(&mut self, width: u16, signed: bool) -> Option<mir::LocalNodeId<mir::Type>> {
        // reuse cached types when possible
        if let Some(existing) = self.type_cache.get(&(width, signed)) {
            return Some(*existing);
        }

        // allocate a new type node
        let ty = mir::Type::Int {
            width,
            is_signed: signed,
        };
        let type_id = self.tree.intern_type(ty);
        self.type_cache.insert((width, signed), type_id);

        Some(type_id)
    }

    /// Determine signedness for a truncate operation.
    fn truncate_signedness(&self, argument: mir::Value) -> Option<bool> {
        // read the argument type
        let ty_id = self.value_types.expect_value_type(argument);
        let ty = self.tree.get(ty_id);
        let (_, signed) = ty.int_info_with_pointer_width(self.target_layout.pointer_bits())?;
        Some(signed)
    }

    /// Check whether signed division is safe to hoist.
    fn scev_signed_division_safe(&self, left: &Scev, right: &Scev) -> bool {
        if self.scev_excludes_value(right, -1) {
            return true;
        }

        self.scev_excludes_signed_min(left)
    }

    /// Check whether a SCEV excludes a specific integer value.
    fn scev_excludes_value(&self, scev: &Scev, target: i128) -> bool {
        match scev {
            Scev::Constant(constant) => match constant {
                mir::Constant::Int { value, .. } => *value != target,
                mir::Constant::UInt { value, .. } => {
                    let Ok(value) = i128::try_from(*value) else {
                        return false;
                    };

                    value != target
                }
                _ => false,
            },
            Scev::Unknown(value_id) => {
                let Some(range) = self.range_for_value_at_preheader(*value_id) else {
                    return false;
                };

                range_excludes_value(range, target)
            }
            _ => false,
        }
    }

    /// Check whether a SCEV excludes the signed minimum value for its type.
    fn scev_excludes_signed_min(&self, scev: &Scev) -> bool {
        match scev {
            Scev::Constant(constant) => match constant {
                mir::Constant::Int {
                    value,
                    width,
                    is_signed,
                } => {
                    if !*is_signed {
                        return true;
                    }

                    *value != signed_min_value(*width)
                }
                mir::Constant::UInt { .. } => true,
                _ => false,
            },
            Scev::Unknown(value_id) => {
                let Some(range) = self.range_for_value_at_preheader(*value_id) else {
                    return false;
                };

                signed_min_excluded(range)
            }
            _ => false,
        }
    }
}

/// Check if a range excludes zero.
fn range_excludes_zero(range: &ValueRange) -> bool {
    match range {
        ValueRange::Integer { min, max, .. } => *min > 0 || *max < 0,
        ValueRange::Boolean {
            can_be_true,
            can_be_false,
        } => *can_be_true && !*can_be_false,
        _ => false,
    }
}

/// Check if a range excludes a specific integer value.
fn range_excludes_value(range: &ValueRange, value: i128) -> bool {
    match range {
        ValueRange::Integer { min, max, .. } => value < *min || value > *max,
        _ => false,
    }
}

/// Check if a range excludes the signed minimum value.
fn signed_min_excluded(range: &ValueRange) -> bool {
    let ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    } = range
    else {
        return false;
    };

    if !*is_signed {
        return true;
    }

    let min_value = signed_min_value(*width);
    min_value < *min || min_value > *max
}

/// Compute the signed minimum value for a bit width.
fn signed_min_value(width: u16) -> i128 {
    let shift = width.saturating_sub(1);
    -(1_i128 << shift)
}

/// Check if a constant is non zero.
fn constant_non_zero(constant: &mir::Constant) -> Option<bool> {
    match constant {
        mir::Constant::Int { value, .. } => Some(*value != 0),
        mir::Constant::UInt { value, .. } => Some(*value != 0),
        mir::Constant::Boolean { value } => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Loop derived multiplications are rewritten as recurrences.
    #[test]
    fn test_strength_reduce_multiply() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 4
    jump b1(v1)

b1(v4: int32):
    v5: boolean = int.lt.s v4, v0
    branch v5, b2, b3

b2:
    v6: int32 = int.mul v4, v3
    v7: int32 = int.add v6, v2
    v8: int32 = int.add v4, v2
    jump b1(v8)

b3:
    return v4
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 4
    jump b1(v1, v1)

b1(v4: int32, v9: int32):
    v5: boolean = int.lt.s v4, v0
    branch v5, b2, b3

b2:
    v6: int32 = int.mul v4, v3
    v7: int32 = int.add v9, v2
    v8: int32 = int.add v4, v2
    v10: int32 = int.add v9, v3
    jump b1(v8, v10)

b3:
    return v4
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_output(expected);
    }

    /// Loop derived multiplications with invariant factors are rewritten.
    #[test]
    fn test_strength_reduce_invariant_multiplier() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: int32 = 1
    jump b1(v2)

b1(v4: int32):
    v5: boolean = int.lt.s v4, v0
    branch v5, b2, b3

b2:
    v6: int32 = int.mul v4, v1
    v7: int32 = int.add v6, v3
    v8: int32 = int.add v4, v3
    jump b1(v8)

b3:
    return v4
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: int32 = 1
    jump b1(v2, v2)

b1(v4: int32, v9: int32):
    v5: boolean = int.lt.s v4, v0
    branch v5, b2, b3

b2:
    v6: int32 = int.mul v4, v1
    v7: int32 = int.add v9, v3
    v8: int32 = int.add v4, v3
    v10: int32 = int.add v9, v1
    jump b1(v8, v10)

b3:
    return v4
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_output(expected);
    }

    /// Loop invariant selects are materialized in the preheader.
    #[test]
    fn test_strength_reduce_invariant_select_multiplier() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = 0
    v4: int32 = 1
    v5: int32 = 2
    jump b1(v3)

b1(v6: int32):
    v7: boolean = int.lt.s v6, v0
    branch v7, b2, b3

b2:
    v8: int32 = select v2, v1, v5
    v9: int32 = int.mul v6, v8
    v10: int32 = int.add v9, v4
    v11: int32 = int.add v6, v4
    jump b1(v11)

b3:
    return v6
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = 0
    v4: int32 = 1
    v5: int32 = 2
    v12: int32 = select v2, v1, v5
    jump b1(v3, v3)

b1(v6: int32, v13: int32):
    v7: boolean = int.lt.s v6, v0
    branch v7, b2, b3

b2:
    v8: int32 = select v2, v1, v5
    v9: int32 = int.mul v6, v8
    v10: int32 = int.add v13, v4
    v11: int32 = int.add v6, v4
    v14: int32 = int.add v13, v12
    jump b1(v11, v14)

b3:
    return v6
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_output(expected);
    }

    /// Multiple loop derived values are strength reduced.
    #[test]
    fn test_strength_reduce_multiple_candidates() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 2
    v4: int32 = 3
    jump b1(v1)

b1(v5: int32):
    v6: boolean = int.lt.s v5, v0
    branch v6, b2, b3

b2:
    v7: int32 = int.mul v5, v3
    v8: int32 = int.add v7, v4
    v9: int32 = int.add v5, v2
    v10: int32 = int.mul v5, v4
    v11: int32 = int.add v10, v2
    jump b1(v9)

b3:
    return v5
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 2
    v4: int32 = 3
    jump b1(v1, v1, v1)

b1(v5: int32, v12: int32, v14: int32):
    v6: boolean = int.lt.s v5, v0
    branch v6, b2, b3

b2:
    v7: int32 = int.mul v5, v3
    v8: int32 = int.add v12, v4
    v9: int32 = int.add v5, v2
    v10: int32 = int.mul v5, v4
    v11: int32 = int.add v14, v2
    v13: int32 = int.add v12, v3
    v15: int32 = int.add v14, v4
    jump b1(v9, v13, v15)

b3:
    return v5
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_output(expected);
    }

    /// Strength reduction requires a canonical preheader.
    #[test]
    fn test_strength_reduce_requires_preheader() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
entry(v0: int32, v1: int32, v2: int32):
    v3: boolean = int.eq v0, v0
    branch v3, b1, b2

b1:
    jump b3(v1)

b2:
    jump b3(v2)

b3(v4: int32):
    v5: int32 = 1
    v6: boolean = int.lt.s v4, v0
    branch v6, b4, b5

b4:
    v7: int32 = int.mul v4, v5
    v8: int32 = int.add v4, v5
    jump b3(v8)

b5:
    return v4
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_unchanged(input);
    }

    /// Strength reduction updates branch preheaders.
    #[test]
    fn test_strength_reduce_branch_preheader() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 4
    v4: boolean = int.eq v0, v0
    branch v4, b1(v1), b4

b1(v5: int32):
    v6: boolean = int.lt.s v5, v0
    branch v6, b2, b3

b2:
    v7: int32 = int.mul v5, v3
    v8: int32 = int.add v7, v2
    v9: int32 = int.add v5, v2
    jump b1(v9)

b3:
    return v5

b4:
    return v1
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 4
    v4: boolean = int.eq v0, v0
    branch v4, b1(v1, v1), b4

b1(v5: int32, v10: int32):
    v6: boolean = int.lt.s v5, v0
    branch v6, b2, b3

b2:
    v7: int32 = int.mul v5, v3
    v8: int32 = int.add v10, v2
    v9: int32 = int.add v5, v2
    v11: int32 = int.add v10, v3
    jump b1(v9, v11)

b3:
    return v5

b4:
    return v1
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_output(expected);
    }

    /// Strength reduction updates switch preheaders.
    #[test]
    fn test_strength_reduce_switch_preheader() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 4
    v4: int32 = 0
    switch v4, b3, 0 => b1(v1)

b1(v5: int32):
    v6: boolean = int.lt.s v5, v0
    branch v6, b2, b3

b2:
    v7: int32 = int.mul v5, v3
    v8: int32 = int.add v7, v2
    v9: int32 = int.add v5, v2
    jump b1(v9)

b3:
    return v5
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 4
    v4: int32 = 0
    switch v4, b3, 0 => b1(v1, v1)

b1(v5: int32, v10: int32):
    v6: boolean = int.lt.s v5, v0
    branch v6, b2, b3

b2:
    v7: int32 = int.mul v5, v3
    v8: int32 = int.add v10, v2
    v9: int32 = int.add v5, v2
    v11: int32 = int.add v10, v3
    jump b1(v9, v11)

b3:
    return v5
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_output(expected);
    }

    /// Values used outside the loop are still strength reduced.
    #[test]
    fn test_strength_reduce_exit_use() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 4
    jump b1(v1)

b1(v4: int32):
    v5: boolean = int.lt.s v4, v0
    branch v5, b2, b3(v4)

b2:
    v6: int32 = int.mul v4, v3
    v7: int32 = int.add v4, v2
    branch v5, b1(v7), b3(v6)

b3(v8: int32):
    return v8
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    v3: int32 = 4
    jump b1(v1, v1)

b1(v4: int32, v9: int32):
    v5: boolean = int.lt.s v4, v0
    branch v5, b2, b3(v4)

b2:
    v6: int32 = int.mul v4, v3
    v7: int32 = int.add v4, v2
    v10: int32 = int.add v9, v3
    branch v5, b1(v7, v10), b3(v9)

b3(v8: int32):
    return v8
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_output(expected);
    }

    /// Strength reduction updates check backedges.
    #[test]
    fn test_strength_reduce_check_latch() {
        // source test
        let input = r#"
function test(v0: [int32; 8]): void {
entry(v0: [int32; 8]):
    v1: uint32 = 0
    v2: uint32 = 1
    v3: uint32 = 8
    jump b1(v1)

b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2, b4

b2:
    v6: uint32 = int.mul v4, v3
    v7: uint32 = int.add v6, v2
    v8: uint32 = int.add v4, v2
    v9: boolean = int.lt.u v8, v3
    check bounds.u v8, v3, v0 => b1(v8), b3

b3:
    return

b4:
    return
}
"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 8]): void {
entry(v0: [int32; 8]):
    v1: uint32 = 0
    v2: uint32 = 1
    v3: uint32 = 8
    jump b1(v1, v1)

b1(v4: uint32, v10: uint32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2, b4

b2:
    v6: uint32 = int.mul v4, v3
    v7: uint32 = int.add v10, v2
    v8: uint32 = int.add v4, v2
    v9: boolean = int.lt.u v8, v3
    v11: uint32 = int.add v10, v3
    check bounds.u v8, v3, v0 => b1(v8, v11), b3

b3:
    return

b4:
    return
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_output(expected);
    }

    /// Cheap induction adds are not strength reduced.
    #[test]
    fn test_strength_reduce_skip_add() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    v2: int32 = 1
    jump b1(v1)

b1(v3: int32):
    v4: boolean = int.lt.s v3, v0
    branch v4, b2, b3

b2:
    v5: int32 = int.add v3, v2
    v6: int32 = int.add v5, v2
    jump b1(v5)

b3:
    return v3
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_unchanged(input);
    }

    /// Strength reduction skips unsafe invariant divisions.
    #[test]
    fn test_strength_reduce_skip_unsafe_division() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = 0
    v3: int32 = 1
    jump b1(v2)

b1(v4: int32):
    v5: boolean = int.lt.s v4, v0
    branch v5, b2, b3

b2:
    v6: int32 = int.div.s v0, v1
    v7: int32 = int.mul v4, v6
    v8: int32 = int.add v4, v3
    jump b1(v8)

b3:
    return v4
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_unchanged(input);
    }

    /// Strength reduction skips signed division with potential min overflow.
    #[test]
    fn test_strength_reduce_skip_signed_divide_min_overflow() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = -1
    jump b1(v0)

b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2, b3

b2:
    v5: int32 = int.div.s v3, v2
    v6: int32 = int.add v3, v1
    jump b1(v6)

b3:
    return v3
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&ReduceLoopStrength);
        test.assert_unchanged(input);
    }
}
