use std::collections::HashMap;

use destack_mir as mir;

use crate::common::mir::analysis::ConstantPropagation;

use super::TypeKey;

/// Hashable key for identifying equivalent expressions in value numbering.
///
/// Two instructions with the same key compute the same value, assuming no
/// intervening side effects. Used by local CSE and global value numbering to
/// detect redundant computations.
///
/// Keys are designed for use in hash maps: they implement `Hash` and `Eq` based
/// on structural equivalence rather than identity. For example, two casts to
/// structurally identical types will have equal keys even if the types have
/// different node IDs in the tree.
///
/// Commutative operations are canonicalized so operand order doesn't matter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExpressionKey {
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
        to_type: TypeKey,
    },
    /// Conditional select (pure, can be CSE'd).
    Select {
        condition: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    },
    /// Field access from aggregate.
    FieldGet { aggregate: mir::Value, index: u32 },
    /// Element access from array.
    ElementGet { array: mir::Value, index: u32 },
}

/// Cached value equivalence for pure expressions.
#[derive(Debug)]
pub struct ValueEquivalence<'a> {
    /// MIR tree.
    tree: &'a mir::Tree,
    /// Map from values to their defining instructions.
    definitions: &'a HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    /// Constant propagation results when available.
    constants: Option<&'a ConstantPropagation>,
    /// Instruction to block ownership for constant lookup.
    instruction_blocks:
        Option<&'a HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>>,
    /// Cache of pairwise equivalence results.
    cache: HashMap<(mir::Value, mir::Value), bool>,
    /// Cached type keys.
    type_keys: HashMap<mir::LocalNodeId<mir::Type>, TypeKey>,
}

impl<'a> ValueEquivalence<'a> {
    /// Create a new value equivalence cache.
    pub fn new(
        tree: &'a mir::Tree,
        definitions: &'a HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    ) -> Self {
        Self {
            tree,
            definitions,
            constants: None,
            instruction_blocks: None,
            cache: HashMap::new(),
            type_keys: HashMap::new(),
        }
    }

    /// Create a new value equivalence cache with constant propagation support.
    pub fn new_with_constants(
        tree: &'a mir::Tree,
        definitions: &'a HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
        constants: &'a ConstantPropagation,
        instruction_blocks: &'a HashMap<
            mir::LocalNodeId<mir::Instruction>,
            mir::LocalNodeId<mir::Block>,
        >,
    ) -> Self {
        Self {
            tree,
            definitions,
            constants: Some(constants),
            instruction_blocks: Some(instruction_blocks),
            cache: HashMap::new(),
            type_keys: HashMap::new(),
        }
    }

