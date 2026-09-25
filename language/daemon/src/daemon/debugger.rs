use tspp_program as program;
use tspp_rpc::{Request, Response, ResponseSender, Status};
use tspp_runtime::debugger::{
    Allocation, Breakpoint, Frame, MemoryMap, Probe, ProbeId, Reference, Root, Watchpoint,
};
use tspp_runtime::diagnostic::RuntimeError;
use tspp_runtime::service::{
    AddBreakpointRequest, AddProbeRequest, AddWatchpointRequest, DebuggerService, EvaluateRequest,
    Evaluation, ListAllocationsRequest, ListBreakpointsRequest, ListProbesRequest,
    ListReferencesRequest, ListRootsRequest, ListWatchpointsRequest, PauseRequest,
    ReadFramesRequest, ReadMemoryMapRequest, ReadMemoryRequest, RemoveBreakpointRequest,
    RemoveProbeRequest, RemoveWatchpointRequest, ResumeRequest, StepRequest,
    UpdateBreakpointRequest, UpdateProbeRequest, UpdateWatchpointRequest,
};
use tspp_runtime::world::RunOutcome;

use super::WorldRegistry;
use crate::service::BYTE_STREAM_CHUNK_BYTE_LEN;

impl DebuggerService for WorldRegistry {
    // =============================================================================
    // Execution
    // =============================================================================

    /// Pause selected World execution.
    async fn pause(&self, _request: Request<PauseRequest>) -> Result<Response<()>, Status> {
        todo!("pause selected World execution at reconstructable Program points")
    }

