use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::Constant;

use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
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
        _context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        // build map of value -> constant for known constants
        let mut constants: HashMap<mir::Value, mir::Constant> = HashMap::new();

        // scan all blocks for constant definitions
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let mir::Instruction::Const { destination, value } = instruction {
                    constants.insert(*destination, value.clone());
                }
            }
        }

        // now fold binary/unary ops with constant operands
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let instruction_ids: Vec<_> = block.instructions.clone();

            for instruction_id in instruction_ids {
                let instruction = tree.get(instruction_id);

                match instruction {
                    mir::Instruction::Binary {
                        destination,
                        operator,
                        left,
                        right,
                    } => {
                        let left_const = constants.get(left);
                        let right_const = constants.get(right);

                        if let (Some(left_val), Some(right_val)) = (left_const, right_const)
                            && let Some(result) =
                                fold_binary(*operator, left_val.clone(), right_val.clone())
                        {
                            // replace with constant
                            let dest = *destination;
                            let new_instruction = mir::Instruction::Const {
                                destination: dest,
                                value: result.clone(),
                            };
                            tree.replace(instruction_id, new_instruction);
                            constants.insert(dest, result);
                        }
                    }

                    mir::Instruction::Unary {
                        destination,
                        operator,
                        argument,
                    } => {
                        if let Some(arg_const) = constants.get(argument)
                            && let Some(result) = fold_unary(*operator, arg_const.clone())
                        {
                            let dest = *destination;
                            let new_instruction = mir::Instruction::Const {
                                destination: dest,
                                value: result.clone(),
                            };
                            tree.replace(instruction_id, new_instruction);
                            constants.insert(dest, result);
                        }
                    }

                    _ => {}
                }
            }
        }

        // constant folding doesn't change the CFG, only instruction contents
        AnalysisPreservation::all()
    }
}

/// Try to fold a binary operation on constants.
fn fold_binary(
    operator: mir::BinaryOperator,
    left: mir::Constant,
    right: mir::Constant,
) -> Option<Constant> {
    match (&left, &right) {
        // signed integer operations (must have same width)
        (
            Constant::Int {
                value: l,
                width: lw,
                is_signed: true,
            },
            Constant::Int {
                value: r,
                width: rw,
                is_signed: true,
            },
        ) if lw == rw => fold_binary_signed(*l, *r, *lw, operator),

        // unsigned integer operations (must have same width)
        (
            Constant::UInt {
                value: l,
                width: lw,
            },
            Constant::UInt {
                value: r,
                width: rw,
            },
        ) if lw == rw => fold_binary_unsigned(*l, *r, *lw, operator),

        // float operations (must have same width)
        (
            Constant::Float {
                bits: lb,
                width: lw,
            },
            Constant::Float {
                bits: rb,
                width: rw,
            },
        ) if lw == rw => fold_binary_float(*lb, *rb, *lw, operator),

        // boolean operations
        (Constant::Boolean { value: l }, Constant::Boolean { value: r }) => {
            fold_binary_bool(*l, *r, operator)
        }

        _ => None,
    }
}

