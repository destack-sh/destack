use destack_program as program;
use program::vm::{AllocationBranch, FunctionCode, Instruction, Op, SliceAllocationBranch};

use crate::diagnostic::Error;

use super::Activation;

/// Profiled allocation operation shape.
enum AllocationShape {
    /// One object allocation.
    Object,
    /// One slice backing allocation.
    Slice {
        /// Frame offset containing the slice length.
        length: u32,
    },
}

impl Activation<'_> {
    /// Record one explicit profile counter at a lowered instruction when profiling.
    pub(crate) fn record_profile_counter_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
    ) -> Result<(), Error> {
        // skip inert profile execution
        let Some(profile) = self.profile.as_deref_mut() else {
            return Ok(());
        };

        // resolve the executable counter site
        let point = Self::program_point_at(function, block, pc)?;
        let Some(site) = self.program.sites().counter(self.program.sections(), point) else {
            return Err(Error::invalid_instruction());
        };

        // update the linked counter row
        profile.increment_counter(site.counter);

        Ok(())
    }

    /// Record one explicit profile sample at a lowered instruction when profiling.
    pub(crate) fn record_profile_sample_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
        key: u64,
    ) -> Result<(), Error> {
        // skip inert profile execution
        let Some(profile) = self.profile.as_deref_mut() else {
            return Ok(());
        };

        // resolve the executable sample site
        let point = Self::program_point_at(function, block, pc)?;
        let Some(site) = self.program.sites().sample(self.program.sections(), point) else {
            return Err(Error::invalid_instruction());
        };

        // update the linked sample row
        profile.record_sample(site.counter, key);

        Ok(())
    }

    /// Record one call site at a lowered instruction when profiling.
    pub(crate) fn record_profile_call_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
    ) -> Result<(), Error> {
        // skip inert profile execution
        let Some(profile) = self.profile.as_deref_mut() else {
            return Ok(());
        };

        // resolve the executable call site
        let point = Self::program_point_at(function, block, pc)?;
        let Some((site, _)) = self.program.sites().call(self.program.sections(), point) else {
            return Err(Error::invalid_instruction());
        };

        // update the linked call row
        profile.record_call(site);

        Ok(())
    }

    /// Record one completed non-transfer instruction when profiling.
    pub(crate) fn record_profile_step_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
        instruction: &Instruction,
    ) -> Result<(), Error> {
        // skip inert profile execution
        if !self.has_profile() {
            return Ok(());
        }

        // record operations that complete on the fallthrough path
        self.record_profile_allocation_at(function, block, pc, instruction)?;

        Ok(())
    }

    /// Record one successful jump transfer when profiling.
    pub(crate) fn record_profile_jump_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
        instruction: &Instruction,
        target: u32,
    ) -> Result<(), Error> {
        // skip inert profile execution
        if !self.has_profile() {
            return Ok(());
        }

        // record the taken control-flow edge
        self.record_profile_edge_at(function, block, pc, target)?;

        // record successful fallible allocation branches only
        if self.allocation_branch_succeeded(instruction, target) {
            self.record_profile_allocation_at(function, block, pc, instruction)?;
        }

        Ok(())
    }

    /// Record one captured continuation when profiling.
    pub(crate) fn record_profile_continuation_capture(
        &mut self,
        frame_state: program::FrameStateId,
    ) -> Result<(), Error> {
        // skip inert profile execution
        if !self.has_profile() {
            return Ok(());
        }

        // resolve the executable continuation site
        let Some((site, _)) = self
            .program
            .sites()
            .continuation_state(self.program.sections(), frame_state)
        else {
            return Err(Error::invalid_instruction());
        };

        let Some(profile) = self.profile.as_deref_mut() else {
            return Ok(());
        };

        // update the linked continuation row
        profile.record_continuation_capture(site);

        Ok(())
    }

    /// Record one control-flow edge transfer when profiling.
    fn record_profile_edge_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
        target: u32,
    ) -> Result<(), Error> {
        // resolve the source and target executable points
        let source = Self::program_point_at(function, block, pc)?;
        let target = function
            .operation_at(target, 0)
            .ok_or(Error::invalid_instruction())?;
        let target = program::ProgramPoint::new(function.function.function, target);

        // resolve the executable edge site
        let Some((site, _)) = self
            .program
            .sites()
            .edge(self.program.sections(), source, target)
        else {
            return Err(Error::invalid_instruction());
        };

        let Some(profile) = self.profile.as_deref_mut() else {
            return Ok(());
        };

        // update the linked edge row
        profile.record_edge(site);

        Ok(())
    }

    /// Return whether one allocation branch jumped to its success edge.
    fn allocation_branch_succeeded(&self, instruction: &Instruction, target: u32) -> bool {
        match instruction.op {
            // object branch allocations
            Op::AllocateHeapZeroedBranch
            | Op::AllocateHeapUninitBranch
            | Op::AllocateSharedHeapZeroedBranch
            | Op::AllocateSharedHeapUninitBranch => {
                let branch = self.side::<AllocationBranch>(instruction);

                branch.success.target == target
            }

            // slice branch allocations
            Op::AllocateSliceZeroedBranch
            | Op::AllocateSliceUninitBranch
            | Op::AllocateSharedSliceZeroedBranch
            | Op::AllocateSharedSliceUninitBranch => {
                let branch = self.side::<SliceAllocationBranch>(instruction);

                branch.success.target == target
            }
            _ => false,
        }
    }

    /// Record one successful allocation instruction when profiling.
    fn record_profile_allocation_at(
        &mut self,
        function: FunctionCode<'_>,
        block: u32,
        pc: usize,
        instruction: &Instruction,
    ) -> Result<(), Error> {
        // ignore non-allocation instructions
        let Some(shape) = self.allocation_shape(instruction) else {
            return Ok(());
        };

        // resolve the executable allocation site
        let point = Self::program_point_at(function, block, pc)?;
        let Some((site_id, site)) = self
            .program
            .sites()
            .allocation(self.program.sections(), point)
        else {
            return Err(Error::invalid_instruction());
        };

        // compute the allocated byte count from linked storage layout
        let layout = self
            .program
            .layout_by_id(site.storage_layout)
            .ok_or(Error::invalid_instruction())?;
        let bytes = match shape {
            AllocationShape::Object => u64::from(layout.size),
            AllocationShape::Slice { length } => {
                let length = self.load_cell_at(length).as_u64();

                u64::from(layout.size) * length
            }
        };

        // update the linked allocation row
        let Some(profile) = self.profile.as_deref_mut() else {
            return Ok(());
        };
        profile.record_allocation(site_id, bytes);

        Ok(())
    }

    /// Return the allocation operation represented by one instruction.
    fn allocation_shape(&self, instruction: &Instruction) -> Option<AllocationShape> {
        let shape = match instruction.op {
            // object allocations
            Op::AllocateHeapZeroed
            | Op::AllocateHeapUninit
            | Op::AllocateHeapSmallNoscanZeroed
            | Op::AllocateHeapSmallNoscanUninit
            | Op::AllocateHeapSmallScanZeroed
            | Op::AllocateHeapSmallScanUninit
            | Op::AllocateHeapSmallSharedEdgeZeroed
            | Op::AllocateHeapSmallSharedEdgeUninit
            | Op::AllocateSharedHeapZeroed
            | Op::AllocateSharedHeapUninit
            | Op::AllocateSharedHeapSmallZeroed
            | Op::AllocateSharedHeapSmallUninit
            | Op::AllocateHeapZeroedBranch
            | Op::AllocateHeapUninitBranch
            | Op::AllocateSharedHeapZeroedBranch
            | Op::AllocateSharedHeapUninitBranch => AllocationShape::Object,

            // slice allocations with inline length operand
            Op::AllocateSliceZeroed
            | Op::AllocateSliceUninit
            | Op::AllocateSharedSliceZeroed
            | Op::AllocateSharedSliceUninit => AllocationShape::Slice {
                length: instruction.b,
            },

            // slice allocations with side-table branch payload
            Op::AllocateSliceZeroedBranch
            | Op::AllocateSliceUninitBranch
            | Op::AllocateSharedSliceZeroedBranch
            | Op::AllocateSharedSliceUninitBranch => {
                let branch = self.side::<SliceAllocationBranch>(instruction);

                AllocationShape::Slice {
                    length: branch.length,
                }
            }
            _ => return None,
        };

        Some(shape)
    }
}
