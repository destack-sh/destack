use destack_bytecode::RegisterSpan;
use destack_program::{Completion, Word};

use crate::diagnostic::{Error, Result};

use super::{Fiber, FrameMapping, Machine, Return};

impl Machine {
    /// Split frames above one detach boundary onto a fresh parked fiber.
    ///
    /// The mounted fiber is left untouched until the suffix is complete, so a
    /// failed split never amputates live frames.
    pub(crate) fn split(
        &mut self,
        fiber: &mut Fiber,
        boundary: usize,
        wake_to: RegisterSpan,
    ) -> Result<()> {
        // the boundary thunk frame and everything above move to the suffix
        let base = fiber
            .frames
            .get(boundary)
            .copied()
            .ok_or_else(Error::invalid_split)?
            .byte_offset();
        let byte_len = fiber
            .stack
            .byte_len()
            .checked_sub(base)
            .ok_or_else(Error::invalid_split)?;

        // the suffix root exits its own fiber where the boundary returned
        let mut frames = fiber.frames[boundary..].to_vec();
        frames[0].return_to = Return::Exit {
            completion: Completion::Return,
        };
        for frame in &mut frames {
            frame.register_offset -= base / Word::BYTE_LEN;
        }

        // copy the live suffix bytes onto the fresh stack
        let mut suffix = self.reserve_fiber(fiber.stack.memory())?;
        suffix.stack.grow(byte_len)?;
        suffix
            .stack
            .copy_from(0, &fiber.stack, base, byte_len)
            .map_err(|_| Error::invalid_split())?;
        suffix.frames = frames;
        suffix.context = fiber.context;
        suffix.wake_to = Some(wake_to);
        suffix.current = fiber.current;

        // rebase moved frame addresses at the exact park state
        let active = suffix
            .frames
            .last()
            .copied()
            .ok_or_else(Error::invalid_split)?;
        let active_state = self.frame_state_at(active, active.pc)?;
        let mappings = self.frame_mappings(&suffix.frames, active_state)?;
        let source_start = fiber.stack.memory_offset(base);
        let source_end = fiber.stack.memory_offset(fiber.stack.byte_len());
        self.rebase_frame_addresses(&mut suffix, source_start..source_end, &mappings)?;

        // release the moved region and hand the suffix to the worker
        fiber.frames.truncate(boundary);
        fiber.stack.truncate(base);
        fiber.detached.push(suffix);

        Ok(())
    }

    /// Rewrite moved frame addresses from the source region onto the suffix stack.
    fn rebase_frame_addresses(
        &self,
        suffix: &mut Fiber,
        source: std::ops::Range<usize>,
        mappings: &[FrameMapping],
    ) -> Result<()> {
        let target_start = suffix.stack.memory_offset(0);

        // visit every live slot through its canonical Program type
        for mapping in mappings {
            let mut is_valid = true;
            let (slots, spans) = mapping.values(&self.program, self.bytecode)?;
            for (slot, span) in slots.iter().zip(spans) {
                let byte_offset = mapping.frame.range(*span) * Word::BYTE_LEN;
                let bytes = suffix
                    .stack
                    .bytes_mut(byte_offset, slot.byte_len as usize)
                    .map_err(|_| Error::invalid_split())?;
                self.program
                    .visit_byte_frame_addresses(slot.ty, bytes, &mut |address| {
                        if !is_valid {
                            return Ok(());
                        }
                        let Some(word) = Word::from_bytes(address) else {
                            is_valid = false;

                            return Ok(());
                        };
                        if word.is_nullish() {
                            return Ok(());
                        }

                        // frame references never cross the task boundary
                        let offset = word.bits() as usize;
                        if !source.contains(&offset) {
                            is_valid = false;

                            return Ok(());
                        }
                        let rebased = target_start + (offset - source.start);
                        let word = Word::from_bits(rebased as u64);
                        address.copy_from_slice(&word.to_bytes());

                        Ok(())
                    })
                    .map_err(Error::program)?;
            }
            if !is_valid {
                return Err(Error::invalid_split());
            }
        }

        Ok(())
    }
}
