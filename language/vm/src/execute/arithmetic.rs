use destack_bytecode::{
    FloatOperation, IntegerOperation, ReduceOperation, Scalar, ScatterOperation,
};
use destack_program::Word;

use crate::diagnostic::{Error, Result};

/// Stateless scalar arithmetic shared by bytecode operations.
pub(super) struct Arithmetic;

impl Arithmetic {
    /// Multiply two scalar values and add them to one accumulator.
    pub(super) fn product_sum(
        scalar: Scalar,
        accumulator: Word,
        left: Word,
        right: Word,
    ) -> Result<Word> {
        if scalar.is_float() {
            Self::float(
                FloatOperation::FusedMultiplyAdd,
                scalar,
                left,
                Some(right),
                Some(accumulator),
            )
        } else if scalar.is_integer() {
            let product = Self::integer(IntegerOperation::Multiply, scalar, left, Some(right))?;

            Self::integer(IntegerOperation::Add, scalar, accumulator, Some(product))
        } else {
            Err(Error::invalid_instruction())
        }
    }

    /// Apply one scalar scatter combination.
    pub(super) fn scatter(
        operation: ScatterOperation,
        scalar: Scalar,
        left: Word,
        right: Word,
    ) -> Result<Word> {
        let operation = match operation {
            ScatterOperation::Replace => return Ok(right),
            ScatterOperation::Add => ReduceOperation::Add,
            ScatterOperation::Multiply => ReduceOperation::Multiply,
            ScatterOperation::Minimum => ReduceOperation::Minimum,
            ScatterOperation::Maximum => ReduceOperation::Maximum,
            ScatterOperation::And => ReduceOperation::And,
            ScatterOperation::Or => ReduceOperation::Or,
            ScatterOperation::Xor => ReduceOperation::Xor,
        };

        Self::reduce(operation, scalar, left, right)
    }
}
