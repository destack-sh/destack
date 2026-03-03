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
    find_preferred_owner_starting_at, find_smallest_owner_enclosing_token,
    lowest_common_owner_ancestor, normalize_formatter_trivia_target_owner,
    normalize_owner_with_shared_end, promote_owner_by_shared_start,
    promote_owner_to_declaration_ancestor, promote_owner_to_node_type_ancestor,
    promote_owner_to_parenthesized_expression_ancestor,
};
use super::semicolon::attach_semicolon_guard_own_line_comment;

/// Return the first dynamic argument owner for one call-like expression.
fn first_dynamic_argument_owner_for_call_like(tree: &NodeTree, owner_id: u32) -> Option<u32> {
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
    seam_context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    let token_before_predecessor_type = seam_context
        .token_before
        .and_then(|token_before_index| {
            previous_non_trivia_token_index(seam_context.semantic_tokens, token_before_index)
        })
        .and_then(|token_index| seam_context.semantic_tokens.get(token_index))
        .map(|token| token.token.ty);
    let token_after_successor_type = seam_context
        .token_after
        .and_then(|token_after_index| {
            next_non_trivia_token_index(seam_context.semantic_tokens, token_after_index)
        })
        .and_then(|token_index| seam_context.semantic_tokens.get(token_index))
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

/// Promote one owner to the nearest call or new expression ancestor.
fn promote_owner_to_call_or_new_expression_ancestor(
    tree: &NodeTree,
    parents: &NodeParentIndex,
    owner_id: u32,
) -> Option<u32> {
    find_owner_in_ancestor_chain(parents, owner_id, |candidate_id| {
        is_call_or_new_expression_owner(tree, candidate_id).then_some(candidate_id)
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
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    if seam.token_before_is(TokenType::Maybe) {
        return true;
    }

    if !seam.token_before_is(TokenType::Dot) {
        return false;
    }

    let Some(token_before_index) = context.token_before else {
        return false;
    };
    token_before_index > 0
        && context.semantic_tokens[token_before_index - 1].token.ty == TokenType::Maybe
}

/// Return whether one seam is one optional-call operator seam around `?.`.
fn seam_is_optional_call_operator(
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
) -> bool {
    let seam_is_between_operator_and_parenthesis = seam.token_after_is(TokenType::OpenParenthesis)
        && token_before_is_optional_call_operator_terminal(context, seam);
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
        .find_map(|owner| promote_owner_to_call_or_new_expression_ancestor(tree, parents, owner))
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

/// Prepared context for one expression seam comment attachment decision.
#[derive(Clone, Copy)]
struct ExpressionCommentContext<'a, 'ctx> {
    /// The syntax tree.
    tree: &'a NodeTree,
    /// Parent links for owner promotion.
    parents: &'a NodeParentIndex,
    /// Seam context.
    seam_context: &'a CommentSeamContext<'ctx>,
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
            token_after_close_parenthesis_following_type(self.seam_context);
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

/// Build one expression seam comment context.
fn build_expression_comment_context<'a, 'ctx>(
    tree: &'a NodeTree,
    parents: &'a NodeParentIndex,
    seam_context: &'a CommentSeamContext<'ctx>,
    seam: &'a CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> ExpressionCommentContext<'a, 'ctx> {
    let preceding_owner = owners.preceding;
    let following_owner = owners.following;
    let token_before_span = seam_context.token_before_span;
    let token_after_span = seam_context.token_after_span;
    let token_before_source_span = token_before_span.map(|token| token.span);
    let token_after_source_span = token_after_span.map(|token| token.span);
    let enclosing_owner = comment_enclosing_owner(seam_context, enclosing_owner_cache);
    let preceding_token_owner =
        token_before_span.and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));
    let following_token_owner =
        token_after_span.and_then(|token| find_smallest_owner_enclosing_token(tree, token.span));

    let has_leading_newline = seam.has_leading_newline;
    let has_trailing_newline = seam.has_trailing_newline;
    let comment_is_line = seam.comment_is_line;
    let comment_is_star = seam.comment_is_star;
    let placement = expression_comment_placement(has_leading_newline, has_trailing_newline);
    let is_inline_star_comment =
        placement == ExpressionCommentPlacement::Remaining && comment_is_star;
    let is_trailing_line_comment =
        placement == ExpressionCommentPlacement::EndOfLine && comment_is_line;

    let token_after_is_spread = seam.token_after_is(TokenType::Spread);
    let token_after_is_close_brace = seam.token_after_is(TokenType::CloseBrace);
    let token_after_is_at = seam.token_after_is(TokenType::At);
    let token_before_is_open_brace = seam.token_before_is(TokenType::OpenBrace);
    let token_before_is_spread = seam.token_before_is(TokenType::Spread);
    let token_after_close_parenthesis_following_type =
        token_after_close_parenthesis_following_type(seam_context);
    let token_after_close_parenthesis_is_logical_operator = matches!(
        token_after_close_parenthesis_following_type,
        Some(TokenType::LogicalAnd | TokenType::LogicalOr | TokenType::Coalesce)
    );

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
    let shared_ternary_owner =
        shared_owner.filter(|owner| is_ternary_if_expression_owner(tree, *owner));
    let ternary_enclosing_owner = shared_ternary_owner
        .or(enclosing_owner.filter(|owner| is_ternary_if_expression_owner(tree, *owner)));
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
    let enclosing_owner_first_dynamic_argument =
        enclosing_owner.and_then(|owner| first_dynamic_argument_owner_for_call_like(tree, owner));

    let semicolon_after_control_head_empty_body = seam.token_before_is(TokenType::CloseParenthesis)
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
        });

    ExpressionCommentContext {
        tree,
        parents,
        seam_context,
        seam,
        owners,
        preceding_owner,
        following_owner,
        token_before_span,
        token_after_span,
        token_before_source_span,
        token_after_source_span,
        preceding_token_owner,
        following_token_owner,
        enclosing_owner,
        has_leading_newline,
        has_trailing_newline,
        comment_is_line,
        comment_is_star,
        placement,
        is_inline_star_comment,
        is_trailing_line_comment,
        token_after_is_spread,
        token_after_is_close_brace,
        token_after_is_at,
        token_before_is_open_brace,
        token_before_is_spread,
        token_after_close_parenthesis_is_logical_operator,
        seam_is_new_signature_declaration,
        seam_has_shared_expression_owner,
        ternary_enclosing_owner,
        is_ternary_seam,
        ternary_following_owner,
        enclosing_owner_first_dynamic_argument,
        semicolon_after_control_head_empty_body,
    }
}

/// Resolve early expression head seam rules.
fn try_attach_comment_expression_head_rules(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam_context = comment_context.seam_context;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let following_owner = comment_context.following_owner;
    let token_after_span = comment_context.token_after_span;
    let token_after_source_span = comment_context.token_after_source_span;
    let enclosing_owner = comment_context.enclosing_owner;
    let preceding_token_owner = comment_context.preceding_token_owner;
    let following_token_owner = comment_context.following_token_owner;
    let has_leading_newline = comment_context.has_leading_newline;
    let has_trailing_newline = comment_context.has_trailing_newline;
    let comment_is_line = comment_context.comment_is_line;
    let comment_is_star = comment_context.comment_is_star;
    let is_inline_star_comment = comment_context.is_inline_star_comment;
    let is_trailing_line_comment = comment_context.is_trailing_line_comment;

    let token_before_is_not = seam.token_before_is(TokenType::Not);
    let token_before_is_arrow =
        seam.token_before_is(TokenType::Arrow) || seam.token_before_is(TokenType::ArrowWide);
    let token_after_is_not = seam.token_after_is(TokenType::Not);
    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);

    // trailing line comments between chained unary `!` heads stay on the following unary operand
    if is_trailing_line_comment
        && token_before_is_not
        && token_after_is_not
        && let Some(target_node) = following_token_owner.or(following_owner)
    {
        let target_node = token_after_source_span.map_or(target_node, |span| {
            promote_owner_by_shared_start(tree, parents, target_node, span.start)
        });
        let target_node = if tree.get_node_type(target_node) == NodeType::Expression {
            target_node
        } else {
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Expression)
                .unwrap_or(target_node)
        };
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // trailing line comments after unary heads before one parenthesized operand stay on the operand prefix
    if is_trailing_line_comment
        && token_after_is_open_parenthesis
        && [preceding_token_owner, preceding_owner]
            .into_iter()
            .flatten()
            .any(|owner| owner_has_unary_expression_ancestor(tree, parents, owner))
        && let Some(target_node) = token_after_span
            .and_then(|token_after_span| {
                find_preferred_owner_starting_at(tree, token_after_span.span)
            })
            .or(following_token_owner)
            .or(following_owner)
    {
        let target_node = token_after_source_span.map_or(target_node, |span| {
            promote_owner_by_shared_start(tree, parents, target_node, span.start)
        });
        let target_node = if tree.get_node_type(target_node) == NodeType::Expression {
            target_node
        } else {
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Expression)
                .unwrap_or(target_node)
        };
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // line comments right after `${` stay on the interpolation expression owner
    if comment_is_line
        && seam_is_template_interpolation_open_brace(seam_context, seam)
        && let Some(target_node) = following_token_owner.or(following_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // inline block comments between a tag expression and template literal stay on the tagged seam
    if is_inline_star_comment
        && seam.token_after_is(TokenType::TemplateString)
        && let Some(target_node) = enclosing_owner
            .or(following_owner)
            .or(following_token_owner)
            .or(preceding_owner)
            .and_then(|owner| {
                promote_owner_to_tagged_template_expression_ancestor(tree, parents, owner)
            })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockInfix));
    }

    // comments after `=>` should stay with the lambda body expression
    if token_before_is_arrow
        && (comment_is_star || comment_is_line)
        && let Some(target_node) = following_owner
            .or(enclosing_owner)
            .or(following_token_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if has_leading_newline || has_trailing_newline {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

    None
}

/// Run one ordered expression comment handler sequence and return the first attachment.
fn run_expression_comment_handlers(
    comment_context: &ExpressionCommentContext<'_, '_>,
    handlers: &[fn(&ExpressionCommentContext<'_, '_>) -> Option<CommentAttachment>],
) -> Option<CommentAttachment> {
    for handler in handlers {
        if let Some(attachment) = handler(comment_context) {
            return Some(attachment);
        }
    }

    None
}

/// Handle middle seam rules for labels and delimiter separators.
fn attach_expression_middle_label_and_separator_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam_context = comment_context.seam_context;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let following_owner = comment_context.following_owner;
    let token_before_span = comment_context.token_before_span;
    let token_after_span = comment_context.token_after_span;
    let token_before_source_span = comment_context.token_before_source_span;
    let preceding_token_owner = comment_context.preceding_token_owner;
    let following_token_owner = comment_context.following_token_owner;
    let enclosing_owner = comment_context.enclosing_owner;
    let is_inline_star_comment = comment_context.is_inline_star_comment;
    let is_trailing_line_comment = comment_context.is_trailing_line_comment;
    let comment_is_line = comment_context.comment_is_line;
    let placement = comment_context.placement;

    let token_after_is_colon = seam.token_after_is(TokenType::Colon);
    let token_after_is_close_brace = seam.token_after_is(TokenType::CloseBrace);
    let token_after_is_comma = seam.token_after_is(TokenType::Comma);
    let token_after_is_less_than = seam.token_after_is(TokenType::LessThan);
    let token_after_is_semicolon = seam.token_after_is(TokenType::Semicolon);
    let token_before_is_colon = seam.token_before_is(TokenType::Colon);
    let token_before_is_comma = seam.token_before_is(TokenType::Comma);
    let token_before_is_close_brace = seam.token_before_is(TokenType::CloseBrace);
    let token_before_is_close_bracket = seam.token_before_is(TokenType::CloseBracket);
    let token_before_is_close_parenthesis = seam.token_before_is(TokenType::CloseParenthesis);
    let token_before_is_assignment_operator =
        token_type_is_assignment_operator(seam.token_before_type);

    // end-of-line line comments after assignment operators belong to the rhs expression
    if is_trailing_line_comment
        && token_before_is_assignment_operator
        && let Some(target_node) = following_token_owner.or(following_owner).map(|owner| {
            token_after_span.map_or(owner, |token_after_span| {
                promote_owner_by_shared_start(tree, parents, owner, token_after_span.span.start)
            })
        })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // closing-tag seams normalize to tree-expression boundary ownership
    if seam_is_tree_closing_tag_head(seam_context, seam)
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
    // line comments after label colons stay with the labelled statement owner
    if is_trailing_line_comment
        && token_before_is_colon
        && let Some(target_node) =
            labelled_statement_owner_for_colon_seam(tree, parents, preceding_owner, following_owner)
    {
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // inline block comments before label colons stay on the labelled statement owner
    if is_inline_star_comment
        && token_after_is_colon
        && let Some(target_node) =
            labelled_statement_owner_for_colon_seam(tree, parents, preceding_owner, following_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // line comments after object member trailing commas inside call arguments stay on the member
    if comment_is_line
        && placement != ExpressionCommentPlacement::OwnLine
        && token_before_is_comma
        && token_after_is_close_brace
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

    // trailing line comments after JSX child expression containers should stay on the child
    if is_trailing_line_comment
        && token_before_is_close_brace
        && token_after_is_less_than
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

    // own-line line comments between a closing delimiter and a following comma stay trailing on the previous value
    if comment_is_line
        && comment_context.has_leading_newline
        && token_after_is_comma
        && (token_before_is_close_brace
            || token_before_is_close_bracket
            || token_before_is_close_parenthesis)
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_source_span);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    let semicolon_comment_seam_owners = [
        preceding_token_owner,
        preceding_owner,
        following_owner,
        enclosing_owner,
    ];
    let member_owner_for_semicolon_comment = semicolon_comment_seam_owners
        .into_iter()
        .flatten()
        .find_map(|owner| {
            promote_owner_to_node_type_ancestor(tree, parents, owner, NodeType::Member)
        });

    // same-line block comments before member semicolons stay on member boundaries
    // declaration semicolon seams are owned by declaration handlers
    if comment_context.comment_is_star
        && !comment_context.has_leading_newline
        && token_after_is_semicolon
        && !comment_context.semicolon_after_control_head_empty_body
        && let Some(target_owner) = member_owner_for_semicolon_comment
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Handle middle seam rules for parenthesized prefixes and semicolon guards.
fn attach_expression_middle_parenthesized_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam = comment_context.seam;
    let following_owner = comment_context.following_owner;
    let token_after_span = comment_context.token_after_span;
    let following_token_owner = comment_context.following_token_owner;
    let has_leading_newline = comment_context.has_leading_newline;
    let comment_is_star = comment_context.comment_is_star;
    let comment_is_line = comment_context.comment_is_line;

    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);
    let token_before_is_open_parenthesis = seam.token_before_is(TokenType::OpenParenthesis);
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

    // own-line comments after `(` should bind to the full expression that starts at the rhs token
    if has_leading_newline && comment_is_line && token_before_is_open_parenthesis {
        let target_node = token_after_span
            .and_then(|token_after_span| {
                find_preferred_owner_starting_at(tree, token_after_span.span)
            })
            .or(following_token_owner)
            .or(following_owner)
            .map(|owner| {
                token_after_span.map_or(owner, |token_after_span| {
                    promote_owner_by_shared_start(tree, parents, owner, token_after_span.span.start)
                })
            });
        if let Some(target_node) = target_node {
            // type-cast owners inside parenthesized wrappers should bind to the consuming group
            let mut target_node = target_node;
            if tree.get_node_type(target_node) == NodeType::Expression {
                let expression_id = LocalNodeId::<Expression>::new(target_node);
                let is_cast_like_type_binary = matches!(
                    tree.get(expression_id),
                    Expression::TypeBinary {
                        operator: ast::TypeBinaryOperator::Cast
                            | ast::TypeBinaryOperator::Satisfies,
                        ..
                    }
                );

                if is_cast_like_type_binary
                    && let Some(parent_id) = parents.get_by_id(target_node)
                    && tree.get_node_type(parent_id) == NodeType::Expression
                {
                    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
                    let is_direct_parenthesized_wrapper = matches!(
                        tree.get(parent_expression_id),
                        Expression::Parenthesized { expression } if expression.id == target_node
                    );

                    if is_direct_parenthesized_wrapper {
                        if let Some(grandparent_id) = parents.get_by_id(parent_id)
                            && tree.get_node_type(grandparent_id) == NodeType::Expression
                        {
                            let grandparent_expression_id =
                                LocalNodeId::<Expression>::new(grandparent_id);
                            target_node = match tree.get(grandparent_expression_id) {
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
                            };
                        }
                    }
                }
            }

            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    None
}

/// Handle middle seam rules around open-brace and mapped-type seams.
fn attach_expression_middle_open_brace_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let following_owner = comment_context.following_owner;
    let token_after_span = comment_context.token_after_span;
    let enclosing_owner = comment_context.enclosing_owner;
    let following_token_owner = comment_context.following_token_owner;
    let has_leading_newline = comment_context.has_leading_newline;
    let has_trailing_newline = comment_context.has_trailing_newline;
    let comment_is_line = comment_context.comment_is_line;
    let comment_is_star = comment_context.comment_is_star;
    let is_inline_star_comment = comment_context.is_inline_star_comment;

    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);
    let token_after_is_open_bracket = seam.token_after_is(TokenType::OpenBracket);
    let token_before_is_open_brace = seam.token_before_is(TokenType::OpenBrace);
    let token_before_is_spread = seam.token_before_is(TokenType::Spread);
    let token_after_is_open_bracket_or_open_parenthesis =
        token_after_is_open_bracket || token_after_is_open_parenthesis;

    // inline comments after `{` before grouped keys and expressions stay on the rhs expression
    if is_inline_star_comment
        && token_before_is_open_brace
        && token_after_is_open_bracket_or_open_parenthesis
    {
        let target_node = following_token_owner.or(following_owner).map(|owner| {
            token_after_span.map_or(owner, |token| {
                promote_owner_by_shared_start(tree, parents, owner, token.span.start)
            })
        });
        if let Some(target_node) = target_node {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    // own-line mapped-type entry comments after `{` should stay with the mapped member head
    if has_leading_newline
        && !has_trailing_newline
        && (comment_is_star || comment_is_line)
        && token_before_is_open_brace
        && token_after_is_open_bracket
        && let Some(target_node) = [
            enclosing_owner,
            following_owner,
            following_token_owner,
            preceding_owner,
        ]
        .into_iter()
        .flatten()
        .find_map(|owner| promote_owner_to_mapped_type_expression_ancestor(tree, parents, owner))
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::BlockPrefix));
    }

    // comments between rest spread and binding names stay on the parameter owner
    if is_inline_star_comment
        && token_before_is_spread
        && let Some(target_node) = enclosing_owner.or(preceding_owner).or(following_owner)
    {
        let target_node =
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Parameter)
                .unwrap_or(target_node);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    None
}

