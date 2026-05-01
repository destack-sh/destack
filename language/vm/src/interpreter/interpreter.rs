use destack_engine::{self as engine, StaticSpace};
use destack_mir as mir;

use crate::Word;
use crate::diagnostic::{Error, FrameInfo, RuntimeError, RuntimeResult};
use crate::interpreter::Continuation;
use crate::isolate::RootSink;
use crate::options::IsolateOptions;
use crate::program::Program;
use crate::snapshot::InterpreterImage;
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

impl Interpreter {
    /// Create a new interpreter engine.
    pub(crate) fn new(options: &IsolateOptions) -> RuntimeResult<Self> {
        let stack = Stack::reserve(options.limits.max_stack_bytes)?;

        Ok(Self {
            frames: Vec::new(),
            stack,
        })
    }

    /// Prepare the stack arena for one top-level run.
    pub(crate) fn reset_stack(&mut self, options: &IsolateOptions) -> RuntimeResult<()> {
        self.frames.clear();
        self.stack.reset(options.limits.max_stack_bytes)?;

        Ok(())
    }

    /// Allocate one frame byte record in the stack arena.
    pub(crate) fn allocate_frame(
        &mut self,
        layout: &engine::FrameLayout,
        _options: &IsolateOptions,
    ) -> RuntimeResult<(usize, *mut u8)> {
        let base = self
            .stack
            .allocate(layout.byte_len as usize, Word::BYTE_LEN)?;
        let frame_base = self.stack.address(base, layout.byte_len as usize)?;

        Ok((base, frame_base))
    }

    /// Release stack bytes above one frame base.
    pub(crate) fn truncate_stack(&mut self, stack_offset: usize) {
        debug_assert!(stack_offset <= self.stack.len());
        self.stack.truncate(stack_offset);
    }

    /// Return the current frames for this engine.
    pub(crate) fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Capture one immutable interpreter image.
    pub(crate) fn image(&self) -> InterpreterImage {
        let stack = self.frames.iter().map(Frame::image).collect();

        InterpreterImage { stack }
    }

    /// Fork this interpreter for one child isolate.
    pub(crate) fn fork(&self) -> RuntimeResult<Self> {
        let stack = self.stack.fork()?;
        let mut frames: Vec<_> = self.frames.iter().map(Frame::clone_for_fork).collect();

        // point cloned frames at the forked stack bytes
        for frame in &mut frames {
            let base = stack
                .address(frame.stack_offset, frame.byte_len)
                .map_err(|_| RuntimeError::new(Error::InvalidContinuation))?;
            frame.replace_bytes(frame.stack_offset, frame.byte_len, base);
        }

        Ok(Self { frames, stack })
    }

    /// Create one interpreter from an immutable image.
    pub(crate) fn from_image(
        program: &Program,
        image: &InterpreterImage,
        options: &IsolateOptions,
    ) -> RuntimeResult<Self> {
        let mut interpreter = Self::new(options)?;

        // restore frame bytes before frame metadata points into them
        for frame_image in &image.stack {
            let layout = program
                .frame_layout_by_id(frame_image.frame_layout)
                .ok_or_else(|| RuntimeError::new(Error::InvalidInstruction))?;
            let base = interpreter
                .stack
                .allocate(frame_image.bytes.len(), Word::BYTE_LEN)?;
            interpreter.stack.write(base, &frame_image.bytes)?;
            let frame_base = interpreter.stack.address(base, frame_image.bytes.len())?;
            let frame =
                Frame::from_image(frame_image, &program.functions, layout, base, frame_base)?;

            interpreter.frames.push(frame);
        }

        Ok(interpreter)
    }

    /// Create a runtime error with current call stack.
    #[cold]
    pub(crate) fn runtime_error(&self, program: &Program, error: Error) -> RuntimeError {
        RuntimeError::new(error).with_call_stack(self.call_stack(program))
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
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "global type".to_string(),
                    })?;

                Ok((id, ty, global.is_import(), global.initializer.clone()))
            })
            .collect::<crate::Result<Vec<_>>>()?;

        // populate static bytes from global initializers
        for (id, ty, is_import, initializer) in global_entries {
            // skip imported globals
            if is_import || program.contains_static(id) {
                continue;
            }

            let layout = program.layout(ty).ok_or_else(|| {
                self.runtime_error(
                    program,
                    Error::TypeMismatch {
                        expected: "compiled global layout".to_string(),
                        actual: format!("{ty:?}"),
                    },
                )
            })?;
            let bytes = match initializer.as_ref() {
                Some(init) => program
                    .initializer_bytes(init, ty)
                    .map_err(|error| self.runtime_error(program, error))?,
                None => vec![0; layout.byte_len],
            };
            if initialized_statics
                .define(
                    program.static_id(id),
                    program.type_id(ty),
                    layout.alignment(),
                    program.tree.get(id).is_mutable(),
                    &bytes,
                )
                .is_none()
            {
                return Err(self.runtime_error(program, Error::InvalidInstruction));
            }
        }

        // store initialized static data
        *statics = initialized_statics.finish();

        Ok(())
    }

    /// Return the current call stack for error reporting.
    fn call_stack(&self, program: &Program) -> Vec<FrameInfo> {
        self.frames
            .iter()
            .map(|f| {
                let func = program.tree.get(f.function);
                let name = program.strings.get(func.name).to_string();
                FrameInfo {
                    function: f.function,
                    block: f.current_block,
                    function_name: Some(name),
                }
            })
            .collect()
    }

    /// Visit one complete root set from active state and suspended continuations.
    pub(crate) fn visit_roots(
        &mut self,
        program: &Program,
        statics: &StaticSpace,
        continuations: &[Continuation],
        roots: &mut impl RootSink,
    ) -> RuntimeResult<()> {
        // active frames
        for frame in &self.frames {
            frame
                .visit_roots(program, roots)
                .map_err(|error| self.runtime_error(program, error))?;
        }

        // suspended continuations
        for continuation in continuations {
            continuation
                .visit_roots(program, roots)
                .map_err(|error| self.runtime_error(program, error))?;
        }

        // statics
        for (_id, region, bytes) in statics.iter_regions() {
            Frame::visit_byte_roots(program, program.type_for_id(region.ty), bytes, roots)
                .map_err(|error| self.runtime_error(program, error))?;
        }

        Ok(())
    }

    /// Visit mutable local root slots from active state and suspended continuations.
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
                .ok_or_else(|| self.runtime_error(program, Error::InvalidInstruction))?;
            let bytes = statics
                .bytes_mut(id)
                .ok_or_else(|| self.runtime_error(program, Error::InvalidInstruction))?;

            Frame::visit_byte_root_slots(program, program.type_for_id(region.ty), bytes, visit)
                .map_err(|error| self.runtime_error(program, error))?;
        }

        Ok(())
    }
}
