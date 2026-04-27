use crate::annotation::{
    FormatLeadingComments, FormatTrailingComments, decorator_prefix_annotations,
    format_dangling_comments, format_node_with_trailing_comments, format_trailing_comments,
    infix_or_postfix_annotations, postfix_annotations, prefix_annotations,
    prefix_comments_before_decorators, write_vertical_prefix_annotations,
};
use crate::chain::transparent_inner_expression;
use crate::collection::member::format_block_of_members;
use crate::context::FormatNodeWithoutTrailingComments;
use crate::declaration::declaration::{
    declaration_export_token, format_declaration_export_modifier, format_super_type_clause,
};
use crate::declaration::empty_block_with_infix_annotations;
use crate::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    write_generic_parameter_list,
};
use crate::expression::{expression_needs_parentheses_in_parent, format_type_member_block_list};
use crate::operator::format_generic_argument_list;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    ClassDeclaration, Declaration, Decorator, EnumDeclaration, EnumField, EnumKind, Expression,
    GenericArgument, GenericParameter, InterfaceDeclaration, InterfaceHeritage, Keyword,
    LocalNodeId, LocalNodeIdAny, Member, Node, NodeType, StructDeclaration, TokenSpan, TokenType,
    Tree, TreeImpl, TypeExpression, TypeMember, WhereClause,
};
use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

/// Write one declaration generic parameter list.
fn write_declaration_generic_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<GenericParameter>],
) -> FormatResult<()> {
    // generic parameters
    if !generic_parameters.is_empty() {
        write_generic_parameter_list(
            f,
            generic_parameters,
            default_generic_parameter_trailing_separator(f),
        )?;
    }

    Ok(())
}

/// Write one declaration where clause list.
fn write_declaration_where_clauses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    where_clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    // where clauses
    if !where_clauses.is_empty() {
        format_where_clause_with_break(f, where_clauses)?;
    }

    Ok(())
}

/// Return one combined span for one node slice.
fn combined_node_span<T>(
    context: &DestackFormatContext<'_>,
    node_ids: &[LocalNodeId<T>],
) -> Option<Span>
where
    T: Node + Clone,
    Tree: TreeImpl<T>,
{
    let first_id = node_ids.first().copied()?;
    let last_id = node_ids.last().copied()?;
    let first_span = context.span(first_id);
    let last_span = context.span(last_id);

    Some(Span::new(first_span.file, first_span.start, last_span.end))
}

/// Return one generic-argument-list span after one expression span.
fn generic_argument_list_span_after_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> Option<Span> {
    if generic_arguments.is_empty() {
        return None;
    }

    let expression_span = context.span(expression_id);
    let arguments_span = combined_node_span(context, generic_arguments)?;
    let open_token = context.next_non_trivia_token_after_span(expression_span)?;

    if open_token.token.ty != TokenType::LessThan {
        return Some(arguments_span);
    }

    let close_token = context.next_non_trivia_token_after_span(arguments_span);
    let close_end = close_token.map_or(arguments_span.end, |token| token.span.end);

    Some(Span::new(
        expression_span.file,
        open_token.span.start,
        close_end,
    ))
}

/// Write one class or interface heritage type list.
fn write_heritage_type_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    enclosing_span: Span,
    types: &[LocalNodeId<TypeExpression>],
) -> FormatResult<()> {
    for (index, type_id) in types.iter().copied().enumerate() {
        let next_type = types.get(index + 1).copied();

        if let Some(next_type) = next_type {
            let type_span = f.context().span(type_id);
            let next_type_start = f.context().span(next_type).start;
            let comma_token = f
                .context()
                .next_non_trivia_token_after_span(type_span)
                .filter(|token| token.token.ty == TokenType::Comma)
                .ok_or_else(|| FormatError::SyntaxError {
                    message: "expected comma between heritage types",
                })?;

            write!(f, [FormatNodeWithoutTrailingComments(type_id), token(",")])?;
            write!(
                f,
                [format_trailing_comments(
                    enclosing_span,
                    comma_token.span,
                    next_type_start
                )]
            )?;
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [FormatNodeWithoutTrailingComments(type_id)])?;
        }
    }

    Ok(())
}

