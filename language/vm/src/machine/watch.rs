use destack_program as program;
use program::vm::{FunctionCode, Instruction, Op, ProjectionId};

use crate::diagnostic::Error;

use super::Activation;

impl Activation<'_> {
    /// Return one stop at the lowered instruction when present.
    pub(crate) fn stop_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
    ) -> Result<Option<(program::StopReason, program::FrameStateId)>, Error> {
        // skip inert stop-point execution
        let Some(stop_points) = self.stop_points else {
            return Ok(None);
        };
        if stop_points.is_empty() {
            return Ok(None);
        }

        // map lowered code coordinates to the executable program point
        let point = Self::program_point_at(function, block, pc)?;

        // skip one retained stop when continue resumes at the same point
        let resume_skip = self.resume_skip.take();
        let reason = stop_points.reason_at(point, resume_skip);
        let Some(reason) = reason else {
            return Ok(None);
        };

        // resolve the resumable frame state for the stop
        let frame_state = self
            .program
            .frame_state_at(point)
            .ok_or(Error::invalid_instruction())?;

        Ok(Some((reason, frame_state)))
    }

    /// Return one watchpoint stop after a memory operation when present.
    pub(crate) fn watch_memory_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
        next_pc: usize,
    ) -> Result<Option<(program::StopReason, program::FrameStateId)>, Error> {
        // skip inert watchpoint execution
        let Some(watch_points) = self.watch_points else {
            return Ok(None);
        };
        if watch_points.is_empty() {
            return Ok(None);
        }

        // map lowered code coordinates to the memory site
        let point = Self::program_point_at(function, block, pc)?;
        let sites = self.program.sites().memory(self.program.sections(), point);
        if sites.is_empty() {
            return Ok(None);
        }

        // decode the touched range only when a watchpoint needs it
        let range = if watch_points.requires_memory_range() {
            self.memory_range_at(function, block, pc)?
        } else {
            None
        };

        // select the first watchpoint matching one memory site
        for site in sites {
            let Some(watchpoint_id) = watch_points.watchpoint_at(*site, range) else {
                continue;
            };

            // resolve the frame state after the memory operation
            let resume_point = Self::program_point_at(function, block, next_pc)?;
            let frame_state = self
                .program
                .frame_state_at(resume_point)
                .ok_or(Error::invalid_instruction())?;

            return Ok(Some((
                program::StopReason::Watchpoint {
                    watchpoint_id,
                    point,
                },
                frame_state,
            )));
        }

        Ok(None)
    }

    /// Return the executed memory range for one lowered memory operation.
    fn memory_range_at(
        &self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
    ) -> Result<Option<program::MemoryRange>, Error> {
        // resolve the lowered memory instruction
        let instruction = function
            .instruction_at(block, pc)
            .ok_or(Error::invalid_instruction())?;
        let Some(cell_layout) = instruction.op.memory_cell_layout() else {
            return Ok(None);
        };

        // scalar memory operations carry their byte width in the opcode
        if let Some(byte_len) = instruction.op.memory_byte_len() {
            let range = self.scalar_memory_range(instruction, cell_layout, byte_len)?;

            return Ok(Some(range));
        }

        // aggregate memory operations carry their width in the projection row
        let Some(range) = self.aggregate_memory_range(instruction, cell_layout)? else {
            return Ok(None);
        };

        Ok(Some(range))
    }

    /// Return the executed memory range for one scalar memory operation.
    fn scalar_memory_range(
        &self,
        instruction: &Instruction,
        cell_layout: program::CellLayout,
        byte_len: usize,
    ) -> Result<program::MemoryRange, Error> {
        // decode the pointer operand shape
        let range = match instruction.op {
            // frame value loads address the source slot through b
            Op::LoadFrameValueU8
            | Op::LoadFrameValueI8
            | Op::LoadFrameValueU16
            | Op::LoadFrameValueI16
            | Op::LoadFrameValueU32
            | Op::LoadFrameValueI32
            | Op::LoadFrameValue64 => {
                self.frame_value_memory_range(instruction.b, instruction.c, byte_len)
            }

            // frame value stores address the destination slot through a
            Op::StoreFrameValue8
            | Op::StoreFrameValue16
            | Op::StoreFrameValue32
            | Op::StoreFrameValue64 => {
                self.frame_value_memory_range(instruction.a, instruction.c, byte_len)
            }

            // pointer loads address the source pointer through b
            Op::LoadHeapU8
            | Op::LoadHeapI8
            | Op::LoadHeapU16
            | Op::LoadHeapI16
            | Op::LoadHeapU32
            | Op::LoadHeapI32
            | Op::LoadHeap64
            | Op::LoadSharedHeapU8
            | Op::LoadSharedHeapI8
            | Op::LoadSharedHeapU16
            | Op::LoadSharedHeapI16
            | Op::LoadSharedHeapU32
            | Op::LoadSharedHeapI32
            | Op::LoadSharedHeap64
            | Op::LoadRawU8
            | Op::LoadRawI8
            | Op::LoadRawU16
            | Op::LoadRawI16
            | Op::LoadRawU32
            | Op::LoadRawI32
            | Op::LoadRaw64
            | Op::LoadStackU8
            | Op::LoadStackI8
            | Op::LoadStackU16
            | Op::LoadStackI16
            | Op::LoadStackU32
            | Op::LoadStackI32
            | Op::LoadStack64
            | Op::LoadFrameU8
            | Op::LoadFrameI8
            | Op::LoadFrameU16
            | Op::LoadFrameI16
            | Op::LoadFrameU32
            | Op::LoadFrameI32
            | Op::LoadFrame64
            | Op::LoadStaticU8
            | Op::LoadStaticI8
            | Op::LoadStaticU16
            | Op::LoadStaticI16
            | Op::LoadStaticU32
            | Op::LoadStaticI32
            | Op::LoadStatic64 => self.cell_memory_range(
                cell_layout,
                instruction.b,
                instruction.c as usize,
                byte_len,
            )?,

            // pointer stores address the destination pointer through a
            _ => self.cell_memory_range(
                cell_layout,
                instruction.a,
                instruction.c as usize,
                byte_len,
            )?,
        };

        Ok(range)
    }

    /// Return the executed memory range for one aggregate memory operation.
    fn aggregate_memory_range(
        &self,
        instruction: &Instruction,
        cell_layout: program::CellLayout,
    ) -> Result<Option<program::MemoryRange>, Error> {
        // decode the aggregate pointer operand
        let pointer = match instruction.op {
            // aggregate loads address the source pointer through b
            Op::LoadHeapAggregate
            | Op::LoadSharedHeapAggregate
            | Op::LoadRawAggregate
            | Op::LoadStackAggregate
            | Op::LoadFrameAggregate
            | Op::LoadStaticAggregate => instruction.b,

            // aggregate stores address the destination pointer through a
            Op::StoreHeapAggregate
            | Op::StoreSharedHeapAggregate
            | Op::StoreRawAggregate
            | Op::StoreStackAggregate
            | Op::StoreFrameAggregate
            | Op::StoreStaticAggregate => instruction.a,

            _ => return Ok(None),
        };

        // decode the aggregate byte range from the projection row
        let projection = self.projection(ProjectionId(instruction.c));
        let byte_offset = projection.byte_offset();
        let byte_len = projection.byte_len();
        let range = self.cell_memory_range(cell_layout, pointer, byte_offset, byte_len)?;

        Ok(Some(range))
    }

    /// Return the executed memory range from one pointer cell.
    fn cell_memory_range(
        &self,
        cell_layout: program::CellLayout,
        pointer: u32,
        byte_offset: usize,
        byte_len: usize,
    ) -> Result<program::MemoryRange, Error> {
        // load the pointer cell before decoding its address space
        let pointer = self.load_cell_at(pointer);
        let range = match cell_layout {
            program::CellLayout::HeapReference => {
                let start = pointer.as_heap_reference().offset() + byte_offset;

                program::MemoryRange::local_heap(start as u64, byte_len as u64)
            }
            program::CellLayout::SharedHeapReference => {
                let start = pointer.as_shared_heap_reference().offset() + byte_offset;

                program::MemoryRange::shared_heap(start as u64, byte_len as u64)
            }
            program::CellLayout::Address => {
                let start = pointer.as_address() + byte_offset;

                program::MemoryRange::address(start as u64, byte_len as u64)
            }
            program::CellLayout::StackPointer => {
                let start = pointer.as_stack_pointer().address() + byte_offset;

                program::MemoryRange::stack(start as u64, byte_len as u64)
            }
            program::CellLayout::FramePointer => {
                let start = pointer.as_frame_pointer().address() + byte_offset;

                program::MemoryRange::frame(start as u64, byte_len as u64)
            }
            program::CellLayout::GlobalAddress => {
                let address = pointer.as_global_address();
                let global = self
                    .program
                    .global(address.global())
                    .ok_or(Error::invalid_instruction())?;
                let start = address.byte_offset() + byte_offset;

                program::MemoryRange::global(
                    address.global(),
                    global.location,
                    start as u64,
                    byte_len as u64,
                )
            }
            _ => return Err(Error::invalid_instruction()),
        };

        Ok(range)
    }

    /// Return the executed memory range from one frame value offset.
    fn frame_value_memory_range(
        &self,
        base: u32,
        byte_offset: u32,
        byte_len: usize,
    ) -> program::MemoryRange {
        // frame values are addressed relative to the active frame
        let pointer = self.frame_pointer_at(base).add_bytes(byte_offset as usize);

        program::MemoryRange::frame(pointer.address() as u64, byte_len as u64)
    }
}
