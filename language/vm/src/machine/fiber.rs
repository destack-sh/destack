use std::sync::Arc;

use destack_bytecode::RegisterSpan;
use destack_memory::{MemoryMap, MemoryRange};
use destack_program as program;
use destack_program::{Context, Word};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Error, Result};

use super::{Frame, NativeStack, Stack};

/// One schedulable execution with a persistent stack in world memory.
#[derive(Debug)]
pub struct Fiber {
    /// The fiber's register stack inside world memory.
    pub(crate) stack: Stack,
    /// The fiber's native machine stack inside world memory, reserved at its first native entry.
    pub(crate) native_stack: Option<NativeStack>,
    /// The active call frames in caller to callee order.
    pub(crate) frames: Vec<Frame>,
    /// The dynamically scoped context carried by this fiber.
    pub(crate) context: Context,
    /// The parked call's result registers awaiting one wake value.
    pub(crate) wake_to: Option<RegisterSpan>,
    /// The logical fiber carrying this execution.
    pub(crate) fiber_id: Option<program::FiberId>,
}

impl Fiber {
    /// Create one idle fiber with a reserved stack.
    pub fn new(memory: Arc<MemoryMap>, stack_bytes: usize) -> Result<Self> {
        let stack = Stack::new(memory, stack_bytes)?;

        Ok(Self {
            stack,
            native_stack: None,
            frames: Vec::new(),
            context: Context::default(),
            wake_to: None,
            fiber_id: None,
        })
    }

    /// Fork this fiber over one already-forked world memory map.
    pub fn fork(&self, memory: Arc<MemoryMap>) -> Self {
        Self {
            native_stack: self
                .native_stack
                .as_ref()
                .map(|native_stack| native_stack.fork(memory.clone())),
            stack: self.stack.fork(memory),
            frames: self.frames.clone(),
            context: self.context,
            wake_to: self.wake_to,
            fiber_id: self.fiber_id,
        }
    }

    /// Return this fiber's native stack, reserved at its first native entry.
    pub fn native_stack(&mut self, byte_len: usize) -> Result<&NativeStack> {
        let native_stack = match self.native_stack.take() {
            Some(native_stack) => native_stack,
            None => NativeStack::new(self.stack.memory(), byte_len)?,
        };

        Ok(self.native_stack.insert(native_stack))
    }

    /// Return whether this fiber holds no execution.
    pub fn is_idle(&self) -> bool {
        self.frames.is_empty()
    }

    /// Return the number of active frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Release every frame, stack byte, and mounted identity.
    pub fn clear(&mut self) {
        self.frames.clear();
        self.stack.clear();
        self.wake_to = None;
        self.fiber_id = None;
    }

    /// Mount one logical fiber identity on this execution.
    pub fn mount(&mut self, fiber_id: program::FiberId) {
        self.fiber_id = Some(fiber_id);
    }

    /// Return the logical fiber executing here when mounted.
    pub fn fiber_id(&self) -> Option<program::FiberId> {
        self.fiber_id
    }

    /// Return the world memory map that owns this fiber's stack.
    pub fn memory(&self) -> Arc<MemoryMap> {
        self.stack.memory()
    }

    /// Return the dynamically scoped context carried by this fiber.
    pub fn context(&self) -> Context {
        self.context
    }

    /// Capture one durable image of this fiber's execution state.
    pub fn image(&self) -> FiberImage {
        FiberImage {
            stack_range: self.stack.range(),
            native_stack_range: self.native_stack.as_ref().map(NativeStack::range),
            stack_byte_len: self.stack.byte_len(),
            frames: self.frames.clone(),
            context: self.context,
            wake_to: self.wake_to,
            fiber_id: self.fiber_id,
        }
    }

    /// Rebuild one fiber from its image over restored world memory.
    pub fn from_image(memory: Arc<MemoryMap>, image: &FiberImage) -> Result<Self> {
        let stack = Stack::from_range(memory, image.stack_range, image.stack_byte_len)?;

        // reject frames or wake targets outside the restored live stack
        for frame in image.frames.iter() {
            let end = frame
                .register_offset
                .checked_add(frame.register_count as usize)
                .and_then(|words| words.checked_mul(Word::BYTE_LEN))
                .ok_or_else(Error::invalid_image)?;
            if end > image.stack_byte_len {
                return Err(Error::invalid_image());
            }
        }
        match (image.wake_to, image.frames.last()) {
            (Some(wake_to), Some(frame)) => {
                let end = wake_to.start.0 as usize + wake_to.word_count as usize;
                if end > frame.register_count as usize {
                    return Err(Error::invalid_image());
                }
            }
            (Some(_), None) => return Err(Error::invalid_image()),
            (None, _) => {}
        }

        let native_stack = image
            .native_stack_range
            .map(|range| NativeStack::from_range(stack.memory(), range));

        Ok(Self {
            stack,
            native_stack,
            frames: image.frames.clone(),
            context: image.context,
            wake_to: image.wake_to,
            fiber_id: image.fiber_id,
        })
    }
}

/// Durable image of one fiber's execution state; stack contents live in world memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiberImage {
    /// The reserved stack range in world memory.
    pub(crate) stack_range: MemoryRange,
    /// The reserved native stack range in world memory, when the fiber entered native code.
    pub(crate) native_stack_range: Option<MemoryRange>,
    /// The live stack byte length.
    pub(crate) stack_byte_len: usize,
    /// The active call frames in caller to callee order.
    pub(crate) frames: Vec<Frame>,
    /// The dynamically scoped context carried by the fiber.
    pub(crate) context: Context,
    /// The parked call's result registers awaiting one wake value.
    pub(crate) wake_to: Option<RegisterSpan>,
    /// The logical fiber executing here.
    pub(crate) fiber_id: Option<program::FiberId>,
}
