use destack_core::StringId;
use destack_fir::format::{Format, FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;

use crate::{BytecodeFormatContext, BytecodeFormatter, TypeId, ValueTag, ValueType};

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
                let name = self.type_name(constraint)?;
                let reference = ty.dynamic_reference().ok_or(FormatError::SyntaxError {
                    message: "dynamic value has no payload reference",
                })?;
                let space = reference.space().name().ok_or(FormatError::SyntaxError {
                    message: "dynamic value has an invalid space",
                })?;

                Ok(format!("dynamic<{name}, space({space})>"))
            }
            ValueTag::TENSOR | ValueTag::TENSOR_VIEW => self.tensor_type_text(ty),
            ValueTag::VECTOR => ty
                .vector_type()
                .map(|vector| format!("vector<{}, {}>", vector.scalar.name(), vector.lane_count))
                .ok_or(FormatError::SyntaxError {
                    message: "bytecode object contains an invalid vector type",
                }),
            ValueTag::WORDS => Ok(format!("words<{}>", ty.word_count())),
            _ => Err(FormatError::SyntaxError {
                message: "bytecode object contains an invalid value type",
            }),
        }
    }

    /// Return one required stable string.
    pub(super) fn string(&self, id: StringId) -> FormatResult<&'a str> {
        self.object.string(id).ok_or(FormatError::SyntaxError {
            message: "bytecode object references a missing string",
        })
    }

    /// Return one required type symbol name.
    pub(super) fn type_name(&self, ty: TypeId) -> FormatResult<&'a str> {
        let name = self.object.type_name(ty).ok_or(FormatError::SyntaxError {
            message: "bytecode object references a missing type",
        })?;

        self.string(name)
    }

    /// Return one reference value type text.
    fn reference_type_text(&self, ty: ValueType) -> FormatResult<String> {
        let reference = ty.reference_type().ok_or(FormatError::SyntaxError {
            message: "reference value has invalid qualifiers",
        })?;
        let kind = reference.kind().name().ok_or(FormatError::SyntaxError {
            message: "reference value has invalid ownership",
        })?;
        let space = reference.space().name().ok_or(FormatError::SyntaxError {
            message: "reference value has invalid space",
        })?;
        let value = format!("ref<{kind}, space({space})>");

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
            Ok("function".to_string())
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
        let element = self.type_name(element)?;
        let kind = reference.kind().name().ok_or(FormatError::SyntaxError {
            message: "slice value has invalid ownership",
        })?;
        let space = reference.space().name().ok_or(FormatError::SyntaxError {
            message: "slice value has invalid space",
        })?;
        let value = format!("slice<{element}, {kind}, space({space})>");

        // wrap slices whose storage is not initialized yet
        if ty.tag() == ValueTag::UNINIT_SLICE {
            Ok(format!("uninit<{value}>"))
        } else {
            Ok(value)
        }
    }

    /// Return one tensor value type text.
    fn tensor_type_text(&self, ty: ValueType) -> FormatResult<String> {
        let scalar = ty.tensor_scalar().ok_or(FormatError::SyntaxError {
            message: "tensor value has no scalar representation",
        })?;
        let tensor = ty.tensor_type().ok_or(FormatError::SyntaxError {
            message: "tensor value has no runtime type",
        })?;
        let name = self.type_name(tensor)?;
        let constructor = if ty.tag() == ValueTag::TENSOR {
            "tensor"
        } else {
            "tensorView"
        };

        if ty.tag() == ValueTag::TENSOR_VIEW {
            let reference = ty.tensor_reference().ok_or(FormatError::SyntaxError {
                message: "tensor view has no backing reference",
            })?;
            let kind = reference.kind().name().ok_or(FormatError::SyntaxError {
                message: "tensor view has invalid reference ownership",
            })?;
            let space = reference.space().name().ok_or(FormatError::SyntaxError {
                message: "tensor view has invalid space",
            })?;

            Ok(format!(
                "{constructor}<{}, {name}, {kind}, space({space}), {}>",
                scalar.name(),
                ty.word_count()
            ))
        } else {
            let reference = ty.tensor_reference().ok_or(FormatError::SyntaxError {
                message: "tensor value has no storage reference",
            })?;
            let space = reference.space().name().ok_or(FormatError::SyntaxError {
                message: "tensor value has invalid space",
            })?;

            Ok(format!(
                "{constructor}<{}, {name}, space({space})>",
                scalar.name()
            ))
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
