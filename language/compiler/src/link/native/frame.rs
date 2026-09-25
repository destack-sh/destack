use tspp_native as native;
use tspp_program::FrameStateId;
use tspp_source::ModuleId;

use crate::LinkResult;

use super::{Image, NativeLinker};

/// One native frame map before final return-address ordering.
#[derive(Debug)]
pub(super) struct PhysicalFrame {
    /// Object containing the physical map.
    pub(super) module: ModuleId,
    /// Object-local physical map identity.
    pub(super) source: u32,
    /// Linked return-address byte offset.
    pub(super) return_offset: u32,
    /// Frame-pointer position relative to the frame marker.
    pub(super) frame_pointer_offset: i32,
    /// Canonical Program frame-state identity.
    pub(super) state: FrameStateId,
    /// Canonical values in Program frame-slot order.
    pub(super) values: Vec<native::FrameValueBuilder>,
}

impl<'a, 'b> NativeLinker<'a, 'b> {
    /// Link and order every physical frame map.
    pub(super) fn link_maps(
        &self,
        image: &mut Image,
    ) -> LinkResult<(Vec<native::FrameMapBuilder>, Vec<u8>)> {
        let mut frames = Vec::new();
        let mut constants = Vec::new();

        // project object-local maps into canonical Program states
        for (module, object) in self.program.objects() {
            let source = self.source(object)?;
            let sections = source.sections();
            let map = source.map();
            let constant_start = i32::try_from(constants.len()).map_err(|_| {
                self.program
                    .layout_overflow("native frame constants exceed i32")
            })?;
            constants.extend_from_slice(map.constants(sections));

            for (index, frame) in map.frames(sections).iter().copied().enumerate() {
                let block = self.block(image, *module, frame.block)?;
                let return_offset =
                    block
                        .offset
                        .checked_add(frame.return_offset)
                        .ok_or_else(|| {
                            self.program
                                .layout_overflow("native return address exceeds u32")
                        })?;
                let source_state = object.frames().get(frame.state as usize).ok_or_else(|| {
                    self.program
                        .invalid_input("native frame map references an unknown frame state")
                })?;
                let state = self
                    .frames
                    .state(*module, source_state.point)
                    .ok_or_else(|| {
                        self.program
                            .invalid_input("native frame state was not linked")
                    })?;
                let values =
                    map.values(sections, frame)
                        .iter()
                        .copied()
                        .map(|value| {
                            let locations = map.locations(sections, value).iter().copied().map(
                                |mut location| {
                                    if location.source == native::FrameSource::Constant {
                                        location.source_offset += constant_start;
                                    }

                                    location
                                },
                            );

                            native::FrameValueBuilder::new().locations(locations)
                        })
                        .collect();
                frames.push(PhysicalFrame {
                    module: *module,
                    source: index as u32,
                    return_offset,
                    frame_pointer_offset: frame.frame_pointer_offset,
                    state,
                    values,
                });
            }
        }

        // assign dense physical ids in return-address order
        frames.sort_unstable_by_key(|frame| frame.return_offset);
        let mut builders = Vec::with_capacity(frames.len());
        for (index, frame) in frames.into_iter().enumerate() {
            if image
                .frames
                .insert((frame.module, frame.source), index as u32)
                .is_some()
            {
                return Err(self.program.invalid_input("duplicate native frame map"));
            }
            builders.push(
                native::FrameMapBuilder::new(
                    frame.return_offset,
                    frame.frame_pointer_offset,
                    frame.state.0,
                )
                .values(frame.values),
            );
        }

        Ok((builders, constants))
    }
}
