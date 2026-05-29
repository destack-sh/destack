use destack_engine as engine;
use destack_mir as mir;
use engine::StaticSpace;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Error, RuntimeError, RuntimeResult, StackTraceFrame};
use crate::interpreter::{Continuation, FrameImage, StackImage};
use crate::options::IsolateOptions;
use crate::program::Program;
use crate::{Result, Word};
use destack_heap::{HeapResult, RootSlot};

use super::{Frame, Stack};

/// Interpreter execution engine.
#[derive(Debug)]
pub struct Interpreter {
    /// Explicit frame stack used for execution and root walking.
    pub(crate) frames: Vec<Frame>,
    /// Page-backed byte stack for frame data.
    pub(crate) stack: Stack,
}

/// Coroutine-capable interpreter outcome.
pub type Outcome = engine::Outcome<Continuation, engine::Value>;

/// Immutable interpreter image.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterImage {
    /// The captured stack bytes.
    pub stack: StackImage,
    /// The captured frame stack.
    pub frames: Vec<FrameImage>,
}

impl Interpreter {
    /// Create a new interpreter engine.
    pub(crate) fn new(options: &IsolateOptions) -> RuntimeResult<Self> {
        let stack = Stack::new(options.limits.stack_bytes)?;

        Ok(Self {
            frames: Vec::new(),
            stack,
        })
    }

    /// Prepare the stack arena for one top-level run.
    pub(crate) fn reset_stack(&mut self, options: &IsolateOptions) -> RuntimeResult<()> {
        self.frames.clear();
        self.stack.reset(options.limits.stack_bytes)?;

        Ok(())
    }

    /// Allocate one frame byte record in the stack arena.
    pub(crate) fn allocate_frame(
        &mut self,
        layout: &engine::FrameLayout,
    ) -> RuntimeResult<(usize, usize)> {
        let base = self
            .stack
            .allocate_zeroed(layout.byte_len as usize, Word::BYTE_LEN)?;
        let frame_base = self.stack.address(base, layout.byte_len as usize)?;

        Ok((base, frame_base))
    }

    /// Release stack bytes above one frame base.
    pub(crate) fn truncate_stack(&mut self, stack_offset: usize) {
        debug_assert!(stack_offset <= self.stack.len());
        self.stack.truncate(stack_offset);
    }

    /// Capture one immutable interpreter image.
    pub(crate) fn image(&self, program: &Program) -> RuntimeResult<InterpreterImage> {
        let stack = self.stack.image()?;
        let frames = self
            .frames
            .iter()
            .map(|frame| frame.image(program))
            .collect::<RuntimeResult<Vec<_>>>()?;

        Ok(InterpreterImage { stack, frames })
    }

    /// Fork this interpreter for one child isolate.
    pub(crate) fn fork(&self) -> RuntimeResult<Self> {
        let stack = self.stack.fork()?;
        let mut frames = Vec::with_capacity(self.frames.len());

        // clone frames over the forked stack bytes
        for frame in &self.frames {
            let base = stack
                .address(frame.stack_offset, frame.byte_len)
                .map_err(|_| RuntimeError::new(Error::invalid_continuation()))?;
            frames.push(frame.fork(base));
        }

        Ok(Self { frames, stack })
    }

    /// Create one interpreter from an immutable image.
    pub(crate) fn from_image(
        program: &Program,
        image: &InterpreterImage,
        options: &IsolateOptions,
    ) -> RuntimeResult<Self> {
        let mut interpreter = Self {
            frames: Vec::with_capacity(image.frames.len()),
            stack: Stack::from_image(&image.stack, options.limits.stack_bytes)?,
        };

        // restore frame metadata over stack image byte ranges
        for frame_image in &image.frames {
            let frame_base = interpreter
                .stack
                .address(frame_image.stack_offset, frame_image.byte_len)?;
            let frame =
                Frame::from_image(frame_image, program, frame_image.stack_offset, frame_base)?;

            interpreter.frames.push(frame);
        }

        Ok(interpreter)
    }

