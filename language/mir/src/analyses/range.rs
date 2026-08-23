use std::collections::VecDeque;

use crate as mir;
use destack_core::{FxIndexMap, float_from_bits, float_to_bits};

use crate::{
    Analysis, ControlTable, FunctionCache, NodeTable, RangeOptions, TargetLayout, fold_binary,
    fold_cast, fold_unary,
};

use super::{Lattice, Mutation};

/// Value ranges for one function.
#[derive(Debug)]
pub struct RangeTable {
    /// Ranges available at block entry indexed by block id.
    block_entry: NodeTable<mir::Block, Option<RangeState>>,
    /// Ranges available at block exit indexed by block id.
    block_exit: NodeTable<mir::Block, Option<RangeState>>,
}

/// Range information for a value.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueRange {
    /// Bounds for floating point values.
    Float {
        /// Finite bounds for the float range.
        bounds: Option<FloatBounds>,
        /// Concrete float format.
        format: mir::FloatType,
        /// Whether NaN is possible.
        can_be_nan: bool,
        /// Whether positive infinity is possible.
        can_be_pos_inf: bool,
        /// Whether negative infinity is possible.
        can_be_neg_inf: bool,
    },
    /// Boolean values that can occur.
    Boolean {
        /// Whether true is possible.
        can_be_true: bool,
        /// Whether false is possible.
        can_be_false: bool,
    },
    /// Integer values within inclusive bounds.
    Integer {
        /// Minimum possible value.
        min: i128,
        /// Maximum possible value.
        max: i128,
        /// Bit width of the integer.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
}

/// Finite bounds for float ranges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatBounds {
    /// Minimum finite value.
    pub min: f64,
    /// Maximum finite value.
    pub max: f64,
}

impl ValueRange {
    /// Build a value range from a MIR constant.
    pub fn from_constant(constant: &mir::Constant) -> Option<Self> {
        // map supported constant kinds to ranges
        match constant {
            mir::Constant::Float { bits, format } => {
                let value = float_from_bits(format.format(), *bits);

                let can_be_nan = value.is_nan();
                let can_be_pos_inf = value.is_infinite() && value.is_sign_positive();
                let can_be_neg_inf = value.is_infinite() && value.is_sign_negative();
                let bounds = if can_be_nan || can_be_pos_inf || can_be_neg_inf {
                    None
                } else {
                    Some(FloatBounds {
                        min: value,
                        max: value,
                    })
                };

                Some(ValueRange::Float {
                    bounds,
                    format: *format,
                    can_be_nan,
                    can_be_pos_inf,
                    can_be_neg_inf,
                })
            }
            mir::Constant::Boolean { value } => Some(ValueRange::Boolean {
                can_be_true: *value,
                can_be_false: !*value,
            }),
            mir::Constant::Int {
                value,
                width,
                is_signed,
            } => Some(ValueRange::Integer {
                min: *value,
                max: *value,
                width: *width,
                is_signed: *is_signed,
            }),
            mir::Constant::UInt { value, width } => Some(ValueRange::Integer {
                min: *value as i128,
                max: *value as i128,
                width: *width,
                is_signed: false,
            }),
            _ => None,
        }
    }

    /// Return a constant value if this range is a single point.
    pub fn as_constant(&self) -> Option<mir::Constant> {
        // convert ranges to exact constants when possible
        match self {
            ValueRange::Float { .. } => None,
            ValueRange::Boolean {
                can_be_true,
                can_be_false,
            } => match (*can_be_true, *can_be_false) {
                (true, false) => Some(mir::Constant::Boolean { value: true }),
                (false, true) => Some(mir::Constant::Boolean { value: false }),
                _ => None,
            },
            ValueRange::Integer {
                min,
                max,
                width,
                is_signed,
            } => {
                // reject non point ranges
                if min != max {
                    return None;
                }

                // select a signed or unsigned constant representation
                if *is_signed {
                    Some(mir::Constant::Int {
                        value: *min,
                        width: *width,
                        is_signed: true,
                    })
                } else {
                    let value = u128::try_from(*min).ok()?;
                    Some(mir::Constant::UInt {
                        value,
                        width: *width,
                    })
                }
            }
        }
    }

    /// Return the boolean constant represented by this range when known.
    pub fn as_boolean_constant(&self) -> Option<bool> {
        let ValueRange::Boolean {
            can_be_true,
            can_be_false,
        } = self
        else {
            return None;
        };

        match (*can_be_true, *can_be_false) {
            (true, false) => Some(true),
            (false, true) => Some(false),
            _ => None,
        }
    }

    /// Evaluate an integer comparison against another range.
    pub fn compare_integer(
        &self,
        operator: mir::BinaryOperator,
        other: &ValueRange,
    ) -> Option<bool> {
        // extract integer ranges for both operands
        let ValueRange::Integer {
            min: left_min,
            max: left_max,
            width: left_width,
            is_signed: left_signed,
        } = self
        else {
            return None;
        };
        let ValueRange::Integer {
            min: right_min,
            max: right_max,
            width: right_width,
            is_signed: right_signed,
        } = other
        else {
            return None;
        };

        // reject mismatched integer widths or signedness
        if left_width != right_width || left_signed != right_signed {
            return None;
        }

        // evaluate comparison from range relationships
        match operator {
            mir::BinaryOperator::Equal => {
                let is_single = left_min == left_max && right_min == right_max;
                if is_single && left_min == right_min {
                    Some(true)
                } else if left_max < right_min || left_min > right_max {
                    Some(false)
                } else {
                    None
                }
            }
            mir::BinaryOperator::NotEqual => {
                let is_single = left_min == left_max && right_min == right_max;
                if left_max < right_min || left_min > right_max {
                    Some(true)
                } else if is_single && left_min == right_min {
                    Some(false)
                } else {
                    None
                }
            }
            mir::BinaryOperator::LessThan => {
                if left_max < right_min {
                    Some(true)
                } else if left_min >= right_max {
                    Some(false)
                } else {
                    None
                }
            }
            mir::BinaryOperator::LessEqual => {
                if left_max <= right_min {
                    Some(true)
                } else if left_min > right_max {
                    Some(false)
                } else {
                    None
                }
            }
            mir::BinaryOperator::GreaterThan => {
                if left_min > right_max {
                    Some(true)
                } else if left_max <= right_min {
                    Some(false)
                } else {
                    None
                }
            }
            mir::BinaryOperator::GreaterEqual => {
                if left_min >= right_max {
                    Some(true)
                } else if left_max < right_min {
                    Some(false)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Merge two ranges by taking the union of possible values.
    pub fn union(&self, other: &Self) -> Option<Self> {
        // merge compatible ranges
        match (self, other) {
            (
                ValueRange::Float {
                    bounds: left_bounds,
                    format: left_format,
                    can_be_nan: left_nan,
                    can_be_pos_inf: left_pos_inf,
                    can_be_neg_inf: left_neg_inf,
                },
                ValueRange::Float {
                    bounds: right_bounds,
                    format: right_format,
                    can_be_nan: right_nan,
                    can_be_pos_inf: right_pos_inf,
                    can_be_neg_inf: right_neg_inf,
                },
            ) => {
                // refuse to merge mismatched float types
                if left_format != right_format {
                    return None;
                }

                let bounds = match (left_bounds, right_bounds) {
                    (Some(left), Some(right)) => Some(FloatBounds {
                        min: left.min.min(right.min),
                        max: left.max.max(right.max),
                    }),
                    (Some(bounds), None) => Some(*bounds),
                    (None, Some(bounds)) => Some(*bounds),
                    _ => None,
                };

                Some(ValueRange::Float {
                    bounds,
                    format: *left_format,
                    can_be_nan: *left_nan || *right_nan,
                    can_be_pos_inf: *left_pos_inf || *right_pos_inf,
                    can_be_neg_inf: *left_neg_inf || *right_neg_inf,
                })
            }
            (
                ValueRange::Boolean {
                    can_be_true: left_true,
                    can_be_false: left_false,
                },
                ValueRange::Boolean {
                    can_be_true: right_true,
                    can_be_false: right_false,
                },
            ) => Some(ValueRange::Boolean {
                can_be_true: *left_true || *right_true,
                can_be_false: *left_false || *right_false,
            }),
            (
                ValueRange::Integer {
                    min: left_min,
                    max: left_max,
                    width: left_width,
                    is_signed: left_signed,
                },
                ValueRange::Integer {
                    min: right_min,
                    max: right_max,
                    width: right_width,
                    is_signed: right_signed,
                },
            ) => {
                // refuse to merge mismatched integer types
                if left_width != right_width || left_signed != right_signed {
                    return None;
                }

                // widen to cover both ranges
                Some(ValueRange::Integer {
                    min: (*left_min).min(*right_min),
                    max: (*left_max).max(*right_max),
                    width: *left_width,
                    is_signed: *left_signed,
                })
            }
            _ => None,
        }
    }
}

/// Mapping from values to their ranges.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RangeState {
    /// Known ranges keyed by value.
    ranges: FxIndexMap<mir::Value, ValueRange>,
}

impl RangeState {
    /// Create an empty range map.
    pub fn new() -> Self {
        Self {
            ranges: FxIndexMap::default(),
        }
    }

    /// Return the range for one value.
    pub fn get(&self, value: impl Into<mir::Value>) -> Option<&ValueRange> {
        self.ranges.get(&value.into())
    }

    /// Insert a range for a value.
    pub fn insert(&mut self, value: impl Into<mir::Value>, range: ValueRange) {
        self.ranges.insert(value.into(), range);
    }

    /// Remove any range for a value.
    pub fn remove(&mut self, value: impl Into<mir::Value>) {
        self.ranges.shift_remove(&value.into());
    }

    /// Iterate over known ranges.
    pub fn iter(&self) -> impl Iterator<Item = (mir::Value, &ValueRange)> + '_ {
        self.ranges.iter().map(|(value, range)| (*value, range))
    }

    /// Widen all ranges to their full type bounds.
    pub fn widen_all(&mut self) {
        for range in self.ranges.values_mut() {
            *range = range.widen();
        }
    }
}

impl Lattice for RangeState {
    /// Merge ranges that are known on all incoming paths.
    fn meet(&self, other: &Self) -> Self {
        let mut ranges = FxIndexMap::default();

        // merge ranges present on every incoming path
        for (value, range) in &self.ranges {
            if let Some(other_range) = other.ranges.get(value)
                && let Some(merged) = range.union(other_range)
            {
                ranges.insert(*value, merged);
            }
        }

        Self { ranges }
    }
}

impl RangeTable {
    /// Build range analysis for a function.
    fn build(
        function: &mir::Function,
        tree: &mir::Tree,
        cfg: &ControlTable,
        target_layout: TargetLayout,
        options: RangeOptions,
    ) -> Self {
        let Some(entry) = function.entry() else {
            return Self {
                block_entry: NodeTable::new(),
                block_exit: NodeTable::new(),
            };
        };

        // init state maps
        let mut block_entry = NodeTable::from_nodes(function.blocks(), || None);
        let mut block_exit = NodeTable::from_nodes(function.blocks(), || None);

        // seed entry state
        *block_entry.get_mut(entry) = Some(RangeState::new());

        // init worklist
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = VecDeque::new();
        let mut in_worklist = NodeTable::from_nodes(function.blocks(), || false);
        let mut update_counts = NodeTable::from_nodes(function.blocks(), || 0u32);
        worklist.push_back(entry);
        *in_worklist.get_mut(entry) = true;

        // process blocks until fixed point
        while let Some(block_id) = worklist.pop_front() {
            // remove block from worklist
            *in_worklist.get_mut(block_id) = false;

            // compute entry state
            let mut entry_state = if block_id == entry {
                block_entry
                    .get(entry)
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| panic!("missing range entry state: {entry:?}"))
            } else {
                // merge predecessor exits
                let mut merged: Option<RangeState> = None;
                for &pred in cfg.predecessors(block_id) {
                    let Some(pred_exit) = block_exit.get(pred).as_ref() else {
                        continue;
                    };

                    merged = Some(match merged {
                        Some(existing) => existing.meet(pred_exit),
                        None => pred_exit.clone(),
                    });
                }

                let Some(merged) = merged else {
                    continue;
                };

                merged
            };

            // apply block parameter ranges
            RangeTable::apply_block_parameters(block_id, tree, cfg, &block_exit, &mut entry_state);

            // check if entry state changed
            let entry_changed = block_entry
                .get(block_id)
                .as_ref()
                .map(|old| old != &entry_state)
                .unwrap_or(true);

            if entry_changed || block_id == entry {
                if entry_changed {
                    *update_counts.get_mut(block_id) += 1;
                    if *update_counts.get(block_id) > options.widen_threshold {
                        entry_state.widen_all();
                    }
                }

                *block_entry.get_mut(block_id) = Some(entry_state.clone());

                // transfer through block
                let exit_state = RangeTable::transfer(
                    block_id,
                    &entry_state,
                    tree,
                    target_layout.pointer_bits(),
                );

                // check if exit state changed
                let exit_changed = block_exit
                    .get(block_id)
                    .as_ref()
                    .map(|old| old != &exit_state)
                    .unwrap_or(true);

                if exit_changed {
                    *block_exit.get_mut(block_id) = Some(exit_state);

                    // add successors to worklist
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

    /// Return ranges at block entry.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> &RangeState {
        let ranges = self.block_entry.get(block);

        match ranges {
            Some(ranges) => ranges,
            None => RangeState::empty(),
        }
    }

    /// Return ranges at block exit.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> &RangeState {
        let ranges = self.block_exit.get(block);

        match ranges {
            Some(ranges) => ranges,
            None => RangeState::empty(),
        }
    }
}

impl Analysis for RangeTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL
        .union(Mutation::VALUE)
        .union(Mutation::LAYOUT);
}

impl RangeTable {
    /// Compute value ranges for one function.
    pub(crate) fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &mut FunctionCache,
    ) -> Self {
        let cfg = analyses.control(function, tree);
        Self::build(
            function,
            tree,
            &cfg,
            analyses.target_layout(),
            analyses.options().range,
        )
    }
}

impl ValueRange {
    /// Widen this range to its full type bounds.
    fn widen(&self) -> Self {
        match self {
            Self::Float { format, .. } => Self::float_full(*format),
            Self::Boolean { .. } => Self::Boolean {
                can_be_true: true,
                can_be_false: true,
            },
            Self::Integer {
                width, is_signed, ..
            } => Self::integer_full(*width, *is_signed).unwrap_or_else(|| self.clone()),
        }
    }
}

/// Parameter range tracking state during merge.
#[derive(Debug, Clone)]
enum ParamRangeState {
    /// No predecessor has been seen yet.
    Unseen,
    /// Ranges accumulated across predecessors.
    Range(ValueRange),
    /// Conflicting or unknown input.
    Overdefined,
}

impl RangeTable {
    /// Apply block parameter ranges derived from predecessor arguments.
    fn apply_block_parameters(
        block_id: mir::LocalNodeId<mir::Block>,
        tree: &mir::Tree,
        cfg: &ControlTable,
        block_exit: &NodeTable<mir::Block, Option<RangeState>>,
        entry_state: &mut RangeState,
    ) {
        // resolve ranges for block parameters
        let ranges = RangeTable::resolve_block_parameters(block_id, tree, cfg, block_exit);
        let block = tree.get(block_id);

        // apply ranges to entry state
        for param in &block.parameters {
            let param_value = param.value;

            if let Some(range) = ranges.get(&param_value) {
                entry_state.insert(param_value, range.clone());
                continue;
            }

            entry_state.remove(param_value);
        }
    }

