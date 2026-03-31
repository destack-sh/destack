use crate::DestackFormatter;
use crate::format::directive::{ignore_ranges_for_nodes, write_ignored_span};
use destack_ast::{Comment, LocalNodeId, Node, NodeTree, NodeTreeImpl};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::hard_line_break;
use destack_fir::write;

/// Format one block of nodes while honoring ignored ranges and entry spacing.
pub(crate) fn format_block_nodes_with_ignore_ranges<'ast, T, F>(
    f: &mut DestackFormatter<'ast, '_>,
    node_ids: &[LocalNodeId<T>],
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Comment>,
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

        // blank line between entries
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        // ignored range passthrough
        if let Some(range_span) = ignore_ranges.get(&node_id.id) {
            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        format_node(f, node_id)?;
    }

    Ok(())
}
