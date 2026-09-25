use tspp_fir::format::{Format, FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{BytecodeFormatContext, BytecodeFormatter, TypeId, ValueTag, ValueType};

use super::instruction::InstructionFormatter;

impl InstructionFormatter<'_, '_, '_> {
    /// Write one trailing value representation.
    pub(super) fn write_representation(&mut self, ty: ValueType) -> FormatResult<()> {
        write!(self.formatter, [token(":"), space(), &ty])
    }

    /// Write one trailing conversion representation.
    pub(super) fn write_conversion(
        &mut self,
        source: ValueType,
        target: ValueType,
    ) -> FormatResult<()> {
        write!(
            self.formatter,
            [
                token(":"),
                space(),
                &source,
                space(),
                token("->"),
                space(),
                &target
            ]
        )
    }
}

impl<'a> BytecodeFormatContext<'a> {
    /// Return one canonical value type text.
    pub(super) fn value_type_text(&self, ty: ValueType) -> FormatResult<String> {
        match ty.tag() {
            ValueTag::SCALAR => ty
                .scalar_type()
                .map(|scalar| scalar.name().to_string())
                .ok_or(FormatError::SyntaxError {
                    message: "bytecode object contains an invalid scalar type",
                }),
            ValueTag::INT128 => Ok("int128".to_string()),
            ValueTag::UINT128 => Ok("uint128".to_string()),
            ValueTag::TYPE_ID => Ok("typeId".to_string()),
            ValueTag::POINTER => Ok("pointer".to_string()),
            ValueTag::REFERENCE | ValueTag::UNINIT_REFERENCE => self.reference_type_text(ty),
            ValueTag::FUNCTION_POINTER | ValueTag::FUNCTION => self.function_value_text(ty),
            ValueTag::SLICE | ValueTag::UNINIT_SLICE => self.slice_type_text(ty),
            ValueTag::DYNAMIC => {
                let constraint = ty.constraint().ok_or(FormatError::SyntaxError {
                    message: "dynamic value has no constraint type",
                })?;
                let name = self.type_text(constraint);
                let reference = ty.dynamic_reference().ok_or(FormatError::SyntaxError {
                    message: "dynamic value has no payload reference",
                })?;
                let kind = reference.kind().name().ok_or(FormatError::SyntaxError {
                    message: "dynamic value has invalid payload ownership",
                })?;
                let storage = reference.storage().name().ok_or(FormatError::SyntaxError {
                    message: "dynamic value has invalid payload storage",
                })?;

                Ok(format!("dynamic<{name}, {kind}, {storage}>"))
            }
            ValueTag::VECTOR => ty
                .vector_type()
                .map(|vector| format!("vector<{}, {}>", vector.scalar.name(), vector.lane_count))
                .ok_or(FormatError::SyntaxError {
                    message: "bytecode object contains an invalid vector type",
                }),
            ValueTag::INDEXED => {
                let ty = ty.indexed_type().ok_or(FormatError::SyntaxError {
                    message: "indexed value has no runtime type",
                })?;

                Ok(self.type_text(ty))
            }
            _ => Err(FormatError::SyntaxError {
                message: "bytecode object contains an invalid value type",
            }),
        }
    }

    /// Return one canonical object-local type id.
    pub(super) fn type_text(&self, ty: TypeId) -> String {
        format!("t{}", ty.0)
    }

    /// Return one reference value type text.
    fn reference_type_text(&self, ty: ValueType) -> FormatResult<String> {
        let reference = ty.reference_type().ok_or(FormatError::SyntaxError {
            message: "reference value has invalid qualifiers",
        })?;
        let kind = reference.kind().name().ok_or(FormatError::SyntaxError {
            message: "reference value has invalid ownership",
        })?;
        let storage = reference.storage().name().ok_or(FormatError::SyntaxError {
            message: "reference value has invalid storage",
        })?;
        let value = format!("ref<{kind}, {storage}>");

        // wrap references whose storage is not initialized yet
        if ty.tag() == ValueTag::UNINIT_REFERENCE {
            Ok(format!("uninit<{value}>"))
        } else {
            Ok(value)
        }
    }

    /// Return one callable value type text.
    fn function_value_text(&self, ty: ValueType) -> FormatResult<String> {
        if ty.tag() == ValueTag::FUNCTION_POINTER {
            Ok("fn".to_string())
        } else {
            let reference = ty.function_reference().ok_or(FormatError::SyntaxError {
                message: "function value has no environment reference",
            })?;
            let kind = reference.kind().name().ok_or(FormatError::SyntaxError {
                message: "function value has invalid environment ownership",
            })?;
            let storage = reference.storage().name().ok_or(FormatError::SyntaxError {
                message: "function value has invalid environment storage",
            })?;

            Ok(format!("function<{kind}, {storage}>"))
        }
    }

    /// Return one slice value type text.
    fn slice_type_text(&self, ty: ValueType) -> FormatResult<String> {
        let element = ty.slice_element().ok_or(FormatError::SyntaxError {
            message: "slice value has no element type",
        })?;
        let reference = ty.slice_reference().ok_or(FormatError::SyntaxError {
            message: "slice value has invalid reference qualifiers",
        })?;
        let element = self.type_text(element);
        let kind = reference.kind().name().ok_or(FormatError::SyntaxError {
            message: "slice value has invalid ownership",
        })?;
        let storage = reference.storage().name().ok_or(FormatError::SyntaxError {
            message: "slice value has invalid storage",
        })?;
        let value = format!("slice<{element}, {kind}, {storage}>");

        // wrap slices whose storage is not initialized yet
        if ty.tag() == ValueTag::UNINIT_SLICE {
            Ok(format!("uninit<{value}>"))
        } else {
            Ok(value)
        }
    }
}

impl<'a> Format<'a, BytecodeFormatContext<'a>> for ValueType {
    /// Format this logical value type.
    fn format(&self, formatter: &mut BytecodeFormatter<'a, '_>) -> FormatResult<()> {
        let text = formatter.context().value_type_text(*self)?;

        write!(formatter, [copied_text(&text)])
    }
}
