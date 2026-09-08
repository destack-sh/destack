use destack_core::FxIndexMap;

use crate as mir;

use crate::{ConstantTable, DefinitionTable};

/// Canonical pure expression for value numbering.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PureExpression {
    /// Binary operation with operator and operands.
    Binary {
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    },
    /// Unary operation with operator and operand.
    Unary {
        operator: mir::UnaryOperator,
        argument: mir::Value,
    },
    /// Cast to a target type.
    Cast {
        operator: mir::CastOperator,
        argument: mir::Value,
        to_type: mir::TypeId,
    },
    /// Conditional select (pure, can be CSE'd).
    Select {
        condition: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    },
    /// Static field access from an aggregate.
    FieldGet { aggregate: mir::Value, field: u32 },
    /// Static element access from a fixed array.
    ElementGet { aggregate: mir::Value, index: u32 },
    /// Discriminant read from a variant value.
    VariantTag { variant: mir::Value },
    /// Static payload access from a variant value.
    VariantPayload { variant: mir::Value, case: u32 },
}

impl PureExpression {
    /// Create a pure expression for one deduplicable instruction.
    pub fn from_instruction(instruction: &mir::Instruction) -> Option<Self> {
        match instruction {
            mir::Instruction::Error => {
                panic!("invalid MIR instruction reached optimizer");
            }

            // canonicalize commutative binary operations
            mir::Instruction::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let left = *left;
                let right = *right;
                let (left, right) = if operator.is_commutative() && right.0 < left.0 {
                    (right, left)
                } else {
                    (left, right)
                };

                Some(Self::Binary {
                    operator: *operator,
                    left,
                    right,
                })
            }

            // pure unary operation
            mir::Instruction::Unary {
                operator, argument, ..
            } => Some(Self::Unary {
                operator: *operator,
                argument: *argument,
            }),

            // pure cast operation
            mir::Instruction::Cast {
                operator,
                argument,
                to_type,
                ..
            } => Some(Self::Cast {
                operator: *operator,
                argument: *argument,
                to_type: *to_type,
            }),

            // pure value selection operations
            mir::Instruction::Select {
                condition,
                then_value,
                else_value,
                ..
            } => Some(Self::Select {
                condition: *condition,
                then_value: *then_value,
                else_value: *else_value,
            }),
            mir::Instruction::VectorSelect {
                mask,
                then_value,
                else_value,
                ..
            } => Some(Self::Select {
                condition: *mask,
                then_value: *then_value,
                else_value: *else_value,
            }),

            // pure field access
            mir::Instruction::FieldGet {
                aggregate, field, ..
            } => Some(Self::FieldGet {
                aggregate: *aggregate,
                field: *field,
            }),
            mir::Instruction::ElementGet {
                aggregate, index, ..
            } => Some(Self::ElementGet {
                aggregate: *aggregate,
                index: *index,
            }),

            // pure variant projection
            mir::Instruction::VariantTag { variant, .. } => {
                Some(Self::VariantTag { variant: *variant })
            }
            mir::Instruction::VariantPayload { variant, case, .. } => Some(Self::VariantPayload {
                variant: *variant,
                case: *case,
            }),

            // construction is not deduplicated
            mir::Instruction::VariantNew { .. } => None,

            // side effects and unstable reads are not pure expressions
            mir::Instruction::Const { .. }
            | mir::Instruction::Call { .. }
            | mir::Instruction::Drop { .. }
            | mir::Instruction::Intrinsic { .. }
            | mir::Instruction::Load { .. }
            | mir::Instruction::VariantTagLoad { .. }
            | mir::Instruction::Store { .. }
            | mir::Instruction::LocalGet { .. }
            | mir::Instruction::LocalSet { .. }
            | mir::Instruction::NewZeroed { .. }
            | mir::Instruction::NewUninit { .. }
            | mir::Instruction::NewComplete { .. }
            | mir::Instruction::NewSliceZeroed { .. }
            | mir::Instruction::NewSliceUninit { .. }
            | mir::Instruction::Release { .. }
            | mir::Instruction::Aggregate { .. }
            | mir::Instruction::VectorSplat { .. }
            | mir::Instruction::VectorExtract { .. }
            | mir::Instruction::VectorInsert { .. }
            | mir::Instruction::VectorShuffle { .. }
            | mir::Instruction::VectorReduce { .. }
            | mir::Instruction::VectorCompare { .. }
            | mir::Instruction::VectorConvert { .. }
            | mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicStore { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. }
            | mir::Instruction::BarrierWrite { .. }
            | mir::Instruction::FieldSet { .. }
            | mir::Instruction::ElementSet { .. }
            | mir::Instruction::SliceView { .. }
            | mir::Instruction::GlobalAddr { .. }
            | mir::Instruction::FunctionAddr { .. }
            | mir::Instruction::FunctionBind { .. }
            | mir::Instruction::FunctionEnvironment { .. }
            | mir::Instruction::FunctionEnvironmentCurrent { .. }
            | mir::Instruction::ContextCurrent { .. }
            | mir::Instruction::ContextReplace { .. }
            | mir::Instruction::ContextBind { .. }
            | mir::Instruction::ContextGet { .. }
            | mir::Instruction::LocalAddr { .. }
            | mir::Instruction::FieldAddr { .. }
            | mir::Instruction::ElementAddr { .. }
            | mir::Instruction::VariantPayloadAddr { .. }
            | mir::Instruction::Assume { .. }
            | mir::Instruction::SliceLength { .. }
            | mir::Instruction::DynamicBind { .. }
            | mir::Instruction::DynamicPayload { .. }
            | mir::Instruction::DynamicType { .. }
            | mir::Instruction::DynamicRead { .. }
            | mir::Instruction::DynamicFind { .. }
            | mir::Instruction::ProfileIncrement { .. }
            | mir::Instruction::ProfileSample { .. }
            | mir::Instruction::Poll
            | mir::Instruction::Breakpoint => None,
        }
    }

    /// Apply value substitutions to this pure expression.
    pub fn substitute(self, substitutions: &FxIndexMap<mir::Value, mir::Value>) -> Self {
        match self {
            Self::Binary {
                operator,
                left,
                right,
            } => {
                let left = *substitutions.get(&left).unwrap_or(&left);
                let right = *substitutions.get(&right).unwrap_or(&right);
                let (left, right) = if operator.is_commutative() && right.0 < left.0 {
                    (right, left)
                } else {
                    (left, right)
                };

                Self::Binary {
                    operator,
                    left,
                    right,
                }
            }
            Self::Unary { operator, argument } => {
                let argument = *substitutions.get(&argument).unwrap_or(&argument);

                Self::Unary { operator, argument }
            }
            Self::Cast {
                operator,
                argument,
                to_type,
            } => {
                let argument = *substitutions.get(&argument).unwrap_or(&argument);

                Self::Cast {
                    operator,
                    argument,
                    to_type,
                }
            }
            Self::Select {
                condition,
                then_value,
                else_value,
            } => {
                let condition = *substitutions.get(&condition).unwrap_or(&condition);
                let then_value = *substitutions.get(&then_value).unwrap_or(&then_value);
                let else_value = *substitutions.get(&else_value).unwrap_or(&else_value);

                Self::Select {
                    condition,
                    then_value,
                    else_value,
                }
            }
            Self::FieldGet { aggregate, field } => {
                let aggregate = *substitutions.get(&aggregate).unwrap_or(&aggregate);

                Self::FieldGet { aggregate, field }
            }
            Self::VariantTag { variant } => {
                let variant = *substitutions.get(&variant).unwrap_or(&variant);

                Self::VariantTag { variant }
            }
            Self::VariantPayload { variant, case } => {
                let variant = *substitutions.get(&variant).unwrap_or(&variant);

                Self::VariantPayload { variant, case }
            }
            Self::ElementGet { aggregate, index } => {
                let aggregate = *substitutions.get(&aggregate).unwrap_or(&aggregate);

                Self::ElementGet { aggregate, index }
            }
        }
    }
}

