use crate::format::annotation::{block_infix_annotations, line_suffix_boundary_annotations};
use crate::format::chain::transparent_inner_expression;
use crate::format::collection::TrailingSeparator;
use crate::format::context::DestackFormatterCommentExt;
use crate::format::declaration::declaration::format_declaration_export_modifier;
use crate::format::declaration::signature::{
    default_static_parameter_trailing_separator, expression_body_requires_head_space,
    format_where_clause_with_break, parameter_is_variadic, should_break_function_parameters,
    signature_return_type_has_line_suffix_boundary_annotation, single_parameter_should_hug,
    write_empty_parameter_list_with_interior_comments, write_function_header_prefix,
    write_signature_dynamic_parameter_list, write_signature_hug_parameter_list,
    write_static_parameter_list,
};
use crate::format::declaration::statement::format_block;
use crate::format::operator::write_type_expression_with_inline_prefix_annotations;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Declaration, DeclarationDescriptor, Expression, FunctionCardinality, FunctionKind,
    FunctionMode, FunctionSignature, Keyword, LocalNodeId, NodeType, Parameter,
};
use destack_fir::format::{Buffer, FormatResult, RemoveSoftLinesBuffer};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_workspace::ArrowParentheses;

/// The grouped call-argument layout shared with lambda formatting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GroupedCallArgumentLayout {
    /// Group the first call argument.
    GroupedFirstArgument,
    /// Group the last call argument.
    GroupedLastArgument,
}

/// The explicit function formatting options.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct FormatFunctionDeclarationOptions {
    /// The grouped call-argument layout, when this lambda is being formatted as one.
    pub grouped_call_argument_layout: Option<GroupedCallArgumentLayout>,
}

/// Return whether this file is a module typescript source.
fn is_module_typescript_file(file_name: &str) -> bool {
    file_name.ends_with(".mts") || file_name.ends_with(".cts")
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

    let Some((_, parent_type)) = context.parent(declaration_expression_id) else {
        return false;
    };

    parent_type == NodeType::Block
}

/// Write one lambda arrow token with local infix spacing.
fn write_lambda_arrow_with_infix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
) -> FormatResult<()> {
    write!(f, [block_infix_annotations(f.context(), node_id)])?;
    if !f.context().has_infix_annotation(node_id) {
        write!(f, [space()])?;
    }
    write!(f, [token("=>")])
}

/// Collect dynamic parameters, including `this`.
fn function_dynamic_parameters(signature: &FunctionSignature) -> Vec<LocalNodeId<Parameter>> {
    let mut dynamic_parameters = Vec::with_capacity(signature.dynamic_parameters.len() + 1);

    if let Some(this_parameter) = signature.this_parameter {
        dynamic_parameters.push(this_parameter);
    }

    dynamic_parameters.extend(signature.dynamic_parameters.iter().copied());
    dynamic_parameters
}

/// Return whether one lambda can omit parentheses around its single parameter.
fn function_can_omit_lambda_parameter_parentheses(
    f: &DestackFormatter<'_, '_>,
    signature: &FunctionSignature,
    dynamic_parameters: &[LocalNodeId<Parameter>],
    has_static_parameters: bool,
) -> bool {
    signature.kind == FunctionKind::Lambda
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
        }
}

/// Write one function static parameter list.
fn write_function_static_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    signature: &FunctionSignature,
) -> FormatResult<bool> {
    let Some(static_parameters) = signature
        .generics
        .as_ref()
        .and_then(|generics| generics.static_parameters.as_ref())
    else {
        return Ok(false);
    };
    if static_parameters.is_empty() {
        return Ok(false);
    }

    let needs_jsx_disambiguation = signature.kind == FunctionKind::Lambda
        && f.context().options.language_type.supports_jsx()
        && static_parameters.len() == 1;
    let needs_module_typescript_trailing_comma = signature.kind == FunctionKind::Lambda
        && static_parameters.len() == 1
        && is_module_typescript_file(&f.context().file.name);
    let trailing_separator = if needs_jsx_disambiguation || needs_module_typescript_trailing_comma {
        TrailingSeparator::Mandatory
    } else {
        default_static_parameter_trailing_separator(f)
    };

    write_static_parameter_list(f, static_parameters, trailing_separator)?;

    Ok(true)
}

