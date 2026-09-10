use crate as mir;
use std::collections::VecDeque;

use crate::{Analysis, ControlTable, DefinitionTable, Lattice, NodeTable, TargetLayout};
use destack_core::{FxIndexMap, float_from_bits, float_to_bits};

// TODO #Incomplete: replace propagation with SCCP after migrating alias and access queries

/// Constant propagation for one function.
#[derive(Debug)]
pub struct ConstantTable {
    /// Constants known at their SSA definitions.
    values: Vec<Option<mir::Constant>>,
    /// Constants available at block entry indexed by block id.
    block_entry: NodeTable<mir::Block, Option<ConstantState>>,
    /// Constants available at block exit indexed by block id.
    block_exit: NodeTable<mir::Block, Option<ConstantState>>,
}

/// Mapping from SSA values to known constants.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConstantState {
    /// Known constant values.
    constants: FxIndexMap<mir::Value, mir::Constant>,
}

impl ConstantState {
    /// Create an empty constant map.
    pub fn new() -> Self {
        Self {
            constants: FxIndexMap::default(),
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

        self.constants.shift_remove(&value);
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
        let mut constants = FxIndexMap::default();

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
    pub fn analyse(
        function: &mir::Function,
        cfg: &ControlTable,
        target_layout: TargetLayout,
        tree: &mir::Tree,
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
                    values: vec![None; function.value_capacity()],
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
                for pred in cfg.predecessors(block_id) {
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

        let values = Self::collect_values(function, tree, &block_entry, &block_exit);

        Self {
            values,
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

    /// Return the constant known for one SSA value.
    pub fn constant(&self, value: mir::Value) -> Option<&mir::Constant> {
        self.values
            .get(value.id() as usize)
            .and_then(Option::as_ref)
    }

    /// Build constant propagation with constant parameters seeded at entry.
    pub fn with_parameter_constants(
        function: &mir::Function,
        tree: &mir::Tree,
        target_layout: TargetLayout,
        param_constants: &FxIndexMap<mir::Value, mir::Constant>,
    ) -> Self {
        // build a control flow graph for the function
        let cfg = ControlTable::analyse(function, tree);

        // seed entry constants from the provided parameter map
        let mut entry_constants = ConstantState::new();
        for (value, constant) in param_constants {
            entry_constants.insert(*value, constant.clone());
        }

        Self::build_with_entry_constants(function, tree, &cfg, target_layout, entry_constants)
    }

    /// Collect constants at their canonical SSA definitions.
    fn collect_values(
        function: &mir::Function,
        tree: &mir::Tree,
        block_entry: &NodeTable<mir::Block, Option<ConstantState>>,
        block_exit: &NodeTable<mir::Block, Option<ConstantState>>,
    ) -> Vec<Option<mir::Constant>> {
        let mut values = vec![None; function.value_capacity()];

        // retain seeded function parameters
        if let Some(entry) = function.entry()
            && let Some(constants) = block_entry.get(entry)
        {
            for parameter in &function.parameters {
                values[parameter.value.id() as usize] = constants.get(parameter.value).cloned();
            }
        }

        // retain block parameters and instruction results
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            if let Some(constants) = block_entry.get(block_id) {
                for parameter in &block.parameters {
                    values[parameter.value.id() as usize] = constants.get(parameter.value).cloned();
                }
            }

            let Some(constants) = block_exit.get(block_id) else {
                continue;
            };
            for &instruction_id in &block.instructions {
                let Some(destination) = tree.get(instruction_id).destination() else {
                    continue;
                };
                values[destination.id() as usize] = constants.get(destination).cloned();
            }
        }

        values
    }
}

impl Analysis for ConstantTable {
    const INVALIDATED_BY: mir::Mutation = mir::Mutation::CONTROL
        .union(mir::Mutation::VALUE)
        .union(mir::Mutation::LAYOUT);
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
    ) -> FxIndexMap<mir::Value, mir::Constant> {
        // early exit for blocks without parameters
        let block = tree.get(block_id);
        if block.parameters.is_empty() {
            return FxIndexMap::default();
        }

        // track parameter states across predecessors
        let mut states = vec![ParamState::Unseen; block.parameters.len()];
        let mut is_seen = false;

        // scan predecessors
        for pred in cfg.predecessors(block_id) {
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
            return FxIndexMap::default();
        }

        // collect constants for parameters
        let mut constants = FxIndexMap::default();
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

/// mir::Constant type information for literal values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantType {
    /// The open type of a value parameter.
    Parameter,
    /// Null pointer constant type.
    Null,
    /// Boolean constant type.
    Boolean,
    /// Integer constant type.
    Int { width: u16, signed: bool },
    /// Floating point constant type.
    Float { format: mir::FloatType },
    /// Character constant type.
    Char,
    /// Untyped storage constant type.
    Storage,
    /// Layout measure constant type.
    Layout,
    /// The type of an associated const read through a witness.
    Witness,
}

/// Lookup interface for constant maps.
pub trait ConstantLookup {
    /// Return the constant value for a MIR value when known.
    fn get_constant(&self, value: mir::Value) -> Option<&mir::Constant>;
}

impl ConstantLookup for FxIndexMap<mir::Value, mir::Constant> {
    /// Return the constant value for a MIR value when known.
    fn get_constant(&self, value: mir::Value) -> Option<&mir::Constant> {
        self.get(&value)
    }
}

/// Return the constant type for a MIR constant.
pub fn constant_type_of(constant: &mir::Constant) -> ConstantType {
    match constant {
        mir::Constant::Parameter(_) => ConstantType::Parameter,
        mir::Constant::Null => ConstantType::Null,
        mir::Constant::Boolean { .. } => ConstantType::Boolean,
        mir::Constant::Int {
            width, is_signed, ..
        } => ConstantType::Int {
            width: *width,
            signed: *is_signed,
        },
        mir::Constant::UInt { width, .. } => ConstantType::Int {
            width: *width,
            signed: false,
        },
        mir::Constant::Float { format, .. } => ConstantType::Float { format: *format },
        mir::Constant::Char { .. } => ConstantType::Char,
        mir::Constant::Uninit | mir::Constant::Zeroed => ConstantType::Storage,
        mir::Constant::Layout { .. } => ConstantType::Layout,
        mir::Constant::Witness { .. } => ConstantType::Witness,
    }
}

/// Check whether a constant type matches a MIR type.
pub fn constant_matches_type(
    constant_type: ConstantType,
    destination_type: impl Into<mir::TypeId>,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> bool {
    let destination_type = destination_type.into();

    match (constant_type, tree.get(destination_type)) {
        (ConstantType::Parameter, _) => true,
        (ConstantType::Null, mir::Type::Pointer { .. }) => true,
        (ConstantType::Boolean, mir::Type::Boolean) => true,
        (ConstantType::Int { width, signed }, ty) => {
            let Some((ty_width, ty_signed)) = ty.int_info_with_pointer_width(pointer_width_bits)
            else {
                return false;
            };
            width == ty_width && signed == ty_signed
        }
        (ConstantType::Float { format }, mir::Type::Float(float_type)) => format == *float_type,
        (ConstantType::Char, mir::Type::Character) => true,
        _ => false,
    }
}

/// Extract a constant value from the given operand.
pub fn constant_for_value(
    value: mir::Value,
    definitions: &DefinitionTable,
    tree: &mir::Tree,
) -> Option<mir::Constant> {
    // find the instruction that defines the value
    let inst_id = definitions.instruction(value)?;
    let inst = tree.get(inst_id);

    // extract constants from direct constant instructions
    match inst {
        mir::Instruction::Const { value, .. } => Some(value.clone()),
        _ => None,
    }
}

/// Resolve constant arguments for a parameter list.
pub fn constant_arguments_for_parameters(
    arguments: &[mir::Value],
    parameters: &[impl mir::TypedParameter],
    constants: &impl ConstantLookup,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> Option<Vec<Option<mir::Constant>>> {
    // ensure argument and parameter counts match
    if arguments.len() != parameters.len() {
        return None;
    }

    // collect constant arguments in order
    let mut resolved = Vec::with_capacity(arguments.len());
    for (argument, parameter) in arguments.iter().zip(parameters.iter()) {
        let argument = *argument;
        let constant = constants.get_constant(argument).and_then(|constant| {
            let constant_type = constant_type_of(constant);
            if constant_matches_type(constant_type, parameter.ty(), pointer_width_bits, tree) {
                Some(constant.clone())
            } else {
                None
            }
        });
        resolved.push(constant);
    }

    Some(resolved)
}

/// Check if a constant is zero.
pub fn constant_is_zero(constant: Option<&mir::Constant>) -> bool {
    matches!(
        constant,
        Some(mir::Constant::Int { value: 0, .. })
            | Some(mir::Constant::UInt { value: 0, .. })
            | Some(mir::Constant::Float { bits: 0, .. })
            | Some(mir::Constant::Boolean { value: false })
    )
}

/// Check if a constant is one.
pub fn constant_is_one(constant: Option<&mir::Constant>) -> bool {
    matches!(
        constant,
        Some(mir::Constant::Int { value: 1, .. })
            | Some(mir::Constant::UInt { value: 1, .. })
            | Some(mir::Constant::Boolean { value: true })
    )
}

/// Check if a constant has all bits set (i.e., -1 for signed, max for unsigned).
pub fn constant_is_all_ones(constant: Option<&mir::Constant>) -> bool {
    match constant {
        Some(mir::Constant::Int { value: -1, .. }) => true,
        Some(mir::Constant::UInt { value, width }) => {
            let mask = mask_to_width(u128::MAX, *width);
            *value == mask
        }
        Some(mir::Constant::Boolean { value: true }) => true,
        _ => false,
    }
}

/// Check if a float constant is positive zero.
pub fn constant_is_float_zero(constant: Option<&mir::Constant>) -> bool {
    matches!(constant, Some(mir::Constant::Float { bits: 0, .. }))
}

/// Check if a float constant is one.
pub fn constant_is_float_one(constant: Option<&mir::Constant>) -> bool {
    match constant {
        Some(mir::Constant::Float { bits, format }) => {
            float_from_bits(format.format(), *bits) == 1.0
        }
        _ => false,
    }
}

/// Create a zero constant matching the given constant's type.
pub fn constant_zero_like(template: &mir::Constant) -> mir::Constant {
    match template {
        mir::Constant::Int {
            width, is_signed, ..
        } => mir::Constant::Int {
            value: 0,
            width: *width,
            is_signed: *is_signed,
        },
        mir::Constant::UInt { width, .. } => mir::Constant::UInt {
            value: 0,
            width: *width,
        },
        mir::Constant::Float { format, .. } => mir::Constant::Float {
            bits: 0,
            format: *format,
        },
        mir::Constant::Boolean { .. } => mir::Constant::Boolean { value: false },
        _ => mir::Constant::Int {
            value: 0,
            width: 32,
            is_signed: true,
        },
    }
}

/// Build a zero constant for a scalar type.
pub fn constant_zero_for_type(ty: &mir::Type, pointer_width_bits: u16) -> Option<mir::Constant> {
    // handle integer types
    let Some((width, signed)) = ty.int_info_with_pointer_width(pointer_width_bits) else {
        // handle non integer scalar types
        return match ty {
            mir::Type::Float(float_type) => Some(mir::Constant::Float {
                bits: 0,
                format: *float_type,
            }),
            mir::Type::Boolean => Some(mir::Constant::Boolean { value: false }),
            _ => None,
        };
    };

    // build integer zero with correct signedness
    if signed {
        Some(mir::Constant::Int {
            value: 0,
            width,
            is_signed: true,
        })
    } else {
        Some(mir::Constant::UInt { value: 0, width })
    }
}

/// Create an all ones constant matching the given constant's type.
pub fn constant_all_ones_like(template: &mir::Constant) -> mir::Constant {
    match template {
        mir::Constant::Int {
            width, is_signed, ..
        } => mir::Constant::Int {
            value: -1,
            width: *width,
            is_signed: *is_signed,
        },
        mir::Constant::UInt { width, .. } => mir::Constant::UInt {
            value: mask_to_width(u128::MAX, *width),
            width: *width,
        },
        mir::Constant::Boolean { .. } => mir::Constant::Boolean { value: true },
        _ => mir::Constant::Int {
            value: -1,
            width: 32,
            is_signed: true,
        },
    }
}

/// Read a scalar constant from an immutable global.
pub fn constant_from_global(
    global: impl Into<mir::GlobalId>,
    tree: &mir::Tree,
) -> Option<mir::Constant> {
    let global = global.into();

    // read global definition
    let global = tree.get(global);
    if global.is_mutable() {
        return None;
    }

    // allow scalar initializers only
    match global.initializer.as_ref()? {
        mir::GlobalInitializer::Scalar(constant) => Some(constant.clone()),
        _ => None,
    }
}

/// Aggregate constant tree.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantTree {
    /// Scalar constant value.
    Scalar(mir::Constant),
    /// Aggregate constants in order.
    Aggregate(Vec<ConstantTree>),
    /// Unknown or unsupported constant value.
    Unknown,
}

/// Read a constant tree from an immutable global initializer.
pub fn constant_tree_from_global(
    global: impl Into<mir::GlobalId>,
    tree: &mir::Tree,
    max_aggregate_elements: usize,
    pointer_width_bits: u16,
) -> Option<ConstantTree> {
    let global = global.into();

    // read global definition
    let global = tree.get(global);
    if global.is_mutable() {
        return None;
    }

    // read initializer
    let initializer = global.initializer.as_ref()?;

    // build constant tree
    Some(constant_tree_from_initializer(
        initializer,
        global.ty,
        tree,
        max_aggregate_elements,
        pointer_width_bits,
    ))
}

/// Fold a pure intrinsic with constant arguments.
pub fn fold_intrinsic(
    intrinsic: mir::Intrinsic,
    arguments: &[mir::Constant],
) -> Option<mir::Constant> {
    // reject empty argument lists
    let first = arguments.first()?;

    // fold integer unary intrinsics
    let folded_integer_unary = match intrinsic {
        mir::Intrinsic::LeadingZeroCount => fold_int_unary(first, |value, width| {
            let leading = value.leading_zeros();
            let adjust = u32::from(128u16.saturating_sub(width));
            Some((leading - adjust) as u128)
        }),
        mir::Intrinsic::TrailingZeroCount => {
            fold_int_unary(first, |value, _width| Some(value.trailing_zeros() as u128))
        }
        mir::Intrinsic::PopulationCount => {
            fold_int_unary(first, |value, _width| Some(value.count_ones() as u128))
        }
        mir::Intrinsic::ByteSwap => fold_int_unary(first, |value, width| {
            if width % 8 != 0 {
                return None;
            }
            let swapped = value.swap_bytes();
            Some(mask_to_width(swapped, width))
        }),
        mir::Intrinsic::BitReverse => fold_int_unary(first, |value, width| {
            let reversed = value.reverse_bits();
            let shifted = reversed >> 128u16.saturating_sub(width);
            Some(mask_to_width(shifted, width))
        }),
        _ => None,
    };
    if folded_integer_unary.is_some() {
        return folded_integer_unary;
    }

    // fold integer binary intrinsics
    let folded_integer_binary = match intrinsic {
        mir::Intrinsic::RotateLeft => fold_int_binary(arguments, |value, shift, width| {
            let shift = (shift % u128::from(width)) as u32;
            let rotated = value.rotate_left(shift);
            Some(mask_to_width(rotated, width))
        }),
        mir::Intrinsic::RotateRight => fold_int_binary(arguments, |value, shift, width| {
            let shift = (shift % u128::from(width)) as u32;
            let rotated = value.rotate_right(shift);
            Some(mask_to_width(rotated, width))
        }),
        _ => None,
    };
    if folded_integer_binary.is_some() {
        return folded_integer_binary;
    }

    // fold float unary intrinsics
    let folded_float_unary = match intrinsic {
        mir::Intrinsic::Sqrt => fold_float_unary(first, |value| value.sqrt()),
        mir::Intrinsic::Cbrt => fold_float_unary(first, |value| value.cbrt()),
        mir::Intrinsic::Abs => fold_float_unary(first, |value| value.abs()),
        mir::Intrinsic::Sin => fold_float_unary(first, |value| value.sin()),
        mir::Intrinsic::Cos => fold_float_unary(first, |value| value.cos()),
        mir::Intrinsic::Tan => fold_float_unary(first, |value| value.tan()),
        mir::Intrinsic::Asin => fold_float_unary(first, |value| value.asin()),
        mir::Intrinsic::Acos => fold_float_unary(first, |value| value.acos()),
        mir::Intrinsic::Atan => fold_float_unary(first, |value| value.atan()),
        mir::Intrinsic::Exp => fold_float_unary(first, |value| value.exp()),
        mir::Intrinsic::Expm1 => fold_float_unary(first, |value| value.exp_m1()),
        mir::Intrinsic::Exp2 => fold_float_unary(first, |value| value.exp2()),
        mir::Intrinsic::Log => fold_float_unary(first, |value| value.ln()),
        mir::Intrinsic::Log1p => fold_float_unary(first, |value| value.ln_1p()),
        mir::Intrinsic::Log2 => fold_float_unary(first, |value| value.log2()),
        mir::Intrinsic::Log10 => fold_float_unary(first, |value| value.log10()),
        mir::Intrinsic::Floor => fold_float_unary(first, |value| value.floor()),
        mir::Intrinsic::Ceil => fold_float_unary(first, |value| value.ceil()),
        mir::Intrinsic::Trunc => fold_float_unary(first, |value| value.trunc()),
        mir::Intrinsic::Round => fold_float_unary(first, round_ties_positive_infinity),
        mir::Intrinsic::RoundTiesEven => fold_float_unary(first, |value| value.round_ties_even()),
        mir::Intrinsic::RoundTiesAway => fold_float_unary(first, |value| value.round()),
        _ => None,
    };
    if folded_float_unary.is_some() {
        return folded_float_unary;
    }

    // fold float binary and ternary intrinsics
    match intrinsic {
        mir::Intrinsic::CopySign => {
            fold_float_binary(arguments, |left, right| left.copysign(right))
        }
        mir::Intrinsic::Atan2 => fold_float_binary(arguments, |left, right| left.atan2(right)),
        mir::Intrinsic::Pow => fold_float_binary(arguments, |left, right| left.powf(right)),
        mir::Intrinsic::Fma => fold_float_ternary(arguments, |a, b, c| a.mul_add(b, c)),
        _ => None,
    }
}

/// Round one binary64 value to the nearest integer with ties toward positive infinity.
fn round_ties_positive_infinity(value: f64) -> f64 {
    let lower = value.floor();
    let distance = value - lower;
    let rounded = if distance < 0.5 { lower } else { lower + 1.0 };

    rounded.copysign(value)
}

/// Fold an integer unary intrinsic when possible.
fn fold_int_unary(
    constant: &mir::Constant,
    f: impl FnOnce(u128, u16) -> Option<u128>,
) -> Option<mir::Constant> {
    // decode the constant payload
    let (value, width, is_signed) = decode_int_constant(constant)?;

    // apply the operation
    let folded = f(value, width)?;

    // reencode with the original signedness
    Some(encode_int_constant(folded, width, is_signed))
}

/// Fold an integer binary intrinsic when possible.
fn fold_int_binary(
    arguments: &[mir::Constant],
    f: impl FnOnce(u128, u128, u16) -> Option<u128>,
) -> Option<mir::Constant> {
    // expect exactly two operands
    let left = arguments.first()?;
    let right = arguments.get(1)?;

    // decode operand payloads
    let (left_value, width, is_signed) = decode_int_constant(left)?;
    let (right_value, right_width, _) = decode_int_constant(right)?;
    if width != right_width {
        return None;
    }

    // apply the operation
    let folded = f(left_value, right_value, width)?;

    // reencode with the original signedness
    Some(encode_int_constant(folded, width, is_signed))
}

/// Fold a float unary intrinsic.
fn fold_float_unary(constant: &mir::Constant, f: impl FnOnce(f64) -> f64) -> Option<mir::Constant> {
    // decode float constant
    let (value, width) = decode_float_constant(constant)?;

    // apply the operation
    let folded = f(value);

    // reencode in the original width
    Some(encode_float_constant(folded, width))
}

/// Fold a float binary intrinsic.
fn fold_float_binary(
    arguments: &[mir::Constant],
    f: impl FnOnce(f64, f64) -> f64,
) -> Option<mir::Constant> {
    // expect exactly two operands
    let left = arguments.first()?;
    let right = arguments.get(1)?;

    // decode operand payloads
    let (left_value, width) = decode_float_constant(left)?;
    let (right_value, right_width) = decode_float_constant(right)?;
    if width != right_width {
        return None;
    }

    // apply the operation
    let folded = f(left_value, right_value);

    // reencode in the original width
    Some(encode_float_constant(folded, width))
}

/// Fold a float ternary intrinsic.
fn fold_float_ternary(
    arguments: &[mir::Constant],
    f: impl FnOnce(f64, f64, f64) -> f64,
) -> Option<mir::Constant> {
    // expect exactly three operands
    let first = arguments.first()?;
    let second = arguments.get(1)?;
    let third = arguments.get(2)?;

    // decode operand payloads
    let (first_value, width) = decode_float_constant(first)?;
    let (second_value, second_width) = decode_float_constant(second)?;
    let (third_value, third_width) = decode_float_constant(third)?;
    if width != second_width || width != third_width {
        return None;
    }

    // apply the operation
    let folded = f(first_value, second_value, third_value);

    // reencode in the original width
    Some(encode_float_constant(folded, width))
}

/// Decode an integer constant into a masked payload.
fn decode_int_constant(constant: &mir::Constant) -> Option<(u128, u16, bool)> {
    match constant {
        mir::Constant::Int {
            value,
            width,
            is_signed,
        } => {
            let masked = mask_to_width(*value as u128, *width);
            Some((masked, *width, *is_signed))
        }
        mir::Constant::UInt { value, width } => Some((*value, *width, false)),
        _ => None,
    }
}

/// Encode a masked integer payload into a constant.
fn encode_int_constant(value: u128, width: u16, is_signed: bool) -> mir::Constant {
    // sign extend when needed
    if is_signed {
        let signed = signed_from_bits(value, width);
        mir::Constant::Int {
            value: signed,
            width,
            is_signed,
        }
    } else {
        mir::Constant::UInt { value, width }
    }
}

/// Decode a float constant into f64 payload and width.
fn decode_float_constant(constant: &mir::Constant) -> Option<(f64, mir::FloatType)> {
    match constant {
        mir::Constant::Float { bits, format } => {
            Some((float_from_bits(format.format(), *bits), *format))
        }
        _ => None,
    }
}

/// Encode a float payload back into a constant.
fn encode_float_constant(value: f64, format: mir::FloatType) -> mir::Constant {
    mir::Constant::Float {
        bits: float_to_bits(format.format(), value),
        format,
    }
}

/// Mask an integer payload down to the specified width.
fn mask_to_width(value: u128, width: u16) -> u128 {
    if width >= 128 {
        value
    } else {
        let mask = (1u128 << width) - 1;
        value & mask
    }
}

/// Sign extend a masked integer payload.
fn signed_from_bits(value: u128, width: u16) -> i128 {
    if width >= 128 {
        return value as i128;
    }

    let mask = (1u128 << width) - 1;
    let masked = value & mask;
    let sign_bit = 1u128 << (width - 1);

    if masked & sign_bit != 0 {
        (masked | !mask) as i128
    } else {
        masked as i128
    }
}

/// Build a constant tree from a global initializer.
fn constant_tree_from_initializer(
    initializer: &mir::GlobalInitializer,
    ty: impl Into<mir::TypeId>,
    tree: &mir::Tree,
    max_aggregate_elements: usize,
    pointer_width_bits: u16,
) -> ConstantTree {
    let ty = ty.into();

    // map initializer kind
    match initializer {
        mir::GlobalInitializer::Zero => {
            constant_tree_from_zero(ty, tree, max_aggregate_elements, pointer_width_bits)
        }
        mir::GlobalInitializer::Scalar(constant) => constant_tree_from_scalar(constant, ty, tree),
        mir::GlobalInitializer::FunctionAddress(_) => ConstantTree::Unknown,
        mir::GlobalInitializer::GlobalAddress(_) => ConstantTree::Unknown,
        mir::GlobalInitializer::String(_) => ConstantTree::Unknown,
        mir::GlobalInitializer::BigInt(_) => ConstantTree::Unknown,
        mir::GlobalInitializer::Bytes(bytes) => {
            constant_tree_from_bytes(bytes, ty, tree, max_aggregate_elements)
        }
        mir::GlobalInitializer::Aggregate(elements) => constant_tree_from_aggregate_initializer(
            elements,
            ty,
            tree,
            max_aggregate_elements,
            pointer_width_bits,
        ),
    }
}

/// Build a scalar constant tree when the type is compatible.
fn constant_tree_from_scalar(
    constant: &mir::Constant,
    ty: impl Into<mir::TypeId>,
    tree: &mir::Tree,
) -> ConstantTree {
    let ty = ty.into();

    // read type
    let ty = tree.get(ty);

    if let mir::Type::Newtype { inner, .. } = ty {
        return constant_tree_from_scalar(constant, *inner, tree);
    }

    // accept scalar types only
    if ty.is_scalar() {
        return ConstantTree::Scalar(constant.clone());
    }

    ConstantTree::Unknown
}

/// Build a zero constant tree for the given type.
fn constant_tree_from_zero(
    ty: impl Into<mir::TypeId>,
    tree: &mir::Tree,
    max_aggregate_elements: usize,
    pointer_width_bits: u16,
) -> ConstantTree {
    let ty = ty.into();

    // read type
    let ty = tree.get(ty);

    // build zero constants by type
    match ty {
        mir::Type::Boolean => ConstantTree::Scalar(mir::Constant::Boolean { value: false }),
        mir::Type::Int {
            width,
            is_signed: signed,
        } => {
            if *signed {
                ConstantTree::Scalar(mir::Constant::Int {
                    value: 0,
                    width: *width,
                    is_signed: true,
                })
            } else {
                ConstantTree::Scalar(mir::Constant::UInt {
                    value: 0,
                    width: *width,
                })
            }
        }
        mir::Type::Isize => ConstantTree::Scalar(mir::Constant::Int {
            value: 0,
            width: pointer_width_bits,
            is_signed: true,
        }),
        mir::Type::Usize => ConstantTree::Scalar(mir::Constant::UInt {
            value: 0,
            width: pointer_width_bits,
        }),
        mir::Type::Float(float_type) => ConstantTree::Scalar(mir::Constant::Float {
            bits: 0,
            format: *float_type,
        }),
        mir::Type::Newtype { inner, .. } => {
            constant_tree_from_zero(*inner, tree, max_aggregate_elements, pointer_width_bits)
        }
        mir::Type::FixedArray {
            element, length, ..
        } => {
            let length = match tree
                .static_value(*length)
                .length()
                .and_then(|length| usize::try_from(length).ok())
            {
                Some(length) => length,
                None => return ConstantTree::Unknown,
            };

            if length > max_aggregate_elements {
                return ConstantTree::Unknown;
            }

            let element_value =
                constant_tree_from_zero(*element, tree, max_aggregate_elements, pointer_width_bits);
            let elements = (0..length).map(|_| element_value.clone()).collect();
            ConstantTree::Aggregate(elements)
        }
        mir::Type::Tuple { elements, .. } => {
            let elements = elements
                .iter()
                .map(|element| {
                    constant_tree_from_zero(
                        *element,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(elements)
        }
        mir::Type::Struct { fields, .. } => {
            let elements = fields
                .iter()
                .map(|field| tree.get(*field).ty)
                .map(|field_ty| {
                    constant_tree_from_zero(
                        field_ty,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(elements)
        }
        _ => ConstantTree::Unknown,
    }
}

/// Build a constant tree from a byte initializer when possible.
fn constant_tree_from_bytes(
    bytes: &[u8],
    ty: impl Into<mir::TypeId>,
    tree: &mir::Tree,
    max_aggregate_elements: usize,
) -> ConstantTree {
    let ty = ty.into();

    // read fixed array type
    let mir::Type::FixedArray {
        element, length, ..
    } = tree.get(ty)
    else {
        return ConstantTree::Unknown;
    };

    // check length constraints
    let length = match tree
        .static_value(*length)
        .length()
        .and_then(|length| usize::try_from(length).ok())
    {
        Some(length) => length,
        None => return ConstantTree::Unknown,
    };

    if length != bytes.len() || length > max_aggregate_elements {
        return ConstantTree::Unknown;
    }

    // require 8 bit integer element type
    let element = *element;

    let mir::Type::Int {
        width,
        is_signed: signed,
    } = tree.get(element)
    else {
        return ConstantTree::Unknown;
    };

    if *width != 8 {
        return ConstantTree::Unknown;
    }

    // build per byte constants
    let elements = if *signed {
        bytes
            .iter()
            .map(|byte| {
                let value = i8::from_ne_bytes([*byte]) as i128;
                ConstantTree::Scalar(mir::Constant::Int {
                    value,
                    width: 8,
                    is_signed: true,
                })
            })
            .collect()
    } else {
        bytes
            .iter()
            .map(|byte| {
                ConstantTree::Scalar(mir::Constant::UInt {
                    value: u128::from(*byte),
                    width: 8,
                })
            })
            .collect()
    };

    ConstantTree::Aggregate(elements)
}

/// Build a constant tree from an aggregate initializer.
fn constant_tree_from_aggregate_initializer(
    elements: &[mir::GlobalInitializer],
    ty: impl Into<mir::TypeId>,
    tree: &mir::Tree,
    max_aggregate_elements: usize,
    pointer_width_bits: u16,
) -> ConstantTree {
    let ty = ty.into();

    // map aggregate initializer to type shape
    match tree.get(ty) {
        mir::Type::FixedArray {
            element, length, ..
        } => {
            let length = match tree
                .static_value(*length)
                .length()
                .and_then(|length| usize::try_from(length).ok())
            {
                Some(length) => length,
                None => return ConstantTree::Unknown,
            };

            if length != elements.len() || length > max_aggregate_elements {
                return ConstantTree::Unknown;
            }

            let values = elements
                .iter()
                .map(|element_init| {
                    constant_tree_from_initializer(
                        element_init,
                        *element,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(values)
        }
        mir::Type::Tuple {
            elements: element_types,
            ..
        } => {
            if element_types.len() != elements.len() {
                return ConstantTree::Unknown;
            }

            let values = elements
                .iter()
                .zip(element_types)
                .map(|(element_init, element_ty)| {
                    constant_tree_from_initializer(
                        element_init,
                        *element_ty,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(values)
        }
        mir::Type::Struct { fields, .. } => {
            if fields.len() != elements.len() {
                return ConstantTree::Unknown;
            }

            let values = elements
                .iter()
                .zip(fields)
                .map(|(element_init, field)| {
                    constant_tree_from_initializer(
                        element_init,
                        tree.get(*field).ty,
                        tree,
                        max_aggregate_elements,
                        pointer_width_bits,
                    )
                })
                .collect();
            ConstantTree::Aggregate(values)
        }
        _ => ConstantTree::Unknown,
    }
}

/// Try to fold a binary operation on constants.
pub fn fold_binary(
    operator: mir::BinaryOperator,
    left: mir::Constant,
    right: mir::Constant,
) -> Option<mir::Constant> {
    match (&left, &right) {
        (
            mir::Constant::Int {
                value: l,
                width: lw,
                is_signed: true,
            },
            mir::Constant::Int {
                value: r,
                width: rw,
                is_signed: true,
            },
        ) if lw == rw => fold_binary_signed(*l, *r, *lw, operator),

        (
            mir::Constant::UInt {
                value: l,
                width: lw,
            },
            mir::Constant::UInt {
                value: r,
                width: rw,
            },
        ) if lw == rw => fold_binary_unsigned(*l, *r, *lw, operator),

        (
            mir::Constant::Float {
                bits: lb,
                format: left_format,
            },
            mir::Constant::Float {
                bits: rb,
                format: right_format,
            },
        ) if left_format == right_format => fold_binary_float(*lb, *rb, *left_format, operator),

        (mir::Constant::Boolean { value: l }, mir::Constant::Boolean { value: r }) => {
            fold_binary_bool(*l, *r, operator)
        }

        _ => None,
    }
}

/// Fold a binary operation on signed integers.
pub fn fold_binary_signed(
    left: i128,
    right: i128,
    width: u16,
    operator: mir::BinaryOperator,
) -> Option<mir::Constant> {
    let result_int = |value: i128| {
        Some(mir::Constant::Int {
            value: truncate_signed(value, width),
            width,
            is_signed: true,
        })
    };
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    match operator {
        mir::BinaryOperator::Add => result_int(left.wrapping_add(right)),
        mir::BinaryOperator::Subtract => result_int(left.wrapping_sub(right)),
        mir::BinaryOperator::Multiply => result_int(left.wrapping_mul(right)),
        mir::BinaryOperator::Divide => {
            if right != 0 {
                result_int(left.wrapping_div(right))
            } else {
                None
            }
        }
        mir::BinaryOperator::Remainder => {
            if right != 0 {
                result_int(left.wrapping_rem(right))
            } else {
                None
            }
        }
        mir::BinaryOperator::And => result_int(left & right),
        mir::BinaryOperator::Or => result_int(left | right),
        mir::BinaryOperator::Xor => result_int(left ^ right),
        mir::BinaryOperator::ShiftLeft => result_int(left.wrapping_shl(right as u32)),
        mir::BinaryOperator::ShiftRight => result_int(left.wrapping_shr(right as u32)),
        mir::BinaryOperator::Equal => result_bool(left == right),
        mir::BinaryOperator::NotEqual => result_bool(left != right),
        mir::BinaryOperator::LessThan => result_bool(left < right),
        mir::BinaryOperator::LessEqual => result_bool(left <= right),
        mir::BinaryOperator::GreaterThan => result_bool(left > right),
        mir::BinaryOperator::GreaterEqual => result_bool(left >= right),
        _ => None,
    }
}

/// Fold a binary operation on unsigned integers.
pub fn fold_binary_unsigned(
    left: u128,
    right: u128,
    width: u16,
    operator: mir::BinaryOperator,
) -> Option<mir::Constant> {
    let result_uint = |value: u128| {
        Some(mir::Constant::UInt {
            value: mask_to_width(value, width),
            width,
        })
    };
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    match operator {
        mir::BinaryOperator::Add => result_uint(left.wrapping_add(right)),
        mir::BinaryOperator::Subtract => result_uint(left.wrapping_sub(right)),
        mir::BinaryOperator::Multiply => result_uint(left.wrapping_mul(right)),
        mir::BinaryOperator::Divide => {
            if right != 0 {
                result_uint(left.wrapping_div(right))
            } else {
                None
            }
        }
        mir::BinaryOperator::Remainder => {
            if right != 0 {
                result_uint(left.wrapping_rem(right))
            } else {
                None
            }
        }
        mir::BinaryOperator::And => result_uint(left & right),
        mir::BinaryOperator::Or => result_uint(left | right),
        mir::BinaryOperator::Xor => result_uint(left ^ right),
        mir::BinaryOperator::ShiftLeft => result_uint(left.wrapping_shl(right as u32)),
        mir::BinaryOperator::ShiftRight | mir::BinaryOperator::UnsignedShiftRight => {
            result_uint(left.wrapping_shr(right as u32))
        }
        mir::BinaryOperator::Equal => result_bool(left == right),
        mir::BinaryOperator::NotEqual => result_bool(left != right),
        mir::BinaryOperator::LessThan => result_bool(left < right),
        mir::BinaryOperator::LessEqual => result_bool(left <= right),
        mir::BinaryOperator::GreaterThan => result_bool(left > right),
        mir::BinaryOperator::GreaterEqual => result_bool(left >= right),
    }
}

/// Fold a binary operation on floats.
pub fn fold_binary_float(
    left_bits: u64,
    right_bits: u64,
    format: mir::FloatType,
    operator: mir::BinaryOperator,
) -> Option<mir::Constant> {
    let result_float = |value: f64| {
        Some(mir::Constant::Float {
            bits: float_to_bits(format.format(), value),
            format,
        })
    };
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    if format == mir::FloatType::Float32 {
        let left = f32::from_bits(left_bits as u32);
        let right = f32::from_bits(right_bits as u32);

        return match operator {
            mir::BinaryOperator::Add => result_float((left + right) as f64),
            mir::BinaryOperator::Subtract => result_float((left - right) as f64),
            mir::BinaryOperator::Multiply => result_float((left * right) as f64),
            mir::BinaryOperator::Divide => result_float((left / right) as f64),
            mir::BinaryOperator::Remainder => result_float((left % right) as f64),
            mir::BinaryOperator::Equal => result_bool(left == right),
            mir::BinaryOperator::NotEqual => result_bool(left != right),
            mir::BinaryOperator::LessThan => result_bool(left < right),
            mir::BinaryOperator::LessEqual => result_bool(left <= right),
            mir::BinaryOperator::GreaterThan => result_bool(left > right),
            mir::BinaryOperator::GreaterEqual => result_bool(left >= right),
            _ => None,
        };
    }

    let left = float_from_bits(format.format(), left_bits);
    let right = float_from_bits(format.format(), right_bits);

    match operator {
        mir::BinaryOperator::Add => result_float(left + right),
        mir::BinaryOperator::Subtract => result_float(left - right),
        mir::BinaryOperator::Multiply => result_float(left * right),
        mir::BinaryOperator::Divide => result_float(left / right),
        mir::BinaryOperator::Remainder => result_float(left % right),
        mir::BinaryOperator::Equal => result_bool(left == right),
        mir::BinaryOperator::NotEqual => result_bool(left != right),
        mir::BinaryOperator::LessThan => result_bool(left < right),
        mir::BinaryOperator::LessEqual => result_bool(left <= right),
        mir::BinaryOperator::GreaterThan => result_bool(left > right),
        mir::BinaryOperator::GreaterEqual => result_bool(left >= right),
        _ => None,
    }
}

/// Fold a binary operation on booleans.
pub fn fold_binary_bool(
    left: bool,
    right: bool,
    operator: mir::BinaryOperator,
) -> Option<mir::Constant> {
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    match operator {
        mir::BinaryOperator::And => result_bool(left && right),
        mir::BinaryOperator::Or => result_bool(left || right),
        mir::BinaryOperator::Xor => result_bool(left ^ right),
        mir::BinaryOperator::Equal => result_bool(left == right),
        mir::BinaryOperator::NotEqual => result_bool(left != right),
        _ => None,
    }
}

/// Try to fold a cast operation on a constant.
pub fn fold_cast(
    operator: mir::CastOperator,
    value: mir::Constant,
    to_type: mir::LocalNodeId<mir::Type>,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> Option<mir::Constant> {
    // load target type
    let target_type = tree.get(to_type);

    // apply cast semantics
    match operator {
        mir::CastOperator::Bitcast => Some(value),

        mir::CastOperator::Truncate => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => return Some(value),
            };

            // truncate integer values
            match value {
                mir::Constant::Int { value, .. } => Some(mir::Constant::Int {
                    value: truncate_signed(value, target_width),
                    width: target_width,
                    is_signed: true,
                }),
                mir::Constant::UInt { value, .. } => Some(mir::Constant::UInt {
                    value: truncate_unsigned(value, target_width),
                    width: target_width,
                }),
                _ => Some(value),
            }
        }

        mir::CastOperator::Saturate => {
            // read target integer bounds
            let (target_width, target_signed) =
                match target_type.int_info_with_pointer_width(pointer_width_bits) {
                    Some(info) => info,
                    None => return Some(value),
                };
            let (min_bound, max_bound) = integer_bounds(target_width, target_signed)?;

            // clamp integer values
            match value {
                mir::Constant::Int { value, .. } => {
                    let value = value.clamp(min_bound, max_bound);
                    Some(integer_constant_from_i128(
                        value,
                        target_width,
                        target_signed,
                    ))
                }
                mir::Constant::UInt { value, .. } => {
                    let value = unsigned_as_saturating_i128(value, max_bound);
                    let value = value.clamp(min_bound, max_bound);
                    Some(integer_constant_from_i128(
                        value,
                        target_width,
                        target_signed,
                    ))
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::ZeroExtend => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => return Some(value),
            };

            // zero extend integer values
            match value {
                mir::Constant::UInt { value, .. } => Some(mir::Constant::UInt {
                    value,
                    width: target_width,
                }),
                mir::Constant::Int { value, width, .. } => {
                    let masked = truncate_unsigned(value as u128, width);
                    Some(mir::Constant::UInt {
                        value: masked,
                        width: target_width,
                    })
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::SignExtend => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => return Some(value),
            };

            // sign extend integer values
            match value {
                mir::Constant::Int { value, width, .. } => Some(mir::Constant::Int {
                    value: sign_extend(value, width, target_width),
                    width: target_width,
                    is_signed: true,
                }),
                mir::Constant::UInt { value, width } => {
                    let as_signed = signed_from_bits(value, width);
                    Some(mir::Constant::Int {
                        value: sign_extend(as_signed, width, target_width),
                        width: target_width,
                        is_signed: true,
                    })
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::FloatToSignedInt => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => 64,
            };

            // convert float to signed int
            match value {
                mir::Constant::Float { bits, format } => {
                    let value = float_from_bits(format.format(), bits);
                    let (min_bound, max_bound) = integer_bounds(target_width, true)?;
                    let converted = float_to_int_checked(value, min_bound, max_bound)?;
                    Some(mir::Constant::Int {
                        value: converted,
                        width: target_width,
                        is_signed: true,
                    })
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::FloatToUnsignedInt => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => 64,
            };

            // convert float to unsigned int
            match value {
                mir::Constant::Float { bits, format } => {
                    let value = float_from_bits(format.format(), bits);
                    let (min_bound, max_bound) = integer_bounds(target_width, false)?;
                    let converted = float_to_int_checked(value, min_bound, max_bound)?;
                    Some(mir::Constant::UInt {
                        value: converted as u128,
                        width: target_width,
                    })
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::FloatToSignedIntSaturating => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => 64,
            };

            // convert float to signed int with saturation
            match value {
                mir::Constant::Float { bits, format } => {
                    let value = float_from_bits(format.format(), bits);
                    let (min_bound, max_bound) = integer_bounds(target_width, true)?;
                    let converted = float_to_int_saturating(value, min_bound, max_bound);
                    Some(mir::Constant::Int {
                        value: converted,
                        width: target_width,
                        is_signed: true,
                    })
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::FloatToUnsignedIntSaturating => {
            // read target integer width
            let target_width = match target_type.int_info_with_pointer_width(pointer_width_bits) {
                Some((width, _)) => width,
                None => 64,
            };

            // convert float to unsigned int with saturation
            match value {
                mir::Constant::Float { bits, format } => {
                    let value = float_from_bits(format.format(), bits);
                    let (min_bound, max_bound) = integer_bounds(target_width, false)?;
                    let converted = float_to_int_saturating(value, min_bound, max_bound);
                    Some(mir::Constant::UInt {
                        value: converted as u128,
                        width: target_width,
                    })
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::SignedIntToFloat => {
            // read target float format
            let target_format = match target_type {
                mir::Type::Float(float_type) => *float_type,
                _ => mir::FloatType::Float64,
            };

            // convert signed int to float
            match value {
                mir::Constant::Int { value, .. } => Some(mir::Constant::Float {
                    bits: float_to_bits(target_format.format(), value as f64),
                    format: target_format,
                }),
                _ => Some(value),
            }
        }

        mir::CastOperator::UnsignedIntToFloat => {
            // read target float format
            let target_format = match target_type {
                mir::Type::Float(float_type) => *float_type,
                _ => mir::FloatType::Float64,
            };

            // convert unsigned int to float
            match value {
                mir::Constant::UInt { value, .. } => Some(mir::Constant::Float {
                    bits: float_to_bits(target_format.format(), value as f64),
                    format: target_format,
                }),
                _ => Some(value),
            }
        }

        mir::CastOperator::FloatTruncate => {
            // convert to target float format
            let target_format = match target_type {
                mir::Type::Float(float_type) => *float_type,
                _ => mir::FloatType::Float32,
            };
            match value {
                mir::Constant::Float { bits, format } => {
                    let value = float_from_bits(format.format(), bits);
                    Some(mir::Constant::Float {
                        bits: float_to_bits(target_format.format(), value),
                        format: target_format,
                    })
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::FloatExtend | mir::CastOperator::FloatConvert => {
            // convert to target float format
            let target_format = match target_type {
                mir::Type::Float(float_type) => *float_type,
                _ => mir::FloatType::Float64,
            };
            match value {
                mir::Constant::Float { bits, format } => {
                    let value = float_from_bits(format.format(), bits);
                    Some(mir::Constant::Float {
                        bits: float_to_bits(target_format.format(), value),
                        format: target_format,
                    })
                }
                _ => Some(value),
            }
        }

        mir::CastOperator::PointerToInt | mir::CastOperator::IntToPointer => None,
    }
}

/// Truncate a signed integer to a target bit width.
fn truncate_signed(value: i128, width: u16) -> i128 {
    // handle full width
    if width >= 128 {
        return value;
    }

    // build bit mask
    let mask = (1u128 << width) - 1;
    let masked = (value as u128) & mask;
    let sign_bit = 1u128 << (width - 1);

    // set sign extension when needed
    if masked & sign_bit != 0 {
        (masked | !mask) as i128
    }
    // otherwise keep masked value
    else {
        masked as i128
    }
}

/// Compute integer bounds for a width and signedness.
fn integer_bounds(width: u16, is_signed: bool) -> Option<(i128, i128)> {
    if width == 0 || width > 128 || (!is_signed && width == 128) {
        return None;
    }

    if is_signed {
        let shift = (width - 1) as u32;
        let min = -(1_i128 << shift);
        let max = (1_i128 << shift) - 1;
        Some((min, max))
    } else {
        let shift = width as u32;
        let max = (1_i128 << shift) - 1;
        Some((0, max))
    }
}

/// Convert a float to an integer when the conversion is in range and finite.
fn float_to_int_checked(value: f64, min_bound: i128, max_bound: i128) -> Option<i128> {
    if !value.is_finite() {
        return None;
    }

    let min_float = min_bound as f64;
    let max_float = max_bound as f64;
    let max_rounded = max_float.trunc() as i128;
    let max_is_rounded_up = max_rounded > max_bound;

    let min_ok = value >= min_float;
    let max_ok = if max_is_rounded_up {
        value < max_float
    } else {
        value <= max_float
    };

    if !min_ok || !max_ok {
        return None;
    }

    let truncated = value.trunc() as i128;
    if truncated < min_bound || truncated > max_bound {
        return None;
    }

    Some(truncated)
}

/// Convert a float to an integer with saturation.
fn float_to_int_saturating(value: f64, min_bound: i128, max_bound: i128) -> i128 {
    if value.is_nan() {
        return 0;
    }

    if !value.is_finite() {
        return if value.is_sign_negative() {
            min_bound
        } else {
            max_bound
        };
    }

    let min_float = min_bound as f64;
    let max_float = max_bound as f64;
    let max_rounded = max_float.trunc() as i128;
    let max_is_rounded_up = max_rounded > max_bound;

    if value <= min_float {
        return min_bound;
    }

    if max_is_rounded_up {
        if value >= max_float {
            return max_bound;
        }
    } else if value >= max_float {
        return max_bound;
    }

    let truncated = value.trunc() as i128;
    if truncated < min_bound {
        return min_bound;
    }

    if truncated > max_bound {
        return max_bound;
    }

    truncated
}

/// Create a signed or unsigned integer constant.
fn integer_constant_from_i128(value: i128, width: u16, is_signed: bool) -> mir::Constant {
    if is_signed {
        mir::Constant::Int {
            value,
            width,
            is_signed: true,
        }
    } else {
        mir::Constant::UInt {
            value: value as u128,
            width,
        }
    }
}

/// Convert an unsigned integer to i128 with target-bound saturation.
fn unsigned_as_saturating_i128(value: u128, max_bound: i128) -> i128 {
    // clamp before crossing the signed boundary
    if value > i128::MAX as u128 {
        max_bound
    } else {
        value as i128
    }
}

/// Truncate an unsigned integer to a target bit width.
fn truncate_unsigned(value: u128, width: u16) -> u128 {
    mask_to_width(value, width)
}

/// Sign extend a value from one width to another.
fn sign_extend(value: i128, from_width: u16, to_width: u16) -> i128 {
    // handle no extend case
    if from_width >= to_width || from_width >= 128 {
        return value;
    }

    // extend using truncate logic
    truncate_signed(value, from_width)
}

/// Try to fold a unary operation on a constant.
pub fn fold_unary(operator: mir::UnaryOperator, value: mir::Constant) -> Option<mir::Constant> {
    match (operator, &value) {
        (
            mir::UnaryOperator::Negate,
            mir::Constant::Int {
                value: v,
                width,
                is_signed: true,
            },
        ) => Some(mir::Constant::Int {
            value: v.wrapping_neg(),
            width: *width,
            is_signed: true,
        }),

        (mir::UnaryOperator::Negate, mir::Constant::Float { bits, format }) => {
            let value = -float_from_bits(format.format(), *bits);

            Some(mir::Constant::Float {
                bits: float_to_bits(format.format(), value),
                format: *format,
            })
        }

        (mir::UnaryOperator::Not, mir::Constant::Boolean { value: v }) => {
            Some(mir::Constant::Boolean { value: !v })
        }

        (
            mir::UnaryOperator::Not,
            mir::Constant::Int {
                value: v,
                width,
                is_signed,
            },
        ) => Some(mir::Constant::Int {
            value: !v,
            width: *width,
            is_signed: *is_signed,
        }),

        (mir::UnaryOperator::Not, mir::Constant::UInt { value: v, width }) => {
            Some(mir::Constant::UInt {
                value: !v,
                width: *width,
            })
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Readonly global loads are not represented as scalar constants.
    #[test]
    fn test_readonly_global_load_not_constant() {
        let test = TestModule::new(
            r#"
readonly global flag: boolean = true

function test(): boolean {
entry:
    v0: ref<boolean, borrowed, 'static, readonly, local> = global.address flag
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
        let test = TestModule::new(
            r#"
global flag: boolean = true

function test(): boolean {
entry:
    v0: ref<boolean, borrowed, 'static, mutable, local> = global.address flag
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
        let test = TestModule::new(
            r#"
readonly global flag: boolean = zeroinit

function test(): boolean {
entry:
    v0: ref<boolean, borrowed, 'static, readonly, local> = global.address flag
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
        let test = TestModule::new(
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
        let test = TestModule::new(
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
        let test = TestModule::new(
            r#"
function test(v0: int64): boolean {
entry(v0: int64):
    v1: boolean = true
    new.slice.uninit.try int32, v0 => b1(v1) | b2

b1(v2: uninit<slice<int32, managed, mutable, local>>, v3: boolean):
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
        let test = TestModule::new(
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
        let test = TestModule::new(
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
