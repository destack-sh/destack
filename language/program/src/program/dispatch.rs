use destack_core::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{FunctionId, TypeId};

/// Durable virtual dispatch table id inside one program.
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

/// Executable dispatch table carried by one durable program.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DispatchTable {
    /// Virtual dispatch tables keyed by dense table index.
    pub virtual_tables: Vec<VirtualTable>,
    /// Dynamic dispatch tables keyed by dense table index.
    pub dynamic_tables: Vec<DynamicTable>,
    /// Dynamic table shapes keyed by constraint type.
    pub dynamic_shapes: Vec<DynamicShape>,
}

impl DispatchTable {
    /// Return the virtual dispatch table id for one concrete type.
    pub fn virtual_table_id(&self, ty: TypeId) -> Option<VirtualTableId> {
        self.virtual_tables
            .iter()
            .position(|table| table.ty == ty)
            .map(|index| VirtualTableId::from(index as u32))
    }

    /// Return the virtual table for one concrete type.
    pub fn virtual_table(&self, ty: TypeId) -> Option<&VirtualTable> {
        self.virtual_table_id(ty)
            .and_then(|id| self.virtual_table_by_id(id))
    }

    /// Return the virtual table for one executable table id.
    pub fn virtual_table_by_id(&self, id: VirtualTableId) -> Option<&VirtualTable> {
        self.virtual_tables.get(id.index())
    }

    /// Return the dynamic dispatch table id for one implementation.
    pub fn dynamic_table_id(&self, concrete: TypeId, constraint: TypeId) -> Option<DynamicTableId> {
        self.dynamic_tables
            .iter()
            .position(|table| table.concrete == concrete && table.constraint == constraint)
            .map(|index| DynamicTableId::from(index as u32))
    }

    /// Return the dynamic table for one concrete type and constraint.
    pub fn dynamic_table(&self, concrete: TypeId, constraint: TypeId) -> Option<&DynamicTable> {
        self.dynamic_table_id(concrete, constraint)
            .and_then(|id| self.dynamic_table_by_id(id))
    }

    /// Return the dynamic table for one executable table id.
    pub fn dynamic_table_by_id(&self, id: DynamicTableId) -> Option<&DynamicTable> {
        self.dynamic_tables.get(id.index())
    }

    /// Return the dynamic table shape for one constraint type.
    pub fn dynamic_shape(&self, constraint: TypeId) -> Option<&DynamicShape> {
        self.dynamic_shapes
            .iter()
            .find(|shape| shape.constraint == constraint)
    }
}

/// Executable virtual dispatch table for one concrete type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct VirtualTable {
    /// Concrete type owning this table.
    pub ty: TypeId,
    /// Drop glue function when one exists.
    pub destructor: Option<FunctionId>,
    /// Method implementations in runtime slot order.
    pub methods: Vec<FunctionId>,
}

/// Executable dynamic dispatch table for one concrete implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicTable {
    /// Concrete type providing the implementation.
    pub concrete: TypeId,
    /// Dynamic constraint type being implemented.
    pub constraint: TypeId,
    /// Entries in runtime slot order.
    pub entries: Vec<DynamicEntry>,
}

/// Executable dynamic table shape for one constraint type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct DynamicShape {
    /// Dynamic constraint type owning this shape.
    pub constraint: TypeId,
    /// Slots in declaration order.
    pub slots: Vec<DynamicSlot>,
}

/// Executable dynamic dispatch table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicEntry {
    /// Slot containing a field byte offset.
    FieldOffset {
        /// Field offset in bytes.
        offset: u32,
    },
    /// Slot containing a function implementation.
    Function {
        /// Concrete function implementation.
        function: FunctionId,
    },
}

/// Executable dynamic slot descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum DynamicSlot {
    /// Field slot.
    Field {
        /// Field name.
        name: StringId,
    },
    /// Function slot.
    Function {
        /// Function name, absent for call signatures.
        name: Option<StringId>,
        /// Function signature.
        signature: TypeId,
    },
}