/// Cached value equivalence for pure expressions.
#[derive(Debug)]
pub struct ValueEquivalence<'a> {
    /// MIR function.
    function: &'a mir::Function,
    /// MIR tree.
    tree: &'a mir::Tree,
    /// Value definitions.
    definitions: &'a DefinitionTable,
    /// Constant propagation results.
    constants: &'a ConstantTable,
    /// Cache of pairwise equivalence results.
    cache: FxIndexMap<(mir::Value, mir::Value), bool>,
}

impl<'a> ValueEquivalence<'a> {
    /// Create a value equivalence cache.
    pub fn new(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        definitions: &'a DefinitionTable,
        constants: &'a ConstantTable,
    ) -> Self {
        Self {
            function,
            tree,
            definitions,
            constants,
            cache: FxIndexMap::default(),
        }
    }

    /// Return true when two values are provably equivalent.
    pub fn equivalent(
        &mut self,
        left: impl Into<mir::Value>,
        right: impl Into<mir::Value>,
    ) -> bool {
        let left = left.into();
        let right = right.into();

        // handle direct identity
        if left == right {
            return true;
        }

        // canonicalize the cache key
        let (a, b) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        if let Some(result) = self.cache.get(&(a, b)) {
            return *result;
        }

        // compute and store the result
        let result = self.equivalent_impl(left, right);
        self.cache.insert((a, b), result);
        result
    }

