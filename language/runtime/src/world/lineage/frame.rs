use std::fmt;

use destack_memory::MemoryImage;
use destack_program as program;
use destack_program::FrameStateId;
use destack_serde::Reflect;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::{RuntimeId, RuntimeImage};
use crate::scheduler::RunnableId;
use crate::worker::{WorkerId, WorkerImage};

/// One captured execution frame.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Frame {
    /// Runtime that owns the frame.
    pub runtime_id: RuntimeId,
    /// Worker that owns the frame.
    pub worker_id: WorkerId,
    /// Retained execution source for the frame.
    pub source: FrameSource,
    /// Captured frame state.
    pub frame_state: FrameStateId,
    /// Canonical live frame bytes.
    pub bytes: Vec<u8>,
}

/// Retained execution source for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum FrameSource {
    /// Execution retained at one handshake or debugger stop.
    Retained {
        /// Retained runnable identifier.
        runnable_id: RunnableId,
        /// Fiber identity whose execution is retained.
        fiber_id: program::FiberId,
    },
    /// Fiber parked or woken in the scheduler.
    Fiber {
        /// Scheduler fiber identity.
        fiber_id: program::FiberId,
    },
}

impl Frame {
    /// Create one captured Frame.
    fn new(
        worker: &WorkerImage,
        source: FrameSource,
        frame_state: FrameStateId,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            runtime_id: worker.runtime_id,
            worker_id: worker.worker_id,
            source,
            frame_state,
            bytes,
        }
    }
}

impl fmt::Debug for Frame {
    /// Format one Frame without traversing canonical frame bytes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Frame")
            .field("runtime_id", &self.runtime_id)
            .field("worker_id", &self.worker_id)
            .field("source", &self.source)
            .field("frame_state", &self.frame_state)
            .finish()
    }
}

impl WorkerImage {
    /// Return all frames retained by one worker image.
    pub(crate) fn frames(
        &self,
        runtime: &RuntimeImage,
        memory: &MemoryImage,
    ) -> RuntimeResult<Vec<Frame>> {
        let machine = vm::Machine::new(runtime.program.clone(), runtime.engine.limits)
            .map_err(Box::<RuntimeError>::from)?;
        let mut frames = Vec::new();

        // expose the execution retained at one handshake or debugger stop
        if let Some(image) = self.machine.stopped() {
            let Some(retained) = self.retained.as_ref() else {
                return Err(RuntimeError::inconsistent_image(format!(
                    "worker {} retains frames without a stopped runnable",
                    self.worker_id.0
                ))
                .boxed());
            };
            let source = FrameSource::Retained {
                runnable_id: retained.id,
                fiber_id: retained.fiber_id,
            };
            self.append_fiber_frames(&machine, memory, source, image, &mut frames)?;
        }

        // expose every parked or woken scheduler fiber
        if let Some(snapshot) = self.event_loop.active() {
            for (fiber_id, image) in snapshot.fibers().executions() {
                let source = FrameSource::Fiber { fiber_id };
                self.append_fiber_frames(&machine, memory, source, image, &mut frames)?;
            }
        }

        Ok(frames)
    }

    /// Append one fiber image's frames through their canonical projections.
    fn append_fiber_frames(
        &self,
        machine: &vm::Machine,
        memory: &MemoryImage,
        source: FrameSource,
        image: &vm::FiberImage,
        frames: &mut Vec<Frame>,
    ) -> RuntimeResult<()> {
        let mut read = |offset: usize, byte_len: usize| memory.read_bytes(offset, byte_len).ok();
        let projected = machine
            .project_frames(image, &mut read)
            .map_err(Box::<RuntimeError>::from)?;

        for (state, bytes) in projected {
            frames.push(Frame::new(self, source, state, bytes));
        }

        Ok(())
    }
}
