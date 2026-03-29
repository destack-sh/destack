use destack_fir::format::FileMarker;
use destack_source::{NodeSpanType, Span};

use super::printer::Printer;

impl<'a> Printer<'a> {
    /// Record one source marker at the current output position.
    pub(crate) fn mark_source(&mut self, source: u32) {
        self.markers.push(FileMarker {
            source,
            dest: self.code.len() as u32,
        });
    }

    /// Return the source span for one lowered JS node when one exists.
    #[inline]
    pub(crate) fn source_span(&self, node_id: u32) -> Option<Span> {
        self.source_map.source_span(self.tree, node_id)
    }

    /// Return one source part span for one lowered JS node when one exists.
    #[inline]
    pub(crate) fn source_part_span(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        self.source_map
            .source_part_span(self.tree, node_id, span_type)
    }
}
