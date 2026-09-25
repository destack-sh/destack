use crate::TsppFormatter;
use crate::annotation::FormatLeadingComments;
use crate::file::{ignore_ranges_for_nodes, write_source_span};
use tspp_dir::{LocalNodeId, Node, Tree, TreeStore};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::{empty_line, hard_line_break};
use tspp_fir::write;

/// Return whether source preserves an empty line between two block entries.
fn block_entries_have_blank_line_between<T>(
    f: &TsppFormatter<'_, '_>,
    previous_node_id: LocalNodeId<T>,
    next_node_id: LocalNodeId<T>,
) -> bool
where
    T: Node + Clone,
    Tree: TreeStore<T>,
{
    let previous_span = f.context().span(previous_node_id);
    let next_span = f.context().span(next_node_id);
    let Some(between_span) = previous_span.gap_to(next_span) else {
        return false;
    };

    f.context().has_blank_line(between_span)
}

/// Write comments between one block delimiter and its first entry.
fn write_initial_block_entry_comments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    start: u32,
    end: u32,
) -> FormatResult<()> {
    if end <= start {
        return Ok(());
    }

    let comment_nodes = f.context().comments().comments_in_range(start, end);
    if comment_nodes.is_empty() {
        return Ok(());
    }

    write!(f, [FormatLeadingComments::Comments(comment_nodes)])
}

/// Format one block of nodes after an optional opening delimiter cursor.
pub(crate) fn format_block_nodes_with_ignore_ranges_after<'ast, T, F>(
    f: &mut TsppFormatter<'ast, '_>,
    node_ids: &[LocalNodeId<T>],
    initial_gap: Option<(u32, u32)>,
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone,
    Tree: TreeStore<T>,
    F: FnMut(&mut TsppFormatter<'ast, '_>, LocalNodeId<T>) -> FormatResult<()>,
{
    // ignore ranges: only compute when the file may contain ignore directives
    let ignore_ranges = if f.context().has_ignore_directive_markers() {
        let source_comments = f.context().source_comments();
        ignore_ranges_for_nodes(f.context(), node_ids, source_comments)
    } else {
        std::collections::HashMap::new()
    };

    let mut skip_until: Option<u32> = None;
    for (index, node_id) in node_ids.iter().copied().enumerate() {
        let node_span = f.context().span(node_id);

        // skip nodes already covered by one ignored span
        if let Some(skip_end) = skip_until {
            if node_span.start < skip_end {
                continue;
            }

            skip_until = None;
        }

        // entry spacing
        if index > 0 {
            let previous_node_id = node_ids[index - 1];
            if block_entries_have_blank_line_between(f, previous_node_id, node_id) {
                write!(f, [empty_line()])?;
            } else {
                write!(f, [hard_line_break()])?;
            }
        }

        // ignored range passthrough
        if let Some(range_span) = ignore_ranges.get(&node_id.id) {
            let comments = f.context().comments().comments_before(range_span.start);
            if !comments.is_empty() {
                write!(f, [FormatLeadingComments::Comments(comments)])?;
            }

            write_source_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        // initial entry comments
        if index == 0
            && let Some((start, end)) = initial_gap
        {
            write_initial_block_entry_comments(f, start, end)?;
        }

        format_node(f, node_id)?;
    }

    Ok(())
}
