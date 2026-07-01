use std::collections::HashMap;

use destack_core::SectionPacker;
use destack_mir as mir;
use destack_program::vm::{
    FrameBinding, FrameEntryBuilder, MoveSlot, ProgramPoint, ResumeStateBuilder, ResumeTable,
};
use destack_program::{FrameLayout, FrameLayoutId, FrameStateId, FunctionId};

use crate::LinkResult;

use super::super::ProgramLinker;
use super::FrameLinker;

/// Linked resume states for VM execution.
#[derive(Debug)]
pub(crate) struct ResumeLinker<'a> {
    /// MIR tree being linked.
    tree: &'a mir::Tree,
    /// Program linker owning dense program id projection.
    program: &'a ProgramLinker,
    /// Build-time resume states.
    states: Vec<ResumeStateBuilder>,
    /// Resume state id by lowered program point.
    state_by_point: HashMap<ProgramPoint, FrameStateId>,
}

impl<'a> ResumeLinker<'a> {
    /// Create one resume linker.
    pub(crate) fn new(tree: &'a mir::Tree, program: &'a ProgramLinker) -> Self {
        Self {
            tree,
            program,
            states: Vec::new(),
            state_by_point: HashMap::new(),
        }
    }

    /// Finish the resume table.
    pub(crate) fn finish(self, sections: &mut SectionPacker) -> ResumeTable {
        ResumeTable::pack(sections, self.states)
    }

    /// Return the next resume state id.
    fn next_id(&self) -> FrameStateId {
        FrameStateId(self.states.len() as u32)
    }

    /// Add one resume state.
    fn push(&mut self, id: FrameStateId, state: ResumeStateBuilder) {
        debug_assert_eq!(id.0 as usize, self.states.len());
        self.state_by_point.insert(state.point, id);
        self.states.push(state);
    }

    /// Build the source resume lookups for one function.
    pub(crate) fn build_entries(
        &mut self,
        function_id: mir::FunctionId,
        program_function: FunctionId,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        block_index_by_id: &HashMap<mir::BlockId, usize>,
        frames: &mut FrameLinker<'_>,
    ) -> LinkResult<(
        HashMap<mir::BlockId, FrameStateId>,
        HashMap<mir::BlockId, FrameStateId>,
    )> {
        let block_ids = self.tree.get(function_id).blocks().to_vec();
        let mut yield_resume = HashMap::new();
        let mut call_resume = HashMap::new();

        // assign resume states to suspension and call edges
        for block_id in block_ids {
            // yield resumes into one single continuation block
            if let Some((resume, resume_arguments)) = self.yield_edge(block_id) {
                let received_value = self.received_value(resume, resume_arguments.len())?;
                let frame_state_id = self.append_entry_state(
                    program_function,
                    frame_layout_id,
                    frame_layout,
                    liveness,
                    block_index_by_id,
                    resume,
                    &resume_arguments,
                    received_value,
                    frames,
                )?;

                yield_resume.insert(block_id, frame_state_id);
            }
            // call continuations may bind one leading return value
            else if let Some((target, arguments)) = self.call_edge(block_id) {
                let received_value = self.received_value(target, arguments.len())?;
                let target_state_id = self.append_entry_state(
                    program_function,
                    frame_layout_id,
                    frame_layout,
                    liveness,
                    block_index_by_id,
                    target,
                    &arguments,
                    received_value,
                    frames,
                )?;

                call_resume.insert(block_id, target_state_id);
            }
        }

        Ok((yield_resume, call_resume))
    }

    /// Append states for every lowered instruction point.
    pub(crate) fn append_source_points(
        &mut self,
        program_function: FunctionId,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        source_points: &[(mir::BlockId, Vec<u32>)],
        frames: &mut FrameLinker<'_>,
    ) -> LinkResult<()> {
        for (block_index, (block, point_by_pc)) in source_points.iter().enumerate() {
            for (pc, source_point) in point_by_pc.iter().copied().enumerate() {
                let point = ProgramPoint::new(program_function, block_index as u32, pc as u32);
                let frame_state_id = self.next_id();
                let return_destination =
                    self.return_destination(frame_layout, frames, *block, source_point)?;

                self.append_state(
                    frame_layout_id,
                    frame_layout,
                    liveness,
                    *block,
                    point,
                    frame_state_id,
                    None,
                    return_destination,
                    Some(source_point),
                    frames,
                )?;
            }
        }

        Ok(())
    }

    /// Return the yield edge leaving one block.
    fn yield_edge(&self, block: mir::BlockId) -> Option<(mir::BlockId, Vec<mir::Value>)> {
        let block = self.tree.get(block);
        let terminator = self.tree.get(block.terminator);

        match terminator {
            mir::Terminator::Yield { resume, .. } => {
                Some((resume.block, resume.arguments(self.tree).to_vec()))
            }
            _ => None,
        }
    }

