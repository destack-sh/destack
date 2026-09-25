use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{
    AllocationSite, BindingId, CallSite, EdgeSite, FunctionId, MemoryAccess, MemoryRange,
    MemorySite, ProgramPoint, TypeId,
};

/// One observable Program execution event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Event {
    /// One Program point reached execution.
    Point {
        /// The reached Program point.
        point: ProgramPoint,
    },
    /// One control-flow edge was taken.
    Edge {
        /// The executed control-flow edge.
        site: EdgeSite,
    },
    /// One function call began.
    Call {
        /// The executed call site.
        site: CallSite,
        /// The resolved callee.
        function: FunctionId,
    },
    /// One frame entered or left execution.
    Frame {
        /// The frame event.
        event: FrameEvent,
        /// The affected function.
        function: FunctionId,
    },
    /// One allocation completed.
    Allocation {
        /// The executed allocation site.
        site: AllocationSite,
        /// The allocated memory range.
        range: MemoryRange,
    },
    /// One memory access completed.
    Memory {
        /// The executed memory site.
        site: MemorySite,
        /// The executed memory access.
        access: MemoryAccess,
        /// The accessed memory range when materialized.
        range: Option<MemoryRange>,
    },
    /// One runtime binding call entered or exited.
    Binding {
        /// The binding event.
        event: BindingEvent,
        /// The called binding.
        binding_id: BindingId,
    },
    /// One language panic began unwinding.
    Panic {
        /// The Program point that raised the panic.
        point: ProgramPoint,
        /// The carried value type when present.
        ty: Option<TypeId>,
    },
}

/// One Program execution event category.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// Program point events.
    Point,
    /// Control-flow edge events.
    Edge,
    /// Function call events.
    Call,
    /// Frame events.
    Frame,
    /// Allocation events.
    Allocation,
    /// Memory access events.
    Memory,
    /// Runtime binding events.
    Binding,
    /// Language panic events.
    Panic,
}

/// Program execution event categories selected for observation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventSet(u8);

/// One runtime binding event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum BindingEvent {
    /// One runtime binding call entered.
    Enter,
    /// One runtime binding call completed successfully.
    Exit,
}

/// One frame event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum FrameEvent {
    /// One frame entered execution.
    Enter,
    /// One frame left execution.
    Exit,
}

impl Event {
    /// Return this event's category.
    pub const fn kind(self) -> EventKind {
        match self {
            Self::Point { .. } => EventKind::Point,
            Self::Edge { .. } => EventKind::Edge,
            Self::Call { .. } => EventKind::Call,
            Self::Frame { .. } => EventKind::Frame,
            Self::Allocation { .. } => EventKind::Allocation,
            Self::Memory { .. } => EventKind::Memory,
            Self::Binding { .. } => EventKind::Binding,
            Self::Panic { .. } => EventKind::Panic,
        }
    }
}

impl EventSet {
    /// Return whether no event category is selected.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Return whether one event category is selected.
    pub const fn contains(self, kind: EventKind) -> bool {
        self.0 & (1_u8 << kind as u8) != 0
    }

    /// Select one event category.
    pub fn insert(&mut self, kind: EventKind) {
        self.0 |= 1_u8 << kind as u8;
    }
}