/// Return the full source span for one interface heritage item.
fn interface_heritage_span(
    context: &DestackFormatContext<'_>,
    heritage: &InterfaceHeritage,
) -> Span {
    context
        .tree
        .get_side_span(
            heritage.expression,
            NodeSpanType::Region(NodeSpanRegion::Type),
        )
        .unwrap_or_else(|| context.span(heritage.expression))
}

/// Write one interface heritage item.
fn write_interface_heritage<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    heritage: &InterfaceHeritage,
) -> FormatResult<()> {
    write!(f, [heritage.expression])?;

    if !heritage.generic_arguments.is_empty() {
        format_generic_argument_list(f, &heritage.generic_arguments)?;
    }

    Ok(())
}

/// Write one interface heritage list.
fn write_interface_heritage_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    enclosing_span: Span,
    heritage_items: &[InterfaceHeritage],
) -> FormatResult<()> {
    for (index, heritage) in heritage_items.iter().enumerate() {
        let next_heritage = heritage_items.get(index + 1);

        if let Some(next_heritage) = next_heritage {
            let heritage_span = interface_heritage_span(f.context(), heritage);
            let next_heritage_start = interface_heritage_span(f.context(), next_heritage).start;
            let comma_token = f
                .context()
                .next_non_trivia_token_after_span(heritage_span)
                .filter(|token| token.token.ty == TokenType::Comma)
                .ok_or_else(|| FormatError::SyntaxError {
                    message: "expected comma between interface heritage items",
                })?;

            write_interface_heritage(f, heritage)?;
            write!(f, [token(",")])?;
            write!(
                f,
                [format_trailing_comments(
                    enclosing_span,
                    comma_token.span,
                    next_heritage_start
                )]
            )?;
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write_interface_heritage(f, heritage)?;
        }
    }

    Ok(())
}

/// Write one declaration member block without its leading separator.
fn write_member_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    // empty body
    if members.is_empty() {
        let node_span = f.context().span(node_id);

        if f.context().comments().has_comment_in_span(node_span) {
            return write!(
                f,
                [
                    token("{"),
                    format_dangling_comments(node_span).with_block_indent(),
                    token("}")
                ]
            );
        }

        return write!(f, [empty_block_with_infix_annotations(node_id)]);
    }

    // member body
    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            format_block_of_members(f, members)
        })))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Write one declaration member body.
fn write_member_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<Member>],
    break_before_body: bool,
) -> FormatResult<()> {
    if break_before_body {
        write!(f, [hard_line_break()])?;
    } else {
        write!(f, [space()])?;
    }

    write_member_block(f, node_id, members)
}

/// Return the opening brace token for one class body.
fn class_body_open_brace_token<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<Member>],
) -> Option<TokenSpan> {
    if let Some(first_member_id) = members.first().copied() {
        let first_member_prefix_start = f
            .context()
            .annotation_ids(first_member_id)
            .iter()
            .copied()
            .map(|annotation_id| f.context().annotation_span(annotation_id).start)
            .next()
            .unwrap_or_else(|| f.context().node_token_start(first_member_id));
        let first_member_start = Span::new(
            f.context().span(first_member_id).file,
            first_member_prefix_start,
            first_member_prefix_start,
        );
        let open_token = f
            .context()
            .previous_non_trivia_token_before_span(first_member_start)?;

        return (open_token.token.ty == TokenType::OpenBrace).then_some(open_token);
    }

    let node_span = f.context().span(node_id);
    let close_token = f.context().last_non_trivia_token_in_span(node_span)?;
    if close_token.token.ty != TokenType::CloseBrace {
        return None;
    }

    let open_token = f
        .context()
        .previous_non_trivia_token_before_span(close_token.span)?;

    (open_token.token.ty == TokenType::OpenBrace).then_some(open_token)
}

/// Write class header comments that appear before the body opening brace.
fn write_class_body_leading_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    let Some(open_token) = class_body_open_brace_token(f, node_id, members) else {
        return Ok(());
    };

    let leading_comments = f
        .context()
        .comments()
        .comments_before(open_token.span.start);
    if leading_comments.iter().any(|comment| !comment.is_line()) {
        write!(f, [FormatLeadingComments::Comments(leading_comments)])?;
    }

    Ok(())
}

