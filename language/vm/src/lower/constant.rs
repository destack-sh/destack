use destack_mir as mir;

use destack_heap::{HeapReference, RawPointer, SharedHeapReference, SharedRawPointer};

use crate::program::{ConstValue, Instruction, Op};
use crate::{Error, ReferenceAddressSpace, Result, Word};

use super::frame::{value_offset, word_offset};
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
            .ok_or_else(|| Error::MissingRepresentation {
                context: "const destination".to_string(),
            })?;
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;

        // inline word constants directly in the instruction
        if layout.is_word() {
            let value = if matches!(value, mir::Constant::Null) {
                self.null_word(destination_type)
            } else {
                Word::from(value)
            };
            let bits = value.bits();

            return Ok(Instruction::new(
                Op::LoadConstWord,
                word_offset(self, destination)?,
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

    /// Return the null word for one reference-like type.
    fn null_word(&self, value_type: mir::LocalNodeId<mir::Type>) -> Word {
        let reference = reference_meta_for_type(self.tree, value_type);

        match reference.kind() {
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned)
                if matches!(reference.address_space(), ReferenceAddressSpace::Shared) =>
            {
                Word::shared_heap_reference(SharedHeapReference::NULL)
            }
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned) => {
                Word::heap_reference(HeapReference::NULL)
            }
            _ if matches!(reference.address_space(), ReferenceAddressSpace::Shared) => {
                Word::shared_raw_pointer(SharedRawPointer::NULL)
            }
            _ => Word::raw_pointer(RawPointer::NULL),
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
            return Err(Error::TypeMismatch {
                expected: "frame-backed constant".to_string(),
                actual: format!("{value:?}"),
            });
        }
    }

    Ok(bytes.into_boxed_slice())
}
