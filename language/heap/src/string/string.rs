#![allow(dead_code)]

use destack_mir as mir;

/// String header slot layout for managed heap strings.
#[derive(Debug)]
pub struct StringLayout;

// string layout as defined by the builtin/lib/native/string/string.ds file
impl StringLayout {
    /// The slot index for `lengthUtf16`.
    pub const LENGTH_UTF16: usize = 0;
    /// The slot index for `lengthBytes`.
    pub const LENGTH_BYTES: usize = 1;
    /// The slot index for `hash`.
    pub const HASH: usize = 2;
    /// The slot index for `capacity`.
    pub const CAPACITY: usize = 3;
    /// The slot index for `flags`.
    pub const FLAGS: usize = 4;
    /// The slot index for `data`.
    pub const DATA: usize = 5;
    /// The total slot count for a string header.
    pub const SLOT_COUNT: usize = 6;
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
    // ensure the type is a struct with the expected slot count
    let mir::Type::Struct { fields, .. } = tree.get(ty) else {
        return false;
    };
    if fields.len() != StringLayout::SLOT_COUNT {
        return false;
    }

    // resolve field types
    let field_type = |index: usize| tree.get(fields[index]).ty;

    // validate header slots
    if !unsigned_int_type_matches(tree, field_type(StringLayout::LENGTH_UTF16), 32) {
        return false;
    }
    if !unsigned_int_type_matches(tree, field_type(StringLayout::LENGTH_BYTES), 32) {
        return false;
    }
    if !unsigned_int_type_matches(tree, field_type(StringLayout::HASH), 64) {
        return false;
    }
    if !unsigned_int_type_matches(tree, field_type(StringLayout::CAPACITY), 32) {
        return false;
    }
    if !unsigned_int_type_matches(tree, field_type(StringLayout::FLAGS), 32) {
        return false;
    }
    if !raw_u8_reference_matches(tree, field_type(StringLayout::DATA)) {
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
