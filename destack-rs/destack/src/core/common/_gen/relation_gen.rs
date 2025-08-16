//! destack.core.common.relation@2025.08.15.1

#![destack::generated(destack.core.common.relation, file)]

use crate::{
    NodeIdentityReference, NodeSpatialReference, NodeTemporalReference, ObjectDefinitionReference,
    PropertyReference,
};

#[destack::generated(NodeIdentityReference, Debug, block)]
impl std::fmt::Debug for NodeIdentityReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeIdentityReference")
    }
}

#[destack::generated(NodeSpatialReference, Debug, block)]
impl std::fmt::Debug for NodeSpatialReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeSpatialReference")
    }
}

#[destack::generated(NodeTemporalReference, Debug, block)]
impl std::fmt::Debug for NodeTemporalReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeTemporalReference")
    }
}

#[destack::generated(ObjectDefinitionReference, Debug, block)]
impl std::fmt::Debug for ObjectDefinitionReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObjectDefinitionReference")
    }
}

#[destack::generated(PropertyReference, Debug, block)]
impl std::fmt::Debug for PropertyReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PropertyReference")
    }
}
