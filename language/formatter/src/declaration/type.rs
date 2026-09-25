use crate::annotation::{
    FormatLeadingComments, FormatTrailingComments, decorator_prefix_annotations,
    format_trailing_comments, infix_or_postfix_annotations, postfix_annotations,
    prefix_annotations, prefix_comments_before_decorators, write_vertical_prefix_annotations,
};
use crate::collection::member::format_block_of_members;
use crate::context::{CapturedFormat, FormatNodeWithoutTrailingComments};
use crate::declaration::declaration::{
    declaration_export_token, format_declaration_export_modifier, format_super_type_clause,
    write_declaration_body_separator, write_member_block,
};
use crate::declaration::empty_block_with_infix_annotations;
use crate::declaration::modifier::write_keyword_prefix;
use crate::declaration::signature::{
    write_declaration_generic_parameters, write_declaration_where_clauses,
};
use crate::expression::{expression_needs_parentheses_in_parent, format_type_member_block_list};
use crate::{FormatNode, TsppFormatContext, TsppFormatter};
use smallvec::SmallVec;
use tspp_dir::{
    ClassDeclaration, Declaration, Decorator, EnumDeclaration, EnumField, Expression,
    InterfaceDeclaration, Keyword, LocalNodeId, LocalNodeIdAny, Member, NodeType,
    StructDeclaration, TokenSpan, TokenType, TypeExpression,
};
use tspp_fir::format::{FormatError, FormatResult};
use tspp_fir::prelude::*;
use tspp_fir::{format_args, write};
use tspp_source::{NodeSpanRegion, NodeSpanType, Span};

/// Write one class or interface heritage type list.
fn write_heritage_type_list<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
                .next_token_after_span(type_span)
                .filter(|token| token.token.ty() == TokenType::Comma)
                .ok_or(FormatError::SyntaxError {
                    message: "expected comma between heritage types",
                })?;

            write!(f, [FormatNodeWithoutTrailingComments(type_id), token(",")])?;
            write!(
                f,
                [format_trailing_comments(
                    enclosing_span,
                    comma_token.span,
                    Some(next_type_start)
                )]
            )?;
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [FormatNodeWithoutTrailingComments(type_id)])?;
        }
    }

    Ok(())
}

/// Return the opening brace token for one class body.
fn class_body_open_brace_token<'ast>(
    f: &TsppFormatter<'ast, '_>,
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
        let open_token = f.context().previous_token_before_span(first_member_start)?;

        return (open_token.token.ty() == TokenType::OpenBrace).then_some(open_token);
    }

    let node_span = f.context().span(node_id);
    let close_token = f.context().last_token_in_span(node_span)?;
    if close_token.token.ty() != TokenType::CloseBrace {
        return None;
    }

    let open_token = f.context().previous_token_before_span(close_token.span)?;

    (open_token.token.ty() == TokenType::OpenBrace).then_some(open_token)
}

/// Write class header comments that appear before the body opening brace.
fn write_class_body_leading_comments<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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

/// Return whether one type expression contains generic arguments.
fn type_expression_has_generic_arguments(
    context: &TsppFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(type_id) {
        TypeExpression::Reference {
            generic_arguments, ..
        } => !generic_arguments.is_empty(),
        TypeExpression::Member {
            left,
            generic_arguments,
            ..
        } => !generic_arguments.is_empty() || type_expression_has_generic_arguments(context, *left),
        _ => false,
    }
}

/// Return whether one type expression is a qualified heritage target without type arguments.
fn type_is_qualified_without_type_arguments(
    context: &TsppFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    match context.tree.get(type_id) {
        TypeExpression::Reference {
            path,
            generic_arguments,
        } => path.segments.len() > 1 && generic_arguments.is_empty(),
        TypeExpression::Member {
            left,
            generic_arguments,
            ..
        } => generic_arguments.is_empty() && !type_expression_has_generic_arguments(context, *left),
        _ => false,
    }
}

