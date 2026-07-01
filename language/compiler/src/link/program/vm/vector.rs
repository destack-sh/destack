use destack_mir as mir;

use crate::LinkResult;

use destack_program::ScalarFormat;
use destack_program::vm::{
    ElementBinaryKernel, Instruction, Op, Projection, VectorBinary, VectorConvert, VectorExtract,
    VectorInsert, VectorReduce, VectorSelect, VectorShuffle, VectorSplat,
};

use super::arithmetic::element_binary_kernel;
use super::lower::BlockLowerer;
use super::pool::Pool;

/// Packed element shape with a direct execution opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PackedVector {
    /// Four 32-bit signed integer elements.
    I32x4,
    /// Four 32-bit unsigned integer elements.
    U32x4,
    /// Two 64-bit signed integer elements.
    I64x2,
    /// Two 64-bit unsigned integer elements.
    U64x2,
    /// Four float32 elements.
    F32x4,
    /// Two float64 elements.
    F64x2,
}

impl<'a> BlockLowerer<'a> {
    /// Return frame element projection for one vector type.
    pub(super) fn vector_element_projection(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<(Projection, u32, mir::LocalNodeId<mir::Type>)> {
        // resolve the VM layout for this vector value
        let layout = self.layout_for_type(ty)?;
        let element = layout
            .element()
            .ok_or_else(|| self.type_mismatch("vector type", format!("{ty:?}")))?;
        let element_count = layout
            .element_count()
            .ok_or_else(|| self.invalid_instruction("vector element count"))?
            as u32;

        // describe one frame element for execute
        let access = Projection::indexed(
            self.function.program.type_id(element.ty),
            element_count as u64,
            element.stride,
            element.byte_len(),
            self.function.cell_layout_for_type(element.ty),
        );

        Ok((access, element_count, element.ty))
    }

    /// Lower one vector splat.
    pub(super) fn lower_vector_splat(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        value: mir::Value,
    ) -> LinkResult<Instruction> {
        // resolve SSA operands
        let destination_type = self.value_type_for_value(destination)?;
        let (dest_element, element_count, element_type) =
            self.vector_element_projection(destination_type)?;

        // select a direct packed opcode when possible
        let op = match self.packed_vector(dest_element, element_count, element_type)? {
            Some(PackedVector::I32x4 | PackedVector::U32x4 | PackedVector::F32x4) => {
                Op::PackedSplat32x4
            }
            Some(PackedVector::I64x2 | PackedVector::U64x2 | PackedVector::F64x2) => {
                Op::PackedSplat64x2
            }
            None => Op::VectorSplat,
        };
        if op != Op::VectorSplat {
            return Ok(Instruction::new(
                op,
                self.value_offset(destination)?,
                self.cell_offset(value)?,
                0,
                0,
            ));
        }

        // fall back to the general vector descriptor
        Ok(pool.instruction_with_side(
            Op::VectorSplat,
            VectorSplat {
                dest_offset: self.value_offset(destination)?,
                value_offset: self.cell_offset(value)?,
                dest_element,
                element_count,
            },
        ))
    }

    /// Lower one vector extract.
    pub(super) fn lower_vector_extract(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        vector: mir::Value,
        index: mir::Value,
    ) -> LinkResult<Instruction> {
        // resolve SSA operands
        let vector_type = self.value_type_for_value(vector)?;
        let (vector_element, element_count, _) = self.vector_element_projection(vector_type)?;

        // store the dynamic index as a cell frame offset
        Ok(pool.instruction_with_side(
            Op::VectorExtract,
            VectorExtract {
                dest_offset: self.cell_offset(destination)?,
                vector_offset: self.value_offset(vector)?,
                index_offset: self.cell_offset(index)?,
                vector_element,
                element_count,
            },
        ))
    }

    /// Lower one vector insert.
    pub(super) fn lower_vector_insert(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        vector: mir::Value,
        index: mir::Value,
        value: mir::Value,
    ) -> LinkResult<Instruction> {
        // resolve SSA operands

        let destination_type = self.value_type_for_value(destination)?;
        let vector_type = self.value_type_for_value(vector)?;
        let (dest_element, element_count, _) = self.vector_element_projection(destination_type)?;
        let (vector_element, _, _) = self.vector_element_projection(vector_type)?;

        // store the dynamic index and inserted scalar as cell frame offsets
        Ok(pool.instruction_with_side(
            Op::VectorInsert,
            VectorInsert {
                dest_offset: self.value_offset(destination)?,
                vector_offset: self.value_offset(vector)?,
                index_offset: self.cell_offset(index)?,
                value_offset: self.cell_offset(value)?,
                dest_element,
                vector_element,
                element_count,
            },
        ))
    }

    /// Lower one vector shuffle.
    pub(super) fn lower_vector_shuffle(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        left: mir::Value,
        right: mir::Value,
        mask: mir::IndexSlice,
    ) -> LinkResult<Instruction> {
        // resolve SSA operands

        let destination_type = self.value_type_for_value(destination)?;
        let left_type = self.value_type_for_value(left)?;
        let right_type = self.value_type_for_value(right)?;
        let (dest_element, _, _) = self.vector_element_projection(destination_type)?;
        let (left_element, left_count, _) = self.vector_element_projection(left_type)?;
        let (right_element, right_count, _) = self.vector_element_projection(right_type)?;
        let mask = pool.u32_range(self.function.indices(mask));

        // pool the shuffle mask because it is variable length
        Ok(pool.instruction_with_side(
            Op::VectorShuffle,
            VectorShuffle {
                dest_offset: self.value_offset(destination)?,
                left_offset: self.value_offset(left)?,
                right_offset: self.value_offset(right)?,
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
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        mask: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    ) -> LinkResult<Instruction> {
        // resolve SSA operands

        let mask_type = self.value_type_for_value(mask)?;
        let then_type = self.value_type_for_value(then_value)?;
        let else_type = self.value_type_for_value(else_value)?;
        let destination_type = self.value_type_for_value(destination)?;
        let (dest_element, _, _) = self.vector_element_projection(destination_type)?;
        let (mask_element, mask_count, _) = self.vector_element_projection(mask_type)?;
        let (then_element, then_count, _) = self.vector_element_projection(then_type)?;
        let (else_element, else_count, _) = self.vector_element_projection(else_type)?;

        // require identical vector widths
        if mask_count != then_count || mask_count != else_count {
            return Err(self.invalid_instruction("vector select width"));
        }

        // byte boolean masks are not packed machine masks
        Ok(pool.instruction_with_side(
            Op::VectorSelect,
            VectorSelect {
                dest_offset: self.value_offset(destination)?,
                mask_offset: self.value_offset(mask)?,
                then_offset: self.value_offset(then_value)?,
                else_offset: self.value_offset(else_value)?,
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
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        operator: mir::VectorReduceOperator,
        vector: mir::Value,
    ) -> LinkResult<Instruction> {
        // resolve SSA operands

        let vector_type = self.value_type_for_value(vector)?;
        let (vector_element, element_count, element) =
            self.vector_element_projection(vector_type)?;

        // reductions keep the operator in the side record
        Ok(pool.instruction_with_side(
            Op::VectorReduce,
            VectorReduce {
                dest_offset: self.cell_offset(destination)?,
                vector_offset: self.value_offset(vector)?,
                kernel: operator,
                vector_element,
                element_layout: self
                    .function
                    .require_scalar_format(element, "scalar vector element")?,
                element_count,
            },
        ))
    }

    /// Lower one vector comparison.
    pub(super) fn lower_vector_compare(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> LinkResult<Instruction> {
        // resolve SSA operands

        let left_type = self.value_type_for_value(left)?;
        let right_type = self.value_type_for_value(right)?;
        let destination_type = self.value_type_for_value(destination)?;
        let (dest_element, element_count, _) = self.vector_element_projection(destination_type)?;
        let (left_element, left_count, element) = self.vector_element_projection(left_type)?;
        let (right_element, right_count, _) = self.vector_element_projection(right_type)?;

        // require identical vector widths
        if element_count != left_count || element_count != right_count {
            return Err(self.invalid_instruction("vector compare width"));
        }

        // comparisons write boolean elements
        let element_layout = self
            .function
            .operand_for_type(element)
            .ok_or_else(|| self.invalid_instruction("vector compare element"))?;
        let kernel = element_binary_kernel(operator, element_layout)
            .filter(is_element_compare_kernel)
            .ok_or_else(|| self.invalid_instruction("vector compare operator"))?;

        // compare writes byte booleans, not packed machine masks
        Ok(pool.instruction_with_side(
            Op::VectorBinary,
            VectorBinary {
                dest_offset: self.value_offset(destination)?,
                left_offset: self.value_offset(left)?,
                right_offset: self.value_offset(right)?,
                dest_element,
                left_element,
                right_element,
                kernel,
                element_layout: self
                    .function
                    .require_scalar_format(element, "scalar vector element")?,
                element_count,
            },
        ))
    }

    /// Lower one vector conversion.
    pub(super) fn lower_vector_convert(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        mode: mir::VectorConvertMode,
        vector: mir::Value,
    ) -> LinkResult<Instruction> {
        // resolve SSA operands
        let dest_type = self.value_type_for_value(destination)?;
        let source_type = self.value_type_for_value(vector)?;
        let (dest_element, dest_count, dest_element_type) =
            self.vector_element_projection(dest_type)?;
        let (source_element, source_count, source_element_type) =
            self.vector_element_projection(source_type)?;

        // require identical vector widths
        if source_count != dest_count {
            return Err(self.invalid_instruction("vector convert width"));
        }

        // conversions keep the mode in the side record
        Ok(pool.instruction_with_side(
            Op::VectorConvert,
            VectorConvert {
                dest_offset: self.value_offset(destination)?,
                vector_offset: self.value_offset(vector)?,
                mode,
                dest_element,
                source_element,
                dest_layout: self
                    .function
                    .require_scalar_format(dest_element_type, "scalar vector element")?,
                source_layout: self
                    .function
                    .require_scalar_format(source_element_type, "scalar vector element")?,
                element_count: dest_count,
            },
        ))
    }

    /// Return the packed shape for one register-sized vector.
    pub(super) fn packed_vector(
        &self,
        element: Projection,
        element_count: u32,
        element_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<Option<PackedVector>> {
        // only tightly packed vectors can use direct packed opcodes
        let layout = self
            .function
            .require_scalar_format(element_type, "scalar vector element")?;
        if element.byte_stride() != element.byte_len() {
            return Ok(None);
        }

        // map scalar layout and element count to one machine vector shape
        Ok(match (layout, element_count) {
            (
                ScalarFormat::Int {
                    width: 32,
                    is_signed: 1,
                },
                4,
            ) => Some(PackedVector::I32x4),
            (
                ScalarFormat::Int {
                    width: 32,
                    is_signed: 0,
                },
                4,
            ) => Some(PackedVector::U32x4),
            (
                ScalarFormat::Int {
                    width: 64,
                    is_signed: 1,
                },
                2,
            ) => Some(PackedVector::I64x2),
            (
                ScalarFormat::Int {
                    width: 64,
                    is_signed: 0,
                },
                2,
            ) => Some(PackedVector::U64x2),
            (
                ScalarFormat::Float {
                    format: mir::FloatType::Float32,
                },
                4,
            ) => Some(PackedVector::F32x4),
            (
                ScalarFormat::Float {
                    format: mir::FloatType::Float64,
                },
                2,
            ) => Some(PackedVector::F64x2),
            _ => None,
        })
    }
}

/// Return whether one element binary kernel produces a boolean vector.
fn is_element_compare_kernel(kernel: &ElementBinaryKernel) -> bool {
    use ElementBinaryKernel::*;

    matches!(
        kernel,
        EqInt
            | EqBool
            | NeInt
            | NeBool
            | LtInt
            | LtUint
            | LeInt
            | LeUint
            | GtInt
            | GtUint
            | GeInt
            | GeUint
            | EqF32
            | EqF64
            | NeF32
            | NeF64
            | LtF32
            | LtF64
            | LeF32
            | LeF64
            | GtF32
            | GtF64
            | GeF32
            | GeF64
            | EqFloat
            | NeFloat
            | LtFloat
            | LeFloat
            | GtFloat
            | GeFloat
    )
}
