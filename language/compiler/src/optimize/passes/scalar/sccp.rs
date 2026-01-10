use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::common::{
    build_use_def_maps, constant_from_global, fold_binary, fold_cast, fold_unary,
};
use crate::optimize::{
    AnalysisKind, AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};

declare_pass! {
    /// Perform sparse conditional constant propagation.
    ///
    /// This pass tracks constant values along executable paths and folds
    /// operations where all operands are constant. It also uses constant
    /// branch conditions to mark unreachable blocks for removal.
    ///
    /// ```mir
    /// function @before() -> i32 {
    /// block0:
    ///     v0 = iconst true
    ///     branch v0, block1, block2
    /// block1:
    ///     v1 = iconst 10i32
    ///     jump block3(v1)
    /// block2:
    ///     v2 = iconst 20i32
    ///     jump block3(v2)
    /// block3(v3: i32):
    ///     v4 = iadd v3, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after() -> i32 {
    /// block0:
    ///     v0 = iconst true
    ///     jump block1
    /// block1:
    ///     v1 = iconst 10i32
    ///     jump block2(v1)
    /// block2(v3: i32):
    ///     v4 = iconst 20i32
    ///     return v4
    /// }
    /// ```
    #[pass(id = "sccp")]
    pub SparseConditionalConstantPropagation,
    "Sparse conditional constant propagation"
}

impl Pass for SparseConditionalConstantPropagation {
    fn metadata(&self) -> &'static PassMetadata {
        SparseConditionalConstantPropagation::metadata()
    }
}

impl FunctionPass for SparseConditionalConstantPropagation {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        // skip extern functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // build use def data
        let use_def = build_use_def_maps(function, tree);

        // run sccp analysis
        let mut state = SccpState::new(tree, &use_def.use_blocks, entry);
        let result = state.run();

        // apply constant folding and reachability
        let (cfg_changed, value_changed) = apply_sccp_result(function, tree, &result);

        // report preserved analyses
        if cfg_changed {
            AnalysisPreservation::none()
        } else if value_changed {
            AnalysisPreservation::Some(vec![AnalysisKind::ControlFlowGraph])
        } else {
            AnalysisPreservation::all()
        }
    }
}

/// Lattice state for SCCP values.
#[derive(Debug, Clone, PartialEq)]
enum LatticeValue {
    /// No information about the value yet.
    Unknown,
    /// A known constant value.
    Constant(mir::Constant),
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
        match self.value_states.get(&value) {
            Some(LatticeValue::Constant(constant)) => Some(constant),
            _ => None,
        }
    }
}

/// SCCP analysis state and worklists.
struct SccpState<'a> {
    /// The MIR tree for instruction lookup.
    tree: &'a mir::NodeTree,
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
    /// Worklist of blocks to process.
    block_worklist: VecDeque<mir::LocalNodeId<mir::Block>>,
    /// Blocks already in the worklist.
    in_worklist: HashSet<mir::LocalNodeId<mir::Block>>,
}

impl<'a> SccpState<'a> {
    /// Create a new SCCP analysis state.
    fn new(
        tree: &'a mir::NodeTree,
        use_blocks: &'a HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>>,
        entry: mir::LocalNodeId<mir::Block>,
    ) -> Self {
        Self {
            tree,
            use_blocks,
            entry,
            value_states: HashMap::new(),
            executable_blocks: HashSet::new(),
            executable_edges: HashSet::new(),
            block_worklist: VecDeque::new(),
            in_worklist: HashSet::new(),
        }
    }

