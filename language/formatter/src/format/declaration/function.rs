use crate::format::annotation::{
    block_infix_annotations, format_raw_comment, line_suffix_boundary_annotations,
    write_raw_comment_slice,
};
use crate::format::chain::transparent_inner_expression;
use crate::format::collection::TrailingSeparator;
use crate::format::context::DestackFormatterCommentExt;
use crate::format::declaration::signature::{
    default_generic_parameter_trailing_separator, expression_body_requires_head_space,
    format_where_clause_with_break, parameter_is_variadic, should_break_function_parameters,
    signature_return_type_has_line_suffix_boundary_annotation, single_parameter_should_hug,
    write_empty_parameter_list_with_interior_comments, write_function_header_prefix,
    write_generic_parameter_list, write_signature_hug_parameter_list,
    write_signature_parameter_list, write_signature_return_type_with_boundary_comments,
};
use crate::format::declaration::statement::format_block;
use crate::format::operator::write_type_expression_with_inline_prefix_annotations;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Ambientness, Comment, Declaration, ExportMode, Expression, FunctionCardinality, FunctionKind,
    FunctionMode, FunctionSignature, GenericParameter, Keyword, LocalNodeId, Name, NodeType,
    Parameter, TokenType,
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

/// Return whether one file name uses one module extension.
fn file_uses_module_only_extension(file_name: &str) -> bool {
    file_name.ends_with(".mts") || file_name.ends_with(".cts")
}

/// Write one declaration export prefix.
fn write_function_export_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    export: Option<ExportMode>,
) -> FormatResult<()> {
    // export
    match export {
        Some(ExportMode::Named) => write!(f, [Keyword::Export, space()])?,
        Some(ExportMode::Default) => {
            write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
        }
        None => {}
    }

    Ok(())
}

/// Write one declaration ambient prefix.
fn write_function_ambient_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    ambient: Ambientness,
) -> FormatResult<()> {
    // ambient
    if ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    Ok(())
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

/// Return comments between the signature close delimiter and one lambda arrow.
fn lambda_arrow_boundary_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    parameters: &[LocalNodeId<Parameter>],
) -> Vec<Comment> {
    let close_parenthesis = if let Some(last_parameter) = parameters.last().copied() {
        context
            .next_non_trivia_token_after_span(context.span(last_parameter))
            .filter(|token| token.token.ty == TokenType::CloseParenthesis)
    } else {
        let node_span = context.span(node_id);
        context
            .nth_non_trivia_token_in_span(node_span, 1)
            .filter(|token| token.token.ty == TokenType::CloseParenthesis)
    };
    let Some(close_parenthesis) = close_parenthesis else {
        return Vec::new();
    };

    context
        .comments()
        .comments_before_character(close_parenthesis.span.end, b'=')
        .to_vec()
}

/// Return comments between a constructor `new` head and the parameter list.
fn constructor_parameter_head_boundary_comments(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> Vec<Comment> {
    let node_span = context.span(node_id);

    context
        .comments()
        .comments_before_character(node_span.start, b'(')
        .to_vec()
}

/// Collect parameters, including `this`.
fn function_parameters(signature: &FunctionSignature) -> Vec<LocalNodeId<Parameter>> {
    let mut parameters = Vec::with_capacity(signature.parameters.len() + 1);

    if let Some(this_parameter) = signature.this_parameter {
        parameters.push(this_parameter);
    }

    parameters.extend(signature.parameters.iter().copied());
    parameters
}

/// Return whether one lambda can omit parentheses around its single parameter.
fn function_can_omit_lambda_parameter_parentheses(
    f: &DestackFormatter<'_, '_>,
    signature: &FunctionSignature,
    parameters: &[LocalNodeId<Parameter>],
    has_generic_parameters: bool,
) -> bool {
    signature.kind == FunctionKind::Lambda
        && signature.this_parameter.is_none()
        && !has_generic_parameters
        && signature.cardinality != FunctionCardinality::Generator
        && parameters.len() == 1
        && matches!(
            f.context().options.arrow_parentheses,
            ArrowParentheses::Avoid
        )
        && {
            let parameter = f.context().tree.get(parameters[0]);
            matches!(
                parameter,
                Parameter::Named {
                    visibility: None,
                    is_readonly: false,
                    declared_type: None,
                    default: None,
                    is_optional: false,
                    ..
                }
            )
        }
}

/// Return whether one single lambda generic parameter needs a trailing separator.
fn single_lambda_generic_parameter_needs_trailing_separator(
    f: &DestackFormatter<'_, '_>,
    signature: &FunctionSignature,
) -> bool {
    if signature.kind != FunctionKind::Lambda || signature.generic_parameters.len() != 1 {
        return false;
    }

    let generic_parameter = f.context().tree.get(signature.generic_parameters[0]);
    let is_plain_parameter = match generic_parameter {
        GenericParameter::Type {
            constraint,
            default,
            ..
        } => constraint.is_none() && default.is_none(),
        GenericParameter::Value {
            declared_type,
            default,
            ..
        } => declared_type.is_none() && default.is_none(),
        GenericParameter::Error => false,
    };
    if !is_plain_parameter {
        return false;
    }

    if f.context().options.language_type.supports_jsx() {
        return true;
    }

    file_uses_module_only_extension(&f.context().file.name)
}

/// Write one function generic parameter list.
fn write_function_generic_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    signature: &FunctionSignature,
) -> FormatResult<bool> {
    let generic_parameters = signature.generic_parameters.as_slice();
    if generic_parameters.is_empty() {
        return Ok(false);
    }

    let trailing_separator =
        if single_lambda_generic_parameter_needs_trailing_separator(f, signature) {
            TrailingSeparator::Mandatory
        } else {
            default_generic_parameter_trailing_separator(f)
        };

    write_generic_parameter_list(f, generic_parameters, trailing_separator)?;

    Ok(true)
}

