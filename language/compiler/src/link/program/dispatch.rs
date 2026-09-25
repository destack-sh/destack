use std::collections::HashMap;
use std::sync::Arc;

use tspp_program::Object;

use tspp_mir as mir;
use tspp_program::{
    DispatchTableBuilder, DynamicEntry, DynamicNamedEntry, DynamicShapeBuilder, DynamicSlot,
    DynamicTableBuilder, DynamicTableId, TypeId, VirtualTableBuilder, VirtualTableId,
};
use tspp_source::{ModuleId, PackageId};

use super::ProgramLinker;
use crate::{LinkError, LinkResult};

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
    pub(crate) fn link(&self) -> LinkResult<DispatchTableBuilder> {
        let mut virtuals = Vec::new();
        let mut dynamics = Vec::new();
        let mut dynamic_shapes = Vec::new();
        let mut shape_index = HashMap::new();

        // project every object's dispatch entries into canonical program ids
        for (module, object) in self.program.objects() {
            for virtual_table in object.dispatch().iter_virtual_tables() {
                let id = self
                    .program
                    .virtual_table_id(*module, virtual_table.concrete)
                    .ok_or_else(|| self.program.invalid_input("missing virtual table id"))?;
                let concrete = self.program.type_id(*module, virtual_table.concrete);
                let table = VirtualTableBuilder::new(concrete).methods(
                    virtual_table
                        .methods
                        .iter()
                        .map(|function| self.program.function_id(*module, *function))
                        .collect::<Vec<_>>(),
                );
                if id.index() == virtuals.len() {
                    virtuals.push(table);
                } else if virtuals.get(id.index()) != Some(&table) {
                    return Err(self.program.invalid_input(format!(
                        "virtual table {concrete:?} has conflicting definitions"
                    )));
                }
            }

            for shape in object.dispatch().iter_dynamic_shapes() {
                let constraint = self.program.type_id(*module, shape.constraint);
                let slots = shape
                    .slots
                    .iter()
                    .map(|slot| self.dynamic_slot(*module, slot))
                    .collect::<LinkResult<Vec<_>>>()?;
                let shape = DynamicShapeBuilder::new(constraint).slots(slots);
                self.insert_dynamic_shape(
                    &mut dynamic_shapes,
                    &mut shape_index,
                    constraint,
                    shape,
                )?;
            }

            for dynamic_table in object.dispatch().iter_dynamic_tables() {
                let id = self
                    .program
                    .dynamic_table_id(*module, dynamic_table.concrete, dynamic_table.constraint)
                    .ok_or_else(|| self.program.invalid_input("missing dynamic table id"))?;
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
                    return Err(self.program.invalid_input(format!(
                        "dynamic table {concrete:?} has conflicting definitions"
                    )));
                }
            }
        }

        Ok(DispatchTableBuilder::new()
            .virtual_tables(virtuals)
            .dynamic_tables(dynamics)
            .dynamic_shapes(dynamic_shapes))
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

    /// Project one MIR dynamic slot into program ids.
    fn dynamic_slot(&self, module: ModuleId, slot: &mir::DynamicSlot) -> LinkResult<DynamicSlot> {
        match slot {
            mir::DynamicSlot::Field { name, .. } => Ok(DynamicSlot::field(*name)),
            mir::DynamicSlot::Function { name, signature } => {
                let signature = self
                    .program
                    .type_signature_id(module, *signature)
                    .ok_or_else(|| self.program.invalid_input("missing dynamic slot signature"))?;

                Ok(DynamicSlot::function(*name, signature))
            }
        }
    }

    /// Insert one canonical dynamic shape or reject a conflicting duplicate.
    fn insert_dynamic_shape(
        &self,
        shapes: &mut Vec<DynamicShapeBuilder>,
        indices: &mut HashMap<TypeId, usize>,
        constraint: TypeId,
        shape: DynamicShapeBuilder,
    ) -> LinkResult<()> {
        let Some(index) = indices.get(&constraint).copied() else {
            indices.insert(constraint, shapes.len());
            shapes.push(shape);

            return Ok(());
        };

        if shapes[index] != shape {
            return Err(self.program.invalid_input(format!(
                "dynamic shape for type {constraint:?} has conflicting definitions"
            )));
        }

        Ok(())
    }
}

impl DispatchLinker<'_> {
    /// Build dense virtual table ids from canonical concrete types.
    pub(crate) fn virtual_ids(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> LinkResult<HashMap<TypeId, VirtualTableId>> {
        let mut ids = HashMap::new();

        // require every virtual receiver to expose its dispatch id
        for (module, object) in objects {
            for table in object.dispatch().iter_virtual_tables() {
                let layout = object
                    .layouts()
                    .type_layout(table.concrete)
                    .ok_or_else(|| {
                        LinkError::invalid_input(
                            package,
                            format!("missing layout for virtual type {:?}", table.concrete),
                        )
                    })?;
                let mir::LayoutShape::Object(object_layout) = &layout.shape else {
                    return Err(LinkError::invalid_input(
                        package,
                        format!(
                            "virtual type {:?} does not have an object layout",
                            table.concrete
                        ),
                    ));
                };
                let Some(dispatch_offset) = object_layout.dispatch_offset else {
                    return Err(LinkError::invalid_input(
                        package,
                        format!("virtual type {:?} has no dispatch offset", table.concrete),
                    ));
                };
                let dispatch_end = u64::from(dispatch_offset) + size_of::<u32>() as u64;
                if dispatch_end > u64::from(layout.size) {
                    return Err(LinkError::invalid_input(
                        package,
                        format!(
                            "virtual type {:?} has a dispatch offset outside its layout",
                            table.concrete
                        ),
                    ));
                }

                let concrete = type_ids[&(*module, table.concrete)];
                let next = VirtualTableId(ids.len() as u32);
                ids.entry(concrete).or_insert(next);
            }
        }

        // require every dispatch field to have one canonical virtual table
        for (module, object) in objects {
            for ty in object.types() {
                let Some(layout) = object.layouts().type_layout(ty.id) else {
                    continue;
                };
                let mir::LayoutShape::Object(layout) = &layout.shape else {
                    continue;
                };
                if layout.dispatch_offset.is_none() {
                    continue;
                }

                let concrete = type_ids[&(*module, ty.id)];
                if !ids.contains_key(&concrete) {
                    return Err(LinkError::invalid_input(
                        package,
                        format!("object type {:?} has no virtual table", ty.id),
                    ));
                }
            }
        }

        Ok(ids)
    }

    /// Build dense dynamic table ids from canonical concrete and constraint types.
    pub(crate) fn dynamic_ids(
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> HashMap<(TypeId, TypeId), DynamicTableId> {
        let mut ids = HashMap::new();

        // assign one entry to each canonical implementation pair
        for (module, object) in objects {
            for table in object.dispatch().iter_dynamic_tables() {
                let key = (
                    type_ids[&(*module, table.concrete)],
                    type_ids[&(*module, table.constraint)],
                );
                let next = DynamicTableId(ids.len() as u32);
                ids.entry(key).or_insert(next);
            }
        }

        ids
    }
}
