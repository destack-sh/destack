use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::ConstantPropagation;
use crate::optimize::common::{
    fold_binary, fold_cast, fold_intrinsic, fold_unary, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, terminator_substitute_uses,
};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, TypeContext, resolve_substitution_chains,
};

declare_pass! {
    /// Fold constant expressions at compile time.
    ///
    /// Evaluates operations on constant values and replaces them with the
    /// computed result. This includes arithmetic, comparisons, and logical
    /// operations where all operands are known constants.
    ///
    /// ```mir
    /// function before(): int32 {
    /// b0:
    ///     v0 = 2int32
    ///     v1 = 3int32
    ///     v2 = int.add v0, v1
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    /// b0:
    ///     v0 = 5int32
    ///     return v0
    /// }
    /// ```
    #[pass(id = "constant-fold")]
    pub ConstantFold,
    "Fold constant expressions"
}

impl FunctionPass for ConstantFold {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // get constant propagation analysis
        let constants = {
            let analyses = ctx.function_analyses(function, tree);
            analyses.get::<ConstantPropagation>().clone()
        };

        // run constant folding
        let changed = run_constant_fold(function, tree, &constants, ctx.type_context());

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "ConstantFold"
    }

    fn id(&self) -> &'static str {
        "constant-fold"
    }
}