/// Handle middle seam rules for declaration heads and index boundaries.
fn attach_expression_middle_declaration_and_index_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam_context = comment_context.seam_context;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let following_owner = comment_context.following_owner;
    let token_before_span = comment_context.token_before_span;
    let token_before_source_span = comment_context.token_before_source_span;
    let enclosing_owner = comment_context.enclosing_owner;
    let preceding_token_owner = comment_context.preceding_token_owner;
    let following_token_owner = comment_context.following_token_owner;
    let has_leading_newline = comment_context.has_leading_newline;
    let is_trailing_line_comment = comment_context.is_trailing_line_comment;
    let comment_is_line = comment_context.comment_is_line;

    let token_after_is_less_than = seam.token_after_is(TokenType::LessThan);
    let token_after_is_open_bracket = seam.token_after_is(TokenType::OpenBracket);
    let token_before_is_comma = seam.token_before_is(TokenType::Comma);
    let token_before_is_open_brace = seam.token_before_is(TokenType::OpenBrace);
    let token_after_is_index_boundary = token_after_is_open_bracket;

    // declaration generic head seams should stay on declaration heads, not jsx trees
    let token_after_starts_tree_expression = [following_token_owner, following_owner]
        .into_iter()
        .flatten()
        .any(|owner| {
            let owner = promote_owner_to_tree_expression_parent(tree, parents, owner);
            tree.get_node_type(owner) == NodeType::Expression
                && matches!(
                    tree.get(LocalNodeId::<Expression>::new(owner)),
                    Expression::TreeExpression { .. }
                )
        });
    if is_trailing_line_comment && token_after_is_less_than && !token_after_starts_tree_expression {
        let declaration_target = seam_context
            .token_after
            .and_then(|token_after_index| {
                find_owner_at_or_after_token(tree, seam_context.semantic_tokens, token_after_index)
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

    // own-line seam comments before index operators stay on the seam operation
    if has_leading_newline
        && token_after_is_index_boundary
        && comment_context.seam_has_shared_expression_owner
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
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        comment_context,
        &[
            attach_expression_middle_label_and_separator_comments,
            attach_expression_middle_parenthesized_comments,
            attach_expression_middle_open_brace_comments,
            attach_expression_middle_declaration_and_index_comments,
        ],
    )
}

/// Return one ordered candidate-owner list for expression seam ownership checks.
fn expression_seam_candidate_owners(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> [Option<u32>; 5] {
    [
        comment_context.enclosing_owner,
        comment_context.preceding_owner,
        comment_context.following_owner,
        comment_context.preceding_token_owner,
        comment_context.following_token_owner,
    ]
}

/// Return one token type after the next non-trivia token beyond seam `token_after`.
fn token_after_close_parenthesis_following_type(
    seam_context: &CommentSeamContext<'_>,
) -> Option<TokenType> {
    seam_context
        .token_after
        .and_then(|index| next_non_trivia_token_index(seam_context.semantic_tokens, index))
        .and_then(|index| seam_context.semantic_tokens.get(index))
        .map(|token| token.token.ty)
}

/// Handle pre-placement semicolon-guard ownership.
fn attach_expression_pre_placement_semicolon_guard_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    // own-line semicolon-guard seams resolve in expression routing for semicolon-adjacent shapes
    if (comment_context.seam.token_before_is(TokenType::Semicolon)
        || comment_context.seam.token_after_is(TokenType::Semicolon))
        && !comment_context.semicolon_after_control_head_empty_body
    {
        return attach_semicolon_guard_own_line_comment(
            comment_context.tree,
            comment_context.parents,
            comment_context.seam_context,
            comment_context.seam,
            comment_context.owners,
        );
    }

    None
}

/// Handle pre-placement spread seam ownership.
fn attach_expression_pre_placement_spread_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    // spread seams keep ownership on the spread argument in expression contexts
    if (comment_context.token_after_is_spread || comment_context.token_before_is_spread)
        && let Some(target_node) = expression_seam_candidate_owners(comment_context)
            .into_iter()
            .flatten()
            .find_map(|owner| {
                promote_owner_to_spread_argument_ancestor(
                    comment_context.tree,
                    comment_context.parents,
                    owner,
                )
            })
    {
        let target_node =
            normalize_formatter_trivia_target_owner(comment_context.tree, target_node);
        if comment_context.token_after_is_spread {
            return Some((Some(target_node), AnnotationPosition::BlockPrefix));
        }

        let position = if comment_context.comment_is_line {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_node), position));
    }

    // inline comments inside empty object spread values stay inside the object literal
    if comment_context.is_inline_star_comment
        && comment_context.token_before_is_open_brace
        && comment_context.token_after_is_close_brace
        && let Some(spread_argument_owner) = expression_seam_candidate_owners(comment_context)
            .into_iter()
            .flatten()
            .find_map(|owner| {
                promote_owner_to_spread_argument_ancestor(
                    comment_context.tree,
                    comment_context.parents,
                    owner,
                )
            })
    {
        let spread_argument_id = LocalNodeId::<Argument>::new(spread_argument_owner);
        if let Argument::Spread {
            value: spread_value_id,
            ..
        } = comment_context.tree.get(spread_argument_id)
            && matches!(
                comment_context.tree.get(*spread_value_id),
                Expression::ObjectExpression { properties, .. } if properties.is_empty()
            )
        {
            let target_node =
                normalize_formatter_trivia_target_owner(comment_context.tree, spread_value_id.id);
            return Some((Some(target_node), AnnotationPosition::BlockInfix));
        }
    }

    None
}