    /// Resume one debugger-stopped Worker.
    async fn resume(
        &self,
        request: Request<ResumeRequest>,
    ) -> Result<Response<RunOutcome>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let outcome = world
            .lock()
            .resume(request.runtime_id, request.worker_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(outcome))
    }

    /// Step one debugger-stopped Frame.
    async fn step(&self, _request: Request<StepRequest>) -> Result<Response<RunOutcome>, Status> {
        todo!("step retained execution by instruction and source-level Frame boundaries")
    }

    // =============================================================================
    // Evaluation
    // =============================================================================

    /// Evaluate one TS++ expression in a selected execution context.
    async fn evaluate(
        &self,
        _request: Request<EvaluateRequest>,
    ) -> Result<Response<Evaluation>, Status> {
        todo!("compile and evaluate TS++ expressions against retained Program state")
    }

    // =============================================================================
    // Frame
    // =============================================================================

    /// Read captured Frames from one World.
    async fn read_frames(
        &self,
        request: Request<ReadFramesRequest>,
    ) -> Result<Response<Vec<Frame>>, Status> {
        let request = request.value;
        let (_, image) = self.image(request.world_id, request.moment)?;
        let frames = match request.worker_id {
            Some(worker_id) => image.worker_frames(worker_id),
            None => image.frames(),
        }
        .map_err(Self::runtime_status)?;

        Ok(Response::new(frames))
    }

    // =============================================================================
    // Memory
    // =============================================================================

    /// Read one World's mapped memory Regions.
    async fn read_memory_map(
        &self,
        _request: Request<ReadMemoryMapRequest>,
    ) -> Result<Response<MemoryMap>, Status> {
        todo!("project Program, Runtime, Worker, heap, and stack memory Regions")
    }

    /// Stream one exact World memory range.
    async fn read_memory(
        &self,
        request: Request<ReadMemoryRequest>,
        mut responses: ResponseSender<Vec<u8>>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let (_, image) = self.image(request.world_id, Some(request.moment))?;
        let mut offset = request.range.offset;
        let mut remaining = request.range.byte_len;
        let chunk_byte_len = remaining.min(BYTE_STREAM_CHUNK_BYTE_LEN);
        let mut bytes = vec![0; chunk_byte_len];

        // validate the range and read its first bounded chunk
        image
            .memory()
            .read_bytes_into(offset, &mut bytes)
            .map_err(Box::<RuntimeError>::from)
            .map_err(Self::runtime_status)?;

        // preserve stream backpressure while advancing through the exact range
        while !bytes.is_empty() {
            // send the current bounded chunk
            responses
                .send(&bytes)
                .await
                .map_err(|error| error.into_status())?;

            // advance and read the next bounded chunk
            offset += bytes.len();
            remaining -= bytes.len();

            let chunk_byte_len = remaining.min(BYTE_STREAM_CHUNK_BYTE_LEN);
            bytes.resize(chunk_byte_len, 0);
            image
                .memory()
                .read_bytes_into(offset, &mut bytes)
                .map_err(Box::<RuntimeError>::from)
                .map_err(Self::runtime_status)?;
        }

        Ok(Response::new(()))
    }

    // =============================================================================
    // Heap
    // =============================================================================

    /// Stream managed heap Allocations in one World.
    async fn list_allocations(
        &self,
        _request: Request<ListAllocationsRequest>,
        _responses: ResponseSender<Vec<Allocation>>,
    ) -> Result<Response<()>, Status> {
        todo!("enumerate live local and shared heap Allocations in stable address order")
    }

    /// Stream heap Roots in one World.
    async fn list_roots(
        &self,
        _request: Request<ListRootsRequest>,
        _responses: ResponseSender<Vec<Root>>,
    ) -> Result<Response<()>, Status> {
        todo!("enumerate Program, Frame, and host heap Roots")
    }

    /// Stream outgoing References from one heap Allocation.
    async fn list_references(
        &self,
        _request: Request<ListReferencesRequest>,
        _responses: ResponseSender<Vec<Reference>>,
    ) -> Result<Response<()>, Status> {
        todo!("trace outgoing References from one live heap Allocation")
    }

    // =============================================================================
    // Breakpoint
    // =============================================================================

    /// List the Breakpoints in one World.
    async fn list_breakpoints(
        &self,
        request: Request<ListBreakpointsRequest>,
    ) -> Result<Response<Vec<Breakpoint>>, Status> {
        let request = request.value;
        let debugger = self.debugger(request.world_id, request.moment)?;

        Ok(Response::new(debugger.breakpoints().to_vec()))
    }

    /// Add one Breakpoint to a World.
    async fn add_breakpoint(
        &self,
        request: Request<AddBreakpointRequest>,
    ) -> Result<Response<program::BreakpointId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let breakpoint_id = world
            .lock()
            .add_breakpoint(request.filter)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(breakpoint_id))
    }

    /// Update one Breakpoint in a World.
    async fn update_breakpoint(
        &self,
        request: Request<UpdateBreakpointRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .update_breakpoint(request.breakpoint)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Remove one Breakpoint from a World.
    async fn remove_breakpoint(
        &self,
        request: Request<RemoveBreakpointRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .remove_breakpoint(request.breakpoint_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    // =============================================================================
    // Watchpoint
    // =============================================================================

    /// List the Watchpoints in one World.
    async fn list_watchpoints(
        &self,
        request: Request<ListWatchpointsRequest>,
    ) -> Result<Response<Vec<Watchpoint>>, Status> {
        let request = request.value;
        let debugger = self.debugger(request.world_id, request.moment)?;

        Ok(Response::new(debugger.watchpoints().to_vec()))
    }

    /// Add one Watchpoint to a World.
    async fn add_watchpoint(
        &self,
        request: Request<AddWatchpointRequest>,
    ) -> Result<Response<program::WatchpointId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let watchpoint_id = world
            .lock()
            .add_watchpoint(request.filter)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(watchpoint_id))
    }

    /// Update one Watchpoint in a World.
    async fn update_watchpoint(
        &self,
        request: Request<UpdateWatchpointRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .update_watchpoint(request.watchpoint)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Remove one Watchpoint from a World.
    async fn remove_watchpoint(
        &self,
        request: Request<RemoveWatchpointRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .remove_watchpoint(request.watchpoint_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    // =============================================================================
    // Probe
    // =============================================================================

    /// List the Probes in one World.
    async fn list_probes(
        &self,
        request: Request<ListProbesRequest>,
    ) -> Result<Response<Vec<Probe>>, Status> {
        let request = request.value;
        let debugger = self.debugger(request.world_id, request.moment)?;

        Ok(Response::new(debugger.probes().to_vec()))
    }

    /// Add one Probe to a World.
    async fn add_probe(
        &self,
        request: Request<AddProbeRequest>,
    ) -> Result<Response<ProbeId>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        let probe_id = world
            .lock()
            .add_probe(request.filter, request.action)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(probe_id))
    }

    /// Update one Probe in a World.
    async fn update_probe(
        &self,
        request: Request<UpdateProbeRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .update_probe(request.probe)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }

    /// Remove one Probe from a World.
    async fn remove_probe(
        &self,
        request: Request<RemoveProbeRequest>,
    ) -> Result<Response<()>, Status> {
        let request = request.value;
        let world = self.world(request.world_id)?;
        world
            .lock()
            .remove_probe(request.probe_id)
            .map_err(Self::runtime_status)?;

        Ok(Response::new(()))
    }
}
