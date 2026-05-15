use destack_mir as mir;

use super::layout::compute_type_layout;
use crate::{CodegenCraneliftError, CodegenCraneliftResult};

/// Native static data bytes and linker relocations.
pub(crate) struct StaticData {
    /// The bytes written into the data section.
    pub(crate) bytes: Vec<u8>,
    /// The linker relocations applied to the data section.
    pub(crate) relocations: Vec<StaticRelocation>,
}

/// Function address written by the linker into static data.
pub(crate) struct StaticRelocation {
    /// The byte offset inside the data section.
    pub(crate) byte_offset: usize,
    /// The target function.
    pub(crate) target: mir::LocalNodeId<mir::Function>,
}

/// One aggregate initializer element and its byte offset.
struct StaticElement {
    /// The element type.
    ty: mir::LocalNodeId<mir::Type>,
    /// The byte offset inside the aggregate.
    byte_offset: usize,
}

/// Lower one MIR global initializer to native static data.
pub(crate) fn lower_static_data(
    tree: &mir::Tree,
    init: &mir::GlobalInitializer,
    ty: mir::LocalNodeId<mir::Type>,
    pointer_bytes: u8,
) -> CodegenCraneliftResult<StaticData> {
    match init {
        mir::GlobalInitializer::Zero => {
            // zero data is sized from the target layout
            let layout = compute_type_layout(tree, ty, pointer_bytes)?;

            Ok(StaticData {
                bytes: vec![0u8; layout.size as usize],
                relocations: Vec::new(),
            })
        }
        mir::GlobalInitializer::Scalar(constant) => {
            // scalar constants lower directly to bytes
            let bytes = lower_scalar_constant(constant, ty, pointer_bytes)?;

            Ok(StaticData {
                bytes,
                relocations: Vec::new(),
            })
        }
        mir::GlobalInitializer::FunctionAddress(function) => {
            // function addresses are linker relocations
            let Some(target) = function.function() else {
                return Err(CodegenCraneliftError::unsupported_type(
                    "missing function address initializer target",
                    ty.into_any(),
                ));
            };

            Ok(StaticData {
                bytes: vec![0u8; pointer_bytes as usize],
                relocations: vec![StaticRelocation {
                    byte_offset: 0,
                    target,
                }],
            })
        }
        mir::GlobalInitializer::Bytes(bytes) => {
            // raw bytes are already target data
            Ok(StaticData {
                bytes: bytes.clone(),
                relocations: Vec::new(),
            })
        }
        mir::GlobalInitializer::Aggregate(elements) => {
            // aggregate data includes target padding
            let layout = compute_type_layout(tree, ty, pointer_bytes)?;
            let static_elements = static_elements(tree, ty, elements.len(), pointer_bytes)?;
            let mut data = StaticData {
                bytes: vec![0u8; layout.size as usize],
                relocations: Vec::new(),
            };

            // copy initialized elements into layout offsets
            for (element_init, element) in elements.iter().zip(static_elements.iter()) {
                let element_data =
                    lower_static_data(tree, element_init, element.ty, pointer_bytes)?;
                let end = element.byte_offset + element_data.bytes.len();

                if end > data.bytes.len() {
                    return Err(CodegenCraneliftError::Internal {
                        message: "static initializer element exceeds aggregate layout".to_string(),
                    });
                }

                data.bytes[element.byte_offset..end].copy_from_slice(&element_data.bytes);
                data.relocations
                    .extend(element_data.relocations.into_iter().map(|relocation| {
                        StaticRelocation {
                            byte_offset: element.byte_offset + relocation.byte_offset,
                            target: relocation.target,
                        }
                    }));
            }

            Ok(data)
        }
    }
}

/// Return aggregate initializer elements in source order.
fn static_elements(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    expected_len: usize,
    pointer_bytes: u8,
) -> CodegenCraneliftResult<Vec<StaticElement>> {
    match tree.get(ty) {
        mir::Type::Struct { .. } | mir::Type::Tuple { .. } | mir::Type::Slice { .. } => {
            record_static_elements(tree, ty, expected_len)
        }
        mir::Type::Array {
            element,
            length,
            copy: _,
        } => array_static_elements(tree, ty, *element, *length, expected_len, pointer_bytes),
        mir_type => Err(CodegenCraneliftError::unsupported_type(
            format!("aggregate initializer for non-aggregate type: {mir_type:?}"),
            ty.into_any(),
        )),
    }
}

