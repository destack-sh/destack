use ast::{
    AnnotationPosition, Argument, Declaration, Expression, FunctionMode, LocalNodeId,
    NodeParentIndex, NodeTree, NodeType, TokenType,
};
use destack_ast as ast;
use destack_source::Span;

use super::attachment::{
    FormatterTriviaOwnerIndex, attach_trailing_comma_close_brace_property_line_comment,
    if_expression_then_owner_without_else, is_empty_dependency_expression_owner,
    promote_owner_to_tree_expression_parent,
};
use super::boundary::{
    CommentAttachment, CommentAttachmentNeighbors, CommentEnclosingOwnerCache, CommentSeamContext,
    CommentSeamData, CommentSeamKeyword, comment_enclosing_owner,
    seam_is_template_interpolation_open_brace,
};
use super::facts::{
    is_tree_closing_tag_head_seam, next_non_trivia_token_index, previous_non_trivia_token_index,
};
use super::operator::try_attach_comment_expression_operator;
use super::ownership::{
    find_owner_at_or_after_token, find_owner_in_ancestor_chain, find_owner_in_candidate_ancestry,
    find_preferred_owner_starting_at, find_smallest_owner_enclosing_range,
    find_smallest_owner_enclosing_token, lowest_common_owner_ancestor,
    normalize_formatter_trivia_target_owner, normalize_owner_with_shared_end,
    promote_owner_by_shared_start, promote_owner_to_declaration_ancestor,
    promote_owner_to_node_type_ancestor, promote_owner_to_parenthesized_expression_ancestor,
};
use super::semicolon::attach_semicolon_guard_own_line_comment;

/// Return the first dynamic argument owner for one call-like expression.
pub(super) fn first_dynamic_argument_owner_for_call_like(
    tree: &NodeTree,
    owner_id: u32,
) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    match tree.get(expression_id) {
        Expression::Call {
            dynamic_arguments, ..
        }
        | Expression::New {
            dynamic_arguments, ..
        } => dynamic_arguments.first().map(|argument_id| argument_id.id),
        Expression::Import { arguments, .. } => arguments.as_ref().and_then(|dynamic_arguments| {
            dynamic_arguments.first().map(|argument_id| argument_id.id)
        }),
        _ => None,
    }
}

/// Return whether one call-like expression has static arguments.
fn call_like_expression_has_static_arguments(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    match tree.get(expression_id) {
        Expression::Call {
            static_arguments, ..
        }
        | Expression::New {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Import { .. } => false,
        _ => false,
    }
}

/// Return the callee owner for one call-like expression.
fn call_like_callee_owner(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    match tree.get(expression_id) {
        Expression::Call { left, .. } | Expression::New { left, .. } => Some(left.id),
        Expression::Import { .. } => None,
        _ => None,
    }
}

/// Normalize one call-callee seam owner while preserving explicit parenthesized wrappers.
fn normalize_call_callee_seam_owner(tree: &NodeTree, owner_id: u32) -> u32 {
    if tree.get_node_type(owner_id) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(owner_id);
        if matches!(tree.get(expression_id), Expression::Parenthesized { .. }) {
            return owner_id;
        }
    }

    normalize_formatter_trivia_target_owner(tree, owner_id)
}

/// Return whether one owner is one call-like expression.
fn is_call_like_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. } | Expression::Import { .. }
    )
}

/// Return whether one seam is a call-like callee boundary before `(`.
fn seam_is_call_like_callee_boundary(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    call_owner: u32,
    preceding_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
) -> bool {
    let Some(callee_owner) = call_like_callee_owner(tree, call_owner) else {
        return false;
    };
    let callee_owner = normalize_formatter_trivia_target_owner(tree, callee_owner);

    [preceding_owner, preceding_token_owner]
        .into_iter()
        .flatten()
        .any(|candidate_owner| {
            let mut current_owner = Some(candidate_owner);
            while let Some(owner_id) = current_owner {
                let normalized_owner = normalize_formatter_trivia_target_owner(tree, owner_id);
                if normalized_owner == callee_owner {
                    return true;
                }

                if owner_id == call_owner || normalized_owner == call_owner {
                    break;
                }

                current_owner = parents.get_by_id(owner_id);
            }

            false
        })
}

/// Return whether one owner belongs to one call-like expression ancestry.
fn owner_has_call_like_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    let mut current_owner = Some(owner_id);
    while let Some(current_id) = current_owner {
        if is_call_like_expression_owner(tree, current_id) {
            return true;
        }

        current_owner = parents.get_by_id(current_id);
    }

    false
}

/// Return whether one token type is one assignment operator token.
fn token_type_is_assignment_operator(token_type: Option<TokenType>) -> bool {
    token_type.is_some_and(|token_type| ast::AssignOperator::from_token(token_type).is_some())
}

/// Promote one owner to the nearest labelled expression ancestor.
fn promote_owner_to_labelled_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        if tree.get_node_type(node_id) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(node_id);
            if matches!(tree.get(expression_id), Expression::Labelled { .. }) {
                return Some(node_id);
            }
        }

        None
    })
}

/// Return whether one owner belongs to a `new (...) => ...` declaration signature.
fn is_new_signature_declaration_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    let Some(declaration_owner) = promote_owner_to_declaration_ancestor(tree, parents, owner_id)
    else {
        return false;
    };

    if tree.get_node_type(declaration_owner) != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(declaration_owner);
    let Declaration::Function { signature, .. } = tree.get(declaration_id) else {
        return false;
    };

    signature.mode == Some(FunctionMode::New)
}

/// Return whether one owner is a ternary if expression.
fn is_ternary_if_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::If {
            kind: ast::IfKind::Ternary,
            ..
        } | Expression::TypeConditional { .. }
    )
}

/// Return whether one owner is a call or new expression.
fn is_call_or_new_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. }
    )
}

/// Return whether one owner is one import call expression.
fn is_import_call_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    matches!(
        tree.get(expression_id),
        Expression::Import {
            source: ast::ImportSource::ImportCall,
            ..
        }
    )
}

/// Return whether one owner is one import or export dependency statement expression with a attribute.
fn is_dependency_attribute_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    match tree.get(expression_id) {
        Expression::Import {
            source, arguments, ..
        } => *source != ast::ImportSource::ImportCall && arguments.is_some(),
        Expression::Export { arguments, .. } => arguments.is_some(),
        _ => false,
    }
}

/// Return one dependency attribute owner candidate from one seam.
fn dependency_attribute_expression_owner_from_seam_candidates(
    tree: &NodeTree,
    candidates: [Option<u32>; 5],
) -> Option<u32> {
    candidates
        .into_iter()
        .flatten()
        .find(|owner_id| is_dependency_attribute_expression_owner(tree, *owner_id))
}

/// Return one empty dependency owner candidate from one seam.
fn empty_dependency_expression_owner_from_seam_candidates(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    candidates: [Option<u32>; 5],
) -> Option<u32> {
    candidates.into_iter().flatten().find_map(|owner_id| {
        let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
            Some(owner_id)
        } else {
            promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)
        }?;
        is_empty_dependency_expression_owner(tree, expression_owner).then_some(expression_owner)
    })
}

/// Return whether one owner is a tree expression owner.
fn is_tree_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    tree.get_node_type(owner_id) == NodeType::Expression
        && matches!(
            tree.get(LocalNodeId::<Expression>::new(owner_id)),
            Expression::TreeExpression { .. }
        )
}

/// Return whether one seam is inside one JSX closing-tag head.
fn seam_is_tree_closing_tag_head(
    seam_ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    let token_before_predecessor_type = seam_ctx
        .token_before
        .and_then(|token_before_index| {
            previous_non_trivia_token_index(seam_ctx.semantic_tokens, token_before_index)
        })
        .and_then(|token_index| seam_ctx.semantic_tokens.get(token_index))
        .map(|token| token.token.ty);
    let token_after_successor_type = seam_ctx
        .token_after
        .and_then(|token_after_index| {
            next_non_trivia_token_index(seam_ctx.semantic_tokens, token_after_index)
        })
        .and_then(|token_index| seam_ctx.semantic_tokens.get(token_index))
        .map(|token| token.token.ty);

    is_tree_closing_tag_head_seam(
        seam.token_before_type,
        token_before_predecessor_type,
        seam.token_after_type,
        token_after_successor_type,
    )
}

/// Return one tree-expression owner for one JSX closing-tag seam.
fn tree_expression_owner_for_closing_tag_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
    following_token_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<u32> {
    [
        preceding_token_owner,
        following_token_owner,
        preceding_owner,
        following_owner,
        enclosing_owner,
    ]
    .into_iter()
    .flatten()
    .map(|owner_id| promote_owner_to_tree_expression_parent(tree, parents, owner_id))
    .find(|owner_id| is_tree_expression_owner(tree, *owner_id))
}

/// Promote one owner to the nearest call-like expression ancestor.
pub(super) fn promote_owner_to_call_like_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |candidate_id| {
        if is_call_or_new_expression_owner(tree, candidate_id)
            || is_import_call_expression_owner(tree, candidate_id)
        {
            return Some(candidate_id);
        }

        None
    })
}

/// Return whether one owner has one unary-expression ancestor.
fn owner_has_unary_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    find_owner_in_ancestor_chain(parents, owner_id, |candidate_id| {
        if tree.get_node_type(candidate_id) != NodeType::Expression {
            return None;
        }

        let expression_id = LocalNodeId::<Expression>::new(candidate_id);
        matches!(
            tree.get(expression_id),
            Expression::Unary { .. } | Expression::TypeUnary { .. }
        )
        .then_some(candidate_id)
    })
    .is_some()
}

/// Promote one owner to the nearest spread-argument ancestor.
fn promote_owner_to_spread_argument_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let argument_owner =
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Argument)?;
    let argument_id = LocalNodeId::<Argument>::new(argument_owner);
    matches!(tree.get(argument_id), Argument::Spread { .. }).then_some(argument_owner)
}

/// Return one empty object expression owner for one candidate owner node.
fn empty_object_expression_owner_for_candidate(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(owner_id);
        if matches!(
            tree.get(expression_id),
            Expression::ObjectExpression { properties, .. } if properties.is_empty()
        ) {
            return Some(owner_id);
        }
    }

    if tree.get_node_type(owner_id) != NodeType::Argument {
        return None;
    }

    let argument_id = LocalNodeId::<Argument>::new(owner_id);
    let value_id = match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
        Argument::Error => return None,
    };
    if matches!(
        tree.get(value_id),
        Expression::ObjectExpression { properties, .. } if properties.is_empty()
    ) {
        return Some(value_id.id);
    }

    None
}

/// Return whether one token is the terminal token of one optional-call operator.
fn token_before_is_optional_call_operator_terminal(
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    if seam.token_before_is(TokenType::Maybe) {
        return true;
    }

    if !seam.token_before_is(TokenType::Dot) {
        return false;
    }

    let Some(token_before_index) = ctx.token_before else {
        return false;
    };
    token_before_index > 0
        && ctx.semantic_tokens[token_before_index - 1].token.ty == TokenType::Maybe
}

/// Return whether one seam is one optional-call operator seam around `?.`.
fn seam_is_optional_call_operator(ctx: &CommentSeamContext<'_>, seam: &CommentSeamData) -> bool {
    let seam_is_between_operator_and_parenthesis = seam.token_after_is(TokenType::OpenParenthesis)
        && token_before_is_optional_call_operator_terminal(ctx, seam);
    let seam_is_between_operator_tokens =
        seam.token_before_is(TokenType::Maybe) && seam.token_after_is(TokenType::Dot);

    seam_is_between_operator_and_parenthesis || seam_is_between_operator_tokens
}

/// Return one optional-call base owner for one optional-call expression owner.
fn optional_call_expression_base_owner(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    let Expression::Call { left, .. } = tree.get(expression_id) else {
        return None;
    };

    let left_expression = tree.get(*left);
    let Expression::Maybe {
        left: maybe_left, ..
    } = left_expression
    else {
        return None;
    };

    Some(maybe_left.id)
}

