use std::collections::{HashMap, HashSet, VecDeque};

use crate::declare_pass;
use destack_mir as mir;

use crate::optimize::common::{
    build_use_def_maps, fold_binary, fold_cast, fold_intrinsic, fold_unary,
    instruction_substitute_uses_in_tree, remap_instruction_memory_accesses,
    terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext, TypeContext};

declare_pass! {
    /// Perform sparse conditional constant propagation.
    ///
    /// This pass tracks constant values along executable paths and folds
    /// operations where all operands are constant.
    /// It also uses constant branch conditions to mark unreachable blocks for removal.
    /// Aggregate values and immutable global initializers are propagated when they can be
    /// represented in the lattice.
    ///
    /// ```mir
    /// function before(): int32 {
    /// b0:
    ///     v0 = true
    ///     branch v0, b1, b2
    /// b1:
    ///     v1 = 10int32
    ///     jump b3(v1)
    /// b2:
    ///     v2 = 20int32
    ///     jump b3(v2)
    /// b3(v3: int32):
    ///     v4 = int.add v3, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    /// b0:
    ///     v0 = true
    ///     jump b1
    /// b1:
    ///     v1 = 10int32
    ///     jump b2(v1)
    /// b2(v3: int32):
    ///     v4 = 20int32
    ///     return v4
    /// }
    /// ```
    #[pass(id = "sccp")]
    pub SparseConditionalConstantPropagation,
    "Sparse conditional constant propagation"
}

impl FunctionPass for SparseConditionalConstantPropagation {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // run SCCP
        let (cfg_changed, value_changed) = run_sccp(function, tree, ctx.type_context());

        if cfg_changed || value_changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "SparseConditionalConstantPropagation"
    }

    fn id(&self) -> &'static str {
        "sccp"
    }
}

/// SCCP logic. Returns (cfg_changed, value_changed).
fn run_sccp(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    type_context: TypeContext,
) -> (bool, bool) {
    // skip extern functions
    let entry = match function.entry {
        Some(entry) => entry,
        None => return (false, false),
    };

    // build use def data
    let use_def = build_use_def_maps(function, tree);

    // run sccp analysis
    let mut state = SccpState::new(tree, &use_def.use_blocks, entry, type_context);
    let result = state.run();

    // apply constant folding and reachability
    apply_sccp_result(function, tree, &result)
}

/// Lattice state for SCCP values.
#[derive(Debug, Clone, PartialEq)]
enum LatticeValue {
    /// No information about the value yet.
    Unknown,
    /// A known constant value.
    Constant(mir::Constant),
    /// A known aggregate value with per element lattice values.
    Aggregate(Vec<LatticeValue>),
    /// A value that is known to vary.
    Overdefined,
}

impl LatticeValue {
    /// Merge two lattice values.
    fn meet(&self, other: &Self) -> Self {
        // merge lattice states
        match (self, other) {
            (Self::Overdefined, _) | (_, Self::Overdefined) => Self::Overdefined,
            (Self::Unknown, value) | (value, Self::Unknown) => value.clone(),
            (Self::Constant(left), Self::Constant(right)) if left == right => {
                Self::Constant(left.clone())
            }
            (Self::Constant(_), Self::Constant(_)) => Self::Overdefined,
            (Self::Aggregate(left), Self::Aggregate(right)) => {
                if left.len() != right.len() {
                    return Self::Overdefined;
                }

                let elements = left
                    .iter()
                    .zip(right)
                    .map(|(left, right)| left.meet(right))
                    .collect();
                Self::Aggregate(elements)
            }
            (Self::Aggregate(_), Self::Constant(_)) | (Self::Constant(_), Self::Aggregate(_)) => {
                Self::Overdefined
            }
        }
    }
}

/// An executable control flow edge with its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ExecutableEdge {
    /// The predecessor block.
    pred: mir::LocalNodeId<mir::Block>,
    /// The target block.
    target: mir::LocalNodeId<mir::Block>,
    /// The arguments passed to the target.
    arguments: Vec<mir::ValueReference>,
}

/// Result of the SCCP analysis.
#[derive(Debug)]
struct SccpResult {
    /// Executable blocks discovered by the analysis.
    executable_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Lattice states for SSA values.
    value_states: HashMap<mir::Value, LatticeValue>,
}

impl SccpResult {
    /// Check if a block is executable.
    fn is_executable(&self, block: mir::LocalNodeId<mir::Block>) -> bool {
        self.executable_blocks.contains(&block)
    }

    /// Get the lattice value for a specific SSA value.
    fn value_state(&self, value: mir::Value) -> LatticeValue {
        self.value_states
            .get(&value)
            .cloned()
            .unwrap_or(LatticeValue::Unknown)
    }

    /// Get the constant value for an SSA value if available.
    fn value_constant(&self, value: mir::Value) -> Option<&mir::Constant> {
        // return a constant when the lattice state is constant
        match self.value_states.get(&value) {
            Some(LatticeValue::Constant(constant)) => Some(constant),
            _ => None,
        }
    }
}

