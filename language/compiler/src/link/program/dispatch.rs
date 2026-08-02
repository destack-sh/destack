use destack_core::SectionPacker;
use destack_mir as mir;
use destack_program::{
    DispatchTable, DynamicEntry, DynamicNamedEntry, DynamicShapeBuilder, DynamicSlot,
    DynamicTableBuilder, VirtualTableBuilder,
};

use super::ProgramLinker;

/// Link MIR dispatch entries into program dispatch tables.
#[derive(Debug)]
pub(crate) struct DispatchLinker<'a> {
    /// MIR dispatch table produced by lower and optimization.
    dispatch: &'a mir::DispatchTable,
    /// Dense program id projection.
    program: &'a ProgramLinker,
}

impl<'a> DispatchLinker<'a> {
    /// Create one dispatch linker.
    pub(crate) fn new(dispatch: &'a mir::DispatchTable, program: &'a ProgramLinker) -> Self {
        Self { dispatch, program }
    }

    /// Link program dispatch tables.
    pub(crate) fn link(&self, sections: &mut SectionPacker) -> DispatchTable {
        let mut virtuals = Vec::new();
        let mut dynamics = Vec::new();
        let mut dynamic_shapes = Vec::new();

        // project virtual dispatch entries into program ids
        for virtual_table in self.dispatch.iter_virtual_tables() {
            virtuals.push(VirtualTableBuilder {
                ty: self.program.type_id(virtual_table.ty),
                methods: virtual_table
                    .methods
                    .iter()
                    .map(|function| self.program.function_id(*function))
                    .collect(),
            });
        }

        // project dynamic shapes into program ids
        for shape in self.dispatch.iter_dynamic_shapes() {
            dynamic_shapes.push(DynamicShapeBuilder {
                constraint: self.program.type_id(shape.constraint),
                slots: shape
                    .slots
                    .iter()
                    .map(|slot| self.dynamic_slot(slot))
                    .collect(),
            });
        }

        // project dynamic table entries into program ids
        for dynamic_table in self.dispatch.iter_dynamic_tables() {
            dynamics.push(DynamicTableBuilder {
                concrete: self.program.type_id(dynamic_table.concrete),
                constraint: self.program.type_id(dynamic_table.constraint),
                entries: dynamic_table
                    .entries
                    .iter()
                    .map(|entry| self.dynamic_entry(entry))
                    .collect(),
                names: dynamic_table
                    .names
                    .iter()
                    .map(|named| DynamicNamedEntry {
                        name: named.name,
                        entry: self.dynamic_entry(&named.entry),
                    })
                    .collect(),
            });
        }

        DispatchTable::pack(sections, virtuals, dynamics, dynamic_shapes)
    }

    /// Project one MIR dynamic entry into program ids.
    fn dynamic_entry(&self, entry: &mir::DynamicEntry) -> DynamicEntry {
        match entry {
            mir::DynamicEntry::Field { offset } => DynamicEntry::field_offset(*offset),
            mir::DynamicEntry::Function { function } => {
                DynamicEntry::function(self.program.function_id(*function))
            }
            mir::DynamicEntry::Absent => DynamicEntry::absent(),
        }
    }

    /// Project one MIR dynamic slot into program ids.
    fn dynamic_slot(&self, slot: &mir::DynamicSlot) -> DynamicSlot {
        match slot {
            mir::DynamicSlot::Field { name, .. } => DynamicSlot::field(*name),
            mir::DynamicSlot::Function { name, signature } => {
                DynamicSlot::function(*name, self.program.type_id(*signature))
            }
        }
    }
}