/// Return one optional-call maybe owner for one optional-call expression owner.
fn optional_call_expression_maybe_owner(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    let Expression::Call { left, .. } = tree.get(expression_id) else {
        return None;
    };

    matches!(tree.get(*left), Expression::Maybe { .. }).then_some(left.id)
}

/// Return one binary/logical owner when one close-paren seam wraps its right operand.
fn binary_owner_with_parenthesized_right_operand(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    let expression_owner = if tree.get_node_type(owner_id) == NodeType::Expression {
        owner_id
    } else {
        promote_owner_to_node_type_ancestor(tree, parents, owner_id, NodeType::Expression)?
    };
    let parenthesized_owner =
        promote_owner_to_parenthesized_expression_ancestor(tree, parents, expression_owner)?;
    let parent_expression_owner = parents
        .get_by_id(parenthesized_owner)
        .filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)?;
    let parent_expression_id = LocalNodeId::<Expression>::new(parent_expression_owner);

    match tree.get(parent_expression_id) {
        Expression::Binary { right, .. } | Expression::TypeBinary { right, .. }
            if right.id == parenthesized_owner =>
        {
            Some(parent_expression_owner)
        }
        _ => None,
    }
}

/// Return whether this owner is one optional-call expression.
fn is_optional_call_expression_owner(tree: &NodeTree, owner_id: u32) -> bool {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return false;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    let Expression::Call { left, .. } = tree.get(expression_id) else {
        return false;
    };

    matches!(tree.get(*left), Expression::Maybe { .. })
}

/// Return one ordered candidate-owner list for one optional-call seam.
fn optional_call_seam_candidate_owners(
    preceding_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
    following_owner: Option<u32>,
    following_token_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> [Option<u32>; 5] {
    [
        preceding_owner,
        preceding_token_owner,
        following_owner,
        following_token_owner,
        enclosing_owner,
    ]
}

/// Return one base owner for one maybe-expression owner.
fn maybe_expression_base_owner(tree: &NodeTree, owner_id: u32) -> Option<u32> {
    if tree.get_node_type(owner_id) != NodeType::Expression {
        return None;
    }

    let expression_id = LocalNodeId::<Expression>::new(owner_id);
    let Expression::Maybe { left, .. } = tree.get(expression_id) else {
        return None;
    };

    Some(left.id)
}

/// Return one labelled-statement owner near one label-colon seam.
fn labelled_statement_owner_for_colon_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
) -> Option<u32> {
    preceding_owner
        .and_then(|owner| promote_owner_to_labelled_expression_ancestor(tree, parents, owner))
        .or_else(|| {
            following_owner.and_then(|owner| {
                promote_owner_to_labelled_expression_ancestor(tree, parents, owner)
            })
        })
}

/// Return one optional-call expression owner for one optional-call seam.
fn optional_call_seam_expression_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
    following_owner: Option<u32>,
    following_token_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<u32> {
    find_owner_in_candidate_ancestry(
        parents,
        optional_call_seam_candidate_owners(
            preceding_owner,
            preceding_token_owner,
            following_owner,
            following_token_owner,
            enclosing_owner,
        ),
        |owner_id| is_optional_call_expression_owner(tree, owner_id).then_some(owner_id),
    )
}

/// Return one optional-call base owner for one optional-call seam.
fn optional_call_seam_base_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
    following_owner: Option<u32>,
    following_token_owner: Option<u32>,
    enclosing_owner: Option<u32>,
) -> Option<u32> {
    find_owner_in_candidate_ancestry(
        parents,
        optional_call_seam_candidate_owners(
            preceding_owner,
            preceding_token_owner,
            following_owner,
            following_token_owner,
            enclosing_owner,
        ),
        |owner_id| {
            maybe_expression_base_owner(tree, owner_id)
                .or_else(|| optional_call_expression_base_owner(tree, owner_id))
        },
    )
}

/// Return one annotation position for one optional-call seam comment.
fn optional_call_seam_position(
    comment_is_line: bool,
    has_leading_newline: bool,
    has_trailing_newline: bool,
) -> AnnotationPosition {
    if comment_is_line {
        if has_leading_newline {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::LinePostfix
        }
    } else if has_leading_newline || has_trailing_newline {
        AnnotationPosition::LinePostfixBoundary
    } else {
        AnnotationPosition::LinePostfix
    }
}

/// Return one normalized preceding expression owner for one tail seam.
fn normalized_preceding_expression_owner_for_tail_seam(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_token_owner: Option<u32>,
    preceding_owner: Option<u32>,
    token_before_source_span: Option<Span>,
) -> Option<u32> {
    let mut target_owner = preceding_token_owner.or(preceding_owner)?;
    target_owner =
        normalize_owner_with_shared_end(tree, parents, target_owner, token_before_source_span);

    if tree.get_node_type(target_owner) != NodeType::Expression
        && let Some(expression_target) =
            promote_owner_to_node_type_ancestor(tree, parents, target_owner, NodeType::Expression)
    {
        target_owner = expression_target;
    }

    Some(normalize_formatter_trivia_target_owner(tree, target_owner))
}

/// Return whether seam candidates have one call-like expression ancestor.
fn seam_candidates_have_call_like_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam_candidates: [Option<u32>; 5],
) -> bool {
    seam_candidates
        .into_iter()
        .flatten()
        .any(|owner| owner_has_call_like_expression_ancestor(tree, parents, owner))
}

/// Return one first call-like expression owner from seam candidates.
fn first_call_like_expression_owner_from_seam_candidates(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam_candidates: [Option<u32>; 5],
) -> Option<u32> {
    seam_candidates
        .into_iter()
        .flatten()
        .find_map(|owner| promote_owner_to_call_like_expression_ancestor(tree, parents, owner))
}

/// Return whether one owner belongs to a do-while expression ancestry.
fn owner_has_do_while_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> bool {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        if tree.get_node_type(node_id) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(node_id)),
                Expression::While {
                    kind: ast::WhileKind::DoWhile,
                    ..
                }
            )
        {
            return Some(node_id);
        }

        None
    })
    .is_some()
}

/// Return whether one expression owner is one direct lambda body expression.
fn is_direct_lambda_body_expression_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    expression_owner: u32,
) -> bool {
    if tree.get_node_type(expression_owner) != NodeType::Expression {
        return false;
    }

    let Some(parent_owner) = parents.get_by_id(expression_owner) else {
        return false;
    };
    if tree.get_node_type(parent_owner) != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_owner);
    let Declaration::Function {
        signature,
        body: Some(body),
        ..
    } = tree.get(declaration_id)
    else {
        return false;
    };

    signature.kind == ast::FunctionKind::Lambda && body.id == expression_owner
}

/// Promote one owner to the nearest tagged-template expression ancestor.
fn promote_owner_to_tagged_template_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        if tree.get_node_type(node_id) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(node_id)),
                Expression::TaggedTemplateExpression { .. }
            )
        {
            return Some(node_id);
        }

        None
    })
}

/// Promote one owner to the nearest mapped-type expression ancestor.
fn promote_owner_to_mapped_type_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |node_id| {
        if tree.get_node_type(node_id) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(node_id)),
                Expression::TypeMapped { .. }
            )
        {
            return Some(node_id);
        }

        None
    })
}

/// Comment placement categories used for attachment routing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpressionCommentPlacement {
    /// Comment starts on its own line.
    OwnLine,
    /// Comment ends one line.
    EndOfLine,
    /// Comment stays inline.
    Remaining,
}

/// Classify one seam comment by newline boundary shape.
fn expression_comment_placement(
    has_leading_newline: bool,
    has_trailing_newline: bool,
) -> ExpressionCommentPlacement {
    if has_leading_newline {
        return ExpressionCommentPlacement::OwnLine;
    }

    if has_trailing_newline {
        return ExpressionCommentPlacement::EndOfLine;
    }

    ExpressionCommentPlacement::Remaining
}

/// Prepared ctx for one expression seam comment attachment decision.
#[derive(Clone, Copy)]
struct ExpressionCommentContext<'a, 'ctx> {
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// Seam ctx.
    seam_ctx: &'a CommentSeamContext<'ctx>,
    /// Seam facts.
    seam: &'a CommentSeamData,
    /// Neighbor owner candidates.
    owners: CommentAttachmentNeighbors,
    /// Neighbor owner before seam.
    preceding_owner: Option<u32>,
    /// Neighbor owner after seam.
    following_owner: Option<u32>,
    /// Token before seam.
    token_before_span: Option<ast::TokenSpan>,
    /// Token after seam.
    token_after_span: Option<ast::TokenSpan>,
    /// Source span before seam.
    token_before_source_span: Option<Span>,
    /// Source span after seam.
    token_after_source_span: Option<Span>,
    /// Token-owner candidate before seam.
    preceding_token_owner: Option<u32>,
    /// Token-owner candidate after seam.
    following_token_owner: Option<u32>,
    /// Enclosing owner candidate.
    enclosing_owner: Option<u32>,
    /// Comment has leading newline.
    has_leading_newline: bool,
    /// Comment has trailing newline.
    has_trailing_newline: bool,
    /// Comment is line comment.
    comment_is_line: bool,
    /// Comment is block comment.
    comment_is_star: bool,
    /// Placement class.
    placement: ExpressionCommentPlacement,
    /// Comment is inline block comment.
    is_inline_star_comment: bool,
    /// Comment is trailing line comment.
    is_trailing_line_comment: bool,
    /// Token after seam is spread.
    token_after_is_spread: bool,
    /// Token after seam is close brace.
    token_after_is_close_brace: bool,
    /// Token after seam is decorator marker.
    token_after_is_at: bool,
    /// Token before seam is open brace.
    token_before_is_open_brace: bool,
    /// Token before seam is spread.
    token_before_is_spread: bool,
    /// Seam followed by logical operator after close parenthesis.
    token_after_close_parenthesis_is_logical_operator: bool,
    /// Seam belongs to `new (...) => ...` declaration shape.
    seam_is_new_signature_declaration: bool,
    /// Seam neighbors share one expression owner.
    seam_has_shared_expression_owner: bool,
    /// Enclosing ternary owner for this seam.
    ternary_enclosing_owner: Option<u32>,
    /// Seam is ternary seam.
    is_ternary_seam: bool,
    /// Following owner normalized for ternary seam.
    ternary_following_owner: Option<u32>,
    /// First dynamic argument owner for enclosing call/new.
    enclosing_owner_first_dynamic_argument: Option<u32>,
    /// Control-head semicolon owns empty body.
    semicolon_after_control_head_empty_body: bool,
}

impl ExpressionCommentContext<'_, '_> {
    /// Return whether this seam should route to statement-level separator ownership.
    fn should_route_call_like_separator_to_statement(self) -> bool {
        let token_after_close_parenthesis_following_type =
            token_after_close_parenthesis_following_type(self.seam_ctx);
        let has_separator_before_close_parenthesis =
            self.seam.token_after_is(TokenType::CloseParenthesis)
                || (self.seam.token_after_is(TokenType::Comma)
                    && token_after_close_parenthesis_following_type
                        == Some(TokenType::CloseParenthesis));
        let seam_is_call_like_closing_separator = has_separator_before_close_parenthesis
            && seam_candidates_have_call_like_expression_ancestor(
                self.tree,
                self.parents,
                expression_seam_candidate_owners(&self),
            );
        let supports_separator_comment =
            self.comment_is_line || (self.comment_is_star && self.has_leading_newline);

        supports_separator_comment
            && self.has_leading_newline
            && has_separator_before_close_parenthesis
            && seam_is_call_like_closing_separator
    }
}

/// Derived ternary seam facts for expression comment routing.
#[derive(Debug, Clone, Copy)]
struct ExpressionTernaryFacts {
    /// Enclosing ternary owner for this seam.
    ternary_enclosing_owner: Option<u32>,
    /// Whether this seam belongs to a ternary expression.
    is_ternary_seam: bool,
    /// Following owner normalized for ternary seam routing.
    ternary_following_owner: Option<u32>,
}