    /// Create a runtime error with current call stack.
    #[cold]
    pub(crate) fn runtime_error(&self, program: &Program, error: Error) -> RuntimeError {
        match self.call_stack(program) {
            Ok(stack) => RuntimeError::new(error).with_call_stack(stack),
            Err(stack_error) => RuntimeError::new(Error::internal(format!(
                "failed to build call stack for {error:?}: {stack_error:?}"
            ))),
        }
    }

    /// Initialize static data from MIR globals.
    pub(crate) fn initialize_statics(
        &mut self,
        program: &Program,
        statics: &mut StaticSpace,
    ) -> RuntimeResult<()> {
        // allocate static bytes
        let mut initialized_statics = StaticSpace::allocator();

        // snapshot globals before writing static bytes
        let global_entries: Vec<_> = program
            .tree
            .iter_nodes::<mir::Global>()
            .map(|(id, global)| {
                let ty = (global.ty)
                    .ty()
                    .ok_or_else(|| Error::invalid_program("global type"))?;

                Ok((id, ty, global.is_import(), global.initializer.clone()))
            })
            .collect::<Result<Vec<_>>>()?;

        // populate static bytes from global initializers
        for (id, ty, is_import, initializer) in global_entries {
            // skip imported globals
            if is_import || program.contains_static(id) {
                continue;
            }

            let layout = program.layout(ty).ok_or_else(|| {
                self.runtime_error(
                    program,
                    Error::type_mismatch("compiled global layout", format!("{ty:?}")),
                )
            })?;
            let bytes = match initializer.as_ref() {
                Some(init) => program
                    .initializer_bytes(init, ty)
                    .map_err(|error| self.runtime_error(program, error))?,
                None => vec![0; layout.byte_len],
            };
            let was_defined = initialized_statics.define(
                program.static_id(id),
                program.value_layout_id(ty),
                layout.alignment(),
                program.tree.get(id).is_mutable(),
                &bytes,
            );
            if !was_defined {
                return Err(self.runtime_error(program, Error::invalid_instruction()));
            }
        }

        // store initialized static data
        *statics = initialized_statics.finish();

        Ok(())
    }

    /// Return the current call stack for error reporting.
    fn call_stack(&self, program: &Program) -> Result<Vec<StackTraceFrame>> {
        self.frames
            .iter()
            .map(|f| {
                let function = f.function();
                let block = f.block_id(program)?;
                let func = program.tree.get(function);
                let name = program.strings.get(func.name).to_string();
                Ok(StackTraceFrame {
                    function,
                    block,
                    function_name: Some(name),
                })
            })
            .collect()
    }

    /// Visit mutable heap root slots from active state and suspended continuations.
    pub(crate) fn visit_root_slots(
        &mut self,
        program: &Program,
        statics: &mut StaticSpace,
        continuations: &mut [Continuation],
        visit: &mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>,
    ) -> RuntimeResult<()> {
        // active frames
        for frame in &mut self.frames {
            let result = frame.visit_root_slots(program, visit);
            if let Err(error) = result {
                return Err(self.runtime_error(program, error));
            }
        }

        // suspended continuations
        for continuation in continuations {
            continuation
                .visit_root_slots(program, visit)
                .map_err(|error| self.runtime_error(program, error))?;
        }

        let static_ids: Vec<_> = statics.ids().collect();

        // statics
        for id in static_ids {
            let region = statics
                .region(id)
                .cloned()
                .ok_or_else(|| self.runtime_error(program, Error::invalid_instruction()))?;
            let bytes = statics
                .bytes_mut(id)
                .ok_or_else(|| self.runtime_error(program, Error::invalid_instruction()))?;

            program
                .visit_byte_root_slots(program.type_for_value_layout(region.layout), bytes, visit)
                .map_err(|error| self.runtime_error(program, error))?;
        }

        Ok(())
    }
}
