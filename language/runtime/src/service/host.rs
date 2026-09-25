use serde::{Deserialize, Serialize};
use tspp_rpc::service;
use tspp_serde::Reflect;

use crate::host::HostEvent;
use crate::world::random::RandomSource;
use crate::world::{ClockSource, Instant};

use super::WorldId;

/// RPC operations over one World's controllable host.
#[service(name = "tspp.world.Host")]
pub trait HostService {
    // =============================================================================
    // Clock
    // =============================================================================

    /// Read one World's effective Clock.
    #[rpc(name = "ReadClock", idempotency = "no_side_effects")]
    fn read_clock(request: ReadClockRequest) -> Clock;

    /// Advance one runtime-controlled Clock.
    #[rpc(name = "AdvanceClock")]
    fn advance_clock(request: AdvanceClockRequest) -> Clock;

    /// Replace one runtime-controlled Clock.
    #[rpc(name = "SetClock")]
    fn set_clock(request: SetClockRequest) -> Clock;

    // =============================================================================
    // Random
    // =============================================================================

    /// Read one World's effective Random source.
    #[rpc(name = "ReadRandom", idempotency = "no_side_effects")]
    fn read_random(request: ReadRandomRequest) -> Random;

    /// Reseed one deterministic Random source.
    #[rpc(name = "ReseedRandom")]
    fn reseed_random(request: ReseedRandomRequest) -> Random;

    // =============================================================================
    // Event
    // =============================================================================

    /// Send one typed host Event into a World.
    #[rpc(name = "SendEvent")]
    fn send_event(request: SendEventRequest) -> ();
}

/// Effective World clock reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Clock {
    /// Effective clock source.
    pub source: ClockSource,
    /// Current wall-clock time.
    pub wall: Instant,
    /// Current monotonic time.
    pub monotonic: Instant,
}

/// Effective World randomness state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Random {
    /// Effective randomness source.
    pub source: RandomSource,
    /// Deterministic root seed, or absent for host randomness.
    pub seed: Option<u64>,
}

/// Request to read one World's effective Clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadClockRequest {
    /// World to inspect.
    pub world_id: WorldId,
}

/// Request to advance one runtime-controlled Clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct AdvanceClockRequest {
    /// World whose Clock advances.
    pub world_id: WorldId,
    /// Absolute wall-clock deadline to reach.
    pub deadline: Instant,
}

/// Request to replace one runtime-controlled Clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SetClockRequest {
    /// World whose Clock changes.
    pub world_id: WorldId,
    /// Replacement wall-clock time.
    pub wall: Instant,
    /// Replacement monotonic time.
    pub monotonic: Instant,
}

/// Request to read one World's effective Random source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReadRandomRequest {
    /// World to inspect.
    pub world_id: WorldId,
}

/// Request to reseed one deterministic Random source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReseedRandomRequest {
    /// World whose Random source changes.
    pub world_id: WorldId,
    /// Replacement deterministic root seed.
    pub seed: u64,
}

/// Request to send one typed host Event into a World.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SendEventRequest {
    /// World receiving the Event.
    pub world_id: WorldId,
    /// Host Event to enqueue.
    pub event: HostEvent,
}
