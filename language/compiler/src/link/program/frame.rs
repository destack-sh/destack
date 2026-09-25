use std::collections::HashMap;

use tspp_mir as mir;
use tspp_program::{object, program};
use tspp_source::ModuleId;

use crate::LinkResult;

use super::ProgramLinker;

/// Link logical object frame states into canonical Program layouts.
#[derive(Debug)]
pub(crate) struct FrameLinker<'a> {
    /// Dense Program identity projection.
    program: &'a ProgramLinker<'a>,
    /// Program frame state ids keyed by module and object-local point.
    states: HashMap<(ModuleId, object::FramePoint), program::FrameStateId>,
}

impl<'a> FrameLinker<'a> {
    /// Create one frame linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self {
            program,
            states: HashMap::new(),
        }
    }

    /// Link every object frame state and canonical layout.
    pub(crate) fn link(&mut self) -> LinkResult<program::FrameTableBuilder> {
        let mut layout_ids = HashMap::<Vec<program::TypeId>, program::FrameLayoutId>::new();
        let mut layouts = Vec::new();
        let mut states = Vec::new();

        // build canonical layouts and collect states before assigning sorted ids
        for (module, object) in self.program.objects() {
            for state in object.frames() {
                let source_types = state.types().collect::<Vec<_>>();
                let types = source_types
                    .iter()
                    .map(|ty| self.program.type_id(*module, *ty))
                    .collect::<Vec<_>>();
                let layout = match layout_ids.get(&types).copied() {
                    Some(layout) => layout,
                    None => {
                        let layout = program::FrameLayoutId(layouts.len() as u32);
                        layouts.push(self.layout(*module, &source_types)?);
                        layout_ids.insert(types, layout);

                        layout
                    }
                };
                let function = self.program.function_id(*module, state.point.function());
                let point = match state.point {
                    object::FramePoint::Entry { .. } => program::FramePoint::entry(function),
                    object::FramePoint::Operation(point) => {
                        let point = program::ProgramPoint::new(function, point.operation);

                        program::FramePoint::operation(point)
                    }
                };
                states.push((point, *module, state.point, layout));
            }
        }

        // assign dense frame state ids in logical point order
        states.sort_unstable_by_key(|(point, _, _, _)| *point);
        let mut linked_states = Vec::with_capacity(states.len());
        for (index, (point, module, source, layout)) in states.into_iter().enumerate() {
            let id = program::FrameStateId(index as u32);

            // reject repeated source points before publishing their ids
            if self.states.insert((module, source), id).is_some() {
                return Err(self.program.invalid_input("duplicate logical frame point"));
            }

            // append the state under its assigned dense id
            linked_states.push(program::FrameState::new(point, layout));
        }

        Ok(program::FrameTableBuilder::new()
            .states(linked_states)
            .layouts(layouts))
    }

    /// Return one linked frame state id.
    pub(crate) fn state(
        &self,
        module: ModuleId,
        point: object::FramePoint,
    ) -> Option<program::FrameStateId> {
        self.states.get(&(module, point)).copied()
    }

    /// Return the number of linked frame states.
    pub(crate) fn len(&self) -> usize {
        self.states.len()
    }

    /// Build one canonical packed frame layout.
    fn layout(
        &self,
        module: ModuleId,
        types: &[mir::TypeId],
    ) -> LinkResult<program::FrameLayoutBuilder> {
        let object = self.program.object(module);
        let mut slots = Vec::with_capacity(types.len());
        let mut byte_len = 0u32;
        let mut frame_alignment = 1u32;

        // pack each live value at its natural alignment
        for ty in types {
            let layout = object
                .layouts()
                .type_layout(*ty)
                .ok_or_else(|| self.program.invalid_input("frame type has no layout"))?;
            byte_len = byte_len.next_multiple_of(layout.alignment);
            frame_alignment = frame_alignment.max(layout.alignment);
            slots.push(program::FrameSlot::new(
                byte_len,
                layout.size,
                self.program.type_id(module, *ty),
            ));
            byte_len += layout.size;
        }
        byte_len = byte_len.next_multiple_of(frame_alignment);

        Ok(program::FrameLayoutBuilder::new(byte_len, frame_alignment).slots(slots))
    }
}