/// Handle pre-placement dependency attribute seam ownership.
fn attach_expression_pre_placement_dependency_attribute_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let seam = comment_context.seam;
    let token_after_is_dependency_attribute_keyword =
        seam.token_after_is_keyword(CommentSeamKeyword::With);
    if !token_after_is_dependency_attribute_keyword {
        return None;
    }

    let seam_candidates = [
        comment_context.preceding_token_owner,
        comment_context.preceding_owner,
        comment_context.enclosing_owner,
        comment_context.following_owner,
        comment_context.following_token_owner,
    ];
    let Some(target_node) = dependency_attribute_expression_owner_from_seam_candidates(
        comment_context.tree,
        seam_candidates,
    ) else {
        return None;
    };

    let target_node = normalize_formatter_trivia_target_owner(comment_context.tree, target_node);
    Some((Some(target_node), AnnotationPosition::BlockInfix))
}

/// Handle pre-placement empty dependency item seam ownership.
fn attach_expression_pre_placement_empty_dependency_item_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let seam = comment_context.seam;
    if !comment_context.is_inline_star_comment {
        return None;
    }

    if !seam.token_before_is(TokenType::OpenBrace) || !seam.token_after_is(TokenType::CloseBrace) {
        return None;
    }

    let seam_candidates = [
        comment_context.preceding_token_owner,
        comment_context.preceding_owner,
        comment_context.enclosing_owner,
        comment_context.following_owner,
        comment_context.following_token_owner,
    ];
    let Some(target_node) = empty_dependency_expression_owner_from_seam_candidates(
        comment_context.tree,
        comment_context.parents,
        seam_candidates,
    ) else {
        return None;
    };

    let target_node = normalize_formatter_trivia_target_owner(comment_context.tree, target_node);
    Some((Some(target_node), AnnotationPosition::LinePostfixBoundary))
}

