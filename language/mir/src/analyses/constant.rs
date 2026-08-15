use std::collections::{HashMap, VecDeque};

use crate as mir;

use crate::{
    Analysis, ConstantLookup, FunctionCache, NodeTable, TargetLayout, fold_binary, fold_cast,
    fold_unary,
};

use super::{ControlTable, Lattice};

/// Constant propagation for one function.
#[derive(Debug)]
pub struct ConstantTable {
    /// Constants available at block entry indexed by block id.
    block_entry: NodeTable<mir::Block, Option<ConstantState>>,
    /// Constants available at block exit indexed by block id.
    block_exit: NodeTable<mir::Block, Option<ConstantState>>,
}

/// Mapping from SSA values to known constants.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConstantState {
    /// Known constant values.
    constants: HashMap<mir::Value, mir::Constant>,
}

impl ConstantState {
    /// Create an empty constant map.
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
        }
    }

    /// Return the constant for one SSA value.
    pub fn get(&self, value: impl Into<mir::Value>) -> Option<&mir::Constant> {
        let value = value.into();
        self.constants.get(&value)
    }

    /// Insert a constant value for a given SSA value.
    pub fn insert(&mut self, value: impl Into<mir::Value>, constant: mir::Constant) {
        let value = value.into();

        self.constants.insert(value, constant);
    }

    /// Remove any constant for a given SSA value.
    pub fn remove(&mut self, value: impl Into<mir::Value>) {
        let value = value.into();

        self.constants.remove(&value);
    }

    /// Iterate over known constants.
    pub fn iter(&self) -> impl Iterator<Item = (mir::Value, &mir::Constant)> + '_ {
        self.constants
            .iter()
            .map(|(value, constant)| (*value, constant))
    }
}

impl ConstantLookup for ConstantState {
    /// Return the constant value for a MIR value when known.
    fn get_constant(&self, value: mir::Value) -> Option<&mir::Constant> {
        self.get(value)
    }
}

impl Lattice for ConstantState {
    /// Intersect constants that agree on both inputs.
    fn meet(&self, other: &Self) -> Self {
        let mut constants = HashMap::new();

        for (value, constant) in &self.constants {
            if let Some(other_constant) = other.constants.get(value)
                && other_constant == constant
            {
                constants.insert(*value, constant.clone());
            }
        }

        Self { constants }
    }
}

