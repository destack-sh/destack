use destack_source::FileId;

use crate::query::QueryTestSession;

/// Resolve a query position from a cursor or marker target.
pub fn resolve_query_position(
    session: &QueryTestSession,
    target: &str,
) -> Result<(FileId, u32), String> {
    // parse cursor targets like $0
    if let Some(cursor_index) = target.strip_prefix('$') {
        let cursor_idx: usize = cursor_index.parse().unwrap_or(0);
        let Some(cursor) = session.markers.cursor(cursor_idx) else {
            return Err(format!("cursor ${cursor_idx} not found"));
        };
        return Ok((session.file_id, cursor.offset));
    }

    // resolve marker targets like def:foo
    let Some(marker) = session.markers.range(target) else {
        return Err(format!("marker '{target}' not found"));
    };

    Ok((marker.span.file, marker.span.start))
}