    /// Resolve ranges for block parameters from predecessor arguments.
    fn resolve_block_parameters(
        block_id: mir::LocalNodeId<mir::Block>,
        tree: &mir::Tree,
        cfg: &ControlTable,
        block_exit: &NodeTable<mir::Block, Option<RangeState>>,
    ) -> FxIndexMap<mir::Value, ValueRange> {
        // early exit for blocks without parameters
        let block = tree.get(block_id);
        if block.parameters.is_empty() {
            return FxIndexMap::default();
        }

        // track parameter states across predecessors
        let mut states = vec![ParamRangeState::Unseen; block.parameters.len()];
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
                    states.fill(ParamRangeState::Overdefined);
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
                        states.fill(ParamRangeState::Overdefined);
                        break;
                    };
                    let argument_range = pred_exit.get(*argument).cloned();
                    states[index] = match (&states[index], argument_range) {
                        (ParamRangeState::Unseen, Some(range)) => ParamRangeState::Range(range),
                        (ParamRangeState::Unseen, None) => ParamRangeState::Overdefined,
                        (ParamRangeState::Range(existing), Some(range)) => {
                            match existing.union(&range) {
                                Some(merged) => ParamRangeState::Range(merged),
                                None => ParamRangeState::Overdefined,
                            }
                        }
                        (ParamRangeState::Range(_), None) => ParamRangeState::Overdefined,
                        (ParamRangeState::Overdefined, _) => ParamRangeState::Overdefined,
                    };
                }
            }
        }

        // return empty if no predecessors were processed
        if !is_seen {
            return FxIndexMap::default();
        }

        // collect ranges for parameters
        let mut ranges = FxIndexMap::default();
        for (param, state) in block.parameters.iter().zip(states) {
            let param_value = param.value;

            if let ParamRangeState::Range(range) = state {
                ranges.insert(param_value, range);
            }
        }

        ranges
    }

    /// Transfer ranges through a block's instructions.
    fn transfer(
        block_id: mir::LocalNodeId<mir::Block>,
        entry_state: &RangeState,
        tree: &mir::Tree,
        pointer_width_bits: u16,
    ) -> RangeState {
        // clone entry state for updates
        let block = tree.get(block_id);
        let mut state = entry_state.clone();

        // update state per instruction
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            let Some(destination) = instruction.destination() else {
                continue;
            };

            if let Some(range) =
                RangeTable::instruction_range(instruction, tree, &state, pointer_width_bits)
            {
                state.insert(destination, range);
            } else {
                state.remove(destination);
            }
        }

        state
    }

    /// Evaluate a range for an instruction when possible.
    fn instruction_range(
        instruction: &mir::Instruction,
        tree: &mir::Tree,
        state: &RangeState,
        pointer_width_bits: u16,
    ) -> Option<ValueRange> {
        match instruction {
            mir::Instruction::Const { value, .. } => ValueRange::from_constant(value),
            mir::Instruction::Binary {
                operator,
                left,
                right,
                ..
            } => ValueRange::binary(*operator, state.get(*left), state.get(*right)),
            mir::Instruction::Unary {
                operator, argument, ..
            } => ValueRange::unary(*operator, state.get(*argument)),
            mir::Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => ValueRange::cast(
                *operator,
                state.get(*argument),
                *to_type,
                tree,
                pointer_width_bits,
            ),
            mir::Instruction::Select {
                then_value,
                else_value,
                ..
            } => ValueRange::select(state.get(*then_value), state.get(*else_value)),
            _ => None,
        }
    }
}

impl ValueRange {
    /// Evaluate a range for a binary instruction.
    fn binary(
        operator: mir::BinaryOperator,
        left: Option<&ValueRange>,
        right: Option<&ValueRange>,
    ) -> Option<ValueRange> {
        // require ranges for both operands
        let left = left?;
        let right = right?;

        // fold exact constants when possible
        if let (Some(left_const), Some(right_const)) = (left.as_constant(), right.as_constant()) {
            let result = fold_binary(operator, left_const, right_const)?;
            return ValueRange::from_constant(&result);
        }

        match left {
            ValueRange::Integer { is_signed, .. } => match operator {
                mir::BinaryOperator::Add => ValueRange::integer_add(left, right),
                mir::BinaryOperator::Subtract => ValueRange::integer_subtract(left, right),
                mir::BinaryOperator::Multiply => ValueRange::integer_multiply(left, right),
                mir::BinaryOperator::Divide if *is_signed => {
                    ValueRange::integer_divide_signed(left, right)
                }
                mir::BinaryOperator::Divide => ValueRange::integer_divide_unsigned(left, right),
                mir::BinaryOperator::Remainder if *is_signed => {
                    ValueRange::integer_remainder_signed(left, right)
                }
                mir::BinaryOperator::Remainder => {
                    ValueRange::integer_remainder_unsigned(left, right)
                }
                operator if operator.is_comparison() => ValueRange::compare(operator, left, right),
                _ => None,
            },
            ValueRange::Float { .. } => match operator {
                mir::BinaryOperator::Add => ValueRange::float_add(left, right),
                mir::BinaryOperator::Subtract => ValueRange::float_subtract(left, right),
                mir::BinaryOperator::Multiply => ValueRange::float_multiply(left, right),
                mir::BinaryOperator::Divide => ValueRange::float_divide(left, right),
                operator if operator.is_comparison() => {
                    ValueRange::compare_float(operator, left, right)
                }
                _ => None,
            },
            ValueRange::Boolean { .. } => None,
        }
    }