/// Core constant folding logic. Returns true if changes were made.
fn run_constant_fold(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    constants: &ConstantPropagation,
    type_context: TypeContext,
) -> bool {
    // track pass state and pending rewrites
    let mut changed = false;
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();

    // fold instructions with local constants
    for &block_id in &function.blocks {
        // seed constants for this block
        let mut block_constants = constants.entry(block_id).clone();
        let block = tree.get(block_id);
        let instruction_ids: Vec<_> = block.instructions.clone();

        // walk instructions in order
        for instruction_id in instruction_ids {
            let instruction = tree.get(instruction_id);
            let destination = instruction.destination();

            // fold instruction when possible
            match instruction {
                mir::Instruction::Const { destination, value } => {
                    if let Some(destination) = destination.value() {
                        block_constants.insert(destination, value.clone());
                    }
                }

                mir::Instruction::Binary {
                    destination,
                    operator,
                    left,
                    right,
                } => {
                    // fold binary ops with constant operands
                    let Some(destination) = destination.value() else {
                        continue;
                    };
                    let Some(left) = left.value() else {
                        block_constants.remove(&destination);
                        continue;
                    };
                    let Some(right) = right.value() else {
                        block_constants.remove(&destination);
                        continue;
                    };

                    let left_const = block_constants.get(&left);
                    let right_const = block_constants.get(&right);

                    if let (Some(left_val), Some(right_val)) = (left_const, right_const)
                        && let Some(result) =
                            fold_binary(*operator, left_val.clone(), right_val.clone())
                    {
                        let new_instruction = mir::Instruction::Const {
                            destination: mir::ValueReference::Value(destination),
                            value: result.clone(),
                        };
                        tree.replace(instruction_id, new_instruction);
                        block_constants.insert(destination, result);
                        changed = true;
                    } else {
                        block_constants.remove(&destination);
                    }
                }

                mir::Instruction::Unary {
                    destination,
                    operator,
                    argument,
                } => {
                    // fold unary ops with constant operands
                    let Some(destination) = destination.value() else {
                        continue;
                    };
                    let Some(argument) = argument.value() else {
                        block_constants.remove(&destination);
                        continue;
                    };

                    if let Some(arg_const) = block_constants.get(&argument)
                        && let Some(result) = fold_unary(*operator, arg_const.clone())
                    {
                        let new_instruction = mir::Instruction::Const {
                            destination: mir::ValueReference::Value(destination),
                            value: result.clone(),
                        };
                        tree.replace(instruction_id, new_instruction);
                        block_constants.insert(destination, result);
                        changed = true;
                    } else {
                        block_constants.remove(&destination);
                    }
                }

                mir::Instruction::Select {
                    destination,
                    condition,
                    then_value,
                    else_value,
                } => {
                    // fold select with constant condition
                    let Some(destination) = destination.value() else {
                        continue;
                    };
                    let Some(condition) = condition.value() else {
                        block_constants.remove(&destination);
                        continue;
                    };
                    let Some(then_value) = then_value.value() else {
                        block_constants.remove(&destination);
                        continue;
                    };
                    let Some(else_value) = else_value.value() else {
                        block_constants.remove(&destination);
                        continue;
                    };

                    if let Some(mir::Constant::Boolean { value: cond_val }) =
                        block_constants.get(&condition)
                    {
                        let selected = if *cond_val { then_value } else { else_value };

                        // if selected value is constant, fold to constant
                        if let Some(result) = block_constants.get(selected) {
                            let new_instruction = mir::Instruction::Const {
                                destination: mir::ValueReference::Value(destination),
                                value: result.clone(),
                            };
                            tree.replace(instruction_id, new_instruction);
                            block_constants.insert(destination, result.clone());
                            changed = true;
                        } else {
                            // condition is constant but selected value isn't
                            substitutions.insert(destination, selected);
                            to_remove.insert(instruction_id);
                            block_constants.remove(&destination);
                            changed = true;
                        }
                    } else {
                        block_constants.remove(&destination);
                    }
                }
                mir::Instruction::Cast {
                    destination,
                    operator,
                    argument,
                    to_type,
                } => {
                    // fold casts with constant operands
                    let Some(destination) = destination.value() else {
                        continue;
                    };
                    let Some(argument) = argument.value() else {
                        block_constants.remove(&destination);
                        continue;
                    };
                    let Some(to_type) = to_type.ty() else {
                        block_constants.remove(&destination);
                        continue;
                    };

                    if let Some(arg_const) = block_constants.get(&argument)
                        && let Some(result) = fold_cast(
                            *operator,
                            arg_const.clone(),
                            to_type,
                            type_context.pointer_width_bits,
                            tree,
                        )
                    {
                        let new_instruction = mir::Instruction::Const {
                            destination: mir::ValueReference::Value(destination),
                            value: result.clone(),
                        };
                        tree.replace(instruction_id, new_instruction);
                        block_constants.insert(destination, result);
                        changed = true;
                    } else {
                        block_constants.remove(&destination);
                    }
                }
                mir::Instruction::Intrinsic {
                    destination: Some(destination),
                    intrinsic,
                    arguments,
                    ..
                } => {
                    // fold pure intrinsics with constant arguments
                    let mut constant_arguments = Vec::new();
                    let Some(destination) = destination.value() else {
                        continue;
                    };

                    for &argument in tree.get_arguments(*arguments) {
                        let Some(argument) = argument.value() else {
                            constant_arguments.clear();
                            break;
                        };
                        let Some(constant) = block_constants.get(&argument) else {
                            constant_arguments.clear();
                            break;
                        };
                        constant_arguments.push(constant.clone());
                    }

                    if !constant_arguments.is_empty() && intrinsic.is_pure() {
                        if let Some(result) = fold_intrinsic(*intrinsic, &constant_arguments) {
                            let new_instruction = mir::Instruction::Const {
                                destination: mir::ValueReference::Value(destination),
                                value: result.clone(),
                            };
                            tree.replace(instruction_id, new_instruction);
                            block_constants.insert(destination, result);
                            changed = true;
                        } else {
                            block_constants.remove(&destination);
                        }
                    } else {
                        block_constants.remove(&destination);
                    }
                }
                _ => {
                    // clear destinations for unknown instructions
                    if let Some(dest) = destination {
                        if let Some(dest) = dest.value() {
                            block_constants.remove(&dest);
                        }
                    }
                }
            }
        }
    }

    // apply substitutions and remove redundant instructions
    if !substitutions.is_empty() || !to_remove.is_empty() {
        // resolve transitive substitutions
        let substitutions = resolve_substitution_chains(substitutions);

        for &block_id in &function.blocks {
            let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();
            for instruction_id in instruction_ids {
                // skip instructions that will be removed
                if to_remove.contains(&instruction_id) {
                    continue;
                }

                let instruction = tree.get(instruction_id).clone();
                let updated =
                    instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);

                // replace instructions when substitutions apply
                if updated != instruction {
                    tree.replace(instruction_id, updated);
                    remap_instruction_memory_accesses(tree, instruction_id, &substitutions);
                }
            }
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator).clone();
            let new_terminator = terminator_substitute_uses(&terminator, &substitutions);
            let new_instructions: Vec<_> = block
                .instructions
                .iter()
                .copied()
                .filter(|id| !to_remove.contains(id))
                .collect();

            // rewrite blocks when instructions or terminators change
            if new_terminator != terminator || new_instructions.len() != block.instructions.len() {
                let mut new_block = block.clone();
                new_block.instructions = new_instructions;
                tree.replace(block.terminator, new_terminator);
                tree.replace(block_id, new_block);
            }
        }

        changed = true;
    }

    // fold terminators with constant conditions
    changed |= fold_terminators(function, tree, constants);

    changed
}

