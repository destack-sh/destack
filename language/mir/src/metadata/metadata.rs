use serde::{Deserialize, Serialize};

use destack_serde::Reflect;

use crate::{LocalNodeId, Type};

use super::{
    DataLayout, DispatchMetadata, DropMetadata, EffectMetadata, FrameMetadata, LayoutMetadata,
    MemoryMetadata, TypeMetadata,
};

/// Structured MIR metadata domains.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect)]
pub struct Metadata {
    /// Target data layout.
    pub data_layout: DataLayout,
    /// Canonical type metadata.
    pub types: TypeMetadata,
    /// Canonical layout metadata.
    pub layouts: LayoutMetadata,
    /// Canonical dispatch metadata.
    pub dispatch: DispatchMetadata,
    /// Canonical drop metadata.
    pub drops: DropMetadata,
    /// Canonical frame metadata.
    pub frames: FrameMetadata,
    /// Memory and alias metadata.
    pub memory: MemoryMetadata,
    /// Function and call effect metadata.
    pub effects: EffectMetadata,
}

impl Metadata {
    /// Copy type-owned metadata from one type id to another.
    pub fn copy_type_metadata(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        self.types.copy_type_metadata(from, to);
        self.layouts.copy_type_metadata(from, to);
        self.dispatch.copy_type_metadata(from, to);
        self.drops.copy_type_metadata(from, to);
    }
}