/// Return record-like initializer elements from layout metadata.
fn record_static_elements(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
    expected_len: usize,
) -> CodegenCraneliftResult<Vec<StaticElement>> {
    // record layouts carry authoritative field offsets
    let layout = tree.metadata.layout.type_layout(ty).ok_or_else(|| {
        CodegenCraneliftError::unsupported_type("missing layout metadata", ty.into_any())
    })?;
    if layout.fields.len() != expected_len {
        return Err(CodegenCraneliftError::Internal {
            message: "aggregate initializer length does not match layout fields".to_string(),
        });
    }

    let elements = layout
        .fields
        .iter()
        .map(|field| StaticElement {
            ty: field.ty,
            byte_offset: field.offset as usize,
        })
        .collect();

    Ok(elements)
}

/// Return array initializer elements from element stride.
fn array_static_elements(
    tree: &mir::Tree,
    array: mir::LocalNodeId<mir::Type>,
    element: mir::TypeReference,
    length: u64,
    expected_len: usize,
    pointer_bytes: u8,
) -> CodegenCraneliftResult<Vec<StaticElement>> {
    // arrays repeat one element layout
    let element = type_id(element, "array element type")?;
    let count = length as usize;
    if count != expected_len {
        return Err(CodegenCraneliftError::Internal {
            message: "array initializer length does not match array type".to_string(),
        });
    }

    let stride = array_static_stride(tree, array, element, pointer_bytes)?;
    let elements = (0..count)
        .map(|index| StaticElement {
            ty: element,
            byte_offset: index * stride,
        })
        .collect();

    Ok(elements)
}

/// Return the byte stride for one array element type.
fn array_static_stride(
    tree: &mir::Tree,
    array: mir::LocalNodeId<mir::Type>,
    element: mir::LocalNodeId<mir::Type>,
    pointer_bytes: u8,
) -> CodegenCraneliftResult<usize> {
    // layout metadata is authoritative when present
    if let Some(layout) = tree.metadata.layout.type_layout(array)
        && let mir::LayoutShape::Array { element_stride, .. } = layout.shape
    {
        return Ok(element_stride as usize);
    }

    // otherwise use the computed element layout
    let element_layout = compute_type_layout(tree, element, pointer_bytes)?;
    let stride = align_to(element_layout.size, element_layout.alignment);

    Ok(stride as usize)
}

/// Lower one scalar constant to native bytes.
fn lower_scalar_constant(
    constant: &mir::Constant,
    ty: mir::LocalNodeId<mir::Type>,
    pointer_bytes: u8,
) -> CodegenCraneliftResult<Vec<u8>> {
    match constant {
        mir::Constant::Null => Ok(vec![0u8; pointer_bytes as usize]),
        mir::Constant::Boolean { value } => Ok(vec![if *value { 1 } else { 0 }]),
        mir::Constant::Int { width, value, .. } => {
            let bytes = integer_bytes(*value as u128, *width, ty)?;

            Ok(bytes)
        }
        mir::Constant::UInt { width, value } => {
            let bytes = integer_bytes(*value, *width, ty)?;

            Ok(bytes)
        }
        mir::Constant::Float { bits, width } => {
            let bytes = float_bytes(*bits, u16::from(*width), ty)?;

            Ok(bytes)
        }
        mir::Constant::Char { value } => {
            let bytes = (*value as u32).to_le_bytes().to_vec();

            Ok(bytes)
        }
    }
}

/// Return little endian integer bytes.
fn integer_bytes(
    value: u128,
    width: u16,
    ty: mir::LocalNodeId<mir::Type>,
) -> CodegenCraneliftResult<Vec<u8>> {
    let bytes = match width {
        8 => vec![value as u8],
        16 => (value as u16).to_le_bytes().to_vec(),
        32 => (value as u32).to_le_bytes().to_vec(),
        64 => (value as u64).to_le_bytes().to_vec(),
        128 => value.to_le_bytes().to_vec(),
        _ => {
            return Err(CodegenCraneliftError::unsupported_type(
                format!("unsupported integer width for global initializer: {width}"),
                ty.into_any(),
            ));
        }
    };

    Ok(bytes)
}