/// SCCP analysis state and worklists.
struct SccpState<'a> {
    /// The MIR tree for instruction lookup.
    tree: &'a mir::Tree,
    /// Blocks that use a given value.
    use_blocks: &'a HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>>,
    /// The entry block.
    entry: mir::LocalNodeId<mir::Block>,
    /// Current lattice values for SSA values.
    value_states: HashMap<mir::Value, LatticeValue>,
    /// Blocks marked executable.
    executable_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Executable edges with their arguments.
    executable_edges: HashSet<ExecutableEdge>,
    /// Blocks that use a value as an edge argument.
    edge_use_blocks: HashMap<mir::Value, HashSet<mir::LocalNodeId<mir::Block>>>,
    /// Worklist of blocks to process.
    block_worklist: VecDeque<mir::LocalNodeId<mir::Block>>,
    /// Blocks already in the worklist.
    in_worklist: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Type context for layout sensitive operations.
    type_context: TypeContext,
}

impl<'a> SccpState<'a> {
    /// Create a new SCCP analysis state.
    fn new(
        tree: &'a mir::Tree,
        use_blocks: &'a HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>>,
        entry: mir::LocalNodeId<mir::Block>,
        type_context: TypeContext,
    ) -> Self {
        // initialize the analysis state
        Self {
            tree,
            use_blocks,
            entry,
            value_states: HashMap::new(),
            executable_blocks: HashSet::new(),
            executable_edges: HashSet::new(),
            edge_use_blocks: HashMap::new(),
            block_worklist: VecDeque::new(),
            in_worklist: HashSet::new(),
            type_context,
        }
    }

    /// Run SCCP and return the analysis result.
    fn run(&mut self) -> SccpResult {
        // seed entry block
        self.mark_block_executable(self.entry);

        // seed entry parameters as overdefined
        self.seed_entry_parameters();

        // process blocks to a fixed point
        while let Some(block_id) = self.block_worklist.pop_front() {
            // drop block from worklist set
            self.in_worklist.remove(&block_id);

            // skip non executable blocks
            if !self.executable_blocks.contains(&block_id) {
                continue;
            }

            // process executable blocks
            self.process_block(block_id);
        }

        // build analysis result
        SccpResult {
            executable_blocks: self.executable_blocks.clone(),
            value_states: self.value_states.clone(),
        }
    }

    /// Seed entry block parameters as overdefined values.
    fn seed_entry_parameters(&mut self) {
        // read the entry block
        let block = self.tree.get(self.entry);

        // treat entry parameters as overdefined
        for param in &block.parameters {
            let Some(value) = param.value.value() else {
                continue;
            };

            self.update_value(value, LatticeValue::Overdefined);
        }
    }

    /// Mark a block as executable and enqueue it.
    fn mark_block_executable(&mut self, block: mir::LocalNodeId<mir::Block>) {
        // mark block executable
        self.executable_blocks.insert(block);

        // enqueue block for processing
        self.enqueue_block(block);
    }

    /// Enqueue a block if it is not already in the worklist.
    fn enqueue_block(&mut self, block: mir::LocalNodeId<mir::Block>) {
        // skip duplicates
        if !self.in_worklist.insert(block) {
            return;
        }

        // push work item
        self.block_worklist.push_back(block);
    }

    /// Process a single executable block.
    fn process_block(&mut self, block_id: mir::LocalNodeId<mir::Block>) {
        // update block parameters
        self.process_block_parameters(block_id);

        // evaluate instructions
        self.process_instructions(block_id);

        // evaluate terminator edges
        self.process_terminator(block_id);
    }

    /// Update block parameter values based on executable edges.
    fn process_block_parameters(&mut self, block_id: mir::LocalNodeId<mir::Block>) {
        // read block parameters
        let block = self.tree.get(block_id);

        // skip blocks without parameters
        if block.parameters.is_empty() {
            return;
        }

        // gather executable incoming edges
        let edges: Vec<ExecutableEdge> = self
            .executable_edges
            .iter()
            .filter(|edge| edge.target == block_id)
            .cloned()
            .collect();

        // skip blocks with no incoming edges
        if edges.is_empty() {
            return;
        }

        // reject mismatched argument counts
        let expected = block.parameters.len();
        if edges.iter().any(|edge| edge.arguments.len() != expected) {
            // mark parameters as overdefined
            for param in &block.parameters {
                let Some(value) = param.value.value() else {
                    continue;
                };

                self.update_value(value, LatticeValue::Overdefined);
            }
            return;
        }

        // merge incoming arguments per parameter
        for (index, param) in block.parameters.iter().enumerate() {
            let Some(value) = param.value.value() else {
                continue;
            };

            let mut merged = LatticeValue::Unknown;

            // combine values from each executable edge
            for edge in &edges {
                let Some(arg) = edge.arguments[index].value() else {
                    merged = LatticeValue::Overdefined;
                    break;
                };

                let incoming = self.value_state(arg);
                merged = merged.meet(&incoming);
            }

            // update parameter lattice value
            self.update_value(value, merged);
        }
    }

    /// Evaluate instructions in a block and update value states.
    fn process_instructions(&mut self, block_id: mir::LocalNodeId<mir::Block>) {
        // snapshot instruction ids
        let block = self.tree.get(block_id);
        let instruction_ids: Vec<_> = block.instructions.clone();

        // evaluate in order
        for instruction_id in instruction_ids {
            // read instruction destination
            let instruction = self.tree.get(instruction_id);

            // skip instructions without destinations
            let Some(destination) = instruction
                .destination()
                .and_then(|destination| destination.value())
            else {
                continue;
            };

            // update lattice with instruction result
            let new_state = self.evaluate_instruction(instruction);
            self.update_value(destination, new_state);
        }
    }

