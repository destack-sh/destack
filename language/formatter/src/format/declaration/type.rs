use crate::format::annotation::{
    FormatLeadingComments, FormatTrailingComments, infix_or_postfix_annotations,
    postfix_annotations, prefix_annotations,
};
use crate::format::chain::transparent_inner_expression;
use crate::format::collection::member::format_block_of_members;
use crate::format::declaration::declaration::{
    format_declaration_export_modifier, format_super_type_clause,
};
use crate::format::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    write_generic_parameter_list,
};
use crate::format::expression::{format_expression, format_type_member_list};
use crate::format::operator::format_generic_argument_list;
use crate::{
    DestackFormatContext, DestackFormatter, FormatNode, empty_block_with_infix_annotations,
};
use destack_ast::{
    ClassDeclaration, Declaration, EnumDeclaration, EnumField, EnumKind, Expression,
    GenericArgument, InterfaceDeclaration, Keyword, LocalNodeId, LocalNodeIdAny, Member, Node,
    NodeTree, NodeTreeImpl, NodeType, StructDeclaration, TypeExpression, TypeMember,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;

/// Write one declaration generic parameter list.
fn write_declaration_generic_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<destack_ast::GenericParameter>],
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
    where_clauses: &[LocalNodeId<destack_ast::WhereClause>],
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
    NodeTree: NodeTreeImpl<T>,
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

    if open_token.token.ty != destack_ast::TokenType::LessThan {
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

/// Write one declaration member block without its leading separator.
fn write_member_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    // empty body
    if members.is_empty() {
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

/// Write one declaration type-member block without its leading separator.
fn write_type_member_block<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<TypeMember>],
) -> FormatResult<()> {
    // empty body
    if members.is_empty() {
        return write!(f, [empty_block_with_infix_annotations(node_id)]);
    }

    // member body
    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            format_type_member_list(f, members)
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
    let generic_parameters_span = combined_node_span(context, &declaration.generic_parameters);
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

/// Return whether one interface heritage layout should use group mode.
fn interface_heritage_should_group(
    context: &DestackFormatContext<'_>,
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

    let previous_span = combined_node_span(context, &declaration.generic_parameters)
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
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // prefixes
        format_declaration_export_modifier(f, node_id, declaration.export)?;
        if declaration.ambient.is_ambient() {
            write!(f, [Keyword::Declare, space()])?;
        }

        if declaration.is_abstract {
            write!(f, [Keyword::Abstract, space()])?;
        }

        // head
        write!(f, [Keyword::Class])?;
        if let Some(name) = declaration.name {
            write!(f, [space(), name])?;
        }

        write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

        let head = format_with(|f: &mut DestackFormatter<'ast, '_>| {
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
                let extends_expression =
                    transparent_inner_expression(f.context(), extends_expression);
                let format_super = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        let extends_expression_node = f.context().tree.get(extends_expression);
                        format_expression(f, extends_expression, extends_expression_node, false)?;

                        if !declaration.extends_generic_arguments.is_empty() {
                            format_generic_argument_list(
                                f,
                                &declaration.extends_generic_arguments,
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
                                    for (index, type_id) in
                                        declaration.implements_types.iter().copied().enumerate()
                                    {
                                        if index > 0 {
                                            write!(f, [token(","), soft_line_break_or_space()])?;
                                        }

                                        write!(f, [type_id])?;
                                    }

                                    Ok(())
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

                        for (index, type_id) in
                            declaration.implements_types.iter().copied().enumerate()
                        {
                            if index > 0 {
                                write!(f, [token(","), soft_line_break_or_space()])?;
                            }

                            write!(f, [type_id])?;
                        }

                        Ok(())
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

        write_member_block(f, node_id, &declaration.members)
    });

    write!(f, [group(&content)])?;

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
                    write!(f, [hard_line_break()])?;
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

        // head
        write!(f, [Keyword::Interface])?;
        if let Some(name) = declaration.name {
            write!(f, [space(), name])?;
        }

        write_declaration_generic_parameters(f, &declaration.generic_parameters)?;
        let heritage = format_with(|f: &mut DestackFormatter<'ast, '_>| {
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
                            |f: &mut DestackFormatter<'ast, '_>| {
                                for (index, type_id) in
                                    declaration.extends_types.iter().copied().enumerate()
                                {
                                    if index > 0 {
                                        write!(f, [token(","), soft_line_break_or_space()])?;
                                    }

                                    write!(f, [type_id])?;
                                }

                                Ok(())
                            }
                        )))
                    ]
                )
            } else {
                let format_extends = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    if !leading_comments.is_empty() {
                        write!(f, [FormatTrailingComments::Comments(leading_comments)])?;
                    }

                    write!(f, [Keyword::Extends, space(), first_extends])
                });

                if heritage_group_mode {
                    write!(f, [soft_line_break_or_space(), group(&format_extends)])
                } else {
                    write!(f, [space(), format_extends])
                }
            }
        });

        if heritage_group_mode {
            let indented =
                format_with(|f: &mut DestackFormatter<'ast, '_>| write!(f, [indent(&heritage)]));
            let heritage_group_id = f.group_id("heritage");

            write!(
                f,
                [group(&indented).with_id(Some(heritage_group_id)), space()]
            )?;
        } else {
            write!(f, [heritage, space()])?;
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
        write!(f, [prefix_annotations(f.context(), node_id)])?;

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