    /// Return true when two values are provably equivalent.
    pub fn equivalent(
        &mut self,
        left: impl Into<mir::ValueReference>,
        right: impl Into<mir::ValueReference>,
    ) -> bool {
        let Some(left) = left.into().value() else {
            return false;
        };
        let Some(right) = right.into().value() else {
            return false;
        };

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

        let Some(left_inst_id) = self.definitions.get(&left) else {
            return false;
        };
        let Some(right_inst_id) = self.definitions.get(&right) else {
            return false;
        };

        let left_inst = self.tree.get(*left_inst_id);
        let right_inst = self.tree.get(*right_inst_id);

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
                let (Some(left_arg), Some(right_arg), Some(right_left), Some(right_right)) = (
                    left_arg.value(),
                    right_arg.value(),
                    right_left.value(),
                    right_right.value(),
                ) else {
                    return false;
                };

                if left_op != right_op {
                    return false;
                }

                if !binary_operator_is_commutative(*left_op) {
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
                let (Some(left_arg), Some(right_arg)) = (left_arg.value(), right_arg.value())
                else {
                    return false;
                };

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

                let (Some(left_key), Some(right_key), Some(left_arg), Some(right_arg)) = (
                    self.type_key(*left_type),
                    self.type_key(*right_type),
                    left_arg.value(),
                    right_arg.value(),
                ) else {
                    return false;
                };

                left_key == right_key && self.equivalent(left_arg, right_arg)
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
                let (
                    Some(left_cond),
                    Some(right_cond),
                    Some(left_then),
                    Some(right_then),
                    Some(left_else),
                    Some(right_else),
                ) = (
                    left_cond.value(),
                    right_cond.value(),
                    left_then.value(),
                    right_then.value(),
                    left_else.value(),
                    right_else.value(),
                )
                else {
                    return false;
                };

                self.equivalent(left_cond, right_cond)
                    && self.equivalent(left_then, right_then)
                    && self.equivalent(left_else, right_else)
            }
            (
                mir::Instruction::FieldGet {
                    aggregate: left_aggregate,
                    index: left_index,
                    ..
                },
                mir::Instruction::FieldGet {
                    aggregate: right_aggregate,
                    index: right_index,
                    ..
                },
            )
            | (
                mir::Instruction::FieldAddr {
                    aggregate: left_aggregate,
                    index: left_index,
                    ..
                },
                mir::Instruction::FieldAddr {
                    aggregate: right_aggregate,
                    index: right_index,
                    ..
                },
            ) => {
                let (Some(left_aggregate), Some(right_aggregate)) =
                    (left_aggregate.value(), right_aggregate.value())
                else {
                    return false;
                };

                left_index == right_index && self.equivalent(left_aggregate, right_aggregate)
            }
            (
                mir::Instruction::ElementGet {
                    array: left_array,
                    index: left_index,
                    ..
                },
                mir::Instruction::ElementGet {
                    array: right_array,
                    index: right_index,
                    ..
                },
            ) => {
                let (Some(left_array), Some(right_array)) =
                    (left_array.value(), right_array.value())
                else {
                    return false;
                };

                left_index == right_index && self.equivalent(left_array, right_array)
            }
            (
                mir::Instruction::ElementAddr {
                    array: left_array,
                    index: left_index,
                    ..
                },
                mir::Instruction::ElementAddr {
                    array: right_array,
                    index: right_index,
                    ..
                },
            ) => {
                let (Some(left_array), Some(right_array), Some(left_index), Some(right_index)) = (
                    left_array.value(),
                    right_array.value(),
                    left_index.value(),
                    right_index.value(),
                ) else {
                    return false;
                };

                self.equivalent(left_array, right_array) && self.equivalent(left_index, right_index)
            }
            (
                mir::Instruction::Struct {
                    ty: left_type,
                    fields: left_fields,
                    ..
                },
                mir::Instruction::Struct {
                    ty: right_type,
                    fields: right_fields,
                    ..
                },
            ) => {
                let (Some(left_key), Some(right_key)) =
                    (self.type_key(*left_type), self.type_key(*right_type))
                else {
                    return false;
                };

                left_key == right_key && self.arguments_equivalent(*left_fields, *right_fields)
            }
            (
                mir::Instruction::Tuple {
                    ty: left_type,
                    elements: left_elements,
                    ..
                },
                mir::Instruction::Tuple {
                    ty: right_type,
                    elements: right_elements,
                    ..
                },
            )
            | (
                mir::Instruction::Array {
                    ty: left_type,
                    elements: left_elements,
                    ..
                },
                mir::Instruction::Array {
                    ty: right_type,
                    elements: right_elements,
                    ..
                },
            ) => {
                let (Some(left_key), Some(right_key)) =
                    (self.type_key(*left_type), self.type_key(*right_type))
                else {
                    return false;
                };

                left_key == right_key && self.arguments_equivalent(*left_elements, *right_elements)
            }
            _ => false,
        }
    }

    fn arguments_equivalent(
        &mut self,
        left: mir::ArgumentSlice,
        right: mir::ArgumentSlice,
    ) -> bool {
        let left_args = self.tree.get_arguments(left);
        let right_args = self.tree.get_arguments(right);
        if left_args.len() != right_args.len() {
            return false;
        }

        left_args
            .iter()
            .zip(right_args.iter())
            .all(|(left, right)| match (left.value(), right.value()) {
                (Some(left), Some(right)) => self.equivalent(left, right),
                _ => false,
            })
    }

    fn type_key(&mut self, ty: impl Into<mir::TypeReference>) -> Option<TypeKey> {
        let ty = ty.into().ty()?;

        if let Some(existing) = self.type_keys.get(&ty) {
            return Some(existing.clone());
        }

        let key = TypeKey::from_type(ty, self.tree);
        self.type_keys.insert(ty, key.clone());

        Some(key)
    }

    fn constant_pair(
        &self,
        left: mir::Value,
        right: mir::Value,
    ) -> Option<(&mir::Constant, &mir::Constant)> {
        let constants = self.constants?;
        let instruction_blocks = self.instruction_blocks?;
        let left_inst_id = self.definitions.get(&left)?;
        let right_inst_id = self.definitions.get(&right)?;
        let left_block = instruction_blocks.get(left_inst_id)?;
        let right_block = instruction_blocks.get(right_inst_id)?;
        let left_constant = constants
            .constant_at_exit(*left_block, left)
            .or_else(|| constants.constant_at_entry(*left_block, left))?;
        let right_constant = constants
            .constant_at_exit(*right_block, right)
            .or_else(|| constants.constant_at_entry(*right_block, right))?;
        Some((left_constant, right_constant))
    }
}

