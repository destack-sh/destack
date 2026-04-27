use destack_ast as ast;
use destack_source::Span;

use crate::LintAstContext;
use crate::rules::common::is_doc_comment_source;

/// Return true when one span contains at least one raw comment.
pub fn span_has_comment(tree: &ast::Tree, span: Span) -> bool {
    tree.comments().iter().any(|comment| {
        let comment_span = comment.span;
        comment_span.file == span.file
            && !comment_span.is_empty()
            && span.contains(comment_span.start)
            && span.contains(comment_span.end.saturating_sub(1))
    })
}

/// Return true when one node has one attached documentation comment.
pub fn node_has_doc(ctx: &LintAstContext<'_>, node_id: u32) -> bool {
    !doc_comments_for_node(ctx, node_id).is_empty()
}

/// Return true when one declaration expression pair has attached documentation.
pub fn expression_or_declaration_has_doc(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
) -> bool {
    node_has_doc(ctx, expression_id.id) || node_has_doc(ctx, declaration_id.id)
}

/// Return all documentation comments attached to one expression and declaration.
pub fn expression_or_declaration_docs(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
) -> Vec<ast::Comment> {
    let mut comments = doc_comments_for_node(ctx, expression_id.id);

    for comment in doc_comments_for_node(ctx, declaration_id.id) {
        let is_duplicate = comments
            .iter()
            .any(|existing| existing.span == comment.span);
        if !is_duplicate {
            comments.push(comment);
        }
    }

    comments
}

/// Return the first documentation comment from expression or declaration ownership.
pub fn first_expression_or_declaration_doc(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
) -> Option<ast::Comment> {
    expression_or_declaration_docs(ctx, expression_id, declaration_id)
        .into_iter()
        .next()
}

/// Return documentation comments attached to one node start.
fn doc_comments_for_node(ctx: &LintAstContext<'_>, node_id: u32) -> Vec<ast::Comment> {
    let node_span = ctx.tree.get_span_by_id(node_id);

    ctx.tree
        .comments()
        .iter()
        .copied()
        .filter(|comment| comment.is_leading() && comment.attached_to == node_span.start)
        .filter(|comment| is_doc_comment_source(ctx.get_span_text(comment.span)))
        .collect()
}