/// Write one declaration type-member block without its leading separator.
fn write_type_member_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<TypeMember>],
) -> FormatResult<()> {
    // empty body
    if members.is_empty() {
        let node_span = f.context().span(node_id);

        if f.context().comments().has_comment_in_span(node_span) {
            return write!(
                f,
                [
                    token("{"),
                    format_dangling_comments(node_span).with_block_indent(),
                    token("}")
                ]
            );
        }

        return write!(f, [empty_block_with_infix_annotations(node_id)]);
    }

    // member body
    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            format_type_member_block_list(f, members)
        })))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Return whether one expression is a memberish heritage target without type arguments.
fn expression_is_memberish_without_type_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
    generic_arguments: &[LocalNodeId<GenericArgument>],
) -> bool {
    if !generic_arguments.is_empty() {
        return false;
    }

    let expression_id = transparent_inner_expression(context, expression_id);

    match context.tree.get(expression_id) {
        Expression::QualifiedReference {
            path,
            generic_arguments,
        } => path.segments.len() > 1 && generic_arguments.is_empty(),
        Expression::Member { .. } => true,
        _ => false,
    }
}

/// Return whether one type expression is a qualified heritage target without type arguments.
fn type_is_qualified_without_type_arguments(
    context: &DestackFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(type_id) {
        TypeExpression::Reference {
            path,
            generic_arguments,
        } => path.segments.len() > 1 && generic_arguments.is_empty(),
        TypeExpression::Member {
            generic_arguments, ..
        } => generic_arguments.is_empty(),
        _ => false,
    }
}

/// Return whether one class heritage layout should use group mode.
fn class_heritage_should_group(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    declaration_expression_id: Option<LocalNodeId<Expression>>,
    declaration: &ClassDeclaration,
) -> bool {
    if usize::from(declaration.extends_expression.is_some()) + declaration.implements_types.len()
        > 1
    {
        return true;
    }

    let parent_is_assignment = declaration_expression_id
        .and_then(|expression_id| context.parent(expression_id))
        .is_some_and(|(parent_id, parent_type)| {
            parent_type == NodeType::Expression
                && matches!(
                    context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Assign { .. }
                )
        });

    if ((declaration_expression_id.is_none() || !parent_is_assignment)
        && declaration.extends_expression.is_some_and(|expression_id| {
            expression_is_memberish_without_type_arguments(
                context,
                expression_id,
                &declaration.extends_generic_arguments,
            )
        }))
        || declaration
            .implements_types
            .first()
            .copied()
            .is_some_and(|type_id| type_is_qualified_without_type_arguments(context, type_id))
    {
        return true;
    }

    let name_span = context.tree.get_main_span(node_id);
    let generic_parameters_span = context.tree.get_side_span(
        node_id,
        NodeSpanType::Region(NodeSpanRegion::GenericParameters),
    );
    let extends_span = declaration
        .extends_expression
        .map(|expression_id| context.span(expression_id));
    let extends_generic_arguments_span = declaration.extends_expression.and_then(|expression_id| {
        generic_argument_list_span_after_expression(
            context,
            expression_id,
            &declaration.extends_generic_arguments,
        )
    });
    let implements_span = declaration
        .implements_types
        .first()
        .copied()
        .map(|type_id| context.span(type_id));
    let spans = [
        name_span,
        generic_parameters_span,
        extends_span,
        extends_generic_arguments_span,
        implements_span,
    ];
    let mut spans = spans.into_iter().flatten().peekable();

    while let Some(span) = spans.next() {
        let Some(next_span) = spans.peek().copied() else {
            break;
        };

        if context
            .comments()
            .has_comment_in_range(span.end, next_span.start)
        {
            return true;
        }
    }

    false
}

/// Return class decorators that appear before one export token.
fn class_decorators_before_export(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    export_start: u32,
) -> Vec<LocalNodeId<Decorator>> {
    let mut annotation_ids = Vec::new();

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        let annotation_span = context.annotation_span(annotation_id);

        if annotation_span.end < export_start {
            annotation_ids.push(annotation_id);
        }
    }

    annotation_ids
}

