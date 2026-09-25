use serde::{Deserialize, Serialize};
use tspp_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
    StringId,
};
use tspp_serde::Reflect;

use super::{FunctionId, SignatureId, TypeId, Word};

/// Durable virtual dispatch table id inside one program.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
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

impl From<Word> for DynamicTableId {
    /// Decode one dynamic table word.
    fn from(word: Word) -> Self {
        Self(word.bits() as u32)
    }
}

impl From<DynamicTableId> for Word {
    /// Encode one dynamic table word.
    fn from(table: DynamicTableId) -> Self {
        Self::from_bits(table.0 as u64)
    }
}

/// Dispatch table carried by one durable program.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry,
)]
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
    /// Flattened name-keyed dynamic entries.
    dynamic_names: SectionSlice<DynamicNamedEntry>,
}

impl DispatchTable {
    /// Return all virtual tables in dense id order.
    pub fn virtual_tables<'a>(&self, sections: SectionImage<'a>) -> &'a [VirtualTable] {
        sections.entries(self.virtual_tables)
    }

    /// Return the virtual table for one table id.
    pub fn virtual_table<'a>(
        &self,
        sections: SectionImage<'a>,
        id: VirtualTableId,
    ) -> Option<&'a VirtualTable> {
        sections.entries(self.virtual_tables).get(id.index())
    }

    /// Return the virtual methods for one virtual table.
    pub fn virtual_methods<'a>(
        &self,
        sections: SectionImage<'a>,
        table: &VirtualTable,
    ) -> &'a [FunctionId] {
        table.methods.slice(sections.entries(self.virtual_methods))
    }

    /// Return all dynamic tables in dense id order.
    pub fn dynamic_tables<'a>(&self, sections: SectionImage<'a>) -> &'a [DynamicTable] {
        sections.entries(self.dynamic_tables)
    }

    /// Return the dynamic table for one table id.
    pub fn dynamic_table<'a>(
        &self,
        sections: SectionImage<'a>,
        id: DynamicTableId,
    ) -> Option<&'a DynamicTable> {
        sections.entries(self.dynamic_tables).get(id.index())
    }

    /// Return the dynamic entries for one dynamic table.
    pub fn dynamic_entries<'a>(
        &self,
        sections: SectionImage<'a>,
        table: &DynamicTable,
    ) -> &'a [DynamicEntry] {
        table.entries.slice(sections.entries(self.dynamic_entries))
    }

    /// Return all dynamic table shapes in constraint order.
    pub fn dynamic_shapes<'a>(&self, sections: SectionImage<'a>) -> &'a [DynamicShape] {
        sections.entries(self.dynamic_shapes)
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

    /// Return the dynamic slots for one dynamic shape.
    pub fn dynamic_slots<'a>(
        &self,
        sections: SectionImage<'a>,
        shape: &DynamicShape,
    ) -> &'a [DynamicSlot] {
        shape.slots.slice(sections.entries(self.dynamic_slots))
    }

    /// Return whether every dispatch range fits its flattened column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let methods = sections.entries(self.virtual_methods).len();
        let entries = sections.entries(self.dynamic_entries).len();
        let slots = sections.entries(self.dynamic_slots).len();

        // check each dispatch table family independently
        let virtual_tables = sections
            .entries(self.virtual_tables)
            .iter()
            .all(|table| table.methods.fits(methods));
        let dynamic_tables = sections
            .entries(self.dynamic_tables)
            .iter()
            .all(|table| table.entries.fits(entries));
        let dynamic_shapes = sections
            .entries(self.dynamic_shapes)
            .iter()
            .all(|shape| shape.slots.fits(slots));

        virtual_tables && dynamic_tables && dynamic_shapes
    }
}

/// Virtual dispatch table for one concrete type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct VirtualTable {
    /// Concrete type owning this table.
    pub concrete: TypeId,
    /// Method implementations in runtime slot order.
    pub methods: EntryRange<FunctionId>,
}

/// Dynamic dispatch table for one concrete implementation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DynamicTable {
    /// Concrete type providing the implementation.
    pub concrete: TypeId,
    /// Dynamic constraint type being implemented.
    pub constraint: TypeId,
    /// Entries in runtime slot order.
    pub entries: EntryRange<DynamicEntry>,
    /// Name-keyed concrete field entries sorted by name.
    pub names: EntryRange<DynamicNamedEntry>,
}

/// One name-keyed entry in a dynamic dispatch table.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DynamicNamedEntry {
    /// The concrete field name.
    pub name: StringId,
    /// The entry backing the name.
    pub entry: DynamicEntry,
}

/// Dynamic table shape for one constraint type.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DynamicShape {
    /// Dynamic constraint type owning this shape.
    pub constraint: TypeId,
    /// Slots in declaration order.
    pub slots: EntryRange<DynamicSlot>,
}

/// Build-time dispatch table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DispatchTableBuilder {
    /// Virtual tables in dense id order.
    virtual_tables: Vec<VirtualTableBuilder>,
    /// Dynamic tables in dense id order.
    dynamic_tables: Vec<DynamicTableBuilder>,
    /// Dynamic shapes in constraint order.
    dynamic_shapes: Vec<DynamicShapeBuilder>,
}

impl DispatchTableBuilder {
    /// Create an empty dispatch table builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set virtual tables in dense id order.
    pub fn virtual_tables(
        mut self,
        virtual_tables: impl IntoIterator<Item = VirtualTableBuilder>,
    ) -> Self {
        self.virtual_tables = virtual_tables.into_iter().collect();

        self
    }

    /// Set dynamic tables in dense id order.
    pub fn dynamic_tables(
        mut self,
        dynamic_tables: impl IntoIterator<Item = DynamicTableBuilder>,
    ) -> Self {
        self.dynamic_tables = dynamic_tables.into_iter().collect();

        self
    }

