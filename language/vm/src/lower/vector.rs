use destack_mir as mir;

use crate::program::{
    ElementBinaryKernel, Instruction, Op, Projection, ScalarLayout, VectorBinary, VectorConvert,
    VectorExtract, VectorInsert, VectorReduce, VectorSelect, VectorShuffle, VectorSplat,
    scalar_layout_from_type, value_layout_from_type, word_layout_from_type,
};
use crate::{Error, Result};

use super::arithmetic::element_binary_kernel;
use super::frame::{value_offset, word_offset};
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
    ) -> Result<(Projection, u32, mir::LocalNodeId<mir::Type>)> {
        // resolve the VM layout for this vector value
        let layout = self.layout_for_type(ty)?;
        let element = layout
            .element()
            .ok_or(Error::type_mismatch("vector type", format!("{ty:?}")))?;
        let element_count = layout.element_count().ok_or(Error::invalid_instruction())? as u32;

        // describe one frame element for execute
        let access = Projection::indexed(
            element.ty,
            element_count as u64,
            element.stride,
            element.byte_len,
            word_layout_from_type(self.tree, element.ty),
        );

        Ok((access, element_count, element.ty))
    }

    /// Lower one vector splat.
    pub(super) fn lower_vector_splat(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("vector splat destination"))?;
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("vector splat value"))?;
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
                value_offset(self, destination)?,
                word_offset(self, value)?,
                0,
                0,
            ));
        }

        // fall back to the general vector descriptor
        Ok(pool.instruction_with_side(
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
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        vector: mir::ValueReference,
        index: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("vector extract destination"))?;
        let vector = vector
            .value()
            .ok_or_else(|| Error::invalid_program("vector extract input"))?;
        let index = index
            .value()
            .ok_or_else(|| Error::invalid_program("vector extract index"))?;
        let vector_type = self.value_type_for_value(vector)?;
        let (vector_element, element_count, _) = self.vector_element_projection(vector_type)?;

        // store the dynamic index as a word frame offset
        Ok(pool.instruction_with_side(
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
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        vector: mir::ValueReference,
        index: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("vector insert destination"))?;
        let vector = vector
            .value()
            .ok_or_else(|| Error::invalid_program("vector insert input"))?;
        let index = index
            .value()
            .ok_or_else(|| Error::invalid_program("vector insert index"))?;
        let value = value
            .value()
            .ok_or_else(|| Error::invalid_program("vector insert value"))?;

        let destination_type = self.value_type_for_value(destination)?;
        let vector_type = self.value_type_for_value(vector)?;
        let (dest_element, element_count, _) = self.vector_element_projection(destination_type)?;
        let (vector_element, _, _) = self.vector_element_projection(vector_type)?;

        // store the dynamic index and inserted scalar as word frame offsets
        Ok(pool.instruction_with_side(
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
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        left: mir::ValueReference,
        right: mir::ValueReference,
        mask: &[u32],
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("vector shuffle destination"))?;
        let left = left
            .value()
            .ok_or_else(|| Error::invalid_program("vector shuffle left"))?;
        let right = right
            .value()
            .ok_or_else(|| Error::invalid_program("vector shuffle right"))?;

        let destination_type = self.value_type_for_value(destination)?;
        let left_type = self.value_type_for_value(left)?;
        let right_type = self.value_type_for_value(right)?;
        let (dest_element, _, _) = self.vector_element_projection(destination_type)?;
        let (left_element, left_count, _) = self.vector_element_projection(left_type)?;
        let (right_element, right_count, _) = self.vector_element_projection(right_type)?;
        let mask = pool.u32_range(mask);

        // pool the shuffle mask because it is variable length
        Ok(pool.instruction_with_side(
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
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        mask: mir::ValueReference,
        then_value: mir::ValueReference,
        else_value: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("vector select destination"))?;
        let mask = mask
            .value()
            .ok_or_else(|| Error::invalid_program("vector select mask"))?;
        let then_value = then_value
            .value()
            .ok_or_else(|| Error::invalid_program("vector select then value"))?;
        let else_value = else_value
            .value()
            .ok_or_else(|| Error::invalid_program("vector select else value"))?;

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
            return Err(Error::invalid_instruction());
        }

        // byte boolean masks are not packed machine masks
        Ok(pool.instruction_with_side(
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
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        operator: mir::VectorReduceOperator,
        vector: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("vector reduce destination"))?;
        let vector = vector
            .value()
            .ok_or_else(|| Error::invalid_program("vector reduce input"))?;

        let vector_type = self.value_type_for_value(vector)?;
        let (vector_element, element_count, element) =
            self.vector_element_projection(vector_type)?;

        // reductions keep the operator in the side record
        Ok(pool.instruction_with_side(
            Op::VectorReduce,
            VectorReduce {
                dest_offset: word_offset(self, destination)?,
                vector_offset: value_offset(self, vector)?,
                kernel: operator,
                vector_element,
                element_layout: vector_scalar_layout(self.tree, element)?,
                element_count,
            },
        ))
    }

    /// Lower one vector comparison.
    pub(super) fn lower_vector_compare(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        operator: mir::BinaryOperator,
        left: mir::ValueReference,
        right: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("vector compare destination"))?;
        let left = left
            .value()
            .ok_or_else(|| Error::invalid_program("vector compare left"))?;
        let right = right
            .value()
            .ok_or_else(|| Error::invalid_program("vector compare right"))?;

        let left_type = self.value_type_for_value(left)?;
        let right_type = self.value_type_for_value(right)?;
        let destination_type = self.value_type_for_value(destination)?;
        let (dest_element, element_count, _) = self.vector_element_projection(destination_type)?;
        let (left_element, left_count, element) = self.vector_element_projection(left_type)?;
        let (right_element, right_count, _) = self.vector_element_projection(right_type)?;

        // require identical vector widths
        if element_count != left_count || element_count != right_count {
            return Err(Error::invalid_instruction());
        }

        // comparisons write boolean elements
        let element_layout = value_layout_from_type(self.tree, element);
        let kernel = element_binary_kernel(operator, element_layout)
            .filter(is_element_compare_kernel)
            .ok_or(Error::invalid_instruction())?;

        // compare writes byte booleans, not packed machine masks
        Ok(pool.instruction_with_side(
            Op::VectorBinary,
            VectorBinary {
                dest_offset: value_offset(self, destination)?,
                left_offset: value_offset(self, left)?,
                right_offset: value_offset(self, right)?,
                dest_element,
                left_element,
                right_element,
                kernel,
                element_layout: vector_scalar_layout(self.tree, element)?,
                element_count,
            },
        ))
    }

    /// Lower one vector conversion.
    pub(super) fn lower_vector_convert(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        mode: mir::VectorConvertMode,
        vector: mir::ValueReference,
    ) -> Result<Instruction> {
        // resolve SSA operands
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("vector convert destination"))?;
        let vector = vector
            .value()
            .ok_or_else(|| Error::invalid_program("vector convert input"))?;
        let dest_type = self.value_type_for_value(destination)?;
        let source_type = self.value_type_for_value(vector)?;
        let (dest_element, dest_count, dest_element_type) =
            self.vector_element_projection(dest_type)?;
        let (source_element, source_count, source_element_type) =
            self.vector_element_projection(source_type)?;

        // require identical vector widths
        if source_count != dest_count {
            return Err(Error::invalid_instruction());
        }

        // conversions keep the mode in the side record
        Ok(pool.instruction_with_side(
            Op::VectorConvert,
            VectorConvert {
                dest_offset: value_offset(self, destination)?,
                vector_offset: value_offset(self, vector)?,
                mode,
                dest_element,
                source_element,
                dest_layout: vector_scalar_layout(self.tree, dest_element_type)?,
                source_layout: vector_scalar_layout(self.tree, source_element_type)?,
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
    ) -> Result<Option<PackedVector>> {
        // only tightly packed vectors can use direct packed opcodes
        let layout = vector_scalar_layout(self.tree, element_type)?;
        if element.byte_stride != element.byte_len {
            return Ok(None);
        }

        // map scalar layout and element count to one machine vector shape
        Ok(match (layout, element_count) {
            (
                ScalarLayout::Int {
                    width: 32,
                    is_signed: true,
                },
                4,
            ) => Some(PackedVector::I32x4),
            (
                ScalarLayout::Int {
                    width: 32,
                    is_signed: false,
                },
                4,
            ) => Some(PackedVector::U32x4),
            (
                ScalarLayout::Int {
                    width: 64,
                    is_signed: true,
                },
                2,
            ) => Some(PackedVector::I64x2),
            (
                ScalarLayout::Int {
                    width: 64,
                    is_signed: false,
                },
                2,
            ) => Some(PackedVector::U64x2),
            (
                ScalarLayout::Float {
                    format: mir::FloatType::Float32,
                },
                4,
            ) => Some(PackedVector::F32x4),
            (
                ScalarLayout::Float {
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
    use ElementBinaryKernel as Kernel;

    matches!(
        kernel,
        Kernel::EqInt
            | Kernel::EqBool
            | Kernel::NeInt
            | Kernel::NeBool
            | Kernel::LtInt
            | Kernel::LtUint
            | Kernel::LeInt
            | Kernel::LeUint
            | Kernel::GtInt
            | Kernel::GtUint
            | Kernel::GeInt
            | Kernel::GeUint
            | Kernel::EqF32
            | Kernel::EqF64
            | Kernel::NeF32
            | Kernel::NeF64
            | Kernel::LtF32
            | Kernel::LtF64
            | Kernel::LeF32
            | Kernel::LeF64
            | Kernel::GtF32
            | Kernel::GtF64
            | Kernel::GeF32
            | Kernel::GeF64
            | Kernel::EqFloat
            | Kernel::NeFloat
            | Kernel::LtFloat
            | Kernel::LeFloat
            | Kernel::GtFloat
            | Kernel::GeFloat
    )
}

/// Return the scalar layout for one vector element.
pub(super) fn vector_scalar_layout(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ScalarLayout> {
    scalar_layout_from_type(tree, ty)
        .ok_or_else(|| Error::type_mismatch("scalar vector element", format!("{ty:?}")))
}
