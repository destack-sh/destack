use crate::format::chain::transparent_inner_expression;
use crate::format::collection::list_like;
use crate::format::collection::property::{format_block_of_members, format_key_with_quotes};
use crate::format::declaration::signature::format_where_clause_with_break;
use crate::format::declaration::statement_list::format_block_of_statements;
use crate::format::expression::{
    expression_has_static_type_arguments, should_drop_parenthesized_expression_wrapper,
    type_index_left_requires_parentheses,
};
use crate::format::operator::{
    flatten_type_binary_expression, format_leading_pipe_union_with_external_prefix,
    is_type_context, needs_parens_in_postfix_position,
};
use crate::{
    Annotation, DestackFormatContext, DestackFormatter, FormatNode,
    empty_block_with_infix_annotations,
};
use destack_ast::{
    AnnotationPosition, BinaryOperator, Declaration, DeclarationDescriptor, DeclarationKind,
    DependencyKind, DependencyMode, Expression, FunctionKind, Generics, Heritage, IfKind,
    ImportAliasTarget, Key, Keyword, LocalNodeId, Member, Mutability, Name, NamespaceKind,
    NodeType, Parameter, TokenType, TypeKind, Visibility,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

use crate::format::declaration::function::format_function_declaration;
use crate::format::declaration::r#type::{
    format_enum_declaration, format_interface_declaration, format_struct_or_class_declaration,
};

const TEMPLATE_LITERAL_TYPE_EQUALS_BREAK_WIDTH: u16 = 80;

/// Normalize one TypeScript type-alias value through transparent grouping wrappers.
///
/// This mirrors oxc and Prettier single-member union transparency in TS fixtures.
fn normalize_typescript_type_alias_value_expression(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> (LocalNodeId<Expression>, Vec<LocalNodeId<Expression>>) {
    if context.options.language_type.is_destack() {
        return (value_id, Vec::new());
    }

    let mut current_id = value_id;
    let mut transparent_wrapper_prefix_annotation_owner_ids = Vec::new();
    loop {
        match context.tree.get(current_id) {
            Expression::Statement(inner_expression_id) => {
                if context.has_prefix_annotation(current_id) {
                    transparent_wrapper_prefix_annotation_owner_ids.push(current_id);
                }
                current_id = *inner_expression_id;
                continue;
            }
            Expression::Parenthesized {
                expression: inner_expression_id,
            } => {
                if context.has_prefix_annotation(current_id) {
                    transparent_wrapper_prefix_annotation_owner_ids.push(current_id);
                }
                current_id = *inner_expression_id;
                continue;
            }
            Expression::Binary { operator, .. }
                if is_type_context(context, current_id)
                    && matches!(
                        operator,
                        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                    ) =>
            {
                let operands = flatten_type_binary_expression(context, current_id, *operator);
                if operands.len() == 1 {
                    if context.has_prefix_annotation(current_id) {
                        transparent_wrapper_prefix_annotation_owner_ids.push(current_id);
                    }
                    current_id = operands[0].1;
                    continue;
                }
            }
            _ => {}
        }

        break;
    }

    (current_id, transparent_wrapper_prefix_annotation_owner_ids)
}

/// Return whether one union value ends with an own-line doc prefix annotation.
fn union_has_trailing_own_line_doc_prefix_annotation(
    context: &DestackFormatContext<'_>,
    value_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(value_id) else {
        return false;
    };

    annotation_ids
        .into_iter()
        .rev()
        .find(|annotation_id| {
            matches!(
                context.annotation(*annotation_id).position(),
                AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
            )
        })
        .is_some_and(|annotation_id| {
            matches!(context.annotation(annotation_id), Annotation::Doc { .. })
                && context.annotation_starts_on_own_line(annotation_id)
                && !context.annotation_next_token_is_on_same_line(annotation_id)
        })
}

/// Return whether one expression has an own-line prefix annotation.
fn expression_has_own_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_has_own_line_prefix_annotation = context
        .visit_annotations(expression_id, |annotations| {
            annotations.iter().copied().any(|annotation_id| {
                matches!(
                    context.annotation(annotation_id).position(),
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) && context.annotation_starts_on_own_line(annotation_id)
            })
        })
        .unwrap_or(false);
    if expression_has_own_line_prefix_annotation {
        return true;
    }

    let expression_id = transparent_inner_expression(context, expression_id);
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };

    context
        .visit_annotations(*declaration_id, |annotations| {
            annotations.iter().copied().any(|annotation_id| {
                matches!(
                    context.annotation(annotation_id).position(),
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) && context.annotation_starts_on_own_line(annotation_id)
            })
        })
        .unwrap_or(false)
}

