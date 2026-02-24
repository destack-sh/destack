use crate::format::collection::list_like;
use crate::format::declaration::declaration::format_declaration_export_modifier;
use crate::format::declaration::signature::{
    FunctionHeaderStyle, format_where_clause_with_break, parameter_is_variadic,
    signature_parameters_should_expand, signature_return_type_has_line_postfix_boundary_annotation,
    signature_should_elide_space_before_body, single_parameter_should_hug,
    write_function_header_prefix, write_signature_dynamic_parameter_list,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Comment, CommentStyle, Declaration, DeclarationDescriptor, Expression,
    FunctionCardinality, FunctionKind, FunctionMode, FunctionSignature, Keyword, LocalNodeId,
    NodeType, Parameter, Pattern,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_workspace::ArrowParentheses;

/// Return whether this file is a module typescript source.
fn is_module_typescript_file(file_name: &str) -> bool {
    file_name.ends_with(".mts") || file_name.ends_with(".cts")
}

/// Return whether a lambda parameter should preserve multiline destructuring.
fn lambda_parameter_should_expand(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let Parameter::Pattern { pattern, .. } = context.tree.get(parameter_id) else {
        return false;
    };

    let is_destructuring_pattern = matches!(
        context.tree.get(*pattern),
        Pattern::Object { .. } | Pattern::TaggedObject { .. } | Pattern::Array { .. }
    );

    if !is_destructuring_pattern {
        return false;
    }

    context.node_has_newline(*pattern)
}

/// Return whether a lambda tail parameter is a simple named binding.
fn lambda_parameter_is_simple_tail(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    matches!(
        context.tree.get(parameter_id),
        Parameter::Named {
            modifiers: None,
            ty: None,
            default: None,
            ..
        }
    )
}

/// Return whether one lambda declaration appears in statement position.
fn lambda_declaration_is_statement_position(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> bool {
    let Some((declaration_expression_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }
    let declaration_expression_id = LocalNodeId::<Expression>::new(declaration_expression_id);

    let Some((parent_id, parent_type)) = context.parent(declaration_expression_id) else {
        return false;
    };
    if parent_type == NodeType::Block {
        return true;
    }

    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_expression_id = LocalNodeId::<Expression>::new(parent_id);
    matches!(
        context.tree.get(parent_expression_id),
        Expression::Statement(inner_id) if inner_id.id == declaration_expression_id.id
    )
}

/// Return whether one lambda declaration is nested under a call argument lambda chain.
fn lambda_declaration_is_call_argument_chain(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> bool {
    let mut current_declaration_id = node_id;
    let mut has_parent_lambda = false;

    loop {
        let Some((declaration_expression_id, parent_type)) = context.parent(current_declaration_id)
        else {
            return false;
        };
        if parent_type != NodeType::Expression {
            return false;
        }
        let declaration_expression_id = LocalNodeId::<Expression>::new(declaration_expression_id);

        let Some((parent_id, parent_type)) = context.parent(declaration_expression_id) else {
            return false;
        };

        match parent_type {
            NodeType::Argument => {
                let argument_id = LocalNodeId::<Argument>::new(parent_id);
                return has_parent_lambda
                    && matches!(
                        context.tree.get(argument_id),
                        Argument::Positional { value, .. } if value.id == declaration_expression_id.id
                    );
            }
            NodeType::Declaration => {
                let parent_declaration_id = LocalNodeId::<Declaration>::new(parent_id);
                let Declaration::Function {
                    signature,
                    body: Some(parent_body_id),
                    ..
                } = context.tree.get(parent_declaration_id)
                else {
                    return false;
                };
                if signature.kind != FunctionKind::Lambda
                    || parent_body_id.id != declaration_expression_id.id
                {
                    return false;
                }

                has_parent_lambda = true;
                current_declaration_id = parent_declaration_id;
            }
            _ => return false,
        }
    }
}

/// Return whether one lambda body is an empty block containing infix annotations only.
fn lambda_body_is_empty_annotated_block(
    context: &DestackFormatContext<'_>,
    body_expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Block(block_id) = context.tree.get(body_expression_id) else {
        return false;
    };
    let block = context.tree.get(*block_id);
    block.expressions.is_empty() && context.has_non_blank_infix_annotation(*block_id)
}

/// Return whether one expression has a block-style prefix comment annotation.
fn expression_has_block_style_prefix_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let has_block_style_prefix_comment = |annotations: &[LocalNodeId<Annotation>]| {
        annotations.iter().any(|annotation_id| {
            let Annotation::Comment { node, position } = context.annotation(*annotation_id) else {
                return false;
            };
            if !matches!(
                position,
                destack_ast::AnnotationPosition::LinePrefix
                    | destack_ast::AnnotationPosition::BlockPrefix
            ) {
                return false;
            }

            let comment = context.tree.get::<Comment>(node);
            comment.style == CommentStyle::Star
        })
    };

    if context
        .visit_annotations(expression_id, has_block_style_prefix_comment)
        .unwrap_or(false)
    {
        return true;
    }

    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };

    context
        .visit_annotations(*declaration_id, has_block_style_prefix_comment)
        .unwrap_or(false)
}

/// Write one lambda arrow token with infix annotation-aware spacing.
fn write_lambda_arrow_with_infix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
) -> FormatResult<()> {
    write!(
        f,
        [f.context().declaration_arrow_infix_annotations(node_id)]
    )?;
    if !f.context().has_declaration_arrow_infix_annotation(node_id) {
        write!(f, [space()])?;
    }
    write!(f, [token("=>")])
}

/// Format a function declaration.
#[allow(clippy::too_many_arguments)]
pub(crate) fn format_function_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let generics = signature.generics.as_ref();

    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == destack_ast::DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // shared function header prefix
    write_function_header_prefix(
        f,
        signature,
        FunctionHeaderStyle::Declaration,
        descriptor.name.is_some(),
    )?;

    // constructor type signatures can own inline seam comments between `new` and `(`
    if signature.mode == Some(FunctionMode::New) {
        write!(f, [f.context().declaration_new_head_annotations(node_id)])?;
    }

    // name / key
    if signature.kind == FunctionKind::Function
        && let Some(name) = descriptor.name
    {
        write!(f, [name])?;
    }

    // static parameters
    let has_static_parameters = generics
        .and_then(|generics| generics.static_parameters.as_ref())
        .is_some_and(|params| !params.is_empty());

    if let Some(static_parameters) =
        generics.and_then(|generics| generics.static_parameters.as_ref())
        && !static_parameters.is_empty()
    {
        let needs_jsx_disambiguation = signature.kind == FunctionKind::Lambda
            && f.context().options.language_type.supports_jsx()
            && static_parameters.len() == 1;
        let needs_module_typescript_trailing_comma = signature.kind == FunctionKind::Lambda
            && static_parameters.len() == 1
            && is_module_typescript_file(&f.context().file.name);
        let mut static_params_list = list_like("<", ">", ",", static_parameters);

        if needs_jsx_disambiguation || needs_module_typescript_trailing_comma {
            static_params_list.force_trailing_separator();
        }

        write!(f, [static_params_list])?;
    }

    // dynamic parameters
    let mut dynamic_parameters = Vec::with_capacity(signature.dynamic_parameters.len() + 1);
    if let Some(this_parameter) = signature.this_parameter {
        dynamic_parameters.push(this_parameter);
    }
    dynamic_parameters.extend(signature.dynamic_parameters.iter().copied());

    // omit arrow function parentheses for simple single param lambdas
    let can_omit_parens = signature.kind == FunctionKind::Lambda
        && signature.this_parameter.is_none()
        && !has_static_parameters
        && signature.cardinality != FunctionCardinality::Generator
        && dynamic_parameters.len() == 1
        && matches!(
            f.context().options.arrow_parentheses,
            ArrowParentheses::Avoid
        )
        && {
            let parameter = f.context().tree.get(dynamic_parameters[0]);
            matches!(
                parameter,
                Parameter::Named {
                    modifiers: None,
                    ty: None,
                    default: None,
                    ..
                }
            )
        };

    // dynamic parameter rendering
    let force_expand_parameters = signature_parameters_should_expand(
        f.context(),
        signature.mode,
        &dynamic_parameters,
        signature.return_type,
        true,
    );
    if can_omit_parens {
        write!(f, [&dynamic_parameters[0]])?;
    } else if dynamic_parameters.len() == 1
        && !force_expand_parameters
        && single_parameter_should_hug(f.context(), dynamic_parameters[0])
    {
        write!(f, [token("("), dynamic_parameters[0], token(")")])?;
    } else if signature.kind == FunctionKind::Lambda
        && dynamic_parameters.len() == 2
        && lambda_parameter_should_expand(f.context(), dynamic_parameters[0])
        && lambda_parameter_is_simple_tail(f.context(), dynamic_parameters[1])
    {
        // keep callback parameters compact: `({ ... }, tail)`
        write!(
            f,
            [
                token("("),
                group(&dynamic_parameters[0]).should_expand(true),
                token(","),
                space(),
                dynamic_parameters[1],
                token(")")
            ]
        )?;
    } else {
        let disallow_trailing_parameter_separator = dynamic_parameters
            .last()
            .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id))
            || (signature.kind == FunctionKind::Lambda && dynamic_parameters.len() == 1);
        write_signature_dynamic_parameter_list(
            f,
            &dynamic_parameters,
            force_expand_parameters,
            disallow_trailing_parameter_separator,
        )?;
    }

    // return type
    if let Some(return_type) = signature.return_type {
        if signature.kind == FunctionKind::Lambda && body.is_none() {
            write_lambda_arrow_with_infix_annotations(f, node_id)?;
            write!(f, [space(), return_type])?;
        } else {
            write!(f, [token(":"), space(), return_type])?;
        }
    }

    // where clause
    if let Some(where_clauses) = generics.and_then(|generics| generics.where_clauses.as_ref())
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    // body
    if let Some(body) = body {
        if signature.kind == FunctionKind::Lambda {
            let body_expression = f.context().tree.get(*body);
            let body_transparent_expression_id =
                crate::format::expression::transparent_inner_expression(f.context(), *body);
            let body_transparent_expression = f.context().tree.get(body_transparent_expression_id);
            let body_is_block = matches!(body_expression, Expression::Block(_));
            let body_is_tree = matches!(body_expression, Expression::TreeExpression { .. });
            let body_is_parenthesized_tree =
                matches!(body_expression, Expression::Parenthesized { .. })
                    && matches!(
                        body_transparent_expression,
                        Expression::TreeExpression { .. }
                    );
            let force_break =
                crate::format::expression::lambda_expression_should_break(f.context(), node_id);

            // arrow is fine since lambdas can only have return type or body
            if body_is_block {
                write_lambda_arrow_with_infix_annotations(f, node_id)?;
                let should_dedent_body =
                    lambda_declaration_is_call_argument_chain(f.context(), node_id)
                        && lambda_body_is_empty_annotated_block(f.context(), *body);
                if should_dedent_body {
                    write!(f, [space(), dedent(body)])?;
                } else {
                    write!(f, [space(), body])?;
                }
            } else if body_is_tree {
                // keep lambda tree bodies with one conditional wrapper pair
                let body_group_id = f.group_id("lambda_body");
                let parenthesized_body =
                    format_with(|f| write!(f, [token("("), soft_block_indent(&body), token(")")]));
                let body_break = format_with(|f| {
                    write!(
                        f,
                        [
                            if_group_breaks(&parenthesized_body).with_group_id(Some(body_group_id)),
                            if_group_fits_on_line(&body).with_group_id(Some(body_group_id))
                        ]
                    )?;
                    Ok(())
                });

                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| write_lambda_arrow_with_infix_annotations(f, node_id)),
                        space(),
                        body_break
                    ])
                    .with_id(Some(body_group_id))
                    .should_expand(force_break)]
                )?;
            } else if body_is_parenthesized_tree {
                // keep `=> (` attached for already-grouped tree bodies
                let body_break = format_with(|f| write!(f, [body]));
                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| write_lambda_arrow_with_infix_annotations(f, node_id)),
                        space(),
                        body_break
                    ])
                    .should_expand(force_break)]
                )?;
            } else {
                // default expression body formatting
                let body_break = format_with(|f| write!(f, [body]));
                let inline_body_expression_id =
                    crate::format::expression::transparent_inner_expression(f.context(), *body);
                let body_is_lambda_with_block_prefix = matches!(
                    f.context().tree.get(inline_body_expression_id),
                    Expression::Declaration(declaration_id)
                        if matches!(
                            f.context().tree.get(*declaration_id),
                            Declaration::Function { signature, .. }
                                if signature.kind == FunctionKind::Lambda
                        )
                )
                    && expression_has_block_style_prefix_comment(
                        f.context(),
                        inline_body_expression_id,
                    );

                if body_is_lambda_with_block_prefix {
                    write!(
                        f,
                        [group(&format_args![
                            format_with(|f| {
                                write_lambda_arrow_with_infix_annotations(f, node_id)
                            }),
                            soft_line_break_or_space(),
                            body_break
                        ])
                        .should_expand(force_break)]
                    )?;
                    return Ok(());
                }

                write!(
                    f,
                    [group(&format_args![
                        format_with(|f| write_lambda_arrow_with_infix_annotations(f, node_id)),
                        indent(&format_args![soft_line_break_or_space(), body_break])
                    ])
                    .should_expand(force_break)]
                )?;
            }
        } else if signature_return_type_has_line_postfix_boundary_annotation(
            f.context(),
            signature.return_type,
        ) {
            write!(f, [hard_line_break(), body])?;
        } else if signature_should_elide_space_before_body(f.context(), signature.return_type) {
            write!(f, [body])?;
        } else {
            write!(f, [space(), body])?;
        }
    }

    // trailing semicolon rules
    let is_exported_lambda_declaration =
        signature.kind == FunctionKind::Lambda && descriptor.export.is_some();
    let is_statement_lambda_declaration = signature.kind == FunctionKind::Lambda
        && lambda_declaration_is_statement_position(f.context(), node_id);
    let is_bodyless_function_declaration =
        signature.kind == FunctionKind::Function && body.is_none();

    let needs_trailing_semicolon = is_exported_lambda_declaration
        || is_statement_lambda_declaration
        || is_bodyless_function_declaration;

    if needs_trailing_semicolon {
        write!(f, [token(";")])?;
    }

    Ok(())
}
