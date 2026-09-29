use std::collections::HashMap;
use std::sync::Arc;

use tspp_program::Object;

use tspp_mir as mir;
use tspp_program::{
    DispatchTableBuilder, DynamicEntry, DynamicNamedEntry, DynamicTableBuilder, DynamicTableId,
    TypeId, VirtualSlot, VirtualTableBuilder, VirtualTableId,
};
use tspp_source::ModuleId;

use super::{ProgramLinker, TypeLinker};
use crate::{CompilerResult, invalid_program_input};

/// Link MIR dispatch entries into program dispatch tables.
#[derive(Debug)]
pub(crate) struct DispatchLinker<'a> {
    /// Dense program id projection.
    program: &'a ProgramLinker<'a>,
}

impl<'a> DispatchLinker<'a> {
    /// Create one dispatch linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link program dispatch tables.
    pub(crate) fn link(&self) -> CompilerResult<DispatchTableBuilder> {
        let mut virtuals = Vec::new();
        let mut dynamics = Vec::new();

        // link every object's class tables
        for (module, object) in self.program.objects() {
            for virtual_table in object.dispatch().iter_virtual_tables() {
                if !virtual_table.is_concrete() {
                    continue;
                }
                let id = self
                    .program
                    .virtual_table_id(*module, virtual_table.concrete)
                    .ok_or_else(|| invalid_program_input("missing virtual table id"))?;
                let concrete = self.program.type_id(*module, virtual_table.concrete);
                let mut slots = Vec::with_capacity(virtual_table.slots.len());
                for slot in &virtual_table.slots {
                    slots.push(self.virtual_slot(*module, virtual_table, slot)?);
                }
                let table = VirtualTableBuilder::new(concrete).slots(slots);
                if id.index() == virtuals.len() {
                    virtuals.push(table);
                } else if virtuals.get(id.index()) != Some(&table) {
                    return Err(invalid_program_input(format!(
                        "virtual table {concrete:?} has conflicting definitions"
                    )));
                }
            }

            // link every object's dynamic tables
            for dynamic_table in object.dispatch().iter_dynamic_tables() {
                let id = self
                    .program
                    .dynamic_table_id(*module, dynamic_table.concrete, dynamic_table.constraint)
                    .ok_or_else(|| invalid_program_input("missing dynamic table id"))?;
                let concrete = self.program.type_id(*module, dynamic_table.concrete);
                let constraint = self.program.type_id(*module, dynamic_table.constraint);
                let entries = dynamic_table
                    .entries
                    .iter()
                    .map(|entry| self.dynamic_entry(*module, entry));
                let names = dynamic_table.names.iter().map(|named| DynamicNamedEntry {
                    name: named.name,
                    entry: self.dynamic_entry(*module, &named.entry),
                });
                let table = DynamicTableBuilder::new(concrete, constraint)
                    .entries(entries)
                    .names(names);

                if id.index() == dynamics.len() {
                    dynamics.push(table);
                } else if dynamics.get(id.index()) != Some(&table) {
                    return Err(invalid_program_input(format!(
                        "dynamic table {concrete:?} has conflicting definitions"
                    )));
                }
            }
        }

        Ok(DispatchTableBuilder::new()
            .virtual_tables(virtuals)
            .dynamic_tables(dynamics))
    }

    /// Project one MIR virtual table slot into program ids.
    fn virtual_slot(
        &self,
        module: ModuleId,
        table: &mir::VirtualTable,
        slot: &mir::VirtualSlot,
    ) -> CompilerResult<VirtualSlot> {
        let class = table.concrete;
        match slot {
            mir::VirtualSlot::Method {
                function,
                arguments,
            } if arguments.is_empty() => Ok(self.program.function_id(module, *function).into()),
            mir::VirtualSlot::Conformance { constraint } => self
                .program
                .dynamic_table_id(module, table.value, *constraint)
                .map(VirtualSlot::from)
                .ok_or_else(|| {
                    invalid_program_input(format!(
                        "class {class:?} has no dynamic table for a conformance slot"
                    ))
                }),
            mir::VirtualSlot::Abstract => Err(invalid_program_input(format!(
                "concrete class {class:?} has an abstract slot"
            ))),
            mir::VirtualSlot::Method { .. } => Err(invalid_program_input(format!(
                "class {class:?} links a method slot its instantiation left open"
            ))),
        }
    }

    /// Project one MIR dynamic entry into program ids.
    fn dynamic_entry(&self, module: ModuleId, entry: &mir::DynamicEntry) -> DynamicEntry {
        match entry {
            mir::DynamicEntry::Field { offset } => DynamicEntry::field_offset(*offset),
            mir::DynamicEntry::Function { function } => {
                DynamicEntry::function(self.program.function_id(module, *function))
            }
            mir::DynamicEntry::Absent => DynamicEntry::absent(),
        }
    }
}

impl DispatchLinker<'_> {
    /// Build dense virtual table ids from canonical concrete types.
    pub(crate) fn virtual_ids(
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> CompilerResult<HashMap<TypeId, VirtualTableId>> {
        let mut ids = HashMap::new();

        // assign one id to each canonical concrete class
        for (module, object) in objects {
            for table in object.dispatch().iter_virtual_tables() {
                if !table.is_concrete() {
                    continue;
                }
                let concrete = TypeLinker::linked(type_ids, *module, table.concrete, object)?;
                let next = VirtualTableId(ids.len() as u32);
                ids.entry(concrete).or_insert(next);
            }
        }

        Ok(ids)
    }

    /// Build dense dynamic table ids from canonical concrete and constraint types.
    pub(crate) fn dynamic_ids(
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> CompilerResult<HashMap<(TypeId, TypeId), DynamicTableId>> {
        let mut ids = HashMap::new();

        // assign one entry to each canonical implementation pair
        for (module, object) in objects {
            for table in object.dispatch().iter_dynamic_tables() {
                let key = (
                    TypeLinker::linked(type_ids, *module, table.concrete, object)?,
                    TypeLinker::linked(type_ids, *module, table.constraint, object)?,
                );
                let next = DynamicTableId(ids.len() as u32);
                ids.entry(key).or_insert(next);
            }
        }

        Ok(ids)
    }
}
