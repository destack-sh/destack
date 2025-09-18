use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::num::NonZeroU32;

use crate::Condition;

#[derive(Debug, Copy, Default, Clone, Eq, PartialEq)]
pub enum GroupMode {
    /// Print group in flat mode.
    #[default]
    Flat,
    /// The group should be printed in expanded mode
    Expand,
    /// Expand mode has been propagated from an enclosing group to this group.
    Propagated,
}

impl GroupMode {
    pub const fn is_flat(&self) -> bool {
        matches!(self, GroupMode::Flat)
    }
}

/// Logical group of elements.
/// (The elements are implicit in the element stream surrounded by group delimiters.)
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Group {
    id: Option<GroupId>,
    mode: Cell<GroupMode>,
}

impl Group {
    pub fn new() -> Self {
        Self {
            id: None,
            mode: Cell::new(GroupMode::Flat),
        }
    }

    #[must_use]
    pub fn with_id(mut self, id: Option<GroupId>) -> Self {
        self.id = id;
        self
    }

    #[must_use]
    pub fn with_mode(mut self, mode: GroupMode) -> Self {
        self.mode = Cell::new(mode);
        self
    }

    pub fn mode(&self) -> GroupMode {
        self.mode.get()
    }

    pub fn propagate_expand(&self) {
        if self.mode.get() == GroupMode::Flat {
            self.mode.set(GroupMode::Propagated);
        }
    }

    pub fn id(&self) -> Option<GroupId> {
        self.id
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConditionalGroup {
    mode: Cell<GroupMode>,
    condition: Condition,
}

impl ConditionalGroup {
    pub fn new(condition: Condition) -> Self {
        Self {
            mode: Cell::new(GroupMode::Flat),
            condition,
        }
    }

    pub fn condition(&self) -> Condition {
        self.condition
    }

    pub fn propagate_expand(&self) {
        self.mode.set(GroupMode::Propagated);
    }

    pub fn mode(&self) -> GroupMode {
        self.mode.get()
    }
}

/// Unique identification for a group (with a name, for debugging).
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct DebugGroupId {
    value: NonZeroU32,
    name: &'static str,
}

impl DebugGroupId {
    #[allow(unused)]
    pub(crate) fn new(value: NonZeroU32, debug_name: &'static str) -> Self {
        Self {
            value,
            name: debug_name,
        }
    }
}

impl Debug for DebugGroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}-{}", self.name, self.value)
    }
}

/// Unique identification for a group.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct ReleaseGroupId {
    value: NonZeroU32,
}

impl ReleaseGroupId {
    /// Creates a new unique group id with the given debug name (only stored in debug builds)
    #[allow(unused)]
    pub(crate) fn new(value: NonZeroU32, _: &'static str) -> Self {
        Self { value }
    }
}

impl From<GroupId> for u32 {
    fn from(id: GroupId) -> Self {
        id.value.get()
    }
}

impl Debug for ReleaseGroupId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.value)
    }
}

#[cfg(not(debug_assertions))]
pub type GroupId = ReleaseGroupId;
#[cfg(debug_assertions)]
pub type GroupId = DebugGroupId;
