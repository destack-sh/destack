use destack_mir as mir;

use crate::value::{RawPointer, Value};

/// String header byte layout for managed runtime strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringLayout {
    /// The native raw-pointer width in bytes.
    native_pointer_bytes: u8,
}

impl StringLayout {
    /// The field index for `lengthUtf16`.
    pub const LENGTH_UTF16_FIELD: usize = 0;
    /// The field index for `lengthBytes`.
    pub const LENGTH_BYTES_FIELD: usize = 1;
    /// The field index for `data`.
    pub const DATA_FIELD: usize = 2;

    /// The total field count for a string header.
    pub const FIELD_COUNT: usize = 3;

    /// The byte offset for `lengthUtf16`.
    pub const LENGTH_UTF16_OFFSET: usize = 0;
    /// The byte offset for `lengthBytes`.
    pub const LENGTH_BYTES_OFFSET: usize = 4;
    /// Create one string layout for the active native pointer width.
    pub const fn new(native_pointer_bytes: u8) -> Self {
        Self {
            native_pointer_bytes,
        }
    }

    /// Return the native raw-pointer width in bytes.
    pub const fn native_pointer_bytes(self) -> u8 {
        self.native_pointer_bytes
    }

    /// Return the byte offset for `data`.
    pub fn data_offset(self) -> usize {
        align_offset(
            Self::LENGTH_BYTES_OFFSET + 4,
            self.native_pointer_bytes as usize,
        )
    }

    /// Return the total byte length for one string header.
    pub fn byte_len(self) -> usize {
        let data_end = self.data_offset() + self.native_pointer_bytes as usize;
        align_offset(data_end, 8)
    }

    /// Return one field byte offset.
    pub fn field_offset(self, index: u32) -> Option<usize> {
        match index as usize {
            Self::LENGTH_UTF16_FIELD => Some(Self::LENGTH_UTF16_OFFSET),
            Self::LENGTH_BYTES_FIELD => Some(Self::LENGTH_BYTES_OFFSET),
            Self::DATA_FIELD => Some(self.data_offset()),
            _ => None,
        }
    }

    /// Read one field from one string header payload.
    pub fn read_field(self, bytes: &[u8], index: u32) -> Option<Value> {
        match index as usize {
            Self::LENGTH_UTF16_FIELD => {
                let raw = Self::read_u32(bytes, Self::LENGTH_UTF16_OFFSET)?;
                Some(Value::uint32(raw))
            }
            Self::LENGTH_BYTES_FIELD => {
                let raw = Self::read_u32(bytes, Self::LENGTH_BYTES_OFFSET)?;
                Some(Value::uint32(raw))
            }
            Self::DATA_FIELD => {
                let raw = self.read_raw_pointer(bytes)?;
                Some(Value::raw_pointer(RawPointer::from_bits(raw)))
            }
            _ => None,
        }
    }

    /// Write one field in one string header payload.
    pub fn write_field(self, bytes: &mut [u8], index: u32, value: Value) -> bool {
        match index as usize {
            Self::LENGTH_UTF16_FIELD => Self::write_u32(bytes, Self::LENGTH_UTF16_OFFSET, value),
            Self::LENGTH_BYTES_FIELD => Self::write_u32(bytes, Self::LENGTH_BYTES_OFFSET, value),
            Self::DATA_FIELD => self.write_raw_pointer(bytes, value),
            _ => false,
        }
    }

    fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
        let window = bytes.get(offset..offset + 4)?;
        let mut raw = [0u8; 4];
        raw.copy_from_slice(window);
        Some(u32::from_le_bytes(raw))
    }

    fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
        let window = bytes.get(offset..offset + 8)?;
        let mut raw = [0u8; 8];
        raw.copy_from_slice(window);
        Some(u64::from_le_bytes(raw))
    }

    fn write_u32(bytes: &mut [u8], offset: usize, value: Value) -> bool {
        let Some(raw) = value.as_uint() else {
            return false;
        };
        let Ok(raw) = u32::try_from(raw) else {
            return false;
        };
        let Some(window) = bytes.get_mut(offset..offset + 4) else {
            return false;
        };

        window.copy_from_slice(&raw.to_le_bytes());
        true
    }

    fn read_raw_pointer(self, bytes: &[u8]) -> Option<u64> {
        let offset = self.data_offset();

        match self.native_pointer_bytes {
            4 => Self::read_u32(bytes, offset).map(u64::from),
            8 => Self::read_u64(bytes, offset),
            _ => None,
        }
    }

    fn write_raw_pointer(self, bytes: &mut [u8], value: Value) -> bool {
        let Some(pointer) = value.as_raw_pointer() else {
            return false;
        };

        let offset = self.data_offset();
        match self.native_pointer_bytes {
            4 => {
                let Ok(raw) = u32::try_from(pointer.bits()) else {
                    return false;
                };
                let Some(window) = bytes.get_mut(offset..offset + 4) else {
                    return false;
                };

                window.copy_from_slice(&raw.to_le_bytes());
                true
            }
            8 => {
                let Some(window) = bytes.get_mut(offset..offset + 8) else {
                    return false;
                };

                window.copy_from_slice(&pointer.bits().to_le_bytes());
                true
            }
            _ => false,
        }
    }
}

/// Align one byte offset to one byte alignment.
fn align_offset(offset: usize, alignment: usize) -> usize {
    let remainder = offset % alignment;
    if remainder == 0 {
        return offset;
    }

    offset + (alignment - remainder)
}

/// MIR type alias for the canonical lowered string layout used by VM tests.
pub const STRING_TYPE_ALIAS: &str = "type String {\n    lengthUtf16: uint32;\n    lengthBytes: uint32;\n    data: ref<uint8, raw>;\n}\n";

/// Return the canonical runtime string layout id when present.
pub fn string_layout_id(tree: &mir::NodeTree) -> Option<mir::LayoutId> {
    tree.string_layout_id()
}

/// Check whether a MIR type matches the runtime string layout.
pub fn string_layout_matches(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> bool {
    let mir::Type::Struct { fields, .. } = tree.get(ty) else {
        return false;
    };

    if fields.len() != StringLayout::FIELD_COUNT {
        return false;
    }

    let field_type = |index: usize| match tree.get(fields[index]).ty {
        mir::TypeReference::Type(ty) => Some(ty),
        mir::TypeReference::Missing | mir::TypeReference::Error => None,
    };

    if !field_type(StringLayout::LENGTH_UTF16_FIELD)
        .is_some_and(|ty| unsigned_int_type_matches(tree, ty, 32))
    {
        return false;
    }
    if !field_type(StringLayout::LENGTH_BYTES_FIELD)
        .is_some_and(|ty| unsigned_int_type_matches(tree, ty, 32))
    {
        return false;
    }
    if !field_type(StringLayout::DATA_FIELD).is_some_and(|ty| raw_u8_reference_matches(tree, ty)) {
        return false;
    }

    true
}

/// Check whether a type is an unsigned integer with the expected width.
fn unsigned_int_type_matches(
    tree: &mir::NodeTree,
    ty: mir::LocalNodeId<mir::Type>,
    width: u16,
) -> bool {
    matches!(
        tree.get(ty),
        mir::Type::Int {
            width: ty_width,
            is_signed: false,
        } if *ty_width == width
    )
}

/// Check whether a type is a raw reference to u8 data.
fn raw_u8_reference_matches(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> bool {
    let mir::Type::Reference { kind, pointee, .. } = tree.get(ty) else {
        return false;
    };
    if *kind != mir::ReferenceKind::Raw {
        return false;
    }

    let mir::TypeReference::Type(pointee) = *pointee else {
        return false;
    };

    unsigned_int_type_matches(tree, pointee, 8)
}
