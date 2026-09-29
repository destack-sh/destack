use serde::{Deserialize, Serialize};
use tspp_core::{
    EntryRange, EntryStore, SectionBuilder, SectionEntry, SectionImage, SectionSlice, StringId,
};
use tspp_serde::Reflect;

use super::{FunctionId, TypeId, Word};

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
    /// Flattened virtual table slots.
    virtual_slots: SectionSlice<VirtualSlot>,
    /// Dynamic tables keyed by dense dynamic table id.
    dynamic_tables: SectionSlice<DynamicTable>,
    /// Flattened dynamic dispatch entries.
    dynamic_entries: SectionSlice<DynamicEntry>,
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

    /// Return the slots of one virtual table.
    pub fn virtual_slots<'a>(
        &self,
        sections: SectionImage<'a>,
        table: &VirtualTable,
    ) -> &'a [VirtualSlot] {
        table.slots.slice(sections.entries(self.virtual_slots))
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

    /// Return whether every dispatch range fits its flattened column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let virtual_slots = sections.entries(self.virtual_slots).len();
        let entries = sections.entries(self.dynamic_entries).len();

        // check each dispatch table family independently
        let virtual_tables = sections
            .entries(self.virtual_tables)
            .iter()
            .all(|table| table.slots.fits(virtual_slots));
        let dynamic_tables = sections
            .entries(self.dynamic_tables)
            .iter()
            .all(|table| table.entries.fits(entries));

        virtual_tables && dynamic_tables
    }
}

/// Virtual table for one class.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct VirtualTable {
    /// The class owning this table.
    pub concrete: TypeId,
    /// The slots in dispatch order.
    pub slots: EntryRange<VirtualSlot>,
}

/// One virtual table slot.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct VirtualSlot(pub u32);

impl From<FunctionId> for VirtualSlot {
    /// Hold one method's function id.
    fn from(function: FunctionId) -> Self {
        Self(function.0)
    }
}

impl From<DynamicTableId> for VirtualSlot {
    /// Hold one conformance's dynamic table id.
    fn from(table: DynamicTableId) -> Self {
        Self(table.0)
    }
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

/// Build-time dispatch table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DispatchTableBuilder {
    /// Virtual tables in dense id order.
    virtual_tables: Vec<VirtualTableBuilder>,
    /// Dynamic tables in dense id order.
    dynamic_tables: Vec<DynamicTableBuilder>,
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

    /// Build this dispatch table into program sections.
    pub(crate) fn build(self, sections: &mut SectionBuilder) -> DispatchTable {
        let mut virtual_tables = Vec::with_capacity(self.virtual_tables.len());
        let mut virtual_slots = EntryStore::new();
        let mut dynamic_tables = Vec::with_capacity(self.dynamic_tables.len());
        let mut dynamic_entries = EntryStore::new();
        let mut dynamic_names = EntryStore::new();

        // flatten virtual table slots
        for virtual_table in self.virtual_tables {
            let slots = virtual_slots.append(virtual_table.slots);

            virtual_tables.push(VirtualTable {
                concrete: virtual_table.concrete,
                slots,
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

        DispatchTable {
            virtual_tables: sections.insert(virtual_tables),
            virtual_slots: sections.insert(virtual_slots.into_entries()),
            dynamic_tables: sections.insert(dynamic_tables),
            dynamic_entries: sections.insert(dynamic_entries.into_entries()),
            dynamic_names: sections.insert(dynamic_names.into_entries()),
        }
    }
}

/// Build-time virtual table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualTableBuilder {
    /// The class owning this table.
    concrete: TypeId,
    /// The slots in dispatch order.
    slots: Vec<VirtualSlot>,
}

impl VirtualTableBuilder {
    /// Create one virtual table builder.
    pub fn new(concrete: TypeId) -> Self {
        Self {
            concrete,
            slots: Vec::new(),
        }
    }

    /// Set the slots in dispatch order.
    pub fn slots(mut self, slots: impl IntoIterator<Item = VirtualSlot>) -> Self {
        self.slots = slots.into_iter().collect();

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