    /// Evaluate a range for a unary instruction.
    fn unary(operator: mir::UnaryOperator, argument: Option<&ValueRange>) -> Option<ValueRange> {
        // require a range for the operand
        let argument = argument?;

        // fold exact constants when possible
        if let Some(constant) = argument.as_constant() {
            let result = fold_unary(operator, constant)?;
            return ValueRange::from_constant(&result);
        }

        match (operator, argument) {
            (mir::UnaryOperator::Negate, ValueRange::Integer { .. }) => {
                ValueRange::integer_negate(argument)
            }
            (mir::UnaryOperator::Negate, ValueRange::Float { .. }) => {
                ValueRange::float_negate(argument)
            }
            _ => None,
        }
    }

    /// Evaluate a range for a cast instruction.
    fn cast(
        operator: mir::CastOperator,
        argument: Option<&ValueRange>,
        to_type: mir::LocalNodeId<mir::Type>,
        tree: &mir::Tree,
        pointer_width_bits: u16,
    ) -> Option<ValueRange> {
        // require a range for the operand
        let argument = argument?;

        // fold exact constants when possible
        if let Some(constant) = argument.as_constant() {
            let result = fold_cast(operator, constant, to_type, pointer_width_bits, tree)?;
            return ValueRange::from_constant(&result);
        }

        // read the target type
        let to_type = tree.get(to_type);

        match operator {
            mir::CastOperator::SignExtend
            | mir::CastOperator::ZeroExtend
            | mir::CastOperator::Truncate
            | mir::CastOperator::Saturate => {
                // require an integer operand range
                let ValueRange::Integer {
                    min,
                    max,
                    width: _,
                    is_signed,
                } = argument
                else {
                    return None;
                };

                // require an integer target type
                let (to_width, to_signed) =
                    to_type.int_info_with_pointer_width(pointer_width_bits)?;

                match operator {
                    mir::CastOperator::SignExtend => {
                        if !*is_signed {
                            return None;
                        }
                        Some(ValueRange::Integer {
                            min: *min,
                            max: *max,
                            width: to_width,
                            is_signed: true,
                        })
                    }
                    mir::CastOperator::ZeroExtend => {
                        if *is_signed {
                            return None;
                        }
                        Some(ValueRange::Integer {
                            min: *min,
                            max: *max,
                            width: to_width,
                            is_signed: false,
                        })
                    }
                    mir::CastOperator::Truncate => {
                        let (min_bound, max_bound) =
                            ValueRange::integer_bounds(to_width, to_signed)?;
                        if *min < min_bound || *max > max_bound {
                            return None;
                        }
                        Some(ValueRange::Integer {
                            min: *min,
                            max: *max,
                            width: to_width,
                            is_signed: to_signed,
                        })
                    }
                    mir::CastOperator::Saturate => {
                        let (min_bound, max_bound) =
                            ValueRange::integer_bounds(to_width, to_signed)?;
                        Some(ValueRange::Integer {
                            min: (*min).clamp(min_bound, max_bound),
                            max: (*max).clamp(min_bound, max_bound),
                            width: to_width,
                            is_signed: to_signed,
                        })
                    }
                    _ => None,
                }
            }
            mir::CastOperator::SignedIntToFloat | mir::CastOperator::UnsignedIntToFloat => {
                // require a float target type
                let mir::Type::Float(float_type) = to_type else {
                    return None;
                };

                ValueRange::from_integer(argument, *float_type, operator)
            }
            mir::CastOperator::FloatToSignedInt
            | mir::CastOperator::FloatToUnsignedInt
            | mir::CastOperator::FloatToSignedIntSaturating
            | mir::CastOperator::FloatToUnsignedIntSaturating => {
                // require an integer target type
                let (to_width, to_signed) =
                    to_type.int_info_with_pointer_width(pointer_width_bits)?;

                ValueRange::from_float(argument, to_width, to_signed, operator)
            }
            mir::CastOperator::FloatTruncate
            | mir::CastOperator::FloatExtend
            | mir::CastOperator::FloatConvert => {
                // require a float target type
                let mir::Type::Float(float_type) = to_type else {
                    return None;
                };

                ValueRange::float_cast(argument, *float_type)
            }
            _ => None,
        }
    }

    /// Evaluate a range for a select instruction.
    fn select(
        then_value: Option<&ValueRange>,
        else_value: Option<&ValueRange>,
    ) -> Option<ValueRange> {
        // require ranges for both arms
        let then_value = then_value?;
        let else_value = else_value?;

        then_value.union(else_value)
    }
}

impl RangeState {
    /// Return a shared empty range map.
    fn empty() -> &'static Self {
        // initialize the shared empty state
        static EMPTY: std::sync::OnceLock<RangeState> = std::sync::OnceLock::new();

        EMPTY.get_or_init(Self::new)
    }
}

impl ValueRange {
    /// Compute integer bounds for a width and signedness.
    fn integer_bounds(width: u16, is_signed: bool) -> Option<(i128, i128)> {
        // reject unsupported widths
        if width == 0 || width > 128 || (!is_signed && width == 128) {
            return None;
        }

        // compute signed or unsigned bounds
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

    /// Extract integer range fields with bounds check.
    fn integer_fields(range: &ValueRange) -> Option<(i128, i128, u16, bool)> {
        // require an integer range
        let ValueRange::Integer {
            min,
            max,
            width,
            is_signed,
        } = range
        else {
            return None;
        };

        Some((*min, *max, *width, *is_signed))
    }

    /// Build an integer range with full bounds.
    fn integer_full(width: u16, is_signed: bool) -> Option<ValueRange> {
        // compute full bounds for the integer width
        let (min, max) = ValueRange::integer_bounds(width, is_signed)?;
        Some(ValueRange::Integer {
            min,
            max,
            width,
            is_signed,
        })
    }

    /// Compute a range for integer addition.
    fn integer_add(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_min, left_max, width, is_signed) = ValueRange::integer_fields(left)?;
        let (right_min, right_max, right_width, right_signed) = ValueRange::integer_fields(right)?;

        // ensure operand types match
        if width != right_width || is_signed != right_signed {
            return None;
        }

        // compute bounds for the sum
        let min = left_min.checked_add(right_min)?;
        let max = left_max.checked_add(right_max)?;

        // clamp to full range if overflow is possible
        let (min_bound, max_bound) = ValueRange::integer_bounds(width, is_signed)?;
        if min < min_bound || max > max_bound {
            return ValueRange::integer_full(width, is_signed);
        }

        Some(ValueRange::Integer {
            min,
            max,
            width,
            is_signed,
        })
    }

    /// Compute a range for integer subtraction.
    fn integer_subtract(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_min, left_max, width, is_signed) = ValueRange::integer_fields(left)?;
        let (right_min, right_max, right_width, right_signed) = ValueRange::integer_fields(right)?;

        // ensure operand types match
        if width != right_width || is_signed != right_signed {
            return None;
        }

        // compute bounds for the difference
        let min = left_min.checked_sub(right_max)?;
        let max = left_max.checked_sub(right_min)?;

        // clamp to full range if overflow is possible
        let (min_bound, max_bound) = ValueRange::integer_bounds(width, is_signed)?;
        if min < min_bound || max > max_bound {
            return ValueRange::integer_full(width, is_signed);
        }

