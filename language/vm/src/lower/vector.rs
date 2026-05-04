use destack_mir as mir;

use crate::program::{Instruction, Op};
use crate::{Error, Result};

use super::arithmetic::binary_operator_operand;
use super::lower::BlockLowerer;
use super::pool::Pool;

/// Return one vector reduction operator operand.
fn vector_reduce_operator_operand(operator: mir::VectorReduceOperator) -> u32 {
    match operator {
        mir::VectorReduceOperator::Add => 0,
        mir::VectorReduceOperator::Multiply => 1,
        mir::VectorReduceOperator::Min => 2,
        mir::VectorReduceOperator::Max => 3,
        mir::VectorReduceOperator::And => 4,
        mir::VectorReduceOperator::Or => 5,
        mir::VectorReduceOperator::Xor => 6,
    }
}

/// Return the vector conversion operation for one mode.
fn vector_convert_op(mode: mir::VectorConvertMode) -> Op {
    match mode {
        mir::VectorConvertMode::Exact => Op::VectorConvertExact,
        mir::VectorConvertMode::RoundTiesEven => Op::VectorConvertRoundTiesEven,
        mir::VectorConvertMode::RoundTowardZero => Op::VectorConvertRoundTowardZero,
        mir::VectorConvertMode::RoundFloor => Op::VectorConvertRoundFloor,
        mir::VectorConvertMode::RoundCeil => Op::VectorConvertRoundCeil,
        mir::VectorConvertMode::Saturate => Op::VectorConvertSaturate,
    }
}

impl<'a> BlockLowerer<'a> {
    /// Lower one vector splat.
    pub(super) fn lower_vector_splat(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector splat destination".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector splat value".to_string(),
        })?;

        Ok(Instruction::new(
            Op::VectorSplat,
            destination.id(),
            value.id(),
            0,
            0,
        ))
    }

    /// Lower one vector extract.
    pub(super) fn lower_vector_extract(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        vector: mir::ValueReference,
        index: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector extract destination".to_string(),
            })?;
        let vector = vector.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector extract input".to_string(),
        })?;
        let index = index.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector extract index".to_string(),
        })?;

        Ok(Instruction::new(
            Op::VectorExtract,
            destination.id(),
            vector.id(),
            index.id(),
            0,
        ))
    }

    /// Lower one vector insert.
    pub(super) fn lower_vector_insert(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        vector: mir::ValueReference,
        index: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector insert destination".to_string(),
            })?;
        let vector = vector.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector insert input".to_string(),
        })?;
        let index = index.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector insert index".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector insert value".to_string(),
        })?;

        Ok(Instruction::new(
            Op::VectorInsert,
            destination.id(),
            vector.id(),
            index.id(),
            value.id(),
        ))
    }

    /// Lower one vector shuffle.
    pub(super) fn lower_vector_shuffle(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        left: mir::ValueReference,
        right: mir::ValueReference,
        mask: &[u32],
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector shuffle destination".to_string(),
            })?;
        let left = left.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector shuffle left".to_string(),
        })?;
        let right = right.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector shuffle right".to_string(),
        })?;

        let mask = pool.u32_range(mask);

        Ok(Instruction::new(
            Op::VectorShuffle,
            destination.id(),
            left.id(),
            right.id(),
            mask.0,
        ))
    }

    /// Lower one vector select.
    pub(super) fn lower_vector_select(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        mask: mir::ValueReference,
        then_value: mir::ValueReference,
        else_value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector select destination".to_string(),
            })?;
        let mask = mask.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector select mask".to_string(),
        })?;
        let then_value = then_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector select then value".to_string(),
            })?;
        let else_value = else_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector select else value".to_string(),
            })?;

        Ok(Instruction::new(
            Op::VectorSelect,
            destination.id(),
            mask.id(),
            then_value.id(),
            else_value.id(),
        ))
    }

    /// Lower one vector reduction.
    pub(super) fn lower_vector_reduce(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        operator: mir::VectorReduceOperator,
        vector: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector reduce destination".to_string(),
            })?;
        let vector = vector.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector reduce input".to_string(),
        })?;

        Ok(Instruction::new(
            Op::VectorReduce,
            destination.id(),
            vector_reduce_operator_operand(operator),
            vector.id(),
            0,
        ))
    }

    /// Lower one vector comparison.
    pub(super) fn lower_vector_compare(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        operator: mir::BinaryOperator,
        left: mir::ValueReference,
        right: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector compare destination".to_string(),
            })?;
        let left = left.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector compare left".to_string(),
        })?;
        let right = right.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector compare right".to_string(),
        })?;

        Ok(Instruction::new(
            Op::VectorCompare,
            destination.id(),
            binary_operator_operand(operator),
            left.id(),
            right.id(),
        ))
    }

    /// Lower one vector conversion.
    pub(super) fn lower_vector_convert(
        &self,
        _pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        mode: mir::VectorConvertMode,
        vector: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "vector convert destination".to_string(),
            })?;
        let vector = vector.value().ok_or_else(|| Error::MissingRepresentation {
            context: "vector convert input".to_string(),
        })?;
        let dest_type = self.value_type_for_value(destination)?;
        let source_type = self.value_type_for_value(vector)?;

        Ok(Instruction::new(
            vector_convert_op(mode),
            destination.id(),
            vector.id(),
            source_type.id,
            dest_type.id,
        ))
    }
}
