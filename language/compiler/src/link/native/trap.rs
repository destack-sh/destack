use tspp_native as native;

use crate::LinkResult;

use super::{Image, NativeLinker};

impl<'a, 'b> NativeLinker<'a, 'b> {
    /// Link every object-local trap to its final native code offset.
    pub(super) fn link_traps(&self, image: &Image) -> LinkResult<Vec<native::CodeTrap>> {
        let mut traps = Vec::new();

        // project object-local offsets into the linked code image
        for (module, object) in self.program.objects() {
            let source = self.source(object)?;
            let sections = source.sections();
            for trap in source.map().traps(sections) {
                let block = self.block(image, *module, trap.block)?;
                let offset = block.offset.checked_add(trap.offset).ok_or_else(|| {
                    self.program
                        .layout_overflow("native trap offset exceeds u32")
                })?;
                traps.push(native::CodeTrap::new(offset, trap.trap));
            }
        }

        traps.sort_unstable_by_key(|trap| (trap.offset, trap.trap.code()));

        Ok(traps)
    }
}
