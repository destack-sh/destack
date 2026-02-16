use crate::argument::list_like;
use crate::signature::{
    FunctionHeaderStyle, parameter_is_variadic, signature_parameters_should_expand,
    signature_return_type_has_line_postfix_boundary_annotation,
    signature_should_elide_space_before_body, single_parameter_should_hug,
    write_function_header_prefix, write_signature_dynamic_parameter_list,
};
use crate::r#where::format_where_clause_with_break;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Declaration, DeclarationDescriptor, Expression, FunctionCardinality, FunctionKind,
    FunctionSignature, Keyword, LocalNodeId, Parameter, Pattern,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_workspace::ArrowParentheses;

/// Return whether this file is a module typescript source.
fn is_module_typescript_source(file_name: &str) -> bool {
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

/// Format a function declaration.
#[allow(clippy::too_many_arguments)]
pub(super) fn format_function_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let generics = signature.generics.as_ref();

    // export
    if let Some(export) = descriptor.export {
        write!(f, [export, space()])?;
    }

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
            && is_module_typescript_source(&f.context().file.name);
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
            write!(
                f,
                [
                    f.context().block_infix_annotations(node_id),
                    space(),
                    token("=>"),
                    space(),
                    return_type
                ]
            )?;
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
            let body_is_block = matches!(body_expression, Expression::Block(_));
            let body_is_tree = matches!(body_expression, Expression::TreeExpression { .. });
            let body_is_parenthesized_tree = matches!(
                body_expression,
                Expression::Parenthesized { expression: inner_id }
                    if matches!(
                        f.context().tree.get(*inner_id),
                        Expression::TreeExpression { .. }
                    )
            );
            let force_break =
                crate::expression::lambda_expression_should_break(f.context(), node_id);

            // arrow is fine since lambdas can only have return type or body
            if body_is_block {
                write!(
                    f,
                    [
                        f.context().block_infix_annotations(node_id),
                        space(),
                        token("=>"),
                        space(),
                        body
                    ]
                )?;
            } else if body_is_tree {
                // tree bodies need conditional parentheses when they break
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
                        f.context().block_infix_annotations(node_id),
                        space(),
                        token("=>"),
                        space(),
                        body_break
                    ])
                    .with_id(Some(body_group_id))
                    .should_expand(force_break)]
                )?;
            } else {
                // default expression body formatting
                let body_break = format_with(|f| write!(f, [body]));

                if body_is_parenthesized_tree {
                    write!(
                        f,
                        [group(&format_args![
                            f.context().block_infix_annotations(node_id),
                            space(),
                            token("=>"),
                            space(),
                            body_break
                        ])
                        .should_expand(force_break)]
                    )?;
                } else {
                    write!(
                        f,
                        [group(&format_args![
                            f.context().block_infix_annotations(node_id),
                            space(),
                            token("=>"),
                            indent(&format_args![soft_line_break_or_space(), body_break])
                        ])
                        .should_expand(force_break)]
                    )?;
                }
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

    // trailing semicolon policy
    let is_exported_lambda_declaration =
        signature.kind == FunctionKind::Lambda && descriptor.export.is_some();
    let is_bodyless_function_declaration =
        signature.kind == FunctionKind::Function && body.is_none();

    let needs_trailing_semicolon =
        is_exported_lambda_declaration || is_bodyless_function_declaration;

    if needs_trailing_semicolon {
        write!(f, [token(";")])?;
    }

    Ok(())
}