/// Try to create an expression key for an instruction.
///
/// Returns `None` for instructions with side effects such as calls and stores.
/// Returns `None` for instructions that are not pure computations like loads.
/// Returns `None` for instructions that cannot be safely deduplicated.
pub fn expression_key_from_instruction(
    instruction: &mir::Instruction,
    tree: &mir::Tree,
) -> Option<ExpressionKey> {
    match instruction {
        mir::Instruction::Error => {
            panic!("recovered MIR instruction reached optimizer");
        }

        // binary operations
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => {
            let (Some(left), Some(right)) = (left.value(), right.value()) else {
                return None;
            };

            // canonicalize commutative ops so (v1 + v0) matches (v0 + v1)
            let (left, right) = if binary_operator_is_commutative(*operator) && right.0 < left.0 {
                (right, left)
            } else {
                (left, right)
            };
            Some(ExpressionKey::Binary {
                operator: *operator,
                left,
                right,
            })
        }

        // unary operations
        mir::Instruction::Unary {
            operator, argument, ..
        } => Some(ExpressionKey::Unary {
            operator: *operator,
            argument: argument.value()?,
        }),

        // cast operations
        mir::Instruction::Cast {
            operator,
            argument,
            to_type,
            ..
        } => {
            let type_key = TypeKey::from_type(to_type.ty()?, tree);
            Some(ExpressionKey::Cast {
                operator: *operator,
                argument: argument.value()?,
                to_type: type_key,
            })
        }

        // select (pure, no side effects)
        mir::Instruction::Select {
            condition,
            then_value,
            else_value,
            ..
        } => Some(ExpressionKey::Select {
            condition: condition.value()?,
            then_value: then_value.value()?,
            else_value: else_value.value()?,
        }),
        mir::Instruction::VectorSelect {
            mask,
            then_value,
            else_value,
            ..
        } => Some(ExpressionKey::Select {
            condition: mask.value()?,
            then_value: then_value.value()?,
            else_value: else_value.value()?,
        }),
        mir::Instruction::TensorSelect {
            mask,
            then_value,
            else_value,
            ..
        } => Some(ExpressionKey::Select {
            condition: mask.value()?,
            then_value: then_value.value()?,
            else_value: else_value.value()?,
        }),

        // field access (pure, no side effects)
        mir::Instruction::FieldGet {
            aggregate, index, ..
        } => Some(ExpressionKey::FieldGet {
            aggregate: aggregate.value()?,
            index: *index,
        }),

        // element access (pure if no bounds check side effects)
        mir::Instruction::ElementGet { array, index, .. } => Some(ExpressionKey::ElementGet {
            array: array.value()?,
            index: *index,
        }),

        // constants are not CSE'd by expression keys (handled by constant folding)
        // mir::Constant doesn't implement Hash/Eq, and constant deduplication
        // is better handled by dedicated constant merging passes
        mir::Instruction::Const { .. } => None,

        // instructions with side effects or that cannot be safely deduplicated
        mir::Instruction::Call { .. }
        | mir::Instruction::CallClass { .. }
        | mir::Instruction::CallInterface { .. }
        | mir::Instruction::CallIndirect { .. }
        | mir::Instruction::Intrinsic { .. }
        | mir::Instruction::Load { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::LocalGet { .. }
        | mir::Instruction::LocalSet { .. }
        | mir::Instruction::New { .. }
        | mir::Instruction::NewSlice { .. }
        | mir::Instruction::RawAlloc { .. }
        | mir::Instruction::RawFree { .. }
        | mir::Instruction::Free { .. }
        | mir::Instruction::Pin { .. }
        | mir::Instruction::Unpin { .. }
        | mir::Instruction::Drop { .. }
        | mir::Instruction::StackAlloc { .. }
        | mir::Instruction::Struct { .. }
        | mir::Instruction::Tuple { .. }
        | mir::Instruction::Array { .. }
        | mir::Instruction::VectorSplat { .. }
        | mir::Instruction::VectorExtract { .. }
        | mir::Instruction::VectorInsert { .. }
        | mir::Instruction::VectorShuffle { .. }
        | mir::Instruction::VectorReduce { .. }
        | mir::Instruction::VectorCompare { .. }
        | mir::Instruction::VectorConvert { .. }
        | mir::Instruction::TensorSplat { .. }
        | mir::Instruction::TensorExtract { .. }
        | mir::Instruction::TensorLoad { .. }
        | mir::Instruction::TensorStore { .. }
        | mir::Instruction::TensorFill { .. }
        | mir::Instruction::TensorCopy { .. }
        | mir::Instruction::TensorReshape { .. }
        | mir::Instruction::TensorBroadcast { .. }
        | mir::Instruction::TensorTranspose { .. }
        | mir::Instruction::TensorCast { .. }
        | mir::Instruction::TensorView { .. }
        | mir::Instruction::TensorSlice { .. }
        | mir::Instruction::TensorPad { .. }
        | mir::Instruction::TensorConcat { .. }
        | mir::Instruction::TensorReduce { .. }
        | mir::Instruction::TensorDot { .. }
        | mir::Instruction::TensorConvolution { .. }
        | mir::Instruction::TensorGather { .. }
        | mir::Instruction::TensorScatter { .. }
        | mir::Instruction::TensorCompare { .. }
        | mir::Instruction::TensorConvert { .. }
        | mir::Instruction::AtomicLoad { .. }
        | mir::Instruction::AtomicStore { .. }
        | mir::Instruction::AtomicCompareExchange { .. }
        | mir::Instruction::AtomicRmw { .. }
        | mir::Instruction::AtomicFence { .. }
        | mir::Instruction::BarrierWrite { .. }
        | mir::Instruction::FieldSet { .. }
        | mir::Instruction::ElementSet { .. }
        | mir::Instruction::Slice { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::FunctionAddr { .. }
        | mir::Instruction::CallableBind { .. }
        | mir::Instruction::CallableEnvironment { .. }
        | mir::Instruction::LocalAddr { .. }
        | mir::Instruction::FieldAddr { .. }
        | mir::Instruction::ElementAddr { .. }
        | mir::Instruction::Assume { .. } => None,
    }
}

/// Check if a binary operator is commutative.
///
/// For commutative operators, operand order does not affect the result.
/// This enables matching `a + b` with `b + a`.
pub fn binary_operator_is_commutative(operator: mir::BinaryOperator) -> bool {
    use mir::BinaryOperator::*;
    matches!(
        operator,
        Add | Multiply
            | FloatAdd
            | FloatMultiply
            | And
            | Or
            | Xor
            | Equal
            | NotEqual
            | FloatEqual
            | FloatNotEqual
    )
}

/// Apply value substitutions to an expression key.
///
/// Replaces value references in the key according to the substitution map.
/// Re canonicalizes commutative operations after substitution.
pub fn expression_key_substitute(
    key: ExpressionKey,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> ExpressionKey {
    match key {
        ExpressionKey::Binary {
            operator,
            left,
            right,
        } => {
            let left = *substitutions.get(&left).unwrap_or(&left);
            let right = *substitutions.get(&right).unwrap_or(&right);

            // re canonicalize after substitution
            let (left, right) = if binary_operator_is_commutative(operator) && right.0 < left.0 {
                (right, left)
            } else {
                (left, right)
            };

            ExpressionKey::Binary {
                operator,
                left,
                right,
            }
        }

        ExpressionKey::Unary { operator, argument } => {
            let argument = *substitutions.get(&argument).unwrap_or(&argument);
            ExpressionKey::Unary { operator, argument }
        }

        ExpressionKey::Cast {
            operator,
            argument,
            to_type,
        } => {
            let argument = *substitutions.get(&argument).unwrap_or(&argument);
            ExpressionKey::Cast {
                operator,
                argument,
                to_type,
            }
        }

        ExpressionKey::Select {
            condition,
            then_value,
            else_value,
        } => {
            let condition = *substitutions.get(&condition).unwrap_or(&condition);
            let then_value = *substitutions.get(&then_value).unwrap_or(&then_value);
            let else_value = *substitutions.get(&else_value).unwrap_or(&else_value);
            ExpressionKey::Select {
                condition,
                then_value,
                else_value,
            }
        }

        ExpressionKey::FieldGet { aggregate, index } => {
            let aggregate = *substitutions.get(&aggregate).unwrap_or(&aggregate);
            ExpressionKey::FieldGet { aggregate, index }
        }

        ExpressionKey::ElementGet { array, index } => {
            let array = *substitutions.get(&array).unwrap_or(&array);
            ExpressionKey::ElementGet { array, index }
        }
    }
}

/// Resolve transitive substitution chains.
///
/// If we have `v4` mapping to `v2` and `v2` mapping to `v0`, this produces `v4` to `v0` and `v2` to `v0`.
/// Handles cycles by stopping when a value maps to itself.
pub fn resolve_substitution_chains(
    mut substitutions: HashMap<mir::Value, mir::Value>,
) -> HashMap<mir::Value, mir::Value> {
    let keys: Vec<_> = substitutions.keys().copied().collect();
    for key in keys {
        let mut current = substitutions[&key];

        // follow the chain until we hit a fixed point
        while let Some(&next) = substitutions.get(&current) {
            if next == current {
                break;
            }
            current = next;
        }

        substitutions.insert(key, current);
    }

    substitutions
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Commutative operators are correctly identified.
    #[test]
    fn test_is_commutative() {
        // commutative
        assert!(binary_operator_is_commutative(mir::BinaryOperator::Add));
        assert!(binary_operator_is_commutative(
            mir::BinaryOperator::Multiply
        ));
        assert!(binary_operator_is_commutative(
            mir::BinaryOperator::FloatAdd
        ));
        assert!(binary_operator_is_commutative(
            mir::BinaryOperator::FloatMultiply
        ));
        assert!(binary_operator_is_commutative(mir::BinaryOperator::And));
        assert!(binary_operator_is_commutative(mir::BinaryOperator::Or));
        assert!(binary_operator_is_commutative(mir::BinaryOperator::Xor));
        assert!(binary_operator_is_commutative(mir::BinaryOperator::Equal));
        assert!(binary_operator_is_commutative(
            mir::BinaryOperator::NotEqual
        ));

        // non commutative
        assert!(!binary_operator_is_commutative(
            mir::BinaryOperator::Subtract
        ));
        assert!(!binary_operator_is_commutative(
            mir::BinaryOperator::SignedDivide
        ));
        assert!(!binary_operator_is_commutative(
            mir::BinaryOperator::UnsignedDivide
        ));
        assert!(!binary_operator_is_commutative(
            mir::BinaryOperator::SignedLessThan
        ));
        assert!(!binary_operator_is_commutative(
            mir::BinaryOperator::ShiftLeft
        ));
    }

    /// Substitution chains are resolved transitively.
    #[test]
    fn test_resolve_substitution_chains() {
        let mut subs = HashMap::new();
        subs.insert(mir::Value(4), mir::Value(2));
        subs.insert(mir::Value(2), mir::Value(0));

        let resolved = resolve_substitution_chains(subs);
        assert_eq!(resolved.get(&mir::Value(4)), Some(&mir::Value(0)));
        assert_eq!(resolved.get(&mir::Value(2)), Some(&mir::Value(0)));
    }
}
