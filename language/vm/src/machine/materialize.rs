use destack_bytecode::{CodeOffset, Instruction, Opcode};
use destack_program::{
    ActivationImage, CallMode, FrameImage, FrameReturn, FrameSegment, FunctionId, ProgramPoint,
    Word,
};

use crate::diagnostic::{Error, Result};

use super::{Fiber, Frame, FrameMapping, Machine, Return};

impl Machine {
    /// Materialize one captured canonical activation onto an idle fiber.
    pub fn materialize(&mut self, fiber: &mut Fiber, image: &ActivationImage) -> Result<()> {
        if !fiber.is_idle() {
            return Err(Error::execution_active());
        }

        let result = self.materialize_image(fiber, image);
        if result.is_err() {
            fiber.clear();
        } else {
            fiber.context = image.context();
        }

        result
    }

    /// Materialize one canonical activation into physical fiber storage.
    fn materialize_image(&mut self, fiber: &mut Fiber, image: &ActivationImage) -> Result<()> {
        if image.frames().first().copied().map(FrameImage::return_to) != Some(FrameReturn::Root) {
            return Err(Error::invalid_image());
        }
        let root_return = Return::Exit {
            completion: image.completion(),
        };
        let mappings = self.materialize_frames(fiber, image.frames(), root_return)?;
        let range = image.memory();
        let mut bytes = fiber
            .stack
            .memory()
            .read_bytes(range.offset, range.byte_len)
            .map_err(|error| Error::program(error.into()))?;

        self.unpack_frames(fiber, &mappings, &mut bytes)
    }

    /// Rebuild each physical frame from its canonical Program state.
    fn materialize_frames(
        &mut self,
        fiber: &mut Fiber,
        images: &[FrameImage],
        root_return: Return,
    ) -> Result<Vec<FrameMapping>> {
        if images.is_empty() {
            return Err(Error::invalid_image());
        }
        let mut mappings = Vec::with_capacity(images.len());
        let mut byte_offset = 0usize;

        for (index, image) in images.iter().copied().enumerate() {
            let state = image.state();
            let canonical = self
                .program
                .frame_state(state)
                .copied()
                .ok_or_else(Error::invalid_image)?;
            let function = canonical.point.function();
            let pc = self.pc(image.point())?;

            // rebuild the child return from its caller's canonical call site
            let return_to = if index == 0 {
                root_return
            } else {
                self.frame_return(images[index - 1], image.return_to())?
            };

            // resolve the physical bytecode frame
            let linked = self
                .bytecode
                .function(self.program.sections(), function.index())
                .ok_or_else(|| Error::undefined_function(function))?;
            let code = linked
                .code()
                .ok_or_else(|| Error::undefined_function(function))?;

            // allocate physical registers at the canonical frame alignment
            let register_offset = fiber.stack.push_words(linked.register_count())?;
            let mut frame = Frame::new(
                function,
                code,
                register_offset,
                linked.register_count,
                return_to,
            );
            frame.pc = pc;

            // retain the frame and its canonical live value mapping
            fiber.frames.push(frame);
            let mapping = self.frame_mapping(frame, state, byte_offset)?;
            byte_offset = mapping.byte_offset + mapping.layout.byte_len() as usize;
            mappings.push(mapping);
        }

        Ok(mappings)
    }

    /// Rebuild one child return from its caller's canonical call state.
    fn frame_return(&self, parent: FrameImage, return_to: FrameReturn) -> Result<Return> {
        if let FrameReturn::Drop { frame_count } = return_to {
            let pc = self.pc(parent.point())?;

            return Ok(Return::Drop {
                pc,
                caller_state: parent.state(),
                frame_count,
            });
        }
        let state = self
            .program
            .frame_state(parent.state())
            .ok_or_else(Error::invalid_image)?;
        let point = state
            .point
            .operation_point()
            .ok_or_else(Error::invalid_image)?;
        let instruction = self
            .bytecode
            .operation(
                self.program.sections(),
                point.function.index(),
                point.operation,
            )
            .map_err(|_| Error::invalid_image())?
            .ok_or_else(Error::invalid_image)?;

        match return_to {
            FrameReturn::Call if Self::is_returning_call(instruction.opcode()) => {
                self.call_return(point, instruction)
            }
            FrameReturn::Root | FrameReturn::Call => Err(Error::invalid_image()),
            FrameReturn::Drop { .. } => {
                unreachable!("drop returns before decoding callers")
            }
        }
    }

