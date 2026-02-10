use destack_ast::{LocalNodeId, Node, NodeTree, NodeTreeImpl};

use crate::DestackFormatContext;

/// Shared multiline-break signals for collection-like layouts.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CollectionBreakScore {
    /// Whether source for the collection spans multiple lines.
    pub has_newline_in_source: bool,
    /// Whether collection items carry annotations.
    pub has_item_annotations: bool,
    /// Whether collection items carry nested structural complexity.
    pub has_nested_complexity: bool,
}

impl CollectionBreakScore {
    /// Return whether multiline layout is preferred for the collection.
    pub(crate) fn should_expand_multiline(self) -> bool {
        self.has_newline_in_source && (self.has_nested_complexity || self.has_item_annotations)
    }
}

/// Return whether any node in a collection has annotations.
pub(crate) fn collection_nodes_have_annotations<T>(
    context: &DestackFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
) -> bool
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    node_ids.iter().any(|node_id| {
        let node_id = LocalNodeId::<T>::new(node_id.id);
        context.has_annotation(node_id)
    })
}

/// Return whether any node in a collection spans multiple lines in source.
pub(crate) fn collection_nodes_have_newline<T>(
    context: &DestackFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
) -> bool
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    node_ids.iter().any(|node_id| {
        let node_id = LocalNodeId::<T>::new(node_id.id);
        context.node_has_newline(node_id)
    })
}

/// Return whether items in a collection are inline between first and last spans.
pub(crate) fn collection_range_is_inline<T>(
    context: &DestackFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
) -> bool
where
    T: Node,
    NodeTree: NodeTreeImpl<T>,
{
    let (Some(first_id), Some(last_id)) = (node_ids.first(), node_ids.last()) else {
        return false;
    };

    let first_span = context.get_span(LocalNodeId::<T>::new(first_id.id));
    let last_span = context.get_span(LocalNodeId::<T>::new(last_id.id));
    if first_span.file != last_span.file || first_span.start >= last_span.end {
        return false;
    }

    !context.has_newline(destack_source::Span::new(
        first_span.file,
        first_span.start,
        last_span.end,
    ))
}

/// Return whether a value collection should force multiline break by complexity.
pub(crate) fn collection_value_should_force_break(
    has_multiple_items: bool,
    has_complex_items: bool,
) -> bool {
    has_multiple_items && has_complex_items
}
