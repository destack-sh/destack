use destack_core::{FxIndexMap, FxIndexSet, float_from_bits, float_to_bits};

use crate as mir;
use crate::{Analysis, DefinitionTable, NodeTable, TargetLayout, UseTable, ValueUse};

/// Constants and executable control-flow edges for one function.
#[derive(Debug)]
pub struct ConstantTable {
    /// Value lattices indexed by SSA value identity.
    values: Vec<ConstantValue>,
    /// Blocks executable under the solved constants.
    blocks: NodeTable<mir::Block, bool>,
    /// Exact executable edges, including distinct targets to the same block.
    edges: FxIndexSet<mir::Edge>,
}

/// Constant lattice for one scalar, aggregate, or variant value.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    /// No executable definition has supplied a value yet.
    Unknown,
    /// One scalar constant.
    Scalar(mir::Constant),
    /// Constant components of an aggregate.
    Aggregate(Vec<ConstantValue>),
    /// One known variant case and its payload.
    Variant {
        /// The logical case index.
        case: u32,
        /// The case's discriminant constant.
        discriminant: mir::Constant,
        /// The case payload, absent for a payloadless case.
        payload: Option<Box<ConstantValue>>,
    },
    /// Executable definitions do not agree on a constant.
    Overdefined,
}

impl ConstantValue {
    /// Merge another executable definition into this value.
    fn merge(&mut self, other: &Self) -> bool {
        // preserve lattice endpoints and identical constants
        if matches!(other, Self::Unknown) || matches!(self, Self::Overdefined) || self == other {
            return false;
        }
        if matches!(self, Self::Unknown) {
            *self = other.clone();
            return true;
        }

        // merge matching aggregate components and variant payloads independently
        match (&mut *self, other) {
            (Self::Aggregate(left), Self::Aggregate(right)) if left.len() == right.len() => {
                let mut changed = false;
                for (left, right) in left.iter_mut().zip(right) {
                    changed |= left.merge(right);
                }
                changed
            }
            (
                Self::Variant {
                    case: left_case,
                    payload: left,
                    ..
                },
                Self::Variant {
                    case: right_case,
                    payload: right,
                    ..
                },
            ) if left_case == right_case => match (left, right) {
                (Some(left), Some(right)) => left.merge(right),
                (None, None) => false,
                _ => unreachable!("one variant case has inconsistent payload shapes"),
            },
            _ => {
                *self = Self::Overdefined;
                true
            }
        }
    }

    /// Return the scalar constant, when one is known.
    pub fn scalar(&self) -> Option<&mir::Constant> {
        match self {
            Self::Scalar(constant) => Some(constant),
            _ => None,
        }
    }
}

impl ConstantTable {
    /// Analyse scalar and aggregate constants along executable control-flow edges.
    pub fn analyse(
        function: mir::FunctionId,
        uses: &UseTable,
        target: TargetLayout,
        tree: &mir::Tree,
    ) -> Self {
        ConstantSolver::new(function, uses, target, tree).solve(&FxIndexMap::default())
    }

    /// Analyse a function with explicitly supplied parameter constants.
    pub fn with_parameter_constants(
        function: mir::FunctionId,
        tree: &mir::Tree,
        target: TargetLayout,
        constants: &FxIndexMap<mir::Value, mir::Constant>,
    ) -> Self {
        let uses = UseTable::analyse(tree.get(function), tree);

        ConstantSolver::new(function, &uses, target, tree).solve(constants)
    }

    /// Return the lattice value solved for one SSA definition.
    pub fn value(&self, value: mir::Value) -> &ConstantValue {
        &self.values[value.id() as usize]
    }

    /// Return the scalar constant solved for one SSA definition.
    pub fn constant(&self, value: mir::Value) -> Option<&mir::Constant> {
        self.value(value).scalar()
    }

    /// Return whether a block is executable under the solved constants.
    pub fn is_executable(&self, block: mir::BlockId) -> bool {
        *self.blocks.get(block)
    }

    /// Return whether an exact control-flow edge is executable.
    pub fn is_edge_executable(&self, edge: mir::Edge) -> bool {
        self.edges.contains(&edge)
    }
}

impl ConstantLookup for ConstantTable {
    /// Return the scalar constant for one SSA definition.
    fn get_constant(&self, value: mir::Value) -> Option<&mir::Constant> {
        self.constant(value)
    }
}

impl Analysis for ConstantTable {
    const INVALIDATED_BY: mir::Mutation = mir::Mutation::CONTROL
        .union(mir::Mutation::VALUE)
        .union(mir::Mutation::LAYOUT);
}

/// Sparse worklist for executable edges and their dependent SSA operations.
struct ConstantSolver<'a> {
    /// The function whose constants are being solved.
    function: mir::FunctionId,
    /// The MIR definitions and operands.
    tree: &'a mir::Tree,
    /// The target's scalar widths.
    target: TargetLayout,
    /// Dependent operations grouped by their used values.
    uses: &'a UseTable,
    /// Operations awaiting reevaluation.
    pending: FxIndexSet<mir::Point>,
    /// The current value and edge lattices.
    table: ConstantTable,
}

impl<'a> ConstantSolver<'a> {
    /// Allocate the value lattices and index their dependent operations.
    fn new(
        function: mir::FunctionId,
        uses: &'a UseTable,
        target: TargetLayout,
        tree: &'a mir::Tree,
    ) -> Self {
        // allocate executable block states and the initial operation worklist
        let blocks = NodeTable::from_nodes(tree.get(function).blocks(), || false);
        let mut pending = FxIndexSet::default();
        if let Some(entry) = tree.get(function).entry() {
            pending.reserve(tree.get(entry).instructions.len() + 1);
        }

        let count = tree.get(function).value_capacity();

        Self {
            function,
            tree,
            target,
            uses,
            pending,
            table: ConstantTable {
                values: vec![ConstantValue::Unknown; count],
                blocks,
                edges: FxIndexSet::default(),
            },
        }
    }

    /// Solve value changes and newly executable edges to a fixed point.
    fn solve(mut self, parameters: &FxIndexMap<mir::Value, mir::Constant>) -> ConstantTable {
        let Some(entry) = self.tree.get(self.function).entry() else {
            return self.table;
        };

        // seed runtime parameters and explicitly supplied constants
        for parameter in &self.tree.get(self.function).parameters {
            let value = parameters
                .get(&parameter.value)
                .map_or(ConstantValue::Overdefined, |constant| {
                    ConstantValue::Scalar(constant.clone())
                });
            self.table.values[parameter.value.id() as usize] = value;
        }
        self.activate(entry);

        // reevaluate only operations affected by a value or executable edge
        while let Some(point) = self.pending.pop() {
            match point {
                mir::Point::Instruction(instruction) => self.instruction(instruction),
                mir::Point::Terminator(block) => self.terminator(block),
            }
        }

        self.table
    }

    /// Schedule each operation when a block first becomes executable.
    fn activate(&mut self, block: mir::BlockId) {
        if *self.table.blocks.get(block) {
            return;
        }
        *self.table.blocks.get_mut(block) = true;

        // evaluate instructions in source order before the terminator
        self.pending.insert(mir::Point::Terminator(block));
        for &instruction in self.tree.get(block).instructions.iter().rev() {
            self.pending.insert(mir::Point::Instruction(instruction));
        }
    }