    /// Run SCCP and return the analysis result.
    fn run(&mut self) -> SccpResult {
        // seed entry block
        self.mark_block_executable(self.entry);

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
        if edges.is_empty() {
            return;
        }

        // reject mismatched argument counts
        let expected = block.parameters.len();
        if edges.iter().any(|edge| edge.arguments.len() != expected) {
            for param in &block.parameters {
                self.update_value(param.value, LatticeValue::Overdefined);
            }
            return;
        }

        // merge incoming arguments per parameter
        for (index, param) in block.parameters.iter().enumerate() {
            let mut merged = LatticeValue::Unknown;

            // combine values from each executable edge
            for edge in &edges {
                let arg = edge.arguments[index];
                let incoming = self.value_state(arg);
                merged = merged.meet(&incoming);
            }

            self.update_value(param.value, merged);
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
        let terminator = &block.terminator;

        // mark edges based on terminator kind
        match terminator {
            mir::Terminator::Jump { target, arguments } => {
                self.mark_edge_executable(block_id, *target, arguments);
            }
            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                // evaluate branch condition
                let condition_state = self.value_state(*condition);

                // mark executable edges for the branch
                if let LatticeValue::Constant(mir::Constant::Boolean { value }) = condition_state {
                    if value {
                        self.mark_edge_executable(block_id, *then_target, then_arguments);
                    } else {
                        self.mark_edge_executable(block_id, *else_target, else_arguments);
                    }
                } else {
                    self.mark_edge_executable(block_id, *then_target, then_arguments);
                    self.mark_edge_executable(block_id, *else_target, else_arguments);
                }
            }
            mir::Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
            } => {
                // evaluate switch condition
                let value_state = self.value_state(*value);

                // mark executable edges for the switch
                if let LatticeValue::Constant(constant) = value_state {
                    if let Some(value) = switch_constant_value(&constant) {
                        if let Some(target) = select_switch_target(value, cases) {
                            let arguments = &target.arguments;
                            self.mark_edge_executable(block_id, target.target, arguments);
                        } else {
                            self.mark_edge_executable(block_id, *default, default_arguments);
                        }
                    } else {
                        self.mark_edge_executable(block_id, *default, default_arguments);
                        for case in cases {
                            self.mark_edge_executable(block_id, case.target, &case.arguments);
                        }
                    }
                } else {
                    self.mark_edge_executable(block_id, *default, default_arguments);
                    for case in cases {
                        self.mark_edge_executable(block_id, case.target, &case.arguments);
                    }
                }
            }
            mir::Terminator::Yield {
                resume,
                resume_arguments,
                ..
            } => {
                self.mark_edge_executable(block_id, *resume, resume_arguments);
            }
            mir::Terminator::Return { .. } | mir::Terminator::Unreachable => {}
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
        if !self.executable_edges.insert(edge) {
            return;
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
    }

    /// Get the lattice state for a value.
    fn value_state(&self, value: mir::Value) -> LatticeValue {
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
            mir::Instruction::GlobalConst { global, .. } => {
                // read global constant
                if let Some(constant) = constant_from_global(*global, self.tree) {
                    LatticeValue::Constant(constant)
                } else {
                    LatticeValue::Overdefined
                }
            }
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
                    LatticeValue::Constant(value) => {
                        fold_cast(*operator, value, *to_type, self.tree)
                            .map(LatticeValue::Constant)
                            .unwrap_or(LatticeValue::Overdefined)
                    }
                    LatticeValue::Overdefined => LatticeValue::Overdefined,
                    LatticeValue::Unknown => LatticeValue::Unknown,
                }
            }
            _ => LatticeValue::Overdefined,
        }
    }
}

/// Convert a constant to a switch value when possible.
fn switch_constant_value(constant: &mir::Constant) -> Option<i64> {
    // convert numeric constants to switch values
    match constant {
        mir::Constant::Int { value, .. } => Some(*value),
        mir::Constant::UInt { value, .. } => i64::try_from(*value).ok(),
        _ => None,
    }
}

/// Select the switch case that matches a constant value.
fn select_switch_target<'a>(
    value: i64,
    cases: &'a [mir::SwitchCase],
) -> Option<&'a mir::SwitchCase> {
    // find matching case
    cases.iter().find(|case| case.value == value)
}

