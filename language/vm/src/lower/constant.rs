use destack_mir as mir;

use destack_heap::{HeapReference, SharedHeapReference};

use crate::program::{ConstValue, Instruction, Op};
use crate::{Cell, Error, ReferenceSpace, Result};

use super::frame::{cell_offset, value_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;
use super::value::reference_meta_for_type;

impl<'a> BlockLowerer<'a> {
    /// Lower one constant instruction.
    pub(super) fn lower_const(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        value: &mir::Constant,
    ) -> Result<Instruction> {
        // resolve the destination frame layout
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("const destination"))?;
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;

        // inline cell constants directly in the instruction
        if layout.is_cell() {
            let value = if matches!(value, mir::Constant::Null) {
                self.null_cell(destination_type)
            } else {
                Cell::from(value)
            };
            let bits = value.bits();

            return Ok(Instruction::new(
                Op::LoadConstCell,
                cell_offset(self, destination)?,
                bits as u32,
                (bits >> 32) as u32,
                0,
            ));
        }

        // pool frame-backed constants
        let value = ConstValue::Bytes(constant_bytes(value, layout.byte_len)?);
        let value = pool.constant(value);

        Ok(Instruction::new(
            Op::LoadConstBytes,
            value_offset(self, destination)?,
            value.0,
            0,
            0,
        ))
    }

    /// Return the null cell for one reference-like type.
    fn null_cell(&self, value_type: mir::LocalNodeId<mir::Type>) -> Cell {
        let reference = reference_meta_for_type(self.tree, value_type);

        match reference.kind() {
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Unique)
                if matches!(reference.space(), ReferenceSpace::Shared) =>
            {
                Cell::shared_heap_reference(SharedHeapReference::NULL)
            }
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Unique) => {
                Cell::heap_reference(HeapReference::NULL)
            }
            _ => Cell::address(0),
        }
    }
}

/// Encode a constant into its frame bytes.
fn constant_bytes(value: &mir::Constant, byte_len: usize) -> Result<Box<[u8]>> {
    let mut bytes = vec![0; byte_len];

    // write the constant into the destination layout
    match value {
        mir::Constant::Int {
            value,
            is_signed: true,
            ..
        } => {
            let source = value.to_le_bytes();
            let copied = byte_len.min(source.len());
            bytes[..copied].copy_from_slice(&source[..copied]);

            if *value < 0 && byte_len > source.len() {
                bytes[source.len()..].fill(0xff);
            }
        }
        mir::Constant::Int { value, .. } => {
            let source = (*value as u128).to_le_bytes();
            let copied = byte_len.min(source.len());
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        mir::Constant::UInt { value, .. } => {
            let source = value.to_le_bytes();
            let copied = byte_len.min(source.len());
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        mir::Constant::Float { bits: value, .. } => {
            let source = value.to_le_bytes();
            let copied = byte_len.min(source.len());
            bytes[..copied].copy_from_slice(&source[..copied]);
        }
        _ => {
            return Err(Error::type_mismatch(
                "frame-backed constant",
                format!("{value:?}"),
            ));
        }
    }

    Ok(bytes.into_boxed_slice())
}