/// Resolve pre-placement expression seam rules.
fn try_attach_comment_expression_pre_placement_rules(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        comment_context,
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
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam_context = comment_context.seam_context;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let following_owner = comment_context.following_owner;
    let preceding_token_owner = comment_context.preceding_token_owner;
    let following_token_owner = comment_context.following_token_owner;
    let enclosing_owner = comment_context.enclosing_owner;
    let comment_is_line = comment_context.comment_is_line;
    let comment_is_star = comment_context.comment_is_star;
    let has_leading_newline = comment_context.has_leading_newline;
    let has_trailing_newline = comment_context.has_trailing_newline;
    let is_inline_star_comment = comment_context.is_inline_star_comment;

    let token_after_is_maybe = seam.token_after_is(TokenType::Maybe);
    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);

    // inline optional-call operator seam comments stay on the call base segment
    // this follows prettier and oxfmt output for `call?./* comment */()`
    let seam_is_optional_call_parenthesis_separator =
        seam.token_before_is(TokenType::Dot) && seam.token_after_is(TokenType::OpenParenthesis);
    let seam_is_optional_call_operator_or_parenthesis_separator =
        seam_is_optional_call_operator(seam_context, seam)
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

/// Handle tail seam rules for chain seams and call-like boundaries.
fn attach_expression_tail_chain_and_call_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let token_before_source_span = comment_context.token_before_source_span;
    let preceding_token_owner = comment_context.preceding_token_owner;
    let has_leading_newline = comment_context.has_leading_newline;
    let has_trailing_newline = comment_context.has_trailing_newline;
    let comment_is_line = comment_context.comment_is_line;
    let comment_is_star = comment_context.comment_is_star;
    let is_inline_star_comment = comment_context.is_inline_star_comment;
    let seam_has_shared_expression_owner = comment_context.seam_has_shared_expression_owner;
    let enclosing_owner_first_dynamic_argument =
        comment_context.enclosing_owner_first_dynamic_argument;

    let token_after_is_dot = seam.token_after_is(TokenType::Dot);
    let token_after_is_open_bracket = seam.token_after_is(TokenType::OpenBracket);
    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);
    let token_before_is_comma = seam.token_before_is(TokenType::Comma);
    let token_before_is_greater_than = seam.token_before_is(TokenType::GreaterThan);
    let token_before_is_open_brace = seam.token_before_is(TokenType::OpenBrace);
    let token_before_is_open_parenthesis = seam.token_before_is(TokenType::OpenParenthesis);
    let seam_candidates = expression_seam_candidate_owners(comment_context);
    let seam_is_call_like_open_parenthesis_boundary = token_after_is_open_parenthesis
        && seam_candidates_have_call_like_expression_ancestor(tree, parents, seam_candidates);

    // same-line line comments between call-like callees and `(` stay as call postfix comments
    if comment_is_line
        && !has_leading_newline
        && seam_is_call_like_open_parenthesis_boundary
        && let Some(call_owner) =
            first_call_like_expression_owner_from_seam_candidates(tree, parents, seam_candidates)
    {
        let call_owner_first_dynamic_argument =
            first_dynamic_argument_owner_for_call_like(tree, call_owner);
        let call_owner_has_static_arguments =
            call_like_expression_has_static_arguments(tree, call_owner);

        if call_owner_has_static_arguments || call_owner_first_dynamic_argument.is_none() {
            let target_node = normalize_formatter_trivia_target_owner(tree, call_owner);
            return Some((Some(target_node), AnnotationPosition::LinePostfix));
        }

        if let Some(target_node) = call_owner_first_dynamic_argument {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    // seam comments before index and inline member operators stay with the left segment
    if !has_leading_newline
        && (token_after_is_open_bracket || (token_after_is_dot && comment_is_star))
        && seam_has_shared_expression_owner
        && !token_before_is_comma
        && !token_before_is_open_brace
        && let Some(target_node) = normalized_preceding_expression_owner_for_tail_seam(
            tree,
            parents,
            preceding_token_owner,
            preceding_owner,
            token_before_source_span,
        )
    {
        if has_trailing_newline {
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // end-of-line line comments before member dots stay on the left expression boundary
    if comment_is_line
        && !has_leading_newline
        && token_after_is_dot
        && !token_before_is_comma
        && !token_before_is_open_brace
        && let Some(target_node) = normalized_preceding_expression_owner_for_tail_seam(
            tree,
            parents,
            preceding_token_owner,
            preceding_owner,
            token_before_source_span,
        )
    {
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // comments between static type-argument closers and call `(` follow call-like ownership
    let seam_is_call_like_static_argument_call_boundary = token_before_is_greater_than
        && token_after_is_open_parenthesis
        && seam_is_call_like_open_parenthesis_boundary;
    if comment_is_line && seam_is_call_like_static_argument_call_boundary {
        let call_owner =
            first_call_like_expression_owner_from_seam_candidates(tree, parents, seam_candidates);

        if has_leading_newline && let Some(target_node) = enclosing_owner_first_dynamic_argument {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        if let Some(target_node) = call_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfix));
        }
    }

    // inline block comments right after `(` in call and new expressions stay on the first argument
    if is_inline_star_comment
        && token_before_is_open_parenthesis
        && let Some(target_node) = enclosing_owner_first_dynamic_argument
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // inline block comments between call callees and `(` stay inside non-empty argument lists
    if is_inline_star_comment
        && token_after_is_open_parenthesis
        && seam_is_call_like_open_parenthesis_boundary
    {
        if let Some(target_node) = enclosing_owner_first_dynamic_argument {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }

        if let Some(target_node) = preceding_token_owner.or(preceding_owner) {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfix));
        }
    }

    None
}