        Some(ValueRange::Integer {
            min,
            max,
            width,
            is_signed,
        })
    }

    /// Compute a range for integer multiplication.
    fn integer_multiply(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_min, left_max, width, is_signed) = ValueRange::integer_fields(left)?;
        let (right_min, right_max, right_width, right_signed) = ValueRange::integer_fields(right)?;

        // ensure operand types match
        if width != right_width || is_signed != right_signed {
            return None;
        }

        // compute candidate products
        let candidates = [
            left_min.checked_mul(right_min),
            left_min.checked_mul(right_max),
            left_max.checked_mul(right_min),
            left_max.checked_mul(right_max),
        ];

        // overflow yields the full range
        if candidates.iter().any(|value| value.is_none()) {
            return ValueRange::integer_full(width, is_signed);
        }

        // select the min and max product
        let values: Vec<i128> = candidates.into_iter().flatten().collect();
        let min = *values.iter().min()?;
        let max = *values.iter().max()?;

        // clamp to full range if overflow is possible
        let (min_bound, max_bound) = ValueRange::integer_bounds(width, is_signed)?;
        if min < min_bound || max > max_bound {
            return ValueRange::integer_full(width, is_signed);
        }

        Some(ValueRange::Integer {
            min,
            max,
            width,
            is_signed,
        })
    }

    /// Compute a range for signed integer division.
    fn integer_divide_signed(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_min, left_max, width, is_signed) = ValueRange::integer_fields(left)?;
        let (right_min, right_max, right_width, right_signed) = ValueRange::integer_fields(right)?;

        // enforce signed division and matching widths
        if !is_signed || !right_signed || width != right_width {
            return None;
        }

        // division by zero yields full range
        if right_min <= 0 && right_max >= 0 {
            return ValueRange::integer_full(width, is_signed);
        }

        // compute candidate quotients
        let candidates = [
            left_min.checked_div(right_min),
            left_min.checked_div(right_max),
            left_max.checked_div(right_min),
            left_max.checked_div(right_max),
        ];

        // overflow yields the full range
        if candidates.iter().any(|value| value.is_none()) {
            return ValueRange::integer_full(width, is_signed);
        }

        // select the min and max quotient
        let values: Vec<i128> = candidates.into_iter().flatten().collect();
        let min = *values.iter().min()?;
        let max = *values.iter().max()?;

        // clamp to full range if overflow is possible
        let (min_bound, max_bound) = ValueRange::integer_bounds(width, is_signed)?;
        if min < min_bound || max > max_bound {
            return ValueRange::integer_full(width, is_signed);
        }

        Some(ValueRange::Integer {
            min,
            max,
            width,
            is_signed,
        })
    }

    /// Compute a range for unsigned integer division.
    fn integer_divide_unsigned(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_min, left_max, width, is_signed) = ValueRange::integer_fields(left)?;
        let (right_min, right_max, right_width, right_signed) = ValueRange::integer_fields(right)?;

        // enforce unsigned division and matching widths
        if is_signed || right_signed || width != right_width {
            return None;
        }

        // division by zero yields full range
        if right_min == 0 {
            return ValueRange::integer_full(width, is_signed);
        }

        // compute the min and max quotient
        let min = left_min.checked_div(right_max)?;
        let max = left_max.checked_div(right_min)?;

        Some(ValueRange::Integer {
            min,
            max,
            width,
            is_signed,
        })
    }

    /// Compute a range for signed integer remainder.
    fn integer_remainder_signed(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (_, _, width, is_signed) = ValueRange::integer_fields(left)?;
        let (right_min, right_max, right_width, right_signed) = ValueRange::integer_fields(right)?;

        // enforce signed remainder and matching widths
        if !is_signed || !right_signed || width != right_width {
            return None;
        }

        // remainder by zero yields full range
        if right_min <= 0 && right_max >= 0 {
            return ValueRange::integer_full(width, is_signed);
        }

        // compute the maximum possible absolute remainder
        let max_abs = right_min.abs().max(right_max.abs());
        let max_abs = max_abs.saturating_sub(1);

        Some(ValueRange::Integer {
            min: -max_abs,
            max: max_abs,
            width,
            is_signed,
        })
    }

    /// Compute a range for unsigned integer remainder.
    fn integer_remainder_unsigned(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (_, _, width, is_signed) = ValueRange::integer_fields(left)?;
        let (right_min, right_max, right_width, right_signed) = ValueRange::integer_fields(right)?;

        // enforce unsigned remainder and matching widths
        if is_signed || right_signed || width != right_width {
            return None;
        }

        // remainder by zero yields full range
        if right_min == 0 {
            return ValueRange::integer_full(width, is_signed);
        }

        // compute the maximum possible remainder
        let max = right_max.saturating_sub(1);

        Some(ValueRange::Integer {
            min: 0,
            max,
            width,
            is_signed,
        })
    }

    /// Compute a range for integer negation.
    fn integer_negate(range: &ValueRange) -> Option<ValueRange> {
        // extract the operand range
        let (min, max, width, is_signed) = ValueRange::integer_fields(range)?;

        // require a signed range
        if !is_signed {
            return None;
        }

        // negation can overflow for the minimum value
        let (min_bound, _max_bound) = ValueRange::integer_bounds(width, is_signed)?;
        if min == min_bound {
            return ValueRange::integer_full(width, is_signed);
        }

        // compute the negated bounds
        let neg_min = max.checked_neg()?;
        let neg_max = min.checked_neg()?;

        Some(ValueRange::Integer {
            min: neg_min,
            max: neg_max,
            width,
            is_signed,
        })
    }

    /// Return the maximum finite magnitude for a float format.
    fn float_max_finite(format: mir::FloatType) -> f64 {
        match format {
            mir::FloatType::Float32 => f32::MAX as f64,
            mir::FloatType::Float64 => f64::MAX,
        }
    }

    /// Build finite bounds that cover all finite values of a float format.
    fn float_finite_bounds(format: mir::FloatType) -> FloatBounds {
        let max = ValueRange::float_max_finite(format);

        FloatBounds { min: -max, max }
    }

    /// Extract float range fields with bounds check.
    fn float_fields(
        range: &ValueRange,
    ) -> Option<(Option<FloatBounds>, mir::FloatType, bool, bool, bool)> {
        // require a float range
        let ValueRange::Float {
            bounds,
            format,
            can_be_nan,
            can_be_pos_inf,
            can_be_neg_inf,
        } = range
        else {
            return None;
        };

        Some((
            *bounds,
            *format,
            *can_be_nan,
            *can_be_pos_inf,
            *can_be_neg_inf,
        ))
    }

    /// Build a float range with full bounds.
    fn float_full(format: mir::FloatType) -> ValueRange {
        let bounds = ValueRange::float_finite_bounds(format);

        ValueRange::Float {
            bounds: Some(bounds),
            format,
            can_be_nan: true,
            can_be_pos_inf: true,
            can_be_neg_inf: true,
        }
    }

    /// Round a float value to the given format.
    fn float_round(format: mir::FloatType, value: f64) -> f64 {
        if format == mir::FloatType::Float32 {
            return (value as f32) as f64;
        }

        float_from_bits(format.format(), float_to_bits(format.format(), value))
    }

    /// Cast an integer value to the given float format.
    fn float_from_integer(format: mir::FloatType, value: i128) -> f64 {
        ValueRange::float_round(format, value as f64)
    }

    /// Apply a float addition with the given format.
    fn float_add_value(format: mir::FloatType, left: f64, right: f64) -> f64 {
        if format == mir::FloatType::Float32 {
            return ((left as f32) + (right as f32)) as f64;
        }

        ValueRange::float_round(format, left + right)
    }

    /// Apply a float subtraction with the given format.
    fn float_subtract_value(format: mir::FloatType, left: f64, right: f64) -> f64 {
        if format == mir::FloatType::Float32 {
            return ((left as f32) - (right as f32)) as f64;
        }

        ValueRange::float_round(format, left - right)
    }

    /// Apply a float multiplication with the given format.
    fn float_multiply_value(format: mir::FloatType, left: f64, right: f64) -> f64 {
        if format == mir::FloatType::Float32 {
            return ((left as f32) * (right as f32)) as f64;
        }

        ValueRange::float_round(format, left * right)
    }

    /// Apply a float division with the given format.
    fn float_divide_value(format: mir::FloatType, left: f64, right: f64) -> f64 {
        if format == mir::FloatType::Float32 {
            return ((left as f32) / (right as f32)) as f64;
        }

        ValueRange::float_round(format, left / right)
    }

    /// Apply a float negation with the given format.
    fn float_negate_value(format: mir::FloatType, value: f64) -> f64 {
        ValueRange::float_round(format, -value)
    }
}

impl FloatBounds {
    /// Build finite bounds from candidate float values.
    fn from_candidates(candidates: &[f64]) -> (Option<FloatBounds>, bool, bool) {
        let mut min: Option<f64> = None;
        let mut max: Option<f64> = None;
        let mut can_be_pos_inf = false;
        let mut can_be_neg_inf = false;

        // scan candidates for finite bounds and infinities
        for value in candidates.iter().copied() {
            if value.is_nan() {
                continue;
            }

            if value.is_infinite() {
                if value.is_sign_positive() {
                    can_be_pos_inf = true;
                } else {
                    can_be_neg_inf = true;
                }
                continue;
            }

            match min {
                Some(current_min) => {
                    let Some(current_max) = max else {
                        min = Some(value);
                        max = Some(value);
                        continue;
                    };

                    let next_min = current_min.min(value);
                    let next_max = current_max.max(value);
                    min = Some(next_min);
                    max = Some(next_max);
                }
                None => {
                    min = Some(value);
                    max = Some(value);
                }
            }
        }

        // build bounds from finite candidates
        let bounds = match (min, max) {
            (Some(min), Some(max)) => Some(FloatBounds { min, max }),
            _ => None,
        };

        (bounds, can_be_pos_inf, can_be_neg_inf)
    }

    /// Return whether this range includes zero.
    fn contains_zero(&self) -> bool {
        self.min <= 0.0 && self.max >= 0.0
    }

    /// Return whether this range contains only zero.
    fn is_zero(&self) -> bool {
        self.min == 0.0 && self.max == 0.0
    }

    /// Return whether this range contains a positive value.
    fn can_be_positive(&self) -> bool {
        self.max > 0.0
    }

    /// Return whether this range contains a negative value.
    fn can_be_negative(&self) -> bool {
        self.min < 0.0
    }

    /// Build comparison bounds including infinities when needed.
    fn effective(
        bounds: Option<FloatBounds>,
        can_be_pos_inf: bool,
        can_be_neg_inf: bool,
    ) -> Option<FloatBounds> {
        if bounds.is_none() && !can_be_pos_inf && !can_be_neg_inf {
            return None;
        }

        // seed bounds from finite values when available
        let mut min = bounds.map_or(0.0, |bounds| bounds.min);
        let mut max = bounds.map_or(0.0, |bounds| bounds.max);

        // expand bounds to include infinities
        if bounds.is_none() {
            min = if can_be_neg_inf {
                f64::NEG_INFINITY
            } else if can_be_pos_inf {
                f64::INFINITY
            } else {
                0.0
            };
            max = if can_be_pos_inf {
                f64::INFINITY
            } else if can_be_neg_inf {
                f64::NEG_INFINITY
            } else {
                0.0
            };
        } else {
            if can_be_neg_inf {
                min = f64::NEG_INFINITY;
            }
            if can_be_pos_inf {
                max = f64::INFINITY;
            }
        }

        Some(FloatBounds { min, max })
    }
}

impl ValueRange {
    /// Compute a range for float addition.
    fn float_add(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_bounds, format, left_nan, left_pos_inf, left_neg_inf) =
            ValueRange::float_fields(left)?;
        let (right_bounds, right_format, right_nan, right_pos_inf, right_neg_inf) =
            ValueRange::float_fields(right)?;

        // ensure operand types match
        if format != right_format {
            return None;
        }

        // track NaN and infinity possibilities
        let mut can_be_nan = left_nan || right_nan;
        let mut can_be_pos_inf = false;
        let mut can_be_neg_inf = false;

        // handle infinity combinations
        if left_pos_inf && right_pos_inf {
            can_be_pos_inf = true;
        }
        if left_neg_inf && right_neg_inf {
            can_be_neg_inf = true;
        }
        if (left_pos_inf && right_neg_inf) || (left_neg_inf && right_pos_inf) {
            can_be_nan = true;
        }
        if left_pos_inf && right_bounds.is_some() {
            can_be_pos_inf = true;
        }
        if right_pos_inf && left_bounds.is_some() {
            can_be_pos_inf = true;
        }
        if left_neg_inf && right_bounds.is_some() {
            can_be_neg_inf = true;
        }
        if right_neg_inf && left_bounds.is_some() {
            can_be_neg_inf = true;
        }

        // derive finite bounds from candidates
        let bounds = match (left_bounds, right_bounds) {
            (Some(left), Some(right)) => {
                let candidates = [
                    ValueRange::float_add_value(format, left.min, right.min),
                    ValueRange::float_add_value(format, left.min, right.max),
                    ValueRange::float_add_value(format, left.max, right.min),
                    ValueRange::float_add_value(format, left.max, right.max),
                ];

                let (bounds, add_pos_inf, add_neg_inf) = FloatBounds::from_candidates(&candidates);
                can_be_pos_inf |= add_pos_inf;
                can_be_neg_inf |= add_neg_inf;
                bounds
            }
            _ => None,
        };

