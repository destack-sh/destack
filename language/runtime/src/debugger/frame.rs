use std::fmt;

use serde::{Deserialize, Serialize};
use tspp_memory::MemoryImage;
use tspp_program as program;
use tspp_serde::Reflect;
use tspp_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::{RuntimeId, RuntimeImage};
use crate::scheduler::RunnableId;
use crate::worker::{WorkerId, WorkerImage};

/// One captured execution frame.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Frame {
    /// Frame identity.
    pub id: FrameId,
    /// Retained Runnable that owns the frame, when execution is stopped.
    pub runnable_id: Option<RunnableId>,
    /// Captured frame state identifier.
    pub frame_state_id: program::FrameStateId,
    /// Canonical frame bytes.
    pub bytes: Vec<u8>,
}

/// Stable identity of one captured execution frame at one Moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct FrameId {
    /// Runtime that owns the frame.
    pub runtime_id: RuntimeId,
    /// Worker that owns the frame.
    pub worker_id: WorkerId,
    /// Fiber that owns the frame.
    pub fiber_id: program::FiberId,
    /// Frame depth from the active frame.
    pub depth: u32,
}

impl Frame {
    /// Create one captured Frame.
    fn new(
        worker: &WorkerImage,
        fiber_id: program::FiberId,
        depth: u32,
        runnable_id: Option<RunnableId>,
        frame_state_id: program::FrameStateId,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            id: FrameId {
                runtime_id: worker.runtime_id,
                worker_id: worker.worker_id,
                fiber_id,
                depth,
            },
            runnable_id,
            frame_state_id,
            bytes,
        }
    }
}

impl fmt::Debug for Frame {
    /// Format one Frame without traversing canonical frame bytes.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Frame")
            .field("id", &self.id)
            .field("runnable_id", &self.runnable_id)
            .field("frame_state_id", &self.frame_state_id)
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
            self.append_fiber_frames(
                &machine,
                memory,
                retained.fiber_id,
                Some(retained.id),
                image,
                &mut frames,
            )?;
        }

        // expose every parked or woken scheduler fiber
        if let Some(snapshot) = self.event_loop.active() {
            for (fiber_id, image) in snapshot.fibers().executions() {
                self.append_fiber_frames(&machine, memory, fiber_id, None, image, &mut frames)?;
            }
        }

        Ok(frames)
    }

    /// Append one fiber image's frames through their canonical projections.
    fn append_fiber_frames(
        &self,
        machine: &vm::Machine,
        memory: &MemoryImage,
        fiber_id: program::FiberId,
        runnable_id: Option<RunnableId>,
        image: &vm::FiberImage,
        frames: &mut Vec<Frame>,
    ) -> RuntimeResult<()> {
        let mut read = |offset: usize, byte_len: usize| memory.read_bytes(offset, byte_len).ok();
        let projected = machine
            .project_frames(image, &mut read)
            .map_err(Box::<RuntimeError>::from)?;

        for (depth, (frame_state_id, bytes)) in projected.into_iter().rev().enumerate() {
            let depth = u32::try_from(depth)
                .map_err(|_| RuntimeError::inconsistent_image("fiber frame depth exceeds u32"))?;
            frames.push(Frame::new(
                self,
                fiber_id,
                depth,
                runnable_id,
                frame_state_id,
                bytes,
            ));
        }

        Ok(())
    }
}