/// Return class decorators that appear after one export token.
fn class_decorators_after_export(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    export_start: u32,
) -> Vec<LocalNodeId<Decorator>> {
    let mut annotation_ids = Vec::new();

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        let annotation_span = context.annotation_span(annotation_id);

        if annotation_span.start > export_start {
            annotation_ids.push(annotation_id);
        }
    }

    annotation_ids
}

/// Return whether one interface heritage layout should use group mode.
fn interface_heritage_should_group(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &InterfaceDeclaration,
) -> bool {
    if declaration.extends.len() > 1 {
        return true;
    }

    if declaration.extends.first().is_some_and(|heritage| {
        expression_is_memberish_without_type_arguments(
            context,
            heritage.expression,
            &heritage.generic_arguments,
        )
    }) {
        return true;
    }

    let previous_span = context
        .tree
        .get_side_span(
            node_id,
            NodeSpanType::Region(NodeSpanRegion::GenericParameters),
        )
        .or(context.tree.get_main_span(node_id));
    let extends_span = declaration
        .extends
        .first()
        .map(|heritage| interface_heritage_span(context, heritage));

    match (previous_span, extends_span) {
        (Some(previous_span), Some(extends_span)) => context
            .comments()
            .has_comment_in_range(previous_span.end, extends_span.start),
        _ => false,
    }
}

/// Format one struct declaration.
pub(crate) fn format_struct_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &StructDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    if declaration.ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    // head
    write!(f, [Keyword::Struct, space(), declaration.name])?;
    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

    // heritage
    format_super_type_clause(f, Keyword::Extends, &declaration.embedded_types)?;
    format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;

    // where clauses
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_member_body(f, node_id, &declaration.members, false)?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