/// Handle tail seam rules for object and logical-operator boundaries.
fn attach_expression_tail_object_and_logical_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let following_owner = comment_context.following_owner;
    let token_before_span = comment_context.token_before_span;
    let token_after_source_span = comment_context.token_after_source_span;
    let following_token_owner = comment_context.following_token_owner;
    let enclosing_owner = comment_context.enclosing_owner;
    let comment_is_line = comment_context.comment_is_line;
    let is_inline_star_comment = comment_context.is_inline_star_comment;
    let is_trailing_line_comment = comment_context.is_trailing_line_comment;

    let token_after_is_open_bracket = seam.token_after_is(TokenType::OpenBracket);
    let token_after_is_close_brace = seam.token_after_is(TokenType::CloseBrace);
    let token_before_is_open_brace = seam.token_before_is(TokenType::OpenBrace);
    let token_before_is_logical_operator = matches!(
        seam.token_before_type,
        Some(TokenType::LogicalAnd | TokenType::LogicalOr | TokenType::Coalesce)
    );

    // comments between object open braces and computed keys stay inside the object
    if is_inline_star_comment && token_before_is_open_brace && token_after_is_open_bracket {
        if let Some(target_node) = preceding_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }

        if let Some(target_node) = enclosing_owner {
            if tree.get_node_type(target_node) == NodeType::Expression {
                let expression_id = LocalNodeId::<Expression>::new(target_node);
                if let Expression::ObjectExpression { properties, .. } = tree.get(expression_id)
                    && let Some(first_property) = properties.first().copied()
                {
                    return Some((Some(first_property.id), AnnotationPosition::LinePrefix));
                }
            }

            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::LinePrefix));
        }
    }

    // comments inside empty object literals stay as object infix comments
    if (is_inline_star_comment || comment_is_line)
        && token_before_is_open_brace
        && token_after_is_close_brace
    {
        let object_owner = [
            token_before_span.and_then(|token| find_preferred_owner_starting_at(tree, token.span)),
            following_token_owner,
            following_owner,
            enclosing_owner,
        ]
        .into_iter()
        .flatten()
        .find_map(|owner| empty_object_expression_owner_for_candidate(tree, owner));
        if let Some(target_node) = object_owner {
            let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
            return Some((Some(target_node), AnnotationPosition::BlockInfix));
        }
    }

    // line comments after logical operators belong to the right operand
    if is_trailing_line_comment
        && token_before_is_logical_operator
        && let Some(target_owner) = following_owner
    {
        let target_owner = token_after_source_span
            .map(|span| promote_owner_by_shared_start(tree, parents, target_owner, span.start))
            .unwrap_or(target_owner);
        let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
        return Some((Some(target_owner), AnnotationPosition::LinePrefix));
    }

    None
}