/// Write one function parameter list.
fn write_function_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    dynamic_parameters: &[LocalNodeId<Parameter>],
    can_omit_parens: bool,
) -> FormatResult<()> {
    let force_expand_parameters = should_break_function_parameters(f.context(), dynamic_parameters);

    if dynamic_parameters.is_empty() {
        return write_empty_parameter_list_with_interior_comments(f, node_id);
    }

    if can_omit_parens {
        return write!(f, [&dynamic_parameters[0]]);
    }

    if dynamic_parameters.len() == 1
        && single_parameter_should_hug(f.context(), dynamic_parameters[0])
        && (!force_expand_parameters
            || parameter_is_destructuring_pattern(f.context(), dynamic_parameters[0]))
    {
        return write_signature_hug_parameter_list(f, dynamic_parameters);
    }

    let disallow_trailing_parameter_separator = dynamic_parameters
        .last()
        .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id))
        || (signature.kind == FunctionKind::Lambda && dynamic_parameters.len() == 1);

    write_signature_dynamic_parameter_list(
        f,
        dynamic_parameters,
        force_expand_parameters,
        disallow_trailing_parameter_separator,
    )
}

/// Write one function return type.
fn write_function_return_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let Some(return_type) = signature.return_type else {
        return Ok(());
    };

    if signature.kind == FunctionKind::Lambda && body.is_none() {
        write_lambda_arrow_with_infix_annotations(f, node_id)?;
        write!(f, [space()])?;
        return write_type_expression_with_inline_prefix_annotations(f, return_type);
    }

    write!(f, [token(":"), space()])?;
    write_type_expression_with_inline_prefix_annotations(f, return_type)
}

/// Write one function parameter list and return type.
fn write_function_parameters_and_return_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    dynamic_parameters: &[LocalNodeId<Parameter>],
    can_omit_parens: bool,
    options: FormatFunctionDeclarationOptions,
) -> FormatResult<()> {
    let format_parameters_and_return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // parameters
        write_function_parameters(f, node_id, signature, dynamic_parameters, can_omit_parens)?;

        // return type
        write_function_return_type(f, node_id, signature, body)
    });

    if signature.kind == FunctionKind::Lambda && options.grouped_call_argument_layout.is_some() {
        let interned = f.intern_with_comment_snapshot(&format_parameters_and_return_type)?;

        if let Some(interned) = interned {
            let mut buffer = RemoveSoftLinesBuffer::new(f);
            buffer.write_node(interned);
        }

        Ok(())
    } else if signature.kind == FunctionKind::Lambda {
        write!(f, [format_parameters_and_return_type])
    } else {
        write!(f, [group(&format_parameters_and_return_type)])
    }
}

/// Return whether one function declaration needs a trailing semicolon.
fn function_declaration_needs_trailing_semicolon(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
) -> bool {
    let is_exported_lambda_declaration =
        signature.kind == FunctionKind::Lambda && descriptor.export.is_some();
    let is_statement_lambda_declaration = signature.kind == FunctionKind::Lambda
        && lambda_declaration_is_statement_position(context, node_id);
    let is_bodyless_function_declaration =
        signature.kind == FunctionKind::Function && body.is_none();

    is_exported_lambda_declaration
        || is_statement_lambda_declaration
        || is_bodyless_function_declaration
}

/// Write one lambda function body.
fn write_lambda_function_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    body: LocalNodeId<Expression>,
    options: FormatFunctionDeclarationOptions,
) -> FormatResult<()> {
    let body_expression = f.context().tree.get(body);
    let body_transparent_expression_id = transparent_inner_expression(f.context(), body);
    let body_transparent_expression = f.context().tree.get(body_transparent_expression_id);
    let body_is_block = matches!(body_expression, Expression::Block(_));
    let body_is_tree = matches!(body_expression, Expression::TreeExpression { .. });
    let body_is_parenthesized_tree = matches!(body_expression, Expression::Parenthesized { .. })
        && matches!(
            body_transparent_expression,
            Expression::TreeExpression { .. }
        );

    if body_is_block {
        write_lambda_arrow_with_infix_annotations(f, node_id)?;
        if expression_body_requires_head_space(f.context(), body) {
            write!(f, [space()])?;
        }
        let Expression::Block(block_id) = body_expression else {
            unreachable!();
        };
        if options.grouped_call_argument_layout.is_some() {
            return write!(f, [*block_id]);
        }

        return format_block(f, *block_id);
    }

    if body_is_tree {
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

        return write!(
            f,
            [group(&format_args![
                format_with(|f| write_lambda_arrow_with_infix_annotations(f, node_id)),
                space(),
                body_break
            ])
            .with_id(Some(body_group_id))
            .should_expand(false)]
        );
    }

    if body_is_parenthesized_tree {
        return write!(
            f,
            [group(&format_args![
                format_with(|f| write_lambda_arrow_with_infix_annotations(f, node_id)),
                space(),
                format_with(|f| write!(f, [body]))
            ])
            .should_expand(false)]
        );
    }

    write!(
        f,
        [group(&format_args![
            format_with(|f| write_lambda_arrow_with_infix_annotations(f, node_id)),
            indent(&format_args![
                soft_line_break_or_space(),
                format_with(|f| write!(f, [body]))
            ])
        ])
        .should_expand(false)]
    )
}

