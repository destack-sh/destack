use destack_source::{FileId, Span};

use crate::query::QueryTestSession;

/// Resolve a query position from a cursor or marker target.
pub fn resolve_query_position(
    session: &QueryTestSession,
    target: &str,
) -> Result<(FileId, u32), String> {
    // parse cursor targets like $0
    if let Some(cursor_index) = target.strip_prefix('$') {
        let cursor_idx: usize = cursor_index
            .parse()
            .map_err(|_| format!("invalid cursor target '{target}'"))?;
        let Some(cursor) = session.markers.cursor(cursor_idx) else {
            return Err(format!("cursor ${cursor_idx} not found"));
        };
        return Ok((cursor.file_id, cursor.offset));
    }

    // resolve marker targets like def:foo
    let Some(marker) = session.markers.range(target) else {
        return Err(format!("marker '{target}' not found"));
    };

    Ok((marker.span.file, marker.span.start))
}

/// Resolve a query span from a cursor or marker target.
pub fn resolve_query_span(session: &QueryTestSession, target: &str) -> Result<Span, String> {
    // convert cursor targets into empty spans at the cursor
    if target.starts_with('$') {
        let (file_id, offset) = resolve_query_position(session, target)?;
        return Ok(Span::new(file_id, offset, offset));
    }

    // resolve marker targets into their full span
    let Some(marker) = session.markers.range(target) else {
        return Err(format!("marker '{target}' not found"));
    };

    Ok(marker.span)
}

#[cfg(test)]
mod tests {
    use super::resolve_query_position;
    use crate::query::QueryTestSession;

    #[test]
    fn test_resolve_query_position_rejects_invalid_cursor_targets() {
        let session = QueryTestSession::from_source("const value = 1;\n");

        let error = resolve_query_position(&session, "$oops").unwrap_err();
        assert_eq!(error, "invalid cursor target '$oops'");
    }
}