        Some(ValueRange::Float {
            bounds,
            format,
            can_be_nan,
            can_be_pos_inf,
            can_be_neg_inf,
        })
    }

    /// Compute a range for float subtraction.
    fn float_subtract(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_bounds, format, left_nan, left_pos_inf, left_neg_inf) =
            ValueRange::float_fields(left)?;
        let (right_bounds, right_format, right_nan, right_pos_inf, right_neg_inf) =
            ValueRange::float_fields(right)?;

        // ensure operand types match
        if format != right_format {
            return None;
        }

        // track NaN and infinity possibilities
        let mut can_be_nan = left_nan || right_nan;
        let mut can_be_pos_inf = false;
        let mut can_be_neg_inf = false;

        // handle infinity combinations
        if left_pos_inf && right_pos_inf {
            can_be_nan = true;
        }
        if left_neg_inf && right_neg_inf {
            can_be_nan = true;
        }
        if left_pos_inf && right_neg_inf {
            can_be_pos_inf = true;
        }
        if left_neg_inf && right_pos_inf {
            can_be_neg_inf = true;
        }
        if left_pos_inf && right_bounds.is_some() {
            can_be_pos_inf = true;
        }
        if left_neg_inf && right_bounds.is_some() {
            can_be_neg_inf = true;
        }
        if right_pos_inf && left_bounds.is_some() {
            can_be_neg_inf = true;
        }
        if right_neg_inf && left_bounds.is_some() {
            can_be_pos_inf = true;
        }

        // derive finite bounds from candidates
        let bounds = match (left_bounds, right_bounds) {
            (Some(left), Some(right)) => {
                let candidates = [
                    ValueRange::float_subtract_value(format, left.min, right.min),
                    ValueRange::float_subtract_value(format, left.min, right.max),
                    ValueRange::float_subtract_value(format, left.max, right.min),
                    ValueRange::float_subtract_value(format, left.max, right.max),
                ];

                let (bounds, sub_pos_inf, sub_neg_inf) = FloatBounds::from_candidates(&candidates);
                can_be_pos_inf |= sub_pos_inf;
                can_be_neg_inf |= sub_neg_inf;
                bounds
            }
            _ => None,
        };

        Some(ValueRange::Float {
            bounds,
            format,
            can_be_nan,
            can_be_pos_inf,
            can_be_neg_inf,
        })
    }

    /// Compute a range for float multiplication.
    fn float_multiply(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_bounds, format, left_nan, left_pos_inf, left_neg_inf) =
            ValueRange::float_fields(left)?;
        let (right_bounds, right_format, right_nan, right_pos_inf, right_neg_inf) =
            ValueRange::float_fields(right)?;

        // ensure operand types match
        if format != right_format {
            return None;
        }

        // track NaN and infinity possibilities
        let mut can_be_nan = left_nan || right_nan;
        let mut can_be_pos_inf = false;
        let mut can_be_neg_inf = false;

        // handle infinity multiplied by infinity
        if left_pos_inf && right_pos_inf {
            can_be_pos_inf = true;
        }
        if left_pos_inf && right_neg_inf {
            can_be_neg_inf = true;
        }
        if left_neg_inf && right_pos_inf {
            can_be_neg_inf = true;
        }
        if left_neg_inf && right_neg_inf {
            can_be_pos_inf = true;
        }

        // handle infinity multiplied by finite values
        if left_pos_inf && let Some(right_bounds) = right_bounds {
            if FloatBounds::contains_zero(&right_bounds) {
                can_be_nan = true;
            }
            if FloatBounds::can_be_positive(&right_bounds) {
                can_be_pos_inf = true;
            }
            if FloatBounds::can_be_negative(&right_bounds) {
                can_be_neg_inf = true;
            }
        }
        if left_neg_inf && let Some(right_bounds) = right_bounds {
            if FloatBounds::contains_zero(&right_bounds) {
                can_be_nan = true;
            }
            if FloatBounds::can_be_positive(&right_bounds) {
                can_be_neg_inf = true;
            }
            if FloatBounds::can_be_negative(&right_bounds) {
                can_be_pos_inf = true;
            }
        }
        if right_pos_inf && let Some(left_bounds) = left_bounds {
            if FloatBounds::contains_zero(&left_bounds) {
                can_be_nan = true;
            }
            if FloatBounds::can_be_positive(&left_bounds) {
                can_be_pos_inf = true;
            }
            if FloatBounds::can_be_negative(&left_bounds) {
                can_be_neg_inf = true;
            }
        }
        if right_neg_inf && let Some(left_bounds) = left_bounds {
            if FloatBounds::contains_zero(&left_bounds) {
                can_be_nan = true;
            }
            if FloatBounds::can_be_positive(&left_bounds) {
                can_be_neg_inf = true;
            }
            if FloatBounds::can_be_negative(&left_bounds) {
                can_be_pos_inf = true;
            }
        }

        // short circuit when one operand is exactly zero and the other is only infinite
        let left_is_zero = left_bounds.as_ref().is_some_and(FloatBounds::is_zero);
        let right_is_zero = right_bounds.as_ref().is_some_and(FloatBounds::is_zero);
        let left_is_infinite_only = left_bounds.is_none() && (left_pos_inf || left_neg_inf);
        let right_is_infinite_only = right_bounds.is_none() && (right_pos_inf || right_neg_inf);

        if (left_is_zero && right_is_infinite_only) || (right_is_zero && left_is_infinite_only) {
            return Some(ValueRange::Float {
                bounds: None,
                format,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false,
            });
        }

        // derive finite bounds from candidates
        let bounds = match (left_bounds, right_bounds) {
            (Some(left), Some(right)) => {
                let candidates = [
                    ValueRange::float_multiply_value(format, left.min, right.min),
                    ValueRange::float_multiply_value(format, left.min, right.max),
                    ValueRange::float_multiply_value(format, left.max, right.min),
                    ValueRange::float_multiply_value(format, left.max, right.max),
                ];

                let (bounds, mul_pos_inf, mul_neg_inf) = FloatBounds::from_candidates(&candidates);
                can_be_pos_inf |= mul_pos_inf;
                can_be_neg_inf |= mul_neg_inf;
                bounds
            }
            _ => None,
        };

        Some(ValueRange::Float {
            bounds,
            format,
            can_be_nan,
            can_be_pos_inf,
            can_be_neg_inf,
        })
    }

    /// Compute a range for float division.
    fn float_divide(left: &ValueRange, right: &ValueRange) -> Option<ValueRange> {
        // extract operand ranges
        let (left_bounds, format, left_nan, left_pos_inf, left_neg_inf) =
            ValueRange::float_fields(left)?;
        let (right_bounds, right_format, right_nan, right_pos_inf, right_neg_inf) =
            ValueRange::float_fields(right)?;

        // ensure operand types match
        if format != right_format {
            return None;
        }

        // reject NaN only operands
        let left_has_non_nan = left_bounds.is_some() || left_pos_inf || left_neg_inf;
        let right_has_non_nan = right_bounds.is_some() || right_pos_inf || right_neg_inf;

        if !left_has_non_nan || !right_has_non_nan {
            return Some(ValueRange::Float {
                bounds: None,
                format,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false,
            });
        }

        // track NaN and infinity possibilities
        let mut can_be_nan = left_nan || right_nan;
        let mut can_be_pos_inf = false;
        let mut can_be_neg_inf = false;

        // infinities divided by infinities yield NaN
        if (left_pos_inf || left_neg_inf) && (right_pos_inf || right_neg_inf) {
            can_be_nan = true;
        }

        // short circuit when the numerator is exactly zero
        if let Some(left_bounds) = left_bounds
            && FloatBounds::is_zero(&left_bounds)
        {
            let right_is_nan_only = right_bounds.is_none() && !right_pos_inf && !right_neg_inf;
            let right_is_zero = right_bounds.as_ref().is_some_and(FloatBounds::is_zero);
            let right_contains_zero = right_bounds
                .as_ref()
                .is_some_and(FloatBounds::contains_zero);
            if right_is_nan_only || (right_is_zero && !right_pos_inf && !right_neg_inf) {
                return Some(ValueRange::Float {
                    bounds: None,
                    format,
                    can_be_nan: true,
                    can_be_pos_inf: false,
                    can_be_neg_inf: false,
                });
            }
            if right_contains_zero {
                can_be_nan = true;
            }

            return Some(ValueRange::Float {
                bounds: Some(FloatBounds { min: 0.0, max: 0.0 }),
                format,
                can_be_nan,
                can_be_pos_inf: false,
                can_be_neg_inf: false,
            });
        }

        // handle division by NaN only denominator
        if right_bounds.is_none() && !right_pos_inf && !right_neg_inf {
            return Some(ValueRange::Float {
                bounds: None,
                format,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false,
            });
        }

        // infinite only divided by infinite only yields NaN
        if left_bounds.is_none()
            && (left_pos_inf || left_neg_inf)
            && right_bounds.is_none()
            && (right_pos_inf || right_neg_inf)
        {
            return Some(ValueRange::Float {
                bounds: None,
                format,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false,
            });
        }

        // handle division by infinite-only denominator
        if right_bounds.is_none() && (right_pos_inf || right_neg_inf) {
            return Some(ValueRange::Float {
                bounds: Some(FloatBounds { min: 0.0, max: 0.0 }),
                format,
                can_be_nan,
                can_be_pos_inf: false,
                can_be_neg_inf: false,
            });
        }

        // handle infinite numerator divided by finite denominator
        if left_pos_inf && let Some(right_bounds) = right_bounds {
            if FloatBounds::can_be_positive(&right_bounds) {
                can_be_pos_inf = true;
            }
            if FloatBounds::can_be_negative(&right_bounds) {
                can_be_neg_inf = true;
            }
        }
        if left_neg_inf && let Some(right_bounds) = right_bounds {
            if FloatBounds::can_be_positive(&right_bounds) {
                can_be_neg_inf = true;
            }
            if FloatBounds::can_be_negative(&right_bounds) {
                can_be_pos_inf = true;
            }
        }

        // return full range when dividing by a range that includes zero
        if let Some(right_bounds) = right_bounds
            && FloatBounds::contains_zero(&right_bounds)
        {
            if left_bounds.is_none() && (left_pos_inf || left_neg_inf) {
                can_be_pos_inf = true;
                can_be_neg_inf = true;

                return Some(ValueRange::Float {
                    bounds: None,
                    format,
                    can_be_nan,
                    can_be_pos_inf,
                    can_be_neg_inf,
                });
            }

            let left_contains_zero = left_bounds.as_ref().is_some_and(FloatBounds::contains_zero);
            let left_is_zero = left_bounds.as_ref().is_some_and(FloatBounds::is_zero);
            let left_has_non_zero = left_bounds.is_some() && !left_is_zero;

            if left_has_non_zero {
                let bounds = ValueRange::float_finite_bounds(format);
                if left_contains_zero {
                    can_be_nan = true;
                }

                return Some(ValueRange::Float {
                    bounds: Some(bounds),
                    format,
                    can_be_nan,
                    can_be_pos_inf: true,
                    can_be_neg_inf: true,
                });
            }

            return Some(ValueRange::float_full(format));
        }

        // derive finite bounds from candidates
        let mut bounds = match (left_bounds, right_bounds) {
            (Some(left), Some(right)) => {
                let candidates = [
                    ValueRange::float_divide_value(format, left.min, right.min),
                    ValueRange::float_divide_value(format, left.min, right.max),
                    ValueRange::float_divide_value(format, left.max, right.min),
                    ValueRange::float_divide_value(format, left.max, right.max),
                ];

                let (bounds, div_pos_inf, div_neg_inf) = FloatBounds::from_candidates(&candidates);
                can_be_pos_inf |= div_pos_inf;
                can_be_neg_inf |= div_neg_inf;
                bounds
            }
            _ => None,
        };

        // include zero when dividing by an infinite denominator
        if let Some(bounds) = &mut bounds
            && (right_pos_inf || right_neg_inf)
            && left_bounds.is_some()
        {
            if bounds.min > 0.0 {
                bounds.min = 0.0;
            }
            if bounds.max < 0.0 {
                bounds.max = 0.0;
            }
        }

        Some(ValueRange::Float {
            bounds,
            format,
            can_be_nan,
            can_be_pos_inf,
            can_be_neg_inf,
        })
    }

    /// Compute a range for float negation.
    fn float_negate(range: &ValueRange) -> Option<ValueRange> {
        // extract operand range
        let (bounds, format, can_be_nan, can_be_pos_inf, can_be_neg_inf) =
            ValueRange::float_fields(range)?;

        let bounds = bounds.map(|bounds| FloatBounds {
            min: ValueRange::float_negate_value(format, bounds.max),
            max: ValueRange::float_negate_value(format, bounds.min),
        });

        Some(ValueRange::Float {
            bounds,
            format,
            can_be_nan,
            can_be_pos_inf: can_be_neg_inf,
            can_be_neg_inf: can_be_pos_inf,
        })
    }

    /// Compute a float range for cast operations between floats.
    fn float_cast(argument: &ValueRange, to_format: mir::FloatType) -> Option<ValueRange> {
        let (bounds, _format, can_be_nan, mut can_be_pos_inf, mut can_be_neg_inf) =
            ValueRange::float_fields(argument)?;

        let bounds = match bounds {
            Some(bounds) => {
                let candidates = [
                    ValueRange::float_round(to_format, bounds.min),
                    ValueRange::float_round(to_format, bounds.max),
                ];
                let (bounds, cast_pos_inf, cast_neg_inf) =
                    FloatBounds::from_candidates(&candidates);
                can_be_pos_inf |= cast_pos_inf;
                can_be_neg_inf |= cast_neg_inf;
                bounds
            }
            None => None,
        };

        Some(ValueRange::Float {
            bounds,
            format: to_format,
            can_be_nan,
            can_be_pos_inf,
            can_be_neg_inf,
        })
    }

    /// Convert an integer range to a float range.
    fn from_integer(
        argument: &ValueRange,
        to_format: mir::FloatType,
        operator: mir::CastOperator,
    ) -> Option<ValueRange> {
        let (min, max, _width, is_signed) = ValueRange::integer_fields(argument)?;
        let expect_signed = match operator {
            mir::CastOperator::SignedIntToFloat => true,
            mir::CastOperator::UnsignedIntToFloat => false,
            _ => return None,
        };

        if is_signed != expect_signed {
            return None;
        }

        // convert integer bounds to float endpoints
        let min_value = ValueRange::float_from_integer(to_format, min);
        let max_value = ValueRange::float_from_integer(to_format, max);

        // derive infinity flags from endpoints
        let mut can_be_pos_inf = min_value.is_infinite() && min_value.is_sign_positive();
        let mut can_be_neg_inf = min_value.is_infinite() && min_value.is_sign_negative();
        can_be_pos_inf |= max_value.is_infinite() && max_value.is_sign_positive();
        can_be_neg_inf |= max_value.is_infinite() && max_value.is_sign_negative();

        // compute finite bounds while clamping infinities
        let bounds = if min_value.is_infinite()
            && max_value.is_infinite()
            && min_value.is_sign_positive() == max_value.is_sign_positive()
        {
            None
        } else {
            let max_finite = ValueRange::float_max_finite(to_format);
            let mut min_bound = min_value;
            let mut max_bound = max_value;

            if min_bound.is_infinite() {
                min_bound = -max_finite;
            }

            if max_bound.is_infinite() {
                max_bound = max_finite;
            }

            if min_bound > max_bound {
                std::mem::swap(&mut min_bound, &mut max_bound);
            }

            Some(FloatBounds {
                min: min_bound,
                max: max_bound,
            })
        };

        Some(ValueRange::Float {
            bounds,
            format: to_format,
            can_be_nan: false,
            can_be_pos_inf,
            can_be_neg_inf,
        })
    }

    /// Convert a float range to an integer range.
    fn from_float(
        argument: &ValueRange,
        to_width: u16,
        to_signed: bool,
        operator: mir::CastOperator,
    ) -> Option<ValueRange> {
        let (bounds, _width, can_be_nan, can_be_pos_inf, can_be_neg_inf) =
            ValueRange::float_fields(argument)?;
        let (expect_signed, is_saturating) = match operator {
            mir::CastOperator::FloatToSignedInt => (true, false),
            mir::CastOperator::FloatToUnsignedInt => (false, false),
            mir::CastOperator::FloatToSignedIntSaturating => (true, true),
            mir::CastOperator::FloatToUnsignedIntSaturating => (false, true),
            _ => return None,
        };

        if to_signed != expect_signed {
            return None;
        }

        // resolve integer bounds for the target type
        let (min_bound, max_bound) = ValueRange::integer_bounds(to_width, to_signed)?;

        if !is_saturating {
            if can_be_nan || can_be_pos_inf || can_be_neg_inf {
                return None;
            }

            let bounds = bounds?;

            if !ValueRange::float_within_integer_bounds(bounds.min, min_bound, max_bound)
                || !ValueRange::float_within_integer_bounds(bounds.max, min_bound, max_bound)
            {
                return None;
            }

            let mut min_value = bounds.min.trunc() as i128;
            let mut max_value = bounds.max.trunc() as i128;

            if min_value > max_value {
                std::mem::swap(&mut min_value, &mut max_value);
            }

            return Some(ValueRange::Integer {
                min: min_value,
                max: max_value,
                width: to_width,
                is_signed: to_signed,
            });
        }

        let mut candidates = Vec::new();

        if can_be_nan {
            candidates.push(0);
        }

        if can_be_pos_inf {
            candidates.push(max_bound);
        }

        if can_be_neg_inf {
            candidates.push(min_bound);
        }

        if let Some(bounds) = bounds {
            let min_value =
                ValueRange::float_to_saturating_integer(bounds.min, min_bound, max_bound);
            let max_value =
                ValueRange::float_to_saturating_integer(bounds.max, min_bound, max_bound);
            candidates.push(min_value);
            candidates.push(max_value);
        }

        if candidates.is_empty() {
            return Some(ValueRange::Integer {
                min: min_bound,
                max: max_bound,
                width: to_width,
                is_signed: to_signed,
            });
        }

        let mut min_value = *candidates.iter().min()?;
        let mut max_value = *candidates.iter().max()?;

        if min_value > max_value {
            std::mem::swap(&mut min_value, &mut max_value);
        }

        Some(ValueRange::Integer {
            min: min_value,
            max: max_value,
            width: to_width,
            is_signed: to_signed,
        })
    }

    /// Return true when a float is finite and within the target integer range.
    fn float_within_integer_bounds(value: f64, min_bound: i128, max_bound: i128) -> bool {
        if !value.is_finite() {
            return false;
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

        min_ok && max_ok
    }

    /// Convert a float to an integer with saturation.
    fn float_to_saturating_integer(value: f64, min_bound: i128, max_bound: i128) -> i128 {
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

    /// Evaluate comparison ranges for floats and return constant booleans when possible.
    fn compare_float(
        operator: mir::BinaryOperator,
        left: &ValueRange,
        right: &ValueRange,
    ) -> Option<ValueRange> {
        let (left_bounds, format, left_nan, left_pos_inf, left_neg_inf) =
            ValueRange::float_fields(left)?;
        let (right_bounds, right_format, right_nan, right_pos_inf, right_neg_inf) =
            ValueRange::float_fields(right)?;

        if format != right_format {
            return None;
        }

        let can_be_nan = left_nan || right_nan;

        let left_has_non_nan = left_bounds.is_some() || left_pos_inf || left_neg_inf;
        let right_has_non_nan = right_bounds.is_some() || right_pos_inf || right_neg_inf;

        if !left_has_non_nan || !right_has_non_nan {
            return match operator {
                mir::BinaryOperator::NotEqual => Some(ValueRange::Boolean {
                    can_be_true: true,
                    can_be_false: false,
                }),
                mir::BinaryOperator::Equal
                | mir::BinaryOperator::LessThan
                | mir::BinaryOperator::LessEqual
                | mir::BinaryOperator::GreaterThan
                | mir::BinaryOperator::GreaterEqual => Some(ValueRange::Boolean {
                    can_be_true: false,
                    can_be_false: true,
                }),
                _ => None,
            };
        }
        let left_bounds = FloatBounds::effective(left_bounds, left_pos_inf, left_neg_inf)?;
        let right_bounds = FloatBounds::effective(right_bounds, right_pos_inf, right_neg_inf)?;

        let mut is_always_true = false;
        let mut is_always_false = false;

        match operator {
            mir::BinaryOperator::Equal => {
                if left_bounds.max < right_bounds.min || right_bounds.max < left_bounds.min {
                    is_always_false = true;
                } else if left_bounds.min == left_bounds.max
                    && left_bounds.min == right_bounds.min
                    && right_bounds.min == right_bounds.max
                    && !can_be_nan
                {
                    is_always_true = true;
                }
            }
            mir::BinaryOperator::NotEqual => {
                if left_bounds.min == left_bounds.max
                    && left_bounds.min == right_bounds.min
                    && right_bounds.min == right_bounds.max
                    && !can_be_nan
                {
                    is_always_false = true;
                } else if left_bounds.max < right_bounds.min || right_bounds.max < left_bounds.min {
                    is_always_true = true;
                }
            }
            mir::BinaryOperator::LessThan => {
                if left_bounds.max < right_bounds.min && !can_be_nan {
                    is_always_true = true;
                } else if left_bounds.min >= right_bounds.max {
                    is_always_false = true;
                }
            }
            mir::BinaryOperator::LessEqual => {
                if left_bounds.max <= right_bounds.min && !can_be_nan {
                    is_always_true = true;
                } else if left_bounds.min > right_bounds.max {
                    is_always_false = true;
                }
            }
            mir::BinaryOperator::GreaterThan => {
                if left_bounds.min > right_bounds.max && !can_be_nan {
                    is_always_true = true;
                } else if left_bounds.max <= right_bounds.min {
                    is_always_false = true;
                }
            }
            mir::BinaryOperator::GreaterEqual => {
                if left_bounds.min >= right_bounds.max && !can_be_nan {
                    is_always_true = true;
                } else if left_bounds.max < right_bounds.min {
                    is_always_false = true;
                }
            }
            _ => return None,
        }

        if is_always_true {
            return Some(ValueRange::Boolean {
                can_be_true: true,
                can_be_false: false,
            });
        }

        if is_always_false {
            return Some(ValueRange::Boolean {
                can_be_true: false,
                can_be_false: true,
            });
        }

        None
    }

    /// Evaluate comparison ranges and return constant booleans when possible.
    fn compare(
        operator: mir::BinaryOperator,
        left: &ValueRange,
        right: &ValueRange,
    ) -> Option<ValueRange> {
        // extract operand ranges
        let (left_min, left_max, width, is_signed) = ValueRange::integer_fields(left)?;
        let (right_min, right_max, right_width, right_signed) = ValueRange::integer_fields(right)?;

        // require matching integer types
        if width != right_width || is_signed != right_signed {
            return None;
        }

        // compute comparison guarantees
        let mut is_always_true = false;
        let mut is_always_false = false;

        match operator {
            mir::BinaryOperator::Equal => {
                if left_max < right_min || right_max < left_min {
                    is_always_false = true;
                } else if left_min == left_max && left_min == right_min && right_min == right_max {
                    is_always_true = true;
                }
            }
            mir::BinaryOperator::NotEqual => {
                if left_max < right_min || right_max < left_min {
                    is_always_true = true;
                } else if left_min == left_max && left_min == right_min && right_min == right_max {
                    is_always_false = true;
                }
            }
            mir::BinaryOperator::LessThan => {
                if left_max < right_min {
                    is_always_true = true;
                } else if left_min >= right_max {
                    is_always_false = true;
                }
            }
            mir::BinaryOperator::LessEqual => {
                if left_max <= right_min {
                    is_always_true = true;
                } else if left_min > right_max {
                    is_always_false = true;
                }
            }
            mir::BinaryOperator::GreaterThan => {
                if left_min > right_max {
                    is_always_true = true;
                } else if left_max <= right_min {
                    is_always_false = true;
                }
            }
            mir::BinaryOperator::GreaterEqual => {
                if left_min >= right_max {
                    is_always_true = true;
                } else if left_max < right_min {
                    is_always_false = true;
                }
            }
            _ => return None,
        }

        // return a constant boolean when the comparison is decided
        if is_always_true {
            return Some(ValueRange::Boolean {
                can_be_true: true,
                can_be_false: false,
            });
        }

        // return a constant boolean when the comparison is decided
        if is_always_false {
            return Some(ValueRange::Boolean {
                can_be_true: false,
                can_be_false: true,
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// Constants appear as exact ranges at block exit.
    #[test]
    fn test_range_from_constant() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 5
    v2: int32 = add v1, v0
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();
        let exit_ranges = ranges.exit(block0);
        let constant_range = exit_ranges.get(value).unwrap();

        assert_eq!(
            constant_range,
            &ValueRange::Integer {
                min: 5,
                max: 5,
                width: 32,
                is_signed: true
            }
        );
    }

    /// Block parameters merge to a union of incoming ranges.
    #[test]
    fn test_range_param_union() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 1
    jump b3(v1)

b2:
    v2: int32 = 3
    jump b3(v2)

b3(v3: int32):
    return v3
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let merge_block = function.block(3);
        let merge = test.tree.get(merge_block);
        let param_value = merge.parameters[0].value;

        let entry_ranges = ranges.entry(merge_block);
        let param_range = entry_ranges.get(param_value).unwrap();

        assert_eq!(
            param_range,
            &ValueRange::Integer {
                min: 1,
                max: 3,
                width: 32,
                is_signed: true
            }
        );
    }

    /// Fallible allocation result parameters do not consume edge arguments.
    #[test]
    fn test_range_fallible_allocation_success_argument() {
        let test = TestProgram::new(
            r#"
function test(v0: int64): int32 {
entry(v0: int64):
    v1: int32 = 7
    new.slice.uninit.try int32, v0 => b1(v1) | b2

b1(v2: uninit<slice<int32, managed, mutable>>, v3: int32):
    return v3

b2:
    v4: int32 = 0
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let success = function.block(1);
        let success_block = test.tree.get(success);
        let argument = success_block.parameters[1].value;
        let range = ranges.entry(success).get(argument);

        assert_eq!(
            range,
            Some(&ValueRange::Integer {
                min: 7,
                max: 7,
                width: 32,
                is_signed: true,
            })
        );
    }

    /// Unknown inputs prevent ranges from being inferred.
    #[test]
    fn test_range_param_unknown() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean, v1: int32): int32 {
entry(v0: boolean, v1: int32):
    branch v0 => b1 | b2

b1:
    v2: int32 = 1
    jump b3(v2)

b2:
    jump b3(v1)

b3(v3: int32):
    return v3
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let merge_block = function.block(3);
        let merge = test.tree.get(merge_block);
        let param_value = merge.parameters[0].value;

        let entry_ranges = ranges.entry(merge_block);
        assert!(entry_ranges.get(param_value).is_none());
    }

    /// Arithmetic ranges are propagated through instructions.
    #[test]
    fn test_range_arithmetic_add() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 2
    jump b3(v1)

b2:
    v2: int32 = 4
    jump b3(v2)

b3(v3: int32):
    v4: int32 = 1
    v5: int32 = add v3, v4
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Integer {
                min: 3,
                max: 5,
                width: 32,
                is_signed: true
            }
        );
    }

    /// Comparisons that are always true become constant ranges.
    #[test]
    fn test_range_comparison_constant() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = 1
    jump b3(v1)

b2:
    v2: int32 = 3
    jump b3(v2)

b3(v3: int32):
    v4: int32 = 10
    v5: boolean = lt v3, v4
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Boolean {
                can_be_true: true,
                can_be_false: false
            }
        );
    }

    /// Float constants appear as exact ranges at block exit.
    #[test]
    fn test_range_float_constant() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: float32 = 1.5
    return v0
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();
        let exit_ranges = ranges.exit(block0);
        let constant_range = exit_ranges.get(value).unwrap();

        assert_eq!(
            constant_range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 1.5, max: 1.5 }),
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Float arithmetic ranges are propagated through instructions.
    #[test]
    fn test_range_float_arithmetic_add() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 1
    jump b3(v1)

