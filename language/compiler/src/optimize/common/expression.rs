use std::collections::HashMap;

use destack_mir as mir;

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
    /// Field access from aggregate.
    FieldGet { aggregate: mir::Value, index: u32 },
    /// Element access from array.
    ElementGet {
        array: mir::Value,
        index: mir::Value,
    },
    /// Load from an immutable global constant.
    GlobalConst {
        global: mir::LocalNodeId<mir::Global>,
    },
}

/// Try to create an expression key for an instruction.
///
/// Returns `None` for instructions that:
/// - Have side effects (calls, stores, allocations)
/// - Are not pure computations (loads, local ops)
/// - Cannot be safely deduplicated (constants handled separately)
pub fn expression_key_from_instruction(
    instruction: &mir::Instruction,
    tree: &mir::NodeTree,
) -> Option<ExpressionKey> {
    match instruction {
        // binary operations
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => {
            // canonicalize commutative ops so (v1 + v0) matches (v0 + v1)
            let (left, right) = if binary_operator_is_commutative(*operator) && right.0 < left.0 {
                (*right, *left)
            } else {
                (*left, *right)
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
            argument: *argument,
        }),

        // cast operations
        mir::Instruction::Cast {
            operator,
            argument,
            to_type,
            ..
        } => {
            let ty = tree.get(*to_type);
            let type_key = TypeKey::from_type(ty, tree);
            Some(ExpressionKey::Cast {
                operator: *operator,
                argument: *argument,
                to_type: type_key,
            })
        }

        // field access (pure, no side effects)
        mir::Instruction::FieldGet {
            aggregate, index, ..
        } => Some(ExpressionKey::FieldGet {
            aggregate: *aggregate,
            index: *index,
        }),

        // element access (pure if no bounds check side effects)
        mir::Instruction::ElementGet { array, index, .. } => Some(ExpressionKey::ElementGet {
            array: *array,
            index: *index,
        }),

        // global constant (immutable, pure)
        mir::Instruction::GlobalConst { global, .. } => {
            Some(ExpressionKey::GlobalConst { global: *global })
        }

        // constants are not CSE'd by expression keys (handled by constant folding)
        // mir::Constant doesn't implement Hash/Eq, and constant deduplication
        // is better handled by dedicated constant merging passes
        mir::Instruction::Const { .. } => None,

        // instructions with side effects or that cannot be safely deduplicated
        mir::Instruction::Call { .. }
        | mir::Instruction::CallIndirect { .. }
        | mir::Instruction::Intrinsic { .. }
        | mir::Instruction::Load { .. }
        | mir::Instruction::Store { .. }
        | mir::Instruction::LocalGet { .. }
        | mir::Instruction::LocalSet { .. }
        | mir::Instruction::ManagedAlloc { .. }
        | mir::Instruction::ManagedAllocArray { .. }
        | mir::Instruction::RawAlloc { .. }
        | mir::Instruction::RawFree { .. }
        | mir::Instruction::StackAlloc { .. }
        | mir::Instruction::Struct { .. }
        | mir::Instruction::Tuple { .. }
        | mir::Instruction::Array { .. }
        | mir::Instruction::FieldSet { .. }
        | mir::Instruction::ElementSet { .. }
        | mir::Instruction::GlobalAddr { .. }
        | mir::Instruction::Drop { .. }
        | mir::Instruction::FieldAddr { .. }
        | mir::Instruction::ElementAddr { .. } => None,
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
/// Re-canonicalizes commutative operations after substitution.
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

            // re-canonicalize after substitution
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

        ExpressionKey::FieldGet { aggregate, index } => {
            let aggregate = *substitutions.get(&aggregate).unwrap_or(&aggregate);
            ExpressionKey::FieldGet { aggregate, index }
        }

        ExpressionKey::ElementGet { array, index } => {
            let array = *substitutions.get(&array).unwrap_or(&array);
            let index = *substitutions.get(&index).unwrap_or(&index);
            ExpressionKey::ElementGet { array, index }
        }

        // global constant has no value operands
        ExpressionKey::GlobalConst { global } => ExpressionKey::GlobalConst { global },
    }
}

/// Resolve transitive substitution chains.
///
/// If we have `v4 -> v2` and `v2 -> v0`, this produces `v4 -> v0` and `v2 -> v0`.
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

        // non-commutative
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