/// Apply SCCP results to the function and tree.
fn apply_sccp_result(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    result: &SccpResult,
) -> (bool, bool) {
    let mut cfg_changed = false;
    let mut value_changed = false;

    // fold instructions and terminators in executable blocks
    for &block_id in &function.blocks {
        // skip non executable blocks
        if !result.is_executable(block_id) {
            continue;
        }

        // snapshot instruction ids and terminator
        let (instruction_ids, terminator) = {
            let block = tree.get(block_id);
            (block.instructions.clone(), block.terminator.clone())
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
            tree.replace(instruction_id, new_instruction);
            value_changed = true;
        }

        // fold constant branches and switches
        if let Some(new_terminator) = fold_constant_terminator(&terminator, result) {
            if new_terminator != terminator {
                tree.get_mut(block_id).terminator = new_terminator;
                cfg_changed = true;
            }
        }
    }

    // remove unreachable blocks
    let original_len = function.blocks.len();
    function
        .blocks
        .retain(|block_id| result.is_executable(*block_id));
    if function.blocks.len() != original_len {
        cfg_changed = true;
    }

    (cfg_changed, value_changed)
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
        then_arguments,
        else_target,
        else_arguments,
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
            let (target, arguments) = if is_true {
                (*then_target, then_arguments.clone())
            } else {
                (*else_target, else_arguments.clone())
            };
            return Some(mir::Terminator::Jump { target, arguments });
        }
    }

    // fold switch on constant value
    if let mir::Terminator::Switch {
        value,
        default,
        default_arguments,
        cases,
    } = terminator
    {
        // resolve switch condition
        let constant = result.value_constant(*value);
        if let Some(constant) = constant {
            // resolve switch constant
            let value = switch_constant_value(constant)?;

            // jump to matching case or default
            if let Some(case) = select_switch_target(value, cases) {
                return Some(mir::Terminator::Jump {
                    target: case.target,
                    arguments: case.arguments.clone(),
                });
            }

            return Some(mir::Terminator::Jump {
                target: *default,
                arguments: default_arguments.clone(),
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
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    branch v0, block1, block2
block1:
    v1 = iconst 10i32
    jump block3(v1)
block2:
    v2 = iconst 20i32
    jump block3(v2)
block3(v3: i32):
    v4 = iadd v3, v3
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    jump block1
block1:
    v1 = iconst 10i32
    jump block2(v1)
block2(v3: i32):
    v4 = iconst 20i32
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SparseConditionalConstantPropagation);
        program.assert_output(expected);
    }

    /// Identical constants on all edges keep block parameters constant.
    #[test]
    fn test_branch_with_same_constants_keeps_param_constant() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 3i32
    jump block3(v1)
block2:
    v2 = iconst 3i32
    jump block3(v2)
block3(v3: i32):
    v4 = iadd v3, v3
    return v4
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 3i32
    jump block3(v1)
block2:
    v2 = iconst 3i32
    jump block3(v2)
block3(v3: i32):
    v4 = iconst 6i32
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SparseConditionalConstantPropagation);
        program.assert_output(expected);
    }

    /// Conflicting constants on different edges do not fold block parameters.
    #[test]
    fn test_branch_with_conflicting_constants_keeps_param_overdefined() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 3i32
    jump block3(v1)
block2:
    v2 = iconst 4i32
    jump block3(v2)
block3(v3: i32):
    v4 = iadd v3, v3
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SparseConditionalConstantPropagation);
        program.assert_unchanged(input);
    }

    /// Multiple edges to the same target with different arguments are overdefined.
    #[test]
    fn test_conflicting_same_target_arguments() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1(v1), block1(v2)
block1(v3: i32):
    v4 = iadd v3, v3
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SparseConditionalConstantPropagation);
        program.assert_unchanged(input);
    }

    /// Switch on a constant value folds to the matching target.
    #[test]
    fn test_switch_on_constant_value() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 2i32
    switch v0, block3, 1 => block1, 2 => block2
block1:
    v1 = iconst 10i32
    return v1
block2:
    v2 = iconst 20i32
    return v2
block3:
    v3 = iconst 30i32
    return v3
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 2i32
    jump block1
block1:
    v2 = iconst 20i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SparseConditionalConstantPropagation);
        program.assert_output(expected);
    }

    /// Global const values fold branch conditions and remove dead blocks.
    #[test]
    fn test_global_const_branch_folding() {
        let input = r#"global @flag: bool = true ; const
function @test() -> i32 {
block0:
    v0 = global.const @flag
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
}"#;
        let expected = r#"global @flag: bool = true ; const
function @test() -> i32 {
block0:
    v0 = iconst true
    jump block1
block1:
    v1 = iconst 1i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SparseConditionalConstantPropagation);
        program.assert_output(expected);
    }

    /// Mutable globals are not treated as constants.
    #[test]
    fn test_mutable_global_not_constant() {
        let input = r#"global @flag: bool = true ; mut
function @test() -> i32 {
block0:
    v0 = global.const @flag
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SparseConditionalConstantPropagation);
        program.assert_unchanged(input);
    }
}