b2:
    v2: float32 = 3
    jump b3(v2)

b3(v3: float32):
    v4: float32 = 2
    v5: float32 = add v3, v4
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 3.0, max: 5.0 }),
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Infinite addition with opposite signs yields NaN only.
    #[test]
    fn test_range_float_add_infinite_nan_only() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = inf
    v2: float32 = -inf
    jump b3(v1, v2)

b2:
    v3: float32 = inf
    v4: float32 = -inf
    jump b3(v3, v4)

b3(v5: float32, v6: float32):
    v7: float32 = add v5, v6
    return v7
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Positive infinity plus finite values stays infinite.
    #[test]
    fn test_range_float_add_infinite_and_finite() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: float32 = inf
    v1: float32 = 2
    v2: float32 = add v0, v1
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: true,
                can_be_neg_inf: false
            }
        );
    }

    /// Infinite subtraction with equal signs yields NaN only.
    #[test]
    fn test_range_float_sub_infinite_nan_only() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = inf
    v2: float32 = inf
    jump b3(v1, v2)

b2:
    v3: float32 = inf
    v4: float32 = inf
    jump b3(v3, v4)

b3(v5: float32, v6: float32):
    v7: float32 = sub v5, v6
    return v7
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Finite values minus positive infinity yield negative infinity.
    #[test]
    fn test_range_float_sub_finite_minus_infinite() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: float32 = 2
    v1: float32 = inf
    v2: float32 = sub v0, v1
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: false,
                can_be_neg_inf: true
            }
        );
    }

    /// Float comparisons that are always true become constant ranges.
    #[test]
    fn test_range_float_comparison_constant() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 1
    jump b3(v1)

