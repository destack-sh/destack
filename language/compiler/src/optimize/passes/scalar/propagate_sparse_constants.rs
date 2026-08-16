use std::collections::{HashMap, HashSet, VecDeque};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    Mutation, TargetLayout, UseTable, fold_binary, fold_cast, fold_intrinsic, fold_unary,
    instruction_substitute_uses_in_tree, remap_instruction_memory_accesses,
    terminator_substitute_uses,
};

declare_pass! {
    /// Perform sparse conditional constant propagation.
    ///
    /// ```mir
    /// function before(): int32 {
    /// b0:
    ///     v0: boolean = true
    ///     branch v0 => b1 | b2
    /// b1:
    ///     v1: int32 = 10
    ///     jump b3(v1)
    /// b2:
    ///     v2: int32 = 20
    ///     jump b3(v2)
    /// b3(v3: int32):
    ///     v4: int32 = add v3, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    /// b0:
    ///     v0: boolean = true
    ///     jump b1
    /// b1:
    ///     v1: int32 = 10
    ///     jump b2(v1)
    /// b2(v3: int32):
    ///     v4: int32 = 20
    ///     return v4
    /// }
    /// ```
    #[pass(id = "propagate-sparse-constants")]
    pub PropagateSparseConstants,
    "Sparse conditional constant propagation"
}

impl FunctionPass for PropagateSparseConstants {
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        _analyses: &mut mir::FunctionCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let accesses = &mut optimized.accesses;

        // run SCCP
        let (cfg_changed, value_changed) =
            run_propagate_sparse_constants(function, tree, accesses, ctx.target_layout());

        if cfg_changed || value_changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// SCCP logic. Returns (cfg_changed, value_changed).
fn run_propagate_sparse_constants(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    target_layout: TargetLayout,
) -> (bool, bool) {
    // skip external functions
    let entry = match function.entry() {
        Some(entry) => entry,
        None => return (false, false),
    };

    // snapshot value uses
    let uses = UseTable::build(function, tree);

    // run sparse conditional constant propagation analysis
    let mut state = PropagateSparseConstantsState::new(tree, &uses, entry, target_layout);
    let result = state.run();

    // apply constant folding and reachability
    apply_propagate_sparse_constants_result(function, tree, accesses, &result)
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
    arguments: Vec<mir::Value>,
}

/// Result of the SCCP analysis.
#[derive(Debug)]
struct PropagateSparseConstantsResult {
    /// Executable blocks discovered by the analysis.
    executable_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Lattice states for SSA values.
    value_states: HashMap<mir::Value, LatticeValue>,
}

impl PropagateSparseConstantsResult {
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
struct PropagateSparseConstantsState<'a> {
    /// The MIR tree for instruction lookup.
    tree: &'a mir::Tree,
    /// Operand uses for SSA values.
    uses: &'a UseTable,
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
    target_layout: TargetLayout,
}

impl<'a> PropagateSparseConstantsState<'a> {
    /// Create a new SCCP analysis state.
    fn new(
        tree: &'a mir::Tree,
        uses: &'a UseTable,
        entry: mir::LocalNodeId<mir::Block>,
        target_layout: TargetLayout,
    ) -> Self {
        // initialize the analysis state
        Self {
            tree,
            uses,
            entry,
            value_states: HashMap::new(),
            executable_blocks: HashSet::new(),
            executable_edges: HashSet::new(),
            edge_use_blocks: HashMap::new(),
            block_worklist: VecDeque::new(),
            in_worklist: HashSet::new(),
            target_layout,
        }
    }

    /// Run SCCP and return the analysis result.
    fn run(&mut self) -> PropagateSparseConstantsResult {
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
        PropagateSparseConstantsResult {
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
            let value = param.value;

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
                let value = param.value;

                self.update_value(value, LatticeValue::Overdefined);
            }
            return;
        }

