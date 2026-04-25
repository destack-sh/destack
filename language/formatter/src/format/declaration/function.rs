use super::lambda::{FunctionCacheMode, write_lambda_arrow_with_infix_annotations};
use crate::format::annotation::{block_infix_annotations, postfix_annotations};
use crate::format::call::expression_is_test_call;
use crate::format::collection::TrailingSeparator;
use crate::format::context::MemoizeFormatExt;
use crate::format::declaration::declaration::write_declaration_export_head_comments;
use crate::format::declaration::signature::{
    default_generic_parameter_trailing_separator, expression_body_requires_head_space,
    format_where_clause_with_break, parameter_is_variadic, should_hug_function_parameters,
    write_empty_parameter_list_with_interior_comments, write_function_header_prefix,
    write_generic_parameter_list, write_signature_hug_parameter_list,
    write_signature_parameter_list, write_signature_return_type,
};
use crate::format::declaration::statement::format_block;
use crate::format::operator::write_type_expression_with_inline_prefix_annotations;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Ambientness, Argument, Declaration, ExportMode, Expression, FunctionCardinality, FunctionKind,
    FunctionSignature, GenericParameter, Keyword, LocalNodeId, Name, NodeType, Parameter,
    TypeExpression,
};
use destack_fir::format::{FormatNodes, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};
use destack_workspace::ArrowParentheses;

/// The cached content wrapper keyed by source span.
pub(crate) struct FormatContentWithCacheMode<T> {
    /// The source span used as the cache key.
    key: Span,
    /// The content to format.
    content: T,
    /// The cache policy for this payload.
    cache_mode: FunctionCacheMode,
}

impl<T> FormatContentWithCacheMode<T> {
    /// Construct one cached content wrapper.
    pub(crate) fn new(key: Span, content: T, cache_mode: FunctionCacheMode) -> Self {
        Self {
            key,
            content,
            cache_mode,
        }
    }
}

impl<'ast, T> Format<DestackFormatContext<'ast>> for FormatContentWithCacheMode<T>
where
    T: Format<DestackFormatContext<'ast>>,
{
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        // uncached
        if self.cache_mode == FunctionCacheMode::NoCache {
            return self.content.format(f);
        }

        // cached
        if let Some(cached) = f.context().get_cached_element(&self.key) {
            f.write_node(cached);
            return Ok(());
        }

        // fresh
        let Some(interned) = f.intern(&self.content)? else {
            return Ok(());
        };

        f.context_mut().cache_element(&self.key, interned.clone());
        f.write_node(interned);

        Ok(())
    }
}

/// Return whether one file name uses one module extension.
fn file_uses_module_only_extension(file_name: &str) -> bool {
    file_name.ends_with(".mts") || file_name.ends_with(".cts")
}

/// Write one declaration export prefix.
pub(crate) fn write_function_export_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: Option<ExportMode>,
) -> FormatResult<()> {
    match export {
        // named export
        Some(ExportMode::Named) => {
            write!(f, [Keyword::Export, space()])?;
            write_declaration_export_head_comments(f, node_id, ExportMode::Named)?;
        }
        // default export
        Some(ExportMode::Default) => {
            write!(f, [Keyword::Export, space(), Keyword::Default, space()])?;
            write_declaration_export_head_comments(f, node_id, ExportMode::Default)?;
        }
        // local declaration
        None => {}
    }

    Ok(())
}

/// Write one declaration ambient prefix.
pub(crate) fn write_function_ambient_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    ambient: Ambientness,
) -> FormatResult<()> {
    // ambient
    if ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    Ok(())
}