b2:
    v2: float32 = 2
    jump b3(v2)

b3(v3: float32):
    v4: float32 = 5
    v5: boolean = lt v3, v4
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Boolean {
                can_be_true: true,
                can_be_false: false
            }
        );
    }

    /// Float comparisons against full range values remain unknown.
    #[test]
    fn test_range_float_comparison_full_range() {
        let test = TestProgram::new(
            r#"
function test(): boolean {
entry:
    v0: float32 = 1
    v1: float32 = 0
    v2: float32 = div v0, v1
    v3: float32 = 5
    v4: boolean = gt v2, v3
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[4];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        assert!(exit_ranges.get(value).is_none());
    }

    /// Float division with both operands able to reach zero yields full bounds.
    #[test]
    fn test_range_float_division_by_zero() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean, v1: boolean): float32 {
entry(v0: boolean, v1: boolean):
    branch v0 => b1 | b2

b1:
    v2: float32 = 0
    jump b3(v2, v1)

b2:
    v3: float32 = 1
    jump b3(v3, v1)

b3(v4: float32, v5: boolean):
    branch v5 => b4 | b5

b4:
    v6: float32 = 0
    jump b6(v4, v6)

b5:
    v7: float32 = 1
    jump b6(v4, v7)

b6(v8: float32, v9: float32):
    v10: float32 = div v9, v8
    return v10
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block6 = function.block(6);
        let block = test.tree.get(block6);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block6);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: Some(FloatBounds {
                    min: -(f32::MAX as f64),
                    max: f32::MAX as f64
                }),
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: true,
                can_be_neg_inf: true
            }
        );
    }

    /// Nonzero divided by a range including zero yields infinities without NaN.
    #[test]
    fn test_range_float_division_nonzero_by_zero_range() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 0
    jump b3(v1)

b2:
    v2: float32 = 1
    jump b3(v2)

b3(v3: float32):
    v4: float32 = 2
    v5: float32 = div v4, v3
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: Some(FloatBounds {
                    min: -(f32::MAX as f64),
                    max: f32::MAX as f64
                }),
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: true,
                can_be_neg_inf: true
            }
        );
    }

    /// Zero divided by a range including zero stays at zero with NaN possible.
    #[test]
    fn test_range_float_division_zero_by_zero_range() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 0
    jump b3(v1)

b2:
    v2: float32 = 1
    jump b3(v2)

b3(v3: float32):
    v4: float32 = 0
    v5: float32 = div v4, v3
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 0.0, max: 0.0 }),
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Zero divided by zero or infinity stays at zero with NaN possible.
    #[test]
    fn test_range_float_division_zero_by_zero_or_infinite() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 0
    jump b3(v1)

b2:
    v2: float32 = inf
    jump b3(v2)

b3(v3: float32):
    v4: float32 = 0
    v5: float32 = div v4, v3
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 0.0, max: 0.0 }),
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Zero multiplied by infinity yields NaN only.
    #[test]
    fn test_range_float_multiply_zero_by_infinite() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 0
    v2: float32 = inf
    jump b3(v1, v2)

