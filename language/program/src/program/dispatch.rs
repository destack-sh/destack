use destack_core::{
    EntryRange, EntryStore, Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice,
    StringId,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{FunctionId, TypeId};

/// Durable virtual dispatch table id inside one program.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct VirtualTableId(pub u32);

impl VirtualTableId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for VirtualTableId {
    /// Convert one raw virtual table id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

/// Durable dynamic dispatch table id inside one program.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct DynamicTableId(pub u32);

impl DynamicTableId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for DynamicTableId {
    /// Convert one raw dynamic table id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

/// Dispatch table carried by one durable program.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DispatchTable {
    /// Virtual tables keyed by dense virtual table id.
    virtual_tables: SectionSlice<VirtualTable>,
    /// Flattened virtual method function ids.
    virtual_methods: SectionSlice<FunctionId>,
    /// Dynamic tables keyed by dense dynamic table id.
    dynamic_tables: SectionSlice<DynamicTable>,
    /// Flattened dynamic dispatch entries.
    dynamic_entries: SectionSlice<DynamicEntry>,
    /// Dynamic table shapes keyed by constraint type.
    dynamic_shapes: SectionSlice<DynamicShape>,
    /// Flattened dynamic slots.
    dynamic_slots: SectionSlice<DynamicSlot>,
}

impl DispatchTable {
    /// Pack one dispatch table from build-time entries.
    pub fn pack(
        sections: &mut SectionPacker,
        virtual_tables: Vec<VirtualTableBuilder>,
        dynamic_tables: Vec<DynamicTableBuilder>,
        dynamic_shapes: Vec<DynamicShapeBuilder>,
    ) -> Self {
        let mut virtual_table_entries = Vec::with_capacity(virtual_tables.len());
        let mut virtual_methods = EntryStore::new();
        let mut dynamic_table_entries = Vec::with_capacity(dynamic_tables.len());
        let mut dynamic_entries = EntryStore::new();
        let mut dynamic_shape_entries = Vec::with_capacity(dynamic_shapes.len());
        let mut dynamic_slots = EntryStore::new();

        // flatten virtual method payloads
        for virtual_table in virtual_tables {
            let methods = virtual_methods.append(virtual_table.methods);

            virtual_table_entries.push(VirtualTable {
                ty: virtual_table.ty,
                methods,
            });
        }

        // flatten dynamic table payloads
        for dynamic in dynamic_tables {
            let entries = dynamic_entries.append(dynamic.entries);

            dynamic_table_entries.push(DynamicTable {
                concrete: dynamic.concrete,
                constraint: dynamic.constraint,
                entries,
            });
        }

        // flatten dynamic shape payloads
        for dynamic_shape in dynamic_shapes {
            let slots = dynamic_slots.append(dynamic_shape.slots);

            dynamic_shape_entries.push(DynamicShape {
                constraint: dynamic_shape.constraint,
                slots,
            });
        }

        let virtual_tables = sections.insert(virtual_table_entries);
        let virtual_methods = sections.insert(virtual_methods.into_entries());
        let dynamic_tables = sections.insert(dynamic_table_entries);
        let dynamic_entries = sections.insert(dynamic_entries.into_entries());
        let dynamic_shapes = sections.insert(dynamic_shape_entries);
        let dynamic_slots = sections.insert(dynamic_slots.into_entries());

        Self {
            virtual_tables,
            virtual_methods,
            dynamic_tables,
            dynamic_entries,
            dynamic_shapes,
            dynamic_slots,
        }
    }

    /// Return the virtual dispatch table id for one concrete type.
    pub fn virtual_table_id(
        &self,
        sections: SectionImage<'_>,
        ty: TypeId,
    ) -> Option<VirtualTableId> {
        sections
            .entries(self.virtual_tables)
            .iter()
            .position(|table| table.ty == ty)
            .map(|index| VirtualTableId::from(index as u32))
    }

    /// Return the virtual table for one concrete type.
    pub fn virtual_table<'a>(
        &self,
        sections: SectionImage<'a>,
        ty: TypeId,
    ) -> Option<&'a VirtualTable> {
        self.virtual_table_id(sections, ty)
            .and_then(|id| self.virtual_table_by_id(sections, id))
    }

    /// Return the virtual table for one table id.
    pub fn virtual_table_by_id<'a>(
        &self,
        sections: SectionImage<'a>,
        id: VirtualTableId,
    ) -> Option<&'a VirtualTable> {
        sections.entries(self.virtual_tables).get(id.index())
    }

    /// Return the dynamic table id for one concrete type and constraint.
    pub fn dynamic_table_id(
        &self,
        sections: SectionImage<'_>,
        concrete: TypeId,
        constraint: TypeId,
    ) -> Option<DynamicTableId> {
        sections
            .entries(self.dynamic_tables)
            .iter()
            .position(|table| table.concrete == concrete && table.constraint == constraint)
            .map(|index| DynamicTableId::from(index as u32))
    }

    /// Return the dynamic table for one concrete type and constraint.
    pub fn dynamic_table<'a>(
        &self,
        sections: SectionImage<'a>,
        concrete: TypeId,
        constraint: TypeId,
    ) -> Option<&'a DynamicTable> {
        self.dynamic_table_id(sections, concrete, constraint)
            .and_then(|id| self.dynamic_table_by_id(sections, id))
    }

    /// Return the dynamic table for one table id.
    pub fn dynamic_table_by_id<'a>(
        &self,
        sections: SectionImage<'a>,
        id: DynamicTableId,
    ) -> Option<&'a DynamicTable> {
        sections.entries(self.dynamic_tables).get(id.index())
    }

    /// Return the dynamic table shape for one constraint type.
    pub fn dynamic_shape<'a>(
        &self,
        sections: SectionImage<'a>,
        constraint: TypeId,
    ) -> Option<&'a DynamicShape> {
        sections
            .entries(self.dynamic_shapes)
            .iter()
            .find(|shape| shape.constraint == constraint)
    }

    /// Return the virtual methods for one virtual table.
    pub fn virtual_methods<'a>(
        &self,
        sections: SectionImage<'a>,
        table: &VirtualTable,
    ) -> &'a [FunctionId] {
        table.methods.slice(sections.entries(self.virtual_methods))
    }

    /// Return the dynamic entries for one dynamic table.
    pub fn dynamic_entries<'a>(
        &self,
        sections: SectionImage<'a>,
        table: &DynamicTable,
    ) -> &'a [DynamicEntry] {
        table.entries.slice(sections.entries(self.dynamic_entries))
    }

    /// Return the dynamic slots for one dynamic shape.
    pub fn dynamic_slots<'a>(
        &self,
        sections: SectionImage<'a>,
        shape: &DynamicShape,
    ) -> &'a [DynamicSlot] {
        shape.slots.slice(sections.entries(self.dynamic_slots))
    }
}

