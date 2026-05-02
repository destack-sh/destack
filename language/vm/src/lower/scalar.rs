use destack_mir as mir;

use crate::program::{
    Binary, BinaryElementwise, BinaryInteger, BinaryWord, CastWideInt, CastWideIntToWord, CastWord,
    CastWordToWideInt, ConstValue, Instruction, LoadConst, Opcode, SelectFrame, SelectWord, Unary,
    UnaryElementwise, UnaryInteger, UnaryWord, ValueLayout, value_layout_from_type,
};
use crate::{Error, ReferenceAddressSpace, Result, Word};

use super::lower::BlockLowerer;
use super::opcode::{
    select_binary_opcode, select_integer_opcode, select_integer_unary_opcode, select_unary_opcode,
};
use super::pool::Pool;
use super::value::reference_meta_for_type;

/// Return one word value's frame byte offset.
fn word_offset(lowerer: &BlockLowerer<'_>, value: mir::Value) -> Result<u32> {
    let region = lowerer
        .frame_layout
        .value(value.0)
        .ok_or(Error::InvalidInstruction)?;
    if !region.is_word {
        return Err(Error::TypeMismatch {
            expected: "word value".to_string(),
            actual: format!("frame-backed value: {value:?}"),
        });
    }

    Ok(region.offset)
}

/// Build word binary operands from frame offsets.
fn binary_word(
    lowerer: &BlockLowerer<'_>,
    dest: mir::Value,
    left: mir::Value,
    right: mir::Value,
) -> Result<BinaryWord> {
    Ok(BinaryWord {
        dest: word_offset(lowerer, dest)?,
        left: word_offset(lowerer, left)?,
        right: word_offset(lowerer, right)?,
    })
}

/// Build word unary operands from frame offsets.
fn unary_word(lowerer: &BlockLowerer<'_>, dest: mir::Value, arg: mir::Value) -> Result<UnaryWord> {
    Ok(UnaryWord {
        dest: word_offset(lowerer, dest)?,
        arg: word_offset(lowerer, arg)?,
    })
}