/// Fold a binary operation on signed integers.
fn fold_binary_signed(
    left: i64,
    right: i64,
    width: u8,
    operator: mir::BinaryOperator,
) -> Option<mir::Constant> {
    let result_int = |value: i64| {
        Some(mir::Constant::Int {
            value,
            width,
            is_signed: true,
        })
    };
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    match operator {
        mir::BinaryOperator::Add => result_int(left.wrapping_add(right)),
        mir::BinaryOperator::Subtract => result_int(left.wrapping_sub(right)),
        mir::BinaryOperator::Multiply => result_int(left.wrapping_mul(right)),
        mir::BinaryOperator::SignedDivide => {
            if right != 0 {
                result_int(left.wrapping_div(right))
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedRemainder => {
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
        mir::BinaryOperator::ArithmeticShiftRight => result_int(left.wrapping_shr(right as u32)),
        mir::BinaryOperator::Equal => result_bool(left == right),
        mir::BinaryOperator::NotEqual => result_bool(left != right),
        mir::BinaryOperator::SignedLessThan => result_bool(left < right),
        mir::BinaryOperator::SignedLessEqual => result_bool(left <= right),
        mir::BinaryOperator::SignedGreaterThan => result_bool(left > right),
        mir::BinaryOperator::SignedGreaterEqual => result_bool(left >= right),
        _ => None,
    }
}

/// Fold a binary operation on unsigned integers.
fn fold_binary_unsigned(
    left: u64,
    right: u64,
    width: u8,
    operator: mir::BinaryOperator,
) -> Option<mir::Constant> {
    let result_uint = |value: u64| Some(mir::Constant::UInt { value, width });
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    match operator {
        mir::BinaryOperator::Add => result_uint(left.wrapping_add(right)),
        mir::BinaryOperator::Subtract => result_uint(left.wrapping_sub(right)),
        mir::BinaryOperator::Multiply => result_uint(left.wrapping_mul(right)),
        mir::BinaryOperator::UnsignedDivide => {
            if right != 0 {
                result_uint(left.wrapping_div(right))
            } else {
                None
            }
        }
        mir::BinaryOperator::UnsignedRemainder => {
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
        mir::BinaryOperator::LogicalShiftRight => result_uint(left.wrapping_shr(right as u32)),
        mir::BinaryOperator::Equal => result_bool(left == right),
        mir::BinaryOperator::NotEqual => result_bool(left != right),
        mir::BinaryOperator::UnsignedLessThan => result_bool(left < right),
        mir::BinaryOperator::UnsignedLessEqual => result_bool(left <= right),
        mir::BinaryOperator::UnsignedGreaterThan => result_bool(left > right),
        mir::BinaryOperator::UnsignedGreaterEqual => result_bool(left >= right),
        _ => None,
    }
}

/// Fold a binary operation on floats.
fn fold_binary_float(
    left_bits: u64,
    right_bits: u64,
    width: u8,
    operator: mir::BinaryOperator,
) -> Option<mir::Constant> {
    let result_float = |value: f64| {
        Some(mir::Constant::Float {
            bits: value.to_bits(),
            width,
        })
    };
    let result_bool = |value: bool| Some(mir::Constant::Boolean { value });

    if width == 32 {
        let left = f32::from_bits(left_bits as u32);
        let right = f32::from_bits(right_bits as u32);
        match operator {
            mir::BinaryOperator::FloatAdd => result_float((left + right) as f64),
            mir::BinaryOperator::FloatSubtract => result_float((left - right) as f64),
            mir::BinaryOperator::FloatMultiply => result_float((left * right) as f64),
            mir::BinaryOperator::FloatDivide => result_float((left / right) as f64),
            mir::BinaryOperator::FloatEqual => result_bool(left == right),
            mir::BinaryOperator::FloatNotEqual => result_bool(left != right),
            mir::BinaryOperator::FloatLessThan => result_bool(left < right),
            mir::BinaryOperator::FloatLessEqual => result_bool(left <= right),
            mir::BinaryOperator::FloatGreaterThan => result_bool(left > right),
            mir::BinaryOperator::FloatGreaterEqual => result_bool(left >= right),
            _ => None,
        }
    } else if width == 64 {
        let left = f64::from_bits(left_bits);
        let right = f64::from_bits(right_bits);
        match operator {
            mir::BinaryOperator::FloatAdd => result_float(left + right),
            mir::BinaryOperator::FloatSubtract => result_float(left - right),
            mir::BinaryOperator::FloatMultiply => result_float(left * right),
            mir::BinaryOperator::FloatDivide => result_float(left / right),
            mir::BinaryOperator::FloatEqual => result_bool(left == right),
            mir::BinaryOperator::FloatNotEqual => result_bool(left != right),
            mir::BinaryOperator::FloatLessThan => result_bool(left < right),
            mir::BinaryOperator::FloatLessEqual => result_bool(left <= right),
            mir::BinaryOperator::FloatGreaterThan => result_bool(left > right),
            mir::BinaryOperator::FloatGreaterEqual => result_bool(left >= right),
            _ => None,
        }
    } else {
        None
    }
}

/// Fold a binary operation on booleans.
fn fold_binary_bool(
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

/// Try to fold a unary operation on a constant.
fn fold_unary(operator: mir::UnaryOperator, value: mir::Constant) -> Option<mir::Constant> {
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

        (mir::UnaryOperator::FloatNegate, mir::Constant::Float { bits, width }) => {
            if *width == 32 {
                let f = f32::from_bits(*bits as u32);
                Some(mir::Constant::Float {
                    bits: ((-f).to_bits()) as u64,
                    width: 32,
                })
            } else if *width == 64 {
                let f = f64::from_bits(*bits);
                Some(mir::Constant::Float {
                    bits: (-f).to_bits(),
                    width: 64,
                })
            } else {
                None
            }
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
    use crate::optimize::common::tests::TestProgram;

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
}