impl ConstantTable {
    /// Build constant propagation for a function.
    fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlTable,
        target_layout: TargetLayout,
    ) -> Self {
        Self::build_with_entry_constants(function, tree, cfg, target_layout, ConstantState::new())
    }

    /// Build constant propagation with seeded entry constants.
    fn build_with_entry_constants(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlTable,
        target_layout: TargetLayout,
        entry_constants: ConstantState,
    ) -> Self {
        // select the entry block
        let entry = match function.entry() {
            Some(entry) => entry,
            None => {
                return Self {
                    block_entry: NodeTable::new(),
                    block_exit: NodeTable::new(),
                };
            }
        };

        // init state maps
        let mut block_entry = NodeTable::from_nodes(function.blocks(), || None);
        let mut block_exit = NodeTable::from_nodes(function.blocks(), || None);

        // seed entry state
        *block_entry.get_mut(entry) = Some(entry_constants);

        // init worklist
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist = NodeTable::from_nodes(function.blocks(), || false);
        worklist.push_back(entry);
        *in_worklist.get_mut(entry) = true;

        // process blocks until fixed point
        while let Some(block_id) = worklist.pop_front() {
            // remove block from worklist
            *in_worklist.get_mut(block_id) = false;

            // compute entry state
            let entry_state = if block_id == entry {
                block_entry.get(entry).as_ref().cloned().unwrap_or_else(|| {
                    panic!("missing constant propagation entry state: {entry:?}")
                })
            } else {
                // merge predecessor exits
                let mut merged: Option<ConstantState> = None;
                for &pred in cfg.predecessors(block_id) {
                    let Some(pred_exit) = block_exit.get(pred).as_ref() else {
                        continue;
                    };

                    merged = Some(match merged {
                        Some(state) => state.meet(pred_exit),
                        None => pred_exit.clone(),
                    });
                }

                // skip until predecessors are processed
                let Some(mut merged_state) = merged else {
                    continue;
                };

                // apply block parameter constants
                Self::apply_parameters(block_id, tree, cfg, &block_exit, &mut merged_state);
                merged_state
            };

            // update entry state if needed
            let entry_changed = block_entry
                .get(block_id)
                .as_ref()
                .map(|old| old != &entry_state)
                .unwrap_or(true);

            if entry_changed || block_id == entry {
                // record entry state
                *block_entry.get_mut(block_id) = Some(entry_state.clone());

                // compute exit state
                let exit_state =
                    Self::transfer(block_id, &entry_state, tree, target_layout.pointer_bits());
                let exit_changed = block_exit
                    .get(block_id)
                    .as_ref()
                    .map(|old| old != &exit_state)
                    .unwrap_or(true);

                if exit_changed {
                    // record exit state
                    *block_exit.get_mut(block_id) = Some(exit_state);

                    // enqueue successors
                    let block = tree.get(block_id);
                    let terminator = tree.get(block.terminator);
                    for succ in terminator.successors(tree) {
                        if !*in_worklist.get(succ) {
                            *in_worklist.get_mut(succ) = true;
                            worklist.push_back(succ);
                        }
                    }
                }
            }
        }

        Self {
            block_entry,
            block_exit,
        }
    }

    /// Return constants at block entry.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> &ConstantState {
        let constants = self.block_entry.get(block);

        match constants {
            Some(constants) => constants,
            None => ConstantState::empty(),
        }
    }

    /// Return constants at block exit.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> &ConstantState {
        let constants = self.block_exit.get(block);

        match constants {
            Some(constants) => constants,
            None => ConstantState::empty(),
        }
    }

    /// Return one constant at block entry.
    pub fn constant_at_entry(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        value: mir::Value,
    ) -> Option<&mir::Constant> {
        self.entry(block).get(value)
    }

    /// Return one constant at block exit.
    pub fn constant_at_exit(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        value: mir::Value,
    ) -> Option<&mir::Constant> {
        self.exit(block).get(value)
    }

    /// Build constant propagation with constant parameters seeded at entry.
    pub fn with_parameter_constants(
        function: &mir::Function,
        tree: &mir::Tree,
        target_layout: TargetLayout,
        param_constants: &HashMap<mir::Value, mir::Constant>,
    ) -> Self {
        // build a control flow graph for the function
        let cfg = ControlTable::build(function, tree);

        // seed entry constants from the provided parameter map
        let mut entry_constants = ConstantState::new();
        for (value, constant) in param_constants {
            entry_constants.insert(*value, constant.clone());
        }

        Self::build_with_entry_constants(function, tree, &cfg, target_layout, entry_constants)
    }
}

impl Analysis for ConstantTable {
    const INVALIDATED_BY: mir::Mutation = mir::Mutation::CONTROL
        .union(mir::Mutation::VALUE)
        .union(mir::Mutation::LAYOUT);
}

impl ConstantTable {
    /// Compute constants for one function.
    pub(crate) fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &mut FunctionCache,
    ) -> Self {
        let cfg = analyses.control(function, tree);
        Self::build(function, tree, &cfg, analyses.target_layout())
    }
}

/// Constant state for a block parameter.
#[derive(Debug, Clone)]
enum ParamState {
    /// No predecessor has been processed yet.
    Unseen,
    /// All seen predecessors agree on a constant value.
    Constant(mir::Constant),
    /// The value differs across predecessors or is not constant.
    Overdefined,
}

impl ConstantTable {
    /// Apply block parameter constants derived from predecessor arguments.
    fn apply_parameters(
        block_id: mir::LocalNodeId<mir::Block>,
        tree: &mir::Tree,
        cfg: &ControlTable,
        block_exit: &NodeTable<mir::Block, Option<ConstantState>>,
        entry_state: &mut ConstantState,
    ) {
        // resolve constants for block parameters
        let constants = Self::resolve_parameters(block_id, tree, cfg, block_exit);
        let block = tree.get(block_id);

        // apply constants to entry state
        for param in &block.parameters {
            let param_value = param.value;

            if let Some(constant) = constants.get(&param_value) {
                entry_state.insert(param_value, constant.clone());
                continue;
            }

            entry_state.remove(param_value);
        }
    }

