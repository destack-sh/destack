use tspp_dir as dir;
use tspp_source::{EnclosingSpan, FileId, Span};

use crate::{ModuleQueryContext, QueryError, QueryResult};

/// Resolve the line start at the given offset.
pub(crate) fn offset_line_start(source: &str, offset: usize) -> QueryResult<usize> {
    // require one exact source offset
    let before = source.get(..offset).ok_or(QueryError::invalid(format!(
        "source offset: {:?}, {:?}",
        offset,
        source.len()
    )))?;

    let start = match before.rfind('\n') {
        Some(newline_index) => newline_index + 1,
        None => 0,
    };

    Ok(start)
}

impl ModuleQueryContext<'_> {
    /// Return the required authored span of a node.
    pub(crate) fn node_span(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> QueryResult<Span> {
        let span = view
            .get_span_by_id(node_id.id)
            .ok_or(QueryError::missing(format!(
                "authored node span: {:?}",
                node_id.into_global(self.module_id())
            )))?;

        Ok(span)
    }

    /// Return the authored main span of a node when it has one.
    pub(crate) fn node_selection_span(
        &self,
        view: dir::View<'_>,
        node_id: dir::LocalNodeIdAny,
    ) -> QueryResult<Option<Span>> {
        let source_id = view.get_source_any(node_id);

        Ok(self.source_index()?.get_main(source_id))
    }

    /// Collect enclosing spans and sort from innermost to outermost.
    pub(crate) fn sorted_enclosing_spans(
        &self,
        file_id: FileId,
        start: u32,
        end: u32,
    ) -> QueryResult<Vec<EnclosingSpan>> {
        let mut enclosing = self
            .source_index()?
            .get_enclosing_spans(file_id, start, end);

        enclosing.sort_by_key(|span| span.length);

        Ok(enclosing)
    }
}

/// Sort spans by file and position and remove duplicates.
pub(crate) fn sort_and_dedup_spans(spans: &mut Vec<Span>) {
    // sort spans for stable ordering and deduplication
    spans.sort_by(|left, right| {
        (left.file.0, left.start, left.end).cmp(&(right.file.0, right.start, right.end))
    });

    // remove exact duplicate spans
    spans.dedup_by(|left, right| {
        left.file == right.file && left.start == right.start && left.end == right.end
    });
}