    /// Set dynamic shapes in constraint order.
    pub fn dynamic_shapes(
        mut self,
        dynamic_shapes: impl IntoIterator<Item = DynamicShapeBuilder>,
    ) -> Self {
        self.dynamic_shapes = dynamic_shapes.into_iter().collect();

        self
    }

    /// Build this dispatch table into program sections.
    pub(crate) fn build(self, sections: &mut SectionBuilder) -> DispatchTable {
        let mut virtual_tables = Vec::with_capacity(self.virtual_tables.len());
        let mut virtual_methods = EntryStore::new();
        let mut dynamic_tables = Vec::with_capacity(self.dynamic_tables.len());
        let mut dynamic_entries = EntryStore::new();
        let mut dynamic_names = EntryStore::new();
        let mut dynamic_shapes = Vec::with_capacity(self.dynamic_shapes.len());
        let mut dynamic_slots = EntryStore::new();

        // flatten virtual method payloads
        for virtual_table in self.virtual_tables {
            let methods = virtual_methods.append(virtual_table.methods);

            virtual_tables.push(VirtualTable {
                concrete: virtual_table.concrete,
                methods,
            });
        }

        // flatten dynamic table payloads
        for dynamic in self.dynamic_tables {
            let entries = dynamic_entries.append(dynamic.entries);
            let names = dynamic_names.append(dynamic.names);

            dynamic_tables.push(DynamicTable {
                concrete: dynamic.concrete,
                constraint: dynamic.constraint,
                entries,
                names,
            });
        }

        // flatten dynamic shape payloads
        for dynamic_shape in self.dynamic_shapes {
            let slots = dynamic_slots.append(dynamic_shape.slots);

            dynamic_shapes.push(DynamicShape {
                constraint: dynamic_shape.constraint,
                slots,
            });
        }

        DispatchTable {
            virtual_tables: sections.insert(virtual_tables),
            virtual_methods: sections.insert(virtual_methods.into_entries()),
            dynamic_tables: sections.insert(dynamic_tables),
            dynamic_entries: sections.insert(dynamic_entries.into_entries()),
            dynamic_shapes: sections.insert(dynamic_shapes),
            dynamic_slots: sections.insert(dynamic_slots.into_entries()),
            dynamic_names: sections.insert(dynamic_names.into_entries()),
        }
    }
}

/// Build-time virtual dispatch table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualTableBuilder {
    /// Concrete type owning this table.
    concrete: TypeId,
    /// Method implementations in runtime slot order.
    methods: Vec<FunctionId>,
}

impl VirtualTableBuilder {
    /// Create one virtual table builder.
    pub fn new(concrete: TypeId) -> Self {
        Self {
            concrete,
            methods: Vec::new(),
        }
    }

    /// Set method implementations in runtime slot order.
    pub fn methods(mut self, methods: impl IntoIterator<Item = FunctionId>) -> Self {
        self.methods = methods.into_iter().collect();

        self
    }
}

/// Build-time dynamic dispatch table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicTableBuilder {
    /// Concrete type providing the implementation.
    concrete: TypeId,
    /// Dynamic constraint type being implemented.
    constraint: TypeId,
    /// Entries in runtime slot order.
    entries: Vec<DynamicEntry>,
    /// Name-keyed concrete field entries sorted by name.
    names: Vec<DynamicNamedEntry>,
}

impl DynamicTableBuilder {
    /// Create one dynamic table builder.
    pub fn new(concrete: TypeId, constraint: TypeId) -> Self {
        Self {
            concrete,
            constraint,
            entries: Vec::new(),
            names: Vec::new(),
        }
    }

    /// Set entries in runtime slot order.
    pub fn entries(mut self, entries: impl IntoIterator<Item = DynamicEntry>) -> Self {
        self.entries = entries.into_iter().collect();

        self
    }

    /// Set name-keyed concrete field entries sorted by name.
    pub fn names(mut self, names: impl IntoIterator<Item = DynamicNamedEntry>) -> Self {
        self.names = names.into_iter().collect();

        self
    }
}

/// Build-time dynamic table shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicShapeBuilder {
    /// Dynamic constraint type owning this shape.
    constraint: TypeId,
    /// Slots in declaration order.
    slots: Vec<DynamicSlot>,
}

impl DynamicShapeBuilder {
    /// Create one dynamic shape builder.
    pub fn new(constraint: TypeId) -> Self {
        Self {
            constraint,
            slots: Vec::new(),
        }
    }

    /// Set slots in declaration order.
    pub fn slots(mut self, slots: impl IntoIterator<Item = DynamicSlot>) -> Self {
        self.slots = slots.into_iter().collect();

        self
    }
}

/// Dynamic dispatch table entry.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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

    /// Create an absent entry, read as undefined.
    pub fn absent() -> Self {
        Self {
            kind: DynamicEntryKind::Absent,
            value: 0,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum DynamicEntryKind {
    /// Field offset entry.
    FieldOffset = 0,
    /// Function entry.
    Function = 1,
    /// Absent entry, read as undefined.
    Absent = 2,
}

/// Dynamic dispatch slot.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DynamicSlot {
    /// Slot kind.
    pub kind: DynamicSlotKind,
    /// Field or function name.
    pub name: Optional<StringId>,
    /// Function signature.
    pub signature: Optional<SignatureId>,
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
    pub fn function(name: Option<StringId>, signature: SignatureId) -> Self {
        Self {
            kind: DynamicSlotKind::Function,
            name: name.into(),
            signature: Optional::some(signature),
        }
    }
}

/// Dynamic dispatch slot kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum DynamicSlotKind {
    /// Field slot.
    Field = 0,
    /// Function slot.
    Function = 1,
}
