use destack_repository::{Environment, WorldOptions};
use destack_runtime::service::WorldId;
use destack_runtime::world::Snapshot;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Request to create one daemon-hosted World.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CreateWorldRequest {
    /// World execution configuration.
    pub options: Option<WorldOptions>,
    /// Ambient World environment.
    pub environment: Option<Environment>,
}

/// Request to restore one daemon-hosted World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct RestoreWorldRequest {
    /// Blob-backed World Snapshot.
    pub snapshot: Snapshot,
}

/// Request to close one daemon-hosted World.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct CloseWorldRequest {
    /// World to close.
    pub world_id: WorldId,
}