    /// Evaluate the terminator and mark executable edges.
    fn process_terminator(&mut self, block_id: mir::LocalNodeId<mir::Block>) {
        // read terminator
        let block = self.tree.get(block_id);
        let terminator = self.tree.get(block.terminator);

        // mark edges based on terminator kind
        match terminator {
            mir::Terminator::Error => {
                panic!("recovered MIR terminator reached optimizer");
            }
            mir::Terminator::Jump { target } => {
                let Some(target_block) = target.block.block() else {
                    return;
                };

                self.mark_edge_executable(block_id, target_block, &target.arguments);
            }
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                // evaluate branch condition
                let condition_state = condition
                    .value()
                    .map(|condition| self.value_state(condition))
                    .unwrap_or(LatticeValue::Overdefined);

                let Some(then_block) = then_target.block.block() else {
                    return;
                };
                let Some(else_block) = else_target.block.block() else {
                    return;
                };

                // mark executable edges for the branch
                if let LatticeValue::Constant(mir::Constant::Boolean { value }) = condition_state {
                    if value {
                        self.mark_edge_executable(block_id, then_block, &then_target.arguments);
                    } else {
                        self.mark_edge_executable(block_id, else_block, &else_target.arguments);
                    }
                } else {
                    self.mark_edge_executable(block_id, then_block, &then_target.arguments);
                    self.mark_edge_executable(block_id, else_block, &else_target.arguments);
                }
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                let Some(success_block) = success.block.block() else {
                    return;
                };
                let Some(failure_block) = failure.block.block() else {
                    return;
                };

                self.mark_edge_executable(block_id, success_block, &success.arguments);
                self.mark_edge_executable(block_id, failure_block, &failure.arguments);
            }
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                // evaluate switch condition
                let value_state = value
                    .value()
                    .map(|value| self.value_state(value))
                    .unwrap_or(LatticeValue::Overdefined);

                let Some(default_block) = default.block.block() else {
                    return;
                };

                // mark executable edges for the switch
                if let LatticeValue::Constant(constant) = value_state {
                    if let Some(value) = switch_constant_value(&constant) {
                        if let Some(target) = select_switch_target(value, cases) {
                            let Some(target_block) = target.target.block.block() else {
                                return;
                            };

                            self.mark_edge_executable(
                                block_id,
                                target_block,
                                &target.target.arguments,
                            );
                        } else {
                            self.mark_edge_executable(block_id, default_block, &default.arguments);
                        }
                    } else {
                        self.mark_edge_executable(block_id, default_block, &default.arguments);
                        for case in cases {
                            let Some(case_block) = case.target.block.block() else {
                                return;
                            };

                            self.mark_edge_executable(block_id, case_block, &case.target.arguments);
                        }
                    }
                } else {
                    self.mark_edge_executable(block_id, default_block, &default.arguments);
                    for case in cases {
                        let Some(case_block) = case.target.block.block() else {
                            return;
                        };

                        self.mark_edge_executable(block_id, case_block, &case.target.arguments);
                    }
                }
            }
            mir::Terminator::Yield { resume, .. } => {
                let Some(resume_block) = resume.block.block() else {
                    return;
                };

                self.mark_edge_executable(block_id, resume_block, &resume.arguments);
            }
            mir::Terminator::Invoke {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeInterface {
                normal_target,
                unwind_target,
                ..
            } => {
                let Some(normal_block) = normal_target.block.block() else {
                    return;
                };
                let Some(unwind_block) = unwind_target.block.block() else {
                    return;
                };

                self.mark_edge_executable(block_id, normal_block, &normal_target.arguments);
                self.mark_edge_executable(block_id, unwind_block, &unwind_target.arguments);
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Throw { .. }
            | mir::Terminator::Trap { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallInterface { .. }
            | mir::Terminator::TailCallIndirect { .. } => {}
        }
    }

    /// Record an executable edge and enqueue its target.
    fn mark_edge_executable(
        &mut self,
        pred: mir::LocalNodeId<mir::Block>,
        target: mir::LocalNodeId<mir::Block>,
        arguments: &[mir::ValueReference],
    ) {
        // record new edge
        let edge = ExecutableEdge {
            pred,
            target,
            arguments: arguments.to_vec(),
        };

        // skip edges already recorded
        if !self.executable_edges.insert(edge) {
            return;
        }

        // record edge argument uses
        for &argument in arguments {
            let Some(argument) = argument.value() else {
                continue;
            };

            self.edge_use_blocks
                .entry(argument)
                .or_default()
                .insert(target);
        }

        // mark target executable and enqueue
        self.executable_blocks.insert(target);
        self.enqueue_block(target);
    }

    /// Update a value state and enqueue uses when it changes.
    fn update_value(&mut self, value: mir::Value, new_state: LatticeValue) {
        // merge with existing state
        let old_state = self.value_state(value);
        let merged = old_state.meet(&new_state);

        // skip unchanged values
        if merged == old_state {
            return;
        }

        // store updated lattice
        self.value_states.insert(value, merged);

        // enqueue blocks that use this value
        if let Some(blocks) = self.use_blocks.get(&value) {
            for &block_id in blocks {
                self.enqueue_block(block_id);
            }
        }

        // enqueue blocks that use this value in edge arguments
        if let Some(blocks) = self.edge_use_blocks.get(&value) {
            // snapshot blocks to avoid aliasing the map
            let blocks: Vec<_> = blocks.iter().copied().collect();
            for block_id in blocks {
                self.enqueue_block(block_id);
            }
        }
    }

    /// Get the lattice state for a value.
    fn value_state(&self, value: mir::Value) -> LatticeValue {
        // lookup lattice state
        self.value_states
            .get(&value)
            .cloned()
            .unwrap_or(LatticeValue::Unknown)
    }

    /// Evaluate an instruction into a lattice value.
    fn evaluate_instruction(&self, instruction: &mir::Instruction) -> LatticeValue {
        // fold instructions that yield constants
        match instruction {
            mir::Instruction::Const { value, .. } => LatticeValue::Constant(value.clone()),
            mir::Instruction::Binary {
                operator,
                left,
                right,
                ..
            } => {
                // read operand lattice states
                let Some(left) = left.value() else {
                    return LatticeValue::Overdefined;
                };
                let Some(right) = right.value() else {
                    return LatticeValue::Overdefined;
                };

                let left_state = self.value_state(left);
                let right_state = self.value_state(right);

                // fold based on operand states
                match (left_state, right_state) {
                    (LatticeValue::Constant(left), LatticeValue::Constant(right)) => {
                        fold_binary(*operator, left, right)
                            .map(LatticeValue::Constant)
                            .unwrap_or(LatticeValue::Overdefined)
                    }
                    (LatticeValue::Overdefined, _) | (_, LatticeValue::Overdefined) => {
                        LatticeValue::Overdefined
                    }
                    (LatticeValue::Aggregate(_), _) | (_, LatticeValue::Aggregate(_)) => {
                        LatticeValue::Overdefined
                    }
                    _ => LatticeValue::Unknown,
                }
            }
            mir::Instruction::Unary {
                operator, argument, ..
            } => {
                // read operand lattice state
                let Some(argument) = argument.value() else {
                    return LatticeValue::Overdefined;
                };

                let argument_state = self.value_state(argument);

                // fold based on operand state
                match argument_state {
                    LatticeValue::Constant(value) => fold_unary(*operator, value)
                        .map(LatticeValue::Constant)
                        .unwrap_or(LatticeValue::Overdefined),
                    LatticeValue::Overdefined => LatticeValue::Overdefined,
                    LatticeValue::Aggregate(_) => LatticeValue::Overdefined,
                    LatticeValue::Unknown => LatticeValue::Unknown,
                }
            }
            mir::Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => {
                // read operand lattice state
                let Some(argument) = argument.value() else {
                    return LatticeValue::Overdefined;
                };
                let Some(to_type) = to_type.ty() else {
                    return LatticeValue::Overdefined;
                };

                let argument_state = self.value_state(argument);

                // fold based on operand state
                match argument_state {
                    LatticeValue::Constant(value) => fold_cast(
                        *operator,
                        value,
                        to_type,
                        self.type_context.pointer_width_bits,
                        self.tree,
                    )
                    .map(LatticeValue::Constant)
                    .unwrap_or(LatticeValue::Overdefined),
                    LatticeValue::Overdefined => LatticeValue::Overdefined,
                    LatticeValue::Aggregate(_) => LatticeValue::Overdefined,
                    LatticeValue::Unknown => LatticeValue::Unknown,
                }
            }
            mir::Instruction::Select {
                condition,
                then_value,
                else_value,
                ..
            } => {
                // evaluate select using condition when possible
                let Some(condition) = condition.value() else {
                    return LatticeValue::Overdefined;
                };
                let Some(then_value) = then_value.value() else {
                    return LatticeValue::Overdefined;
                };
                let Some(else_value) = else_value.value() else {
                    return LatticeValue::Overdefined;
                };

                let condition_state = self.value_state(condition);
                let then_state = self.value_state(then_value);
                let else_state = self.value_state(else_value);

                match condition_state {
                    LatticeValue::Constant(mir::Constant::Boolean { value }) => {
                        if value {
                            then_state
                        } else {
                            else_state
                        }
                    }
                    LatticeValue::Unknown => match (then_state, else_state) {
                        (LatticeValue::Constant(left), LatticeValue::Constant(right))
                            if left == right =>
                        {
                            LatticeValue::Constant(left)
                        }
                        (LatticeValue::Overdefined, _)
                        | (_, LatticeValue::Overdefined)
                        | (LatticeValue::Aggregate(_), _)
                        | (_, LatticeValue::Aggregate(_)) => LatticeValue::Overdefined,
                        _ => LatticeValue::Unknown,
                    },
                    LatticeValue::Overdefined => LatticeValue::Overdefined,
                    LatticeValue::Aggregate(_) => LatticeValue::Overdefined,
                    _ => LatticeValue::Unknown,
                }
            }
            mir::Instruction::Struct { fields, .. } => {
                // evaluate aggregate fields
                let arguments = self.tree.get_arguments(*fields);
                self.evaluate_aggregate(arguments)
            }
            mir::Instruction::Tuple { elements, .. } | mir::Instruction::Array { elements, .. } => {
                // evaluate aggregate elements
                let arguments = self.tree.get_arguments(*elements);
                self.evaluate_aggregate(arguments)
            }
            mir::Instruction::FieldGet {
                aggregate, index, ..
            } => {
                // evaluate field get from aggregates
                let Some(aggregate) = aggregate.value() else {
                    return LatticeValue::Overdefined;
                };

                let aggregate_state = self.value_state(aggregate);
                self.evaluate_field_get(aggregate_state, *index as usize)
            }
            mir::Instruction::FieldSet {
                aggregate,
                index,
                value,
                ..
            } => {
                // evaluate field set on aggregates
                let Some(aggregate) = aggregate.value() else {
                    return LatticeValue::Overdefined;
                };
                let Some(value) = value.value() else {
                    return LatticeValue::Overdefined;
                };

                let aggregate_state = self.value_state(aggregate);
                let value_state = self.value_state(value);
                self.evaluate_field_set(aggregate_state, *index as usize, value_state)
            }
            mir::Instruction::ElementGet { array, index, .. } => {
                // evaluate element get from arrays
                let Some(array) = array.value() else {
                    return LatticeValue::Overdefined;
                };

                let array_state = self.value_state(array);
                self.evaluate_element_get(array_state, *index as usize)
            }
            mir::Instruction::ElementSet {
                array,
                index,
                value,
                ..
            } => {
                // evaluate element set on arrays
                let Some(array) = array.value() else {
                    return LatticeValue::Overdefined;
                };
                let Some(value) = value.value() else {
                    return LatticeValue::Overdefined;
                };

                let array_state = self.value_state(array);
                let value_state = self.value_state(value);
                self.evaluate_element_set(array_state, *index as usize, value_state)
            }
            mir::Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => {
                // fold pure intrinsics with constant arguments
                if !intrinsic.is_pure() {
                    return LatticeValue::Overdefined;
                }

                let mut constants = Vec::new();
                for &argument in self.tree.get_arguments(*arguments) {
                    let Some(argument) = argument.value() else {
                        return LatticeValue::Overdefined;
                    };

                    let argument_state = self.value_state(argument);
                    match argument_state {
                        LatticeValue::Constant(constant) => constants.push(constant),
                        LatticeValue::Overdefined => return LatticeValue::Overdefined,
                        LatticeValue::Aggregate(_) => return LatticeValue::Overdefined,
                        LatticeValue::Unknown => return LatticeValue::Unknown,
                    }
                }

                if let Some(result) = fold_intrinsic(*intrinsic, &constants) {
                    LatticeValue::Constant(result)
                } else {
                    LatticeValue::Overdefined
                }
            }
            _ => LatticeValue::Overdefined,
        }
    }

    /// Build an aggregate lattice value from operand values.
    fn evaluate_aggregate(&self, values: &[mir::ValueReference]) -> LatticeValue {
        // collect operand lattice values
        let elements = values
            .iter()
            .map(|value| {
                value
                    .value()
                    .map(|value| self.value_state(value))
                    .unwrap_or(LatticeValue::Overdefined)
            })
            .collect();
        LatticeValue::Aggregate(elements)
    }

    /// Evaluate a field.get on an aggregate lattice value.
    fn evaluate_field_get(&self, aggregate_state: LatticeValue, index: usize) -> LatticeValue {
        // extract element from aggregate state
        match aggregate_state {
            LatticeValue::Aggregate(elements) => elements
                .get(index)
                .cloned()
                .unwrap_or(LatticeValue::Overdefined),
            LatticeValue::Unknown => LatticeValue::Unknown,
            LatticeValue::Overdefined => LatticeValue::Overdefined,
            LatticeValue::Constant(_) => LatticeValue::Overdefined,
        }
    }

    /// Evaluate a field.set on an aggregate lattice value.
    fn evaluate_field_set(
        &self,
        aggregate_state: LatticeValue,
        index: usize,
        value_state: LatticeValue,
    ) -> LatticeValue {
        // update element when aggregate shape is known
        match aggregate_state {
            LatticeValue::Aggregate(mut elements) => {
                if index >= elements.len() {
                    return LatticeValue::Overdefined;
                }

                elements[index] = value_state;
                LatticeValue::Aggregate(elements)
            }
            LatticeValue::Unknown => LatticeValue::Unknown,
            LatticeValue::Overdefined => LatticeValue::Overdefined,
            LatticeValue::Constant(_) => LatticeValue::Overdefined,
        }
    }

    /// Evaluate an element.get on an array lattice value.
    fn evaluate_element_get(&self, array_state: LatticeValue, index: usize) -> LatticeValue {
        // extract element from array state
        match array_state {
            LatticeValue::Aggregate(elements) => elements
                .get(index)
                .cloned()
                .unwrap_or(LatticeValue::Overdefined),
            LatticeValue::Unknown => LatticeValue::Unknown,
            LatticeValue::Overdefined => LatticeValue::Overdefined,
            LatticeValue::Constant(_) => LatticeValue::Overdefined,
        }
    }

    /// Evaluate an element.set on an array lattice value.
    fn evaluate_element_set(
        &self,
        array_state: LatticeValue,
        index: usize,
        value_state: LatticeValue,
    ) -> LatticeValue {
        // update element when array shape is known
        match array_state {
            LatticeValue::Aggregate(mut elements) => {
                if index >= elements.len() {
                    return LatticeValue::Overdefined;
                }

                elements[index] = value_state;
                LatticeValue::Aggregate(elements)
            }
            LatticeValue::Unknown => LatticeValue::Unknown,
            LatticeValue::Overdefined => LatticeValue::Overdefined,
            LatticeValue::Constant(_) => LatticeValue::Overdefined,
        }
    }
}

/// Convert a constant to a switch value when possible.
fn switch_constant_value(constant: &mir::Constant) -> Option<i128> {
    // convert numeric constants to switch values
    match constant {
        mir::Constant::Int { value, .. } => Some(*value),
        mir::Constant::UInt { value, .. } => i128::try_from(*value).ok(),
        _ => None,
    }
}

/// Select the switch case that matches a constant value.
fn select_switch_target(value: i128, cases: &[mir::SwitchCase]) -> Option<&mir::SwitchCase> {
    // find matching case
    cases
        .iter()
        .find(|case| case.value.integer() == Some(value))
}

/// Apply SCCP results to the function and tree.
fn apply_sccp_result(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    result: &SccpResult,
) -> (bool, bool) {
    // track cfg and value changes
    let mut cfg_changed = false;
    let mut value_changed = false;

    // ensure next value id is fresh before inserting new values
    function.recompute_next_value_id(tree);

    // insert consts for constant block params and build substitutions
    let mut substitutions = HashMap::new();
    value_changed |=
        function_insert_block_param_constants(function, tree, result, &mut substitutions);

    // fold instructions and terminators in executable blocks
    let block_ids: Vec<_> = function.blocks.clone();
    for block_id in block_ids {
        // skip non executable blocks
        if !result.is_executable(block_id) {
            continue;
        }

        // snapshot instruction ids and terminator
        let (instruction_ids, terminator) = {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator).clone();
            (block.instructions.clone(), terminator)
        };

        // fold instruction results
        for instruction_id in instruction_ids {
            // read instruction destination
            let instruction = tree.get(instruction_id);
            let Some(destination) = instruction.destination().and_then(|value| value.value())
            else {
                continue;
            };

            // lookup constant value
            let constant = match result.value_constant(destination) {
                Some(constant) => constant.clone(),
                None => continue,
            };

            // skip existing const instructions
            if matches!(
                instruction,
                mir::Instruction::Const { value, .. } if *value == constant
            ) {
                continue;
            }

            // replace instruction with constant
            let new_instruction = mir::Instruction::Const {
                destination: destination.into(),
                value: constant,
            };
            tree.replace(instruction_id, new_instruction);
            value_changed = true;
        }

        // fold constant branches and switches
        if let Some(new_terminator) = fold_constant_terminator(&terminator, result)
            && new_terminator != terminator
        {
            let terminator_id = tree.get(block_id).terminator;
            tree.replace(terminator_id, new_terminator);
            cfg_changed = true;
        }
    }

