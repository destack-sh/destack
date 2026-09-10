use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::NodeType;

/// Dense metadata for one global DIR node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Reflect)]
pub(crate) struct NodeIndexEntry {
    /// The packed local id and node type.
    packed: u32,
}

impl NodeIndexEntry {
    const NODE_TYPE_SHIFT: u32 = 24;
    const LOCAL_ID_MASK: u32 = (1 << Self::NODE_TYPE_SHIFT) - 1;

    /// Pack one local id and node type into a dense entry.
    #[inline]
    pub(crate) fn new(local_id: u32, node_type: NodeType) -> Self {
        debug_assert!(
            local_id <= Self::LOCAL_ID_MASK,
            "DIR node local id exceeds packed index capacity: {local_id}"
        );

        Self {
            packed: local_id | ((node_type as u32) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Return the local arena id for this entry.
    #[inline]
    pub(crate) fn local_id(self) -> u32 {
        self.packed & Self::LOCAL_ID_MASK
    }

    /// Return the concrete node type for this entry.
    #[inline]
    pub(crate) fn node_type(self) -> NodeType {
        match (self.packed >> Self::NODE_TYPE_SHIFT) as u8 {
            1 => NodeType::Expression,
            2 => NodeType::TypeExpression,
            3 => NodeType::Block,
            4 => NodeType::Catch,
            5 => NodeType::Declaration,
            6 => NodeType::Declarator,
            7 => NodeType::Property,
            8 => NodeType::TypeMember,
            9 => NodeType::TypeMappedParameter,
            10 => NodeType::Member,
            11 => NodeType::EnumField,
            12 => NodeType::WhereClause,
            13 => NodeType::DependencyItem,
            14 => NodeType::GenericParameter,
            15 => NodeType::Parameter,
            16 => NodeType::GenericArgument,
            17 => NodeType::TupleElement,
            18 => NodeType::Argument,
            19 => NodeType::TreeAttribute,
            20 => NodeType::TreeChild,
            21 => NodeType::MatchArm,
            22 => NodeType::Pattern,
            23 => NodeType::PatternField,
            24 => NodeType::AssignPattern,
            25 => NodeType::AssignPatternField,
            26 => NodeType::Decorator,
            27 => NodeType::SwitchCase,
            _ => unreachable!("invalid DIR node type tag in packed node index"),
        }
    }
}

/// Initial buffer capacities for a DIR tree.
#[derive(Debug, Copy, Clone, Default)]
pub struct TreeCapacity {
    /// The expected total node count.
    pub nodes: usize,
    /// The expected expression count.
    pub expressions: usize,
    /// The expected type expression count.
    pub type_expressions: usize,
}

impl TreeCapacity {
    /// Create capacities from an expected total node count.
    pub fn nodes(nodes: usize) -> Self {
        Self {
            nodes,
            ..Self::default()
        }
    }
}

/// Snapshot of tree allocation state for speculative restores.
#[derive(Debug, Copy, Clone)]
pub struct TreeMark {
    /// The global node id cursor.
    pub(crate) next_global_id: u32,
    /// The decorator attachment log length.
    pub(crate) decorator_attachments_len: usize,
}

impl TreeMark {
    /// Return the next global node id captured by this mark.
    #[inline]
    pub fn next_global_id(self) -> u32 {
        self.next_global_id
    }
}
