use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::ConstantPropagation;
use crate::optimize::common::{constant_from_global, fold_binary, fold_cast, fold_unary};
use crate::optimize::{
    AnalysisKind, AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};

declare_pass! {
    /// Fold constant expressions at compile time.
    ///
    /// Evaluates operations on constant values and replaces them with the
    /// computed result. This includes arithmetic, comparisons, and logical
    /// operations where all operands are known constants.
    ///
    /// ```mir
    /// function @before() -> i32 {
    /// block0:
    ///     v0 = iconst 2i32
    ///     v1 = iconst 3i32
    ///     v2 = iadd v0, v1
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after() -> i32 {
    /// block0:
    ///     v0 = iconst 5i32
    ///     return v0
    /// }
    /// ```
    #[pass(id = "constant-fold")]
    pub ConstantFold,
    "Fold constant expressions"
}

impl Pass for ConstantFold {
    fn metadata(&self) -> &'static PassMetadata {
        ConstantFold::metadata()
    }
}

impl FunctionPass for ConstantFold {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        // setup constant propagation state
        let constants = context
            .analyses
            .get::<ConstantPropagation>(function, tree, context);
        let mut changed = false;

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
                        block_constants.insert(*destination, value.clone());
                    }

                    mir::Instruction::GlobalConst {
                        destination,
                        global,
                    } => {
                        // fold immutable scalar globals
                        if let Some(constant) = constant_from_global(*global, tree) {
                            block_constants.insert(*destination, constant.clone());
                            let new_instruction = mir::Instruction::Const {
                                destination: *destination,
                                value: constant,
                            };
                            tree.replace(instruction_id, new_instruction);
                            changed = true;
                        } else {
                            block_constants.remove(*destination);
                        }
                    }

                    mir::Instruction::Binary {
                        destination,
                        operator,
                        left,
                        right,
                    } => {
                        // fold binary ops with constant operands
                        let left_const = block_constants.get(*left);
                        let right_const = block_constants.get(*right);

                        if let (Some(left_val), Some(right_val)) = (left_const, right_const)
                            && let Some(result) =
                                fold_binary(*operator, left_val.clone(), right_val.clone())
                        {
                            let dest = *destination;
                            let new_instruction = mir::Instruction::Const {
                                destination: dest,
                                value: result.clone(),
                            };
                            tree.replace(instruction_id, new_instruction);
                            block_constants.insert(dest, result);
                            changed = true;
                        } else {
                            block_constants.remove(*destination);
                        }
                    }

                    mir::Instruction::Unary {
                        destination,
                        operator,
                        argument,
                    } => {
                        // fold unary ops with constant operands
                        if let Some(arg_const) = block_constants.get(*argument)
                            && let Some(result) = fold_unary(*operator, arg_const.clone())
                        {
                            let dest = *destination;
                            let new_instruction = mir::Instruction::Const {
                                destination: dest,
                                value: result.clone(),
                            };
                            tree.replace(instruction_id, new_instruction);
                            block_constants.insert(dest, result);
                            changed = true;
                        } else {
                            block_constants.remove(*destination);
                        }
                    }

                    mir::Instruction::Select {
                        destination,
                        condition,
                        then_value,
                        else_value,
                    } => {
                        // fold select with constant condition
                        if let Some(mir::Constant::Boolean { value: cond_val }) =
                            block_constants.get(*condition)
                        {
                            let selected = if *cond_val { *then_value } else { *else_value };

                            // if selected value is constant, fold to constant
                            if let Some(result) = block_constants.get(selected) {
                                let dest = *destination;
                                let new_instruction = mir::Instruction::Const {
                                    destination: dest,
                                    value: result.clone(),
                                };
                                tree.replace(instruction_id, new_instruction);
                                block_constants.insert(dest, result.clone());
                                changed = true;
                            } else {
                                // condition is constant but selected value isn't
                                // we could replace with a copy, but let copy_propagate handle it
                                block_constants.remove(*destination);
                            }
                        } else {
                            block_constants.remove(*destination);
                        }
                    }
                    mir::Instruction::Cast {
                        destination,
                        operator,
                        argument,
                        to_type,
                    } => {
                        // fold casts with constant operands
                        if let Some(arg_const) = block_constants.get(*argument)
                            && let Some(result) =
                                fold_cast(*operator, arg_const.clone(), *to_type, tree)
                        {
                            let dest = *destination;
                            let new_instruction = mir::Instruction::Const {
                                destination: dest,
                                value: result.clone(),
                            };
                            tree.replace(instruction_id, new_instruction);
                            block_constants.insert(dest, result);
                            changed = true;
                        } else {
                            block_constants.remove(*destination);
                        }
                    }
                    _ => {
                        // clear destinations for unknown instructions
                        if let Some(dest) = destination {
                            block_constants.remove(dest);
                        }
                    }
                }
            }
        }

        // constant folding doesn't change the CFG, only instruction contents
        if changed {
            AnalysisPreservation::Some(vec![AnalysisKind::ControlFlowGraph])
        } else {
            AnalysisPreservation::all()
        }
    }
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
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iconst 3i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Chained arithmetic operations fold through intermediate results.
    #[test]
    fn test_fold_chained_operations() {
        // 2 * 3 = 6, then 6 + 4 = 10
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = imul v0, v1
    v3 = iconst 4i32
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 2i32
    v1 = iconst 3i32
    v2 = iconst 6i32
    v3 = iconst 4i32
    v4 = iconst 10i32
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Comparison of constants folds to boolean result.
    #[test]
    fn test_fold_comparison() {
        let input = r#"function @test() -> bool {
block0:
    v0 = iconst 5i32
    v1 = iconst 3i32
    v2 = icmp_sgt v0, v1
    return v2
}"#;
        let expected = r#"function @test() -> bool {
block0:
    v0 = iconst 5i32
    v1 = iconst 3i32
    v2 = iconst true
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Operations with non-constant operands are not folded.
    #[test]
    fn test_preserve_non_constant_operands() {
        // v0 is a parameter, not a constant
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_unchanged(input);
    }

    /// Unary negation instruction folds constant to negated value.
    #[test]
    fn test_fold_unary_negation_instruction() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = ineg v0
    return v1
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = iconst -42i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Boolean not folds true to false.
    #[test]
    fn test_fold_boolean_not() {
        let input = r#"function @test() -> bool {
block0:
    v0 = iconst true
    v1 = bnot v0
    return v1
}"#;
        let expected = r#"function @test() -> bool {
block0:
    v0 = iconst true
    v1 = iconst false
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Unsigned integer division folds correctly.
    #[test]
    fn test_fold_unsigned_division() {
        let input = r#"function @test() -> u32 {
block0:
    v0 = iconst 10u32
    v1 = iconst 3u32
    v2 = udiv v0, v1
    return v2
}"#;
        let expected = r#"function @test() -> u32 {
block0:
    v0 = iconst 10u32
    v1 = iconst 3u32
    v2 = iconst 3u32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Division by zero is not folded to avoid compile-time UB.
    #[test]
    fn test_preserve_division_by_zero() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 10i32
    v1 = iconst 0i32
    v2 = sdiv v0, v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_unchanged(input);
    }

    /// Constants from earlier blocks are available for folding in later blocks.
    #[test]
    fn test_fold_across_blocks() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 5i32
    v2 = iconst 3i32
    branch v0, block1, block2