    // substitute constant uses after folding
    if !substitutions.is_empty() {
        value_changed |= function_substitute_constant_uses(function, tree, &substitutions);
    }

    // remove unreachable blocks
    let original_len = function.blocks.len();

    // retain only executable blocks
    function
        .blocks
        .retain(|block_id| result.is_executable(*block_id));

    // record cfg changes when blocks are removed
    if function.blocks.len() != original_len {
        cfg_changed = true;
    }

    (cfg_changed, value_changed)
}

/// Insert constants for block parameters and populate substitutions.
fn function_insert_block_param_constants(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    result: &SccpResult,
    substitutions: &mut HashMap<mir::Value, mir::Value>,
) -> bool {
    // track whether any updates occurred
    let mut changed = false;

    // scan blocks for executable constants
    let block_ids: Vec<_> = function.blocks.clone();
    for block_id in block_ids {
        // skip non executable blocks
        if !result.is_executable(block_id) {
            continue;
        }

        // snapshot block parameters
        let params = tree.get(block_id).parameters.clone();

        // skip blocks without parameters
        if params.is_empty() {
            continue;
        }

        // build consts to insert at block entry
        let mut inserted_constants: Vec<(mir::Constant, mir::LocalNodeId<mir::Type>, mir::Value)> =
            Vec::new();
        let mut new_instructions = Vec::new();

        // scan parameters for constant values
        for param in &params {
            // skip non constant parameters
            let Some(value) = param.value.value() else {
                continue;
            };
            let Some(ty) = param.ty.ty() else {
                continue;
            };
            let Some(constant) = result.value_constant(value) else {
                continue;
            };

            // reuse an existing constant instruction when possible
            let existing = inserted_constants
                .iter()
                .find(|(value, current_ty, _)| value == constant && *current_ty == ty)
                .map(|(_, _, value)| *value);

            // insert a new constant when needed
            let const_value = if let Some(value) = existing {
                value
            } else {
                let new_value = function.next_typed_value(ty);
                let instruction = mir::Instruction::Const {
                    destination: new_value.into(),
                    value: constant.clone(),
                };
                let instruction_id = tree.insert(instruction);
                inserted_constants.push((constant.clone(), ty, new_value));
                new_instructions.push(instruction_id);
                new_value
            };

            substitutions.insert(value, const_value);
        }

        // insert constants at block entry when needed
        if !new_instructions.is_empty() {
            // insert consts at block entry
            let block = tree.get_mut(block_id);
            let mut updated = new_instructions;
            updated.extend(block.instructions.iter().copied());
            block.instructions = updated;
            changed = true;
        }
    }

    changed
}

