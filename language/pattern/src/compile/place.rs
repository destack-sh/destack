use tspp_core::StringId;
use tspp_dir as dir;
use tspp_source::{NodeSpanType, Span};

use super::marker::{Marker, MarkerError};
use crate::{Metavariable, MetavariableId, MetavariableUse, Sequence};

/// The DIR location resolved for a marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MarkerTarget {
    /// A complete node.
    Node(dir::LocalNodeIdAny),
    /// A placeholder in a repeated node list.
    Nodes {
        /// The placeholder element.
        node: dir::LocalNodeIdAny,
        /// The source span occupied by the placeholder element.
        node_span: Span,
    },
    /// A name stored directly on a node.
    Name {
        /// The node storing the name.
        node: dir::LocalNodeIdAny,
        /// The source span identifying the name.
        span_type: NodeSpanType,
    },
}

impl Marker {
    /// Resolve the marker to a DIR location.
    pub(super) fn resolve(&self, tree: &dir::Tree) -> Result<MarkerTarget, MarkerError> {
        let exact_nodes = self.exact_nodes(tree);

        // repeated markers must occupy repeated lists
        if self.is_nodes {
            for node in exact_nodes {
                if Self::is_repeated_element(tree, node) {
                    return Ok(MarkerTarget::Nodes {
                        node,
                        node_span: self.token_span,
                    });
                }
            }

            // include structural prefixes owned by a repeated element
            for node in dir::View::new(tree).iter_node_ids() {
                if tree.get_main_span_by_id(node.id) != Some(self.span)
                    || !Self::is_repeated_element(tree, node)
                {
                    continue;
                }
                let Some(node_span) = tree.get_span_by_id(node.id) else {
                    return Err(MarkerError::MissingNodeSpan { span: self.span });
                };

                return Ok(MarkerTarget::Nodes { node, node_span });
            }

            return Err(MarkerError::InvalidRepeated { span: self.span });
        }

        // complete nodes take precedence over names stored on their parents
        if let Some(node) = exact_nodes.into_iter().next() {
            return Ok(MarkerTarget::Node(node));
        }

        let (node, span_type) = self
            .exact_side_node(tree)
            .ok_or(MarkerError::Invalid { span: self.span })?;

        Ok(MarkerTarget::Name { node, span_type })
    }

    /// Return exact range nodes from deepest to shallowest.
    fn exact_nodes(&self, tree: &dir::Tree) -> Vec<dir::LocalNodeIdAny> {
        let range = self.token_span.range();
        let view = dir::View::new(tree);
        let mut nodes = view
            .iter_node_ids()
            .filter(|node| {
                tree.get_span_by_id(node.id)
                    .is_some_and(|span| span.range() == range)
            })
            .collect::<Vec<_>>();
        nodes.sort_unstable_by_key(|node| std::cmp::Reverse(Self::node_depth(tree, *node)));

        nodes
    }

    /// Return the typed side span exactly occupied by this marker.
    fn exact_side_node(&self, tree: &dir::Tree) -> Option<(dir::LocalNodeIdAny, NodeSpanType)> {
        let owner = tree.find_innermost_node_span_owner(self.span)?;
        if owner.span_type == NodeSpanType::Enclosing {
            return None;
        }
        let owner_span = tree.get_side_span_by_id(owner.source_id, owner.span_type)?;
        if owner_span != self.span {
            return None;
        }
        let node_type = tree.get_node_type(owner.source_id);
        let node = dir::LocalNodeIdAny::new(owner.source_id, node_type);

        Some((node, owner.span_type))
    }

    /// Return a node's structural depth.
    fn node_depth(tree: &dir::Tree, node: dir::LocalNodeIdAny) -> usize {
        let mut depth = 0;
        let mut current = node.id;
        while let Some(parent) = tree.get_parent(current) {
            depth += 1;
            current = parent.id;
        }

        depth
    }

    /// Return whether a node occupies a supported repeated list.
    fn is_repeated_element(tree: &dir::Tree, node: dir::LocalNodeIdAny) -> bool {
        Sequence::find(tree, node).is_some()
    }
}

impl MarkerTarget {
    /// Build the declaration required by this target.
    pub(super) fn metavariable(self, name: StringId) -> Metavariable {
        match self {
            Self::Node(node) => Metavariable::Node {
                name,
                node_type: node.ty,
            },
            Self::Nodes { node, .. } => Metavariable::Nodes {
                name,
                node_type: node.ty,
            },
            Self::Name { .. } => Metavariable::Name { name },
        }
    }

    /// Build a marker use at this target.
    pub(super) fn metavariable_use(
        self,
        variable: Option<MetavariableId>,
        span: Span,
    ) -> MetavariableUse {
        match self {
            Self::Node(node) => MetavariableUse::Node {
                variable,
                node,
                span,
            },
            Self::Nodes { node, node_span } => MetavariableUse::Nodes {
                variable,
                node,
                node_span,
                span,
            },
            Self::Name { node, span_type } => MetavariableUse::Name {
                variable,
                node,
                span_type,
                span,
            },
        }
    }
}
