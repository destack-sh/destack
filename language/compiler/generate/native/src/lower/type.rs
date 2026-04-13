use cranelift_codegen::ir as cir;
use destack_mir as mir;

use crate::{CodegenCraneliftError, CodegenCraneliftResult};

/// Lower a MIR type to a Cranelift IR type.
pub(crate) fn lower_type(
    tree: &mir::NodeTree,
    type_id: mir::LocalNodeId<mir::Type>,
    pointer_bytes: u8,
) -> CodegenCraneliftResult<cir::Type> {
    let mir_type = tree.get(type_id);
    match mir_type {
        mir::Type::Void => {
            // void is handled specially at the function level (no return)
            // but we need a placeholder for internal use
            Ok(cir::types::I8)
        }

        mir::Type::Boolean => Ok(cir::types::I8),

        mir::Type::Int {
            width,
            is_signed: _,
        } => match width {
            8 => Ok(cir::types::I8),
            16 => Ok(cir::types::I16),
            32 => Ok(cir::types::I32),
            64 => Ok(cir::types::I64),
            128 => Ok(cir::types::I128),
            _ => Err(CodegenCraneliftError::unsupported_type(
                format!("integer width {width} not supported",),
                type_id.into_any(),
            )),
        },
        mir::Type::Isize | mir::Type::Usize => {
            let ty = match pointer_bytes {
                4 => cir::types::I32,
                8 => cir::types::I64,
                _ => {
                    return Err(CodegenCraneliftError::unsupported_type(
                        format!("unsupported pointer size: {pointer_bytes} bytes"),
                        type_id.into_any(),
                    ));
                }
            };
            Ok(ty)
        }

        mir::Type::Float { width } => match width {
            32 => Ok(cir::types::F32),
            64 => Ok(cir::types::F64),
            _ => Err(CodegenCraneliftError::unsupported_type(
                format!("float width {width} not supported"),
                type_id.into_any(),
            )),
        },

        mir::Type::TypeDescriptor
        | mir::Type::TypeId
        | mir::Type::Reference { .. }
        | mir::Type::FunctionPointer { .. }
        | mir::Type::TensorReference { .. } => {
            // inline pointer_type
            let ty = match pointer_bytes {
                4 => cir::types::I32,
                8 => cir::types::I64,
                _ => {
                    return Err(CodegenCraneliftError::unsupported_type(
                        format!("unsupported pointer size: {pointer_bytes} bytes"),
                        type_id.into_any(),
                    ));
                }
            };
            Ok(ty)
        }

        mir::Type::Array { .. } => Err(CodegenCraneliftError::unsupported_type(
            "array types must be lowered to memory operations",
            type_id.into_any(),
        )),

        mir::Type::Tuple { .. } => Err(CodegenCraneliftError::unsupported_type(
            "tuple types must be lowered to struct operations",
            type_id.into_any(),
        )),

        mir::Type::Struct { .. } => Err(CodegenCraneliftError::unsupported_type(
            "struct types must be lowered to memory operations",
            type_id.into_any(),
        )),

        mir::Type::Closure { .. } => Err(CodegenCraneliftError::unsupported_type(
            "closures must be lowered to aggregate operations",
            type_id.into_any(),
        )),

        mir::Type::Newtype { inner, .. } => {
            let inner = inner.ty().ok_or_else(|| CodegenCraneliftError::Internal {
                message: "missing or malformed MIR type in native lowering: newtype inner type"
                    .into(),
            })?;
            lower_type(tree, inner, pointer_bytes)
        }

        mir::Type::Vector { .. } => Err(CodegenCraneliftError::unsupported_type(
            "vector types are not yet supported by the native backend",
            type_id.into_any(),
        )),

        mir::Type::Tensor { .. } => Err(CodegenCraneliftError::unsupported_type(
            "tensor types are not yet supported by the native backend",
            type_id.into_any(),
        )),
    }
}