/// Return whether one type-alias value is a union or intersection through transparent wrappers.
fn type_alias_value_is_type_binary_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;
    loop {
        match context.tree.get(current_id) {
            Expression::Statement(inner_expression_id) => {
                current_id = *inner_expression_id;
            }
            Expression::Parenthesized {
                expression: inner_expression_id,
            } => {
                current_id = *inner_expression_id;
            }
            Expression::Binary { operator, .. } => {
                return is_type_context(context, current_id)
                    && matches!(
                        operator,
                        BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd
                    );
            }
            _ => return false,
        }
    }
}

/// Build annotation facts for one transparent wrapper owner.
fn transparent_wrapper_annotation_facts_for_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> (bool, bool, bool) {
    let mut has_block_prefix_annotation = false;
    let mut has_own_line_prefix_annotation = false;
    let mut has_own_line_pipe_prefix_annotation = false;

    context
        .visit_annotations(expression_id, |annotations| {
            for annotation_id in annotations.iter().copied() {
                let position = context.annotation(annotation_id).position();
                let starts_on_own_line = context.annotation_starts_on_own_line(annotation_id);

                if position == AnnotationPosition::BlockPrefix {
                    has_block_prefix_annotation = true;
                }

                if matches!(
                    position,
                    AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
                ) && starts_on_own_line
                {
                    has_own_line_prefix_annotation = true;
                }

                if position == AnnotationPosition::LinePrefix
                    && starts_on_own_line
                    && context.annotation_next_token_is_on_same_line(annotation_id)
                    && context.annotation_next_non_whitespace_token_type(annotation_id)
                        == Some(TokenType::ElementwiseOr)
                {
                    has_own_line_pipe_prefix_annotation = true;
                }

                if has_block_prefix_annotation
                    && has_own_line_prefix_annotation
                    && has_own_line_pipe_prefix_annotation
                {
                    break;
                }
            }
        })
        .unwrap_or(());

    (
        has_block_prefix_annotation,
        has_own_line_prefix_annotation,
        has_own_line_pipe_prefix_annotation,
    )
}

/// Build annotation facts for transparent wrapper owners around one type-alias value.
fn collect_transparent_wrapper_annotation_facts(
    context: &DestackFormatContext<'_>,
    expression_ids: &[LocalNodeId<Expression>],
) -> (bool, bool, bool) {
    let mut has_block_prefix_annotation = false;
    let mut has_own_line_prefix_annotation = false;
    let mut has_own_line_pipe_prefix_annotation = false;

    for expression_id in expression_ids.iter().copied() {
        let (
            expression_has_block_prefix_annotation,
            expression_has_own_line_prefix_annotation,
            expression_has_own_line_pipe_prefix_annotation,
        ) = transparent_wrapper_annotation_facts_for_expression(context, expression_id);
        has_block_prefix_annotation |= expression_has_block_prefix_annotation;
        has_own_line_prefix_annotation |= expression_has_own_line_prefix_annotation;
        has_own_line_pipe_prefix_annotation |= expression_has_own_line_pipe_prefix_annotation;

        if has_block_prefix_annotation
            && has_own_line_prefix_annotation
            && has_own_line_pipe_prefix_annotation
        {
            break;
        }
    }

    (
        has_block_prefix_annotation,
        has_own_line_prefix_annotation,
        has_own_line_pipe_prefix_annotation,
    )
}

/// Format one declaration export modifier and export-head seam comments.
pub(crate) fn format_declaration_export_modifier<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &destack_ast::DeclarationDescriptor,
) -> FormatResult<()> {
    if let Some(export) = descriptor.export {
        write!(
            f,
            [export, space(), f.context().any_prefix_annotations(node_id)]
        )?;
    }

    Ok(())
}

/// Format a super type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    format_super_type_clause_with_expand(f, keyword, types, false, false)
}