/// Build ternary seam facts from seam owners and token boundaries.
fn build_expression_ternary_facts(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    shared_owner: Option<u32>,
    enclosing_owner: Option<u32>,
    following_owner: Option<u32>,
    following_token_owner: Option<u32>,
    token_before_source_span: Option<Span>,
    token_after_source_span: Option<Span>,
) -> ExpressionTernaryFacts {
    let shared_ternary_owner =
        shared_owner.filter(|owner| is_ternary_if_expression_owner(tree, *owner));
    let seam_enclosing_ternary_owner = token_before_source_span
        .zip(token_after_source_span)
        .and_then(|(before_span, after_span)| {
            find_smallest_owner_enclosing_range(tree, before_span.start, after_span.end)
        })
        .and_then(|owner| {
            if tree.get_node_type(owner) == NodeType::Expression {
                Some(owner)
            } else {
                promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Expression)
            }
        })
        .filter(|owner| is_ternary_if_expression_owner(tree, *owner));
    let ternary_enclosing_owner = shared_ternary_owner
        .or(enclosing_owner.filter(|owner| is_ternary_if_expression_owner(tree, *owner)))
        .or(seam_enclosing_ternary_owner);
    let is_ternary_seam = ternary_enclosing_owner.is_some();
    let ternary_following_owner = if is_ternary_seam {
        following_token_owner.or(following_owner).map(|owner| {
            let owner = token_after_source_span.map_or(owner, |span| {
                promote_owner_by_shared_start(tree, parents, owner, span.start)
            });
            let owner = promote_owner_to_tree_expression_parent(tree, parents, owner);
            normalize_formatter_trivia_target_owner(tree, owner)
        })
    } else {
        None
    };

    ExpressionTernaryFacts {
        ternary_enclosing_owner,
        is_ternary_seam,
        ternary_following_owner,
    }
}

/// Return whether one semicolon seam is the empty body after a control head.
fn seam_is_control_head_semicolon_empty_body(
    tree: &NodeTree,
    seam: &CommentSeamData,
    following_owner: Option<u32>,
) -> bool {
    seam.token_before_is(TokenType::CloseParenthesis)
        && seam.token_after_is(TokenType::Semicolon)
        && following_owner.is_some_and(|owner| {
            if tree.get_node_type(owner) == NodeType::Block {
                return true;
            }

            if tree.get_node_type(owner) != NodeType::Expression {
                return false;
            }

            matches!(
                tree.get(LocalNodeId::<Expression>::new(owner)),
                Expression::Block(_)
            )
        })
}

/// Placement facts derived from one seam newline shape.
#[derive(Clone, Copy)]
struct ExpressionCommentPlacementFacts {
    /// Placement class for this comment seam.
    placement: ExpressionCommentPlacement,
    /// Whether this is one inline block comment.
    is_inline_star_comment: bool,
    /// Whether this is one trailing line comment.
    is_trailing_line_comment: bool,
}

/// Build placement facts for one expression seam comment.
fn build_expression_comment_placement_facts(
    seam: &CommentSeamData,
) -> ExpressionCommentPlacementFacts {
    let placement =
        expression_comment_placement(seam.has_leading_newline, seam.has_trailing_newline);
    let is_inline_star_comment =
        placement == ExpressionCommentPlacement::Remaining && seam.comment_is_star;
    let is_trailing_line_comment =
        placement == ExpressionCommentPlacement::EndOfLine && seam.comment_is_line;

    ExpressionCommentPlacementFacts {
        placement,
        is_inline_star_comment,
        is_trailing_line_comment,
    }
}

/// Boundary token facts used by expression seam routing.
#[derive(Clone, Copy)]
struct ExpressionCommentBoundaryFacts {
    /// Token after seam is spread.
    token_after_is_spread: bool,
    /// Token after seam is close brace.
    token_after_is_close_brace: bool,
    /// Token after seam is decorator marker.
    token_after_is_at: bool,
    /// Token before seam is open brace.
    token_before_is_open_brace: bool,
    /// Token before seam is spread.
    token_before_is_spread: bool,
    /// Token after close parenthesis is one logical operator.
    token_after_close_parenthesis_is_logical_operator: bool,
}

/// Build boundary token facts for one expression seam.
fn build_expression_comment_boundary_facts(
    seam_ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> ExpressionCommentBoundaryFacts {
    let token_after_close_parenthesis_following_type =
        token_after_close_parenthesis_following_type(seam_ctx);
    let token_after_close_parenthesis_is_logical_operator = matches!(
        token_after_close_parenthesis_following_type,
        Some(TokenType::LogicalAnd | TokenType::LogicalOr | TokenType::Coalesce)
    );

    ExpressionCommentBoundaryFacts {
        token_after_is_spread: seam.token_after_is(TokenType::Spread),
        token_after_is_close_brace: seam.token_after_is(TokenType::CloseBrace),
        token_after_is_at: seam.token_after_is(TokenType::At),
        token_before_is_open_brace: seam.token_before_is(TokenType::OpenBrace),
        token_before_is_spread: seam.token_before_is(TokenType::Spread),
        token_after_close_parenthesis_is_logical_operator,
    }
}

/// Owner facts used by expression seam routing.
#[derive(Clone, Copy)]
struct ExpressionCommentOwnerFacts {
    /// Enclosing owner candidate.
    enclosing_owner: Option<u32>,
    /// Token owner before seam.
    preceding_token_owner: Option<u32>,
    /// Token owner after seam.
    following_token_owner: Option<u32>,
    /// Whether seam neighbors belong to `new (...) => ...`.
    seam_is_new_signature_declaration: bool,
    /// Whether seam neighbors share one expression owner.
    seam_has_shared_expression_owner: bool,
    /// Ternary seam facts.
    ternary_facts: ExpressionTernaryFacts,
    /// First dynamic argument of the enclosing call-like owner.
    enclosing_owner_first_dynamic_argument: Option<u32>,
    /// Whether seam is one control-head empty-body semicolon.
    semicolon_after_control_head_empty_body: bool,
}

/// Build owner facts for one expression seam.
#[allow(clippy::too_many_arguments)]
fn build_expression_comment_owner_facts(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    seam_ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    preceding_owner: Option<u32>,
    following_owner: Option<u32>,
    token_before_source_span: Option<Span>,
    token_after_source_span: Option<Span>,
) -> ExpressionCommentOwnerFacts {
    let enclosing_owner = comment_enclosing_owner(seam_ctx, enclosing_owner_cache);
    let preceding_token_owner = seam_ctx
        .token_before_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let following_token_owner = seam_ctx
        .token_after_span
        .and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));

    let seam_is_new_signature_declaration = preceding_owner
        .is_some_and(|owner| is_new_signature_declaration_owner(tree, parents, owner))
        || following_owner
            .is_some_and(|owner| is_new_signature_declaration_owner(tree, parents, owner));

    let shared_owner =
        preceding_owner
            .zip(following_owner)
            .and_then(|(left_owner, right_owner)| {
                lowest_common_owner_ancestor(tree, parents, left_owner, right_owner)
            });
    let seam_has_shared_expression_owner =
        shared_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Expression);

    let ternary_facts = build_expression_ternary_facts(
        tree,
        parents,
        shared_owner,
        enclosing_owner,
        following_owner,
        following_token_owner,
        token_before_source_span,
        token_after_source_span,
    );

    let enclosing_owner_first_dynamic_argument =
        enclosing_owner.and_then(|owner| first_dynamic_argument_owner_for_call_like(tree, owner));

    let semicolon_after_control_head_empty_body =
        seam_is_control_head_semicolon_empty_body(tree, seam, following_owner);

    ExpressionCommentOwnerFacts {
        enclosing_owner,
        preceding_token_owner,
        following_token_owner,
        seam_is_new_signature_declaration,
        seam_has_shared_expression_owner,
        ternary_facts,
        enclosing_owner_first_dynamic_argument,
        semicolon_after_control_head_empty_body,
    }
}

/// Build one expression seam comment ctx.
fn build_expression_comment_ctx<'a, 'ctx>(
    tree: &'a NodeTree,
    parents: &'a NodeParentIndex,
    seam_ctx: &'a CommentSeamContext<'ctx>,
    seam: &'a CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> ExpressionCommentContext<'a, 'ctx> {
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;
    let token_before_span = seam_ctx.token_before_span;
    let token_after_span = seam_ctx.token_after_span;
    let token_before_source_span = token_before_span.map(|token| token.span);
    let token_after_source_span = token_after_span.map(|token| token.span);
    let placement_facts = build_expression_comment_placement_facts(seam);
    let boundary_facts = build_expression_comment_boundary_facts(seam_ctx, seam);
    let owner_facts = build_expression_comment_owner_facts(
        tree,
        parents,
        seam_ctx,
        seam,
        enclosing_owner_cache,
        preceding_owner,
        following_owner,
        token_before_source_span,
        token_after_source_span,
    );

    ExpressionCommentContext {
        tree,
        parents,
        seam_ctx,
        seam,
        owners,
        preceding_owner,
        following_owner,
        token_before_span,
        token_after_span,
        token_before_source_span,
        token_after_source_span,
        preceding_token_owner: owner_facts.preceding_token_owner,
        following_token_owner: owner_facts.following_token_owner,
        enclosing_owner: owner_facts.enclosing_owner,
        has_leading_newline: seam.has_leading_newline,
        has_trailing_newline: seam.has_trailing_newline,
        comment_is_line: seam.comment_is_line,
        comment_is_star: seam.comment_is_star,
        placement: placement_facts.placement,
        is_inline_star_comment: placement_facts.is_inline_star_comment,
        is_trailing_line_comment: placement_facts.is_trailing_line_comment,
        token_after_is_spread: boundary_facts.token_after_is_spread,
        token_after_is_close_brace: boundary_facts.token_after_is_close_brace,
        token_after_is_at: boundary_facts.token_after_is_at,
        token_before_is_open_brace: boundary_facts.token_before_is_open_brace,
        token_before_is_spread: boundary_facts.token_before_is_spread,
        token_after_close_parenthesis_is_logical_operator: boundary_facts
            .token_after_close_parenthesis_is_logical_operator,
        seam_is_new_signature_declaration: owner_facts.seam_is_new_signature_declaration,
        seam_has_shared_expression_owner: owner_facts.seam_has_shared_expression_owner,
        ternary_enclosing_owner: owner_facts.ternary_facts.ternary_enclosing_owner,
        is_ternary_seam: owner_facts.ternary_facts.is_ternary_seam,
        ternary_following_owner: owner_facts.ternary_facts.ternary_following_owner,
        enclosing_owner_first_dynamic_argument: owner_facts.enclosing_owner_first_dynamic_argument,
        semicolon_after_control_head_empty_body: owner_facts
            .semicolon_after_control_head_empty_body,
    }
}

