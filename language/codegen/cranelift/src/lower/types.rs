use cranelift_codegen::ir::{Type as CraneliftType, types as cl_types};
use destack_mir::{LocalNodeId, NodeTree, Type as MirType};

use crate::CraneliftError;

/// Map a MIR type to a Cranelift IR type.
pub(crate) fn lower_type(
    tree: &NodeTree,
    type_id: LocalNodeId<MirType>,
) -> Result<CraneliftType, CraneliftError> {
    let mir_type = tree.get(type_id);
    lower_type_inner(tree, mir_type)
}

fn lower_type_inner(_tree: &NodeTree, mir_type: &MirType) -> Result<CraneliftType, CraneliftError> {
    match mir_type {
        MirType::Void => {
            // void is handled specially at the function level (no return)
            // but we need a placeholder for internal use
            Ok(cl_types::I8)
        }

        MirType::Boolean => Ok(cl_types::I8),

        MirType::Int { width, signed: _ } => match width {
            8 => Ok(cl_types::I8),
            16 => Ok(cl_types::I16),
            32 => Ok(cl_types::I32),
            64 => Ok(cl_types::I64),
            128 => Ok(cl_types::I128),
            _ => Err(CraneliftError::UnsupportedType {
                description: format!("integer width {width} not supported"),
            }),
        },

        MirType::Float { width } => match width {
            32 => Ok(cl_types::F32),
            64 => Ok(cl_types::F64),
            _ => Err(CraneliftError::UnsupportedType {
                description: format!("float width {width} not supported"),
            }),
        },

        MirType::Pointer { .. } => {
            // pointers are always pointer-sized (we use I64 for now)
            // nocheckin TODO: use target :Pointer size
            Ok(cl_types::I64)
        }

        MirType::Array { .. } => Err(CraneliftError::UnsupportedType {
            description: "array types must be lowered to memory operations".into(),
        }),

        MirType::Tuple { .. } => Err(CraneliftError::UnsupportedType {
            description: "tuple types must be lowered to struct operations".into(),
        }),

        MirType::Struct { .. } => Err(CraneliftError::UnsupportedType {
            description: "struct types must be lowered to memory operations".into(),
        }),

        MirType::FunctionPointer { .. } => {
            // function pointers are just pointers (see above :Pointers)
            Ok(cl_types::I64)
        }
    }
}

/// Check if a MIR type is void.
pub(crate) fn is_void_type(tree: &NodeTree, type_id: LocalNodeId<MirType>) -> bool {
    matches!(tree.get(type_id), MirType::Void)
}