/// Return whether one class heritage layout should use group mode.
fn class_heritage_should_group(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    declaration_expression_id: Option<LocalNodeId<Expression>>,
    declaration: &ClassDeclaration,
) -> bool {
    if usize::from(declaration.extends_type.is_some()) + declaration.implements_types.len() > 1 {
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
        && declaration
            .extends_type
            .is_some_and(|type_id| type_is_qualified_without_type_arguments(context, type_id)))
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
        .extends_type
        .map(|type_id| context.span(type_id));
    let implements_span = declaration
        .implements_types
        .first()
        .copied()
        .map(|type_id| context.span(type_id));
    let spans = [
        name_span,
        generic_parameters_span,
        extends_span,
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

/// Split class decorators around one export token.
fn split_class_decorators(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    export_start: u32,
) -> (
    SmallVec<[LocalNodeId<Decorator>; 4]>,
    SmallVec<[LocalNodeId<Decorator>; 4]>,
) {
    let mut before = SmallVec::new();
    let mut after = SmallVec::new();

    for annotation_id in context.annotation_ids(node_id).iter().copied() {
        let annotation_span = context.annotation_span(annotation_id);

        if annotation_span.end < export_start {
            before.push(annotation_id);
        } else if annotation_span.start > export_start {
            after.push(annotation_id);
        }
    }

    (before, after)
}

/// Return whether one interface heritage layout should use group mode.
fn interface_heritage_should_group(
    context: &TsppFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &InterfaceDeclaration,
) -> bool {
    if declaration.extends_types.len() > 1 {
        return true;
    }

    if declaration
        .extends_types
        .first()
        .copied()
        .is_some_and(|type_id| type_is_qualified_without_type_arguments(context, type_id))
    {
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
        .extends_types
        .first()
        .copied()
        .map(|type_id| context.span(type_id));

    match (previous_span, extends_span) {
        (Some(previous_span), Some(extends_span)) => context
            .comments()
            .has_comment_in_range(previous_span.end, extends_span.start),
        _ => false,
    }
}

/// Format one struct declaration.
pub(crate) fn format_struct_declaration<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &StructDeclaration,
) -> FormatResult<()> {
    let header = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        // prefixes
        format_declaration_export_modifier(f, node_id, declaration.export)?;
        if declaration.is_ambient {
            write!(f, [Keyword::Declare, space()])?;
        }
        write_keyword_prefix(f, Keyword::Shared, declaration.is_shared)?;

        // head
        write!(f, [Keyword::Struct, space(), declaration.name])?;
        write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

        // heritage
        format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;

        // where clauses
        write_declaration_where_clauses(f, &declaration.where_clauses)
    });
    let header_group_id = f.group_id();
    write!(f, [group(&header).with_id(Some(header_group_id))])?;

    // body
    write_declaration_body_separator(f, header_group_id)?;
    write_member_block(f, node_id, &declaration.members, format_block_of_members)?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

/// Write one class declaration header.
fn write_class_header<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &ClassDeclaration,
    heritage_group_mode: bool,
    parent_is_assignment: bool,
) -> FormatResult<()> {
    // head
    write!(f, [Keyword::Class])?;
    let head = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        if let Some(name) = declaration.name {
            write!(f, [space(), name])?;
        }

        write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

        let following_span_start = declaration
            .extends_type
            .map(|type_id| f.context().span(type_id).start)
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

        if let Some(extends_type) = declaration.extends_type {
            let comments = f
                .context()
                .comments()
                .comments_before(f.context().span(extends_type).start);

            if comments.iter().any(|comment| comment.preceded_by_newline()) {
                write!(
                    f,
                    [indent(&format_with(|f: &mut TsppFormatter<'ast, '_>| {
                        write!(f, [FormatTrailingComments::Comments(comments)])
                    }))]
                )?;
            }
        }

        Ok(())
    });
    let heritage = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        if let Some(extends_type) = declaration.extends_type {
            let extends_comments = if !declaration.implements_types.is_empty() {
                Vec::new()
            } else {
                let body_start = class_body_open_brace_token(f, node_id, &declaration.members)
                    .map_or_else(|| f.context().span(node_id).end, |token| token.span.start);

                f.context()
                    .comments()
                    .comments_in_range(f.context().span(extends_type).end, body_start)
                    .to_vec()
            };
            let has_trailing_line_comments =
                extends_comments.iter().any(|comment| comment.is_line());
            let format_super = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                let content = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                    if declaration.implements_types.is_empty() {
                        write!(f, [FormatNodeWithoutTrailingComments(extends_type)])?;

                        if !has_trailing_line_comments {
                            write!(f, [FormatTrailingComments::Comments(&extends_comments)])?;
                        }
                    } else {
                        let first_implements_type = declaration.implements_types.first().ok_or(
                            FormatError::SyntaxError {
                                message: "implements clause requires at least one type",
                            },
                        )?;
                        let following_span_start = f.context().span(*first_implements_type).start;

                        write!(f, [FormatNodeWithoutTrailingComments(extends_type)])?;
                        write!(
                            f,
                            [format_trailing_comments(
                                f.context().span(node_id),
                                f.context().span(extends_type),
                                Some(following_span_start),
                            )]
                        )?;
                    }

                    Ok(())
                });

                if parent_is_assignment {
                    let content = CapturedFormat::new(f, content)?;

                    write!(
                        f,
                        [group(&format_args![
                            if_group_breaks(&format_args![
                                token("("),
                                soft_block_indent(&content),
                                token(")")
                            ]),
                            if_group_fits_on_line(&content)
                        ])]
                    )
                } else {
                    write!(f, [content])
                }
            });
            let format_extends = format_with(|f: &mut TsppFormatter<'ast, '_>| {
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

            if usize::from(declaration.extends_type.is_some()) + declaration.implements_types.len()
                > 1
            {
                write!(
                    f,
                    [
                        soft_line_break_or_space(),
                        format_with(|f: &mut TsppFormatter<'ast, '_>| {
                            write!(f, [FormatLeadingComments::Comments(leading_comments)])
                        }),
                        (!leading_comments.is_empty()).then_some(hard_line_break()),
                        Keyword::Implements,
                        group(&soft_line_indent_or_space(&format_with(
                            |f: &mut TsppFormatter<'ast, '_>| {
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
                let format_implements = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                    write!(
                        f,
                        [
                            format_with(|f: &mut TsppFormatter<'ast, '_>| {
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

    // heritage
    if heritage_group_mode {
        write!(f, [head, indent(&heritage)])?;
    } else {
        write!(f, [head, heritage])?;
    }

    // where clauses
    write_declaration_where_clauses(f, &declaration.where_clauses)
}

/// Format one class declaration.
pub(crate) fn format_class_declaration<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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

    // split decorators around export
    let export_start = declaration_export_token(f.context(), node_id).map(|token| token.span.start);
    let (decorators_before_export, decorators_after_export) =
        if let Some(export_start) = export_start {
            split_class_decorators(f.context(), node_id, export_start)
        } else {
            (SmallVec::new(), SmallVec::new())
        };
    let content = format_with(|f: &mut TsppFormatter<'ast, '_>| {
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
        if declaration.is_ambient {
            write!(f, [Keyword::Declare, space()])?;
        }
        write_keyword_prefix(f, Keyword::Shared, declaration.is_shared)?;

        if declaration.is_abstract {
            write!(f, [Keyword::Abstract, space()])?;
        }
        if declaration.is_final {
            write!(f, [Keyword::Final, space()])?;
        }

        let header = format_with(|f: &mut TsppFormatter<'ast, '_>| {
            write_class_header(
                f,
                node_id,
                declaration,
                heritage_group_mode,
                parent_is_assignment,
            )
        });
        let header_group_id = f.group_id();
        write!(f, [group(&header).with_id(Some(header_group_id))])?;

        // body
        write_class_body_leading_comments(f, node_id, &declaration.members)?;
        write_declaration_body_separator(f, header_group_id)?;
        write_member_block(f, node_id, &declaration.members, format_block_of_members)
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
    context: &TsppFormatContext<'_>,
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
    f: &TsppFormatter<'_, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    fields: &[LocalNodeId<EnumField>],
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    let nodes = ordered_enum_body_nodes(f.context(), fields, members);

    // empty body
    if nodes.is_empty() {
        return write!(f, [empty_block_with_infix_annotations(node_id)]);
    }

    // body
    write!(f, [token("{"), hard_line_break()])?;

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
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &EnumDeclaration,
) -> FormatResult<()> {
    let header = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        // prefixes
        format_declaration_export_modifier(f, node_id, declaration.export)?;
        if declaration.is_ambient {
            write!(f, [Keyword::Declare, space()])?;
        }
        write_keyword_prefix(f, Keyword::Shared, declaration.is_shared)?;

        // head
        write!(f, [Keyword::Enum])?;
        if let Some(name) = declaration.name {
            write!(f, [space(), name])?;
        }

        write_declaration_generic_parameters(f, &declaration.generic_parameters)?;
        format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;
        write_declaration_where_clauses(f, &declaration.where_clauses)
    });
    let header_group_id = f.group_id();
    write!(f, [group(&header).with_id(Some(header_group_id))])?;

    // body
    write_declaration_body_separator(f, header_group_id)?;
    write_enum_body(f, node_id, &declaration.fields, &declaration.members)?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

/// Format one interface declaration.
pub(crate) fn format_interface_declaration<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &InterfaceDeclaration,
) -> FormatResult<()> {
    let heritage_group_mode = interface_heritage_should_group(f.context(), node_id, declaration);
    let content = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        // prefixes
        format_declaration_export_modifier(f, node_id, declaration.export)?;
        if declaration.is_ambient {
            write!(f, [Keyword::Declare, space()])?;
        }

        write_keyword_prefix(f, Keyword::Shared, declaration.is_shared)?;

        if declaration.is_nominal {
            write!(f, [Keyword::Newtype, space()])?;
        }

        let header = format_with(|f: &mut TsppFormatter<'ast, '_>| {
            // head
            let head = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                write!(f, [Keyword::Interface])?;
                if let Some(name) = declaration.name {
                    write!(f, [space(), name])?;
                }

                write_declaration_generic_parameters(f, &declaration.generic_parameters)
            });

            let heritage = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                let Some(first_extends) = declaration.extends_types.first().copied() else {
                    return Ok(());
                };

                let leading_comments = f
                    .context()
                    .comments()
                    .comments_before(f.context().span(first_extends).start);

                if declaration.extends_types.len() > 1 {
                    write!(
                        f,
                        [
                            soft_line_break_or_space(),
                            Keyword::Extends,
                            group(&soft_line_indent_or_space(&format_with(
                                |f: &mut TsppFormatter<'ast, '_>| {
                                    write_heritage_type_list(
                                        f,
                                        f.context().span(node_id),
                                        &declaration.extends_types,
                                    )
                                }
                            )))
                        ]
                    )
                } else {
                    let format_extends = format_with(|f: &mut TsppFormatter<'ast, '_>| {
                        if !leading_comments.is_empty() {
                            write!(f, [FormatTrailingComments::Comments(leading_comments)])?;
                        }

                        write!(f, [Keyword::Extends, space()])?;
                        write!(f, [first_extends])
                    });

                    if heritage_group_mode {
                        write!(f, [soft_line_break_or_space(), group(&format_extends)])
                    } else {
                        write!(f, [space(), format_extends])
                    }
                }
            });

            // heritage
            if heritage_group_mode {
                write!(f, [head, indent(&heritage)])?;
            } else {
                write!(f, [head, heritage])?;
            }

            // where clauses
            write_declaration_where_clauses(f, &declaration.where_clauses)
        });
        let header_group_id = f.group_id();
        write!(f, [group(&header).with_id(Some(header_group_id))])?;

        // body
        write_declaration_body_separator(f, header_group_id)?;
        write_member_block(
            f,
            node_id,
            &declaration.members,
            format_type_member_block_list,
        )
    });

    write!(f, [group(&content)])?;

    // postfix annotations
    write!(f, [postfix_annotations(f.context(), node_id)])
}

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        node_id: LocalNodeId<EnumField>,
        f: &mut TsppFormatter<'ast, '_>,
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
