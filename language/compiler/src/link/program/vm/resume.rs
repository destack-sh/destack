use std::collections::HashMap;

use destack_core::SectionPacker;
use destack_mir as mir;
use destack_program::vm::{
    FrameBinding, FrameEntryBuilder, MoveSlot, ResumeStateBuilder, ResumeTable,
};
use destack_program::{FrameLayout, FrameLayoutId, FrameStateId, FunctionId, ProgramPoint};

use crate::LinkResult;

use super::super::ProgramLinker;
use super::FrameLinker;
use super::lower::SourceBlock;

/// Frame states entered after one invocation completes or unwinds.
#[derive(Debug, Clone, Copy)]
pub(super) struct InvokeStates {
    /// The normal return state.
    pub(super) normal: FrameStateId,
    /// The panic unwind state.
    pub(super) unwind: FrameStateId,
}

/// Normal and unwind edges leaving one invocation block.
struct InvokeEdges<'a> {
    /// The normal return edge.
    normal: &'a mir::BlockTarget,
    /// The panic unwind edge.
    unwind: &'a mir::BlockTarget,
    /// The call result type.
    result: mir::TypeId,
}

/// Linked resume states for VM execution.
#[derive(Debug)]
pub(crate) struct ResumeLinker<'a> {
    /// MIR tree being linked.
    tree: &'a mir::Tree,
    /// Program linker owning dense program id projection.
    program: &'a ProgramLinker,
    /// Build-time resume states.
    states: Vec<PendingResumeState>,
    /// Entry states waiting for lowered block start operations.
    pending_entry_points: Vec<(FrameStateId, mir::BlockId)>,
}

impl<'a> ResumeLinker<'a> {
    /// Create one resume linker.
    pub(crate) fn new(tree: &'a mir::Tree, program: &'a ProgramLinker) -> Self {
        Self {
            tree,
            program,
            states: Vec::new(),
            pending_entry_points: Vec::new(),
        }
    }

    /// Finish the resume table.
    pub(crate) fn finish(self, sections: &mut SectionPacker) -> LinkResult<ResumeTable> {
        let states = self
            .states
            .into_iter()
            .map(|state| state.finish(self.program))
            .collect::<LinkResult<Vec<_>>>()?;

        Ok(ResumeTable::pack(sections, states))
    }

    /// Return the next resume state id.
    fn next_id(&self) -> FrameStateId {
        FrameStateId(self.states.len() as u32)
    }

    /// Add one resume state.
    fn push(&mut self, id: FrameStateId, state: PendingResumeState) {
        debug_assert_eq!(id.0 as usize, self.states.len());
        self.states.push(state);
    }

    /// Build the source resume lookups for one function.
    pub(super) fn build_entries(
        &mut self,
        function_id: mir::FunctionId,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        frames: &mut FrameLinker<'_>,
    ) -> LinkResult<(
        HashMap<mir::BlockId, FrameStateId>,
        HashMap<mir::BlockId, InvokeStates>,
    )> {
        let block_ids = self.tree.get(function_id).blocks().to_vec();
        let mut yield_resume = HashMap::new();
        let mut invokes = HashMap::new();

        // assign resume states to suspension and call edges
        for block_id in block_ids {
            // yield resumes into one single continuation block
            if let Some(resume) = Self::yield_edge(self.tree, block_id) {
                let resume_arguments = resume.arguments(self.tree);
                let received_value = self.received_resume(resume.block, resume_arguments.len())?;
                let frame_state_id = self.append_entry_state(
                    frame_layout_id,
                    frame_layout,
                    liveness,
                    resume.block,
                    resume_arguments,
                    Some(received_value),
                    frames,
                )?;

                yield_resume.insert(block_id, frame_state_id);
            }
            // invokes bind one return value only on their normal edge
            else if let Some(edges) = Self::invoke_edges(self.tree, self.program, block_id)? {
                let normal_arguments = edges.normal.arguments(self.tree);
                let received_value =
                    self.received_result(edges.normal.block, normal_arguments.len(), edges.result)?;
                let normal_state = self.append_entry_state(
                    frame_layout_id,
                    frame_layout,
                    liveness,
                    edges.normal.block,
                    normal_arguments,
                    received_value,
                    frames,
                )?;
                let unwind_arguments = edges.unwind.arguments(self.tree);
                let unwind_state = self.append_entry_state(
                    frame_layout_id,
                    frame_layout,
                    liveness,
                    edges.unwind.block,
                    unwind_arguments,
                    None,
                    frames,
                )?;

                invokes.insert(
                    block_id,
                    InvokeStates {
                        normal: normal_state,
                        unwind: unwind_state,
                    },
                );
            }
        }

        Ok((yield_resume, invokes))
    }

