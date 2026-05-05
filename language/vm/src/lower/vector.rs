use destack_mir as mir;

use crate::program::{
    ElementAccess, Instruction, Op, PointerClass, ScalarLayout, VectorBinary, VectorConvert,
    VectorExtract, VectorInsert, VectorReduce, VectorSelect, VectorShuffle, VectorSplat,
    scalar_layout_from_type, value_layout_from_type, word_layout_from_type,
};
use crate::{Error, ReferenceMeta, Result};

use super::arithmetic::vector_binary_op;
use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Return frame element access for one vector type.
    pub(super) fn vector_element_access(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<(ElementAccess, u32, mir::LocalNodeId<mir::Type>)> {
        let layout = self.layout_for_type(ty)?;
        let element = layout.element().ok_or(Error::TypeMismatch {
            expected: "vector type".to_string(),
            actual: format!("{ty:?}"),
        })?;
        let element_count = layout.element_count().ok_or(Error::InvalidInstruction)? as u32;

        let access = ElementAccess {
            pointer_class: PointerClass::Frame,
            reference: ReferenceMeta::NONE,
            value_type: element.ty,
            length: element_count as u64,
            byte_stride: element.stride,
            byte_len: element.byte_len,
            word_layout: word_layout_from_type(self.tree, element.ty),
        };

        Ok((access, element_count, element.ty))
    }

    /// Lower one vector splat.
    pub(super) fn lower_vector_splat(
        &self,
        pool: &mut Pool<'_>,
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
        let destination_type = self.value_type_for_value(destination)?;
        let (dest_element, element_count, _) = self.vector_element_access(destination_type)?;

        Ok(pool.instruction_with_side_record(
            Op::VectorSplat,
            VectorSplat {
                dest_offset: value_offset(self, destination)?,
                value_offset: word_offset(self, value)?,
                dest_element,
                element_count,
            },
        ))
    }

    /// Lower one vector extract.
    pub(super) fn lower_vector_extract(
        &self,
        pool: &mut Pool<'_>,
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
        let vector_type = self.value_type_for_value(vector)?;
        let (vector_element, element_count, _) = self.vector_element_access(vector_type)?;

        Ok(pool.instruction_with_side_record(
            Op::VectorExtract,
            VectorExtract {
                dest_offset: word_offset(self, destination)?,
                vector_offset: value_offset(self, vector)?,
                index_offset: word_offset(self, index)?,
                vector_element,
                element_count,
            },
        ))
    }

    /// Lower one vector insert.
    pub(super) fn lower_vector_insert(
        &self,
        pool: &mut Pool<'_>,
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

        let destination_type = self.value_type_for_value(destination)?;
        let vector_type = self.value_type_for_value(vector)?;
        let (dest_element, element_count, _) = self.vector_element_access(destination_type)?;
        let (vector_element, _, _) = self.vector_element_access(vector_type)?;

        Ok(pool.instruction_with_side_record(
            Op::VectorInsert,
            VectorInsert {
                dest_offset: value_offset(self, destination)?,
                vector_offset: value_offset(self, vector)?,
                index_offset: word_offset(self, index)?,
                value_offset: word_offset(self, value)?,
                dest_element,
                vector_element,
                element_count,
            },
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

        let destination_type = self.value_type_for_value(destination)?;
        let left_type = self.value_type_for_value(left)?;
        let right_type = self.value_type_for_value(right)?;
        let (dest_element, _, _) = self.vector_element_access(destination_type)?;
        let (left_element, left_count, _) = self.vector_element_access(left_type)?;
        let (right_element, right_count, _) = self.vector_element_access(right_type)?;
        let mask = pool.u32_range(mask);

        Ok(pool.instruction_with_side_record(
            Op::VectorShuffle,
            VectorShuffle {
                dest_offset: value_offset(self, destination)?,
                left_offset: value_offset(self, left)?,
                right_offset: value_offset(self, right)?,
                mask,
                dest_element,
                left_element,
                right_element,
                left_count,
                right_count,
            },
        ))
    }

    /// Lower one vector select.
    pub(super) fn lower_vector_select(
        &self,
        pool: &mut Pool<'_>,
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

        let mask_type = self.value_type_for_value(mask)?;
        let then_type = self.value_type_for_value(then_value)?;
        let else_type = self.value_type_for_value(else_value)?;
        let destination_type = self.value_type_for_value(destination)?;
        let (dest_element, _, _) = self.vector_element_access(destination_type)?;
        let (mask_element, mask_count, _) = self.vector_element_access(mask_type)?;
        let (then_element, then_count, _) = self.vector_element_access(then_type)?;
        let (else_element, else_count, _) = self.vector_element_access(else_type)?;
        if mask_count != then_count || mask_count != else_count {
            return Err(Error::InvalidInstruction);
        }

        Ok(pool.instruction_with_side_record(
            Op::VectorSelect,
            VectorSelect {
                dest_offset: value_offset(self, destination)?,
                mask_offset: value_offset(self, mask)?,
                then_offset: value_offset(self, then_value)?,
                else_offset: value_offset(self, else_value)?,
                dest_element,
                mask_element,
                then_element,
                else_element,
                element_count: mask_count,
            },
        ))
    }

    /// Lower one vector reduction.
    pub(super) fn lower_vector_reduce(
        &self,
        pool: &mut Pool<'_>,
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

        let vector_type = self.value_type_for_value(vector)?;
        let (vector_element, element_count, element) = self.vector_element_access(vector_type)?;

        Ok(pool.instruction_with_side_record(
            vector_reduce_op(operator),
            VectorReduce {
                dest_offset: word_offset(self, destination)?,
                vector_offset: value_offset(self, vector)?,
                vector_element,
                element_layout: vector_scalar_layout(self.tree, element)?,
                element_count,
            },
        ))
    }

    /// Lower one vector comparison.
    pub(super) fn lower_vector_compare(
        &self,
        pool: &mut Pool<'_>,
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

        let left_type = self.value_type_for_value(left)?;
        let right_type = self.value_type_for_value(right)?;
        let destination_type = self.value_type_for_value(destination)?;
        let (dest_element, element_count, _) = self.vector_element_access(destination_type)?;
        let (left_element, left_count, element) = self.vector_element_access(left_type)?;
        let (right_element, right_count, _) = self.vector_element_access(right_type)?;
        if element_count != left_count || element_count != right_count {
            return Err(Error::InvalidInstruction);
        }
        let element_layout = value_layout_from_type(self.tree, element);
        let op = vector_binary_op(operator, element_layout)
            .filter(is_vector_compare_op)
            .ok_or(Error::InvalidInstruction)?;

        Ok(pool.instruction_with_side_record(
            op,
            VectorBinary {
                dest_offset: value_offset(self, destination)?,
                left_offset: value_offset(self, left)?,
                right_offset: value_offset(self, right)?,
                dest_element,
                left_element,
                right_element,
                element_layout: vector_scalar_layout(self.tree, element)?,
                element_count,
            },
        ))
    }

    /// Lower one vector conversion.
    pub(super) fn lower_vector_convert(
        &self,
        pool: &mut Pool<'_>,
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
        let (dest_element, dest_count, dest_element_type) =
            self.vector_element_access(dest_type)?;
        let (source_element, source_count, source_element_type) =
            self.vector_element_access(source_type)?;
        if source_count != dest_count {
            return Err(Error::InvalidInstruction);
        }

        Ok(pool.instruction_with_side_record(
            vector_convert_op(mode),
            VectorConvert {
                dest_offset: value_offset(self, destination)?,
                vector_offset: value_offset(self, vector)?,
                dest_element,
                source_element,
                dest_layout: vector_scalar_layout(self.tree, dest_element_type)?,
                source_layout: vector_scalar_layout(self.tree, source_element_type)?,
                element_count: dest_count,
            },
        ))
    }
}

/// Return whether one vector binary opcode produces a boolean vector.
fn is_vector_compare_op(op: &Op) -> bool {
    matches!(
        op,
        Op::VectorEqInt
            | Op::VectorEqBool
            | Op::VectorNeInt
            | Op::VectorNeBool
            | Op::VectorLtInt
            | Op::VectorLtUint
            | Op::VectorLeInt
            | Op::VectorLeUint
            | Op::VectorGtInt
            | Op::VectorGtUint
            | Op::VectorGeInt
            | Op::VectorGeUint
            | Op::VectorEqF32
            | Op::VectorEqF64
            | Op::VectorNeF32
            | Op::VectorNeF64
            | Op::VectorLtF32
            | Op::VectorLtF64
            | Op::VectorLeF32
            | Op::VectorLeF64
            | Op::VectorGtF32
            | Op::VectorGtF64
            | Op::VectorGeF32
            | Op::VectorGeF64
    )
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

/// Return the vector reduction operation for one operator.
fn vector_reduce_op(operator: mir::VectorReduceOperator) -> Op {
    match operator {
        mir::VectorReduceOperator::Add => Op::VectorReduceAdd,
        mir::VectorReduceOperator::Multiply => Op::VectorReduceMultiply,
        mir::VectorReduceOperator::Min => Op::VectorReduceMin,
        mir::VectorReduceOperator::Max => Op::VectorReduceMax,
        mir::VectorReduceOperator::And => Op::VectorReduceAnd,
        mir::VectorReduceOperator::Or => Op::VectorReduceOr,
        mir::VectorReduceOperator::Xor => Op::VectorReduceXor,
    }
}

/// Return the scalar layout for one vector element.
pub(super) fn vector_scalar_layout(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ScalarLayout> {
    scalar_layout_from_type(tree, ty).ok_or_else(|| Error::TypeMismatch {
        expected: "scalar vector element".to_string(),
        actual: format!("{ty:?}"),
    })
}
