use destack_mir as mir;

use crate::value::{RawPointer, Value};

/// String header byte layout for managed runtime strings.
#[derive(Debug)]
pub struct StringLayout;

impl StringLayout {
    /// The field index for `lengthUtf16`.
    pub const LENGTH_UTF16_FIELD: usize = 0;
    /// The field index for `lengthBytes`.
    pub const LENGTH_BYTES_FIELD: usize = 1;
    /// The field index for `hash`.
    pub const HASH_FIELD: usize = 2;
    /// The field index for `capacity`.
    pub const CAPACITY_FIELD: usize = 3;
    /// The field index for `flags`.
    pub const FLAGS_FIELD: usize = 4;
    /// The field index for `data`.
    pub const DATA_FIELD: usize = 5;

    /// The total field count for a string header.
    pub const FIELD_COUNT: usize = 6;

    /// The byte offset for `lengthUtf16`.
    pub const LENGTH_UTF16_OFFSET: usize = 0;
    /// The byte offset for `lengthBytes`.
    pub const LENGTH_BYTES_OFFSET: usize = 4;
    /// The byte offset for `hash`.
    pub const HASH_OFFSET: usize = 8;
    /// The byte offset for `capacity`.
    pub const CAPACITY_OFFSET: usize = 16;
    /// The byte offset for `flags`.
    pub const FLAGS_OFFSET: usize = 20;
    /// The byte offset for `data`.
    pub const DATA_OFFSET: usize = 24;

    /// The total byte length for one string header.
    pub const BYTE_LEN: usize = 32;

    /// Return one field byte offset.
    pub fn field_offset(index: u32) -> Option<usize> {
        match index as usize {
            Self::LENGTH_UTF16_FIELD => Some(Self::LENGTH_UTF16_OFFSET),
            Self::LENGTH_BYTES_FIELD => Some(Self::LENGTH_BYTES_OFFSET),
            Self::HASH_FIELD => Some(Self::HASH_OFFSET),
            Self::CAPACITY_FIELD => Some(Self::CAPACITY_OFFSET),
            Self::FLAGS_FIELD => Some(Self::FLAGS_OFFSET),
            Self::DATA_FIELD => Some(Self::DATA_OFFSET),
            _ => None,
        }
    }

    /// Read one field from one string header payload.
    pub fn read_field(bytes: &[u8], index: u32) -> Option<Value> {
        match index as usize {
            Self::LENGTH_UTF16_FIELD => {
                let raw = Self::read_u32(bytes, Self::LENGTH_UTF16_OFFSET)?;
                Some(Value::uint32(raw))
            }
            Self::LENGTH_BYTES_FIELD => {
                let raw = Self::read_u32(bytes, Self::LENGTH_BYTES_OFFSET)?;
                Some(Value::uint32(raw))
            }
            Self::HASH_FIELD => {
                let raw = Self::read_u64(bytes, Self::HASH_OFFSET)?;
                Some(Value::uint64(raw))
            }
            Self::CAPACITY_FIELD => {
                let raw = Self::read_u32(bytes, Self::CAPACITY_OFFSET)?;
                Some(Value::uint32(raw))
            }
            Self::FLAGS_FIELD => {
                let raw = Self::read_u32(bytes, Self::FLAGS_OFFSET)?;
                Some(Value::uint32(raw))
            }
            Self::DATA_FIELD => {
                let raw = Self::read_u64(bytes, Self::DATA_OFFSET)?;
                Some(Value::raw_pointer(RawPointer::from_bits(raw)))
            }
            _ => None,
        }
    }

    /// Write one field in one string header payload.
    pub fn write_field(bytes: &mut [u8], index: u32, value: Value) -> bool {
        match index as usize {
            Self::LENGTH_UTF16_FIELD => Self::write_u32(bytes, Self::LENGTH_UTF16_OFFSET, value),
            Self::LENGTH_BYTES_FIELD => Self::write_u32(bytes, Self::LENGTH_BYTES_OFFSET, value),
            Self::HASH_FIELD => Self::write_u64(bytes, Self::HASH_OFFSET, value),
            Self::CAPACITY_FIELD => Self::write_u32(bytes, Self::CAPACITY_OFFSET, value),
            Self::FLAGS_FIELD => Self::write_u32(bytes, Self::FLAGS_OFFSET, value),
            Self::DATA_FIELD => Self::write_raw_pointer(bytes, Self::DATA_OFFSET, value),
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

    fn write_u64(bytes: &mut [u8], offset: usize, value: Value) -> bool {
        let Some(raw) = value.as_uint() else {
            return false;
        };
        let Some(window) = bytes.get_mut(offset..offset + 8) else {
            return false;
        };

        window.copy_from_slice(&raw.to_le_bytes());
        true
    }

    fn write_raw_pointer(bytes: &mut [u8], offset: usize, value: Value) -> bool {
        let Some(pointer) = value.as_raw_pointer() else {
            return false;
        };
        let Some(window) = bytes.get_mut(offset..offset + 8) else {
            return false;
        };

        window.copy_from_slice(&pointer.bits().to_le_bytes());
        true
    }
}

/// Flag indicating the hash field is populated.
pub const STRING_FLAG_HAS_HASH: u32 = 1 << 0;
/// Flag indicating the payload is ASCII-only.
pub const STRING_FLAG_IS_ASCII: u32 = 1 << 1;
/// Flag indicating the payload is static read-only data.
pub const STRING_FLAG_IS_STATIC: u32 = 1 << 2;
/// Flag indicating the string contents are interned.
pub const STRING_FLAG_IS_INTERNED: u32 = 1 << 3;
/// Flag indicating the payload is owned externally.
pub const STRING_FLAG_IS_EXTERNAL: u32 = 1 << 4;

/// MIR type alias for the runtime string layout used by VM tests.
pub const STRING_TYPE_ALIAS: &str = "type @String = { lengthUtf16: u32, lengthBytes: u32, hash: u64, capacity: u32, flags: u32, data: ref<raw u8> }\n";

/// Check whether a MIR type matches the runtime string layout.
pub fn string_layout_matches(tree: &mir::NodeTree, ty: mir::LocalNodeId<mir::Type>) -> bool {
    let mir::Type::Struct { fields, .. } = tree.get(ty) else {
        return false;
    };

    if fields.len() != StringLayout::FIELD_COUNT {
        return false;
    }

    let field_type = |index: usize| tree.get(fields[index]).ty;

    if !unsigned_int_type_matches(tree, field_type(StringLayout::LENGTH_UTF16_FIELD), 32) {
        return false;
    }
    if !unsigned_int_type_matches(tree, field_type(StringLayout::LENGTH_BYTES_FIELD), 32) {
        return false;
    }
    if !unsigned_int_type_matches(tree, field_type(StringLayout::HASH_FIELD), 64) {
        return false;
    }
    if !unsigned_int_type_matches(tree, field_type(StringLayout::CAPACITY_FIELD), 32) {
        return false;
    }
    if !unsigned_int_type_matches(tree, field_type(StringLayout::FLAGS_FIELD), 32) {
        return false;
    }
    if !raw_u8_reference_matches(tree, field_type(StringLayout::DATA_FIELD)) {
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

    unsigned_int_type_matches(tree, *pointee, 8)
}
