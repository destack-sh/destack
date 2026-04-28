use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{Comment, Expression, LocalNodeId, NodeType};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::write;

use super::serialize::format_jsdoc_body;

/// Format one declaration documentation comment.
pub(crate) fn format_jsdoc_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comment: Comment,
) -> FormatResult<bool> {
    // only declaration documentation is reformatted
    if !comment.is_jsdoc()
        || comment.is_legal()
        || !comment_documents_declaration(f.context(), comment)
    {
        return Ok(false);
    }

    // jsdoc formatting is opt-in
    let Some(jsdoc_options) = f.context().options.jsdoc.as_ref() else {
        return Ok(false);
    };

    // comments wrap within the remaining line width
    let source = f.context().source_text();
    let line_width = usize::from(f.context().options.line_width);
    let tab_width = usize::from(f.context().options.indent_width);
    let indent_width = source
        .bytes_to(comment.span.start)
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .map(|byte| if byte == b'\t' { tab_width } else { 1 })
        .sum::<usize>();
    let available_width = line_width.saturating_sub(indent_width);

    // preserve comments that the jsdoc formatter intentionally skips
    let Some(formatted) = format_jsdoc_body(
        &comment,
        jsdoc_options,
        &source,
        available_width,
        &f.context().options,
    ) else {
        return Ok(false);
    };

    // emit the formatted comment through the normal ir writer
    write!(f, [formatted])?;

    Ok(true)
}

/// Return whether one Jsdoc comment documents a declaration node.
fn comment_documents_declaration(context: &DestackFormatContext<'_>, comment: Comment) -> bool {
    // trailing and dangling comments do not document declarations
    if !comment.is_leading() {
        return false;
    }

    // leading comments attach to the next source position
    let attached_to = comment.attached_to;
    let candidates = context
        .tree
        .source_map
        .get_enclosing_spans(attached_to, attached_to);

    // declarations can be direct nodes or expression wrappers
    candidates
        .iter()
        .filter(|candidate| candidate.span.start == attached_to)
        .any(
            |candidate| match context.tree.get_node_type(candidate.idx) {
                NodeType::Declaration
                | NodeType::Member
                | NodeType::TypeMember
                | NodeType::EnumField => true,
                NodeType::Expression => {
                    let node_id = LocalNodeId::<Expression>::new(candidate.idx);
                    matches!(
                        context.tree.get(node_id),
                        Expression::Declaration(_)
                            | Expression::Export { .. }
                            | Expression::ExportNamespace { .. }
                            | Expression::Import { .. }
                            | Expression::Let { .. }
                            | Expression::Using { .. }
                    )
                }
                _ => false,
            },
        )
}