    /// Merge an executable value and schedule its dependent operations.
    fn update(&mut self, value: mir::Value, incoming: ConstantValue) {
        if !self.table.values[value.id() as usize].merge(&incoming) {
            return;
        }

        // schedule only uses in executable blocks
        for use_site in self.uses.uses(value) {
            if !self.table.is_executable(use_site.block()) {
                continue;
            }
            let point = match use_site {
                ValueUse::Instruction { instruction, .. } => mir::Point::Instruction(*instruction),
                ValueUse::Terminator { block, .. } => mir::Point::Terminator(*block),
            };
            self.pending.insert(point);
        }
    }

    /// Evaluate one instruction's result under the current operand lattices.
    fn instruction(&mut self, id: mir::LocalNodeId<mir::Instruction>) {
        let instruction = &self.tree.get(id).clone();
        let Some(destination) = instruction.destination() else {
            return;
        };

        // propagate aggregate components and selected values without flattening their lattices
        let result = match instruction {
            mir::Instruction::Intrinsic {
                intrinsic:
                    intrinsic @ (mir::Intrinsic::AddOverflow
                    | mir::Intrinsic::SubOverflow
                    | mir::Intrinsic::MulOverflow),
                arguments,
                ..
            } => {
                let arguments = self.tree.get_values(*arguments);
                if arguments
                    .iter()
                    .any(|value| matches!(self.table.value(*value), ConstantValue::Unknown))
                {
                    ConstantValue::Unknown
                } else {
                    let constants = arguments
                        .iter()
                        .map(|value| self.table.constant(*value).cloned())
                        .collect::<Option<Vec<_>>>();
                    let folded =
                        constants.and_then(|arguments| fold_overflow(*intrinsic, &arguments));

                    folded.map_or(ConstantValue::Overdefined, |(value, overflow)| {
                        ConstantValue::Aggregate(vec![
                            ConstantValue::Scalar(value),
                            ConstantValue::Scalar(mir::Constant::Boolean { value: overflow }),
                        ])
                    })
                }
            }
            mir::Instruction::Copy { value, .. } => self.table.value(*value).clone(),
            mir::Instruction::Const { value, .. } => match value {
                mir::Constant::Uninit
                | mir::Constant::Parameter(_)
                | mir::Constant::Layout { .. }
                | mir::Constant::Witness { .. } => ConstantValue::Overdefined,
                _ => ConstantValue::Scalar(value.clone()),
            },
            mir::Instruction::Select {
                condition,
                then_value,
                else_value,
                ..
            } => match self.table.value(*condition) {
                ConstantValue::Scalar(mir::Constant::Boolean { value: true }) => {
                    self.table.value(*then_value).clone()
                }
                ConstantValue::Scalar(mir::Constant::Boolean { value: false }) => {
                    self.table.value(*else_value).clone()
                }
                ConstantValue::Unknown => ConstantValue::Unknown,
                _ => {
                    let mut value = self.table.value(*then_value).clone();
                    value.merge(self.table.value(*else_value));
                    value
                }
            },
            mir::Instruction::Aggregate { values, .. } => ConstantValue::Aggregate(
                self.tree
                    .get_values(*values)
                    .iter()
                    .map(|value| self.table.value(*value).clone())
                    .collect(),
            ),
            mir::Instruction::FieldGet {
                aggregate,
                field: index,
                ..
            }
            | mir::Instruction::ElementGet {
                aggregate, index, ..
            } => match self.table.value(*aggregate) {
                ConstantValue::Aggregate(values) => values[*index as usize].clone(),
                ConstantValue::Unknown => ConstantValue::Unknown,
                _ => ConstantValue::Overdefined,
            },
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
            } => match self.table.value(*aggregate) {
                ConstantValue::Aggregate(values) => {
                    let mut values = values.clone();
                    values[*index as usize] = self.table.value(*value).clone();
                    ConstantValue::Aggregate(values)
                }
                ConstantValue::Unknown => ConstantValue::Unknown,
                _ => ConstantValue::Overdefined,
            },
            mir::Instruction::VariantNew {
                case,
                payload,
                result_type,
                ..
            } => {
                let ty = mir::Substitution::resolve(*result_type, self.tree);
                let mir::Type::Variant { cases, .. } = self.tree.get(ty) else {
                    unreachable!("variant.new requires a represented variant type");
                };
                ConstantValue::Variant {
                    case: *case,
                    discriminant: cases[*case as usize].discriminant.clone(),
                    payload: payload.map(|value| Box::new(self.table.value(value).clone())),
                }
            }
            mir::Instruction::VariantTag { variant, .. } => match self.table.value(*variant) {
                ConstantValue::Variant { discriminant, .. } => {
                    ConstantValue::Scalar(discriminant.clone())
                }
                ConstantValue::Unknown => ConstantValue::Unknown,
                _ => ConstantValue::Overdefined,
            },
            mir::Instruction::VariantPayload { variant, case, .. } => {
                match self.table.value(*variant) {
                    ConstantValue::Variant {
                        case: selected,
                        payload: Some(payload),
                        ..
                    } if case == selected => *payload.clone(),
                    ConstantValue::Unknown => ConstantValue::Unknown,
                    _ => ConstantValue::Overdefined,
                }
            }
            mir::Instruction::Binary { .. }
            | mir::Instruction::Unary { .. }
            | mir::Instruction::Cast { .. }
            | mir::Instruction::Intrinsic { .. } => self.scalar(instruction),
            _ => ConstantValue::Overdefined,
        };