    /// Rebuild one ordinary caller transition.
    fn call_return(&self, point: ProgramPoint, instruction: Instruction<'_>) -> Result<Return> {
        let (_, site) = self.program.call(point).ok_or_else(Error::invalid_image)?;
        if site.mode != CallMode::Return {
            return Err(Error::invalid_image());
        }
        let mut operands = instruction.operands();
        let registers = operands.span().map_err(|_| Error::invalid_image())?;
        let pc = self.pc(point)?;
        let resume = site.resume.get().ok_or_else(Error::invalid_image)?;
        let normal = Some(self.call_offset(point.function, resume)?);
        let unwind = site
            .unwind
            .get()
            .map(|point| self.call_offset(point.function, point))
            .transpose()?;

        Ok(Return::Call {
            pc,
            registers,
            normal,
            unwind,
        })
    }

    /// Return whether one opcode enters a callee that returns to its caller.
    const fn is_returning_call(opcode: Opcode) -> bool {
        matches!(
            opcode,
            Opcode::CALL
                | Opcode::CALL_INDIRECT
                | Opcode::CALL_VIRTUAL
                | Opcode::CALL_DYNAMIC
                | Opcode::INVOKE
                | Opcode::INVOKE_INDIRECT
                | Opcode::INVOKE_VIRTUAL
                | Opcode::INVOKE_DYNAMIC
        )
    }

    /// Resolve one call destination inside its caller function.
    fn call_offset(&self, function: FunctionId, point: ProgramPoint) -> Result<CodeOffset> {
        if point.function != function {
            return Err(Error::invalid_image());
        }

        self.pc(point)
    }

    /// Resolve one canonical program point into a bytecode offset.
    fn pc(&self, point: ProgramPoint) -> Result<CodeOffset> {
        self.bytecode
            .operation_offset(
                self.program.sections(),
                point.function.index(),
                point.operation,
            )
            .ok_or_else(Error::invalid_image)
    }

    /// Copy canonical image bytes into their physical register spans.
    fn unpack_frames(
        &self,
        fiber: &mut Fiber,
        mappings: &[FrameMapping],
        bytes: &mut [u8],
    ) -> Result<()> {
        let expected = mappings.last().map_or(0, |mapping| {
            mapping.byte_offset + mapping.layout.byte_len() as usize
        });
        if bytes.len() != expected {
            return Err(Error::invalid_image());
        }

        // move every frame address from its canonical offset onto the fiber stack
        let mut segments = Vec::new();
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;
            for (slot, span) in slots.iter().zip(spans) {
                let start = mapping.byte_offset + slot.offset as usize;
                let physical = mapping.frame.byte_offset() + span.start.index() * Word::BYTE_LEN;
                segments.push(FrameSegment {
                    source: start..start + slot.byte_len as usize,
                    target: fiber.stack.memory_offset(physical),
                });
            }
        }
        let frames = mappings
            .iter()
            .map(|mapping| (mapping.byte_offset, &mapping.layout));
        self.program
            .relocate_frame_addresses(frames, bytes, |address| {
                let Some(offset) = ActivationImage::decode_frame_address(address) else {
                    return Ok(None);
                };
                let moved = segments
                    .iter()
                    .find_map(|segment| segment.relocate(offset))
                    .ok_or(destack_program::Error::StrayFrameAddress { address })?;

                Ok(Some(moved as u64))
            })?;

        // copy canonical live values into their physical register spans
        for mapping in mappings {
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;
            for (slot, span) in slots.iter().zip(spans) {
                let source = Self::slot_bytes(
                    bytes,
                    mapping.byte_offset + slot.offset as usize,
                    slot.byte_len as usize,
                )?;
                let target = mapping.frame.range(*span) * Word::BYTE_LEN;
                fiber.stack.write_bytes(target, source)?;
            }
        }

        Ok(())
    }

    /// Borrow one canonical slot from mutable image bytes.
    fn slot_bytes(bytes: &mut [u8], offset: usize, byte_len: usize) -> Result<&mut [u8]> {
        let end = offset + byte_len;

        bytes.get_mut(offset..end).ok_or_else(Error::invalid_image)
    }
}