/// Handle tail seam rules around ternary branches.
fn attach_expression_tail_ternary_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let token_before_source_span = comment_context.token_before_source_span;
    let preceding_token_owner = comment_context.preceding_token_owner;
    let has_leading_newline = comment_context.has_leading_newline;
    let has_trailing_newline = comment_context.has_trailing_newline;
    let comment_is_line = comment_context.comment_is_line;
    let comment_is_star = comment_context.comment_is_star;
    let is_inline_star_comment = comment_context.is_inline_star_comment;
    let ternary_enclosing_owner = comment_context.ternary_enclosing_owner;
    let ternary_following_owner = comment_context.ternary_following_owner;
    let is_ternary_seam = comment_context.is_ternary_seam;

    let token_after_is_colon = seam.token_after_is(TokenType::Colon);
    let token_after_is_maybe = seam.token_after_is(TokenType::Maybe);
    let token_before_is_maybe = seam.token_before_is(TokenType::Maybe);
    let token_before_is_colon = seam.token_before_is(TokenType::Colon);

    // inline block comments before `:` in ternaries follow oxfmt branch ownership
    if is_ternary_seam
        && !seam.token_before_is_return_type_colon
        && is_inline_star_comment
        && !has_leading_newline
        && token_after_is_colon
    {
        let ternary_else_owner = ternary_enclosing_owner
            .filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)
            .and_then(|owner| {
                let expression_id = LocalNodeId::<Expression>::new(owner);
                match tree.get(expression_id) {
                    Expression::If {
                        kind: ast::IfKind::Ternary,
                        else_expression: Some(else_expression),
                        ..
                    } => Some(else_expression.id),
                    _ => None,
                }
            });

        if let Some(target_owner) = ternary_else_owner
            .filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)
            .filter(|owner| {
                matches!(
                    tree.get(LocalNodeId::<Expression>::new(*owner)),
                    Expression::TreeExpression { .. }
                )
            })
        {
            let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
            return Some((Some(target_owner), AnnotationPosition::LinePrefix));
        }

        if let Some(target_owner) = preceding_token_owner.or(preceding_owner) {
            let target_owner = normalize_owner_with_shared_end(
                tree,
                parents,
                target_owner,
                token_before_source_span,
            );
            let target_owner = normalize_formatter_trivia_target_owner(tree, target_owner);
            return Some((Some(target_owner), AnnotationPosition::LinePostfixBoundary));
        }
    }

    // comments before ternary question separators stay on the preceding branch
    if is_ternary_seam
        && !seam.token_before_is_return_type_colon
        && (comment_is_star || comment_is_line)
        && !has_leading_newline
        && token_after_is_maybe
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_source_span);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let position = if has_trailing_newline || comment_is_line {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::LinePostfix
        };
        return Some((Some(target_node), position));
    }

    // comments after ternary separators stay with the following branch
    if is_ternary_seam
        && !seam.token_before_is_return_type_colon
        && comment_is_star
        && (token_before_is_maybe || token_before_is_colon)
        && let Some(target_owner) = ternary_following_owner
    {
        let position = if has_leading_newline || has_trailing_newline {
            AnnotationPosition::BlockPrefix
        } else {
            AnnotationPosition::LinePrefix
        };
        return Some((Some(target_owner), position));
    }

    None
}