/// Format a super type clause and optionally force line breaking.
pub(crate) fn format_super_type_clause_with_expand<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<Expression>],
    force_expand: bool,
    start_on_new_line: bool,
) -> FormatResult<()> {
    assert!(!types.is_empty());
    let should_group_clause = !start_on_new_line
        && !force_expand
        && super_type_clause_prefers_group_mode(f.context(), types);

    let format_clause = format_with(|f| {
        if start_on_new_line {
            write!(f, [hard_line_break()])?;
        } else if force_expand || should_group_clause {
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [space()])?;
        }

        write!(f, [keyword, space()])?;

        let format_types = types
            .iter()
            .copied()
            .map(|type_id| format_with(move |f| format_super_type_expression(f, keyword, type_id)));
        if start_on_new_line && !force_expand {
            f.join_with(&format_args![&token(","), space()])
                .entries(format_types)
                .finish()
        } else {
            f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                .entries(format_types)
                .finish()
        }
    });

    if start_on_new_line || force_expand || should_group_clause {
        write!(
            f,
            [group(&indent(&format_clause)).should_expand(force_expand)]
        )
    } else {
        write!(f, [group(&format_clause)])
    }
}

/// Return whether one super-type clause should use grouped head layout.
fn super_type_clause_prefers_group_mode(
    context: &DestackFormatContext<'_>,
    types: &[LocalNodeId<Expression>],
) -> bool {
    if types.len() > 1 {
        return true;
    }

    let Some(type_id) = types.first().copied() else {
        return false;
    };
    if expression_has_static_type_arguments(context, type_id) {
        return false;
    }

    let type_id = match context.tree.get(type_id) {
        Expression::Statement(inner_type_id)
        | Expression::Parenthesized {
            expression: inner_type_id,
        } => *inner_type_id,
        _ => type_id,
    };

    matches!(
        context.tree.get(type_id),
        Expression::Path { path, .. } if path.segments.len() > 1
    ) || matches!(
        context.tree.get(type_id),
        Expression::Member { .. } | Expression::PrivateMember { .. }
    )
}

/// Format one super type expression.
fn format_super_type_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    expression_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let should_wrap_class_extends_head = keyword == Keyword::Extends
        && class_extends_expression_requires_parenthesized_head(f.context(), expression_id);
    if should_wrap_class_extends_head {
        write!(f, [token("("), expression_id, token(")")])?;
    } else {
        write!(f, [expression_id])?;
    }

    Ok(())
}

/// Return whether one class extends head requires explicit parenthesized grouping.
fn class_extends_expression_requires_parenthesized_head(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(expression_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Class { heritage, .. } = context.tree.get(declaration_id) else {
        return false;
    };
    let Some(extends_types) = heritage.extends_types.as_ref() else {
        return false;
    };
    if !extends_types.contains(&expression_id) {
        return false;
    }

    super_type_has_invalid_unparenthesized_head(context.tree, expression_id)
}

/// Return whether one class heritage expression starts with an invalid unparenthesized head.
fn super_type_has_invalid_unparenthesized_head(
    tree: &destack_ast::NodeTree,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    // unparenthesized lambdas are not valid in class heritage heads
    if matches!(
        tree.get(expression_id),
        Expression::Declaration(declaration_id)
            if matches!(
                tree.get(*declaration_id),
                Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
            )
    ) {
        return true;
    }

    // these forms require explicit parentheses in class heritage expressions
    matches!(
        tree.get(expression_id),
        Expression::ObjectExpression { .. }
            | Expression::Unary { .. }
            | Expression::Binary { .. }
            | Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::If { .. }
            | Expression::Assign { .. }
            | Expression::SequenceExpression { .. }
    )
}

/// Format a global augmentation declaration.
pub(crate) fn format_global_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // keyword
    write!(f, [token("global")])?;

    // body
    write!(f, [space()])?;
    if expressions.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
    } else {
        write!(f, [token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&block_indent(&format_with(|f| {
                format_block_of_statements(f, expressions, false)
            })))]
        )?;
        write!(
            f,
            [f.context().block_infix_annotations(node_id), token("}")]
        )?;
    }

    Ok(())
}

/// Format a namespace declaration.
pub(crate) fn format_namespace_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: NamespaceKind,
    generics: &Generics,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // keyword
    if kind == NamespaceKind::Module {
        write!(f, [token("module")])?;
    } else {
        write!(f, [Keyword::Namespace])?;
    }

    // name / key
    if let Some(name) = descriptor.name {
        write!(f, [space()])?;
        if matches!(name, Name::String(_)) {
            format_key_with_quotes(f, Key::Name(name), true)?;
        } else {
            write!(f, [name])?;
        }
    }

    // where
    if let Some(where_clauses) = generics.where_clauses.as_ref()
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    // body
    write!(f, [space()])?;
    if expressions.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
    } else {
        write!(f, [token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&block_indent(&format_with(|f| {
                format_block_of_statements(f, expressions, false)
            })))]
        )?;
        write!(
            f,
            [
                hard_line_break(),
                f.context().block_infix_annotations(node_id),
                token("}")
            ]
        )?;
    }

    Ok(())
}