/// Handle unary `!` chain seam comments in head placement.
fn attach_expression_head_unary_not_chain_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_trailing_line_comment
        || !ctx.seam.token_before_is(TokenType::Not)
        || !ctx.seam.token_after_is(TokenType::Not)
    {
        return None;
    }

    // trailing line comments between chained unary `!` heads stay on the following unary operand
    let target_owner = ctx.following_token_owner.or(ctx.following_owner)?;
    let target_owner = ctx.token_after_source_span.map_or(target_owner, |span| {
        promote_owner_by_shared_start(ctx.tree, ctx.parents, target_owner, span.start)
    });
    let target_owner = if ctx.tree.get_node_type(target_owner) == NodeType::Expression {
        target_owner
    } else {
        promote_owner_to_node_type_ancestor(
            ctx.tree,
            ctx.parents,
            target_owner,
            NodeType::Expression,
        )
        .unwrap_or(target_owner)
    };
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Handle unary head comments before parenthesized operands in head placement.
fn attach_expression_head_unary_parenthesized_operand_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_trailing_line_comment || !ctx.seam.token_after_is(TokenType::OpenParenthesis) {
        return None;
    }

    let seam_has_unary_ancestor = [ctx.preceding_token_owner, ctx.preceding_owner]
        .into_iter()
        .flatten()
        .any(|owner| owner_has_unary_expression_ancestor(ctx.tree, ctx.parents, owner));
    if !seam_has_unary_ancestor {
        return None;
    }

    // trailing line comments after unary heads before one parenthesized operand stay on the operand prefix
    let target_owner = ctx
        .token_after_span
        .and_then(|token_after_span| {
            find_preferred_owner_starting_at(ctx.tree, token_after_span.span)
        })
        .or(ctx.following_token_owner)
        .or(ctx.following_owner)?;
    let target_owner = ctx.token_after_source_span.map_or(target_owner, |span| {
        promote_owner_by_shared_start(ctx.tree, ctx.parents, target_owner, span.start)
    });
    let target_owner = if ctx.tree.get_node_type(target_owner) == NodeType::Expression {
        target_owner
    } else {
        promote_owner_to_node_type_ancestor(
            ctx.tree,
            ctx.parents,
            target_owner,
            NodeType::Expression,
        )
        .unwrap_or(target_owner)
    };
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Handle template interpolation `${` seam comments in head placement.
fn attach_expression_head_template_interpolation_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.comment_is_line || !seam_is_template_interpolation_open_brace(ctx.seam_ctx, ctx.seam) {
        return None;
    }

    // line comments right after `${` stay on the interpolation expression owner
    let target_owner = ctx.following_token_owner.or(ctx.following_owner)?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Handle tagged-template seam comments in head placement.
fn attach_expression_head_tagged_template_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment || !ctx.seam.token_after_is(TokenType::TemplateString) {
        return None;
    }

    // inline block comments between a tag expression and template literal stay on the tagged seam
    let target_owner = ctx
        .enclosing_owner
        .or(ctx.following_owner)
        .or(ctx.following_token_owner)
        .or(ctx.preceding_owner)
        .and_then(|owner| {
            promote_owner_to_tagged_template_expression_ancestor(ctx.tree, ctx.parents, owner)
        })?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockInfix))
}

/// Handle lambda arrow seam comments in head placement.
fn attach_expression_head_arrow_body_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let token_before_is_arrow = ctx.seam.token_before_is(TokenType::Arrow)
        || ctx.seam.token_before_is(TokenType::ArrowWide);
    if !token_before_is_arrow || !(ctx.comment_is_star || ctx.comment_is_line) {
        return None;
    }

    // comments after `=>` should stay with the lambda body expression
    let target_owner = ctx
        .following_owner
        .or(ctx.enclosing_owner)
        .or(ctx.following_token_owner)?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    let position = if ctx.has_leading_newline || ctx.has_trailing_newline {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };
    Some((Some(target_owner), position))
}

/// Resolve early expression head seam rules.
fn try_attach_comment_expression_head_rules(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_head_unary_not_chain_comment,
            attach_expression_head_unary_parenthesized_operand_comment,
            attach_expression_head_template_interpolation_comment,
            attach_expression_head_tagged_template_comment,
            attach_expression_head_arrow_body_comment,
        ],
    )
}

/// Run one ordered expression comment handler sequence and return the first attachment.
fn run_expression_comment_handlers(
    ctx: &ExpressionCommentContext<'_, '_>,
    handlers: &[fn(&ExpressionCommentContext<'_, '_>) -> Option<CommentAttachment>],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(ctx) {
            return Some(attachment);
        }
    }

    None
}

/// Handle assignment-operator seam line comments in middle placement.
fn attach_expression_middle_assignment_operator_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam,
        following_owner,
        token_after_span,
        following_token_owner,
        is_trailing_line_comment,
        ..
    } = *ctx;

    // end-of-line line comments after assignment operators belong to the rhs expression
    if is_trailing_line_comment
        && token_type_is_assignment_operator(seam.token_before_type)
        && let Some(target_node) = following_token_owner.or(following_owner).map(|owner| {
            token_after_span.map_or(owner, |token_after_span| {
                promote_owner_by_shared_start(tree, parents, owner, token_after_span.span.start)
            })
        })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    None
}

/// Handle closing-tag seam comments in middle placement.
fn attach_expression_middle_tree_closing_tag_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam_ctx,
        seam,
        preceding_owner,
        following_owner,
        preceding_token_owner,
        following_token_owner,
        enclosing_owner,
        comment_is_line,
        ..
    } = *ctx;

    // closing-tag seams normalize to tree-expression boundary ownership
    if seam_is_tree_closing_tag_head(seam_ctx, seam)
        && let Some(target_node) = tree_expression_owner_for_closing_tag_seam(
            tree,
            parents,
            preceding_owner,
            following_owner,
            preceding_token_owner,
            following_token_owner,
            enclosing_owner,
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if comment_is_line {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::BlockPostfix
        };
        return Some((Some(target_node), position));
    }

    None
}

/// Handle label-colon seam comments in middle placement.
fn attach_expression_middle_label_colon_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam,
        preceding_owner,
        following_owner,
        is_inline_star_comment,
        is_trailing_line_comment,
        ..
    } = *ctx;

    // line comments after label colons stay with the labelled statement owner
    if is_trailing_line_comment
        && seam.token_before_is(TokenType::Colon)
        && let Some(target_node) =
            labelled_statement_owner_for_colon_seam(tree, parents, preceding_owner, following_owner)
    {
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // inline block comments before label colons stay on the labelled statement owner
    if is_inline_star_comment
        && seam.token_after_is(TokenType::Colon)
        && let Some(target_node) =
            labelled_statement_owner_for_colon_seam(tree, parents, preceding_owner, following_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    None
}

/// Handle trailing-comma object-property seams in middle placement.
fn attach_expression_middle_trailing_comma_close_brace_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam,
        preceding_owner,
        token_before_span,
        comment_is_line,
        placement,
        ..
    } = *ctx;

    // line comments after object member trailing commas inside call arguments stay on the member
    if comment_is_line
        && placement != ExpressionCommentPlacement::OwnLine
        && seam.token_before_is(TokenType::Comma)
        && seam.token_after_is(TokenType::CloseBrace)
        && preceding_owner.is_some_and(|owner| tree.get_node_type(owner) == NodeType::Argument)
        && let Some(attachment) = attach_trailing_comma_close_brace_property_line_comment(
            tree,
            parents,
            token_before_span,
            preceding_owner,
        )
    {
        return Some(attachment);
    }

    None
}

/// Handle JSX child-container seam comments in middle placement.
fn attach_expression_middle_jsx_child_container_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        seam,
        preceding_owner,
        following_owner,
        is_trailing_line_comment,
        ..
    } = *ctx;

    // trailing line comments after JSX child expression containers should stay on the child
    if is_trailing_line_comment
        && seam.token_before_is(TokenType::CloseBrace)
        && seam.token_after_is(TokenType::LessThan)
        && let (Some(preceding_owner), Some(following_owner)) = (preceding_owner, following_owner)
        && tree.get_node_type(preceding_owner) == NodeType::Argument
        && tree.get_node_type(following_owner) == NodeType::Expression
        && matches!(
            tree.get(LocalNodeId::<Expression>::new(following_owner)),
            Expression::TreeExpression { .. }
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, preceding_owner);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Handle own-line comments before trailing separator commas in middle placement.
fn attach_expression_middle_own_line_before_comma_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam,
        preceding_owner,
        token_before_source_span,
        preceding_token_owner,
        comment_is_line,
        has_leading_newline,
        ..
    } = *ctx;

    // own-line line comments between a closing delimiter and a following comma stay trailing on the previous value
    if comment_is_line
        && has_leading_newline
        && seam.token_after_is(TokenType::Comma)
        && (seam.token_before_is(TokenType::CloseBrace)
            || seam.token_before_is(TokenType::CloseBracket)
            || seam.token_before_is(TokenType::CloseParenthesis))
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_source_span);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Handle inline member-semicolon seam comments in middle placement.
fn attach_expression_middle_member_semicolon_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam,
        preceding_owner,
        following_owner,
        preceding_token_owner,
        enclosing_owner,
        comment_is_star,
        has_leading_newline,
        ..
    } = *ctx;

    // same-line block comments before member semicolons stay on member boundaries
    // declaration semicolon seams are owned by declaration handlers
    if comment_is_star
        && !has_leading_newline
        && seam.token_after_is(TokenType::Semicolon)
        && !ctx.semicolon_after_control_head_empty_body
        && let Some(target_owner) = [
            preceding_token_owner,
            preceding_owner,
            following_owner,
            enclosing_owner,
        ]
        .into_iter()
        .flatten()
        .find_map(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
        })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Handle middle seam rules for labels and delimiter separators.
fn attach_expression_middle_label_and_separator_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_middle_assignment_operator_line_comment,
            attach_expression_middle_tree_closing_tag_comment,
            attach_expression_middle_label_colon_comment,
            attach_expression_middle_trailing_comma_close_brace_comment,
            attach_expression_middle_jsx_child_container_comment,
            attach_expression_middle_own_line_before_comma_comment,
            attach_expression_middle_member_semicolon_comment,
        ],
    )
}

/// Handle middle seam rules for parenthesized prefixes and semicolon guards.
fn attach_expression_middle_parenthesized_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_middle_semicolon_guard_parenthesized_comment,
            attach_expression_middle_open_parenthesis_own_line_comment,
        ],
    )
}

/// Handle middle seam comments right after semicolon guards before one parenthesized expression.
fn attach_expression_middle_semicolon_guard_parenthesized_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        seam,
        following_owner,
        following_token_owner,
        has_leading_newline,
        comment_is_star,
        ..
    } = *ctx;

    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);
    let token_before_is_semicolon = seam.token_before_is(TokenType::Semicolon);

    // same-line comments after no-semi guards should stay on the guarded expression
    if token_before_is_semicolon
        && token_after_is_open_parenthesis
        && comment_is_star
        && let Some(mut target_node) = following_token_owner.or(following_owner)
    {
        if tree.get_node_type(target_node) == NodeType::Expression {
            let expression_id = LocalNodeId::<Expression>::new(target_node);
            if !matches!(tree.get(expression_id), Expression::Parenthesized { .. }) {
                target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            }
        } else {
            target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        }

        let position = if has_leading_newline {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

    None
}

/// Return one normalized parenthesized own-line comment target owner.
fn normalize_parenthesized_open_owner_for_own_line_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    target_node: u32,
) -> u32 {
    if tree.get_node_type(target_node) != NodeType::Expression {
        return target_node;
    }

    let expression_id = LocalNodeId::<Expression>::new(target_node);
    let is_cast_like_type_binary = matches!(
        tree.get(expression_id),
        Expression::TypeBinary {
            operator: ast::TypeBinaryOperator::Cast | ast::TypeBinaryOperator::Satisfies,
            ..
        }
    );
    if !is_cast_like_type_binary {
        return target_node;
    }

    let Some(parent_id) = parents.get_by_id(target_node) else {
        return target_node;
    };
    if tree.get_node_type(parent_id) != NodeType::Expression {
        return target_node;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    let is_direct_parenthesized_wrapper = matches!(
        tree.get(parent_expression_id),
        Expression::Parenthesized { expression } if expression.id == target_node
    );
    if !is_direct_parenthesized_wrapper {
        return target_node;
    }

    let Some(grandparent_id) = parents.get_by_id(parent_id) else {
        return target_node;
    };
    if tree.get_node_type(grandparent_id) != NodeType::Expression {
        return target_node;
    }

    let grandparent_expression_id = LocalNodeId::<Expression>::new(grandparent_id);
    match tree.get(grandparent_expression_id) {
        Expression::If {
            kind: ast::IfKind::Ternary,
            ..
        } => parent_id,
        Expression::Binary { .. }
        | Expression::Member { .. }
        | Expression::PrivateMember { .. }
        | Expression::Index { .. }
        | Expression::Call { .. }
        | Expression::Must { .. }
        | Expression::Maybe { .. }
        | Expression::TaggedTemplateExpression { .. } => grandparent_id,
        _ => target_node,
    }
}

/// Handle middle seam own-line comments right after `(` tokens.
fn attach_expression_middle_open_parenthesis_own_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam,
        following_owner,
        token_after_span,
        following_token_owner,
        has_leading_newline,
        comment_is_line,
        ..
    } = *ctx;

    let token_before_is_open_parenthesis = seam.token_before_is(TokenType::OpenParenthesis);
    if !(has_leading_newline && comment_is_line && token_before_is_open_parenthesis) {
        return None;
    }

    // own-line comments after `(` should bind to the full expression that starts at the rhs token
    let target_node = token_after_span
        .and_then(|token_after_span| find_preferred_owner_starting_at(tree, token_after_span.span))
        .or(following_token_owner)
        .or(following_owner)
        .map(|owner| {
            token_after_span.map_or(owner, |token_after_span| {
                promote_owner_by_shared_start(tree, parents, owner, token_after_span.span.start)
            })
        });
    let target_node = target_node?;

    // type-cast owners inside parenthesized wrappers should bind to the consuming group
    let target_node =
        normalize_parenthesized_open_owner_for_own_line_comment(tree, parents, target_node);
    let target_node = normalize_formatter_trivia_target_owner(tree, target_node);

    Some((Some(target_node), AnnotationPosition::LinePrefix))
}

