use destack_program::DropEntry;

use super::ProgramLinker;

/// Link MIR destructors into one program drop table.
pub(crate) struct DropLinker<'a> {
    /// Dense program identity projection.
    program: &'a ProgramLinker<'a>,
}

impl<'a> DropLinker<'a> {
    /// Create one drop linker.
    pub(crate) const fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link destructors in dense drop id order.
    pub(crate) fn link(&self) -> Vec<DropEntry> {
        let mut entries = Vec::new();

        // preserve the dense DropId order established by ProgramLinker
        for (module, function) in self.program.destructors() {
            entries.push(DropEntry {
                function: self.program.function_id(module, function),
            });
        }

        entries
    }
}
