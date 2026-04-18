use serde::{Deserialize, Serialize};

use crate::runtime::bindings::BindingAffinity;
use crate::runtime::{ExecutionContext, ExecutionContextId, execution_context_satisfies};

/// Stored resource-affinity requirement for one live resource entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceAffinity {
    /// Require the owning worker event-loop context.
    EventLoop,
    /// Require the captured owner execution context.
    Owner(ExecutionContextId),
    /// Require the process main execution context.
    ProcessMain,
}

impl ResourceAffinity {
    /// Build one resource-affinity requirement from one binding affinity and execution context.
    pub const fn from_binding_affinity(
        affinity: BindingAffinity,
        execution_context: ExecutionContext,
    ) -> Option<Self> {
        match affinity {
            BindingAffinity::Any => None,
            BindingAffinity::EventLoop => Some(Self::EventLoop),
            BindingAffinity::Owner => Some(Self::Owner(execution_context.id)),
            BindingAffinity::ProcessMain => Some(Self::ProcessMain),
        }
    }

    /// Return the binding-affinity class represented by this resource requirement.
    pub const fn binding_affinity(self) -> BindingAffinity {
        match self {
            Self::EventLoop => BindingAffinity::EventLoop,
            Self::Owner(_) => BindingAffinity::Owner,
            Self::ProcessMain => BindingAffinity::ProcessMain,
        }
    }

    /// Return the captured owner token when this requirement is owner-affine.
    pub const fn owner_affinity(self) -> Option<ExecutionContextId> {
        match self {
            Self::Owner(owner_affinity) => Some(owner_affinity),
            Self::EventLoop | Self::ProcessMain => None,
        }
    }

    /// Return whether one execution context satisfies this resource requirement.
    pub const fn satisfies(
        self,
        execution_context: ExecutionContext,
        event_loop_context_id: ExecutionContextId,
    ) -> bool {
        execution_context_satisfies(
            execution_context,
            event_loop_context_id,
            self.binding_affinity(),
            self.owner_affinity(),
        )
    }
}