    /// Append states for every lowered instruction point.
    pub(crate) fn append_source_points(
        &mut self,
        program_function: FunctionId,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        source_points: &[SourceBlock],
        frames: &mut FrameLinker<'_>,
    ) -> LinkResult<()> {
        for block in source_points {
            for (pc, source_point) in block.point_by_pc.iter().copied().enumerate() {
                let operation = block.start + pc as u32;
                let point = ProgramPoint::new(program_function, operation);
                let frame_state_id = self.next_id();
                let return_destination =
                    self.return_destination(frame_layout, frames, block.block, source_point)?;

                self.append_state(
                    frame_layout_id,
                    frame_layout,
                    liveness,
                    block.block,
                    Some(point),
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

    /// Resolve pending entry state points from lowered block starts.
    pub(crate) fn resolve_entry_points(
        &mut self,
        function: FunctionId,
        source_points: &[SourceBlock],
    ) -> LinkResult<()> {
        let starts = source_points
            .iter()
            .map(|block| (block.block, block.start))
            .collect::<HashMap<_, _>>();
        let pending = std::mem::take(&mut self.pending_entry_points);

        for (frame_state, block) in pending {
            let start = starts
                .get(&block)
                .copied()
                .ok_or_else(|| self.program.invalid_instruction("resume block start"))?;
            let point = ProgramPoint::new(function, start);
            let state = self
                .states
                .get_mut(frame_state.0 as usize)
                .ok_or_else(|| self.program.invalid_instruction("resume state"))?;

            state.point = Some(point);
        }

        Ok(())
    }

    /// Return the yield edge leaving one block.
    fn yield_edge(tree: &mir::Tree, block: mir::BlockId) -> Option<&mir::BlockTarget> {
        let block = tree.get(block);
        let terminator = tree.get(block.terminator);

        match terminator {
            mir::Terminator::Yield { resume, .. } => Some(resume),
            _ => None,
        }
    }

    /// Return the normal and unwind edges leaving one invocation block.
    fn invoke_edges<'tree>(
        tree: &'tree mir::Tree,
        program: &ProgramLinker,
        block: mir::BlockId,
    ) -> LinkResult<Option<InvokeEdges<'tree>>> {
        let block = tree.get(block);
        let terminator = tree.get(block.terminator);

        let mir::Terminator::Invoke {
            call,
            target,
            unwind,
        } = terminator
        else {
            return Ok(None);
        };
        let result = tree
            .get(call.signature)
            .function_signature_parts()
            .map(|(_, _, result)| result)
            .ok_or_else(|| program.invalid_instruction("invoke signature"))?;

        Ok(Some(InvokeEdges {
            normal: target,
            unwind,
            result,
        }))
    }

    /// Return the resume command received by one yield continuation.
    fn received_resume(
        &self,
        block: mir::BlockId,
        explicit_argument_count: usize,
    ) -> LinkResult<mir::Value> {
        let entry_block = self.tree.get(block);
        let expected_count = explicit_argument_count + 1;

        // require the resume command before any explicit edge arguments
        if entry_block.parameters.len() != expected_count {
            return Err(self
                .program
                .invalid_instruction("yield resume target arity"));
        }

        entry_block
            .parameters
            .first()
            .map(|parameter| parameter.value)
            .ok_or_else(|| self.program.invalid_instruction("yield resume command"))
    }

    /// Return the result received by one normal invocation edge when present.
    fn received_result(
        &self,
        block: mir::BlockId,
        explicit_argument_count: usize,
        result: mir::TypeId,
    ) -> LinkResult<Option<mir::Value>> {
        let entry_block = self.tree.get(block);
        let result = self.tree.repr_type(result);
        let has_result = !matches!(self.tree.get(result), mir::Type::Void);
        let expected_count = explicit_argument_count + usize::from(has_result);

        // require the signature result before any explicit edge arguments
        if entry_block.parameters.len() != expected_count {
            return Err(self.program.invalid_instruction("invoke target arity"));
        }

        if has_result {
            let value = entry_block
                .parameters
                .first()
                .map(|parameter| parameter.value)
                .ok_or_else(|| self.program.invalid_instruction("invoke result"))?;

            Ok(Some(value))
        } else {
            Ok(None)
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
            mir::Instruction::Call { destination, .. } => {
                let Some(destination) = destination else {
                    return Ok(None);
                };
                let destination = frames.move_slot(frame_layout, *destination)?;

                Ok(Some(destination))
            }
            _ => Ok(None),
        }
    }

    /// Append one resume state and its entry bindings.
    fn append_entry_state(
        &mut self,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
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

        // require one argument for every explicit block parameter
        if entry_parameters.len() != arguments.len() {
            return Err(self.program.invalid_instruction("resume target arity"));
        }

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
        self.append_state(
            frame_layout_id,
            frame_layout,
            liveness,
            block,
            None,
            frame_state_id,
            Some(frame_entry),
            None,
            None,
            frames,
        )?;
        self.pending_entry_points.push((frame_state_id, block));

        Ok(frame_state_id)
    }

    /// Append one resume state and its attached tables.
    fn append_state(
        &mut self,
        frame_layout_id: FrameLayoutId,
        frame_layout: &FrameLayout,
        liveness: &mir::FunctionLiveness,
        block: mir::BlockId,
        point: Option<ProgramPoint>,
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
            PendingResumeState {
                point,
                source_point,
                entry: frame_entry,
                return_destination,
            },
        );

        Ok(())
    }
}

/// Build-time resume state before all points have been resolved.
#[derive(Debug)]
struct PendingResumeState {
    /// Program point for this state, when already known.
    point: Option<ProgramPoint>,
    /// Source instruction point within the lowered block, when one exists.
    source_point: Option<u32>,
    /// Entry bindings for block-entry states.
    entry: Option<FrameEntryBuilder>,
    /// Caller return destination for post-call states.
    return_destination: Option<MoveSlot>,
}

impl PendingResumeState {
    /// Finish this pending state.
    fn finish(self, program: &ProgramLinker) -> LinkResult<ResumeStateBuilder> {
        let Some(point) = self.point else {
            return Err(program.invalid_instruction("unresolved resume point"));
        };

        Ok(ResumeStateBuilder {
            point,
            source_point: self.source_point,
            entry: self.entry,
            return_destination: self.return_destination,
        })
    }
}
