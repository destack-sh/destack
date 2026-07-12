use std::fmt::{Debug, Formatter};
use std::num::NonZeroU32;

use super::Condition;

/// The layout selected for one formatting group.
#[repr(u8)]
#[derive(Debug, Copy, Default, Clone, Eq, PartialEq)]
pub enum GroupMode {
    /// Print the group on one line when it fits.
    #[default]
    Flat = 0,
    /// Always print the group in expanded mode.
    Expand = 1,
    /// Print the group in expanded mode because nested content requires it.
    Propagated = 2,
}

impl GroupMode {
    /// Return whether this group may print on one line.
    pub const fn is_flat(self) -> bool {
        matches!(self, Self::Flat)
    }
}

/// One logical group written into the instruction tape.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct Group {
    /// The optional externally referenced group identifier.
    id: Option<GroupId>,
    /// The initial group layout.
    mode: GroupMode,
}

impl Group {
    /// Create one flat anonymous group.
    pub const fn new() -> Self {
        Self {
            id: None,
            mode: GroupMode::Flat,
        }
    }

    /// Set the externally referenced group identifier.
    #[must_use]
    pub const fn with_id(mut self, id: Option<GroupId>) -> Self {
        self.id = id;

        self
    }

    /// Set the initial group layout.
    #[must_use]
    pub const fn with_mode(mut self, mode: GroupMode) -> Self {
        self.mode = mode;

        self
    }

    /// Return the externally referenced group identifier.
    pub const fn id(self) -> Option<GroupId> {
        self.id
    }

    /// Return the initial group layout.
    pub const fn mode(self) -> GroupMode {
        self.mode
    }
}

/// One conditionally active logical group.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ConditionalGroup {
    /// The condition controlling this group.
    condition: Condition,
}

impl ConditionalGroup {
    /// Create one conditional group.
    pub const fn new(condition: Condition) -> Self {
        Self { condition }
    }

    /// Return the condition controlling this group.
    pub const fn condition(self) -> Condition {
        self.condition
    }
}

/// The dense index of one group in a formatted document.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct GroupIndex(u32);

impl GroupIndex {
    /// Create one group index.
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return this index as a vector offset.
    pub(crate) const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Return the encoded index.
    pub(crate) const fn value(self) -> u32 {
        self.0
    }
}

/// The mutable layout row for one regular or conditional group.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum GroupState {
    /// One regular group.
    Regular {
        /// The optional externally referenced group identifier.
        id: Option<GroupId>,
        /// The selected group layout.
        mode: GroupMode,
    },
    /// One conditionally active group.
    Conditional {
        /// The condition controlling this group.
        condition: Condition,
        /// The selected group layout.
        mode: GroupMode,
    },
}

impl GroupState {
    /// Create one regular group row.
    pub(crate) const fn regular(group: Group) -> Self {
        Self::Regular {
            id: group.id(),
            mode: group.mode(),
        }
    }

    /// Create one conditional group row.
    pub(crate) const fn conditional(group: ConditionalGroup) -> Self {
        Self::Conditional {
            condition: group.condition(),
            mode: GroupMode::Flat,
        }
    }

    /// Return the selected group layout.
    pub(crate) const fn mode(self) -> GroupMode {
        match self {
            Self::Regular { mode, .. } | Self::Conditional { mode, .. } => mode,
        }
    }

    /// Return the optional identifier of one regular group.
    pub(crate) const fn id(self) -> Option<GroupId> {
        match self {
            Self::Regular { id, .. } => id,
            Self::Conditional { .. } => None,
        }
    }

    /// Return the condition of this conditional group row.
    pub(crate) fn condition(self) -> Condition {
        let condition = match self {
            Self::Conditional { condition, .. } => Some(condition),
            Self::Regular { .. } => None,
        };
        debug_assert!(condition.is_some());

        // safety: conditional group opcodes only reference conditional rows
        unsafe { condition.unwrap_unchecked() }
    }

    /// Propagate expanded layout into this group.
    pub(crate) fn expand(&mut self) {
        let mode = match self {
            Self::Regular { mode, .. } | Self::Conditional { mode, .. } => mode,
        };

        if mode.is_flat() {
            *mode = GroupMode::Propagated;
        }
    }
}

/// A document-local identifier for a group referenced by later instructions.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct GroupId(NonZeroU32);

impl GroupId {
    /// Create one group identifier.
    pub(crate) const fn new(value: NonZeroU32) -> Self {
        Self(value)
    }
}

impl From<GroupId> for u32 {
    fn from(id: GroupId) -> Self {
        id.0.get()
    }
}

impl Debug for GroupId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}