    /// Resolve constant values for block parameters from predecessor arguments.
    fn resolve_parameters(
        block_id: mir::LocalNodeId<mir::Block>,
        tree: &mir::Tree,
        cfg: &ControlTable,
        block_exit: &NodeTable<mir::Block, Option<ConstantState>>,
    ) -> HashMap<mir::Value, mir::Constant> {
        // early exit for blocks without parameters
        let block = tree.get(block_id);
        if block.parameters.is_empty() {
            return HashMap::new();
        }

        // track parameter states across predecessors
        let mut states = vec![ParamState::Unseen; block.parameters.len()];
        let mut is_seen = false;

        // scan predecessors
        for &pred in cfg.predecessors(block_id) {
            let Some(pred_exit) = block_exit.get(pred).as_ref() else {
                continue;
            };

            // process every exact edge from this predecessor
            let pred_block = tree.get(pred);
            let pred_terminator = tree.get(pred_block.terminator);
            for (edge, target) in pred_terminator
                .targets(tree, pred)
                .into_iter()
                .filter(|(_, target)| target.block == block_id)
            {
                is_seen = true;

                // require the verified target shape
                let Some(parameters) =
                    pred_terminator.target_parameters(tree, edge.successor, target)
                else {
                    states.fill(ParamState::Overdefined);
                    continue;
                };
                let arguments = target.arguments(tree);

                // update parameter states from arguments
                for (parameter, argument) in parameters.iter().zip(arguments) {
                    let Some(index) = block
                        .parameters
                        .iter()
                        .position(|candidate| candidate.value == parameter.value)
                    else {
                        states.fill(ParamState::Overdefined);
                        break;
                    };
                    let argument_constant = pred_exit.get(*argument);
                    states[index] = match (&states[index], argument_constant) {
                        (ParamState::Unseen, Some(constant)) => {
                            ParamState::Constant(constant.clone())
                        }
                        (ParamState::Unseen, None) => ParamState::Overdefined,
                        (ParamState::Constant(existing), Some(constant))
                            if existing == constant =>
                        {
                            ParamState::Constant(existing.clone())
                        }
                        (ParamState::Constant(_), Some(_)) => ParamState::Overdefined,
                        (ParamState::Constant(_), None) => ParamState::Overdefined,
                        (ParamState::Overdefined, _) => ParamState::Overdefined,
                    };
                }
            }
        }

        // return empty if no predecessors were processed
        if !is_seen {
            return HashMap::new();
        }

        // collect constants for parameters
        let mut constants = HashMap::new();
        for (param, state) in block.parameters.iter().zip(states) {
            let param_value = param.value;

            if let ParamState::Constant(constant) = state {
                constants.insert(param_value, constant);
            }
        }

        constants
    }

    /// Transfer constants through a block's instructions.
    fn transfer(
        block_id: mir::LocalNodeId<mir::Block>,
        entry_state: &ConstantState,
        tree: &mir::Tree,
        pointer_width_bits: u16,
    ) -> ConstantState {
        // clone entry state for updates
        let block = tree.get(block_id);
        let mut state = entry_state.clone();

        // update state per instruction
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            let Some(destination) = instruction.destination() else {
                continue;
            };

            if let Some(constant) =
                Self::instruction_constant(instruction, tree, &state, pointer_width_bits)
            {
                state.insert(destination, constant);
            } else {
                state.remove(destination);
            }
        }

        state
    }

    /// Evaluate a constant for an instruction when possible.
    fn instruction_constant(
        instruction: &mir::Instruction,
        tree: &mir::Tree,
        state: &ConstantState,
        pointer_width_bits: u16,
    ) -> Option<mir::Constant> {
        // evaluate known constant producing instructions
        match instruction {
            mir::Instruction::Const { value, .. } => Some(value.clone()),
            mir::Instruction::Binary {
                operator,
                left,
                right,
                ..
            } => {
                // fold binary ops with constant operands
                let left_constant = state.get(*left)?;
                let right_constant = state.get(*right)?;
                fold_binary(*operator, left_constant.clone(), right_constant.clone())
            }
            mir::Instruction::Unary {
                operator, argument, ..
            } => {
                // fold unary ops with constant operands
                let arg_constant = state.get(*argument)?;
                fold_unary(*operator, arg_constant.clone())
            }
            mir::Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => {
                // fold casts with constant operands
                let arg_constant = state.get(*argument)?;
                fold_cast(
                    *operator,
                    arg_constant.clone(),
                    *to_type,
                    pointer_width_bits,
                    tree,
                )
            }
            _ => None,
        }
    }
}

impl ConstantState {
    /// Return a shared empty constant map.
    fn empty() -> &'static Self {
        // initialize the shared empty state
        static EMPTY: std::sync::OnceLock<ConstantState> = std::sync::OnceLock::new();

