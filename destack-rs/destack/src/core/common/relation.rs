//! destack.core.common.relation

#![destack::partial(destack.core.common.relation, file)]

use crate::{NodeType, StructType, Uuid};

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

#[destack::generated(PropertyReference, -, block)]
/// A reference to a builtin object's Property.
pub struct PropertyReference {
    pub node_type: Option<NodeType>,
    pub struct_type: Option<StructType>,
    pub id: Option<u8>,
    pub custom_property: Option<i64>,
}
