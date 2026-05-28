use destack_engine as engine;
use destack_mir as mir;
use engine::ValueLayoutId;

/// One lowered frame move slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MoveSlot {
    /// The value layout id.
    pub layout: ValueLayoutId,
    /// Byte offset from the frame base.
    pub offset: u32,
    /// Slot byte length.
    pub byte_len: u32,
    /// Whether this slot stores one word.
    pub is_word: bool,
}

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
    /// Destination frame slot.
    pub dest: MoveSlot,
    /// Source frame slot or void fill.
    pub source: MoveSource,
}

/// Source for one lowered frame move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MoveSource {
    /// Move from a frame slot.
    Slot(MoveSlot),
    /// Write the canonical void value.
    Void,
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
