use crate::format::chain::{lambda_expression_should_break, transparent_inner_expression};
use crate::format::collection::TrailingSeparator;
use crate::format::declaration::declaration::format_declaration_export_modifier;
use crate::format::declaration::signature::{
    default_static_parameter_trailing_separator, expression_body_requires_head_space,
    format_where_clause_with_break, parameter_is_variadic, signature_parameters_should_expand,
    signature_return_type_has_line_postfix_boundary_annotation,
    signature_should_elide_space_before_body, single_parameter_should_hug,
    write_empty_parameter_list_with_interior_comments, write_function_header_prefix,
    write_signature_dynamic_parameter_list, write_static_parameter_list,
};
use crate::format::declaration::statement::format_block;
use crate::format::operator::write_expression_with_inline_prefix_annotations;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Argument, Declaration, DeclarationDescriptor, Expression, FunctionCardinality, FunctionKind,
    FunctionMode, FunctionSignature, Keyword, LocalNodeId, NodeType, Parameter, Pattern,
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

/// Return whether one parameter uses a destructuring pattern.
fn parameter_is_destructuring_pattern(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    matches!(
        context.tree.get(parameter_id),
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. }
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
    parent_expression_id.id == declaration_expression_id.id
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

/// Return whether one lambda declaration is the body of a parent lambda declaration.
fn lambda_declaration_has_parent_lambda_body(
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
    if parent_type != NodeType::Declaration {
        return false;
    }

    let parent_declaration_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Function {
        signature,
        body: Some(parent_body_id),
        ..
    } = context.tree.get(parent_declaration_id)
    else {
        return false;
    };

    signature.kind == FunctionKind::Lambda && parent_body_id.id == declaration_expression_id.id
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
    block.is_empty() && context.has_infix_annotation(*block_id)
}

/// Write one lambda arrow token with local infix spacing.
fn write_lambda_arrow_with_infix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
) -> FormatResult<()> {
    write!(
        f,
        [crate::format::annotation::block_infix_annotations(
            f.context(),
            node_id
        )]
    )?;
    if !f.context().has_infix_annotation(node_id) {
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
    write_function_header_prefix(f, signature, true, descriptor.name.is_some())?;

    // constructor type signatures can own inline seam comments between `new` and `(`
    if signature.mode == Some(FunctionMode::New) {
        write!(
            f,
            [crate::format::annotation::block_infix_annotations(
                f.context(),
                node_id
            )]
        )?;
    }

    // declaration name seam
    if signature.kind == FunctionKind::Function && descriptor.name.is_some() {
        write!(
            f,
            [crate::format::annotation::block_infix_annotations(
                f.context(),
                node_id
            )]
        )?;
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
        write_static_parameter_list(
            f,
            static_parameters,
            if needs_jsx_disambiguation || needs_module_typescript_trailing_comma {
                TrailingSeparator::Mandatory
            } else {
                default_static_parameter_trailing_separator(f)
            },
        )?;
    }

    // declaration parameter head seam
    if signature.kind == FunctionKind::Function {
        write!(
            f,
            [crate::format::annotation::block_infix_annotations(
                f.context(),
                node_id
            )]
        )?;
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
    if dynamic_parameters.is_empty() {
        write_empty_parameter_list_with_interior_comments(f, node_id)?;
    } else if can_omit_parens {
        write!(f, [&dynamic_parameters[0]])?;
    } else if dynamic_parameters.len() == 1
        && single_parameter_should_hug(f.context(), dynamic_parameters[0])
        && (!force_expand_parameters
            || parameter_is_destructuring_pattern(f.context(), dynamic_parameters[0]))
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
            write!(f, [space()])?;
            write_expression_with_inline_prefix_annotations(f, return_type)?;
        } else {
            write!(f, [token(":"), space()])?;
            write_expression_with_inline_prefix_annotations(f, return_type)?;
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
            let body_transparent_expression_id = transparent_inner_expression(f.context(), *body);
            let body_transparent_expression = f.context().tree.get(body_transparent_expression_id);
            let body_is_block = matches!(body_expression, Expression::Block(_));
            let body_is_tree = matches!(body_expression, Expression::TreeExpression { .. });
            let body_is_parenthesized_tree =
                matches!(body_expression, Expression::Parenthesized { .. })
                    && matches!(
                        body_transparent_expression,
                        Expression::TreeExpression { .. }
                    );
            let body_is_lambda_declaration = matches!(
                body_transparent_expression,
                Expression::Declaration(body_declaration_id)
                    if matches!(
                        f.context().tree.get(*body_declaration_id),
                        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
                    )
            );
            let lambda_is_non_head_chain_link =
                lambda_declaration_has_parent_lambda_body(f.context(), node_id);
            let force_break = lambda_expression_should_break(f.context(), node_id);

            // arrow is fine since lambdas can only have return type or body
            if body_is_block {
                write_lambda_arrow_with_infix_annotations(f, node_id)?;
                let should_dedent_body =
                    lambda_declaration_is_call_argument_chain(f.context(), node_id)
                        && lambda_body_is_empty_annotated_block(f.context(), *body);
                if should_dedent_body {
                    if expression_body_requires_head_space(f.context(), *body) {
                        write!(f, [space()])?;
                    }
                    let Expression::Block(block_id) = body_expression else {
                        unreachable!();
                    };
                    write!(f, [dedent(&format_with(|f| format_block(f, *block_id)))])?;
                } else {
                    if expression_body_requires_head_space(f.context(), *body) {
                        write!(f, [space()])?;
                    }
                    let Expression::Block(block_id) = body_expression else {
                        unreachable!();
                    };
                    format_block(f, *block_id)?;
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
                let should_avoid_extra_chain_indent =
                    body_is_lambda_declaration && lambda_is_non_head_chain_link;
                if should_avoid_extra_chain_indent {
                    write!(
                        f,
                        [group(&format_args![
                            format_with(|f| write_lambda_arrow_with_infix_annotations(f, node_id)),
                            soft_line_break_or_space(),
                            body_break
                        ])
                        .should_expand(force_break)]
                    )?;
                } else {
                    write!(
                        f,
                        [group(&format_args![
                            format_with(|f| write_lambda_arrow_with_infix_annotations(f, node_id)),
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
            if expression_body_requires_head_space(f.context(), *body) {
                write!(f, [space()])?;
            }
            if let Expression::Block(block_id) = f.context().tree.get(*body) {
                format_block(f, *block_id)?;
            } else {
                write!(f, [body])?;
            }
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

    write!(
        f,
        [crate::format::annotation::line_postfix_boundary_annotations(f.context(), node_id)]
    )?;

    if needs_trailing_semicolon {
        write!(f, [token(";")])?;
    }

    Ok(())
}