/// Handle tail seam rules for expression boundaries and trailing grouped seams.
fn attach_expression_tail_expression_boundary_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let token_before_source_span = comment_context.token_before_source_span;
    let preceding_token_owner = comment_context.preceding_token_owner;
    let enclosing_owner = comment_context.enclosing_owner;
    let has_leading_newline = comment_context.has_leading_newline;
    let has_trailing_newline = comment_context.has_trailing_newline;
    let comment_is_line = comment_context.comment_is_line;
    let comment_is_star = comment_context.comment_is_star;
    let is_trailing_line_comment = comment_context.is_trailing_line_comment;
    let is_ternary_seam = comment_context.is_ternary_seam;
    let token_after_close_parenthesis_is_logical_operator =
        comment_context.token_after_close_parenthesis_is_logical_operator;

    let token_after_is_close_brace = seam.token_after_is(TokenType::CloseBrace);
    let token_after_is_close_parenthesis = seam.token_after_is(TokenType::CloseParenthesis);
    let token_before_is_close_parenthesis = seam.token_before_is(TokenType::CloseParenthesis);

    // trailing line comments after non-block if consequents stay on the consequent statement
    if is_trailing_line_comment
        && !token_before_is_close_parenthesis
        && !token_after_is_close_parenthesis
        && !is_ternary_seam
        && let Some(enclosing_owner) = enclosing_owner
        && let Some(then_owner) = if_expression_then_owner_without_else(tree, enclosing_owner)
    {
        let attachment = (Some(then_owner), AnnotationPosition::LinePostfixBoundary);
        return Some(attachment);
    }

    // comments before tree-expression container `}` stay on the container expression
    if !has_leading_newline
        && (comment_is_line || comment_is_star)
        && token_after_is_close_brace
        && let Some(preceding_expression_owner) = preceding_owner
            .filter(|owner| tree.get_node_type(*owner) == NodeType::Expression)
            .filter(|owner| {
                parents
                    .get_by_id(*owner)
                    .is_some_and(|parent_id| tree.get_node_type(parent_id) == NodeType::Argument)
            })
    {
        let position = if comment_is_line || has_trailing_newline {
            AnnotationPosition::LinePostfixBoundary
        } else {
            AnnotationPosition::LinePostfix
        };
        return Some((Some(preceding_expression_owner), position));
    }

    // trailing line comments before nested `)` tokens stay on the inner grouped expression
    if is_trailing_line_comment
        && token_before_is_close_parenthesis
        && token_after_is_close_parenthesis
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node = if tree.get_node_type(target_node) == NodeType::Expression {
            target_node
        } else {
            promote_owner_to_node_type_ancestor(tree, parents, target_node, NodeType::Expression)
                .unwrap_or(target_node)
        };
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // trailing line comments before one closing `)` stay on the grouped-expression boundary owner
    if comment_is_line
        && has_leading_newline
        && token_after_is_close_parenthesis
        && token_after_close_parenthesis_is_logical_operator
        && !token_before_is_close_parenthesis
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node =
            normalize_owner_with_shared_end(tree, parents, target_node, token_before_source_span);
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    None
}

/// Handle tail seam rules for grouped close-parenthesis ownership.
fn attach_expression_tail_grouping_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let token_before_source_span = comment_context.token_before_source_span;
    let following_owner = comment_context.following_owner;
    let preceding_token_owner = comment_context.preceding_token_owner;
    let following_token_owner = comment_context.following_token_owner;
    let enclosing_owner = comment_context.enclosing_owner;
    let is_inline_star_comment = comment_context.is_inline_star_comment;
    let is_ternary_seam = comment_context.is_ternary_seam;

    let token_after_is_close_parenthesis = seam.token_after_is(TokenType::CloseParenthesis);
    let token_before_is_close_parenthesis = seam.token_before_is(TokenType::CloseParenthesis);

    // nested `)` seams keep inline block comments on the outer grouped wrapper
    if is_inline_star_comment
        && token_before_is_close_parenthesis
        && token_after_is_close_parenthesis
        && let Some(target_node) = following_token_owner.or(following_owner).and_then(|owner| {
            promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
        })
        && matches!(
            tree.get(LocalNodeId::<Expression>::new(target_node)),
            Expression::Parenthesized { .. }
        )
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // inline comments before `)` in do-while conditions should stay on the condition expression
    if is_inline_star_comment
        && token_after_is_close_parenthesis
        && [
            preceding_token_owner,
            preceding_owner,
            following_owner,
            enclosing_owner,
            following_token_owner,
        ]
        .into_iter()
        .flatten()
        .any(|owner| owner_has_do_while_expression_ancestor(tree, parents, owner))
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // inline comments before `)` in ternary branches stay on the inner branch expression
    if is_inline_star_comment
        && token_after_is_close_parenthesis
        && is_ternary_seam
        && let Some(target_node) = preceding_token_owner.or(preceding_owner)
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
    }

    // inline comments before one closing `)` stay on the inner operand
    if is_inline_star_comment
        && token_after_is_close_parenthesis
        && preceding_token_owner
            .or(preceding_owner)
            .is_some_and(|owner| tree.get_node_type(owner) == NodeType::Expression)
        && let Some(target_node) = normalized_preceding_expression_owner_for_tail_seam(
            tree,
            parents,
            preceding_token_owner,
            preceding_owner,
            token_before_source_span,
        )
        && owner_has_unary_expression_ancestor(tree, parents, target_node)
    {
        return Some((Some(target_node), AnnotationPosition::LinePostfix));
    }

    // inline comments before `)` on grouped expressions stay with the grouped wrapper
    if is_inline_star_comment && token_after_is_close_parenthesis {
        let is_nested_declaration_wrapper_boundary = preceding_owner
            .zip(following_owner)
            .is_some_and(|(left_owner, right_owner)| {
                let left_owner = normalize_formatter_trivia_target_owner(tree, left_owner);
                let right_owner = normalize_formatter_trivia_target_owner(tree, right_owner);
                tree.get_node_type(left_owner) == NodeType::Declaration
                    && tree.get_node_type(right_owner) == NodeType::Declaration
                    && left_owner != right_owner
            });

        if !is_nested_declaration_wrapper_boundary
            && let Some(target_node) = preceding_token_owner
                .or(preceding_owner)
                .and_then(|owner| {
                    promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
                })
                .or_else(|| {
                    enclosing_owner.and_then(|owner| {
                        promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner)
                    })
                })
            && tree.get_node_type(target_node) == NodeType::Expression
            && matches!(
                tree.get(LocalNodeId::<Expression>::new(target_node)),
                Expression::Parenthesized { .. }
            )
            && !is_direct_lambda_body_expression_owner(tree, parents, target_node)
        {
            return Some((Some(target_node), AnnotationPosition::LinePostfixBoundary));
        }
    }

    None
}

