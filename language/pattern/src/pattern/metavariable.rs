use tspp_core::{Arena, StringId};
use tspp_dir as dir;
use tspp_source::{NodeSpanType, Span};

/// The index of a metavariable in a pattern.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MetavariableId(pub u32);

/// A value bound to the same name throughout a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metavariable {
    /// A DIR node.
    Node {
        /// The interned name without the leading dollar sign.
        name: StringId,
        /// The required node type.
        node_type: dir::NodeType,
    },
    /// Zero or more DIR nodes from the same repeated list.
    Nodes {
        /// The interned name without the leading dollar signs.
        name: StringId,
        /// The required element type.
        node_type: dir::NodeType,
    },
    /// An identifier-like value stored directly on a DIR node.
    Name {
        /// The interned name without the leading dollar sign.
        name: StringId,
    },
}

impl Metavariable {
    /// Return the interned name without leading dollar signs.
    pub fn name(self) -> StringId {
        match self {
            Self::Node { name, .. } | Self::Nodes { name, .. } | Self::Name { name } => name,
        }
    }

    /// Return the required node type for node values.
    pub fn node_type(self) -> Option<dir::NodeType> {
        match self {
            Self::Node { node_type, .. } | Self::Nodes { node_type, .. } => Some(node_type),
            Self::Name { .. } => None,
        }
    }
}

/// The metavariables declared by a pattern.
#[derive(Debug)]
pub struct MetavariableTable {
    /// The declarations in source order.
    pub(crate) metavariables: Arena<Metavariable>,
}

impl MetavariableTable {
    /// Create an empty table.
    pub(crate) fn new() -> Self {
        Self {
            metavariables: Arena::new(),
        }
    }

    /// Find a declaration by authored name.
    pub fn find(&self, name: &str) -> Option<MetavariableId> {
        self.find_id(StringId::for_text(name))
    }

    /// Find a declaration by interned name.
    pub(crate) fn find_id(&self, name: StringId) -> Option<MetavariableId> {
        self.metavariables
            .iter()
            .position(|metavariable| metavariable.name() == name)
            .map(|index| MetavariableId(index as u32))
    }

    /// Insert a declaration.
    pub(crate) fn insert(&mut self, metavariable: Metavariable) -> MetavariableId {
        MetavariableId(self.metavariables.allocate(metavariable))
    }

    /// Return a declaration.
    pub fn get(&self, variable: MetavariableId) -> &Metavariable {
        self.metavariables.get(variable.0)
    }

    /// Iterate over declarations in source order.
    pub fn iter(&self) -> impl Iterator<Item = (MetavariableId, &Metavariable)> {
        self.metavariables
            .iter()
            .enumerate()
            .map(|(index, variable)| (MetavariableId(index as u32), variable))
    }

    /// Return the declaration count.
    pub fn len(&self) -> usize {
        self.metavariables.len()
    }

    /// Return whether the table has no declarations.
    pub fn is_empty(&self) -> bool {
        self.metavariables.is_empty()
    }
}

/// The index of a metavariable marker in a fragment.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MetavariableUseId(pub(crate) u32);

/// A metavariable marker compiled into a DIR fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetavariableUse {
    /// A marker replacing a DIR node.
    Node {
        /// The declaration, or none for an anonymous marker.
        variable: Option<MetavariableId>,
        /// The replaced node.
        node: dir::LocalNodeIdAny,
        /// The authored marker.
        span: Span,
    },
    /// A marker replacing zero or more nodes in a repeated list.
    Nodes {
        /// The declaration, or none for an anonymous marker.
        variable: Option<MetavariableId>,
        /// The placeholder element parsed from the marker.
        node: dir::LocalNodeIdAny,
        /// The source span occupied by the placeholder element.
        node_span: Span,
        /// The authored marker.
        span: Span,
    },
    /// A marker replacing a name stored directly on a DIR node.
    Name {
        /// The declaration, or none for an anonymous marker.
        variable: Option<MetavariableId>,
        /// The node storing the name.
        node: dir::LocalNodeIdAny,
        /// The source span identifying the name.
        span_type: NodeSpanType,
        /// The authored marker.
        span: Span,
    },
}

impl MetavariableUse {
    /// Return the declaration, or none for an anonymous marker.
    pub fn variable(self) -> Option<MetavariableId> {
        match self {
            Self::Node { variable, .. }
            | Self::Nodes { variable, .. }
            | Self::Name { variable, .. } => variable,
        }
    }

    /// Return the authored marker.
    pub fn span(self) -> Span {
        match self {
            Self::Node { span, .. } | Self::Nodes { span, .. } | Self::Name { span, .. } => span,
        }
    }
}

/// The sparse index entry for a name marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct NameMetavariableUse {
    /// The node storing the name.
    node: dir::LocalNodeIdAny,
    /// The source span identifying the name.
    span_type: NodeSpanType,
    /// The marker identifier.
    use_id: MetavariableUseId,
}

/// The metavariable markers in a DIR fragment.
#[derive(Debug, PartialEq, Eq)]
pub struct MetavariableUses {
    /// The markers in source order.
    uses: Vec<MetavariableUse>,
    /// The node marker indexed by local node identifier.
    use_by_node: Vec<Option<MetavariableUseId>>,
    /// The name markers sorted by node and source span.
    name_uses: Vec<NameMetavariableUse>,
}

impl MetavariableUses {
    /// Create an empty marker index for a DIR tree.
    pub(crate) fn new(node_count: usize) -> Self {
        Self {
            uses: Vec::new(),
            use_by_node: vec![None; node_count],
            name_uses: Vec::new(),
        }
    }

    /// Insert a marker.
    pub(crate) fn insert(&mut self, metavariable_use: MetavariableUse) {
        let use_id = MetavariableUseId(self.uses.len() as u32);

        // index node markers densely
        match metavariable_use {
            MetavariableUse::Node { node, .. } | MetavariableUse::Nodes { node, .. } => {
                let slot = &mut self.use_by_node[node.id as usize];
                assert!(slot.is_none(), "DIR node has multiple metavariable markers");
                *slot = Some(use_id);
            }

            // retain name markers for sorted sparse lookup
            MetavariableUse::Name {
                node, span_type, ..
            } => {
                self.name_uses.push(NameMetavariableUse {
                    node,
                    span_type,
                    use_id,
                });
            }
        }

        self.uses.push(metavariable_use);
    }

    /// Finish lookup indexes after insertion.
    pub(crate) fn finish(&mut self) {
        self.name_uses
            .sort_unstable_by_key(|entry| (entry.node.id, entry.span_type));
    }

    /// Iterate over markers in source order.
    pub fn iter(&self) -> impl Iterator<Item = &MetavariableUse> {
        self.uses.iter()
    }

    /// Return the marker replacing a node.
    pub fn get_node(&self, node: dir::LocalNodeIdAny) -> Option<&MetavariableUse> {
        let use_id = self.use_by_node.get(node.id as usize).copied().flatten()?;

        self.uses.get(use_id.0 as usize)
    }

    /// Return the marker replacing a name.
    pub fn get_name(
        &self,
        node: dir::LocalNodeIdAny,
        span_type: NodeSpanType,
    ) -> Option<&MetavariableUse> {
        let index = self
            .name_uses
            .binary_search_by_key(&(node.id, span_type), |entry| {
                (entry.node.id, entry.span_type)
            })
            .ok()?;
        let use_id = self.name_uses[index].use_id;

        self.uses.get(use_id.0 as usize)
    }
}