/// Return whether one function declaration is one direct test-call callback argument.
fn function_declaration_is_test_call_argument(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> bool {
    // declaration expression parent
    let Some((declaration_expression_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }
    let declaration_expression_id = LocalNodeId::<Expression>::new(declaration_expression_id);

    // argument parent
    let Some((argument_id, parent_type)) = context.parent(declaration_expression_id) else {
        return false;
    };
    if parent_type != NodeType::Argument {
        return false;
    }
    let argument_id = LocalNodeId::<Argument>::new(argument_id);

    // call expression parent
    let Some((call_expression_id, parent_type)) = context.parent(argument_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }
    let call_expression_id = LocalNodeId::<Expression>::new(call_expression_id);

    expression_is_test_call(context, call_expression_id)
}

/// Return the parameter container span for one function-like declaration.
pub(crate) fn function_parameter_container_span(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> Span {
    context
        .tree
        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Parameters))
        .unwrap_or_else(|| unreachable!("function declaration should own its parameter container"))
}

/// Collect parameters, including `this`.
pub(crate) fn function_parameters(signature: &FunctionSignature) -> Vec<LocalNodeId<Parameter>> {
    // allocate the combined list
    let mut parameters = Vec::with_capacity(signature.parameters.len() + 1);

    // include `this` first
    if let Some(this_parameter) = signature.this_parameter {
        parameters.push(this_parameter);
    }

    // append ordinary parameters
    parameters.extend(signature.parameters.iter().copied());

    parameters
}

/// Return whether one lambda can omit parentheses around its single parameter.
pub(crate) fn function_can_omit_lambda_parameter_parentheses(
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

/// Return whether one generic parameter is simple enough for grouped function parameters.
fn function_grouping_generic_parameter_is_plain(
    context: &DestackFormatContext<'_>,
    generic_parameter_id: LocalNodeId<GenericParameter>,
) -> bool {
    match context.tree.get(generic_parameter_id) {
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
    }
}

/// Return whether one function-like head should group its parameter container first.
pub(crate) fn should_group_function_parameters(
    f: &mut DestackFormatter<'_, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    parameter_count: usize,
) -> FormatResult<bool> {
    match signature.generic_parameters.as_slice() {
        [] => {}
        [generic_parameter_id]
            if function_grouping_generic_parameter_is_plain(f.context(), *generic_parameter_id) => {
        }
        _ => return Ok(false),
    }

    let Some(return_type) = signature.return_type else {
        return Ok(false);
    };
    if parameter_count != 1 {
        return Ok(false);
    }

    if matches!(
        f.context().tree.get(return_type),
        TypeExpression::Object { .. } | TypeExpression::Mapped { .. }
    ) {
        return Ok(true);
    }

    let format_return_type = format_with(|f: &mut DestackFormatter<'_, '_>| {
        write_function_return_type(f, node_id, signature, &None, &[])
    })
    .memoized();
    let will_break = format_return_type
        .inspect(f)?
        .is_some_and(|content| content.will_break());

    Ok(will_break)
}

/// Write one function generic parameter list.
pub(crate) fn write_function_generic_parameters<'ast>(
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
pub(crate) fn write_function_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    parameters: &[LocalNodeId<Parameter>],
    can_omit_parens: bool,
) -> FormatResult<()> {
    if parameters.is_empty() {
        return write_empty_parameter_list_with_interior_comments(f, node_id);
    }

    if can_omit_parens {
        return write!(f, [&parameters[0]]);
    }

    if should_hug_function_parameters(f.context(), parameters, can_omit_parens) {
        return write_signature_hug_parameter_list(f, parameters);
    }

    if function_declaration_is_test_call_argument(f.context(), node_id) {
        return write_signature_hug_parameter_list(f, parameters);
    }

    let disallow_trailing_parameter_separator = parameters
        .last()
        .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id))
        || (signature.kind == FunctionKind::Lambda && parameters.len() == 1);

    write_signature_parameter_list(f, parameters, disallow_trailing_parameter_separator)
}