        self.update(destination, result);
    }

    /// Evaluate scalar operations after all required operands become constants.
    fn scalar(&self, instruction: &mir::Instruction) -> ConstantValue {
        // defer unresolved operands and reject runtime-dependent operations
        let operands = instruction.reads(self.tree);
        if operands
            .iter()
            .any(|value| matches!(self.table.value(*value), ConstantValue::Overdefined))
        {
            return ConstantValue::Overdefined;
        }
        if operands
            .iter()
            .any(|value| matches!(self.table.value(*value), ConstantValue::Unknown))
        {
            return ConstantValue::Unknown;
        }

        // reuse the MIR constant folders for scalar operation semantics
        let folded = match instruction {
            mir::Instruction::Binary {
                operator,
                left,
                right,
                ..
            } => match (self.table.constant(*left), self.table.constant(*right)) {
                (Some(left), Some(right)) => fold_binary(*operator, left.clone(), right.clone()),
                _ => None,
            },
            mir::Instruction::Unary {
                operator, argument, ..
            } => self
                .table
                .constant(*argument)
                .and_then(|value| fold_unary(*operator, value.clone())),
            mir::Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => self.table.constant(*argument).and_then(|value| {
                fold_cast(
                    *operator,
                    value.clone(),
                    *to_type,
                    self.target.pointer_bits(),
                    self.tree,
                )
            }),
            mir::Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => {
                let constants = self
                    .tree
                    .get_values(*arguments)
                    .iter()
                    .map(|value| self.table.constant(*value).cloned())
                    .collect::<Option<Vec<_>>>();
                constants.and_then(|constants| fold_intrinsic(*intrinsic, &constants))
            }
            _ => None,
        };

        folded.map_or(ConstantValue::Overdefined, ConstantValue::Scalar)
    }

    /// Evaluate a runtime check whose required scalar operands are constants.
    fn check(&self, constraint: &mir::CheckConstraint) -> Option<bool> {
        match constraint {
            mir::CheckConstraint::Bounds {
                index,
                length,
                is_signed,
                ..
            } => {
                let (index, width, _) = decode_int_constant(self.table.constant(*index)?)?;
                let (length, _, _) = decode_int_constant(self.table.constant(*length)?)?;
                let is_negative = *is_signed && index & (1 << (width - 1)) != 0;

                Some(!is_negative && index < length)
            }
            mir::CheckConstraint::Null { value } => match self.table.constant(*value)? {
                mir::Constant::Null => Some(false),
                _ => None,
            },
            mir::CheckConstraint::DivZero { divisor } => {
                let (bits, _, _) = decode_int_constant(self.table.constant(*divisor)?)?;

                Some(bits != 0)
            }
            mir::CheckConstraint::ShiftRange {
                value,
                bit_width,
                is_signed,
            } => {
                let (bits, width, _) = decode_int_constant(self.table.constant(*value)?)?;
                let is_negative = *is_signed && bits & (1 << (width - 1)) != 0;

                Some(!is_negative && bits < u128::from(*bit_width))
            }
            mir::CheckConstraint::Narrow {
                value,
                to_width,
                is_signed,
            } => {
                let (bits, width, source_signed) =
                    decode_int_constant(self.table.constant(*value)?)?;
                let to_width = u16::from(*to_width);
                if !(1..=128).contains(&to_width) {
                    return None;
                }
                let is_negative = source_signed && bits & (1 << (width - 1)) != 0;
                if *is_signed {
                    let (minimum, maximum) = integer_bounds(to_width, true)?;
                    if is_negative {
                        Some(signed_from_bits(bits, width) >= minimum)
                    } else {
                        Some(bits <= maximum as u128)
                    }
                } else {
                    let maximum = u128::MAX >> (128 - to_width);

                    Some(!is_negative && bits <= maximum)
                }
            }
            mir::CheckConstraint::Overflow {
                operator,
                left,
                right,
                is_signed,
            } => {
                let (left, width, _) = decode_int_constant(self.table.constant(*left)?)?;
                let (right, _, _) = decode_int_constant(self.table.constant(*right)?)?;
                if *is_signed {
                    let left = signed_from_bits(left, width);
                    let right = signed_from_bits(right, width);
                    let result = match operator {
                        mir::BinaryOperator::Add => left.checked_add(right),
                        mir::BinaryOperator::Subtract => left.checked_sub(right),
                        mir::BinaryOperator::Multiply => left.checked_mul(right),
                        mir::BinaryOperator::Divide => left.checked_div(right),
                        _ => return None,
                    };
                    let (minimum, maximum) = integer_bounds(width, true)?;

                    Some(result.is_some_and(|value| value >= minimum && value <= maximum))
                } else {
                    let result = match operator {
                        mir::BinaryOperator::Add => left.checked_add(right),
                        mir::BinaryOperator::Subtract => left.checked_sub(right),
                        mir::BinaryOperator::Multiply => left.checked_mul(right),
                        mir::BinaryOperator::Divide => left.checked_div(right),
                        _ => return None,
                    };
                    let maximum = u128::MAX >> (128 - width);

                    Some(result.is_some_and(|value| value <= maximum))
                }
            }
            mir::CheckConstraint::IsType { .. } | mir::CheckConstraint::IsSubtype { .. } => None,
        }
    }

    /// Propagate values through each executable successor of one terminator.
    fn terminator(&mut self, block: mir::BlockId) {
        let terminator = &self.tree.get(self.tree.get(block).terminator).clone();
        let targets = terminator
            .targets(self.tree, block)
            .into_iter()
            .map(|(edge, target)| (edge, target.clone()))
            .collect::<Vec<_>>();
        let selected = match terminator {
            mir::Terminator::Branch { condition, .. } => match self.table.value(*condition) {
                ConstantValue::Unknown => return,
                ConstantValue::Scalar(mir::Constant::Boolean { value }) => Some(if *value {
                    mir::Successor::BranchThen
                } else {
                    mir::Successor::BranchElse
                }),
                _ => None,
            },
            mir::Terminator::Check { constraint, .. } => {
                if let Some(is_valid) = self.check(constraint) {
                    Some(if is_valid {
                        mir::Successor::CheckSuccess
                    } else {
                        mir::Successor::CheckFailure
                    })
                } else if constraint
                    .uses()
                    .iter()
                    .any(|value| matches!(self.table.value(*value), ConstantValue::Unknown))
                {
                    return;
                } else {
                    None
                }
            }
            mir::Terminator::Switch { value, cases, .. } => {
                let value = match self.table.value(*value) {
                    ConstantValue::Unknown => return,
                    ConstantValue::Scalar(mir::Constant::Int { value, .. }) => Some(*value),
                    ConstantValue::Scalar(mir::Constant::UInt { value, .. }) => {
                        Some(*value as i128)
                    }
                    _ => None,
                };
                value.map(|value| {
                    if self
                        .tree
                        .get_switch_cases(*cases)
                        .iter()
                        .any(|case| case.value == value)
                    {
                        mir::Successor::SwitchCase { value }
                    } else {
                        mir::Successor::SwitchDefault
                    }
                })
            }
            mir::Terminator::VariantSwitch { value, .. } => match self.table.value(*value) {
                ConstantValue::Unknown => return,
                ConstantValue::Variant { case, .. } => {
                    let case = mir::Successor::SwitchCase {
                        value: i128::from(*case),
                    };
                    Some(if targets.iter().any(|(edge, _)| edge.successor == case) {
                        case
                    } else {
                        mir::Successor::SwitchDefault
                    })
                }
                _ => None,
            },
            _ => None,
        };

        // bind explicit edge arguments after any values produced by the terminator itself
        for (edge, target) in targets {
            if selected.is_some_and(|selected| selected != edge.successor) {
                continue;
            }
            self.table.edges.insert(edge);
            let result_count = terminator.target_result_count(self.tree, edge.successor);
            let parameters = &self.tree.get(target.block).parameters.clone();
            let arguments = &target.arguments(self.tree).to_vec();
            assert_eq!(
                parameters.len(),
                result_count + arguments.len(),
                "MIR edge parameter count mismatch"
            );
            for parameter in &parameters[..result_count] {
                self.update(parameter.value, ConstantValue::Overdefined);
            }
            for (parameter, argument) in parameters[result_count..].iter().zip(arguments) {
                self.update(parameter.value, self.table.value(*argument).clone());
            }
            self.activate(target.block);
        }
    }
}

