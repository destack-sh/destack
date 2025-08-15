//! destack.core.common.relation@2025.08.15.1

#![destack::partial(destack.core.common.relation, file)]

use crate::HandleType;
use crate::NodeType;
use crate::ObjectKind;
use crate::StructType;
use crate::Uuid;

#[destack::generated(NodeIdentityReference, struct, block)]
/// A reference to a Node in an unknown space.
pub struct NodeIdentityReference {
    r#type: NodeType,
    id: Uuid,
}

#[destack::generated(NodeSpatialReference, struct, block)]
/// A reference to a Node in space.
pub struct NodeSpatialReference {
    r#type: NodeType,
    id: Uuid,
    space_id: Uuid,
}

#[destack::generated(NodeTemporalReference, struct, block)]
/// A reference to a Node in spacetime.
pub struct NodeTemporalReference {
    r#type: NodeType,
    id: Uuid,
    space_id: Uuid,
    branch_id: Uuid,
    snapshot_id: Uuid,
    epoch: u64,
}

#[destack::generated(ObjectDefinitionReference, struct, block)]
/// Reference to an object "type" (builtin, custom or trait).
pub struct ObjectDefinitionReference {
    kind: ObjectKind,
    node_type: NodeType,
    struct_type: StructType,
    handle_type: HandleType,
    definition: i64, /* TODO */
}

#[destack::generated(PropertyReference, struct, block)]
/// A reference to a builtin object's Property.
pub struct PropertyReference {
    node_type: NodeType,
    struct_type: StructType,
    handle_type: HandleType,
    id: u8,
    custom_property: i64, /* TODO */
}
