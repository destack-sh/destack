//! destack.core.common.relation@2025.08.15.1

#![destack::partial(destack.core.common.relation, file)]

use crate::{HandleType, NodeType, ObjectKind, StructType, Uuid};

#[destack::generated(NodeIdentityReference, -, block)]
/// A reference to a Node in an unknown space.
pub struct NodeIdentityReference {
    pub r#type: NodeType,
    pub id: Uuid,
}

#[destack::generated(NodeSpatialReference, -, block)]
/// A reference to a Node in space.
pub struct NodeSpatialReference {
    pub r#type: NodeType,
    pub id: Uuid,
    pub space_id: Uuid,
}

#[destack::generated(NodeTemporalReference, -, block)]
/// A reference to a Node in spacetime.
pub struct NodeTemporalReference {
    pub r#type: NodeType,
    pub id: Uuid,
    pub space_id: Uuid,
    pub branch_id: Uuid,
    pub snapshot_id: Uuid,
    pub epoch: u64,
}

#[destack::generated(ObjectDefinitionReference, -, block)]
/// Reference to an object "type" (builtin, custom or trait).
pub struct ObjectDefinitionReference {
    pub kind: ObjectKind,
    pub node_type: Option<NodeType>,
    pub struct_type: Option<StructType>,
    pub handle_type: Option<HandleType>,
    pub definition: Option<i64>,
}

#[destack::generated(PropertyReference, -, block)]
/// A reference to a builtin object's Property.
pub struct PropertyReference {
    pub node_type: Option<NodeType>,
    pub struct_type: Option<StructType>,
    pub id: Option<u8>,
    pub custom_property: Option<i64>,
}
