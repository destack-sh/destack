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
}

impl ArgumentRange {
    /// Create an empty argument range.
    pub(crate) const fn empty() -> Self {
        Self { start: 0, len: 0 }
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

/// Move pair for parameter binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MovePair {
    /// Destination SSA value id.
    pub dest: u32,
    /// Source SSA value id.
    pub src: u32,
}

/// Move range within one function move pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MoveRange {
    /// Start offset into the move pool.
    pub start: u32,
    /// Number of pairs in the range.
    pub len: u32,
}

impl MoveRange {
    /// Create an empty move range.
    pub(crate) const fn empty() -> Self {
        Self { start: 0, len: 0 }
    }

    /// Slice pairs from the pool for this range.
    #[inline(always)]
    pub(crate) fn slice<'a>(&self, pool: &'a [MovePair]) -> &'a [MovePair] {
        // compute range bounds
        let start = self.start as usize;
        let len = self.len as usize;

        // validate bounds in debug builds
        debug_assert!(
            start + len <= pool.len(),
            "move pool out of bounds for range"
        );

        // return move slice
        &pool[start..start + len]
    }
}

/// Pack an optional SSA value into the lowered program encoding.
pub(crate) fn pack_optional_value(value: Option<mir::Value>) -> mir::Value {
    value.unwrap_or(mir::Value(INVALID_VALUE_ID))
}

/// Return whether one lowered SSA value carries the packed absent-value marker.
pub(crate) fn is_invalid_value(value: mir::Value) -> bool {
    value.0 == INVALID_VALUE_ID
}