/// Fold branch, check, and switch terminators when conditions are constant.
fn fold_terminators(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    constants: &ConstantPropagation,
) -> bool {
    // track whether any terminators change
    let mut changed = false;

    // scan blocks for foldable terminators
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let exit_constants = constants.exit(block_id);

        let new_terminator = match terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let Some(condition) = condition.value() else {
                    continue;
                };
                let condition_constant = exit_constants.get(&condition);
                let condition_value = match condition_constant {
                    Some(mir::Constant::Boolean { value }) => Some(*value),
                    _ => None,
                };

                condition_value.map(|is_true| {
                    let target = if is_true {
                        then_target.clone()
                    } else {
                        else_target.clone()
                    };
                    mir::Terminator::Jump { target }
                })
            }
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                let Some(value) = value.value() else {
                    continue;
                };
                let constant_value = exit_constants.get(&value);
                let selected = match constant_value {
                    Some(mir::Constant::Int { value, .. }) => Some(*value),
                    Some(mir::Constant::UInt { value, .. }) => i64::try_from(*value).ok(),
                    _ => None,
                };

                selected.map(|value| {
                    let mut target = default.clone();
                    for case in cases {
                        if case.value.integer() == Some(value) {
                            target = case.target.clone();
                            break;
                        }
                    }

                    mir::Terminator::Jump { target }
                })
            }
            _ => None,
        };

        // update the terminator when a constant fold applies
        if let Some(new_terminator) = new_terminator {
            tree.replace(block.terminator, new_terminator);
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::common::{fold_binary_signed, fold_unary};

    /// Signed integer addition folds to result, division by zero returns None.
    #[test]
    fn test_fold_binary_signed() {
        assert_eq!(
            fold_binary_signed(1, 2, 32, mir::BinaryOperator::Add),
            Some(mir::Constant::Int {
                value: 3,
                width: 32,
                is_signed: true
            })
        );

        assert_eq!(
            fold_binary_signed(10, 0, 32, mir::BinaryOperator::SignedDivide),
            None
        );
    }

    /// Unary negation folds signed integer to its negated value.
    #[test]
    fn test_fold_unary_negation() {
        assert_eq!(
            fold_unary(
                mir::UnaryOperator::Negate,
                mir::Constant::Int {
                    value: 5,
                    width: 32,
                    is_signed: true
                }
            ),
            Some(mir::Constant::Int {
                value: -5,
                width: 32,
                is_signed: true
            })
        );
    }

    /// Binary add of two constants folds to their sum.
    #[test]
    fn test_fold_binary_add() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = int.add v0, v1
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = 3int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Chained arithmetic operations fold through intermediate results.
    #[test]
    fn test_fold_chained_operations() {
        // 2 * 3 = 6, then 6 + 4 = 10
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = int.mul v0, v1
    v3: int32 = 4int32
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 2int32
    v1: int32 = 3int32
    v2: int32 = 6int32
    v3: int32 = 4int32
    v4: int32 = 10int32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Comparison of constants folds to boolean result.
    #[test]
    fn test_fold_comparison() {
        let input = r#"
function test(): boolean {
b0:
    v0: int32 = 5int32
    v1: int32 = 3int32
    v2: boolean = int.gt.s v0, v1
    return v2
}"#;
        let expected = r#"
function test(): boolean {
b0:
    v0: int32 = 5int32
    v1: int32 = 3int32
    v2: boolean = true
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Operations with non-constant operands are not folded.
    #[test]
    fn test_preserve_non_constant_operands() {
        // v0 is a parameter, not a constant
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 2int32
    v2: int32 = int.add v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_unchanged(input);
    }

    /// Unary negation instruction folds constant to negated value.
    #[test]
    fn test_fold_unary_negation_instruction() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    v1: int32 = int.negate v0
    return v1
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    v1: int32 = -42int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Boolean not folds true to false.
    #[test]
    fn test_fold_boolean_not() {
        let input = r#"
function test(): boolean {
b0:
    v0: boolean = true
    v1: boolean = int.not v0
    return v1
}"#;
        let expected = r#"
function test(): boolean {
b0:
    v0: boolean = true
    v1: boolean = false
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Unsigned integer division folds correctly.
    #[test]
    fn test_fold_unsigned_division() {
        let input = r#"
function test(): uint32 {
b0:
    v0: uint32 = 10uint32
    v1: uint32 = 3uint32
    v2: uint32 = int.div.u v0, v1
    return v2
}"#;
        let expected = r#"
function test(): uint32 {
b0:
    v0: uint32 = 10uint32
    v1: uint32 = 3uint32
    v2: uint32 = 3uint32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Division by zero is not folded to avoid compile-time UB.
    #[test]
    fn test_preserve_division_by_zero() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 0int32
    v2: int32 = int.div.s v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_unchanged(input);
    }

    /// Constants from earlier blocks are available for folding in later blocks.
    #[test]
    fn test_fold_across_blocks() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 5int32
    v2: int32 = 3int32
    branch v0, b1, b2
b1:
    v3: int32 = int.add v1, v2
    return v3
b2:
    v4: int32 = int.mul v1, v2
    return v4
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 5int32
    v2: int32 = 3int32
    branch v0, b1, b2
b1:
    v3: int32 = 8int32
    return v3
b2:
    v4: int32 = 15int32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Readonly global loads do not fold through unary operations.
    #[test]
    fn test_preserve_readonly_global_load_boolean() {
        let input = r#"
global flag: boolean, readonly = true
function test(): boolean {
b0:
    v0: ref<boolean, raw, readonly> = global.address flag
    v1: boolean = load v0
    v2: boolean = int.not v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_unchanged(input);
    }

    /// Mutable globals do not fold through constant operations.
    #[test]
    fn test_preserve_mutable_global_load_boolean() {
        let input = r#"
global flag: boolean = true
function test(): boolean {
b0:
    v0: ref<boolean, raw> = global.address flag
    v1: boolean = load v0
    v2: boolean = int.not v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_unchanged(input);
    }

    /// Block parameter constants fold within successor blocks.
    #[test]
    fn test_fold_block_param_constant() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 3int32
    jump b1(v0)
b1(v1: int32):
    v2: int32 = int.add v1, v1
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 3int32
    jump b1(v0)
b1(v1: int32):
    v2: int32 = 6int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Sign extend casts fold to constants.
    #[test]
    fn test_fold_cast_sign_extend() {
        let input = r#"
function test(): int64 {
b0:
    v0: int32 = -1int32
    v1: int64 = cast.extend.s v0 -> int64
    return v1
}"#;
        let expected = r#"
function test(): int64 {
b0:
    v0: int32 = -1int32
    v1: int64 = -1int64
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Truncate casts fold to constants.
    #[test]
    fn test_fold_cast_truncate() {
        let input = r#"
function test(): uint8 {
b0:
    v0: uint16 = 257uint16
    v1: uint8 = cast.truncate v0 -> uint8
    return v1
}"#;
        let expected = r#"
function test(): uint8 {
b0:
    v0: uint16 = 257uint16
    v1: uint8 = 1uint8
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Bitwise and, or, xor fold correctly on integer constants.
    #[test]
    fn test_fold_bitwise_operations() {
        // 10 & 12 = 8, 10 | 12 = 14, 10 ^ 12 = 6
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 12int32
    v2: int32 = int.and v0, v1
    v3: int32 = int.or v0, v1
    v4: int32 = int.xor v0, v1
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 10int32
    v1: int32 = 12int32
    v2: int32 = 8int32
    v3: int32 = 14int32
    v4: int32 = 6int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Shift left and arithmetic shift right fold correctly.
    #[test]
    fn test_fold_shift_operations() {
        // 8 << 2 = 32, 8 >> 2 = 2 (signed/arithmetic)
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 8int32
    v1: int32 = 2int32
    v2: int32 = int.shiftLeft v0, v1
    v3: int32 = int.shiftRight.s v0, v1
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 8int32
    v1: int32 = 2int32
    v2: int32 = 32int32
    v3: int32 = 2int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Boolean and/or operations fold correctly.
    #[test]
    fn test_fold_boolean_and_or() {
        // true && false = false, true || false = true
        let input = r#"
function test(): boolean {
b0:
    v0: boolean = true
    v1: boolean = false
    v2: boolean = int.and v0, v1
    v3: boolean = int.or v0, v1
    return v2
}"#;
        let expected = r#"
function test(): boolean {
b0:
    v0: boolean = true
    v1: boolean = false
    v2: boolean = false
    v3: boolean = true
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Select with constant true condition folds to then_value.
    #[test]
    fn test_fold_select_true_condition() {
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 42int32
    v2: int32 = 0int32
    v3: int32 = select v0, v1, v2
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 42int32
    v2: int32 = 0int32
    v3: int32 = 42int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Select with constant false condition folds to else_value.
    #[test]
    fn test_fold_select_false_condition() {
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = false
    v1: int32 = 42int32
    v2: int32 = 0int32
    v3: int32 = select v0, v1, v2
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = false
    v1: int32 = 42int32
    v2: int32 = 0int32
    v3: int32 = 0int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Select with non-constant condition is preserved.
    #[test]
    fn test_preserve_select_non_constant_condition() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 42int32
    v2: int32 = 0int32
    v3: int32 = select v0, v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_unchanged(input);
    }

    /// Select with constant condition forwards non constant values.
    #[test]
    fn test_fold_select_constant_to_copy() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: boolean = true
    v2: int32 = int.add v0, v0
    v3: int32 = select v1, v2, v0
    return v3
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: boolean = true
    v2: int32 = int.add v0, v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Branches with constant conditions are folded to jumps.
    #[test]
    fn test_fold_constant_branch() {
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = true
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = true
    jump b1
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }

    /// Pure intrinsics with constant arguments fold to constants.
    #[test]
    fn test_fold_intrinsic_clz() {
        let input = r#"
function test(): uint32 {
b0:
    v0: uint32 = 8uint32
    v1: uint32 = intrinsic.leadingZeroCount(v0)
    return v1
}"#;
        let expected = r#"
function test(): uint32 {
b0:
    v0: uint32 = 8uint32
    v1: uint32 = 28uint32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ConstantFold);
        test.assert_output(expected);
    }
}
