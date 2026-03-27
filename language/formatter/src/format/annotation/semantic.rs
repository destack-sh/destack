use ast::{
    AnnotationPosition, DependencyItem, DependencyMode, Expression, LocalNodeId, NodeParentIndex,
    NodeTree, NodeType, TokenSpan, TokenType,
};
use destack_ast as ast;
use destack_source::{File, Span};
use smallvec::SmallVec;

use super::attachment::{
    TriviaOwnerIndex, assignment_like_rhs_owner_for_doc_annotation,
    parenthesized_rhs_owner_for_doc_annotation, trailing_statement_owner_for_doc_annotation,
};
use super::ownership::{
    find_owner_at_or_after_token_with_node_type, promote_owner_to_declaration_ancestor,
    promote_owner_to_node_type_ancestor,
};
use crate::{Annotation, AnnotationEntry};

/// Return whether one target node id exists in projection storage.
pub(super) fn annotation_projection_has_target_node(
    by_node_id: &[SmallVec<[LocalNodeId<Annotation>; 4]>],
    target_node_id: u32,
) -> bool {
    (target_node_id as usize) < by_node_id.len()
}

/// Push one projected annotation when the target node id exists.
pub(super) fn push_annotation_projection_entry(
    entries: &mut Vec<AnnotationEntry>,
    by_node_id: &mut [SmallVec<[LocalNodeId<Annotation>; 4]>],
    target_node_id: u32,
    annotation: Annotation,
    span: Span,
) -> bool {
    if !annotation_projection_has_target_node(by_node_id, target_node_id) {
        return false;
    }

    let local_id = LocalNodeId::new(entries.len() as u32);
    entries.push(AnnotationEntry { annotation, span });
    by_node_id[target_node_id as usize].push(local_id);
    true
}

/// Sort node-local annotation ids by source span order.
pub(super) fn sort_annotation_projection_by_source(
    entries: &[AnnotationEntry],
    by_node_id: &mut [SmallVec<[LocalNodeId<Annotation>; 4]>],
) {
    for annotation_ids in by_node_id {
        if annotation_ids.len() <= 1 {
            continue;
        }

        annotation_ids.sort_by(
            |left: &LocalNodeId<Annotation>, right: &LocalNodeId<Annotation>| {
                let left_span = entries[left.id as usize].span;
                let right_span = entries[right.id as usize].span;
                left_span
                    .start
                    .cmp(&right_span.start)
                    .then(left_span.end.cmp(&right_span.end))
                    .then(left.id.cmp(&right.id))
            },
        );
    }
}

/// Return whether one declaration owner is the default export value of one export expression chain.
fn declaration_expression_owner_for_declaration_target(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    target_node_id: u32,
) -> Option<u32> {
    if tree.get_node_type(target_node_id) != NodeType::Declaration {
        return None;
    }

    let declaration_expression_node_id = parents.get_by_id(target_node_id)?;
    if tree.get_node_type(declaration_expression_node_id) != NodeType::Expression {
        return None;
    }

    let declaration_expression_id = LocalNodeId::<Expression>::new(declaration_expression_node_id);
    let Expression::Declaration(declaration_id) = tree.get(declaration_expression_id) else {
        return None;
    };
    if declaration_id.id != target_node_id {
        return None;
    }

    Some(declaration_expression_node_id)
}