/// Virtual dispatch table for one concrete type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VirtualTable {
    /// Concrete type owning this table.
    pub ty: TypeId,
    /// Method implementations in runtime slot order.
    pub methods: EntryRange<FunctionId>,
}

/// Dynamic dispatch table for one concrete implementation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicTable {
    /// Concrete type providing the implementation.
    pub concrete: TypeId,
    /// Dynamic constraint type being implemented.
    pub constraint: TypeId,
    /// Entries in runtime slot order.
    pub entries: EntryRange<DynamicEntry>,
}

/// Dynamic table shape for one constraint type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicShape {
    /// Dynamic constraint type owning this shape.
    pub constraint: TypeId,
    /// Slots in declaration order.
    pub slots: EntryRange<DynamicSlot>,
}

/// Build-time virtual dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VirtualTableBuilder {
    /// Concrete type owning this table.
    pub ty: TypeId,
    /// Method implementations in runtime slot order.
    pub methods: Vec<FunctionId>,
}

/// Build-time dynamic dispatch table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicTableBuilder {
    /// Concrete type providing the implementation.
    pub concrete: TypeId,
    /// Dynamic constraint type being implemented.
    pub constraint: TypeId,
    /// Entries in runtime slot order.
    pub entries: Vec<DynamicEntry>,
}

/// Build-time dynamic table shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicShapeBuilder {
    /// Dynamic constraint type owning this shape.
    pub constraint: TypeId,
    /// Slots in declaration order.
    pub slots: Vec<DynamicSlot>,
}

/// Dynamic dispatch table entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicEntry {
    /// Entry kind.
    pub kind: DynamicEntryKind,
    /// Field byte offset or function id.
    pub value: u32,
}

impl DynamicEntry {
    /// Create a field offset entry.
    pub fn field_offset(offset: u32) -> Self {
        Self {
            kind: DynamicEntryKind::FieldOffset,
            value: offset,
        }
    }

    /// Create a function entry.
    pub fn function(function: FunctionId) -> Self {
        Self {
            kind: DynamicEntryKind::Function,
            value: function.0,
        }
    }

    /// Return the field byte offset when this is a field entry.
    pub fn field_offset_value(self) -> Option<u32> {
        (self.kind == DynamicEntryKind::FieldOffset).then_some(self.value)
    }

    /// Return the function id when this is a function entry.
    pub fn function_value(self) -> Option<FunctionId> {
        (self.kind == DynamicEntryKind::Function).then_some(FunctionId(self.value))
    }
}

/// Dynamic dispatch entry kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicEntryKind {
    /// Field offset entry.
    FieldOffset = 0,
    /// Function entry.
    Function = 1,
}

/// Dynamic dispatch slot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicSlot {
    /// Slot kind.
    pub kind: DynamicSlotKind,
    /// Field or function name.
    pub name: Optional<StringId>,
    /// Function signature type.
    pub signature: Optional<TypeId>,
}

impl DynamicSlot {
    /// Create a field slot.
    pub fn field(name: StringId) -> Self {
        Self {
            kind: DynamicSlotKind::Field,
            name: Optional::some(name),
            signature: Optional::none(),
        }
    }

    /// Create a function slot.
    pub fn function(name: Option<StringId>, signature: TypeId) -> Self {
        Self {
            kind: DynamicSlotKind::Function,
            name: name.into(),
            signature: Optional::some(signature),
        }
    }
}

/// Dynamic dispatch slot kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicSlotKind {
    /// Field slot.
    Field = 0,
    /// Function slot.
    Function = 1,
}

// SAFETY: dispatch ids and entries contain only fixed-width program entries.
unsafe impl SectionEntry for VirtualTableId {}
unsafe impl SectionEntry for DynamicTableId {}
unsafe impl SectionEntry for VirtualTable {}
unsafe impl SectionEntry for DynamicTable {}
unsafe impl SectionEntry for DynamicShape {}
unsafe impl SectionEntry for DynamicEntry {}
unsafe impl SectionEntry for DynamicSlot {}