/// Return little endian float bytes.
fn float_bytes(
    bits: u64,
    width: u16,
    ty: mir::LocalNodeId<mir::Type>,
) -> CodegenCraneliftResult<Vec<u8>> {
    let bytes = match width {
        32 => (bits as u32).to_le_bytes().to_vec(),
        64 => bits.to_le_bytes().to_vec(),
        _ => {
            return Err(CodegenCraneliftError::unsupported_type(
                format!("unsupported float width for global initializer: {width}"),
                ty.into_any(),
            ));
        }
    };

    Ok(bytes)
}

/// Align a byte offset up to one alignment.
fn align_to(offset: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        return offset;
    }

    let remainder = offset % alignment;
    if remainder == 0 {
        return offset;
    }

    offset + (alignment - remainder)
}

/// Return one concrete MIR type from a recoverable reference.
fn type_id(
    ty: mir::TypeReference,
    context: &str,
) -> CodegenCraneliftResult<mir::LocalNodeId<mir::Type>> {
    ty.ty().ok_or_else(|| CodegenCraneliftError::Internal {
        message: format!("missing or malformed MIR type in native static lowering: {context}"),
    })
}

#[cfg(test)]
mod tests {
    use destack_source::FileId;

    use super::*;

    const POINTER_BYTES: u8 = 8;

    /// Lowers aggregate globals with target padding.
    #[test]
    fn test_lower_static_data_preserves_record_padding() {
        let source = r#"
readonly global padded: { int8, int32, int16 }, space(static) = { 1int8, 100int32, 50int16 };
"#;
        let (tree, global) = parse_global(source, "padded");

        let initializer = global.initializer.as_ref().expect("missing initializer");
        let ty = global.ty.ty().expect("missing global type");
        let data = lower_static_data(&tree, initializer, ty, POINTER_BYTES)
            .expect("failed to lower static data");

        let expected = vec![1, 0, 0, 0, 100, 0, 0, 0, 50, 0, 0, 0];
        assert_eq!(data.bytes, expected);
        assert!(data.relocations.is_empty());
    }

    /// Lowers function addresses as relocations inside aggregate globals.
    #[test]
    fn test_lower_static_data_records_function_address_relocation() {
        let source = r#"
function target(): void {
b0:
    return
}

readonly global table: [ref<void, raw, readonly, space(static), nullable>; 2], space(static) = { null, functionAddress target };
"#;
        let (tree, global) = parse_global(source, "table");

        let initializer = global.initializer.as_ref().expect("missing initializer");
        let ty = global.ty.ty().expect("missing global type");
        let data = lower_static_data(&tree, initializer, ty, POINTER_BYTES)
            .expect("failed to lower static data");

        let target = first_function(&tree);
        assert_eq!(data.bytes, vec![0u8; 16]);
        assert_eq!(data.relocations.len(), 1);
        assert_eq!(data.relocations[0].byte_offset, 8);
        assert_eq!(data.relocations[0].target, target);
    }

    /// Parse one MIR source and return one global by name.
    fn parse_global(source: &str, name: &str) -> (mir::Tree, mir::Global) {
        let (tree, strings) =
            mir::parse::Parser::parse(FileId::new(0), source, mir::parse::ParseOptions::default())
                .finish()
                .expect("failed to parse MIR");

        let global = tree
            .iter_nodes::<mir::Global>()
            .find_map(|(_, global)| (strings.get(global.name) == name).then_some(global.clone()))
            .expect("missing global");

        (tree, global)
    }

    /// Return the first function id in one parsed fixture.
    fn first_function(tree: &mir::Tree) -> mir::LocalNodeId<mir::Function> {
        tree.iter_nodes::<mir::Function>()
            .next()
            .map(|(function_id, _)| function_id)
            .expect("missing function")
    }
}