        // merge incoming arguments per parameter
        for (index, param) in block.parameters.iter().enumerate() {
            let value = param.value;

            let mut merged = LatticeValue::Unknown;

            // combine values from each executable edge
            for edge in &edges {
                let incoming = self.value_state(edge.arguments[index]);
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
            let Some(destination) = instruction.destination() else {
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
                panic!("invalid MIR terminator reached optimizer");
            }
            mir::Terminator::Jump { target } => {
                let target_block = target.block;
                let arguments = target.arguments(self.tree);

                self.mark_edge_executable(block_id, target_block, arguments);
            }
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                // evaluate branch condition
                let condition_state = self.value_state(*condition);

                let then_block = then_target.block;
                let else_block = else_target.block;

                // mark executable edges for the branch
                if let LatticeValue::Constant(mir::Constant::Boolean { value }) = condition_state {
                    if value {
                        let arguments = then_target.arguments(self.tree);
                        self.mark_edge_executable(block_id, then_block, arguments);
                    } else {
                        let arguments = else_target.arguments(self.tree);
                        self.mark_edge_executable(block_id, else_block, arguments);
                    }
                } else {
                    let then_arguments = then_target.arguments(self.tree);
                    self.mark_edge_executable(block_id, then_block, then_arguments);
                    let else_arguments = else_target.arguments(self.tree);
                    self.mark_edge_executable(block_id, else_block, else_arguments);
                }
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                let success_block = success.block;
                let failure_block = failure.block;

                let success_arguments = success.arguments(self.tree);
                self.mark_edge_executable(block_id, success_block, success_arguments);
                let failure_arguments = failure.arguments(self.tree);
                self.mark_edge_executable(block_id, failure_block, failure_arguments);
            }
            mir::Terminator::NewZeroedTry {
                success, failure, ..
            }
            | mir::Terminator::NewUninitTry {
                success, failure, ..
            } => {
                let success_block = success.block;
                let failure_block = failure.block;

                let success_arguments = success.arguments(self.tree);
                self.mark_edge_executable(block_id, success_block, success_arguments);
                let failure_arguments = failure.arguments(self.tree);
                self.mark_edge_executable(block_id, failure_block, failure_arguments);
            }
            mir::Terminator::NewSliceZeroedTry {
                length,
                success,
                failure,
                ..
            }
            | mir::Terminator::NewSliceUninitTry {
                length,
                success,
                failure,
                ..
            } => {
                let success_block = success.block;
                let failure_block = failure.block;

                let success_arguments = success.arguments(self.tree);
                self.mark_edge_executable(block_id, success_block, success_arguments);
                let failure_arguments = failure.arguments(self.tree);
                self.mark_edge_executable(block_id, failure_block, failure_arguments);

                self.edge_use_blocks
                    .entry(*length)
                    .or_default()
                    .insert(block_id);
            }
            mir::Terminator::VariantSwitch { default, cases, .. } => {
                // variant tags stay unfolded: every edge may execute
                if let Some(default) = default {
                    let arguments = default.arguments(self.tree);
                    self.mark_edge_executable(block_id, default.block, arguments);
                }
                let cases = self.tree.get_switch_cases(*cases);
                for case in cases {
                    let case_block = case.target.block;
                    let arguments = case.target.arguments(self.tree);

                    self.mark_edge_executable(block_id, case_block, arguments);
                }
            }
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                // evaluate switch condition
                let value_state = self.value_state(*value);

                let default_block = default.block;

                // mark executable edges for the switch
                if let LatticeValue::Constant(constant) = value_state {
                    if let Some(value) = switch_constant_value(&constant) {
                        let cases = self.tree.get_switch_cases(*cases);
                        if let Some(target) = select_switch_target(value, cases) {
                            let target_block = target.target.block;
                            let arguments = target.target.arguments(self.tree);

                            self.mark_edge_executable(block_id, target_block, arguments);
                        } else {
                            let arguments = default.arguments(self.tree);
                            self.mark_edge_executable(block_id, default_block, arguments);
                        }
                    } else {
                        let arguments = default.arguments(self.tree);
                        self.mark_edge_executable(block_id, default_block, arguments);
                        let cases = self.tree.get_switch_cases(*cases);
                        for case in cases {
                            let case_block = case.target.block;
                            let arguments = case.target.arguments(self.tree);

                            self.mark_edge_executable(block_id, case_block, arguments);
                        }
                    }
                } else {
                    let arguments = default.arguments(self.tree);
                    self.mark_edge_executable(block_id, default_block, arguments);
                    let cases = self.tree.get_switch_cases(*cases);
                    for case in cases {
                        let case_block = case.target.block;
                        let arguments = case.target.arguments(self.tree);

                        self.mark_edge_executable(block_id, case_block, arguments);
                    }
                }
            }
            mir::Terminator::Invoke { target, unwind, .. } => {
                let target_block = target.block;
                let arguments = target.arguments(self.tree);

                self.mark_edge_executable(block_id, target_block, arguments);
                let arguments = unwind.arguments(self.tree);
                self.mark_edge_executable(block_id, unwind.block, arguments);
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Panic { .. }
            | mir::Terminator::UnwindResume
            | mir::Terminator::Abort { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::TailCall { .. } => {}
        }
    }