b2:
    v3: float32 = 0
    v4: float32 = inf
    jump b3(v3, v4)

b3(v5: float32, v6: float32):
    v7: float32 = mul v5, v6
    return v7
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Infinity times a range containing zero yields NaN with infinity.
    #[test]
    fn test_range_float_multiply_infinite_by_zero_range() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 0
    jump b3(v1)

b2:
    v2: float32 = 2
    jump b3(v2)

b3(v3: float32):
    v4: float32 = inf
    v5: float32 = mul v4, v3
    return v5
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: true,
                can_be_neg_inf: false
            }
        );
    }

    /// Infinite divided by infinite yields NaN only.
    #[test]
    fn test_range_float_division_infinite_by_infinite() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = inf
    v2: float32 = inf
    jump b3(v1, v2)

b2:
    v3: float32 = inf
    v4: float32 = inf
    jump b3(v3, v4)

b3(v5: float32, v6: float32):
    v7: float32 = div v5, v6
    return v7
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Infinite divided by zero yields infinities with unknown sign.
    #[test]
    fn test_range_float_division_infinite_by_zero() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: float32 = inf
    v1: float32 = 0
    v2: float32 = div v0, v1
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: true,
                can_be_neg_inf: true
            }
        );
    }

    /// Finite values divided by infinity yield zero.
    #[test]
    fn test_range_float_division_finite_by_infinite() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: float32 = 2
    v1: float32 = inf
    v2: float32 = div v0, v1
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 0.0, max: 0.0 }),
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Positive infinity divided by negative finite values yields negative infinity.
    #[test]
    fn test_range_float_division_infinite_by_negative() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: float32 = inf
    v1: float32 = -2
    v2: float32 = div v0, v1
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: false,
                can_be_neg_inf: true
            }
        );
    }

    /// Zero multiplied by a range that can be infinite stays at zero with NaN possible.
    #[test]
    fn test_range_float_multiply_zero_by_full_range() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: float32 = 1
    v1: float32 = 0
    v2: float32 = div v0, v1
    v3: float32 = 0
    v4: float32 = mul v3, v2
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[4];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 0.0, max: 0.0 }),
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Float to signed int casts truncate finite constants.
    #[test]
    fn test_range_float_to_int_cast() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: float32 = 3.9000000953674316
    v1: int32 = cast.floatToInt.s v0 -> int32
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Integer {
                min: 3,
                max: 3,
                width: 32,
                is_signed: true
            }
        );
    }

    /// Saturating float to signed int casts clamp out of range values.
    #[test]
    fn test_range_float_to_int_saturating_bounds() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 1
    jump b3(v1)

b2:
    v2: float32 = 100000002004087730000
    jump b3(v2)

b3(v3: float32):
    v4: int32 = cast.floatToIntSaturating.s v3 -> int32
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Integer {
                min: 1,
                max: 2_147_483_647,
                width: 32,
                is_signed: true
            }
        );
    }

    /// Saturating integer casts clamp merged integer ranges.
    #[test]
    fn test_range_int_saturating_cast() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int8 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: int32 = -300
    jump b3(v1)

b2:
    v2: int32 = 300
    jump b3(v2)

b3(v3: int32):
    v4: int8 = cast.saturate v3 -> int8
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Integer {
                min: i8::MIN as i128,
                max: i8::MAX as i128,
                width: 8,
                is_signed: true
            }
        );
    }

    /// Saturating float to signed int casts map NaN to zero.
    #[test]
    fn test_range_float_to_int_saturating_nan_only() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: float32 = 0
    v1: float32 = div v0, v0
    v2: int32 = cast.floatToIntSaturating.s v1 -> int32
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Integer {
                min: 0,
                max: 0,
                width: 32,
                is_signed: true
            }
        );
    }

    /// Signed int to float casts preserve the range.
    #[test]
    fn test_range_int_to_float_cast() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: int32 = 4
    v1: float32 = cast.intToFloat.s v0 -> float32
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 4.0, max: 4.0 }),
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// Float truncation saturates large f64 values to infinity.
    #[test]
    fn test_range_float_truncate_overflow() {
        let test = TestProgram::new(
            r#"
function test(): float32 {
entry:
    v0: float64 = 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
    v1: float32 = cast.floatTruncate v0 -> float32
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        let range = exit_ranges.get(value).unwrap();

        assert_eq!(
            range,
            &ValueRange::Float {
                bounds: None,
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: true,
                can_be_neg_inf: false
            }
        );
    }

    /// Merging NaN only and finite values preserves finite bounds with NaN possible.
    #[test]
    fn test_range_float_union_nan_only() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 0
    v2: float32 = div v1, v1
    jump b3(v2)

b2:
    v3: float32 = 1
    jump b3(v3)

b3(v4: float32):
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let merge_block = function.block(3);
        let merge = test.tree.get(merge_block);
        let param_value = merge.parameters[0].value;

        let entry_ranges = ranges.entry(merge_block);
        let param_range = entry_ranges.get(param_value).unwrap();

        assert_eq!(
            param_range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 1.0, max: 1.0 }),
                format: mir::FloatType::Float32,
                can_be_nan: true,
                can_be_pos_inf: false,
                can_be_neg_inf: false
            }
        );
    }

    /// NaN comparisons against equality are always false.
    #[test]
    fn test_range_float_comparison_nan_eq_false() {
        let test = TestProgram::new(
            r#"
function test(): boolean {
entry:
    v0: float32 = 0
    v1: float32 = div v0, v0
    v2: float32 = 1
    v3: boolean = eq v1, v2
    return v3
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[3];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        assert_eq!(
            exit_ranges.get(value).unwrap(),
            &ValueRange::Boolean {
                can_be_true: false,
                can_be_false: true
            }
        );
    }

    /// Positive infinity comparisons against finite values are constant.
    #[test]
    fn test_range_float_comparison_infinite_gt_finite() {
        let test = TestProgram::new(
            r#"
function test(): boolean {
entry:
    v0: float32 = inf
    v1: float32 = 1
    v2: boolean = gt v0, v1
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        assert_eq!(
            exit_ranges.get(value).unwrap(),
            &ValueRange::Boolean {
                can_be_true: true,
                can_be_false: false
            }
        );
    }

    /// Positive infinity comparisons against finite values are constant for less equal.
    #[test]
    fn test_range_float_comparison_infinite_le_finite() {
        let test = TestProgram::new(
            r#"
function test(): boolean {
entry:
    v0: float32 = inf
    v1: float32 = 1
    v2: boolean = le v0, v1
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        assert_eq!(
            exit_ranges.get(value).unwrap(),
            &ValueRange::Boolean {
                can_be_true: false,
                can_be_false: true
            }
        );
    }

    /// NaN comparisons against inequality are always true.
    #[test]
    fn test_range_float_comparison_nan_ne_true() {
        let test = TestProgram::new(
            r#"
function test(): boolean {
entry:
    v0: float32 = 0
    v1: float32 = div v0, v0
    v2: float32 = 1
    v3: boolean = ne v1, v2
    return v3
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[3];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        assert_eq!(
            exit_ranges.get(value).unwrap(),
            &ValueRange::Boolean {
                can_be_true: true,
                can_be_false: false
            }
        );
    }

    /// NaN mixed with finite values keeps comparisons unknown for greater equal.
    #[test]
    fn test_range_float_comparison_nan_ge_unknown() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 0
    v2: float32 = div v1, v1
    jump b3(v2)

b2:
    v3: float32 = 1
    jump b3(v3)

b3(v4: float32):
    v5: float32 = 1
    v6: boolean = ge v4, v5
    return v6
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        assert!(exit_ranges.get(value).is_none());
    }

    /// Unions preserve finite bounds and track infinity.
    #[test]
    fn test_range_float_union_infinite_and_finite() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): float32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = inf
    jump b3(v1)

b2:
    v2: float32 = 2
    jump b3(v2)

b3(v3: float32):
    return v3
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let merge_block = function.block(3);
        let merge = test.tree.get(merge_block);
        let param_value = merge.parameters[0].value;

        let entry_ranges = ranges.entry(merge_block);
        let param_range = entry_ranges.get(param_value).unwrap();

        assert_eq!(
            param_range,
            &ValueRange::Float {
                bounds: Some(FloatBounds { min: 2.0, max: 2.0 }),
                format: mir::FloatType::Float32,
                can_be_nan: false,
                can_be_pos_inf: true,
                can_be_neg_inf: false
            }
        );
    }

    /// Float to int casts are unknown when values may be out of range.
    #[test]
    fn test_range_float_to_int_cast_bounds() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = 1
    jump b3(v1)

b2:
    v2: float32 = 100000002004087730000
    jump b3(v2)

b3(v3: float32):
    v4: int32 = cast.floatToInt.s v3 -> int32
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        assert!(exit_ranges.get(value).is_none());
    }

    /// Float to int casts are unknown when the input may be infinite.
    #[test]
    fn test_range_float_to_int_cast_positive_infinite() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: float32 = inf
    v1: int32 = cast.floatToInt.s v0 -> int32
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        assert!(exit_ranges.get(value).is_none());
    }

    /// Float to int casts are unknown when values may be out of range.
    #[test]
    fn test_range_float_to_int_cast_negative_bounds() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    v1: float32 = -1
    jump b3(v1)

b2:
    v2: float32 = -100000002004087730000
    jump b3(v2)

b3(v3: float32):
    v4: int32 = cast.floatToInt.s v3 -> int32
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block3 = function.block(3);
        let block = test.tree.get(block3);
        let instruction_id = block.instructions[0];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block3);
        assert!(exit_ranges.get(value).is_none());
    }

    /// Float to int casts are unknown when the input may be infinite.
    #[test]
    fn test_range_float_to_int_cast_negative_infinite() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: float32 = -inf
    v1: int32 = cast.floatToInt.s v0 -> int32
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[1];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        assert!(exit_ranges.get(value).is_none());
    }

    /// NaN only float to int casts are unknown.
    #[test]
    fn test_range_float_to_int_cast_nan_only() {
        let test = TestProgram::new(
            r#"
function test(): int32 {
entry:
    v0: float32 = 0
    v1: float32 = div v0, v0
    v2: int32 = cast.floatToInt.s v1 -> int32
    return v2
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let mut analyses = test.function_analyses();
        let ranges = analyses.range(function, &test.tree);

        let block0 = function.block(0);
        let block = test.tree.get(block0);
        let instruction_id = block.instructions[2];
        let instruction = test.tree.get(instruction_id);
        let value = instruction.destination().unwrap();

        let exit_ranges = ranges.exit(block0);
        assert!(exit_ranges.get(value).is_none());
    }
}