/// Handle middle seam rules around open-brace and mapped-type seams.
fn attach_expression_middle_open_brace_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_middle_open_brace_rhs_inline_comment,
            attach_expression_middle_open_brace_mapped_type_own_line_comment,
            attach_expression_middle_spread_parameter_inline_comment,
        ],
    )
}

/// Handle inline comments after `{` before grouped keys and expressions.
fn attach_expression_middle_open_brace_rhs_inline_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let token_after_is_open_bracket_or_open_parenthesis =
        ctx.seam.token_after_is(TokenType::OpenBracket)
            || ctx.seam.token_after_is(TokenType::OpenParenthesis);
    if !ctx.is_inline_star_comment
        || !ctx.seam.token_before_is(TokenType::OpenBrace)
        || !token_after_is_open_bracket_or_open_parenthesis
    {
        return None;
    }

    // inline comments after `{` before grouped keys and expressions stay on the rhs expression
    let target_owner = ctx
        .following_token_owner
        .or(ctx.following_owner)
        .map(|owner| {
            ctx.token_after_span.map_or(owner, |token| {
                promote_owner_by_shared_start(ctx.tree, ctx.parents, owner, token.span.start)
            })
        })?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Handle own-line mapped-type entry comments after `{`.
fn attach_expression_middle_open_brace_mapped_type_own_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.has_leading_newline
        || ctx.has_trailing_newline
        || !(ctx.comment_is_star || ctx.comment_is_line)
        || !ctx.seam.token_before_is(TokenType::OpenBrace)
        || !ctx.seam.token_after_is(TokenType::OpenBracket)
    {
        return None;
    }

    // own-line mapped-type entry comments after `{` should stay with the mapped member head
    let target_owner = [
        ctx.enclosing_owner,
        ctx.following_owner,
        ctx.following_token_owner,
        ctx.preceding_owner,
    ]
    .into_iter()
    .flatten()
    .find_map(|owner| {
        promote_owner_to_mapped_type_expression_ancestor(ctx.tree, ctx.parents, owner)
    })?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockPrefix))
}

/// Handle inline spread comments between `...` and binding names.
fn attach_expression_middle_spread_parameter_inline_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment || !ctx.seam.token_before_is(TokenType::Spread) {
        return None;
    }

    // comments between rest spread and binding names stay on the parameter owner
    let target_owner = ctx
        .enclosing_owner
        .or(ctx.preceding_owner)
        .or(ctx.following_owner)?;
    let target_owner = promote_owner_to_node_type_ancestor(
        ctx.tree,
        ctx.parents,
        target_owner,
        NodeType::Parameter,
    )
    .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Handle middle seam rules for declaration heads and index boundaries.
fn attach_expression_middle_declaration_and_index_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_middle_declaration_head_comment,
            attach_expression_middle_index_boundary_comment,
        ],
    )
}

/// Return whether seam rhs starts one JSX tree expression.
fn seam_rhs_starts_tree_expression(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    following_owner: Option<u32>,
    following_token_owner: Option<u32>,
) -> bool {
    [following_token_owner, following_owner]
        .into_iter()
        .flatten()
        .any(|owner| {
            let owner = promote_owner_to_tree_expression_parent(tree, parents, owner);
            tree.get_node_type(owner) == NodeType::Expression
                && matches!(
                    tree.get(LocalNodeId::<Expression>::new(owner)),
                    Expression::TreeExpression { .. }
                )
        })
}

/// Handle middle seam declaration generic-head comments before `<`.
fn attach_expression_middle_declaration_head_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam_ctx,
        seam,
        preceding_owner,
        following_owner,
        following_token_owner,
        is_trailing_line_comment,
        ..
    } = *ctx;

    let token_after_is_less_than = seam.token_after_is(TokenType::LessThan);
    let token_after_starts_tree_expression =
        seam_rhs_starts_tree_expression(tree, parents, following_owner, following_token_owner);
    if is_trailing_line_comment && token_after_is_less_than && !token_after_starts_tree_expression {
        let declaration_target = seam_ctx
            .token_after
            .and_then(|token_after_index| {
                find_owner_at_or_after_token(tree, seam_ctx.semantic_tokens, token_after_index)
            })
            .and_then(|owner| promote_owner_to_declaration_ancestor(tree, parents, owner))
            .or_else(|| {
                following_owner.and_then(|owner| {
                    promote_owner_to_declaration_ancestor(tree, parents, owner).or_else(|| {
                        (tree.get_node_type(owner) == NodeType::Declaration).then_some(owner)
                    })
                })
            })
            .or_else(|| {
                preceding_owner.and_then(|owner| {
                    promote_owner_to_declaration_ancestor(tree, parents, owner).or_else(|| {
                        (tree.get_node_type(owner) == NodeType::Declaration).then_some(owner)
                    })
                })
            });

        if let Some(target_node) = declaration_target {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    None
}

/// Handle middle seam comments before index boundaries.
fn attach_expression_middle_index_boundary_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam,
        preceding_owner,
        token_before_span,
        token_before_source_span,
        enclosing_owner,
        preceding_token_owner,
        has_leading_newline,
        comment_is_line,
        ..
    } = *ctx;

    let token_after_is_index_boundary = seam.token_after_is(TokenType::OpenBracket);
    let token_before_is_comma = seam.token_before_is(TokenType::Comma);
    let token_before_is_open_brace = seam.token_before_is(TokenType::OpenBrace);

    // own-line seam comments before index operators stay on the seam operation
    if has_leading_newline
        && token_after_is_index_boundary
        && ctx.seam_has_shared_expression_owner
        && !token_before_is_comma
        && !token_before_is_open_brace
    {
        if comment_is_line
            && let Some(target_node) = token_before_span
                .and_then(|token| find_preferred_owner_starting_at(tree, token.span))
                .or(preceding_token_owner)
                .or(preceding_owner)
        {
            let target_node = normalize_owner_with_shared_end(
                tree,
                parents,
                target_node,
                token_before_source_span,
            );
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        if let Some(target_node) = enclosing_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }
    }

    None
}

/// Resolve middle expression seam rules around delimiters, labels, and mapped/index seams.
fn try_attach_comment_expression_middle_rules(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_middle_label_and_separator_comments,
            attach_expression_middle_parenthesized_comments,
            attach_expression_middle_open_brace_comments,
            attach_expression_middle_declaration_and_index_comments,
        ],
    )
}

/// Return one ordered candidate-owner list for expression seam ownership checks.
fn expression_seam_candidate_owners(ctx: &ExpressionCommentContext<'_, '_>) -> [Option<u32>; 5] {
    [
        ctx.enclosing_owner,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.preceding_token_owner,
        ctx.following_token_owner,
    ]
}

/// Return one token type after the next non-trivia token beyond seam `token_after`.
fn token_after_close_parenthesis_following_type(
    seam_ctx: &CommentSeamContext<'_>,
) -> Option<TokenType> {
    seam_ctx
        .token_after
        .and_then(|index| next_non_trivia_token_index(seam_ctx.semantic_tokens, index))
        .and_then(|index| seam_ctx.semantic_tokens.get(index))
        .map(|token| token.token.ty)
}

/// Handle pre-placement semicolon-guard ownership.
fn attach_expression_pre_placement_semicolon_guard_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    // own-line semicolon-guard seams resolve in expression routing for semicolon-adjacent shapes
    if (ctx.seam.token_before_is(TokenType::Semicolon)
        || ctx.seam.token_after_is(TokenType::Semicolon))
        && !ctx.semicolon_after_control_head_empty_body
    {
        return attach_semicolon_guard_own_line_comment(
            ctx.tree,
            ctx.parents,
            ctx.seam_ctx,
            ctx.seam,
            ctx.owners,
        );
    }

    None
}

/// Handle pre-placement spread seam ownership.
fn attach_expression_pre_placement_spread_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    // spread seams keep ownership on the spread argument in expression contexts
    if (ctx.token_after_is_spread || ctx.token_before_is_spread)
        && let Some(target_node) = expression_seam_candidate_owners(ctx)
            .into_iter()
            .flatten()
            .find_map(|owner| {
                promote_owner_to_spread_argument_ancestor(ctx.tree, ctx.parents, owner)
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(ctx.tree, target_node);
        if ctx.token_after_is_spread {
            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }

        let position = if ctx.comment_is_line {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

    // inline comments inside empty object spread values stay inside the object literal
    if ctx.is_inline_star_comment
        && ctx.token_before_is_open_brace
        && ctx.token_after_is_close_brace
        && let Some(spread_argument_owner) = expression_seam_candidate_owners(ctx)
            .into_iter()
            .flatten()
            .find_map(|owner| {
                promote_owner_to_spread_argument_ancestor(ctx.tree, ctx.parents, owner)
            })
    {
        let spread_argument_id = LocalNodeId::<Argument>::new(spread_argument_owner);
        if let Argument::Spread {
            value: spread_value_id,
            ..
        } = ctx.tree.get(spread_argument_id)
            && matches!(
                ctx.tree.get(*spread_value_id),
                Expression::ObjectExpression { properties, .. } if properties.is_empty()
            )
        {
            let target_node = normalize_formatter_trivia_target_owner(ctx.tree, spread_value_id.id);
            return Some((Some(target_node), AnnotationPosition::BlockInfix));
        }
    }

    None
}

/// Handle pre-placement dependency attribute seam ownership.
fn attach_expression_pre_placement_dependency_attribute_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let token_after_is_dependency_attribute_keyword =
        ctx.seam.token_after_is_keyword(CommentSeamKeyword::With);
    if !token_after_is_dependency_attribute_keyword {
        return None;
    }

    let seam_candidates = [
        ctx.preceding_token_owner,
        ctx.preceding_owner,
        ctx.enclosing_owner,
        ctx.following_owner,
        ctx.following_token_owner,
    ];
    let target_node =
        dependency_attribute_expression_owner_from_seam_candidates(ctx.tree, seam_candidates)?;

    let target_node = normalize_formatter_trivia_target_owner(ctx.tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Handle pre-placement empty dependency item seam ownership.
fn attach_expression_pre_placement_empty_dependency_item_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment {
        return None;
    }

    if !ctx.seam.token_before_is(TokenType::OpenBrace)
        || !ctx.seam.token_after_is(TokenType::CloseBrace)
    {
        return None;
    }

    let seam_candidates = [
        ctx.preceding_token_owner,
        ctx.preceding_owner,
        ctx.enclosing_owner,
        ctx.following_owner,
        ctx.following_token_owner,
    ];
    let target_node = empty_dependency_expression_owner_from_seam_candidates(
        ctx.tree,
        ctx.parents,
        seam_candidates,
    )?;

    let target_node = normalize_formatter_trivia_target_owner(ctx.tree, target_node);
    Some((Some(target_node), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve pre-placement expression seam rules.
fn try_attach_comment_expression_pre_placement_rules(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_pre_placement_semicolon_guard_comments,
            attach_expression_pre_placement_empty_dependency_item_comments,
            attach_expression_pre_placement_dependency_attribute_comments,
            attach_expression_pre_placement_spread_comments,
        ],
    )
}

/// Handle tail seam rules around optional calls and optional operators.
fn attach_expression_tail_optional_call_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let ExpressionCommentContext {
        tree,
        parents,
        seam_ctx,
        seam,
        preceding_owner,
        following_owner,
        preceding_token_owner,
        following_token_owner,
        enclosing_owner,
        comment_is_line,
        comment_is_star,
        has_leading_newline,
        has_trailing_newline,
        is_inline_star_comment,
        ..
    } = *ctx;

    let token_after_is_maybe = seam.token_after_is(TokenType::Maybe);
    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);

    // inline optional-call operator seam comments stay on the call base segment
    // this follows prettier and oxfmt output for `call?./* comment */()`
    let seam_is_optional_call_parenthesis_separator =
        seam.token_before_is(TokenType::Dot) && seam.token_after_is(TokenType::OpenParenthesis);
    let seam_is_optional_call_operator_or_parenthesis_separator =
        seam_is_optional_call_operator(seam_ctx, seam)
            || seam_is_optional_call_parenthesis_separator;
    if (comment_is_star || comment_is_line)
        && (seam_is_optional_call_operator_or_parenthesis_separator || token_after_is_maybe)
        && let Some(target_node) = if comment_is_line {
            optional_call_seam_expression_owner(
                tree,
                parents,
                preceding_owner,
                preceding_token_owner,
                following_owner,
                following_token_owner,
                enclosing_owner,
            )
            .or_else(|| {
                optional_call_seam_base_owner(
                    tree,
                    parents,
                    preceding_owner,
                    preceding_token_owner,
                    following_owner,
                    following_token_owner,
                    enclosing_owner,
                )
            })
        } else {
            optional_call_seam_base_owner(
                tree,
                parents,
                preceding_owner,
                preceding_token_owner,
                following_owner,
                following_token_owner,
                enclosing_owner,
            )
        }
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position =
            optional_call_seam_position(comment_is_line, has_leading_newline, has_trailing_newline);
        return Some((Some(target_node), position));
    }

    // inline optional-call comments between the operator and `(` stay before `?.`
    if is_inline_star_comment
        && token_after_is_open_parenthesis
        && let Some(target_node) =
            enclosing_owner.and_then(|owner| optional_call_expression_maybe_owner(tree, owner))
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    None
}