/// Format an import alias declaration.
pub(crate) fn format_import_alias_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: DependencyKind,
    target: &ImportAliasTarget,
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // keyword
    write!(f, [Keyword::Import])?;
    if kind == DependencyKind::Type {
        write!(f, [space(), Keyword::Type])?;
    }

    // name
    if let Some(name) = descriptor.name {
        write!(f, [space(), name])?;
    }

    // target
    write!(f, [space(), token("="), space()])?;
    match target {
        ImportAliasTarget::Require { target } => {
            write!(
                f,
                [
                    token("require"),
                    token("("),
                    token("\""),
                    target,
                    token("\""),
                    token(")")
                ]
            )?;
        }
        ImportAliasTarget::Path { value } => {
            write!(f, [*value])?;
        }
    }

    write!(f, [token(";")])?;
    write!(f, [f.context().line_postfix_boundary_annotations(node_id)])?;
    Ok(())
}

/// Format an extension declaration.
pub(crate) fn format_extension_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    generics: &Generics,
    target_type: LocalNodeId<Expression>,
    heritage: &Heritage,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // keyword
    write!(f, [Keyword::Extension])?;

    // named extensions: `extension Name<T> of Target`
    // anonymous extensions: `extension<T> of Target`
    if let Some(name) = descriptor.name {
        // named: name first, then generics
        write!(f, [space(), name])?;

        if let Some(static_arguments) = generics.static_parameters.as_ref()
            && !static_arguments.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("<"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                            .entries(static_arguments)
                            .finish()
                    })),
                    token(">")
                ])]
            )?;
        }
    } else {
        // anonymous: generics first, no name
        if let Some(static_arguments) = generics.static_parameters.as_ref()
            && !static_arguments.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("<"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                            .entries(static_arguments)
                            .finish()
                    })),
                    token(">")
                ])]
            )?;
        }
    }

    // for keyword + target type
    write!(f, [space(), Keyword::For, space(), target_type])?;

    // implements types
    if let Some(implements_types) = heritage.implements_types.as_ref()
        && !implements_types.is_empty()
    {
        format_super_type_clause(f, Keyword::Implements, implements_types)?;
    }

    // where
    if let Some(where_clauses) = generics.where_clauses.as_ref()
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    // body
    if members.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        return Ok(());
    }

    // body
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            format_block_of_members(f, members)
        })),])]
    )?;
    write!(f, [hard_line_break(), token("}")])?;

    Ok(())
}

