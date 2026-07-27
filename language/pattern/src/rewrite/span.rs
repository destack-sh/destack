use std::fmt::{Display, Formatter};

use destack_dir as dir;
use destack_source::Span;

/// Complete source spans for visible DIR nodes.
pub(super) struct NodeSpans {
    /// The source span indexed by local node identifier.
    spans: Vec<Option<Span>>,
}

/// An invariant violation while indexing candidate source spans.
pub(super) enum NodeSpanError {
    /// A visible decorator has no source span.
    MissingDecorator,
    /// An attached decorator and its owner occupy different files.
    DifferentFiles,
}

impl NodeSpans {
    /// Index visible node spans and propagate attached decorators to their ancestors.
    pub(super) fn new(candidate: dir::View<'_>) -> Result<Self, NodeSpanError> {
        let mut spans = vec![None; candidate.next_global_id() as usize];
        for node in candidate.iter_node_ids() {
            spans[node.id as usize] = candidate.get_span_by_id(node.id);
        }

        // extend every ancestor across decorators in its selected subtree
        for decorator in candidate.iter_node_ids_of_type::<dir::Decorator>() {
            let decorator_span = candidate
                .get_span_by_id(decorator.id)
                .ok_or(NodeSpanError::MissingDecorator)?;
            let mut node = decorator.into_any();
            while let Some(parent) = candidate.get_parent_any(node) {
                if let Some(span) = spans[parent.id as usize].as_mut() {
                    if span.file != decorator_span.file {
                        return Err(NodeSpanError::DifferentFiles);
                    }
                    span.start = span.start.min(decorator_span.start);
                    span.end = span.end.max(decorator_span.end);
                }
                node = parent;
            }
        }

        Ok(Self { spans })
    }

    /// Return the complete visible source span for one node.
    pub(super) fn get(&self, node: dir::LocalNodeIdAny) -> Option<Span> {
        self.spans.get(node.id as usize).copied().flatten()
    }
}

impl Display for NodeSpanError {
    /// Format the violated source span invariant.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::MissingDecorator => "candidate decorator has no source span",
            Self::DifferentFiles => "candidate decorator and owner occupy different files",
        };

        formatter.write_str(message)
    }
}
