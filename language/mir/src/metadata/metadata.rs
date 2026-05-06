use serde::{Deserialize, Serialize};

use crate::{LocalNodeId, Type};

use super::{
    DataLayout, DebugMetadata, DispatchMetadata, LayoutMetadata, MemoryMetadata, Provenance,
    TypeMetadata,
};

/// Structured MIR metadata domains.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// Target data layout.
    pub data_layout: DataLayout,
    /// Canonical type facts.
    pub types: TypeMetadata,
    /// Canonical layout facts.
    pub layout: LayoutMetadata,
    /// Canonical dispatch facts.
    pub dispatch: DispatchMetadata,
    /// Provenance and source-tracking facts.
    pub provenance: Provenance,
    /// Debug metadata.
    pub debug: DebugMetadata,
    /// Memory and alias metadata.
    pub memory: MemoryMetadata,
}

impl Metadata {
    /// Copy type-owned metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        self.types.copy_type_metadata(from, to);
        self.layout.copy_type_metadata(from, to);
        self.dispatch.copy_type_metadata(from, to);
    }
}