/// Return whether one export expression references one expression as its default item value.
fn export_expression_has_default_item_value(
    tree: &NodeTree,
    export_expression_node_id: u32,
    declaration_expression_node_id: u32,
) -> bool {
    if tree.get_node_type(export_expression_node_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(export_expression_node_id);
    let Expression::Export { items, .. } = tree.get(expression_id) else {
        return false;
    };

    items.iter().any(|item_id| {
        let item = tree.get(*item_id);
        matches!(
            item,
            DependencyItem::Item {
                mode: DependencyMode::Default,
                value: Some(value_id),
                ..
            } if value_id.id == declaration_expression_node_id
        )
    })
}

/// Return whether one declaration owner is the default export value of one export expression chain.
fn declaration_is_default_export_value(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    target_node_id: u32,
) -> bool {
    let Some(declaration_expression_node_id) =
        declaration_expression_owner_for_declaration_target(tree, parents, target_node_id)
    else {
        return false;
    };

    let mut ancestor_id = parents.get_by_id(declaration_expression_node_id);
    while let Some(node_id) = ancestor_id {
        if export_expression_has_default_item_value(tree, node_id, declaration_expression_node_id) {
            return true;
        }

        ancestor_id = parents.get_by_id(node_id);
    }

    false
}

/// Return the first non-whitespace token index after one span.
fn first_non_whitespace_token_index_after_span(tokens: &[TokenSpan], span: Span) -> Option<usize> {
    let mut token_index = tokens.partition_point(|token| token.span.start < span.end);
    while let Some(token) = tokens.get(token_index) {
        if !matches!(token.token.ty, TokenType::Whitespace | TokenType::Newline) {
            return Some(token_index);
        }

        token_index += 1;
    }

    None
}

/// Resolve the declaration target for one doc comment directly before one decorator.
fn doc_decorator_declaration_target(
    tree: &NodeTree,
    owner_index: &TriviaOwnerIndex,
    token_index: usize,
) -> Option<u32> {
    let declaration_id = find_owner_at_or_after_token_with_node_type(
        tree,
        owner_index,
        token_index,
        NodeType::Declaration,
    )?;
    Some(declaration_id)
}

/// Resolve rhs projection target for one doc annotation in canonical priority order.
fn resolve_doc_rhs_projection_target(
    file: &File,
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    tokens: &[TokenSpan],
    annotation_span: Span,
) -> Option<(u32, AnnotationPosition)> {
    if let Some((trailing_owner, trailing_position)) = trailing_statement_owner_for_doc_annotation(
        file,
        tree,
        parents,
        owner_index,
        tokens,
        annotation_span,
    ) {
        return Some((trailing_owner, trailing_position));
    }

    if let Some((parenthesized_owner, parenthesized_position)) =
        parenthesized_rhs_owner_for_doc_annotation(file, tree, tokens, annotation_span)
    {
        return Some((parenthesized_owner, parenthesized_position));
    }

    assignment_like_rhs_owner_for_doc_annotation(
        file,
        tree,
        parents,
        owner_index,
        tokens,
        annotation_span,
    )
}

/// Resolve one semantic doc annotation target and optional position override.
fn resolve_doc_annotation_target(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    annotation_span: Span,
    default_target_node_id: u32,
) -> (u32, Option<AnnotationPosition>) {
    let mut target_node_id = default_target_node_id;
    let mut doc_position_override = None;

    // doc comments directly before decorators belong to the decorated declaration
    if let Some(token_index) = first_non_whitespace_token_index_after_span(tokens, annotation_span)
    {
        let token = tokens[token_index];
        if token.token.ty == TokenType::At
            && let Some(declaration_id) =
                doc_decorator_declaration_target(tree, owner_index, token_index)
        {
            target_node_id = declaration_id;
        }
    }

    // rhs projection order: trailing statement, parenthesized rhs, assignment rhs
    if let Some((target_node_id, position)) =
        resolve_doc_rhs_projection_target(file, tree, parents, owner_index, tokens, annotation_span)
    {
        doc_position_override = Some(position);
        return (target_node_id, doc_position_override);
    }

    (target_node_id, doc_position_override)
}

/// Resolve one decorator target owner from one first token after the annotation span.
fn resolve_decorator_target_at_token(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    token_index: usize,
) -> Option<u32> {
    if let Some(member_target) = find_owner_at_or_after_token_with_node_type(
        tree,
        owner_index,
        token_index,
        NodeType::Member,
    )
    .and_then(|owner| promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member))
    {
        return Some(member_target);
    }

    if let Some(property_target) = find_owner_at_or_after_token_with_node_type(
        tree,
        owner_index,
        token_index,
        NodeType::Property,
    )
    .and_then(|owner| promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Property))
    {
        return Some(property_target);
    }

    find_owner_at_or_after_token_with_node_type(
        tree,
        owner_index,
        token_index,
        NodeType::Declaration,
    )
    .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
}

/// Resolve one semantic decorator annotation target.
fn resolve_decorator_annotation_target(
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    annotation_span: Span,
    default_target_node_id: u32,
) -> u32 {
    if tree.get_node_type(default_target_node_id) != NodeType::Expression {
        return default_target_node_id;
    }

    let Some(token_index) = first_non_whitespace_token_index_after_span(tokens, annotation_span)
    else {
        return default_target_node_id;
    };

    if let Some(target_node_id) =
        resolve_decorator_target_at_token(tree, parents, owner_index, token_index)
    {
        return target_node_id;
    }

    default_target_node_id
}

