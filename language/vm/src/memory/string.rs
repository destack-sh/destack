/// String header slot layout for managed heap strings.
pub(crate) struct StringLayout;

// StringLayout as defined by the builtin/lib/native/string/string.ds file.
impl StringLayout {
    /// The slot index for `lengthUtf16`.
    pub(crate) const LENGTH_UTF16: usize = 0;
    /// The slot index for `lengthBytes`.
    pub(crate) const LENGTH_BYTES: usize = 1;
    /// The slot index for `hash`.
    pub(crate) const HASH: usize = 2;
    /// The slot index for `capacity`.
    pub(crate) const CAPACITY: usize = 3;
    /// The slot index for `flags`.
    pub(crate) const FLAGS: usize = 4;
    /// The slot index for `data`.
    pub(crate) const DATA: usize = 5;
    /// The total slot count for a string header.
    pub(crate) const SLOT_COUNT: usize = 6;
}

/// Flag indicating the hash field is populated.
#[allow(dead_code)]
pub(crate) const STRING_FLAG_HAS_HASH: u32 = 1 << 0;
/// Flag indicating the payload is ASCII-only.
pub(crate) const STRING_FLAG_IS_ASCII: u32 = 1 << 1;
/// Flag indicating the payload is static read-only data.
pub(crate) const STRING_FLAG_IS_STATIC: u32 = 1 << 2;
/// Flag indicating the string contents are interned.
pub(crate) const STRING_FLAG_IS_INTERNED: u32 = 1 << 3;
/// Flag indicating the payload is owned externally.
#[allow(dead_code)]
pub(crate) const STRING_FLAG_IS_EXTERNAL: u32 = 1 << 4;