    /// Return the call continuation edge leaving one block.
    fn call_edge(&self, block: mir::BlockId) -> Option<(mir::BlockId, Vec<mir::Value>)> {
        let block = self.tree.get(block);
        let terminator = self.tree.get(block.terminator);

        match terminator {
            mir::Terminator::Call { target, .. }
            | mir::Terminator::CallIndirect { target, .. }
            | mir::Terminator::CallVirtual { target, .. }
            | mir::Terminator::CallDynamic { target, .. } => {
                Some((target.block, target.arguments(self.tree).to_vec()))
            }
            _ => None,
        }
    }

    /// Return the call destination before one source instruction point.
    fn return_destination(
        &self,
        frame_layout: &FrameLayout,
        frames: &FrameLinker<'_>,
        block: mir::BlockId,
        source_point: u32,
    ) -> LinkResult<Option<MoveSlot>> {
        // block entry has no preceding call
        if source_point == 0 {
            return Ok(None);
        }

        // inspect the source instruction immediately before this point
        let block = self.tree.get(block);
        let instruction_index = source_point as usize - 1;
        let Some(instruction_id) = block.instructions.get(instruction_index).copied() else {
            return Ok(None);
        };
        let instruction = self.tree.get(instruction_id);

        match instruction {
            mir::Instruction::Call { destination, .. }
            | mir::Instruction::CallVirtual { destination, .. }
            | mir::Instruction::CallDynamic { destination, .. }
            | mir::Instruction::CallIndirect { destination, .. } => {
                let Some(destination) = destination else {
                    return Ok(None);
                };
                let destination = frames.move_slot(frame_layout, *destination)?;

                Ok(Some(destination))
            }
            _ => Ok(None),
        }
    }

    /// Return the leading received value for one edge when present.
    fn received_value(
        &self,
        block: mir::BlockId,
        explicit_argument_count: usize,
    ) -> LinkResult<Option<mir::Value>> {
        let entry_block = self.tree.get(block);

        // block edges bind the received value before explicit arguments
        if entry_block.parameters.len() == explicit_argument_count + 1 {
            return Ok(entry_block
                .parameters
                .first()
                .map(|parameter| parameter.value));
        }

        Ok(None)
    }

    /// Append one resume state and its entry bindings.
    #[allow(clippy::too_many_arguments)]
    fn append_entry_state(
        &mut self,
        function: FunctionId,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        block_index_by_id: &HashMap<mir::BlockId, usize>,
        block: mir::BlockId,
        arguments: &[mir::Value],
        received_value: Option<mir::Value>,
        frames: &mut FrameLinker<'_>,
    ) -> LinkResult<FrameStateId> {
        let entry_block = self.tree.get(block);
        let entry_parameters = if received_value.is_some() {
            &entry_block.parameters[1..]
        } else {
            &entry_block.parameters[..]
        };
        let bindings = entry_parameters
            .iter()
            .zip(arguments.iter())
            .map(|(parameter, argument)| {
                let destination = parameter.value;

                let source = frame_layout
                    .value_slot_id(argument.0)
                    .ok_or_else(|| self.program.invalid_instruction("resume argument slot"))?;
                let destination = frame_layout
                    .value_slot_id(destination.0)
                    .ok_or_else(|| self.program.invalid_instruction("resume parameter slot"))?;

                Ok(FrameBinding {
                    source,
                    destination,
                })
            })
            .collect::<LinkResult<Vec<_>>>()?;

        let frame_entry = FrameEntryBuilder {
            bindings,
            received_value: if let Some(value) = received_value {
                let slot = frame_layout
                    .value_slot_id(value.0)
                    .ok_or_else(|| self.program.invalid_instruction("resume received slot"))?;

                Some(slot)
            } else {
                None
            },
        };

        let frame_state_id = self.next_id();
        let block_index = block_index_by_id
            .get(&block)
            .copied()
            .ok_or_else(|| self.program.invalid_instruction("resume block index"))?;
        let point = ProgramPoint::new(function, block_index as u32, 0);

        self.append_state(
            frame_layout_id,
            frame_layout,
            liveness,
            block,
            point,
            frame_state_id,
            Some(frame_entry),
            None,
            None,
            frames,
        )?;

        Ok(frame_state_id)
    }

    /// Append one resume state and its attached tables.
    #[allow(clippy::too_many_arguments)]
    fn append_state(
        &mut self,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::BlockId,
        point: ProgramPoint,
        frame_state: FrameStateId,
        frame_entry: Option<FrameEntryBuilder>,
        return_destination: Option<MoveSlot>,
        source_point: Option<u32>,
        frames: &mut FrameLinker<'_>,
    ) -> LinkResult<()> {
        frames.append_materialization(
            frame_layout_id,
            frame_layout,
            liveness,
            block,
            frame_state,
            frame_entry.as_ref(),
            source_point,
        )?;
        self.push(
            frame_state,
            ResumeStateBuilder {
                point,
                source_point,
                entry: frame_entry,
                return_destination,
            },
        );

        Ok(())
    }
}