/// Attach one call-callee fallback seam comment using canonical call-callee owner routing.
fn attach_call_callee_fallback_comment(
    tree: &NodeTree,
    call_like_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
    preceding_owner: Option<u32>,
    position: AnnotationPosition,
) -> Option<CommentAttachment> {
    let fallback_owner = call_like_owner
        .and_then(|owner| call_like_callee_owner(tree, owner))
        .or(preceding_token_owner.or(preceding_owner))?;
    let target_node = normalize_call_callee_seam_owner(tree, fallback_owner);
    Some((Some(target_node), position))
}

/// Attach line comments at call-like callee-to-argument boundaries.
fn attach_tail_call_like_open_parenthesis_line_comment(
    tree: &NodeTree,
    call_owner: u32,
    has_leading_newline: bool,
    enclosing_owner_first_dynamic_argument: Option<u32>,
) -> Option<CommentAttachment> {
    let call_owner_first_dynamic_argument =
        first_dynamic_argument_owner_for_call_like(tree, call_owner);

    // own-line comments before call arguments stay with the first argument when present
    if has_leading_newline {
        if let Some(target_node) = call_owner_first_dynamic_argument {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, call_owner);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // compact line comments stay on call boundaries when no dynamic argument exists
    let call_owner_has_static_arguments =
        call_like_expression_has_static_arguments(tree, call_owner);
    if call_owner_has_static_arguments || call_owner_first_dynamic_argument.is_none() {
        let target_node = normalize_formatter_trivia_target_owner(tree, call_owner);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // otherwise bind to the first dynamic argument seam
    if let Some(target_node) =
        call_owner_first_dynamic_argument.or(enclosing_owner_first_dynamic_argument)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    None
}

/// Return one normalized left-segment owner for tail seam attachment.
fn tail_seam_left_segment_owner(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_token_owner: Option<u32>,
    preceding_owner: Option<u32>,
    token_before_source_span: Option<Span>,
) -> Option<u32> {
    normalized_preceding_expression_owner_for_tail_seam(
        tree,
        parents,
        preceding_token_owner,
        preceding_owner,
        token_before_source_span,
    )
}

/// Attach non-own-line seam comments that stay with the left expression segment.
fn attach_tail_shared_expression_seam_comment(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    preceding_token_owner: Option<u32>,
    preceding_owner: Option<u32>,
    token_before_source_span: Option<Span>,
    has_trailing_newline: bool,
) -> Option<CommentAttachment> {
    let target_node = tail_seam_left_segment_owner(
        tree,
        parents,
        preceding_token_owner,
        preceding_owner,
        token_before_source_span,
    )?;

    if has_trailing_newline {
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    Some((Some(target_node), AnnotationPosition::LinePostfix))
}

/// Attach static type-argument call-boundary comments before call arguments.
fn attach_tail_static_type_argument_call_boundary_comment(
    tree: &NodeTree,
    call_owner: Option<u32>,
    preceding_token_owner: Option<u32>,
    preceding_owner: Option<u32>,
    has_leading_newline: bool,
    enclosing_owner_first_dynamic_argument: Option<u32>,
) -> Option<CommentAttachment> {
    if has_leading_newline && let Some(target_node) = enclosing_owner_first_dynamic_argument {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    attach_call_callee_fallback_comment(
        tree,
        call_owner,
        preceding_token_owner,
        preceding_owner,
        AnnotationPosition::LinePostfix,
    )
}

/// Attach inline block comments around call and new parenthesis seams.
fn attach_tail_inline_call_parenthesis_comment(
    tree: &NodeTree,
    enclosing_owner: Option<u32>,
    enclosing_owner_first_dynamic_argument: Option<u32>,
    token_before_is_open_parenthesis: bool,
    token_after_is_open_parenthesis: bool,
    preceding_token_owner: Option<u32>,
    preceding_owner: Option<u32>,
) -> Option<CommentAttachment> {
    // inline block comments right after `(` in call and new expressions stay on the first argument
    if token_before_is_open_parenthesis
        && let Some(target_node) = enclosing_owner_first_dynamic_argument
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // inline block comments between call callees and `(` stay on callee seams
    if token_after_is_open_parenthesis
        && !token_before_is_open_parenthesis
        && let Some(call_like_owner) =
            enclosing_owner.filter(|owner| is_call_like_expression_owner(tree, *owner))
    {
        if let Some(target_node) = first_dynamic_argument_owner_for_call_like(tree, call_like_owner)
        {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        return attach_call_callee_fallback_comment(
            tree,
            Some(call_like_owner),
            preceding_token_owner,
            preceding_owner,
            AnnotationPosition::LinePostfix,
        );
    }

    None
}

/// Tail seam facts for chain and call boundary routing.
#[derive(Debug, Clone, Copy)]
struct TailChainAndCallFacts {
    /// Candidate call-like owner for this seam.
    call_owner: Option<u32>,
    /// Seam is one call-like callee boundary before `(`.
    seam_is_call_like_open_parenthesis_boundary: bool,
    /// Token after seam is one member dot.
    token_after_is_dot: bool,
    /// Token after seam is one index open bracket.
    token_after_is_open_bracket: bool,
    /// Token after seam is one argument open parenthesis.
    token_after_is_open_parenthesis: bool,
    /// Token before seam is one comma.
    token_before_is_comma: bool,
    /// Token before seam is one static type argument closer.
    token_before_is_greater_than: bool,
    /// Token before seam is one open brace.
    token_before_is_open_brace: bool,
    /// Token before seam is one open parenthesis.
    token_before_is_open_parenthesis: bool,
}

/// Build tail seam facts for chain and call boundary routing.
fn tail_chain_and_call_facts(ctx: &ExpressionCommentContext<'_, '_>) -> TailChainAndCallFacts {
    let token_after_is_open_parenthesis = ctx.seam.token_after_is(TokenType::OpenParenthesis);
    let seam_candidates = expression_seam_candidate_owners(ctx);
    let call_owner = first_call_like_expression_owner_from_seam_candidates(
        ctx.tree,
        ctx.parents,
        seam_candidates,
    );
    let seam_is_call_like_open_parenthesis_boundary = token_after_is_open_parenthesis
        && call_owner.is_some_and(|call_owner| {
            seam_is_call_like_callee_boundary(
                ctx.tree,
                ctx.parents,
                call_owner,
                ctx.preceding_owner,
                ctx.preceding_token_owner,
            )
        });

    TailChainAndCallFacts {
        call_owner,
        seam_is_call_like_open_parenthesis_boundary,
        token_after_is_dot: ctx.seam.token_after_is(TokenType::Dot),
        token_after_is_open_bracket: ctx.seam.token_after_is(TokenType::OpenBracket),
        token_after_is_open_parenthesis,
        token_before_is_comma: ctx.seam.token_before_is(TokenType::Comma),
        token_before_is_greater_than: ctx.seam.token_before_is(TokenType::GreaterThan),
        token_before_is_open_brace: ctx.seam.token_before_is(TokenType::OpenBrace),
        token_before_is_open_parenthesis: ctx.seam.token_before_is(TokenType::OpenParenthesis),
    }
}

/// Attach call-like callee boundary line comments before argument parentheses.
fn attach_tail_call_like_open_parenthesis_boundary_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
    facts: TailChainAndCallFacts,
) -> Option<CommentAttachment> {
    if !ctx.comment_is_line || !facts.seam_is_call_like_open_parenthesis_boundary {
        return None;
    }

    let call_owner = facts.call_owner?;
    attach_tail_call_like_open_parenthesis_line_comment(
        ctx.tree,
        call_owner,
        ctx.has_leading_newline,
        ctx.enclosing_owner_first_dynamic_argument,
    )
}

/// Attach chain continuation comments that stay on the left segment.
fn attach_tail_chain_continuation_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
    facts: TailChainAndCallFacts,
) -> Option<CommentAttachment> {
    if ctx.has_leading_newline {
        return None;
    }

    let is_chain_continuation =
        facts.token_after_is_open_bracket || (facts.token_after_is_dot && ctx.comment_is_star);
    if !is_chain_continuation
        || !ctx.seam_has_shared_expression_owner
        || facts.token_before_is_comma
        || facts.token_before_is_open_brace
    {
        return None;
    }

    attach_tail_shared_expression_seam_comment(
        ctx.tree,
        ctx.parents,
        ctx.preceding_token_owner,
        ctx.preceding_owner,
        ctx.token_before_source_span,
        ctx.has_trailing_newline,
    )
}

/// Attach dot-boundary line comments to the left expression seam.
fn attach_tail_member_dot_line_boundary_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
    facts: TailChainAndCallFacts,
) -> Option<CommentAttachment> {
    if !ctx.comment_is_line
        || ctx.has_leading_newline
        || !facts.token_after_is_dot
        || facts.token_before_is_comma
        || facts.token_before_is_open_brace
    {
        return None;
    }

    let target_node = tail_seam_left_segment_owner(
        ctx.tree,
        ctx.parents,
        ctx.preceding_token_owner,
        ctx.preceding_owner,
        ctx.token_before_source_span,
    )?;
    Some((Some(target_node), AnnotationPosition::LinePostfixBoundary))
}

/// Attach static type-argument call-boundary line comments.
fn attach_tail_static_type_argument_call_boundary_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
    facts: TailChainAndCallFacts,
) -> Option<CommentAttachment> {
    let seam_is_call_like_static_argument_call_boundary = facts.token_before_is_greater_than
        && facts.token_after_is_open_parenthesis
        && facts.seam_is_call_like_open_parenthesis_boundary;
    if !ctx.comment_is_line || !seam_is_call_like_static_argument_call_boundary {
        return None;
    }

    attach_tail_static_type_argument_call_boundary_comment(
        ctx.tree,
        facts.call_owner,
        ctx.preceding_token_owner,
        ctx.preceding_owner,
        ctx.has_leading_newline,
        ctx.enclosing_owner_first_dynamic_argument,
    )
}

/// Attach inline call parenthesis block comments around call/new seams.
fn attach_tail_inline_call_parenthesis_block_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
    facts: TailChainAndCallFacts,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment {
        return None;
    }

    attach_tail_inline_call_parenthesis_comment(
        ctx.tree,
        ctx.enclosing_owner,
        ctx.enclosing_owner_first_dynamic_argument,
        facts.token_before_is_open_parenthesis,
        facts.token_after_is_open_parenthesis,
        ctx.preceding_token_owner,
        ctx.preceding_owner,
    )
}