/// Format one class declaration.
pub(crate) fn format_class_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration_expression_id: Option<LocalNodeId<Expression>>,
    declaration: &ClassDeclaration,
) -> FormatResult<()> {
    let heritage_group_mode =
        class_heritage_should_group(f.context(), node_id, declaration_expression_id, declaration);
    let parent_is_assignment = declaration_expression_id
        .and_then(|expression_id| f.context().parent(expression_id))
        .is_some_and(|(parent_id, parent_type)| {
            parent_type == NodeType::Expression
                && matches!(
                    f.context()
                        .tree
                        .get(LocalNodeId::<Expression>::new(parent_id)),
                    Expression::Assign { .. }
                )
        });
    let export_start = declaration_export_token(f.context(), node_id).map(|token| token.span.start);
    let decorators_before_export = export_start.map_or_else(Vec::new, |export_start| {
        class_decorators_before_export(f.context(), node_id, export_start)
    });
    let decorators_after_export = export_start.map_or_else(Vec::new, |export_start| {
        class_decorators_after_export(f.context(), node_id, export_start)
    });
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // decorator and export prefixes
        if declaration.export.is_some() {
            write!(f, [prefix_comments_before_decorators(f.context(), node_id)])?;

            if !decorators_before_export.is_empty() {
                write_vertical_prefix_annotations(f, &decorators_before_export)?;
            }

            format_declaration_export_modifier(f, node_id, declaration.export)?;

            if !decorators_after_export.is_empty() {
                write!(f, [hard_line_break()])?;
                write_vertical_prefix_annotations(f, &decorators_after_export)?;
            }
        } else if f.context().has_annotation(node_id) {
            write!(f, [prefix_comments_before_decorators(f.context(), node_id)])?;
            write!(f, [decorator_prefix_annotations(f.context(), node_id)])?;
        } else {
            write!(f, [prefix_annotations(f.context(), node_id)])?;
        }

        // declaration prefixes
        if declaration.ambient.is_ambient() {
            write!(f, [Keyword::Declare, space()])?;
        }

        if declaration.is_abstract {
            write!(f, [Keyword::Abstract, space()])?;
        }

        // head
        write!(f, [Keyword::Class])?;

        let head = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if let Some(name) = declaration.name {
                write!(f, [space(), name])?;
            }

            write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

            let following_span_start = declaration
                .extends_expression
                .map(|expression_id| f.context().span(expression_id).start)
                .or_else(|| {
                    declaration
                        .implements_types
                        .first()
                        .copied()
                        .map(|type_id| f.context().span(type_id).start)
                });

            if let (Some(name_span), Some(following_span_start)) = (
                f.context().tree.get_main_span(node_id),
                following_span_start,
            ) {
                let generic_parameter_comments = {
                    let comments = f.context().comments();
                    comments
                        .comments_in_range(name_span.end, following_span_start)
                        .to_vec()
                };

                if !generic_parameter_comments.is_empty() {
                    write!(
                        f,
                        [FormatTrailingComments::Comments(
                            &generic_parameter_comments
                        )]
                    )?;
                }
            }

            if let Some(extends_expression) = declaration.extends_expression {
                let comments = f
                    .context()
                    .comments()
                    .comments_before(f.context().span(extends_expression).start);

                if comments.iter().any(|comment| comment.preceded_by_newline()) {
                    write!(
                        f,
                        [indent(&format_with(
                            |f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [FormatTrailingComments::Comments(comments)])
                            }
                        ))]
                    )?;
                }
            }

            Ok(())
        });
        let heritage = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if let Some(extends_expression) = declaration.extends_expression {
                let extends_comments = if !declaration.extends_generic_arguments.is_empty()
                    || !declaration.implements_types.is_empty()
                {
                    Vec::new()
                } else {
                    let body_start = class_body_open_brace_token(f, node_id, &declaration.members)
                        .map_or_else(|| f.context().span(node_id).end, |token| token.span.start);

                    f.context()
                        .comments()
                        .comments_in_range(f.context().span(extends_expression).end, body_start)
                        .to_vec()
                };
                let has_trailing_line_comments =
                    extends_comments.iter().any(|comment| comment.is_line());
                let format_super = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    let enclosing_span = f.context().span(node_id);
                    let type_arguments_span = generic_argument_list_span_after_expression(
                        f.context(),
                        extends_expression,
                        &declaration.extends_generic_arguments,
                    );
                    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        if !declaration.extends_generic_arguments.is_empty() {
                            let Some(type_arguments_span) = type_arguments_span else {
                                unreachable!("extends generic arguments have a source span");
                            };
                            let following_span_start = type_arguments_span.start;

                            write!(
                                f,
                                [format_node_with_trailing_comments(
                                    enclosing_span,
                                    extends_expression,
                                    following_span_start
                                )]
                            )?;
                            format_generic_argument_list(
                                f,
                                &declaration.extends_generic_arguments,
                            )?;
                        } else if declaration.implements_types.is_empty() {
                            write!(f, [FormatNodeWithoutTrailingComments(extends_expression)])?;

                            if !has_trailing_line_comments {
                                write!(f, [FormatTrailingComments::Comments(&extends_comments)])?;
                            }
                        } else {
                            let [first_implements_type, ..] =
                                declaration.implements_types.as_slice()
                            else {
                                unreachable!("implements types are not empty");
                            };
                            let following_span_start =
                                f.context().span(*first_implements_type).start;

                            write!(
                                f,
                                [format_node_with_trailing_comments(
                                    enclosing_span,
                                    extends_expression,
                                    following_span_start
                                )]
                            )?;
                        }

                        Ok(())
                    });

                    if parent_is_assignment {
                        let Some(content) = f.intern(&content)? else {
                            return Ok(());
                        };
                        let flat_content = format_with({
                            let content = content.clone();
                            move |f: &mut DestackFormatter<'ast, '_>| {
                                f.write_node(content.clone());
                                Ok(())
                            }
                        });
                        let expanded_content =
                            format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                                f.write_node(content.clone());
                                Ok(())
                            });

                        write!(
                            f,
                            [group(&format_args![
                                if_group_breaks(&format_args![
                                    token("("),
                                    soft_block_indent(&expanded_content),
                                    token(")")
                                ]),
                                if_group_fits_on_line(&flat_content)
                            ])]
                        )
                    } else {
                        write!(f, [content])
                    }
                });
                let format_extends = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    write!(f, [Keyword::Extends, space(), format_super])
                });

                if heritage_group_mode {
                    write!(f, [soft_line_break_or_space(), group(&format_extends)])?;
                } else {
                    write!(f, [space(), format_extends])?;
                }
            }

            if let Some(first_implements) = declaration.implements_types.first().copied() {
                let leading_comments = f
                    .context()
                    .comments()
                    .comments_before(f.context().span(first_implements).start);

                if usize::from(declaration.extends_expression.is_some())
                    + declaration.implements_types.len()
                    > 1
                {
                    write!(
                        f,
                        [
                            soft_line_break_or_space(),
                            format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                write!(f, [FormatLeadingComments::Comments(leading_comments)])
                            }),
                            (!leading_comments.is_empty()).then_some(hard_line_break()),
                            Keyword::Implements,
                            group(&soft_line_indent_or_space(&format_with(
                                |f: &mut DestackFormatter<'ast, '_>| {
                                    write_heritage_type_list(
                                        f,
                                        f.context().span(node_id),
                                        &declaration.implements_types,
                                    )
                                }
                            )))
                        ]
                    )?;
                } else {
                    let format_implements = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    write!(f, [FormatLeadingComments::Comments(leading_comments)])
                                }),
                                Keyword::Implements,
                                space()
                            ]
                        )?;

                        write_heritage_type_list(
                            f,
                            f.context().span(node_id),
                            &declaration.implements_types,
                        )
                    });

                    if heritage_group_mode {
                        write!(f, [soft_line_break_or_space(), group(&format_implements)])?;
                    } else {
                        write!(f, [space(), format_implements])?;
                    }
                }
            }

            Ok(())
        });

        if heritage_group_mode {
            let indented = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [head, indent(&heritage)])
            });
            let heritage_group_id = f.group_id("heritage");

            write!(
                f,
                [group(&indented).with_id(Some(heritage_group_id)), space()]
            )?;

            if !declaration.members.is_empty() {
                write!(
                    f,
                    [if_group_breaks(&hard_line_break()).with_group_id(Some(heritage_group_id))]
                )?;
            }
        } else {
            write!(f, [head, heritage, space()])?;
        }

        write_declaration_where_clauses(f, &declaration.where_clauses)?;
        if !declaration.where_clauses.is_empty() {
            write!(f, [space()])?;
        }

        write_class_body_leading_comments(f, node_id, &declaration.members)?;
        write_member_block(f, node_id, &declaration.members)
    });

    // decorated class expressions own their grouped parentheses
    let class_expression_needs_parentheses =
        declaration_expression_id.is_some_and(|expression_id| {
            expression_needs_parentheses_in_parent(f.context(), expression_id)
        }) && f.context().has_annotation(node_id);

    if declaration_expression_id.is_some() && class_expression_needs_parentheses {
        write!(f, [soft_block_indent(&content)])?;
    } else {
        write!(f, [group(&content)])?;
    }

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

