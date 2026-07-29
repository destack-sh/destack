use crate::worker::WorkerId;
use crate::worker::scheduler::Wake;
use crate::world::RuntimeId;

/// Wake addressed to one worker in one runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerWake {
    /// Runtime that owns the worker.
    pub runtime_id: RuntimeId,
    /// Worker that receives the wake.
    pub worker_id: WorkerId,
    /// Wake payload routed by the worker event loop.
    pub wake: Wake,
}