/// Handle tail seam rules for chain seams and call-like boundaries.
fn attach_expression_tail_chain_and_call_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let facts = tail_chain_and_call_facts(ctx);
    for handler in [
        attach_tail_call_like_open_parenthesis_boundary_line_comment,
        attach_tail_chain_continuation_comment,
        attach_tail_member_dot_line_boundary_comment,
        attach_tail_static_type_argument_call_boundary_line_comment,
        attach_tail_inline_call_parenthesis_block_comment,
    ] {
        if let Some(attachment) = handler(ctx, facts) {
            return Some(attachment);
        }
    }

    None
}

/// Handle tail seam rules for object and logical-operator boundaries.
fn attach_expression_tail_object_and_logical_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_tail_object_open_brace_index_comment,
            attach_expression_tail_empty_object_infix_comment,
            attach_expression_tail_logical_operator_line_comment,
        ],
    )
}

/// Handle object open-brace seam comments before computed keys in tail placement.
fn attach_expression_tail_object_open_brace_index_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment
        || !ctx.seam.token_before_is(TokenType::OpenBrace)
        || !ctx.seam.token_after_is(TokenType::OpenBracket)
    {
        return None;
    }

    // comments between object open braces and computed keys stay inside the object
    if let Some(target_owner) = ctx.preceding_owner {
        let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
    }

    let target_owner = ctx.enclosing_owner?;
    if ctx.tree.get_node_type(target_owner) == NodeType::Expression {
        let expression_id = LocalNodeId::<Expression>::new(target_owner);
        if let Expression::ObjectExpression { properties, .. } = ctx.tree.get(expression_id)
            && let Some(first_property) = properties.first().copied()
        {
            return Some((Some(first_property.id), AnnotationPosition::LinePrefix));
        }
    }

    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Handle comments inside empty object literals in tail placement.
fn attach_expression_tail_empty_object_infix_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !(ctx.is_inline_star_comment || ctx.comment_is_line)
        || !ctx.seam.token_before_is(TokenType::OpenBrace)
        || !ctx.seam.token_after_is(TokenType::CloseBrace)
    {
        return None;
    }

    // comments inside empty object literals stay as object infix comments
    let target_owner = [
        ctx.token_before_span
            .and_then(|token| find_preferred_owner_starting_at(ctx.tree, token.span)),
        ctx.following_token_owner,
        ctx.following_owner,
        ctx.enclosing_owner,
    ]
    .into_iter()
    .flatten()
    .find_map(|owner| empty_object_expression_owner_for_candidate(ctx.tree, owner))?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::BlockInfix))
}

/// Handle logical-operator tail line comments.
fn attach_expression_tail_logical_operator_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let token_before_is_logical_operator = matches!(
        ctx.seam.token_before_type,
        Some(TokenType::LogicalAnd | TokenType::LogicalOr | TokenType::Coalesce)
    );
    if !ctx.is_trailing_line_comment || !token_before_is_logical_operator {
        return None;
    }

    // line comments after logical operators belong to the right operand
    let target_owner = ctx.following_owner?;
    let target_owner = ctx
        .token_after_source_span
        .map(|span| promote_owner_by_shared_start(ctx.tree, ctx.parents, target_owner, span.start))
        .unwrap_or(target_owner);
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Handle tail seam rules around ternary branches.
fn attach_expression_tail_ternary_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_tail_ternary_before_colon_inline_comment,
            attach_expression_tail_ternary_before_question_comment,
            attach_expression_tail_ternary_after_separator_comment,
        ],
    )
}

/// Handle inline block comments before ternary `:` separators.
fn attach_expression_tail_ternary_before_colon_inline_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_ternary_seam
        || ctx.seam.token_before_is_return_type_colon
        || !ctx.is_inline_star_comment
        || ctx.has_leading_newline
        || !ctx.seam.token_after_is(TokenType::Colon)
    {
        return None;
    }

    // inline block comments before `:` in ternaries follow oxfmt branch ownership
    let ternary_else_owner = ctx
        .ternary_enclosing_owner
        .filter(|owner| ctx.tree.get_node_type(*owner) == NodeType::Expression)
        .and_then(|owner| {
            let expression_id = LocalNodeId::<Expression>::new(owner);
            match ctx.tree.get(expression_id) {
                Expression::If {
                    kind: ast::IfKind::Ternary,
                    else_expression: Some(else_expression),
                    ..
                } => Some(else_expression.id),
                _ => None,
            }
        });

    if let Some(target_owner) = ternary_else_owner
        .filter(|owner| ctx.tree.get_node_type(*owner) == NodeType::Expression)
        .filter(|owner| {
            matches!(
                ctx.tree.get(LocalNodeId::<Expression>::new(*owner)),
                Expression::TreeExpression { .. }
            )
        })
    {
        let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    let target_owner = ctx.preceding_token_owner.or(ctx.preceding_owner)?;
    let target_owner = normalize_owner_with_shared_end(
        ctx.tree,
        ctx.parents,
        target_owner,
        ctx.token_before_source_span,
    );
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle comments before ternary `?` separators.
fn attach_expression_tail_ternary_before_question_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_ternary_seam
        || ctx.seam.token_before_is_return_type_colon
        || !(ctx.comment_is_star || ctx.comment_is_line)
        || ctx.has_leading_newline
        || !ctx.seam.token_after_is(TokenType::Maybe)
    {
        return None;
    }

    // inline comments before ternary question separators stay on the preceding branch
    let target_owner = ctx.preceding_token_owner.or(ctx.preceding_owner)?;
    let target_owner = normalize_owner_with_shared_end(
        ctx.tree,
        ctx.parents,
        target_owner,
        ctx.token_before_source_span,
    );
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    let position = if ctx.has_trailing_newline || ctx.comment_is_line {
        AnnotationPosition::LinePostfixBoundary
    } else {
        AnnotationPosition::LinePostfix
    };
    Some((Some(target_owner), position))
}

/// Handle comments after ternary `?` and `:` separators.
fn attach_expression_tail_ternary_after_separator_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let token_before_is_separator =
        ctx.seam.token_before_is(TokenType::Maybe) || ctx.seam.token_before_is(TokenType::Colon);
    if !ctx.is_ternary_seam
        || ctx.seam.token_before_is_return_type_colon
        || !(ctx.comment_is_star || (ctx.comment_is_line && ctx.has_leading_newline))
        || !token_before_is_separator
    {
        return None;
    }

    // comments after ternary separators stay with the following branch
    let target_owner = ctx.ternary_following_owner?;
    let position = if ctx.comment_is_line {
        AnnotationPosition::LinePrefix
    } else if ctx.has_leading_newline || ctx.has_trailing_newline {
        AnnotationPosition::BlockPrefix
    } else {
        AnnotationPosition::LinePrefix
    };
    Some((Some(target_owner), position))
}

/// Handle tail seam rules for expression boundaries and trailing grouped seams.
fn attach_expression_tail_expression_boundary_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_tail_if_consequent_boundary_line_comment,
            attach_expression_tail_ternary_close_brace_inline_comment,
            attach_expression_tail_tree_container_close_brace_comment,
            attach_expression_tail_nested_close_parenthesis_line_comment,
            attach_expression_tail_grouped_close_parenthesis_line_comment,
        ],
    )
}

/// Handle trailing-line comments after non-block `if` consequents.
fn attach_expression_tail_if_consequent_boundary_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_trailing_line_comment
        || ctx.seam.token_before_is(TokenType::CloseParenthesis)
        || ctx.seam.token_after_is(TokenType::CloseParenthesis)
        || ctx.is_ternary_seam
    {
        return None;
    }

    // trailing line comments after non-block if consequents stay on the consequent statement
    let enclosing_owner = ctx.enclosing_owner?;
    let then_owner = if_expression_then_owner_without_else(ctx.tree, enclosing_owner)?;
    Some((Some(then_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle inline comments after ternary alternate close-brace boundaries.
fn attach_expression_tail_ternary_close_brace_inline_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if ctx.has_leading_newline
        || !ctx.comment_is_star
        || !ctx.seam.token_after_is(TokenType::CloseBrace)
    {
        return None;
    }

    // inline comments after ternary alternate branches stay on ternary boundary seams
    let target_owner = find_owner_in_candidate_ancestry(
        ctx.parents,
        [
            ctx.preceding_token_owner,
            ctx.preceding_owner,
            ctx.enclosing_owner,
            None,
            None,
        ],
        |owner_id| is_ternary_if_expression_owner(ctx.tree, owner_id).then_some(owner_id),
    )?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    let position = if ctx.has_trailing_newline {
        AnnotationPosition::LinePostfixBoundary
    } else {
        AnnotationPosition::LinePostfix
    };
    Some((Some(target_owner), position))
}

/// Handle comments before tree-expression container close braces.
fn attach_expression_tail_tree_container_close_brace_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if ctx.has_leading_newline
        || !(ctx.comment_is_line || ctx.comment_is_star)
        || !ctx.seam.token_after_is(TokenType::CloseBrace)
    {
        return None;
    }

    // comments before tree-expression container `}` stay on the container expression
    let preceding_expression_owner = ctx
        .preceding_owner
        .filter(|owner| ctx.tree.get_node_type(*owner) == NodeType::Expression)
        .filter(|owner| {
            ctx.parents
                .get_by_id(*owner)
                .is_some_and(|parent_id| ctx.tree.get_node_type(parent_id) == NodeType::Argument)
        })?;
    let position = if ctx.comment_is_line || ctx.has_trailing_newline {
        AnnotationPosition::LinePostfixBoundary
    } else {
        AnnotationPosition::LinePostfix
    };
    Some((Some(preceding_expression_owner), position))
}

/// Handle trailing-line comments before nested close-parenthesis seams.
fn attach_expression_tail_nested_close_parenthesis_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_trailing_line_comment
        || !ctx.seam.token_before_is(TokenType::CloseParenthesis)
        || !ctx.seam.token_after_is(TokenType::CloseParenthesis)
    {
        return None;
    }

    // trailing line comments before nested `)` tokens stay on the inner grouped expression
    let target_owner = ctx.preceding_token_owner.or(ctx.preceding_owner)?;
    let target_owner = if ctx.tree.get_node_type(target_owner) == NodeType::Expression {
        target_owner
    } else {
        promote_owner_to_node_type_ancestor(
            ctx.tree,
            ctx.parents,
            target_owner,
            NodeType::Expression,
        )
        .unwrap_or(target_owner)
    };
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle leading-newline line comments before grouped close-parenthesis seams.
fn attach_expression_tail_grouped_close_parenthesis_line_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.comment_is_line
        || !ctx.has_leading_newline
        || !ctx.seam.token_after_is(TokenType::CloseParenthesis)
        || !ctx.token_after_close_parenthesis_is_logical_operator
        || ctx.seam.token_before_is(TokenType::CloseParenthesis)
    {
        return None;
    }

    // trailing line comments before one closing `)` stay on the grouped-expression boundary owner
    let target_owner = ctx.preceding_token_owner.or(ctx.preceding_owner)?;
    let target_owner = normalize_owner_with_shared_end(
        ctx.tree,
        ctx.parents,
        target_owner,
        ctx.token_before_source_span,
    );
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle tail seam rules for grouped close-parenthesis ownership.
fn attach_expression_tail_grouping_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_tail_grouping_nested_close_paren_comment,
            attach_expression_tail_grouping_do_while_close_paren_comment,
            attach_expression_tail_grouping_ternary_close_paren_comment,
            attach_expression_tail_grouping_binary_rhs_close_paren_comment,
            attach_expression_tail_grouping_unary_close_paren_comment,
        ],
    )
}