    fn equivalent_impl(&mut self, left: mir::Value, right: mir::Value) -> bool {
        if let Some((left_constant, right_constant)) = self.constant_pair(left, right) {
            return left_constant == right_constant;
        }

        let Some(left_inst_id) = self.definitions.instruction(left) else {
            return false;
        };
        let Some(right_inst_id) = self.definitions.instruction(right) else {
            return false;
        };

        let left_inst = self.tree.get(left_inst_id);
        let right_inst = self.tree.get(right_inst_id);

        match (left_inst, right_inst) {
            (
                mir::Instruction::Const {
                    value: left_value, ..
                },
                mir::Instruction::Const {
                    value: right_value, ..
                },
            ) => left_value == right_value,
            (
                mir::Instruction::GlobalAddr {
                    global: left_global,
                    ..
                },
                mir::Instruction::GlobalAddr {
                    global: right_global,
                    ..
                },
            ) => left_global == right_global,
            (
                mir::Instruction::LocalAddr {
                    local: left_local, ..
                },
                mir::Instruction::LocalAddr {
                    local: right_local, ..
                },
            ) => left_local == right_local,
            (
                mir::Instruction::Binary {
                    operator: left_op,
                    left: left_arg,
                    right: right_arg,
                    ..
                },
                mir::Instruction::Binary {
                    operator: right_op,
                    left: right_left,
                    right: right_right,
                    ..
                },
            ) => {
                let left_arg = *left_arg;
                let right_arg = *right_arg;
                let right_left = *right_left;
                let right_right = *right_right;

                if left_op != right_op {
                    return false;
                }

                if !left_op.is_commutative() {
                    return self.equivalent(left_arg, right_left)
                        && self.equivalent(right_arg, right_right);
                }

                (self.equivalent(left_arg, right_left) && self.equivalent(right_arg, right_right))
                    || (self.equivalent(left_arg, right_right)
                        && self.equivalent(right_arg, right_left))
            }
            (
                mir::Instruction::Unary {
                    operator: left_op,
                    argument: left_arg,
                    ..
                },
                mir::Instruction::Unary {
                    operator: right_op,
                    argument: right_arg,
                    ..
                },
            ) => {
                let left_arg = *left_arg;
                let right_arg = *right_arg;

                left_op == right_op && self.equivalent(left_arg, right_arg)
            }
            (
                mir::Instruction::Cast {
                    operator: left_op,
                    argument: left_arg,
                    to_type: left_type,
                    ..
                },
                mir::Instruction::Cast {
                    operator: right_op,
                    argument: right_arg,
                    to_type: right_type,
                    ..
                },
            ) => {
                if left_op != right_op {
                    return false;
                }

                let left_arg = *left_arg;
                let right_arg = *right_arg;

                left_type == right_type && self.equivalent(left_arg, right_arg)
            }
            (
                mir::Instruction::Select {
                    condition: left_cond,
                    then_value: left_then,
                    else_value: left_else,
                    ..
                },
                mir::Instruction::Select {
                    condition: right_cond,
                    then_value: right_then,
                    else_value: right_else,
                    ..
                },
            ) => {
                let left_cond = *left_cond;
                let right_cond = *right_cond;
                let left_then = *left_then;
                let right_then = *right_then;
                let left_else = *left_else;
                let right_else = *right_else;

                self.equivalent(left_cond, right_cond)
                    && self.equivalent(left_then, right_then)
                    && self.equivalent(left_else, right_else)
            }
            (
                mir::Instruction::Aggregate {
                    destination: left_destination,
                    values: left_values,
                    ..
                },
                mir::Instruction::Aggregate {
                    destination: right_destination,
                    values: right_values,
                    ..
                },
            ) => {
                let Some(left_type) = self.function.value_type(*left_destination) else {
                    return false;
                };
                let Some(right_type) = self.function.value_type(*right_destination) else {
                    return false;
                };
                left_type == right_type && self.arguments_equivalent(*left_values, *right_values)
            }
            (
                mir::Instruction::FieldGet {
                    aggregate: left_aggregate,
                    field: left_field,
                    ..
                },
                mir::Instruction::FieldGet {
                    aggregate: right_aggregate,
                    field: right_field,
                    ..
                },
            )
            | (
                mir::Instruction::FieldAddr {
                    aggregate: left_aggregate,
                    field: left_field,
                    ..
                },
                mir::Instruction::FieldAddr {
                    aggregate: right_aggregate,
                    field: right_field,
                    ..
                },
            ) => {
                let left_aggregate = *left_aggregate;
                let right_aggregate = *right_aggregate;

                left_field == right_field && self.equivalent(left_aggregate, right_aggregate)
            }
            (
                mir::Instruction::ElementGet {
                    aggregate: left_aggregate,
                    index: left_index,
                    ..
                },
                mir::Instruction::ElementGet {
                    aggregate: right_aggregate,
                    index: right_index,
                    ..
                },
            ) => {
                let left_aggregate = *left_aggregate;
                let right_aggregate = *right_aggregate;

                left_index == right_index && self.equivalent(left_aggregate, right_aggregate)
            }
            (
                mir::Instruction::ElementAddr {
                    base: left_base,
                    index: left_index,
                    ..
                },
                mir::Instruction::ElementAddr {
                    base: right_base,
                    index: right_index,
                    ..
                },
            ) => {
                let left_base = *left_base;
                let right_base = *right_base;
                let left_index = *left_index;
                let right_index = *right_index;

                self.equivalent(left_base, right_base) && self.equivalent(left_index, right_index)
            }
            _ => false,
        }
    }

    fn arguments_equivalent(&mut self, left: mir::ValueSlice, right: mir::ValueSlice) -> bool {
        let left_args = self.tree.get_values(left);
        let right_args = self.tree.get_values(right);
        if left_args.len() != right_args.len() {
            return false;
        }

        left_args
            .iter()
            .zip(right_args.iter())
            .all(|(left, right)| self.equivalent(*left, *right))
    }

    fn constant_pair(
        &self,
        left: mir::Value,
        right: mir::Value,
    ) -> Option<(&mir::Constant, &mir::Constant)> {
        let left_constant = self.constants.constant(left)?;
        let right_constant = self.constants.constant(right)?;

        Some((left_constant, right_constant))
    }
}
