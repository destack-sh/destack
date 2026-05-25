use serde::{Deserialize, Serialize};

use crate::NodeType;

/// Dense metadata for one global DIR node id.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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
            local_id < Self::LOCAL_ID_MASK,
            "DIR node local id exceeds packed index capacity: {local_id}"
        );

        Self {
            packed: local_id | ((node_type as u32) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Create one placeholder entry for a reserved node slot.
    #[inline]
    pub(crate) fn placeholder(node_type: NodeType) -> Self {
        Self {
            packed: Self::LOCAL_ID_MASK | ((node_type as u32) << Self::NODE_TYPE_SHIFT),
        }
    }

    /// Return the local arena id for this entry.
    #[inline]
    pub(crate) fn local_id(self) -> u32 {
        self.packed & Self::LOCAL_ID_MASK
    }

    /// Return whether this entry is a reserved slot without arena storage.
    #[inline]
    pub(crate) fn is_placeholder(self) -> bool {
        self.local_id() == Self::LOCAL_ID_MASK
    }

    /// Return the concrete node type for this entry.
    #[inline]
    pub(crate) fn node_type(self) -> NodeType {
        match (self.packed >> Self::NODE_TYPE_SHIFT) as u8 {
            0 => NodeType::Expression,
            1 => NodeType::TypeExpression,
            2 => NodeType::Block,
            3 => NodeType::Catch,
            4 => NodeType::Declaration,
            5 => NodeType::Declarator,
            6 => NodeType::Property,
            7 => NodeType::TypeMember,
            8 => NodeType::TypeMappedParameter,
            9 => NodeType::Member,
            10 => NodeType::EnumField,
            11 => NodeType::WhereClause,
            12 => NodeType::DependencyItem,
            13 => NodeType::GenericParameter,
            14 => NodeType::Parameter,
            15 => NodeType::GenericArgument,
            16 => NodeType::TupleElement,
            17 => NodeType::Argument,
            18 => NodeType::MatchCase,
            19 => NodeType::Pattern,
            20 => NodeType::PatternField,
            21 => NodeType::AssignPattern,
            22 => NodeType::AssignPatternField,
            23 => NodeType::Decorator,
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
    /// The expected comment count.
    pub comments: usize,
}

impl TreeCapacity {
    /// Create capacities from an expected total node count.
    pub fn nodes(nodes: usize) -> Self {
        Self {
            nodes,
            comments: nodes / 16,
            ..Self::default()
        }
    }
}

/// Snapshot of tree allocation state for speculative restores.
#[derive(Debug, Copy, Clone)]
pub struct TreeMark {
    /// The global node id cursor.
    pub(crate) next_global_id: u32,
    /// The comment list length.
    pub(crate) comments_len: usize,
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