/// Write one non-lambda function body.
fn write_non_lambda_function_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    signature: &FunctionSignature,
    body: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if signature_return_type_has_line_suffix_boundary_annotation(f.context(), signature.return_type)
    {
        return write!(f, [hard_line_break(), body]);
    }

    if expression_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }

    if let Expression::Block(block_id) = f.context().tree.get(body) {
        return format_block(f, *block_id);
    }

    write!(f, [body])
}

/// Write one function body and trailing semicolon.
fn write_function_body_and_terminator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    options: FormatFunctionDeclarationOptions,
) -> FormatResult<()> {
    // body
    if let Some(body) = body {
        if signature.kind == FunctionKind::Lambda {
            write_lambda_function_body(f, node_id, *body, options)?;
        } else {
            write_non_lambda_function_body(f, signature, *body)?;
        }
    }

    write!(f, [line_suffix_boundary_annotations(f.context(), node_id)])?;

    if function_declaration_needs_trailing_semicolon(
        f.context(),
        node_id,
        descriptor,
        signature,
        body,
    ) {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Write the shared function head before the body.
fn write_function_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    options: FormatFunctionDeclarationOptions,
) -> FormatResult<()> {
    let generics = signature.generics.as_ref();
    let dynamic_parameters = function_dynamic_parameters(signature);
    let has_static_parameters = signature
        .generics
        .as_ref()
        .and_then(|generics| generics.static_parameters.as_ref())
        .is_some_and(|params| !params.is_empty());
    let can_omit_parens = function_can_omit_lambda_parameter_parentheses(
        f,
        signature,
        &dynamic_parameters,
        has_static_parameters,
    );

    // shared function header prefix
    write_function_header_prefix(f, signature, true, descriptor.name.is_some())?;

    // constructor type signatures can own inline comments between `new` and `(`
    if signature.mode == Some(FunctionMode::New) {
        write!(f, [block_infix_annotations(f.context(), node_id)])?;
    }

    // declaration name boundary
    if signature.kind == FunctionKind::Function && descriptor.name.is_some() {
        write!(f, [block_infix_annotations(f.context(), node_id)])?;
    }

    // name / key
    if signature.kind == FunctionKind::Function
        && let Some(name) = descriptor.name
    {
        write!(f, [name])?;
    }

    // static parameters
    if signature.kind == FunctionKind::Lambda && options.grouped_call_argument_layout.is_some() {
        let static_parameters = format_with(|f| {
            write_function_static_parameters(f, signature)?;
            Ok(())
        });
        let interned = f.intern_with_comment_snapshot(&static_parameters)?;

        if let Some(interned) = interned {
            let mut buffer = RemoveSoftLinesBuffer::new(f);
            buffer.write_node(interned);
        }
    } else {
        write_function_static_parameters(f, signature)?;
    }

    // declaration parameter head boundary
    if signature.kind == FunctionKind::Function {
        write!(f, [block_infix_annotations(f.context(), node_id)])?;
    }

    // parameters and return type
    write_function_parameters_and_return_type(
        f,
        node_id,
        signature,
        body,
        &dynamic_parameters,
        can_omit_parens,
        options,
    )?;

    // where clause
    if let Some(where_clauses) = generics.and_then(|generics| generics.where_clauses.as_ref())
        && !where_clauses.is_empty()
    {
        if signature.kind == FunctionKind::Lambda && options.grouped_call_argument_layout.is_some()
        {
            let where_clause = format_with(|f| format_where_clause_with_break(f, where_clauses));
            let interned = f.intern_with_comment_snapshot(&where_clause)?;

            if let Some(interned) = interned {
                let mut buffer = RemoveSoftLinesBuffer::new(f);
                buffer.write_node(interned);
            }
        } else {
            format_where_clause_with_break(f, where_clauses)?;
        }
    }

    Ok(())
}

/// Format a function declaration.
pub(crate) fn format_function_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    format_function_declaration_with_options(
        f,
        node_id,
        descriptor,
        signature,
        body,
        FormatFunctionDeclarationOptions::default(),
    )
}

/// Format a function declaration with explicit formatting options.
#[allow(clippy::too_many_arguments)]
pub(crate) fn format_function_declaration_with_options<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    options: FormatFunctionDeclarationOptions,
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == destack_ast::DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // head
    write_function_head(f, node_id, descriptor, signature, body, options)?;

    // body and terminator
    write_function_body_and_terminator(f, node_id, descriptor, signature, body, options)
}
