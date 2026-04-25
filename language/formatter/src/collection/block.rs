use crate::DestackFormatter;
use crate::annotation::FormatLeadingComments;
use crate::file::{ignore_ranges_for_nodes, write_ignored_span};
use destack_ast::{LocalNodeId, Node, NodeTree, NodeTreeImpl};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{empty_line, hard_line_break};
use destack_fir::write;

/// Return whether source preserves an empty line between two block entries.
fn block_entries_have_blank_line_between<T>(
    f: &DestackFormatter<'_, '_>,
    previous_node_id: LocalNodeId<T>,
    next_node_id: LocalNodeId<T>,
) -> bool
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
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
    f: &mut DestackFormatter<'ast, '_>,
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
    f: &mut DestackFormatter<'ast, '_>,
    node_ids: &[LocalNodeId<T>],
    initial_gap: Option<(u32, u32)>,
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
    F: FnMut(&mut DestackFormatter<'ast, '_>, LocalNodeId<T>) -> FormatResult<()>,
{
    let comment_tokens = f.context().comment_tokens();
    let ignore_ranges = ignore_ranges_for_nodes(f.context(), node_ids, comment_tokens);

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
            write_ignored_span(f, *range_span)?;
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