/// Write one function return type.
pub(crate) fn write_function_return_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    _parameters: &[LocalNodeId<Parameter>],
) -> FormatResult<()> {
    let Some(return_type) = signature.return_type else {
        return Ok(());
    };

    // lambda type head
    if signature.kind == FunctionKind::Lambda && body.is_none() {
        write_lambda_arrow_with_infix_annotations(f, node_id, FunctionCacheMode::NoCache)?;
        write!(f, [space()])?;
        return write_type_expression_with_inline_prefix_annotations(f, return_type);
    }

    write_signature_return_type(f, node_id, return_type)
}

/// Write one function parameter list and return type.
fn write_function_parameters_and_return_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    parameters: &[LocalNodeId<Parameter>],
    can_omit_parens: bool,
    cache_mode: FunctionCacheMode,
) -> FormatResult<()> {
    let group_parameters =
        should_group_function_parameters(f, node_id, signature, parameters.len())?;
    let format_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_function_parameters(f, node_id, signature, parameters, can_omit_parens)
    });
    let format_parameters = FormatContentWithCacheMode::new(
        function_parameter_container_span(f.context(), node_id),
        format_parameters,
        cache_mode,
    );

    if group_parameters {
        write!(f, [group(&format_parameters)])?;
    } else {
        write!(f, [format_parameters])?;
    }

    write_function_return_type(f, node_id, signature, body, parameters)
}

/// Write one non-lambda function body.
fn write_function_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _signature: &FunctionSignature,
    body: LocalNodeId<Expression>,
    cache_mode: FunctionCacheMode,
) -> FormatResult<()> {
    let body_span = f.context().span(body);
    let body_content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        if expression_body_requires_head_space(f.context(), body) {
            write!(f, [space()])?;
        }

        if let Expression::Block(block_id) = f.context().tree.get(body) {
            return format_block(f, *block_id);
        }

        write!(f, [body])
    });

    FormatContentWithCacheMode::new(body_span, body_content, cache_mode).format(f)
}

/// Write one function body and trailing semicolon.
fn write_function_body_and_terminator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    cache_mode: FunctionCacheMode,
) -> FormatResult<()> {
    // body
    if let Some(body) = body {
        write_function_body(f, signature, *body, cache_mode)?;
    }

    // terminator
    write!(f, [postfix_annotations(f.context(), node_id)])?;

    if body.is_none() {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Write one function head before the body.
fn write_function_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    name: Option<Name>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    cache_mode: FunctionCacheMode,
) -> FormatResult<()> {
    let parameters = function_parameters(signature);

    // head prefix
    let head_prefix = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_function_header_prefix(f, signature, true, name.is_some())?;

        if name.is_some() {
            write!(f, [block_infix_annotations(f.context(), node_id)])?;
        }

        if let Some(name) = name {
            write!(f, [name])?;
        }

        write_function_generic_parameters(f, signature)?;

        Ok(())
    });
    let head_prefix =
        FormatContentWithCacheMode::new(f.context().span(node_id), head_prefix, cache_mode);
    write!(f, [head_prefix])?;

    // parameter separator
    write!(f, [block_infix_annotations(f.context(), node_id)])?;

    // signature
    let format_signature = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_function_parameters_and_return_type(
            f,
            node_id,
            signature,
            body,
            &parameters,
            false,
            cache_mode,
        )
    });

    write!(f, [group(&format_signature)])?;

    // where clause
    let where_clauses = signature.where_clauses.as_slice();
    if !where_clauses.is_empty() {
        format_where_clause_with_break(f, where_clauses)?;
    }

    Ok(())
}

/// Format one function declaration.
pub(crate) fn format_function_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: Option<ExportMode>,
    ambient: Ambientness,
    name: Option<Name>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    cache_mode: FunctionCacheMode,
) -> FormatResult<()> {
    debug_assert_eq!(signature.kind, FunctionKind::Function);

    write_function_export_prefix(f, node_id, export)?;
    write_function_ambient_prefix(f, ambient)?;
    write_function_head(f, node_id, name, signature, body, cache_mode)?;
    write_function_body_and_terminator(f, node_id, signature, body, cache_mode)
}
