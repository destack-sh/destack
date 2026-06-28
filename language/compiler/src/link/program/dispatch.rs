use destack_mir as mir;
use destack_program::{
    DispatchTable, DynamicEntry, DynamicShape, DynamicSlot, DynamicTable, VirtualTable,
};

use super::ProgramLinker;

/// Link MIR dispatch rows into executable dispatch tables.
#[derive(Debug)]
pub(crate) struct DispatchLinker<'a> {
    /// MIR dispatch table produced by lower and optimization.
    dispatch: &'a mir::DispatchTable,
    /// Dense executable id projection for this program.
    program: &'a ProgramLinker,
}

impl<'a> DispatchLinker<'a> {
    /// Create one dispatch linker.
    pub(crate) fn new(dispatch: &'a mir::DispatchTable, program: &'a ProgramLinker) -> Self {
        Self { dispatch, program }
    }

    /// Link executable dispatch tables.
    pub(crate) fn link(&self) -> DispatchTable {
        let mut table = DispatchTable::default();

        // project virtual dispatch rows into program ids
        for virtual_table in self.dispatch.iter_virtual_tables() {
            table.virtual_tables.push(VirtualTable {
                ty: self.program.type_id(virtual_table.ty),
                destructor: virtual_table
                    .destructor
                    .map(|function| self.program.function_id(function)),
                methods: virtual_table
                    .methods
                    .iter()
                    .map(|function| self.program.function_id(*function))
                    .collect(),
            });
        }

        // project dynamic shapes into program ids
        for shape in self.dispatch.iter_dynamic_shapes() {
            table.dynamic_shapes.push(DynamicShape {
                constraint: self.program.type_id(shape.constraint),
                slots: shape
                    .slots
                    .iter()
                    .map(|slot| self.dynamic_slot(slot))
                    .collect(),
            });
        }

        // project dynamic implementation rows into program ids
        for dynamic_table in self.dispatch.iter_dynamic_tables() {
            table.dynamic_tables.push(DynamicTable {
                concrete: self.program.type_id(dynamic_table.concrete),
                constraint: self.program.type_id(dynamic_table.constraint),
                entries: dynamic_table
                    .entries
                    .iter()
                    .map(|entry| self.dynamic_entry(entry))
                    .collect(),
            });
        }

        table
    }

    /// Project one MIR dynamic entry into executable ids.
    fn dynamic_entry(&self, entry: &mir::DynamicEntry) -> DynamicEntry {
        match entry {
            mir::DynamicEntry::FieldOffset { offset } => {
                DynamicEntry::FieldOffset { offset: *offset }
            }
            mir::DynamicEntry::Function { function } => DynamicEntry::Function {
                function: self.program.function_id(*function),
            },
        }
    }

    /// Project one MIR dynamic slot into executable ids.
    fn dynamic_slot(&self, slot: &mir::DynamicSlot) -> DynamicSlot {
        match slot {
            mir::DynamicSlot::Field { name, .. } => DynamicSlot::Field { name: *name },
            mir::DynamicSlot::Function { name, signature } => DynamicSlot::Function {
                name: *name,
                signature: self.program.type_id(*signature),
            },
        }
    }
}