/// Return the ordered enum body nodes.
fn ordered_enum_body_nodes(
    context: &DestackFormatContext<'_>,
    fields: &[LocalNodeId<EnumField>],
    members: &[LocalNodeId<Member>],
) -> Vec<LocalNodeIdAny> {
    let mut nodes = Vec::with_capacity(fields.len() + members.len());

    // field nodes
    for field_id in fields {
        nodes.push((*field_id).into_any());
    }

    // member nodes
    for member_id in members {
        nodes.push((*member_id).into_any());
    }

    // source order
    nodes.sort_by_key(|node_id| context.span_by_id(node_id.id).start);
    nodes
}

/// Return whether source preserves an empty line between two enum body nodes.
fn enum_body_nodes_have_blank_line_between(
    f: &DestackFormatter<'_, '_>,
    previous_id: LocalNodeIdAny,
    next_id: LocalNodeIdAny,
) -> bool {
    let previous_span = f.context().span_by_id(previous_id.id);
    let next_span = f.context().span_by_id(next_id.id);
    let Some(gap) = previous_span.gap_to(next_span) else {
        return false;
    };

    f.context().has_blank_line(gap)
}

/// Write one enum body.
fn write_enum_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    fields: &[LocalNodeId<EnumField>],
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    let nodes = ordered_enum_body_nodes(f.context(), fields, members);

    // empty body
    if nodes.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        return Ok(());
    }

    // body
    write!(f, [space(), token("{"), hard_line_break()])?;

    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            for (index, node_id) in nodes.iter().copied().enumerate() {
                if index > 0 {
                    let previous_node_id = nodes[index - 1];
                    if previous_node_id.ty != node_id.ty
                        || enum_body_nodes_have_blank_line_between(f, previous_node_id, node_id)
                    {
                        write!(f, [empty_line()])?;
                    } else {
                        write!(f, [hard_line_break()])?;
                    }
                }

                write!(f, [node_id])?;
            }

            Ok(())
        })))]
    )?;

    write!(f, [hard_line_break(), token("}")])
}