/// Substitute constant values in instruction and terminator uses.
fn function_substitute_constant_uses(
    function: &mir::Function,
    tree: &mut mir::Tree,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> bool {
    // track whether any substitutions occur
    let mut changed = false;

    // rewrite constants in every block
    for &block_id in &function.blocks {
        // snapshot instructions and terminator
        let (instruction_ids, terminator_id, terminator) = {
            let block = tree.get(block_id);
            let terminator_id = block.terminator;
            let terminator = tree.get(terminator_id).clone();
            (block.instructions.clone(), terminator_id, terminator)
        };

        // rewrite instruction uses
        for instruction_id in instruction_ids {
            let instruction = tree.get(instruction_id).clone();

            // skip instructions without substituted operands
            if !instruction_needs_substitution(&instruction, tree, substitutions) {
                continue;
            }

            let new_instruction =
                instruction_substitute_uses_in_tree(&instruction, substitutions, tree);

            // update instruction when rewritten
            if new_instruction != instruction {
                tree.replace(instruction_id, new_instruction);
                remap_instruction_memory_accesses(tree, instruction_id, substitutions);
                changed = true;
            }
        }

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(&terminator, substitutions);

        // update terminator when rewritten
        if new_terminator != terminator {
            tree.replace(terminator_id, new_terminator);
            changed = true;
        }
    }

    changed
}

/// Check if an instruction uses any substituted values.
fn instruction_needs_substitution(
    instruction: &mir::Instruction,
    tree: &mir::Tree,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> bool {
    // check inline operands
    if instruction.uses().iter().any(|value| {
        value
            .value()
            .is_some_and(|value| substitutions.contains_key(&value))
    }) {
        return true;
    }

    // check externalized arguments
    if let Some(arguments) = instruction.argument_slice() {
        return tree.get_arguments(arguments).iter().any(|value| {
            value
                .value()
                .is_some_and(|value| substitutions.contains_key(&value))
        });
    }

    false
}

/// Fold a terminator when its condition is constant.
fn fold_constant_terminator(
    terminator: &mir::Terminator,
    result: &SccpResult,
) -> Option<mir::Terminator> {
    // fold conditional branches
    if let mir::Terminator::Branch {
        condition,
        then_target,
        else_target,
    } = terminator
    {
        // resolve branch condition
        let condition_state = condition
            .value()
            .map(|value| result.value_state(value))
            .unwrap_or(LatticeValue::Overdefined);
        let is_true = match condition_state {
            LatticeValue::Constant(mir::Constant::Boolean { value }) => Some(value),
            _ => None,
        };

        // replace branch with jump when constant
        if let Some(is_true) = is_true {
            let target = if is_true {
                then_target.clone()
            } else {
                else_target.clone()
            };
            return Some(mir::Terminator::Jump { target });
        }
    }

    // fold switch on constant value
    if let mir::Terminator::Switch {
        value,
        default,
        cases,
    } = terminator
    {
        // resolve switch condition
        let constant = value.value().and_then(|value| result.value_constant(value));
        if let Some(constant) = constant {
            // resolve switch constant
            let value = switch_constant_value(constant)?;

            // jump to matching case or default
            if let Some(case) = select_switch_target(value, cases) {
                return Some(mir::Terminator::Jump {
                    target: case.target.clone(),
                });
            }

            return Some(mir::Terminator::Jump {
                target: default.clone(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Constant branch collapses and propagates constants to block parameters.
    #[test]
    fn test_constant_branch_propagates_block_param() {
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = true
    branch v0, b1, b2
b1:
    v1: int32 = 10int32
    jump b3(v1)
b2:
    v2: int32 = 20int32
    jump b3(v2)
b3(v3: int32):
    v4: int32 = int.add v3, v3
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = true
    jump b1
b1:
    v1: int32 = 10int32
    jump b2(v1)
b2(v2: int32):
    v3: int32 = 10int32
    v4: int32 = 20int32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Identical constants on all edges keep block parameters constant.
    #[test]
    fn test_branch_with_same_constants_keeps_param_constant() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 3int32
    jump b3(v1)
b2:
    v2: int32 = 3int32
    jump b3(v2)
b3(v3: int32):
    v4: int32 = int.add v3, v3
    return v4
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 3int32
    jump b3(v1)
b2:
    v2: int32 = 3int32
    jump b3(v2)
b3(v3: int32):
    v4: int32 = 3int32
    v5: int32 = 6int32
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Conflicting constants on different edges do not fold block parameters.
    #[test]
    fn test_branch_with_conflicting_constants_keeps_param_overdefined() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 3int32
    jump b3(v1)
b2:
    v2: int32 = 4int32
    jump b3(v2)
b3(v3: int32):
    v4: int32 = int.add v3, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_unchanged(input);
    }

    /// Multiple edges to the same target with different arguments are overdefined.
    #[test]
    fn test_conflicting_same_target_arguments() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1(v1), b1(v2)
b1(v3: int32):
    v4: int32 = int.add v3, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_unchanged(input);
    }

    /// Switch on a constant value folds to the matching target.
    #[test]
    fn test_switch_on_constant_value() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 2int32
    switch v0, b3, 1 => b1, 2 => b2
b1:
    v1: int32 = 10int32
    return v1
b2:
    v2: int32 = 20int32
    return v2
b3:
    v3: int32 = 30int32
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 2int32
    jump b1
b1:
    v1: int32 = 20int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Readonly global loads are not scalar constants.
    #[test]
    fn test_readonly_global_load_not_constant() {
        let input = r#"
global flag: boolean, readonly = true
function test(): int32 {
b0:
    v0: ref<boolean, raw, readonly> = global.address flag
    v1: boolean = load v0
    branch v1, b1, b2
b1:
    v2: int32 = 1int32
    return v2
b2:
    v3: int32 = 2int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_unchanged(input);
    }

    /// Mutable globals are not treated as constants.
    #[test]
    fn test_mutable_global_not_constant() {
        let input = r#"
global flag: boolean = true
function test(): int32 {
b0:
    v0: ref<boolean, raw> = global.address flag
    v1: boolean = load v0
    branch v1, b1, b2
b1:
    v2: int32 = 1int32
    return v2
b2:
    v3: int32 = 2int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_unchanged(input);
    }

    /// Block parameter constants are substituted in uses.
    #[test]
    fn test_substitute_block_param_uses() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 3int32
    jump b1(v0)
b1(v1: int32):
    jump b2(v1)
b2(v2: int32):
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 3int32
    jump b1(v0)
b1(v1: int32):
    v2: int32 = 3int32
    jump b2(v2)
b2(v3: int32):
    v4: int32 = 3int32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Call arguments are substituted when constants are available.
    #[test]
    fn test_substitute_call_arguments() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(): int32 {
b0:
    v0: int32 = 5int32
    jump b1(v0)
b1(v1: int32):
    v2: int32 = call callee(v1): (int32) -> int32
    return v2
}"#;
        let expected = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(): int32 {
b0:
    v0: int32 = 5int32
    jump b1(v0)
b1(v1: int32):
    v2: int32 = 5int32
    v3: int32 = call callee(v2): (int32) -> int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Switch constants compare by integer value without narrowing.
    #[test]
    fn test_switch_uint64_does_not_match_negative_case() {
        let input = r#"
function test(): int32 {
b0:
    v0: uint64 = 18446744073709551615uint64
    switch v0, b2, -1 => b1
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: uint64 = 18446744073709551615uint64
    jump b2
b2:
    v1: int32 = 2int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Field access folds when a struct has constant fields.
    #[test]
    fn test_struct_field_get_constant() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 5int32
    v1: int32 = 7int32
    v2: { int32, int32 } = struct { int32, int32 } (v0, v1)
    v3: int32 = field.get v2, 0
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 5int32
    v1: int32 = 7int32
    v2: { int32, int32 } = struct { int32, int32 } (v0, v1)
    v3: int32 = 5int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Field access keeps constant elements even when other fields vary.
    #[test]
    fn test_struct_field_get_partial_constant() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 4int32
    v2: { int32, int32 } = struct { int32, int32 } (v1, v0)
    v3: int32 = field.get v2, 0
    return v3
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 4int32
    v2: { int32, int32 } = struct { int32, int32 } (v1, v0)
    v3: int32 = 4int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Field set updates aggregate constants for later field access.
    #[test]
    fn test_struct_field_set_constant() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: { int32, int32 } = struct { int32, int32 } (v0, v1)
    v3: int32 = 9int32
    v4: { int32, int32 } = field.set v2, 1, v3
    v5: int32 = field.get v4, 1
    return v5
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: { int32, int32 } = struct { int32, int32 } (v0, v1)
    v3: int32 = 9int32
    v4: { int32, int32 } = field.set v2, 1, v3
    v5: int32 = 9int32
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Element access folds for constant array indices.
    #[test]
    fn test_array_element_get_constant_index() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: int32 = 30int32
    v3: int32[3] = array int32[3] (v0, v1, v2)
    v4: int64 = 1int64
    v5: int32 = element.get v3, v4
    return v5
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 20int32
    v2: int32 = 30int32
    v3: int32[3] = array int32[3] (v0, v1, v2)
    v4: int64 = 1int64
    v5: int32 = 20int32
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Element set updates array constants for later element access.
    #[test]
    fn test_array_element_set_constant_index() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32[3] = array int32[3] (v0, v1, v2)
    v4: int64 = 1int64
    v5: int32 = 9int32
    v6: int32[3] = element.set v3, v4, v5
    v7: int32 = element.get v6, v4
    return v7
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    v3: int32[3] = array int32[3] (v0, v1, v2)
    v4: int64 = 1int64
    v5: int32 = 9int32
    v6: int32[3] = element.set v3, v4, v5
    v7: int32 = 9int32
    return v7
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Readonly global aggregate loads are not aggregate constants.
    #[test]
    fn test_readonly_global_struct_field_get_not_constant() {
        let input = r#"
global pair: { int32, int32 }, readonly = {1int32, 2int32}
function test(): int32 {
b0:
    v0: ref<{ int32, int32 }, raw, readonly> = global.address pair
    v1: { int32, int32 } = load v0
    v2: int32 = field.get v1, 1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_unchanged(input);
    }

    /// Zero initializer loads are not aggregate constants.
    #[test]
    fn test_global_zero_initializer_field_get_not_constant() {
        let input = r#"
global pair: (int32, int32), readonly = zeroInit
function test(): int32 {
b0:
    v0: ref<(int32, int32), raw, readonly> = global.address pair
    v1: (int32, int32) = load v0
    v2: int32 = field.get v1, 0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_unchanged(input);
    }

    /// Byte initializer loads are not array constants.
    #[test]
    fn test_global_bytes_element_get_not_constant() {
        let input = r#"
global data: uint8[4], readonly = b"test"
function test(): uint8 {
b0:
    v0: ref<uint8[4], raw, readonly> = global.address data
    v1: uint8[4] = load v0
    v2: int64 = 2int64
    v3: uint8 = element.get v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_unchanged(input);
    }

    /// Constant selects are folded to the chosen value.
    #[test]
    fn test_fold_select_constant_condition() {
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 10int32
    v2: int32 = 20int32
    v3: int32 = select v0, v1, v2
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 10int32
    v2: int32 = 20int32
    v3: int32 = 10int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }

    /// Pure intrinsics with constant operands are folded.
    #[test]
    fn test_fold_intrinsic_constant() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 8int32
    v1: int32 = intrinsic.leadingZeroCount(v0)
    return v1
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 8int32
    v1: int32 = 28int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SparseConditionalConstantPropagation);
        test.assert_output(expected);
    }
}
