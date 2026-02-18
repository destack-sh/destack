use destack_ast as ast;
use destack_source::Span;

/// Return true when one span contains at least one comment trivia.
pub fn span_has_comment_trivia(tree: &ast::NodeTree, span: Span) -> bool {
    tree.comment_trivia().iter().any(|trivia| {
        let trivia_span = trivia.span;
        trivia_span.file == span.file
            && !trivia_span.is_empty()
            && span.contains(trivia_span.start)
            && span.contains(trivia_span.end.saturating_sub(1))
    })
}

/// Return true when one node has attached documentation.
pub fn node_has_doc(tree: &ast::NodeTree, node_id: u32) -> bool {
    !tree.get_docs_for(node_id).is_empty()
}

/// Return true when one declaration expression pair has attached documentation.
pub fn expression_or_declaration_has_doc(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
) -> bool {
    node_has_doc(tree, expression_id.id) || node_has_doc(tree, declaration_id.id)
}

/// Return all docs attached to expression and declaration ownership.
pub fn expression_or_declaration_docs(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
) -> Vec<(ast::LocalNodeId<ast::Doc>, ast::AnnotationPosition)> {
    let mut docs = tree.get_docs_for(expression_id.id);
    docs.extend(tree.get_docs_for(declaration_id.id));
    docs
}

/// Return the first doc annotation from expression or declaration ownership.
pub fn first_expression_or_declaration_doc(
    tree: &ast::NodeTree,
    expression_id: ast::LocalNodeId<ast::Expression>,
    declaration_id: ast::LocalNodeId<ast::Declaration>,
) -> Option<(ast::LocalNodeId<ast::Doc>, ast::AnnotationPosition)> {
    expression_or_declaration_docs(tree, expression_id, declaration_id)
        .into_iter()
        .next()
}
