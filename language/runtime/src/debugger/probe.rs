use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_serde::Reflect;

use crate::runtime::RuntimeId;
use crate::worker::WorkerId;

/// One installed runtime Probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Probe {
    /// Stable probe identifier.
    pub id: ProbeId,
    /// Runtime activity selected by this probe.
    pub filter: ProbeFilter,
    /// Action to perform when this probe matches.
    pub action: ProbeAction,
    /// Whether this probe can currently match.
    pub is_enabled: bool,
    /// Number of Program execution events matched by this Probe.
    pub hit_count: u64,
}

/// One filtered Program execution event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ProbeFilter {
    /// Runtime selected by the filter, or every Runtime when absent.
    pub runtime_id: Option<RuntimeId>,
    /// Worker selected by the filter, or every Worker when absent.
    pub worker_id: Option<WorkerId>,
    /// Fiber selected by the filter, or every execution when absent.
    pub fiber_id: Option<program::FiberId>,
    /// Program execution event selected by the filter.
    pub event: EventFilter,
}

/// Probe action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ProbeAction {
    /// Emit observations at one matching-hit interval.
    Observe {
        /// The number of matching hits between observations.
        interval: NonZeroU64,
    },
    /// Count matching hits.
    Count,
}

/// One Program execution event filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum EventFilter {
    /// Program points, optionally restricted to one exact point.
    Point {
        /// The selected Program point, or every point when absent.
        point: Option<program::ProgramPoint>,
    },
    /// Control-flow edge events.
    Edge {
        /// The selected source point, or every source when absent.
        source: Option<program::ProgramPoint>,
        /// The selected target point, or every target when absent.
        target: Option<program::ProgramPoint>,
    },
    /// Function call events.
    Call {
        /// The selected Program point, or every call point when absent.
        point: Option<program::ProgramPoint>,
        /// The selected callee, or every callee when absent.
        function: Option<program::FunctionId>,
    },
    /// Frame events.
    Frame {
        /// The selected frame event, or every frame event when absent.
        event: Option<program::FrameEvent>,
        /// The selected function, or every function when absent.
        function: Option<program::FunctionId>,
    },
    /// Allocation events.
    Allocation {
        /// The selected Program point, or every allocation point when absent.
        point: Option<program::ProgramPoint>,
        /// The selected result type, or every allocation type when absent.
        result_type: Option<program::TypeId>,
    },
    /// Memory access events.
    Memory {
        /// The selected memory access operation.
        access: program::MemoryAccess,
        /// The selected memory target.
        target: program::MemoryTarget,
    },
    /// Runtime binding events.
    Binding {
        /// The selected binding event, or every binding event when absent.
        event: Option<program::BindingEvent>,
        /// The selected binding, or every binding when absent.
        binding_id: Option<program::BindingId>,
    },
    /// Language panic events.
    Panic {
        /// The selected Program point, or every panic point when absent.
        point: Option<program::ProgramPoint>,
        /// The selected value type, or every panic type when absent.
        ty: Option<program::TypeId>,
    },
}

/// Runtime probe identifier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub struct ProbeId(u64);

impl Probe {
    /// Create one enabled probe.
    pub const fn new(id: ProbeId, filter: ProbeFilter, action: ProbeAction) -> Self {
        Self {
            id,
            filter,
            action,
            is_enabled: true,
            hit_count: 0,
        }
    }
}

impl ProbeFilter {
    /// Return whether this filter selects one execution event.
    pub fn selects(
        &self,
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        fiber_id: Option<program::FiberId>,
        event: program::Event,
    ) -> bool {
        let runtime_matches = self
            .runtime_id
            .is_none_or(|selected| selected == runtime_id);
        let worker_matches = self.worker_id.is_none_or(|selected| selected == worker_id);
        let fiber_matches = self
            .fiber_id
            .is_none_or(|selected| Some(selected) == fiber_id);

        runtime_matches && worker_matches && fiber_matches && self.event.selects(event)
    }

    /// Return whether this filter can select events from one Worker.
    pub fn selects_worker(&self, runtime_id: RuntimeId, worker_id: WorkerId) -> bool {
        let runtime_matches = self
            .runtime_id
            .is_none_or(|selected| selected == runtime_id);
        let worker_matches = self.worker_id.is_none_or(|selected| selected == worker_id);

        runtime_matches && worker_matches
    }
}

impl EventFilter {
    /// Return this filter's Program execution event category.
    pub const fn kind(self) -> program::EventKind {
        match self {
            Self::Point { .. } => program::EventKind::Point,
            Self::Edge { .. } => program::EventKind::Edge,
            Self::Call { .. } => program::EventKind::Call,
            Self::Frame { .. } => program::EventKind::Frame,
            Self::Allocation { .. } => program::EventKind::Allocation,
            Self::Memory { .. } => program::EventKind::Memory,
            Self::Binding { .. } => program::EventKind::Binding,
            Self::Panic { .. } => program::EventKind::Panic,
        }
    }

    /// Return whether this filter selects one Program execution event.
    pub fn selects(self, event: program::Event) -> bool {
        match (self, event) {
            (Self::Point { point }, program::Event::Point { point: actual }) => {
                point.is_none_or(|selected| selected == actual)
            }
            (Self::Edge { source, target }, program::Event::Edge { site }) => {
                let source_matches = source.is_none_or(|selected| selected == site.source);
                let target_matches = target.is_none_or(|selected| selected == site.target);

                source_matches && target_matches
            }
            (
                Self::Call { point, function },
                program::Event::Call {
                    site,
                    function: actual,
                },
            ) => {
                let point_matches = point.is_none_or(|selected| selected == site.point);
                let function_matches = function.is_none_or(|selected| selected == actual);

                point_matches && function_matches
            }
            (
                Self::Frame { event, function },
                program::Event::Frame {
                    event: actual,
                    function: actual_function,
                },
            ) => {
                let event_matches = event.is_none_or(|selected| selected == actual);
                let function_matches = function.is_none_or(|selected| selected == actual_function);

                event_matches && function_matches
            }
            (Self::Allocation { point, result_type }, program::Event::Allocation { site, .. }) => {
                let point_matches = point.is_none_or(|selected| selected == site.point);
                let type_matches = result_type.is_none_or(|selected| selected == site.result_type);

                point_matches && type_matches
            }
            (
                Self::Memory { access, target },
                program::Event::Memory {
                    site,
                    access: actual,
                    range,
                },
            ) => access.selects(actual) && target.selects(site, range),
            (
                Self::Binding { event, binding_id },
                program::Event::Binding {
                    event: actual,
                    binding_id: actual_binding,
                },
            ) => {
                let event_matches = event.is_none_or(|selected| selected == actual);
                let binding_matches = binding_id.is_none_or(|selected| selected == actual_binding);

                event_matches && binding_matches
            }
            (
                Self::Panic { point, ty },
                program::Event::Panic {
                    point: actual,
                    ty: actual_type,
                },
            ) => {
                let point_matches = point.is_none_or(|selected| selected == actual);
                let type_matches = ty.is_none_or(|selected| Some(selected) == actual_type);

                point_matches && type_matches
            }
            _ => false,
        }
    }
}

impl ProbeId {
    /// Create one probe identifier.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw probe identifier value.
    pub const fn get(self) -> u64 {
        self.0
    }
}