/// Format one enum declaration.
pub(crate) fn format_enum_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &EnumDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    if declaration.ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    if declaration.kind == EnumKind::Const {
        write!(f, [Keyword::Const, space()])?;
    }

    // head
    write!(f, [Keyword::Enum])?;
    if let Some(name) = declaration.name {
        write!(f, [space(), name])?;
    }

    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;
    format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_enum_body(f, node_id, &declaration.fields, &declaration.members)?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

/// Format one interface declaration.
pub(crate) fn format_interface_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &InterfaceDeclaration,
) -> FormatResult<()> {
    let heritage_group_mode = interface_heritage_should_group(f.context(), node_id, declaration);
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // prefixes
        format_declaration_export_modifier(f, node_id, declaration.export)?;
        if declaration.ambient.is_ambient() {
            write!(f, [Keyword::Declare, space()])?;
        }

        if declaration.is_nominal {
            write!(f, [Keyword::Newtype, space()])?;
        }

        let head = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [Keyword::Interface])?;
            if let Some(name) = declaration.name {
                write!(f, [space(), name])?;
            }

            write_declaration_generic_parameters(f, &declaration.generic_parameters)
        });

        let heritage = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let Some(first_extends) = declaration.extends.first() else {
                return Ok(());
            };

            let leading_comments = f
                .context()
                .comments()
                .comments_before(interface_heritage_span(f.context(), first_extends).start);

            if declaration.extends.len() > 1 {
                write!(
                    f,
                    [
                        soft_line_break_or_space(),
                        Keyword::Extends,
                        group(&soft_line_indent_or_space(&format_with(
                            |f: &mut DestackFormatter<'ast, '_>| {
                                write_interface_heritage_list(
                                    f,
                                    f.context().span(node_id),
                                    &declaration.extends,
                                )
                            }
                        )))
                    ]
                )
            } else {
                let format_extends = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    if !leading_comments.is_empty() {
                        write!(f, [FormatTrailingComments::Comments(leading_comments)])?;
                    }

                    write!(f, [Keyword::Extends, space()])?;
                    write_interface_heritage(f, first_extends)
                });

                if heritage_group_mode {
                    write!(f, [soft_line_break_or_space(), group(&format_extends)])
                } else {
                    write!(f, [space(), format_extends])
                }
            }
        });

        if heritage_group_mode {
            let indented = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [head, indent(&heritage)])
            });
            let heritage_group_id = f.group_id("heritage");

            write!(
                f,
                [group(&indented).with_id(Some(heritage_group_id)), space()]
            )?;
        } else {
            write!(f, [head, heritage, space()])?;
        }

        write_declaration_where_clauses(f, &declaration.where_clauses)?;
        if !declaration.where_clauses.is_empty() {
            write!(f, [space()])?;
        }

        write_type_member_block(f, node_id, &declaration.members)
    });

    write!(f, [group(&content)])?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        node_id: LocalNodeId<EnumField>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_comments_before_decorators(f.context(), node_id)])?;
        write!(f, [decorator_prefix_annotations(f.context(), node_id)])?;

        // name
        write!(f, [self.name])?;

        // value
        if let Some(value) = self.value {
            write!(f, [space(), token("="), space(), value])?;
        }

        // separator
        write!(f, [token(",")])?;
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}
