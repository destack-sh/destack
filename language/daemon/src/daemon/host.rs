use tspp_rpc::{Request, Response, Status};
use tspp_runtime::service::{
    AdvanceClockRequest, Clock, HostService, Random, ReadClockRequest, ReadRandomRequest,
    ReseedRandomRequest, SendEventRequest, SetClockRequest,
};
use tspp_runtime::world::Instant;
use tspp_runtime::world::random::RandomSource;

use super::WorldRegistry;

impl HostService for WorldRegistry {
    // =============================================================================
    // Clock
    // =============================================================================

    /// Read one World's effective Clock.
    async fn read_clock(
        &self,
        request: Request<ReadClockRequest>,
    ) -> Result<Response<Clock>, Status> {
        let world = self.world(request.value.world_id)?;
        let world = world.lock();
        let clock = Clock {
            source: world.clock_source(),
            wall: Instant::new(world.wall_nanos()),
            monotonic: Instant::new(world.mono_nanos()),
        };

        Ok(Response::new(clock))
    }

    /// Advance one runtime-controlled Clock.
    async fn advance_clock(
        &self,
        request: Request<AdvanceClockRequest>,
    ) -> Result<Response<Clock>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let mut world = world.lock();
        world
            .advance(request.deadline)
            .map_err(Self::runtime_status)?;
        let clock = Clock {
            source: world.clock_source(),
            wall: Instant::new(world.wall_nanos()),
            monotonic: Instant::new(world.mono_nanos()),
        };

        Ok(Response::new(clock))
    }

    /// Replace one runtime-controlled Clock.
    async fn set_clock(
        &self,
        _request: Request<SetClockRequest>,
    ) -> Result<Response<Clock>, Status> {
        todo!("replace runtime-controlled wall and monotonic Clock state as one mutation")
    }

    // =============================================================================
    // Random
    // =============================================================================

    /// Read one World's effective Random source.
    async fn read_random(
        &self,
        request: Request<ReadRandomRequest>,
    ) -> Result<Response<Random>, Status> {
        let world = self.world(request.value.world_id)?;
        let world = world.lock();
        let source = world.random_source();
        let seed = match source {
            RandomSource::Host => None,
            RandomSource::Deterministic => Some(world.random().root_seed()),
        };

        Ok(Response::new(Random { source, seed }))
    }

    /// Reseed one deterministic Random source.
    async fn reseed_random(
        &self,
        _request: Request<ReseedRandomRequest>,
    ) -> Result<Response<Random>, Status> {
        todo!("reseed deterministic Random state as one replayable World mutation")
    }

    // =============================================================================
    // Event
    // =============================================================================

    /// Send one typed host Event into a World.
    async fn send_event(&self, request: Request<SendEventRequest>) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world.lock().send(request.event);

        Ok(Response::new(()))
    }
}
