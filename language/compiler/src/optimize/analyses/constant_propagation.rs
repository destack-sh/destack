use std::collections::{HashMap, HashSet, VecDeque};

use destack_mir as mir;

use crate::optimize::common::{
    ConstantLookup, SuccessorArguments, fold_binary, fold_cast, fold_unary,
    terminator_arguments_for_successor_checked,
};
use crate::optimize::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis, TypeContext};

use super::{ControlFlowGraph, Lattice};

/// Mapping from SSA values to known constants.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConstantMap {
    /// Known constant values.
    constants: HashMap<mir::Value, mir::Constant>,
}

impl ConstantMap {
    /// Create an empty constant map.
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
        }
    }

    /// Get the constant value for a given SSA value.
    pub fn get(&self, value: impl Into<mir::ValueReference>) -> Option<&mir::Constant> {
        let value = value.into().value()?;
        self.constants.get(&value)
    }

    /// Insert a constant value for a given SSA value.
    pub fn insert(&mut self, value: impl Into<mir::ValueReference>, constant: mir::Constant) {
        let Some(value) = value.into().value() else {
            return;
        };

        self.constants.insert(value, constant);
    }

    /// Remove any constant for a given SSA value.
    pub fn remove(&mut self, value: impl Into<mir::ValueReference>) {
        let Some(value) = value.into().value() else {
            return;
        };

        self.constants.remove(&value);
    }

    /// Iterate over known constants.
    pub fn iter(&self) -> impl Iterator<Item = (mir::Value, &mir::Constant)> + '_ {
        self.constants
            .iter()
            .map(|(value, constant)| (*value, constant))
    }
}

impl ConstantLookup for ConstantMap {
    /// Return the constant value for a MIR value when known.
    fn get_constant(&self, value: mir::Value) -> Option<&mir::Constant> {
        self.get(value)
    }
}

impl Lattice for ConstantMap {
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

/// Constant propagation analysis.
///
/// Tracks constant values at block entry and exit using a forward dataflow
/// analysis with SSA aware handling of block parameters.
#[derive(Debug)]
pub struct ConstantPropagation {
    /// Constants available at block entry.
    block_entry: HashMap<mir::LocalNodeId<mir::Block>, ConstantMap>,
    /// Constants available at block exit.
    block_exit: HashMap<mir::LocalNodeId<mir::Block>, ConstantMap>,
}

impl ConstantPropagation {
    /// Build constant propagation for a function.
    fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlFlowGraph,
        type_context: TypeContext,
    ) -> Self {
        Self::build_with_entry_constants(function, tree, cfg, type_context, ConstantMap::new())
    }