block1:
    v3 = iadd v1, v2
    return v3
block2:
    v4 = imul v1, v2
    return v4
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 5i32
    v2 = iconst 3i32
    branch v0, block1, block2
block1:
    v3 = iconst 8i32
    return v3
block2:
    v4 = iconst 15i32
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Global constants fold through unary operations.
    #[test]
    fn test_fold_global_const_boolean() {
        let input = r#"global @flag: bool = true ; const
function @test() -> bool {
block0:
    v0 = global.const @flag
    v1 = bnot v0
    return v1
}"#;
        let expected = r#"global @flag: bool = true ; const
function @test() -> bool {
block0:
    v0 = iconst true
    v1 = iconst false
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Mutable globals do not fold through constant operations.
    #[test]
    fn test_preserve_mutable_global_const() {
        let input = r#"global @flag: bool = true ; mut
function @test() -> bool {
block0:
    v0 = global.const @flag
    v1 = bnot v0
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_unchanged(input);
    }

    /// Block parameter constants fold within successor blocks.
    #[test]
    fn test_fold_block_param_constant() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 3i32
    jump block1(v0)
block1(v1: i32):
    v2 = iadd v1, v1
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 3i32
    jump block1(v0)
block1(v1: i32):
    v2 = iconst 6i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Sign extend casts fold to constants.
    #[test]
    fn test_fold_cast_sign_extend() {
        let input = r#"function @test() -> i64 {
block0:
    v0 = iconst -1i32
    v1 = sextend v0 -> i64
    return v1
}"#;
        let expected = r#"function @test() -> i64 {
block0:
    v0 = iconst -1i32
    v1 = iconst -1i64
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Truncate casts fold to constants.
    #[test]
    fn test_fold_cast_truncate() {
        let input = r#"function @test() -> u8 {
block0:
    v0 = iconst 257u16
    v1 = trunc v0 -> u8
    return v1
}"#;
        let expected = r#"function @test() -> u8 {
block0:
    v0 = iconst 257u16
    v1 = iconst 1u8
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Bitwise and, or, xor fold correctly on integer constants.
    #[test]
    fn test_fold_bitwise_operations() {
        // 10 & 12 = 8, 10 | 12 = 14, 10 ^ 12 = 6
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 10i32
    v1 = iconst 12i32
    v2 = band v0, v1
    v3 = bor v0, v1
    v4 = bxor v0, v1
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 10i32
    v1 = iconst 12i32
    v2 = iconst 8i32
    v3 = iconst 14i32
    v4 = iconst 6i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Shift left and arithmetic shift right fold correctly.
    #[test]
    fn test_fold_shift_operations() {
        // 8 << 2 = 32, 8 >> 2 = 2 (signed/arithmetic)
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 8i32
    v1 = iconst 2i32
    v2 = ishl v0, v1
    v3 = sshr v0, v1
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 8i32
    v1 = iconst 2i32
    v2 = iconst 32i32
    v3 = iconst 2i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Boolean and/or operations fold correctly.
    #[test]
    fn test_fold_boolean_and_or() {
        // true && false = false, true || false = true
        let input = r#"function @test() -> bool {
block0:
    v0 = iconst true
    v1 = iconst false
    v2 = band v0, v1
    v3 = bor v0, v1
    return v2
}"#;
        let expected = r#"function @test() -> bool {
block0:
    v0 = iconst true
    v1 = iconst false
    v2 = iconst false
    v3 = iconst true
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Select with constant true condition folds to then_value.
    #[test]
    fn test_fold_select_true_condition() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    v2 = iconst 0i32
    v3 = select v0, v1, v2
    return v3
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    v2 = iconst 0i32
    v3 = iconst 42i32
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Select with constant false condition folds to else_value.
    #[test]
    fn test_fold_select_false_condition() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst false
    v1 = iconst 42i32
    v2 = iconst 0i32
    v3 = select v0, v1, v2
    return v3
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst false
    v1 = iconst 42i32
    v2 = iconst 0i32
    v3 = iconst 0i32
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_output(expected);
    }

    /// Select with non-constant condition is preserved.
    #[test]
    fn test_preserve_select_non_constant_condition() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 42i32
    v2 = iconst 0i32
    v3 = select v0, v1, v2
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&ConstantFold);
        program.assert_unchanged(input);
    }
}
