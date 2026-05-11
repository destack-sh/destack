use serde::{Deserialize, Serialize};

use crate::{LocalNodeId, Type};

use super::{
    DataLayout, DebugMetadata, DispatchMetadata, DropMetadata, LayoutMetadata, MemoryMetadata,
    ProvenanceMetadata, TypeMetadata,
};

/// Structured MIR metadata domains.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// Target data layout.
    pub data_layout: DataLayout,
    /// Canonical type metadata.
    pub types: TypeMetadata,
    /// Canonical layout metadata.
    pub layout: LayoutMetadata,
    /// Canonical dispatch metadata.
    pub dispatch: DispatchMetadata,
    /// Canonical drop metadata.
    pub drop: DropMetadata,
    /// Provenance and source tracking metadata.
    pub provenance: ProvenanceMetadata,
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
        self.drop.copy_type_metadata(from, to);
    }
}