/// Return one integer value layout.
fn integer_layout(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> Result<(u16, bool)> {
    match value_layout_from_type(tree, ty) {
        ValueLayout::Int { width, signed } => Ok((width, signed)),
        actual => Err(Error::TypeMismatch {
            expected: "integer cast operand".to_string(),
            actual: format!("{actual:?}"),
        }),
    }
}

/// Encode a constant into its frame bytes.
fn constant_bytes(value: &mir::Constant, byte_len: usize) -> Result<Box<[u8]>> {
    let mut bytes = vec![0; byte_len];

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

impl<'a> BlockLowerer<'a> {
    /// Lower one constant instruction.
    pub(super) fn lower_const(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        value: &mir::Constant,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "const destination".to_string(),
            })?;
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;

        let value = if matches!(value, mir::Constant::Null) {
            ConstValue::Word(self.null_word(destination_type))
        } else if layout.is_word() {
            ConstValue::Word(Word::from(value))
        } else {
            ConstValue::Bytes(constant_bytes(value, layout.byte_len)?)
        };
        let value = pool.constant(value);

        Ok(Instruction::new(
            Opcode::LoadConst,
            LoadConst {
                dest: destination,
                value,
            },
        ))
    }

    /// Lower one binary instruction.
    pub(super) fn lower_binary(
        &self,
        destination: mir::ValueReference,
        operator: mir::BinaryOperator,
        left: mir::ValueReference,
        right: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "binary destination".to_string(),
            })?;
        let left = left.value().ok_or_else(|| Error::MissingRepresentation {
            context: "binary left operand".to_string(),
        })?;
        let right = right.value().ok_or_else(|| Error::MissingRepresentation {
            context: "binary right operand".to_string(),
        })?;
        let left_type = self.value_type_for_value(left)?;

        if matches!(
            self.tree.get(left_type),
            mir::Type::Vector { .. } | mir::Type::Tensor { .. }
        ) {
            let result_type = self.value_type_for_value(destination)?;
            return Ok(Instruction::new(
                Opcode::BinaryElementwise,
                BinaryElementwise {
                    dest: destination,
                    op: operator,
                    left,
                    right,
                    result_type,
                },
            ));
        }

        let layout = self.value_layout_map().get(left);
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(opcode) = select_integer_opcode(operator, signed, width)
        {
            return Ok(Instruction::new(
                opcode,
                BinaryInteger {
                    dest: word_offset(self, destination)?,
                    left: word_offset(self, left)?,
                    right: word_offset(self, right)?,
                    width: width as u8,
                    is_signed: signed,
                },
            ));
        }

        let layout = layout.or_else(|| Some(value_layout_from_type(self.tree, left_type)));
        let opcode = select_binary_opcode(layout, operator);
        if opcode != Opcode::BinaryWideInt && opcode != Opcode::BinaryWideUint {
            return Ok(Instruction::new(
                opcode,
                binary_word(self, destination, left, right)?,
            ));
        }

        Ok(Instruction::new(
            opcode,
            Binary {
                dest: destination,
                op: operator,
                left,
                right,
            },
        ))
    }

    /// Lower one unary instruction.
    pub(super) fn lower_unary(
        &self,
        destination: mir::ValueReference,
        operator: mir::UnaryOperator,
        argument: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "unary destination".to_string(),
            })?;
        let argument = argument
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "unary argument".to_string(),
            })?;
        let argument_type = self.value_type_for_value(argument)?;

        if matches!(
            self.tree.get(argument_type),
            mir::Type::Vector { .. } | mir::Type::Tensor { .. }
        ) {
            let result_type = self.value_type_for_value(destination)?;
            return Ok(Instruction::new(
                Opcode::UnaryElementwise,
                UnaryElementwise {
                    dest: destination,
                    op: operator,
                    arg: argument,
                    result_type,
                },
            ));
        }

        let layout = self.value_layout_map().get(argument);
        if let Some(ValueLayout::Int { width, signed }) = layout
            && let Some(opcode) = select_integer_unary_opcode(operator, signed, width)
        {
            return Ok(Instruction::new(
                opcode,
                UnaryInteger {
                    dest: word_offset(self, destination)?,
                    arg: word_offset(self, argument)?,
                    width: width as u8,
                    is_signed: signed,
                },
            ));
        }

        let opcode = select_unary_opcode(self.value_layout_map(), argument, operator);
        if opcode != Opcode::UnaryWideInt {
            return Ok(Instruction::new(
                opcode,
                unary_word(self, destination, argument)?,
            ));
        }

        Ok(Instruction::new(
            opcode,
            Unary {
                dest: destination,
                op: operator,
                arg: argument,
            },
        ))
    }

    /// Lower one cast instruction.
    pub(super) fn lower_cast(
        &self,
        destination: mir::ValueReference,
        operator: mir::CastOperator,
        argument: mir::ValueReference,
        to_type: mir::TypeReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "cast destination".to_string(),
            })?;
        let argument = argument
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "cast argument".to_string(),
            })?;
        let to_type = to_type.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "cast destination type".to_string(),
        })?;
        let destination_type = self.value_type_for_value(destination)?;
        let argument_type = self.value_type_for_value(argument)?;
        let destination_is_word = self.layout_for_type(destination_type)?.is_word();
        let argument_is_word = self.layout_for_type(argument_type)?.is_word();

        if destination_is_word && argument_is_word {
            return Ok(Instruction::new(
                Opcode::CastWord,
                CastWord {
                    dest: word_offset(self, destination)?,
                    op: operator,
                    arg: word_offset(self, argument)?,
                    to_type: to_type.id,
                },
            ));
        }

        let (source_width, source_signed) = integer_layout(self.tree, argument_type)?;
        let (dest_width, dest_signed) = integer_layout(self.tree, to_type)?;
        let source_signed = matches!(operator, mir::CastOperator::SignExtend) || source_signed;

        if argument_is_word {
            return Ok(Instruction::new(
                Opcode::CastWordToWideInt,
                CastWordToWideInt {
                    dest: destination,
                    op: operator,
                    arg: word_offset(self, argument)?,
                    source_width,
                    source_signed,
                    dest_width,
                },
            ));
        }

        if destination_is_word {
            return Ok(Instruction::new(
                Opcode::CastWideIntToWord,
                CastWideIntToWord {
                    dest: word_offset(self, destination)?,
                    op: operator,
                    arg: argument,
                    source_width,
                    source_signed,
                    dest_width,
                    dest_signed,
                },
            ));
        }

        Ok(Instruction::new(
            Opcode::CastWideInt,
            CastWideInt {
                dest: destination,
                op: operator,
                arg: argument,
                source_width,
                source_signed,
                dest_width,
            },
        ))
    }

    /// Lower one select instruction.
    pub(super) fn lower_select(
        &self,
        destination: mir::ValueReference,
        condition: mir::ValueReference,
        then_value: mir::ValueReference,
        else_value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select destination".to_string(),
            })?;
        let condition = condition
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select condition".to_string(),
            })?;
        let then_value = then_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select then value".to_string(),
            })?;
        let else_value = else_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select else value".to_string(),
            })?;

        let destination_type = self.value_type_for_value(destination)?;
        if self.layout_for_type(destination_type)?.is_word() {
            return Ok(Instruction::new(
                Opcode::SelectWord,
                SelectWord {
                    dest: word_offset(self, destination)?,
                    condition: word_offset(self, condition)?,
                    then_value: word_offset(self, then_value)?,
                    else_value: word_offset(self, else_value)?,
                },
            ));
        }

        Ok(Instruction::new(
            Opcode::SelectFrame,
            SelectFrame {
                dest: destination,
                condition: word_offset(self, condition)?,
                then_value,
                else_value,
            },
        ))
    }

    /// Return the null word for one reference-like type.
    fn null_word(&self, value_type: mir::LocalNodeId<mir::Type>) -> Word {
        let reference = reference_meta_for_type(self.tree, value_type);
        match reference.kind() {
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned)
                if matches!(reference.address_space(), ReferenceAddressSpace::Shared) =>
            {
                Word::shared_heap_reference(destack_heap::SharedHeapReference::NULL)
            }
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned) => {
                Word::heap_reference(destack_heap::HeapReference::NULL)
            }
            _ if matches!(reference.address_space(), ReferenceAddressSpace::Shared) => {
                Word::shared_raw_pointer(destack_heap::SharedRawPointer::NULL)
            }
            _ => Word::raw_pointer(destack_heap::RawPointer::NULL),
        }
    }
}