/// The scalar or symbolic type of a constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantType {
    /// The open type of a value parameter.
    Parameter,
    /// Null pointer constant type.
    Null,
    /// Undefined constant type at a polymorphic representation.
    Undefined,
    /// Boolean constant type.
    Boolean,
    /// Integer constant type.
    Int {
        /// The integer width in bits.
        width: u16,
        /// Whether the integer is signed.
        signed: bool,
    },
    /// Floating point constant type.
    Float {
        /// The floating point format.
        format: mir::FloatType,
    },
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
        mir::Constant::Undefined => ConstantType::Undefined,
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

    // compare the constant type with the destination type
    match (constant_type, tree.get(destination_type)) {
        (ConstantType::Parameter, _) => true,
        (ConstantType::Null, mir::Type::Pointer { .. }) => true,
        (ConstantType::Null | ConstantType::Undefined, mir::Type::Parameter { .. }) => true,
        (ConstantType::Boolean, mir::Type::Boolean) => true,
        (ConstantType::Int { width, signed }, ty) => {
            let Some((ty_width, ty_signed)) = ty.integer(pointer_width_bits) else {
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
    let Some((width, signed)) = ty.integer(pointer_width_bits) else {
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

/// Fold an intrinsic when constant operands satisfy its preconditions.
pub fn fold_intrinsic(
    intrinsic: mir::Intrinsic,
    arguments: &[mir::Constant],
) -> Option<mir::Constant> {
    // reject empty argument lists
    let first = arguments.first()?;

    // fold scalar integer arithmetic and preserve prediction hints
    if intrinsic == mir::Intrinsic::Expect {
        return Some(first.clone());
    }
    if let Some(result) = fold_integer_intrinsic(intrinsic, arguments) {
        return Some(result);
    }

    // fold integer unary intrinsics
    let folded_integer_unary = match intrinsic {
        mir::Intrinsic::IsolateLowestOne => {
            fold_int_unary(first, |value, _| Some(value & value.wrapping_neg()))
        }
        mir::Intrinsic::LeadingZeroCount => fold_int_unary(first, |value, width| {
            let leading = value.leading_zeros();
            let adjust = u32::from(128u16.saturating_sub(width));
            Some((leading - adjust) as u128)
        }),
        mir::Intrinsic::TrailingZeroCount => fold_int_unary(first, |value, width| {
            Some(u128::from(value.trailing_zeros().min(u32::from(width))))
        }),
        mir::Intrinsic::PopulationCount => {
            fold_int_unary(first, |value, _width| Some(value.count_ones() as u128))
        }
        mir::Intrinsic::ByteSwap => fold_int_unary(first, |value, width| {
            if width % 8 != 0 {
                return None;
            }
            let swapped = value.swap_bytes() >> (128 - width);
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
            let complementary = (u32::from(width) - shift) % u32::from(width);
            let rotated = (value << shift) | (value >> complementary);
            Some(mask_to_width(rotated, width))
        }),
        mir::Intrinsic::RotateRight => fold_int_binary(arguments, |value, shift, width| {
            let shift = (shift % u128::from(width)) as u32;
            let complementary = (u32::from(width) - shift) % u32::from(width);
            let rotated = (value >> shift) | (value << complementary);
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
        mir::Intrinsic::Fma => fold_fma(arguments),
        _ => None,
    }
}

/// Fold integer arithmetic and report whether its declared range overflowed.
fn fold_overflow(
    intrinsic: mir::Intrinsic,
    arguments: &[mir::Constant],
) -> Option<(mir::Constant, bool)> {
    let [left, right] = arguments else {
        return None;
    };
    let (left, width, is_signed) = decode_int_constant(left)?;
    let (right, right_width, right_signed) = decode_int_constant(right)?;
    if width != right_width || is_signed != right_signed {
        return None;
    }

    // compute the wrapped result independently of overflow detection
    let (wrapped, unsigned) = match intrinsic {
        mir::Intrinsic::AddOverflow | mir::Intrinsic::AddUnchecked | mir::Intrinsic::SatAdd => {
            (left.wrapping_add(right), left.checked_add(right))
        }
        mir::Intrinsic::SubOverflow | mir::Intrinsic::SubUnchecked | mir::Intrinsic::SatSub => {
            (left.wrapping_sub(right), left.checked_sub(right))
        }
        mir::Intrinsic::MulOverflow | mir::Intrinsic::MulUnchecked => {
            (left.wrapping_mul(right), left.checked_mul(right))
        }
        _ => return None,
    };
    let mask = u128::MAX >> (128 - width);
    let overflow = if is_signed {
        let left = signed_from_bits(left, width);
        let right = signed_from_bits(right, width);
        let result = match intrinsic {
            mir::Intrinsic::AddOverflow | mir::Intrinsic::AddUnchecked | mir::Intrinsic::SatAdd => {
                left.checked_add(right)
            }
            mir::Intrinsic::SubOverflow | mir::Intrinsic::SubUnchecked | mir::Intrinsic::SatSub => {
                left.checked_sub(right)
            }
            _ => left.checked_mul(right),
        };
        let maximum = (mask >> 1) as i128;

        result.is_none_or(|value| value < -maximum - 1 || value > maximum)
    } else {
        unsigned.is_none_or(|value| value > mask)
    };

    Some((encode_int_constant(wrapped, width, is_signed), overflow))
}

/// Fold scalar integer intrinsics in the operand's declared width.
fn fold_integer_intrinsic(
    intrinsic: mir::Intrinsic,
    arguments: &[mir::Constant],
) -> Option<mir::Constant> {
    let first = arguments.first()?;
    let second = arguments.get(1)?;
    let (left, width, is_signed) = decode_int_constant(first)?;
    let (right, right_width, right_signed) = decode_int_constant(second)?;
    if width != right_width || is_signed != right_signed {
        return None;
    }
    let mask = u128::MAX >> (128 - width);

    // enforce unchecked arithmetic preconditions and select saturation endpoints
    match intrinsic {
        mir::Intrinsic::AddUnchecked
        | mir::Intrinsic::SubUnchecked
        | mir::Intrinsic::MulUnchecked => {
            let (value, overflow) = fold_overflow(intrinsic, arguments)?;

            return (!overflow).then_some(value);
        }
        mir::Intrinsic::SatAdd | mir::Intrinsic::SatSub => {
            let (value, overflow) = fold_overflow(intrinsic, arguments)?;
            if !overflow {
                return Some(value);
            }
            let endpoint = if is_signed {
                if left & (1 << (width - 1)) == 0 {
                    mask >> 1
                } else {
                    1 << (width - 1)
                }
            } else if intrinsic == mir::Intrinsic::SatAdd {
                mask
            } else {
                0
            };

            return Some(encode_int_constant(endpoint, width, is_signed));
        }
        mir::Intrinsic::ShlUnchecked | mir::Intrinsic::ShrUnchecked => {
            let operator = if intrinsic == mir::Intrinsic::ShlUnchecked {
                mir::BinaryOperator::ShiftLeft
            } else {
                mir::BinaryOperator::ShiftRight
            };

            return fold_binary(operator, first.clone(), second.clone());
        }
        _ => {}
    }

    // apply signed arithmetic with explicit division and range preconditions
    let value = if is_signed {
        let left = signed_from_bits(left, width);
        let right = signed_from_bits(right, width);
        let minimum = -((mask >> 1) as i128) - 1;
        match intrinsic {
            mir::Intrinsic::Midpoint => {
                let floor = (left & right) + ((left ^ right) >> 1);

                (floor + i128::from(floor < 0 && (left ^ right) & 1 != 0)) as u128
            }
            mir::Intrinsic::Clamp => {
                let (upper, upper_width, upper_signed) = decode_int_constant(arguments.get(2)?)?;
                if upper_width != width || !upper_signed {
                    return None;
                }
                let upper = signed_from_bits(upper, width);
                if right > upper {
                    return None;
                }

                left.clamp(right, upper) as u128
            }
            mir::Intrinsic::DivideCeil
            | mir::Intrinsic::DivUnchecked
            | mir::Intrinsic::RemUnchecked
            | mir::Intrinsic::RemainderEuclidean => {
                if right == 0 || (left == minimum && right == -1) {
                    return None;
                }
                match intrinsic {
                    mir::Intrinsic::DivideCeil => {
                        (left / right + i128::from(left % right != 0 && (left < 0) == (right < 0)))
                            as u128
                    }
                    mir::Intrinsic::DivUnchecked => (left / right) as u128,
                    mir::Intrinsic::RemUnchecked => (left % right) as u128,
                    _ => left.rem_euclid(right) as u128,
                }
            }
            mir::Intrinsic::IsMultipleOf => {
                let value = if right == 0 {
                    left == 0
                } else if right == -1 {
                    true
                } else {
                    left % right == 0
                };

                return Some(mir::Constant::Boolean { value });
            }
            mir::Intrinsic::AbsDiff => {
                return Some(encode_int_constant(left.abs_diff(right), width, false));
            }
            _ => return None,
        }
    }
    // apply unsigned arithmetic in the full u128 range
    else {
        match intrinsic {
            mir::Intrinsic::Midpoint => (left & right) + ((left ^ right) >> 1),
            mir::Intrinsic::Clamp => {
                let (upper, upper_width, upper_signed) = decode_int_constant(arguments.get(2)?)?;
                if upper_width != width || upper_signed || right > upper {
                    return None;
                }

                left.clamp(right, upper)
            }
            mir::Intrinsic::DivideCeil if right != 0 => left.div_ceil(right),
            mir::Intrinsic::DivUnchecked if right != 0 => left / right,
            mir::Intrinsic::RemUnchecked | mir::Intrinsic::RemainderEuclidean if right != 0 => {
                left % right
            }
            mir::Intrinsic::IsMultipleOf => {
                let value = if right == 0 {
                    left == 0
                } else {
                    left.is_multiple_of(right)
                };

                return Some(mir::Constant::Boolean { value });
            }
            mir::Intrinsic::AbsDiff => left.abs_diff(right),
            _ => return None,
        }
    };

    Some(encode_int_constant(value, width, is_signed))
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

/// Fold a fused multiply-add in the declared precision.
fn fold_fma(arguments: &[mir::Constant]) -> Option<mir::Constant> {
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

    // round the fused operation once in the declared precision
    let folded = match width {
        mir::FloatType::Float32 => {
            (first_value as f32).mul_add(second_value as f32, third_value as f32) as f64
        }
        mir::FloatType::Float64 => first_value.mul_add(second_value, third_value),
    };

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
        } if (1..=128).contains(width) => {
            let masked = mask_to_width(*value as u128, *width);
            Some((masked, *width, *is_signed))
        }
        mir::Constant::UInt { value, width } if (1..=128).contains(width) => {
            Some((mask_to_width(*value, *width), *width, false))
        }
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

    // mask the value to its declared integer width
    let mask = (1u128 << width) - 1;
    let masked = value & mask;
    let sign_bit = 1u128 << (width - 1);

    // extend the sign bit through the host integer
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

    // resolve the underlying newtype
    if let mir::Type::Newtype { value, .. } = ty {
        return constant_tree_from_scalar(constant, *value, tree);
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
        mir::Type::Newtype { value, .. } => {
            constant_tree_from_zero(*value, tree, max_aggregate_elements, pointer_width_bits)
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

            // bound aggregate expansion by its element count
            if length > max_aggregate_elements {
                return ConstantTree::Unknown;
            }

            // construct the repeated element value
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

    // require matching lengths within the aggregate limit
    if length != bytes.len() || length > max_aggregate_elements {
        return ConstantTree::Unknown;
    }

    // require 8 bit integer element type
    let element = *element;

    // require an integer element type
    let mir::Type::Int {
        width,
        is_signed: signed,
    } = tree.get(element)
    else {
        return ConstantTree::Unknown;
    };

    // require one byte per element
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

            // require matching lengths within the aggregate limit
            if length != elements.len() || length > max_aggregate_elements {
                return ConstantTree::Unknown;
            }

            // collect the aggregate elements
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

            // collect the aggregate fields
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

            // collect the aggregate elements
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
    // restrict evaluation to representable integers and valid shift counts
    if !(1..=128).contains(&width) {
        return None;
    }
    if matches!(
        operator,
        mir::BinaryOperator::ShiftLeft
            | mir::BinaryOperator::ShiftRight
            | mir::BinaryOperator::UnsignedShiftRight
    ) && u128::try_from(right)
        .ok()
        .is_none_or(|right| right >= u128::from(width))
    {
        return None;
    }

    // construct signed results in the operand width
    let result_int = |value: i128| {
        Some(mir::Constant::Int {
            value: signed_from_bits(value as u128, width),
            width,
            is_signed: true,
        })
    };
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    // fold the signed integer operation
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
        mir::BinaryOperator::UnsignedShiftRight => {
            result_int((mask_to_width(left as u128, width) >> right as u32) as i128)
        }
        mir::BinaryOperator::Equal => result_bool(left == right),
        mir::BinaryOperator::NotEqual => result_bool(left != right),
        mir::BinaryOperator::LessThan => result_bool(left < right),
        mir::BinaryOperator::LessEqual => result_bool(left <= right),
        mir::BinaryOperator::GreaterThan => result_bool(left > right),
        mir::BinaryOperator::GreaterEqual => result_bool(left >= right),
    }
}

/// Fold a binary operation on unsigned integers.
pub fn fold_binary_unsigned(
    left: u128,
    right: u128,
    width: u16,
    operator: mir::BinaryOperator,
) -> Option<mir::Constant> {
    // restrict evaluation to representable integers and valid shift counts
    if !(1..=128).contains(&width) {
        return None;
    }
    if matches!(
        operator,
        mir::BinaryOperator::ShiftLeft
            | mir::BinaryOperator::ShiftRight
            | mir::BinaryOperator::UnsignedShiftRight
    ) && right >= u128::from(width)
    {
        return None;
    }

    // construct unsigned results in the operand width
    let result_uint = |value: u128| {
        Some(mir::Constant::UInt {
            value: mask_to_width(value, width),
            width,
        })
    };
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    // fold the unsigned integer operation
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

    // round the result to its declared float format
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

    // decode the float operands
    let left = float_from_bits(format.format(), left_bits);
    let right = float_from_bits(format.format(), right_bits);

    // fold the float operation
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

    // fold the boolean operation
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
    to_type: mir::TypeId,
    pointer_width_bits: u16,
    tree: &mir::Tree,
) -> Option<mir::Constant> {
    let target = tree.type_definition(to_type);

    match operator {
        // preserve the source bits at an equally sized scalar type
        mir::CastOperator::Bitcast => {
            let (bits, width) = match value {
                mir::Constant::Float { bits, format } => (u128::from(bits), format.width()),
                value => {
                    let (bits, width, _) = decode_int_constant(&value)?;

                    (bits, width)
                }
            };
            match target {
                mir::Type::Float(format) if format.width() == width => Some(mir::Constant::Float {
                    bits: bits as u64,
                    format: *format,
                }),
                _ => {
                    let (target_width, is_signed) = target.integer(pointer_width_bits)?;

                    (target_width == width).then(|| encode_int_constant(bits, width, is_signed))
                }
            }
        }
        // extend by source signedness, then retain the destination bits
        mir::CastOperator::IntToInt | mir::CastOperator::IntToIntSaturating => {
            let (bits, width, is_signed) = decode_int_constant(&value)?;
            let (target_width, target_signed) = target.integer(pointer_width_bits)?;
            let is_negative = is_signed && signed_from_bits(bits, width) < 0;
            let converted = if operator == mir::CastOperator::IntToInt {
                if is_signed {
                    signed_from_bits(bits, width) as u128
                } else {
                    bits
                }
            } else if target_signed {
                let (minimum, maximum) = integer_bounds(target_width, true)?;
                if is_negative {
                    signed_from_bits(bits, width).max(minimum) as u128
                } else {
                    bits.min(maximum as u128)
                }
            } else if is_negative {
                0
            } else {
                bits.min(mask_to_width(u128::MAX, target_width))
            };

            Some(encode_int_constant(
                mask_to_width(converted, target_width),
                target_width,
                target_signed,
            ))
        }
        // check the truncated float against exact powers of two before host conversion
        mir::CastOperator::FloatToInt | mir::CastOperator::FloatToIntSaturating => {
            let mir::Constant::Float { bits, format } = value else {
                return None;
            };
            let value = float_from_bits(format.format(), bits).trunc();
            let (width, is_signed) = target.integer(pointer_width_bits)?;
            let upper = 2.0_f64.powi(i32::from(width) - i32::from(is_signed));
            let lower = if is_signed { -upper } else { 0.0 };
            if operator == mir::CastOperator::FloatToInt
                && (!value.is_finite() || value < lower || value >= upper)
            {
                return None;
            }

            // host casts saturate and map NaN to zero before narrowing the range
            let bits = if is_signed {
                let (minimum, maximum) = integer_bounds(width, true)?;

                (value as i128).clamp(minimum, maximum) as u128
            } else {
                (value as u128).min(mask_to_width(u128::MAX, width))
            };

            Some(encode_int_constant(bits, width, is_signed))
        }
        // round integers directly into the requested float format
        mir::CastOperator::IntToFloat => {
            let (bits, width, is_signed) = decode_int_constant(&value)?;
            let mir::Type::Float(format) = target else {
                return None;
            };
            let bits = match (format, is_signed) {
                (mir::FloatType::Float32, true) => {
                    (signed_from_bits(bits, width) as f32).to_bits() as u64
                }
                (mir::FloatType::Float32, false) => (bits as f32).to_bits() as u64,
                (mir::FloatType::Float64, true) => (signed_from_bits(bits, width) as f64).to_bits(),
                (mir::FloatType::Float64, false) => (bits as f64).to_bits(),
            };

            Some(mir::Constant::Float {
                bits,
                format: *format,
            })
        }
        // convert between float formats and preserve equal-format bits
        mir::CastOperator::FloatToFloat => {
            let mir::Constant::Float { bits, format } = value else {
                return None;
            };
            let mir::Type::Float(target) = target else {
                return None;
            };
            let bits = if format == *target {
                bits
            } else {
                float_to_bits(target.format(), float_from_bits(format.format(), bits))
            };

            Some(mir::Constant::Float {
                bits,
                format: *target,
            })
        }
        mir::CastOperator::PointerToInt
        | mir::CastOperator::IntToPointer
        | mir::CastOperator::ReferenceToPointer
        | mir::CastOperator::PointerToReference => None,
    }
}

/// Compute integer bounds for a width and signedness.
fn integer_bounds(width: u16, is_signed: bool) -> Option<(i128, i128)> {
    if width == 0 || width > 128 || (!is_signed && width == 128) {
        return None;
    }

    // interpret the result with its declared signedness
    if is_signed {
        let max = i128::MAX >> (128 - width);
        let min = !max;
        Some((min, max))
    } else {
        let max = (u128::MAX >> (128 - width)) as i128;
        Some((0, max))
    }
}

/// Try to fold a unary operation on a constant.
pub fn fold_unary(operator: mir::UnaryOperator, value: mir::Constant) -> Option<mir::Constant> {
    // fold integers in their declared width
    if let Some((bits, width, is_signed)) = decode_int_constant(&value) {
        let bits = match operator {
            mir::UnaryOperator::Negate => bits.wrapping_neg(),
            mir::UnaryOperator::Not => !bits,
        };

        return Some(encode_int_constant(
            mask_to_width(bits, width),
            width,
            is_signed,
        ));
    }

    // preserve floating negation and boolean inversion
    match (operator, value) {
        (mir::UnaryOperator::Negate, mir::Constant::Float { bits, format }) => {
            let value = -float_from_bits(format.format(), bits);

            Some(mir::Constant::Float {
                bits: float_to_bits(format.format(), value),
                format,
            })
        }
        (mir::UnaryOperator::Not, mir::Constant::Boolean { value }) => {
            Some(mir::Constant::Boolean { value: !value })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Fold numeric casts at signed, unsigned, floating, and bit-pattern boundaries.
    #[test]
    fn test_fold_cast_boundaries() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = -1
    v1: uint16 = cast.intToInt v0 -> uint16
    v2: uint128 = 340282366920938463463374607431768211455
    v3: int8 = cast.intToIntSaturating v2 -> int8
    v4: uint128 = cast.intToIntSaturating v0 -> uint128
    v5: float64 = 256.0
    v6: uint8 = cast.floatToIntSaturating v5 -> uint8
    v7: uint8 = cast.floatToInt v5 -> uint8
    v8: float64 = -0.5
    v9: uint8 = cast.floatToInt v8 -> uint8
    v10: uint32 = 1065353216
    v11: float32 = cast.bit v10 -> float32
    v12: float64 = cast.intToFloat v0 -> float64
    v13: float64 = cast.floatToFloat v11 -> float64
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let actual = [1, 3, 4, 6, 7, 9, 11, 12, 13]
            .map(|index| constants.constant(mir::Value(index)).cloned());

        assert_eq!(
            actual,
            [
                Some(mir::Constant::uint16(65535)),
                Some(mir::Constant::int8(127)),
                Some(mir::Constant::UInt {
                    value: 0,
                    width: 128
                }),
                Some(mir::Constant::uint8(255)),
                None,
                Some(mir::Constant::uint8(0)),
                Some(mir::Constant::float32(1.0)),
                Some(mir::Constant::float64(-1.0)),
                Some(mir::Constant::float64(1.0)),
            ]
        );
    }

    /// Mutable globals are not treated as constants.
    #[test]
    fn test_keep_mutable_global_loads_unknown() {
        let test = TestModule::new(
            r#"
global flag: boolean = true

function test(): boolean {
entry:
    v0: ref<boolean, borrowed, 'static, mutable> = address @flag
    v1: boolean = load (*v0)
    return v1
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function_id, &test.tree);

        let constant = analysis.constant(mir::Value::new(1)).cloned();
        assert_eq!(constant, None);
    }

    /// Constant results of binary operations are propagated.
    #[test]
    fn test_fold_constant_addition() {
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
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function_id, &test.tree);

        let constant = analysis.constant(mir::Value::new(2)).cloned();
        assert_eq!(
            constant,
            Some(mir::Constant::Int {
                value: 5,
                width: 32,
                is_signed: true,
            })
        );
    }

    /// Preserve agreeing parameters while widening conflicting parameters at the same join.
    #[test]
    fn test_join_agreeing_and_conflicting_arguments() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): boolean {
entry(v0: boolean):
    v1: boolean = true
    v2: boolean = false
    branch v0 => left | right

left:
    jump join(v1, v1)

right:
    jump join(v1, v2)

join(v3: boolean, v4: boolean):
    return v4
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);

        assert_eq!(
            constants.value(mir::Value(3)),
            &ConstantValue::Scalar(mir::Constant::Boolean { value: true })
        );
        assert_eq!(constants.value(mir::Value(4)), &ConstantValue::Overdefined);
    }

    /// Fallible allocation result parameters do not consume edge arguments.
    #[test]
    fn test_preserve_arguments_after_allocation_results() {
        let test = TestModule::new(
            r#"
function test(v0: int64): boolean {
entry(v0: int64):
    v1: boolean = true
    new.slice.uninit.try int32, v0, local => b1(v1) | b2

b1(v2: uninit<slice<int32, managed, mutable, local>>, v3: boolean):
    return v3

b2:
    v4: boolean = false
    return v4
}
"#,
        );

        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function_id, &test.tree);

        let success = test.tree.get(function_id).block(1);
        let success_block = test.tree.get(success);
        let argument = success_block.parameters[1].value;
        let constant = analysis.constant(argument).cloned();

        assert_eq!(constant, Some(mir::Constant::Boolean { value: true }));
    }

    /// Conflicting arguments to a single target are not treated as constants.
    #[test]
    fn test_distinguish_arguments_on_repeated_edges() {
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
        let mut analyses = test.function_analyses();
        let analysis = analyses.constant(function_id, &test.tree);

        let constant = analysis.constant(mir::Value::new(3)).cloned();
        assert_eq!(constant, None);
    }

    /// Follow only the selected edge when two branch arms pass different values to one block.
    #[test]
    fn test_select_branch_arguments() {
        let program = TestModule::new(
            r#"
function test(): int32 {
entry:
    v0: boolean = true
    v1: int32 = 7
    v2: int32 = 9
    branch v0 => join(v1) | join(v2)

join(v3: int32):
    return v3
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let edge = mir::Edge::new(
            program.tree.get(program.entry_function_id()).block(0),
            mir::Successor::BranchThen,
            program.tree.get(program.entry_function_id()).block(1),
        );

        assert_eq!(
            constants.constant(mir::Value(3)),
            Some(&mir::Constant::int32(7))
        );
        assert_eq!(constants.edges.iter().copied().collect::<Vec<_>>(), [edge]);
    }

    /// Widen a loop recurrence after its executable backedge supplies a different value.
    #[test]
    fn test_widen_constants_changed_by_loop_backedges() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 0
    v2: int32 = 1
    jump loop(v1)

loop(v3: int32):
    v4: int32 = add v3, v2
    branch v0 => loop(v4) | exit(v3)

exit(v5: int32):
    return v5
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let actual = (0..6)
            .map(|value| constants.value(mir::Value(value)).clone())
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            [
                ConstantValue::Overdefined,
                ConstantValue::Scalar(mir::Constant::int32(0)),
                ConstantValue::Scalar(mir::Constant::int32(1)),
                ConstantValue::Overdefined,
                ConstantValue::Overdefined,
                ConstantValue::Overdefined,
            ]
        );
    }

    /// Preserve constant fields when another field remains dynamic.
    #[test]
    fn test_preserve_constant_fields_beside_dynamic_fields() {
        let program = TestModule::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 7
    v2: (int32, int32) = aggregate (v0, v1)
    v3: int32 = field.get v2, 1
    v4: int32 = field.get v2, 0
    return v3
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);

        assert_eq!(
            constants.constant(mir::Value(3)),
            Some(&mir::Constant::int32(7))
        );
        assert_eq!(constants.value(mir::Value(4)), &ConstantValue::Overdefined);
    }

    /// Fold integer bit operations in the operand width.
    #[test]
    fn test_fold_bit_operations_in_the_declared_width() {
        let program = TestModule::new(
            r#"
function test(): uint32 {
entry:
    v0: uint32 = 0
    v1: uint32 = 1
    v2: uint32 = 2147483648
    v3: uint32 = intrinsic.math.bits.trailingZeroCount(v0)
    v4: uint32 = intrinsic.math.bits.byteSwap(v1)
    v5: uint32 = intrinsic.math.bits.rotateLeft(v2, v1)
    v6: uint32 = intrinsic.math.bits.rotateRight(v1, v1)
    v7: uint32 = not v0
    v8: uint32 = intrinsic.math.bits.isolateLowestOne(v2)
    return v7
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let actual = (3..9)
            .map(|value| constants.constant(mir::Value(value)).cloned())
            .collect::<Vec<_>>();
        let expected = [32, 16777216, 1, 2147483648, 4294967295, 2147483648]
            .map(|value| Some(mir::Constant::UInt { value, width: 32 }));

        assert_eq!(actual, expected);
    }

    /// Select a failed division check before evaluating the guarded arithmetic.
    #[test]
    fn test_select_check_failure() {
        let program = TestModule::new(
            r#"
function test(): int32 {
entry:
    v0: int32 = 0
    check div.zero v0 => divide | failure

divide:
    v1: int32 = 7
    v2: int32 = div v1, v0
    return v2

failure:
    v3: int32 = -1
    return v3
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let executable = program
            .tree
            .get(program.entry_function_id())
            .blocks()
            .iter()
            .map(|block| constants.is_executable(*block))
            .collect::<Vec<_>>();

        assert_eq!(executable, [true, false, true]);
        assert_eq!(constants.value(mir::Value(2)), &ConstantValue::Unknown);
    }

    /// Round a fused multiply-add once at binary32 precision.
    #[test]
    fn test_round_float32_fma() {
        let program = TestModule::new(
            r#"
function test(): float32 {
entry:
    v0: float32 = 1.000244140625
    v1: float32 = 8.271806125530277e-25
    v2: float32 = intrinsic.math.float.fma(v0, v0, v1)
    return v2
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);

        assert_eq!(
            constants.constant(mir::Value(2)),
            Some(&mir::Constant::Float {
                bits: 0x3f801001,
                format: mir::FloatType::Float32
            }),
        );
    }

    /// Preserve declared rounding and integer limits during scalar conversion.
    #[test]
    fn test_convert_float_limits() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int64 = 4611686293305294849
    v1: uint64 = 9223372586610589697
    v2: float32 = cast.intToFloat v0 -> float32
    v3: float32 = cast.intToFloat v1 -> float32
    v4: float64 = 1.7014118346046923e38
    v5: int128 = cast.floatToInt v4 -> int128
    v6: int128 = cast.floatToIntSaturating v4 -> int128
    v7: float64 = 127.9
    v8: int8 = cast.floatToInt v7 -> int8
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let actual = [2, 3, 5, 6, 8].map(|value| constants.constant(mir::Value(value)).cloned());

        assert_eq!(
            actual,
            [
                Some(mir::Constant::Float {
                    bits: 0x5e800001,
                    format: mir::FloatType::Float32
                }),
                Some(mir::Constant::Float {
                    bits: 0x5f000001,
                    format: mir::FloatType::Float32
                }),
                None,
                Some(mir::Constant::Int {
                    value: i128::MAX,
                    width: 128,
                    is_signed: true
                }),
                Some(mir::Constant::Int {
                    value: 127,
                    width: 8,
                    is_signed: true
                }),
            ]
        );
    }

    /// Clamp overflowing signed addition and subtraction to their respective limits.
    #[test]
    fn test_saturate_signed_addition_and_subtraction() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = 127
    v1: int8 = 1
    v2: int8 = -128
    v3: int8 = intrinsic.math.arithmetic.saturating.add(v0, v1)
    v4: int8 = intrinsic.math.arithmetic.saturating.subtract(v2, v1)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        for (value, expected) in [
            (3, mir::Constant::int8(127)),
            (4, mir::Constant::int8(-128)),
        ] {
            assert_eq!(
                constants.constant(mir::Value(value)),
                Some(&expected),
                "v{value}"
            );
        }
    }

    /// Produce both the wrapped integer and overflow flag from overflowing addition.
    #[test]
    fn test_fold_wrapped_value_and_overflow_flag() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = 127
    v1: int8 = 1
    v2: (int8, boolean) = intrinsic.math.arithmetic.overflowing.add(v0, v1)
    v3: int8 = field.get v2, 0
    v4: boolean = field.get v2, 1
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        for (value, expected) in [
            (3, mir::Constant::int8(-128)),
            (4, mir::Constant::Boolean { value: true }),
        ] {
            assert_eq!(
                constants.constant(mir::Value(value)),
                Some(&expected),
                "v{value}"
            );
        }

        assert_eq!(
            constants.value(mir::Value(2)),
            &ConstantValue::Aggregate(vec![
                ConstantValue::Scalar(mir::Constant::int8(-128)),
                ConstantValue::Scalar(mir::Constant::Boolean { value: true }),
            ])
        );
    }

    /// Round the midpoint of signed minimum and maximum toward zero.
    #[test]
    fn test_round_signed_midpoint_toward_zero() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = 127
    v1: int8 = -128
    v2: int8 = intrinsic.math.arithmetic.midpoint(v0, v1)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let expected = mir::Constant::int8(0);
        assert_eq!(constants.constant(mir::Value(2)), Some(&expected));
    }

    /// Round a negative quotient upward and return a nonnegative Euclidean remainder.
    #[test]
    fn test_fold_signed_division_rounding() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = -128
    v1: int8 = 3
    v2: int8 = intrinsic.math.arithmetic.divideCeil(v0, v1)
    v3: int8 = intrinsic.math.arithmetic.remainderEuclidean(v0, v1)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        for (value, expected) in [(2, mir::Constant::int8(-42)), (3, mir::Constant::int8(1))] {
            assert_eq!(
                constants.constant(mir::Value(value)),
                Some(&expected),
                "v{value}"
            );
        }
    }

    /// Represent the full distance between signed limits in the unsigned result type.
    #[test]
    fn test_fold_unsigned_distance_between_signed_limits() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = 127
    v1: int8 = -128
    v2: uint8 = intrinsic.math.arithmetic.absDiff(v1, v0)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let expected = mir::Constant::UInt {
            value: 255,
            width: 8,
        };
        assert_eq!(constants.constant(mir::Value(2)), Some(&expected));
    }

    /// Recognize that the signed minimum is divisible by minus one.
    #[test]
    fn test_fold_divisibility_by_minus_one() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = -128
    v1: int8 = -1
    v2: boolean = intrinsic.math.arithmetic.isMultipleOf(v0, v1)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let expected = mir::Constant::Boolean { value: true };
        assert_eq!(constants.constant(mir::Value(2)), Some(&expected));
    }

    /// Clamp values below, within, and above the declared integer interval.
    #[test]
    fn test_fold_clamp_endpoints() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = -128
    v1: int8 = 1
    v2: int8 = 3
    v3: int8 = 127
    v4: int8 = intrinsic.math.arithmetic.clamp(v0, v1, v2)
    v5: int8 = intrinsic.math.arithmetic.clamp(v1, v1, v2)
    v6: int8 = intrinsic.math.arithmetic.clamp(v3, v1, v2)
    v7: int8 = 2
    v8: int8 = intrinsic.math.arithmetic.clamp(v7, v1, v2)
    v9: int8 = intrinsic.math.arithmetic.clamp(v2, v1, v2)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        for (value, expected) in [(4, 1), (5, 1), (6, 3), (8, 2), (9, 3)] {
            assert_eq!(
                constants.constant(mir::Value(value)),
                Some(&mir::Constant::int8(expected)),
                "v{value}"
            );
        }
    }

    /// Fold unchecked addition within the signed range, including its maximum.
    #[test]
    fn test_fold_unchecked_addition_within_integer_limits() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int8 = 127
    v1: int8 = 1
    v2: int8 = 3
    v3: int8 = intrinsic.math.arithmetic.unchecked.add(v1, v2)
    v4: int8 = 0
    v5: int8 = intrinsic.math.arithmetic.unchecked.add(v0, v4)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);

        assert_eq!(
            constants.constant(mir::Value(3)),
            Some(&mir::Constant::int8(4))
        );
        assert_eq!(
            constants.constant(mir::Value(5)),
            Some(&mir::Constant::int8(127))
        );
    }

    /// Select a switch case without merging default arguments sent to the same target.
    #[test]
    fn test_select_switch_arguments_on_repeated_targets() {
        let program = TestModule::new(
            r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 7
    v2: int32 = 9
    switch v0, join(v2), 1 => join(v1), 2 => unused(v2)

join(v3: int32):
    return v3

unused(v4: int32):
    return v4
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let edge = mir::Edge::new(
            program.tree.get(function).block(0),
            mir::Successor::SwitchCase { value: 1 },
            program.tree.get(function).block(1),
        );

        assert_eq!(
            constants.constant(mir::Value(3)),
            Some(&mir::Constant::int32(7))
        );
        assert_eq!(constants.value(mir::Value(4)), &ConstantValue::Unknown);
        assert_eq!(constants.edges.iter().copied().collect::<Vec<_>>(), [edge]);
    }
}