        EMPTY.get_or_init(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// Readonly global loads are not represented as scalar constants.
    #[test]
    fn test_readonly_global_load_not_constant() {
        let test = TestProgram::new(
            r#"
readonly global flag: boolean = true

function test(): boolean {
entry:
    v0: ref<boolean, borrowed, readonly> = global.address flag
    v1: boolean = load v0
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function, &test.tree);

        let block0 = function.block(0);
        let constant = analysis
            .constant_at_exit(block0, mir::Value::new(1))
            .cloned();
        assert_eq!(constant, None);
    }

    /// Mutable globals are not treated as constants.
    #[test]
    fn test_mutable_global_not_constant() {
        let test = TestProgram::new(
            r#"
global flag: boolean = true

function test(): boolean {
entry:
    v0: ref<boolean, borrowed, mutable> = global.address flag
    v1: boolean = load v0
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function, &test.tree);

        let block0 = function.block(0);
        let constant = analysis
            .constant_at_exit(block0, mir::Value::new(1))
            .cloned();
        assert_eq!(constant, None);
    }

    /// Non scalar globals are not treated as constants.
    #[test]
    fn test_non_scalar_global_not_constant() {
        let test = TestProgram::new(
            r#"
readonly global flag: boolean = zeroInit

function test(): boolean {
entry:
    v0: ref<boolean, borrowed, readonly> = global.address flag
    v1: boolean = load v0
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function, &test.tree);

        let block0 = function.block(0);
        let constant = analysis
            .constant_at_exit(block0, mir::Value::new(1))
            .cloned();
        assert_eq!(constant, None);
    }

    /// Constant results of binary operations are propagated.
    #[test]
    fn test_constant_from_binary() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: int32 = 2
    v1: int32 = 3
    v2: int32 = add v0, v1
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function, &test.tree);

        let block0 = function.block(0);
        let constant = analysis
            .constant_at_exit(block0, mir::Value::new(2))
            .cloned();
        assert_eq!(
            constant,
            Some(mir::Constant::Int {
                value: 5,
                width: 32,
                is_signed: true,
            })
        );
    }

    /// Block parameters become constant when all predecessors agree.
    #[test]
    fn test_constant_from_block_param() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    v1: boolean = true
    branch v0 => b1(v1) | b2(v1)

b1(v2: boolean):
    jump b3(v2)

b2(v3: boolean):
    jump b3(v3)

b3(v4: boolean):
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function, &test.tree);

        let block3 = function.block(3);
        let constant = analysis
            .constant_at_entry(block3, mir::Value::new(4))
            .cloned();
        assert_eq!(constant, Some(mir::Constant::Boolean { value: true }));
    }

    /// Fallible allocation result parameters do not consume edge arguments.
    #[test]
    fn test_constant_from_fallible_allocation_success_argument() {
        let test = TestProgram::new(
            r#"
function test(v0: int64): boolean {
entry(v0: int64):
    v1: boolean = true
    new.slice.uninit.try int32, v0 => b1(v1) | b2

b1(v2: uninit<slice<int32, managed, mutable>>, v3: boolean):
    return v3

b2:
    v4: boolean = false
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function, &test.tree);

        let success = function.block(1);
        let success_block = test.tree.get(success);
        let argument = success_block.parameters[1].value;
        let constant = analysis.constant_at_entry(success, argument).cloned();

        assert_eq!(constant, Some(mir::Constant::Boolean { value: true }));
    }

    /// Block parameters are not constant when predecessors disagree.
    #[test]
    fn test_conflicting_block_param() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    v1: boolean = true
    v2: boolean = false
    branch v0 => b1(v1) | b2(v2)

b1(v3: boolean):
    jump b3(v3)

b2(v4: boolean):
    jump b3(v4)

b3(v5: boolean):
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function, &test.tree);

        let block3 = function.block(3);
        let constant = analysis
            .constant_at_entry(block3, mir::Value::new(5))
            .cloned();
        assert_eq!(constant, None);
    }

    /// Conflicting arguments to a single target are not treated as constants.
    #[test]
    fn test_conflicting_target_arguments() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    v1: boolean = true
    v2: boolean = false
    branch v0 => b1(v1) | b1(v2)

b1(v3: boolean):
    return v3
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function, &test.tree);

        let block1 = function.block(1);
        let constant = analysis
            .constant_at_entry(block1, mir::Value::new(3))
            .cloned();
        assert_eq!(constant, None);
    }
}