    /// Build constant propagation with seeded entry constants.
    fn build_with_entry_constants(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlFlowGraph,
        type_context: TypeContext,
        entry_constants: ConstantMap,
    ) -> Self {
        // entry block selection
        let entry = match function.entry {
            Some(entry) => entry,
            None => {
                return Self {
                    block_entry: HashMap::new(),
                    block_exit: HashMap::new(),
                };
            }
        };

        // init state maps
        let mut block_entry: HashMap<mir::LocalNodeId<mir::Block>, ConstantMap> = HashMap::new();
        let mut block_exit: HashMap<mir::LocalNodeId<mir::Block>, ConstantMap> = HashMap::new();

        // seed entry state
        block_entry.insert(entry, entry_constants);

        // init worklist
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();
        worklist.push_back(entry);
        in_worklist.insert(entry);

        // process blocks until fixed point
        while let Some(block_id) = worklist.pop_front() {
            // remove block from worklist
            in_worklist.remove(&block_id);

            // compute entry state
            let entry_state = if block_id == entry {
                block_entry
                    .get(&entry)
                    .cloned()
                    .unwrap_or_else(ConstantMap::new)
            } else {
                // merge predecessor exits
                let mut merged: Option<ConstantMap> = None;
                for &pred in cfg.predecessors(block_id) {
                    let Some(pred_exit) = block_exit.get(&pred) else {
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
                apply_block_param_constants(block_id, tree, cfg, &block_exit, &mut merged_state);
                merged_state
            };

            // update entry state if needed
            let entry_changed = block_entry
                .get(&block_id)
                .map(|old| old != &entry_state)
                .unwrap_or(true);

            if entry_changed || block_id == entry {
                // record entry state
                block_entry.insert(block_id, entry_state.clone());

                // compute exit state
                let exit_state = transfer_block(
                    block_id,
                    &entry_state,
                    tree,
                    type_context.pointer_width_bits,
                );
                let exit_changed = block_exit
                    .get(&block_id)
                    .map(|old| old != &exit_state)
                    .unwrap_or(true);

                if exit_changed {
                    // record exit state
                    block_exit.insert(block_id, exit_state);

                    // enqueue successors
                    let block = tree.get(block_id);
                    let terminator = tree.get(block.terminator);
                    for succ in terminator.successors() {
                        let Some(succ) = succ.block() else {
                            continue;
                        };

                        if in_worklist.insert(succ) {
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

    /// Get the constants at block entry.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> &ConstantMap {
        // read entry state
        match self.block_entry.get(&block) {
            Some(constants) => constants,
            None => empty_map(),
        }
    }

    /// Get the constants at block exit.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> &ConstantMap {
        // read exit state
        match self.block_exit.get(&block) {
            Some(constants) => constants,
            None => empty_map(),
        }
    }

    /// Get a constant value at block entry.
    pub fn constant_at_entry(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        value: mir::Value,
    ) -> Option<&mir::Constant> {
        self.entry(block).get(value)
    }

    /// Get a constant value at block exit.
    pub fn constant_at_exit(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        value: mir::Value,
    ) -> Option<&mir::Constant> {
        self.exit(block).get(value)
    }
}

/// Build constant propagation with constant parameters seeded at entry.
pub fn constant_propagation_with_params(
    function: &mir::Function,
    tree: &mir::Tree,
    type_context: TypeContext,
    param_constants: &HashMap<mir::Value, mir::Constant>,
) -> ConstantPropagation {
    // build a control flow graph for the function
    let cfg = ControlFlowGraph::build(function, tree);

    // seed entry constants from the provided parameter map
    let mut entry_constants = ConstantMap::new();
    for (value, constant) in param_constants {
        entry_constants.insert(*value, constant.clone());
    }

    ConstantPropagation::build_with_entry_constants(
        function,
        tree,
        &cfg,
        type_context,
        entry_constants,
    )
}

impl Analysis for ConstantPropagation {
    const ID: AnalysisId = AnalysisId("constprop");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for ConstantPropagation {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();
        Self::build(function, tree, &cfg, analyses.type_context())
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

/// Apply block parameter constants derived from predecessor arguments.
fn apply_block_param_constants(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
    block_exit: &HashMap<mir::LocalNodeId<mir::Block>, ConstantMap>,
    entry_state: &mut ConstantMap,
) {
    // resolve constants for block parameters
    let constants = resolve_block_param_constants(block_id, tree, cfg, block_exit);
    let block = tree.get(block_id);

    // apply constants to entry state
    for param in &block.parameters {
        let Some(param_value) = param.value.value() else {
            continue;
        };

        if let Some(constant) = constants.get(&param_value) {
            entry_state.insert(param_value, constant.clone());
            continue;
        }

        entry_state.remove(param_value);
    }
}

/// Resolve constant values for block parameters from predecessor arguments.
fn resolve_block_param_constants(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
    block_exit: &HashMap<mir::LocalNodeId<mir::Block>, ConstantMap>,
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
        let Some(pred_exit) = block_exit.get(&pred) else {
            continue;
        };

        // collect arguments for this edge
        let pred_block = tree.get(pred);
        let pred_terminator = tree.get(pred_block.terminator);
        let args = match terminator_arguments_for_successor_checked(pred_terminator, block_id) {
            SuccessorArguments::Missing => continue,
            SuccessorArguments::Conflict => {
                states.fill(ParamState::Overdefined);
                is_seen = true;
                continue;
            }
            SuccessorArguments::Consistent(args) => args,
        };

        // mark that we saw a predecessor
        is_seen = true;

        // reject mismatched argument counts
        if args.len() != states.len() {
            states.fill(ParamState::Overdefined);
            continue;
        }

        // update parameter states from arguments
        for (index, arg) in args.iter().enumerate() {
            let arg_constant = pred_exit.get(*arg);
            states[index] = match (&states[index], arg_constant) {
                (ParamState::Unseen, Some(constant)) => ParamState::Constant(constant.clone()),
                (ParamState::Unseen, None) => ParamState::Overdefined,
                (ParamState::Constant(existing), Some(constant)) if existing == constant => {
                    ParamState::Constant(existing.clone())
                }
                (ParamState::Constant(_), Some(_)) => ParamState::Overdefined,
                (ParamState::Constant(_), None) => ParamState::Overdefined,
                (ParamState::Overdefined, _) => ParamState::Overdefined,
            };
        }
    }

    // return empty if no predecessors were processed
    if !is_seen {
        return HashMap::new();
    }

    // collect constants for parameters
    let mut constants = HashMap::new();
    for (param, state) in block.parameters.iter().zip(states.into_iter()) {
        let Some(param_value) = param.value.value() else {
            continue;
        };

        if let ParamState::Constant(constant) = state {
            constants.insert(param_value, constant);
        }
    }

    constants
}

/// Transfer constants through a block's instructions.
fn transfer_block(
    block_id: mir::LocalNodeId<mir::Block>,
    entry_state: &ConstantMap,
    tree: &mir::Tree,
    pointer_width_bits: u16,
) -> ConstantMap {
    // clone entry state for updates
    let block = tree.get(block_id);
    let mut state = entry_state.clone();

    // update state per instruction
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        let Some(destination) = instruction.destination() else {
            continue;
        };
        let Some(destination) = destination.value() else {
            continue;
        };

        if let Some(constant) =
            constant_for_instruction(instruction, tree, &state, pointer_width_bits)
        {
            state.insert(destination, constant);
        } else {
            state.remove(destination);
        }
    }

    state
}

/// Evaluate a constant for an instruction when possible.
fn constant_for_instruction(
    instruction: &mir::Instruction,
    tree: &mir::Tree,
    state: &ConstantMap,
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
                to_type.ty()?,
                pointer_width_bits,
                tree,
            )
        }
        _ => None,
    }
}

/// Return a shared empty constant map.
fn empty_map() -> &'static ConstantMap {
    // init shared empty map
    static EMPTY: std::sync::OnceLock<ConstantMap> = std::sync::OnceLock::new();
    EMPTY.get_or_init(ConstantMap::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Readonly global loads are not represented as scalar constants.
    #[test]
    fn test_readonly_global_load_not_constant() {
        let test = TestProgram::new(
            r#"
global flag: boolean, readonly = true
function test(): boolean {
b0:
    v0: ref<boolean, raw, readonly> = global.address flag
    v1: boolean = load v0
    return v1
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<ConstantPropagation>();

        let block0 = function.blocks[0];
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
b0:
    v0: ref<boolean, raw> = global.address flag
    v1: boolean = load v0
    return v1
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<ConstantPropagation>();

        let block0 = function.blocks[0];
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
global flag: boolean, readonly = zeroInit
function test(): boolean {
b0:
    v0: ref<boolean, raw, readonly> = global.address flag
    v1: boolean = load v0
    return v1
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<ConstantPropagation>();

        let block0 = function.blocks[0];
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
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = int.add v0, v1
    return v2
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<ConstantPropagation>();

        let block0 = function.blocks[0];
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
b0(v0: boolean):
    v1: boolean = true
    branch v0, b1(v1), b2(v1)
b1(v2: boolean):
    jump b3(v2)
b2(v3: boolean):
    jump b3(v3)
b3(v4: boolean):
    return v4
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<ConstantPropagation>();

        let block3 = function.blocks[3];
        let constant = analysis
            .constant_at_entry(block3, mir::Value::new(4))
            .cloned();
        assert_eq!(constant, Some(mir::Constant::Boolean { value: true }));
    }

    /// Block parameters are not constant when predecessors disagree.
    #[test]
    fn test_conflicting_block_param() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): boolean {
b0(v0: boolean):
    v1: boolean = true
    v2: boolean = false
    branch v0, b1(v1), b2(v2)
b1(v3: boolean):
    jump b3(v3)
b2(v4: boolean):
    jump b3(v4)
b3(v5: boolean):
    return v5
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<ConstantPropagation>();

        let block3 = function.blocks[3];
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
b0(v0: boolean):
    v1: boolean = true
    v2: boolean = false
    branch v0, b1(v1), b1(v2)
b1(v3: boolean):
    return v3
}"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let analysis = analyses.get::<ConstantPropagation>();

        let block1 = function.blocks[1];
        let constant = analysis
            .constant_at_entry(block1, mir::Value::new(3))
            .cloned();
        assert_eq!(constant, None);
    }
}