/// Format a type alias declaration.
pub(crate) fn format_type_alias_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: TypeKind,
    mutability: Option<Mutability>,
    static_parameters: &Option<Vec<LocalNodeId<Parameter>>>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let (value_id, transparent_wrapper_prefix_annotation_owner_ids) =
        normalize_typescript_type_alias_value_expression(f.context(), value_id);
    let (
        transparent_wrapper_has_block_prefix_annotation,
        transparent_wrapper_has_own_line_prefix_annotation,
        transparent_wrapper_has_own_line_pipe_prefix_annotation,
    ) = collect_transparent_wrapper_annotation_facts(
        f.context(),
        transparent_wrapper_prefix_annotation_owner_ids.as_slice(),
    );
    let value_expression = f.context().tree.get(value_id);
    let value_is_type_binary = type_alias_value_is_type_binary_expression(f.context(), value_id);
    let value_is_type_union = matches!(
        value_expression,
        Expression::Binary {
            operator: BinaryOperator::ElementwiseOr,
            ..
        }
    ) && is_type_context(f.context(), value_id);
    let transparent_wrapper_has_own_line_pipe_prefix_annotation = value_is_type_union
        && transparent_wrapper_has_block_prefix_annotation
        && transparent_wrapper_has_own_line_pipe_prefix_annotation;

    let header = format_with(|f| {
        // export
        format_declaration_export_modifier(f, node_id, descriptor)?;

        // kind
        if descriptor.kind == DeclarationKind::Declaration {
            write!(f, [Keyword::Declare, space()])?;
        }

        // keyword
        if mutability == Some(Mutability::Immutable) {
            // for readonly type expression
            write!(f, [Keyword::Readonly])?;
        } else if kind == TypeKind::Structural {
            write!(f, [Keyword::Type])?;
        } else {
            write!(f, [Keyword::Newtype])?;
        }

        // name
        if let Some(name) = descriptor.name {
            write!(f, [space(), name])?;
        }

        // static parameters
        if let Some(static_parameters) = static_parameters {
            write!(f, [list_like("<", ">", ",", static_parameters)])?;
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
        }

        Ok(())
    });

    let tree = f.context().tree;
    let format_value = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if !transparent_wrapper_prefix_annotation_owner_ids.is_empty() {
            let format_transparent_wrapper_prefix =
                format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    for expression_id in transparent_wrapper_prefix_annotation_owner_ids
                        .iter()
                        .copied()
                    {
                        write!(f, [f.context().any_prefix_annotations(expression_id)])?;
                    }

                    Ok(())
                });

            if value_is_type_binary {
                write!(f, [indent(&format_transparent_wrapper_prefix)])?;
            } else {
                write!(f, [format_transparent_wrapper_prefix])?;
            }
        }

        if transparent_wrapper_has_own_line_pipe_prefix_annotation {
            let operands = flatten_type_binary_expression(
                f.context(),
                value_id,
                BinaryOperator::ElementwiseOr,
            );
            return format_leading_pipe_union_with_external_prefix(
                f, value_id, &operands, false, true,
            );
        }

        write!(f, [value_id])
    });

    // prefer keeping the value on a single line
    let format_inline = format_with(|f| {
        write!(f, [header, space(), token("=")])?;
        write!(f, [space()])?;
        write!(f, [format_value])?;
        Ok(())
    });

    let format_soft_break = format_with(|f| {
        if value_is_type_binary {
            write!(
                f,
                [group(&format_args![
                    header,
                    space(),
                    token("="),
                    soft_line_break_or_space(),
                    format_value
                ])]
            )
        } else {
            write!(
                f,
                [group(&format_args![
                    header,
                    space(),
                    token("="),
                    indent(&format_args![soft_line_break_or_space(), format_value])
                ])]
            )
        }
    });
    let line_width = f.context().options.line_width;
    let should_break_template_literal_type_after_equals = match value_expression {
        Expression::TypeTemplateLiteral { spans, .. } => {
            line_width <= TEMPLATE_LITERAL_TYPE_EQUALS_BREAK_WIDTH
                && spans.iter().any(|span_id| {
                    matches!(
                        tree.get(*span_id),
                        Expression::TypeConditional { .. }
                            | Expression::If {
                                kind: IfKind::Ternary,
                                ..
                            }
                    )
                })
        }
        _ => false,
    };
    let should_break_after_equals = match value_expression {
        Expression::TypeConditional { left, .. } => {
            !matches!(tree.get(*left), Expression::Parenthesized { .. })
        }
        _ => false,
    };
    let value_has_own_line_prefix_annotation =
        expression_has_own_line_prefix_annotation(f.context(), value_id)
            || transparent_wrapper_has_own_line_prefix_annotation;
    let should_break_for_prefix_annotation = if value_is_type_binary {
        transparent_wrapper_has_own_line_pipe_prefix_annotation
    } else {
        value_has_own_line_prefix_annotation
    };
    let should_break_for_union_trailing_doc_prefix_annotation = value_is_type_union
        && union_has_trailing_own_line_doc_prefix_annotation(f.context(), value_id);
    let value_prefers_inline_after_equals = match value_expression {
        Expression::Parenthesized { expression } => {
            !should_drop_parenthesized_expression_wrapper(f.context(), value_id, *expression)
        }
        Expression::Index { left, .. } => {
            matches!(tree.get(*left), Expression::Parenthesized { .. })
                || needs_parens_in_postfix_position(tree, *left)
        }
        Expression::TypeIndex { left, .. } => {
            matches!(tree.get(*left), Expression::Parenthesized { .. })
                || type_index_left_requires_parentheses(tree.get(*left))
        }
        _ => true,
    };
    if should_break_after_equals
        || should_break_template_literal_type_after_equals
        || should_break_for_prefix_annotation
        || should_break_for_union_trailing_doc_prefix_annotation
    {
        format_soft_break.format(f)?;
    } else if value_prefers_inline_after_equals {
        format_inline.format(f)?;
    } else {
        format_soft_break.format(f)?;
    }

    // type alias declarations need trailing semicolon (like const/let)
    write!(f, [token(";")])?;
    write!(f, [f.context().line_postfix_boundary_annotations(node_id)])?;

    Ok(())
}