/// Handle tail seam rules for cast and parenthesized cast targets.
fn attach_expression_tail_cast_comments(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    let tree = comment_context.tree;
    let parents = comment_context.parents;
    let seam = comment_context.seam;
    let preceding_owner = comment_context.preceding_owner;
    let following_owner = comment_context.following_owner;
    let token_after_span = comment_context.token_after_span;
    let following_token_owner = comment_context.following_token_owner;
    let enclosing_owner = comment_context.enclosing_owner;
    let is_inline_star_comment = comment_context.is_inline_star_comment;
    let seam_is_new_signature_declaration = comment_context.seam_is_new_signature_declaration;
    let ternary_enclosing_owner = comment_context.ternary_enclosing_owner;

    let token_after_is_open_brace = seam.token_after_is(TokenType::OpenBrace);
    let token_after_is_open_parenthesis = seam.token_after_is(TokenType::OpenParenthesis);
    let token_before_is_open_parenthesis = seam.token_before_is(TokenType::OpenParenthesis);
    let token_before_is_maybe = seam.token_before_is(TokenType::Maybe);

    // comments before closure-cast object literals stay with the rhs cast target
    if is_inline_star_comment
        && token_after_is_open_brace
        && token_before_is_open_parenthesis
        && let Some(mut target_node) = following_token_owner
            .or(following_owner)
            .or(enclosing_owner)
    {
        if tree.get_node_type(target_node) != NodeType::Expression
            && let Some(expression_target) = promote_owner_to_node_type_ancestor(
                tree,
                parents,
                target_node,
                NodeType::Expression,
            )
        {
            target_node = expression_target;
        }

        if tree.get_node_type(target_node) == NodeType::Expression
            && let Expression::Parenthesized { expression } =
                tree.get(LocalNodeId::<Expression>::new(target_node))
        {
            target_node = expression.id;
        }

        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    // comments before parenthesized cast targets stay with the grouped expression
    if is_inline_star_comment
        && token_after_is_open_parenthesis
        && !(token_before_is_maybe && ternary_enclosing_owner.is_none())
        && !seam_is_new_signature_declaration
        && let Some(target_node) = (if token_before_is_open_parenthesis {
            following_token_owner.or(following_owner)
        } else {
            following_token_owner
                .or(following_owner)
                .or(enclosing_owner)
        })
        .map(|owner| {
            token_after_span.map_or(owner, |token| {
                promote_owner_by_shared_start(tree, parents, owner, token.span.start)
            })
        })
        .and_then(|owner| {
            promote_owner_to_parenthesized_expression_ancestor(tree, parents, owner).or(Some(owner))
        })
    {
        let target_node = normalize_formatter_trivia_target_owner(tree, target_node);
        let target_node = if preceding_owner
            .is_some_and(|owner| tree.get_node_type(owner) == NodeType::Declaration)
            && tree.get_node_type(target_node) == NodeType::Expression
            && let Expression::Parenthesized { expression } =
                tree.get(LocalNodeId::<Expression>::new(target_node))
        {
            expression.id
        } else {
            target_node
        };
        return Some((Some(target_node), AnnotationPosition::LinePrefix));
    }

    None
}

/// Resolve tail expression seam rules around optional chains, ternaries, and grouped expressions.
fn try_attach_comment_expression_tail_rules(
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    run_expression_comment_handlers(
        comment_context,
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
    comment_context: &ExpressionCommentContext<'_, '_>,
) -> Option<CommentAttachment> {
    // current placement classes share the same middle and tail routing
    if let Some(attachment) = try_attach_comment_expression_middle_rules(comment_context) {
        return Some(attachment);
    }

    try_attach_comment_expression_tail_rules(comment_context)
}

/// Attach one expression seam comment fallback.
fn attach_expression_comment_fallback(
    comment_context: &ExpressionCommentContext<'_, '_>,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
) -> Option<CommentAttachment> {
    try_attach_comment_expression_operator(
        comment_context.tree,
        comment_context.parents,
        comment_context.seam_context,
        comment_context.seam,
        enclosing_owner_cache,
        comment_context.owners,
    )
}

/// Resolve expression and type seam comment rules.
pub(crate) fn try_attach_comment_expression(
    tree: &NodeTree,
    _owner_index: &FormatterTriviaOwnerIndex,
    parents: &NodeParentIndex,
    context: &CommentSeamContext<'_>,
    seam: &CommentSeamData,
    enclosing_owner_cache: &mut CommentEnclosingOwnerCache,
    owners: CommentAttachmentNeighbors,
) -> Option<CommentAttachment> {
    let comment_context = build_expression_comment_context(
        tree,
        parents,
        context,
        seam,
        enclosing_owner_cache,
        owners,
    );

    // call-like separator seams before `)` should use statement-level separator ownership
    if comment_context.should_route_call_like_separator_to_statement() {
        return None;
    }

    // decorator seams belong to declaration-specific routing.
    if comment_context.token_after_is_at {
        return None;
    }

    // own-line member-dot seams are owned by dedicated own-line routing
    let own_line_member_dot_seam = comment_context.has_leading_newline
        && (comment_context.comment_is_line || comment_context.comment_is_star)
        && comment_context.seam.token_after_is(TokenType::Dot);
    if own_line_member_dot_seam {
        return None;
    }

    // file-head line comments should stay as statement-prefix comments
    let file_head_end_of_line_comment = comment_context.comment_is_line
        && comment_context.seam_context.token_before.is_none()
        && comment_context.has_trailing_newline;
    if file_head_end_of_line_comment {
        return None;
    }

    // inline comments before statement semicolons use semicolon/end-of-line ownership routing
    let inline_before_semicolon_seam = comment_context.comment_is_line
        && !comment_context.has_leading_newline
        && comment_context.seam.token_after_is(TokenType::Semicolon)
        && !comment_context.seam.token_before_is(TokenType::Semicolon);
    if inline_before_semicolon_seam {
        return None;
    }

    if let Some(attachment) = try_attach_comment_expression_head_rules(&comment_context) {
        return Some(attachment);
    }

    if let Some(attachment) = try_attach_comment_expression_pre_placement_rules(&comment_context) {
        return Some(attachment);
    }

    if let Some(attachment) = attach_expression_comment_by_placement(&comment_context) {
        return Some(attachment);
    }

    if let Some(attachment) =
        attach_expression_comment_fallback(&comment_context, enclosing_owner_cache)
    {
        return Some(attachment);
    }

    None
}
