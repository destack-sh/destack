use destack_mir as mir;

/// Sentinel value id used for optional destinations.
pub(crate) const INVALID_VALUE_ID: u32 = u32::MAX;

/// Argument range within one function argument pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ArgumentRange {
    /// Start offset into the argument pool.
    pub start: u32,
    /// Number of arguments in the range.
    pub len: u32,
    /// Whether the arguments are contiguous SSA ids.
    pub is_contiguous: bool,
    /// First SSA value id when contiguous.
    pub contiguous_start: u32,
}

impl ArgumentRange {
    /// Create an empty argument range.
    pub(crate) const fn empty() -> Self {
        Self {
            start: 0,
            len: 0,
            is_contiguous: false,
            contiguous_start: 0,
        }
    }

    /// Slice arguments from the pool for this range.
    #[inline(always)]
    pub(crate) fn slice<'a>(&self, pool: &'a [mir::Value]) -> &'a [mir::Value] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "argument pool out of bounds for range"
        );

        // return argument slice
        &pool[start..start + len]
    }
}

/// Switch case range within one function switch pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct SwitchRange {
    /// Start offset into the switch case pool.
    pub start: u32,
    /// Number of cases in the range.
    pub len: u32,
}

impl SwitchRange {
    /// Create an empty switch range.
    pub(crate) const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }
}

/// Copy pair for parameter binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CopyPair {
    /// Destination SSA value id.
    pub dest: u32,
    /// Source SSA value id.
    pub src: u32,
}

/// Copy range within one function copy pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CopyRange {
    /// Start offset into the copy pool.
    pub start: u32,
    /// Number of pairs in the range.
    pub len: u32,
    /// Whether the copies are contiguous pairs.
    pub is_contiguous: bool,
    /// First source id when contiguous.
    pub contiguous_src: u32,
    /// First destination id when contiguous.
    pub contiguous_dest: u32,
}

impl CopyRange {
    /// Create an empty copy range.
    pub(crate) const fn empty() -> Self {
        Self {
            start: 0,
            len: 0,
            is_contiguous: false,
            contiguous_src: 0,
            contiguous_dest: 0,
        }
    }

    /// Slice pairs from the pool for this range.
    #[inline(always)]
    pub(crate) fn slice<'a>(&self, pool: &'a [CopyPair]) -> &'a [CopyPair] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "copy pool out of bounds for range"
        );

        // return copy slice
        &pool[start..start + len]
    }
}

/// Pack an optional SSA value into the lowered module encoding.
pub(crate) fn pack_optional_value(value: Option<mir::Value>) -> mir::Value {
    value.unwrap_or(mir::Value(INVALID_VALUE_ID))
}

/// Return whether one lowered SSA slot carries the packed absent-value marker.
pub(crate) fn is_invalid_value(value: mir::Value) -> bool {
    value.0 == INVALID_VALUE_ID
}