impl<'ast> Format<DestackFormatContext<'ast>> for Visibility {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Visibility::Public => write!(f, [Keyword::Public]),
            Visibility::Protected => write!(f, [Keyword::Protected]),
            Visibility::Private => write!(f, [Keyword::Private]),
        }
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for DependencyMode {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            DependencyMode::Item => write!(f, [Keyword::Export]),
            DependencyMode::Default => write!(f, [Keyword::Export, space(), Keyword::Default]),
            DependencyMode::Namespace => write!(f, [Keyword::Export]),
        }
    }
}

impl<'ast> FormatNode<'ast, Declaration> for Declaration {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declaration>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let is_lambda_declaration = matches!(
            self,
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        );
        let declaration_expression_id =
            if let Some((parent_id, parent_type)) = f.context().parent(node_id) {
                if parent_type == NodeType::Expression {
                    let expression_id = LocalNodeId::<Expression>::new(parent_id);
                    match f.context().tree.get(expression_id) {
                        Expression::Declaration(parent_declaration_id)
                            if *parent_declaration_id == node_id =>
                        {
                            Some(expression_id)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
        let mut declaration_emits_boundary_before_terminator = false;

        match self {
            // global augmentation
            Declaration::Global {
                descriptor,
                expressions,
            } => {
                format_global_declaration(f, node_id, descriptor, expressions)?;
            }

            // module
            Declaration::Namespace {
                descriptor,
                kind,
                generics,
                expressions,
            } => {
                format_namespace_declaration(f, node_id, descriptor, *kind, generics, expressions)?;
            }

            // type alias
            Declaration::Type {
                descriptor,
                kind,
                mutability,
                static_parameters,
                value: value_id,
            } => {
                declaration_emits_boundary_before_terminator = true;
                format_type_alias_declaration(
                    f,
                    node_id,
                    descriptor,
                    *kind,
                    *mutability,
                    static_parameters,
                    *value_id,
                )?;
            }

            // import alias
            Declaration::ImportAlias {
                descriptor,
                kind,
                target,
            } => {
                declaration_emits_boundary_before_terminator = true;
                format_import_alias_declaration(f, node_id, descriptor, *kind, target)?;
            }

            // struct or class
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                members,
            }
            | Declaration::Class {
                descriptor,
                generics,
                heritage,
                members,
            } => {
                let is_class = matches!(self, Declaration::Class { .. });
                if format_struct_or_class_declaration(
                    f,
                    node_id,
                    declaration_expression_id,
                    descriptor,
                    generics,
                    heritage,
                    members,
                    is_class,
                )? {
                    return Ok(());
                }
            }

            // enum
            Declaration::Enum {
                descriptor,
                kind,
                generics,
                heritage,
                fields,
                members,
            } => {
                if format_enum_declaration(
                    f, node_id, descriptor, *kind, generics, heritage, fields, members,
                )? {
                    return Ok(());
                }
            }

            // interface
            Declaration::Interface {
                descriptor,
                kind,
                generics,
                heritage,
                members,
            } => {
                if format_interface_declaration(
                    f, node_id, descriptor, *kind, generics, heritage, members,
                )? {
                    return Ok(());
                }
            }

            // extension
            Declaration::Extension {
                descriptor,
                generics,
                target_type,
                heritage,
                members,
            } => {
                format_extension_declaration(
                    f,
                    node_id,
                    descriptor,
                    generics,
                    *target_type,
                    heritage,
                    members,
                )?;
            }

            // function
            Declaration::Function {
                descriptor,
                signature,
                body,
            } => {
                declaration_emits_boundary_before_terminator = true;
                format_function_declaration(f, node_id, descriptor, signature, body)?;
            }
        }

        if !declaration_emits_boundary_before_terminator {
            write!(f, [f.context().line_postfix_boundary_annotations(node_id)])?;
        }

        let should_skip_blank_postfix_annotations =
            is_lambda_declaration && declaration_expression_id.is_some();
        if should_skip_blank_postfix_annotations {
            if let Some(annotation_ids) = f.context().annotations(node_id) {
                for annotation_id in annotation_ids {
                    let annotation = f.context().annotation(annotation_id);
                    if matches!(annotation, Annotation::Blank { .. }) {
                        continue;
                    }
                    if !matches!(
                        annotation.position(),
                        AnnotationPosition::BlockPostfix | AnnotationPosition::LinePostfix
                    ) {
                        continue;
                    }
                    annotation.format_node(annotation_id, f)?;
                }
            }
        } else {
            write!(
                f,
                [f.context()
                    .any_postfix_except_line_postfix_boundary_annotations(node_id)]
            )?;
        }

        Ok(())
    }
}