    /// Record an executable edge and enqueue its target.
    fn mark_edge_executable(
        &mut self,
        pred: mir::LocalNodeId<mir::Block>,
        target: mir::LocalNodeId<mir::Block>,
        arguments: &[mir::Value],
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
        for block_id in self.uses.blocks(value) {
            self.enqueue_block(block_id);
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
                let left_state = self.value_state(*left);
                let right_state = self.value_state(*right);

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
                let argument_state = self.value_state(*argument);

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
                let argument_state = self.value_state(*argument);

                // fold based on operand state
                match argument_state {
                    LatticeValue::Constant(value) => fold_cast(
                        *operator,
                        value,
                        *to_type,
                        self.target_layout.pointer_bits(),
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
                let condition_state = self.value_state(*condition);
                let then_state = self.value_state(*then_value);
                let else_state = self.value_state(*else_value);

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
            mir::Instruction::Aggregate { values, .. } => {
                // evaluate aggregate values
                let arguments = self.tree.get_values(*values);
                self.evaluate_aggregate(arguments)
            }
            mir::Instruction::FieldGet {
                aggregate,
                field: index,
                ..
            }
            | mir::Instruction::ElementGet {
                aggregate, index, ..
            } => {
                // evaluate field get from aggregates
                let aggregate_state = self.value_state(*aggregate);
                self.evaluate_field_get(aggregate_state, *index as usize)
            }
            mir::Instruction::FieldSet {
                aggregate,
                field: index,
                value,
                ..
            }
            | mir::Instruction::ElementSet {
                aggregate,
                index,
                value,
                ..
            } => {
                // evaluate field set on aggregates
                let aggregate_state = self.value_state(*aggregate);
                let value_state = self.value_state(*value);
                self.evaluate_field_set(aggregate_state, *index as usize, value_state)
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
                for &argument in self.tree.get_values(*arguments) {
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
    fn evaluate_aggregate(&self, values: &[mir::Value]) -> LatticeValue {
        // collect operand lattice values
        let elements = values
            .iter()
            .map(|value| self.value_state(*value))
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
    cases.iter().find(|case| case.value == value)
}

/// Apply SCCP results to the function and tree.
fn apply_propagate_sparse_constants_result(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    result: &PropagateSparseConstantsResult,
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
    let block_ids = function.blocks().to_vec();
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
            let Some(destination) = instruction.destination() else {
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
                destination,
                value: constant,
            };
            tree.set(instruction_id, new_instruction);
            value_changed = true;
        }

        // fold constant branches and switches
        if let Some(new_terminator) = fold_constant_terminator(tree, &terminator, result)
            && new_terminator != terminator
        {
            let terminator_id = tree.get(block_id).terminator;
            tree.set(terminator_id, new_terminator);
            cfg_changed = true;
        }
    }

    // substitute constant uses after folding
    if !substitutions.is_empty() {
        value_changed |=
            function_substitute_constant_uses(function, tree, accesses, &substitutions);
    }

    // remove unreachable blocks
    let original_len = function.blocks().len();

    // retain only executable blocks
    function.retain_blocks(|block_id| result.is_executable(block_id), tree);

    // record cfg changes when blocks are removed
    if function.blocks().len() != original_len {
        cfg_changed = true;
    }

    (cfg_changed, value_changed)
}

/// Insert constants for block parameters and populate substitutions.
fn function_insert_block_param_constants(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    result: &PropagateSparseConstantsResult,
    substitutions: &mut HashMap<mir::Value, mir::Value>,
) -> bool {
    // track whether any updates occurred
    let mut changed = false;

    // scan blocks for executable constants
    let block_ids = function.blocks().to_vec();
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
            let value = param.value;
            let ty = param.ty;
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
                    destination: new_value,
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
            let block = tree.get(block_id);
            let mut updated = new_instructions;
            updated.extend(block.instructions.iter().copied());
            function.replace_block_instructions(block_id, updated, tree);
            changed = true;
        }
    }

    changed
}

/// Substitute constant values in instruction and terminator uses.
fn function_substitute_constant_uses(
    function: &mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> bool {
    // track whether any substitutions occur
    let mut changed = false;

    // rewrite constants in every block
    for &block_id in function.blocks() {
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
                tree.set(instruction_id, new_instruction);
                remap_instruction_memory_accesses(accesses, instruction_id, substitutions);
                changed = true;
            }
        }

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(tree, &terminator, substitutions);

        // update terminator when rewritten
        if new_terminator != terminator {
            tree.set(terminator_id, new_terminator);
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
    if instruction
        .uses()
        .iter()
        .any(|value| substitutions.contains_key(value))
    {
        return true;
    }

    // check externalized arguments
    if let Some(arguments) = instruction.argument_slice() {
        return tree
            .get_values(arguments)
            .iter()
            .any(|value| substitutions.contains_key(value));
    }

    false
}

/// Fold a terminator when its condition is constant.
fn fold_constant_terminator(
    tree: &mir::Tree,
    terminator: &mir::Terminator,
    result: &PropagateSparseConstantsResult,
) -> Option<mir::Terminator> {
    // fold conditional branches
    if let mir::Terminator::Branch {
        condition,
        then_target,
        else_target,
    } = terminator
    {
        // resolve branch condition
        let condition_state = result.value_state(*condition);
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
        let constant = result.value_constant(*value);
        if let Some(constant) = constant {
            // resolve switch constant
            let value = switch_constant_value(constant)?;

            // jump to matching case or default
            let cases = tree.get_switch_cases(*cases);
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
entry:
    v0: boolean = true
    branch v0 => b1 | b2

b1:
    v1: int32 = 10
    jump b3(v1)

b2:
    v2: int32 = 20
    jump b3(v2)

b3(v3: int32):
    v4: int32 = add v3, v3
    return v4
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: boolean = true
    jump b1

b1:
    v1: int32 = 10
    jump b2(v1)

b2(v3: int32):
    v5: int32 = 10
    v4: int32 = 20
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Identical constants on all edges keep block parameters constant.
    #[test]
    fn test_branch_with_same_constants_keeps_param_constant() {
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 3
    jump b3(v1)

b2:
    v2: int32 = 3
    jump b3(v2)

b3(v3: int32):
    v4: int32 = add v3, v3
    return v4
}
"#;
        let expected = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 3
    jump b3(v1)

b2:
    v2: int32 = 3
    jump b3(v2)

b3(v3: int32):
    v5: int32 = 3
    v4: int32 = 6
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Conflicting constants on different edges do not fold block parameters.
    #[test]
    fn test_branch_with_conflicting_constants_keeps_param_overdefined() {
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 3
    jump b3(v1)

b2:
    v2: int32 = 4
    jump b3(v2)

b3(v3: int32):
    v4: int32 = add v3, v3
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_unchanged(input);
    }

    /// Multiple edges to the same target with different arguments are overdefined.
    #[test]
    fn test_conflicting_same_target_arguments() {
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    v2: int32 = 2
    branch v0 => b1(v1) | b1(v2)

b1(v3: int32):
    v4: int32 = add v3, v3
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_unchanged(input);
    }

    /// Switch on a constant value folds to the matching target.
    #[test]
    fn test_switch_on_constant_value() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 2
    switch v0, b3, 1 => b1, 2 => b2

b1:
    v1: int32 = 10
    return v1

b2:
    v2: int32 = 20
    return v2

b3:
    v3: int32 = 30
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 2
    jump b1

b1:
    v2: int32 = 20
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Readonly global loads are not scalar constants.
    #[test]
    fn test_readonly_global_load_not_constant() {
        let input = r#"
readonly global flag: boolean = true

function test(): int32 {
entry:
    v0: ref<boolean, borrowed, readonly> = global.address flag
    v1: boolean = load v0
    branch v1 => b1 | b2

b1:
    v2: int32 = 1
    return v2

b2:
    v3: int32 = 2
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_unchanged(input);
    }

    /// Mutable globals are not treated as constants.
    #[test]
    fn test_mutable_global_not_constant() {
        let input = r#"
global flag: boolean = true

function test(): int32 {
entry:
    v0: ref<boolean, borrowed, mutable> = global.address flag
    v1: boolean = load v0
    branch v1 => b1 | b2

b1:
    v2: int32 = 1
    return v2

b2:
    v3: int32 = 2
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_unchanged(input);
    }

    /// Block parameter constants are substituted in uses.
    #[test]
    fn test_substitute_block_param_uses() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 3
    jump b1(v0)

b1(v1: int32):
    jump b2(v1)

b2(v2: int32):
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 3
    jump b1(v0)

b1(v1: int32):
    v3: int32 = 3
    jump b2(v3)

b2(v2: int32):
    v4: int32 = 3
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Call arguments are substituted when constants are available.
    #[test]
    fn test_substitute_call_arguments() {
        let input = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(): int32 {
entry:
    v0: int32 = 5
    jump b1(v0)

b1(v1: int32):
    v2: int32 = call callee(v1): (int32) => int32
    return v2
}
"#;
        let expected = r#"
function callee(v0: int32): int32 {
entry(v0: int32):
    return v0
}

function test(): int32 {
entry:
    v0: int32 = 5
    jump b1(v0)

b1(v1: int32):
    v3: int32 = 5
    v2: int32 = call callee(v3): (int32) => int32
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Switch constants compare by integer value without narrowing.
    #[test]
    fn test_switch_uint64_does_not_match_negative_case() {
        let input = r#"
function test(): int32 {
entry:
    v0: uint64 = 18446744073709551615
    switch v0, b2, -1 => b1

b1:
    v1: int32 = 1
    return v1

b2:
    v2: int32 = 2
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: uint64 = 18446744073709551615
    jump b1

b1:
    v2: int32 = 2
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Field access folds when a struct has constant fields.
    #[test]
    fn test_struct_field_get_constant() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 5
    v1: int32 = 7
    v2: { int32, int32 } = aggregate (v0, v1)
    v3: int32 = field.get v2, 0
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 5
    v1: int32 = 7
    v2: { int32, int32 } = aggregate (v0, v1)
    v3: int32 = 5
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Field access keeps constant elements even when other fields vary.
    #[test]
    fn test_struct_field_get_partial_constant() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 4
    v2: { int32, int32 } = aggregate (v1, v0)
    v3: int32 = field.get v2, 0
    return v3
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 4
    v2: { int32, int32 } = aggregate (v1, v0)
    v3: int32 = 4
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Field set updates aggregate constants for later field access.
    #[test]
    fn test_struct_field_set_constant() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: { int32, int32 } = aggregate (v0, v1)
    v3: int32 = 9
    v4: { int32, int32 } = field.set v2, 1, v3
    v5: int32 = field.get v4, 1
    return v5
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: { int32, int32 } = aggregate (v0, v1)
    v3: int32 = 9
    v4: { int32, int32 } = field.set v2, 1, v3
    v5: int32 = 9
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Element get folds for a statically selected fixed-array element.
    #[test]
    fn test_fold_element_get() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 10
    v1: int32 = 20
    v2: int32 = 30
    v3: [int32; 3] = aggregate (v0, v1, v2)
    v4: int64 = 1
    v5: int32 = element.get v3, 1
    return v5
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 10
    v1: int32 = 20
    v2: int32 = 30
    v3: [int32; 3] = aggregate (v0, v1, v2)
    v4: int64 = 1
    v5: int32 = 20
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Element set folds into a later element get at the same index.
    #[test]
    fn test_fold_element_set_then_get() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    v4: int64 = 1
    v5: int32 = 9
    v6: [int32; 3] = element.set v3, 1, v5
    v7: int32 = element.get v6, 1
    return v7
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: [int32; 3] = aggregate (v0, v1, v2)
    v4: int64 = 1
    v5: int32 = 9
    v6: [int32; 3] = element.set v3, 1, v5
    v7: int32 = 9
    return v7
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Readonly global aggregate loads are not aggregate constants.
    #[test]
    fn test_readonly_global_struct_field_get_not_constant() {
        let input = r#"
readonly global pair: { int32, int32 } = {1int32, 2int32}

function test(): int32 {
entry:
    v0: ref<{ int32, int32 }, borrowed, readonly> = global.address pair
    v1: { int32, int32 } = load v0
    v2: int32 = field.get v1, 1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_unchanged(input);
    }

    /// Zero initializer loads are not aggregate constants.
    #[test]
    fn test_global_zero_initializer_field_get_not_constant() {
        let input = r#"
readonly global pair: (int32, int32) = zeroInit

function test(): int32 {
entry:
    v0: ref<(int32, int32), borrowed, readonly> = global.address pair
    v1: (int32, int32) = load v0
    v2: int32 = field.get v1, 0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_unchanged(input);
    }

    /// Byte initializer loads are not array constants.
    #[test]
    fn test_global_bytes_field_get_not_constant() {
        let input = r#"
readonly global data: [uint8; 4] = b"test"

function test(): uint8 {
entry:
    v0: ref<[uint8; 4], borrowed, readonly> = global.address data
    v1: [uint8; 4] = load v0
    v2: int64 = 2
    v3: uint8 = field.get v1, 2
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_unchanged(input);
    }

    /// Constant selects are folded to the chosen value.
    #[test]
    fn test_fold_select_constant_condition() {
        let input = r#"
function test(): int32 {
entry:
    v0: boolean = true
    v1: int32 = 10
    v2: int32 = 20
    v3: int32 = select v0, v1, v2
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: boolean = true
    v1: int32 = 10
    v2: int32 = 20
    v3: int32 = 10
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }

    /// Pure intrinsics with constant operands are folded.
    #[test]
    fn test_fold_intrinsic_constant() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 8
    v1: int32 = intrinsic.math.bits.leadingZeroCount(v0)
    return v1
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 8
    v1: int32 = 28
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PropagateSparseConstants);
        test.assert_output(expected);
    }
}