/// Handle nested `)` inline comments that stay on outer grouped wrappers.
fn attach_expression_tail_grouping_nested_close_paren_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment
        || !ctx.seam.token_before_is(TokenType::CloseParenthesis)
        || !ctx.seam.token_after_is(TokenType::CloseParenthesis)
    {
        return None;
    }

    let seam_is_direct_lambda_body = ctx
        .preceding_token_owner
        .or(ctx.preceding_owner)
        .is_some_and(|owner| is_direct_lambda_body_expression_owner(ctx.tree, ctx.parents, owner));
    if !seam_is_direct_lambda_body {
        return None;
    }

    // nested `)` seams keep inline block comments on the outer grouped wrapper
    let target_owner = ctx
        .following_token_owner
        .or(ctx.following_owner)
        .and_then(|owner| {
            promote_owner_to_parenthesized_expression_ancestor(ctx.tree, ctx.parents, owner)
        })?;
    if !matches!(
        ctx.tree.get(LocalNodeId::<Expression>::new(target_owner)),
        Expression::Parenthesized { .. }
    ) {
        return None;
    }

    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle do-while close-paren inline comments in grouped seams.
fn attach_expression_tail_grouping_do_while_close_paren_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment || !ctx.seam.token_after_is(TokenType::CloseParenthesis) {
        return None;
    }

    let seam_has_do_while_ancestor = [
        ctx.preceding_token_owner,
        ctx.preceding_owner,
        ctx.following_owner,
        ctx.enclosing_owner,
        ctx.following_token_owner,
    ]
    .into_iter()
    .flatten()
    .any(|owner| owner_has_do_while_expression_ancestor(ctx.tree, ctx.parents, owner));
    if !seam_has_do_while_ancestor {
        return None;
    }

    // inline comments before `)` in do-while conditions should stay on the condition expression
    let target_owner = ctx.preceding_token_owner.or(ctx.preceding_owner)?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle ternary close-paren inline comments in grouped seams.
fn attach_expression_tail_grouping_ternary_close_paren_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment
        || !ctx.seam.token_after_is(TokenType::CloseParenthesis)
        || !ctx.is_ternary_seam
    {
        return None;
    }

    // inline comments before `)` in ternary branches stay on the inner branch expression
    let target_owner = ctx.preceding_token_owner.or(ctx.preceding_owner)?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle binary rhs close-paren inline comments in grouped seams.
fn attach_expression_tail_grouping_binary_rhs_close_paren_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment
        || !ctx.seam.token_after_is(TokenType::CloseParenthesis)
        || ctx.seam.token_before_is(TokenType::CloseParenthesis)
    {
        return None;
    }

    // inline comments before one close paren on parenthesized rhs binary operands
    // stay with the containing binary expression boundary
    let target_owner = ctx
        .preceding_token_owner
        .or(ctx.preceding_owner)
        .and_then(|owner| {
            binary_owner_with_parenthesized_right_operand(ctx.tree, ctx.parents, owner)
        })?;
    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary))
}

/// Handle unary grouped close-paren inline comments.
fn attach_expression_tail_grouping_unary_close_paren_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment
        || !ctx.seam.token_after_is(TokenType::CloseParenthesis)
        || ctx.seam.token_before_is(TokenType::CloseParenthesis)
    {
        return None;
    }

    let seam_has_expression_preceding_owner = ctx
        .preceding_token_owner
        .or(ctx.preceding_owner)
        .is_some_and(|owner| ctx.tree.get_node_type(owner) == NodeType::Expression);
    if !seam_has_expression_preceding_owner {
        return None;
    }

    // inline comments before one closing `)` stay on the inner operand
    let target_owner = normalized_preceding_expression_owner_for_tail_seam(
        ctx.tree,
        ctx.parents,
        ctx.preceding_token_owner,
        ctx.preceding_owner,
        ctx.token_before_source_span,
    )?;
    if !owner_has_unary_expression_ancestor(ctx.tree, ctx.parents, target_owner) {
        return None;
    }

    Some((Some(target_owner), AnnotationPosition::LinePostfix))
}

/// Handle tail seam rules for cast and parenthesized cast targets.
fn attach_expression_tail_cast_comments(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_tail_cast_open_brace_comment,
            attach_expression_tail_cast_open_parenthesis_comment,
        ],
    )
}

/// Handle inline comments before closure-cast object literals in tail placement.
fn attach_expression_tail_cast_open_brace_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment
        || !ctx.seam.token_after_is(TokenType::OpenBrace)
        || !ctx.seam.token_before_is(TokenType::OpenParenthesis)
    {
        return None;
    }

    // comments before closure-cast object literals stay with the rhs cast target
    let mut target_owner = ctx
        .following_token_owner
        .or(ctx.following_owner)
        .or(ctx.enclosing_owner)?;
    if ctx.tree.get_node_type(target_owner) != NodeType::Expression
        && let Some(expression_target) = promote_owner_to_node_type_ancestor(
            ctx.tree,
            ctx.parents,
            target_owner,
            NodeType::Expression,
        )
    {
        target_owner = expression_target;
    }

    if ctx.tree.get_node_type(target_owner) == NodeType::Expression
        && let Expression::Parenthesized { expression } =
            ctx.tree.get(LocalNodeId::<Expression>::new(target_owner))
    {
        target_owner = expression.id;
    }

    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Handle inline comments before parenthesized cast targets in tail placement.
fn attach_expression_tail_cast_open_parenthesis_comment(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    if !ctx.is_inline_star_comment || !ctx.seam.token_after_is(TokenType::OpenParenthesis) {
        return None;
    }

    if ctx.seam.token_before_is(TokenType::Maybe) && ctx.ternary_enclosing_owner.is_none() {
        return None;
    }

    if ctx.seam_is_new_signature_declaration {
        return None;
    }

    // comments before parenthesized cast targets stay with the grouped expression
    let target_owner = (if ctx.seam.token_before_is(TokenType::OpenParenthesis) {
        ctx.following_token_owner.or(ctx.following_owner)
    } else {
        ctx.following_token_owner
            .or(ctx.following_owner)
            .or(ctx.enclosing_owner)
    })
    .map(|owner| {
        ctx.token_after_span.map_or(owner, |token| {
            promote_owner_by_shared_start(ctx.tree, ctx.parents, owner, token.span.start)
        })
    })
    .and_then(|owner| {
        promote_owner_to_parenthesized_expression_ancestor(ctx.tree, ctx.parents, owner)
            .or(Some(owner))
    })?;

    let target_owner = normalize_formatter_trivia_target_owner(ctx.tree, target_owner);
    let target_owner = if ctx
        .preceding_owner
        .is_some_and(|owner| ctx.tree.get_node_type(owner) == NodeType::Declaration)
        && ctx.tree.get_node_type(target_owner) == NodeType::Expression
        && let Expression::Parenthesized { expression } =
            ctx.tree.get(LocalNodeId::<Expression>::new(target_owner))
    {
        expression.id
    } else {
        target_owner
    };
    Some((Some(target_owner), AnnotationPosition::LinePrefix))
}

/// Resolve tail expression seam rules around optional chains, ternaries, and grouped expressions.
fn try_attach_comment_expression_tail_rules(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        ctx,
        &[
            attach_expression_tail_optional_call_comments,
            attach_expression_tail_chain_and_call_comments,
            attach_expression_tail_object_and_logical_comments,
            attach_expression_tail_ternary_comments,
            attach_expression_tail_expression_boundary_comments,
            attach_expression_tail_grouping_comments,
            attach_expression_tail_cast_comments,
        ],
    )
}

/// Attach one expression seam comment by placement.
fn attach_expression_comment_by_placement(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    // current placement classes share the same middle and tail routing
    if let Some(attachment) = try_attach_comment_expression_middle_rules(ctx) {
        return Some(attachment);
    }

    try_attach_comment_expression_tail_rules(ctx)
}

/// Attach one expression seam comment fallback.
fn attach_expression_comment_fallback(
    ctx: &ExpressionCommentContext<'_, '_>,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
) -> Option<CommentAttachment> {
    try_attach_comment_expression_operator(
        ctx.tree,
        ctx.parents,
        ctx.seam_ctx,
        ctx.seam,
        enclosing_owner_cache,
        ctx.owners,
    )
}

/// Return whether one expression seam should route to statement separator ownership.
fn should_skip_expression_comment_for_call_like_separator(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> bool {
    ctx.should_route_call_like_separator_to_statement()
}

/// Return whether one expression seam should be delegated to decorator routing.
fn should_skip_expression_comment_for_decorator(ctx: &ExpressionCommentContext<'_, '_>) -> bool {
    ctx.token_after_is_at
}

/// Return whether one expression seam is one own-line member-dot seam.
fn should_skip_expression_comment_for_own_line_member_dot(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> bool {
    ctx.has_leading_newline
        && (ctx.comment_is_line || ctx.comment_is_star)
        && ctx.seam.token_after_is(TokenType::Dot)
}

/// Return whether one expression seam is one file-head end-of-line comment.
fn should_skip_expression_comment_for_file_head_end_of_line(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> bool {
    ctx.comment_is_line && ctx.seam_ctx.token_before.is_none() && ctx.has_trailing_newline
}

/// Return whether one expression seam is one inline comment before one statement semicolon.
fn should_skip_expression_comment_for_inline_before_semicolon(
    ctx: &ExpressionCommentContext<'_, '_>,
) -> bool {
    ctx.comment_is_line
        && !ctx.has_leading_newline
        && ctx.seam.token_after_is(TokenType::Semicolon)
        && !ctx.seam.token_before_is(TokenType::Semicolon)
}

/// Return whether one expression seam should skip expression-routing attachment.
fn should_skip_expression_comment_attachment(ctx: &ExpressionCommentContext<'_, '_>) -> bool {
    should_skip_expression_comment_for_call_like_separator(ctx)
        || should_skip_expression_comment_for_decorator(ctx)
        || should_skip_expression_comment_for_own_line_member_dot(ctx)
        || should_skip_expression_comment_for_file_head_end_of_line(ctx)
        || should_skip_expression_comment_for_inline_before_semicolon(ctx)
}

/// Resolve expression and type seam comment rules.
pub(crate) fn try_attach_comment_expression(
    tree: &NodeTree,
    _owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    ctx: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let expression_ctx =
        build_expression_comment_ctx(tree, parents, ctx, seam, enclosing_owner_cache, owners);

    // statement and dedicated seam owners run before expression routing
    if should_skip_expression_comment_attachment(&expression_ctx) {
        return None;
    }

    if let Some(attachment) = try_attach_comment_expression_head_rules(&expression_ctx) {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_comment_expression_pre_placement_rules(&expression_ctx) {
        return Some(attachment);
    }

    if let Some(attachment) = attach_expression_comment_by_placement(&expression_ctx) {
        return Some(attachment);
    }

    if let Some(attachment) =
        attach_expression_comment_fallback(&expression_ctx, enclosing_owner_cache)
    {
        return Some(attachment);
    }

    None
}