/// Write one function parameter list.
fn write_function_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    parameters: &[LocalNodeId<Parameter>],
    can_omit_parens: bool,
) -> FormatResult<()> {
    let force_expand_parameters = should_break_function_parameters(f.context(), parameters);

    if parameters.is_empty() {
        return write_empty_parameter_list_with_interior_comments(f, node_id);
    }

    if can_omit_parens {
        return write!(f, [&parameters[0]]);
    }

    if parameters.len() == 1
        && single_parameter_should_hug(f.context(), parameters[0])
        && (!force_expand_parameters
            || parameter_is_destructuring_pattern(f.context(), parameters[0]))
    {
        return write_signature_hug_parameter_list(f, parameters);
    }

    let disallow_trailing_parameter_separator = parameters
        .last()
        .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id))
        || (signature.kind == FunctionKind::Lambda && parameters.len() == 1);

    write_signature_parameter_list(
        f,
        parameters,
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
    parameters: &[LocalNodeId<Parameter>],
) -> FormatResult<()> {
    let Some(return_type) = signature.return_type else {
        return Ok(());
    };

    if signature.kind == FunctionKind::Lambda && body.is_none() {
        let arrow_boundary_comments =
            lambda_arrow_boundary_comments(f.context(), node_id, parameters);
        write_raw_comment_slice(f, &arrow_boundary_comments)?;
        write_lambda_arrow_with_infix_annotations(f, node_id)?;
        write!(f, [space()])?;
        return write_type_expression_with_inline_prefix_annotations(f, return_type);
    }

    write!(f, [token(":"), space()])?;
    write_signature_return_type_with_boundary_comments(f, return_type)
}

/// Write one function parameter list and return type.
fn write_function_parameters_and_return_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    parameters: &[LocalNodeId<Parameter>],
    can_omit_parens: bool,
    options: FormatFunctionDeclarationOptions,
) -> FormatResult<()> {
    let format_parameters_and_return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // parameters
        write_function_parameters(f, node_id, signature, parameters, can_omit_parens)?;

        // return type
        write_function_return_type(f, node_id, signature, body, parameters)
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
    export: Option<ExportMode>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
) -> bool {
    let is_exported_lambda_declaration = signature.kind == FunctionKind::Lambda && export.is_some();
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
    export: Option<ExportMode>,
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

    if function_declaration_needs_trailing_semicolon(f.context(), node_id, export, signature, body)
    {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Write the shared function head before the body.
fn write_function_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    name: Option<Name>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    options: FormatFunctionDeclarationOptions,
) -> FormatResult<()> {
    let parameters = function_parameters(signature);
    let has_generic_parameters = !signature.generic_parameters.is_empty();
    let can_omit_parens = function_can_omit_lambda_parameter_parentheses(
        f,
        signature,
        &parameters,
        has_generic_parameters,
    );

    // shared function header prefix
    write_function_header_prefix(f, signature, true, name.is_some())?;

    // constructor type signatures can own inline comments between `new` and `(`
    if signature.mode == Some(FunctionMode::New) {
        let constructor_head_comments =
            constructor_parameter_head_boundary_comments(f.context(), node_id);
        if let Some((first_comment, remaining_comments)) = constructor_head_comments.split_first() {
            format_raw_comment(f, *first_comment)?;
            write_raw_comment_slice(f, remaining_comments)?;
        }
        write!(f, [block_infix_annotations(f.context(), node_id)])?;
    }

    // declaration name boundary
    if signature.kind == FunctionKind::Function && name.is_some() {
        write!(f, [block_infix_annotations(f.context(), node_id)])?;
    }

    // name / key
    if signature.kind == FunctionKind::Function
        && let Some(name) = name
    {
        write!(f, [name])?;
    }

    // generic parameters
    if signature.kind == FunctionKind::Lambda && options.grouped_call_argument_layout.is_some() {
        let generic_parameters = format_with(|f| {
            write_function_generic_parameters(f, signature)?;
            Ok(())
        });
        let interned = f.intern_with_comment_snapshot(&generic_parameters)?;

        if let Some(interned) = interned {
            let mut buffer = RemoveSoftLinesBuffer::new(f);
            buffer.write_node(interned);
        }
    } else {
        write_function_generic_parameters(f, signature)?;
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
        &parameters,
        can_omit_parens,
        options,
    )?;

    // where clause
    let where_clauses = signature.where_clauses.as_slice();
    if !where_clauses.is_empty() {
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
    export: Option<ExportMode>,
    ambient: Ambientness,
    name: Option<Name>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    format_function_declaration_with_options(
        f,
        node_id,
        export,
        ambient,
        name,
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
    export: Option<ExportMode>,
    ambient: Ambientness,
    name: Option<Name>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    options: FormatFunctionDeclarationOptions,
) -> FormatResult<()> {
    // export
    write_function_export_prefix(f, export)?;

    // ambient
    write_function_ambient_prefix(f, ambient)?;

    // head
    write_function_head(f, node_id, name, signature, body, options)?;

    // body and terminator
    write_function_body_and_terminator(f, node_id, export, signature, body, options)
}
