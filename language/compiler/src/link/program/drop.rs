use destack_core::SectionPacker;
use destack_program::{DropEntry, DropTable};

use super::ProgramLinker;

/// Link MIR destructors into one program drop table.
pub(crate) struct DropLinker<'a> {
    /// Dense program identity projection.
    program: &'a ProgramLinker,
}

impl<'a> DropLinker<'a> {
    /// Create one drop linker.
    pub(crate) const fn new(program: &'a ProgramLinker) -> Self {
        Self { program }
    }

    /// Link destructors in dense drop id order.
    pub(crate) fn link(&self, sections: &mut SectionPacker) -> DropTable {
        let mut entries = Vec::new();

        // preserve the dense DropId order established by ProgramLinker
        for function in self.program.destructors() {
            entries.push(DropEntry {
                function: self.program.function_id(function),
            });
        }

        DropTable::pack(sections, entries)
    }
}