/// Build one formatter annotation from one parser semantic annotation.
fn semantic_annotation(
    file: &File,
    tree: &NodeTree,
    parents: &NodeParentIndex,
    ast_annotation: &ast::Annotation,
    annotation_span: Span,
    target_node_id: u32,
    doc_position_override: Option<AnnotationPosition>,
) -> Annotation {
    match ast_annotation {
        ast::Annotation::Doc { node, position } => Annotation::Doc {
            node: *node,
            position: doc_position_override.unwrap_or(*position),
        },
        ast::Annotation::Decorator { node, position } => {
            let mut position = *position;

            // decorators that start on the owner's line should stay inline
            let owner_span = tree.get_span_by_id(target_node_id);
            let decorator_starts_on_owner_line = annotation_span.end > annotation_span.start
                && file.is_same_line(annotation_span.end.saturating_sub(1), owner_span.start);
            let owner_is_declaration = tree.get_node_type(target_node_id) == NodeType::Declaration;
            let owner_is_default_export_value =
                declaration_is_default_export_value(tree, parents, target_node_id);
            if position == AnnotationPosition::BlockPrefix
                && decorator_starts_on_owner_line
                && owner_is_declaration
                && owner_is_default_export_value
            {
                position = AnnotationPosition::LinePrefix;
            }

            Annotation::Decorator {
                node: *node,
                position,
            }
        }
    }
}

/// Resolve one semantic annotation target and optional doc-position override.
fn semantic_annotation_target_for_parser_annotation(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    ast_annotation: &ast::Annotation,
    annotation_span: Span,
    target_id: u32,
) -> (u32, Option<AnnotationPosition>) {
    match ast_annotation {
        ast::Annotation::Doc { .. } => resolve_doc_annotation_target(
            file,
            tree,
            tokens,
            parents,
            owner_index,
            annotation_span,
            target_id,
        ),
        ast::Annotation::Decorator { .. } => {
            let target_node_id = resolve_decorator_annotation_target(
                tree,
                tokens,
                parents,
                owner_index,
                annotation_span,
                target_id,
            );
            (target_node_id, None)
        }
    }
}

/// Project one parser semantic annotation into formatter projection storage.
fn project_parser_semantic_annotation(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    entries: &mut Vec<AnnotationEntry>,
    by_node_id: &mut [SmallVec<[LocalNodeId<Annotation>; 4]>],
    target_id: u32,
    annotation_id: LocalNodeId<ast::Annotation>,
) {
    let ast_annotation = tree.get(annotation_id);
    let annotation_span = tree.get_span(annotation_id);
    let (target_node_id, doc_position_override) = semantic_annotation_target_for_parser_annotation(
        file,
        tree,
        tokens,
        parents,
        owner_index,
        ast_annotation,
        annotation_span,
        target_id,
    );
    let annotation = semantic_annotation(
        file,
        tree,
        parents,
        ast_annotation,
        annotation_span,
        target_node_id,
        doc_position_override,
    );

    push_annotation_projection_entry(
        entries,
        by_node_id,
        target_node_id,
        annotation,
        annotation_span,
    );
}

/// Project parser side semantic annotations into formatter annotation entries.
pub(super) fn project_parser_semantic_annotations(
    file: &File,
    tree: &NodeTree,
    tokens: &[TokenSpan],
    parents: &NodeParentIndex,
    owner_index: &TriviaOwnerIndex,
    entries: &mut Vec<AnnotationEntry>,
    by_node_id: &mut [SmallVec<[LocalNodeId<Annotation>; 4]>],
) {
    // add parser semantic annotations first
    for (&target_id, annotation_ids) in tree.get_all_annotations() {
        if !annotation_projection_has_target_node(by_node_id, target_id) {
            continue;
        }

        for &annotation_id in annotation_ids {
            project_parser_semantic_annotation(
                file,
                tree,
                tokens,
                parents,
                owner_index,
                entries,
                by_node_id,
                target_id,
                annotation_id,
            );
        }
    }
}
