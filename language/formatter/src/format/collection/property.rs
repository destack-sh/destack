use crate::format::annotation::{
    block_infix_annotations, decorator_prefix_annotations, format_raw_comment,
    infix_or_postfix_annotations, line_suffix_boundary_annotations, postfix_annotations,
    postfix_annotations_without_line_suffix_boundary, prefix_annotations_without_decorators,
};
use crate::format::chain::transparent_inner_expression;
use crate::format::declaration::is_poorly_breakable_member_or_call_chain;
use crate::format::declaration::signature::{
    default_static_parameter_trailing_separator, expression_body_requires_head_space,
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
    format_where_clause_with_break, parameter_is_variadic, should_break_function_parameters,
    signature_return_type_has_line_suffix_boundary_annotation, single_parameter_should_hug,
    write_empty_parameter_list_with_interior_comments, write_function_header_prefix,
    write_signature_dynamic_parameter_list, write_signature_hug_parameter_list,
    write_static_parameter_list,
};
use crate::format::declaration::statement::write_block_body;
use crate::format::directive::{node_has_ignore_directive, write_ignored_node};
use crate::format::expression::write_expression_without_prefix_annotations;
use crate::format::operator::{
    write_colon_prefixed_type_annotation, write_type_expression_with_inline_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, BindingModifier, Comment, Expression, FunctionSignature, Key, Keyword,
    LocalNodeId, Name, Node, NodeTree, NodeTreeImpl, Property, ScalarLiteral, is_identifier_compat,
};
use destack_core::StringId;
use destack_fir::format::{FormatNodes, FormatResult, Formatter as FirFormatter, VecBuffer, text};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_workspace::{QuoteProperty, QuoteStyle};

impl<'ast> Format<DestackFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        format_name_with_quotes(f, *self, false)
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for Key {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        format_key_with_quotes(f, *self, false)
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}

/// Format a key with quote rules controls.
pub(crate) fn format_key_with_quotes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    key: Key,
    force_quote_keys: bool,
) -> FormatResult<()> {
    match key {
        Key::Name(name) => {
            format_name_with_quotes(f, name, force_quote_keys)?;
        }
        Key::Private(name) => {
            write!(f, [token("#")])?;
            write!(f, [name])?;
        }
        Key::Expression(expression) => {
            write!(f, [token("[")])?;
            write!(f, [expression])?;
            write!(f, [token("]")])?;
        }
        Key::NamedExpression { name, key } => {
            write!(f, [token("[")])?;
            write!(f, [name])?;
            write!(f, [token(":"), space()])?;
            write!(f, [key])?;
            write!(f, [token("]")])?;
        }
    }

    Ok(())
}

/// Check whether a string is an identifier safe to leave unquoted in JavaScript.
pub(crate) fn is_identifier_for_quotes(content: &str) -> bool {
    is_identifier_compat(content)
}

/// Format a name key while applying quote rules.
fn format_name_with_quotes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: Name,
    force_quote_keys: bool,
) -> FormatResult<()> {
    let context = f.context();
    let is_destack = context.options.language_type.is_destack();
    let mut quote_props = context.options.quote_props;
    if is_destack {
        quote_props = QuoteProperty::Preserve;
    }

    match name {
        Name::Identifier(string_id) => {
            let content = context.strings.get(string_id);
            if content.starts_with('#')
                && (context.options.language_type.is_javascript()
                    || context.options.language_type.is_typescript())
            {
                let name = content.strip_prefix('#').unwrap_or(content);
                if name.is_empty() {
                    write!(f, [token("#")])?;
                } else {
                    write!(f, [token("#"), text(name)])?;
                }
                return Ok(());
            }

            let should_quote = quote_props == QuoteProperty::Consistent && force_quote_keys;
            if should_quote {
                format_quoted_name(f, string_id)?;
            } else {
                string_id.format(f)?;
            }
        }
        Name::String(string_id) => {
            let content = context.strings.get(string_id);
            let is_ident = is_identifier_for_quotes(content);

            // preserve cannot roundtrip source quote intent because keyword-like keys
            // are normalized as string names in the parser model
            let should_quote = force_quote_keys || !is_ident;

            if should_quote {
                format_quoted_name(f, string_id)?;
            } else {
                string_id.format(f)?;
            }
        }
        Name::Number(string_id) => {
            string_id.format(f)?;
        }
    }

    Ok(())
}

/// Format a quoted key using the preferred quote character.
fn format_quoted_name<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    string_id: StringId,
) -> FormatResult<()> {
    let mut quote_style = f.context().options.quote_style;
    if quote_style == QuoteStyle::Semantic && !f.context().options.language_type.is_destack() {
        quote_style = QuoteStyle::Double;
    }
    let content = f.context().strings.get(string_id);
    let quote_char = quote_style.char_for(content);
    let quote_str = if quote_char == '"' { "\"" } else { "'" };

    write!(f, [token(quote_str), string_id, token(quote_str)])
}

/// Write a field type annotation.
#[inline]
fn write_field_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write_colon_prefixed_type_annotation(f, value)
}

/// The assignment-like layout used for field initializers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FieldLikeLayout {
    Fluid,
    BreakAfterOperator,
    NeverBreakAfterOperator,
}

/// Return the assignment-like layout for one field initializer.
fn field_like_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    default: LocalNodeId<Expression>,
    is_left_short: bool,
    left_may_break: bool,
) -> FieldLikeLayout {
    let default_id = transparent_inner_expression(f.context(), default);

    if is_poorly_breakable_member_or_call_chain(f, default_id) && !is_left_short {
        return FieldLikeLayout::BreakAfterOperator;
    }

    if !left_may_break
        && matches!(
            f.context().tree.get(default_id),
            Expression::Declaration(_)
                | Expression::TemplateExpression { .. }
                | Expression::ScalarLiteral(
                    ScalarLiteral::Boolean(_)
                        | ScalarLiteral::Integer(_)
                        | ScalarLiteral::Bigint(_)
                        | ScalarLiteral::Float(_)
                        | ScalarLiteral::String(_)
                )
        )
    {
        return FieldLikeLayout::NeverBreakAfterOperator;
    }

    FieldLikeLayout::Fluid
}

/// Write the shared left side of one field-like assignment shell.
fn write_field_like_left<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
    key: Option<Key>,
    value: Option<LocalNodeId<Expression>>,
    force_quote_keys: bool,
) -> FormatResult<()> {
    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;

    // key
    if let Some(key) = key {
        format_key_with_quotes(f, key, force_quote_keys)?;
    }

    // modifiers
    format_binding_modifiers_postfix_maybe(f, modifiers)?;

    // value
    if let Some(value) = value {
        write_field_type_annotation(f, value)?;
    }

    Ok(())
}

/// Format shared property or member field output.
pub(crate) fn format_field_like<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
    key: Option<Key>,
    value: Option<LocalNodeId<Expression>>,
    default: Option<LocalNodeId<Expression>>,
    force_quote_keys: bool,
) -> FormatResult<()> {
    // no initializer
    let Some(default) = default else {
        write_field_like_left(f, modifiers, key, value, force_quote_keys)?;
        return Ok(());
    };

    // left side
    let mut buffer = VecBuffer::new(f.state_mut());
    write_field_like_left(
        &mut FirFormatter::new(&mut buffer),
        modifiers,
        key,
        value,
        force_quote_keys,
    )?;
    let left_nodes = buffer.into_vec();
    let left_may_break = left_nodes.will_break();
    let is_left_short = left_nodes
        .single_line_width()
        .is_some_and(|width| width < (u32::from(f.context().options.indent_width) + 3));
    let layout = field_like_layout(f, default, is_left_short, left_may_break);

    let left = f.intern_vec(left_nodes);
    let left = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        if let Some(left) = &left {
            f.write_node(left.clone());
        }

        Ok(())
    });

    let right = format_with(|f: &mut DestackFormatter<'ast, '_>| write!(f, [default]));
    let inner_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if left_may_break {
            write!(f, [left])?;
        } else {
            write!(f, [group(&left)])?;
        }

        write!(f, [space(), token("=")])?;

        match layout {
            FieldLikeLayout::Fluid => {
                let group_id = f.group_id("field_like_rhs");
                write!(
                    f,
                    [
                        group(&indent(&soft_line_break_or_space())).with_id(Some(group_id)),
                        line_suffix_boundary(),
                        indent_if_group_breaks(&right, group_id)
                    ]
                )
            }
            FieldLikeLayout::BreakAfterOperator => {
                write!(f, [group(&soft_line_indent_or_space(&right))])
            }
            FieldLikeLayout::NeverBreakAfterOperator => {
                write!(f, [space(), right])
            }
        }
    });

    write!(f, [group(&inner_content)])?;

    Ok(())
}

/// Format shared property or member method output.
pub(crate) fn format_method_like<'ast, N>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    modifiers: Option<BindingModifier>,
    key: Option<Key>,
    signature: &FunctionSignature,
    body: Option<LocalNodeId<Expression>>,
    force_quote_keys: bool,
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<N>,
{
    let generics = signature.generics.as_ref();
    let dynamic_parameters = method_dynamic_parameters(signature);

    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;

    // shared function header prefix
    write_function_header_prefix(f, signature, false, key.is_some())?;

    // key
    if let Some(key) = key {
        format_key_with_quotes(f, key, force_quote_keys)?;
    }

    // name boundary comments
    write!(f, [block_infix_annotations(f.context(), node_id)])?;

    // name postfix modifiers: `?` and `!` belong on the method name
    format_binding_modifiers_postfix_maybe(f, modifiers)?;

    // static parameters
    if let Some(static_parameters) =
        generics.and_then(|generics| generics.static_parameters.as_ref())
        && !static_parameters.is_empty()
    {
        write_static_parameter_list(
            f,
            static_parameters,
            default_static_parameter_trailing_separator(f),
        )?;
    }

    // optional marker to parameter list boundary
    write!(f, [block_infix_annotations(f.context(), node_id)])?;

    write_method_parameters_and_return_type(f, node_id, signature, &dynamic_parameters)?;

    // where clauses
    if let Some(where_clauses) = generics.and_then(|generics| generics.where_clauses.as_ref())
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    // body
    if let Some(body) = body {
        write_method_signature_boundary_and_body(f, node_id, signature, body)?;
    }

    Ok(())
}

/// Collect dynamic method parameters, including `this`.
fn method_dynamic_parameters(
    signature: &FunctionSignature,
) -> Vec<LocalNodeId<destack_ast::Parameter>> {
    let mut dynamic_parameters = Vec::with_capacity(signature.dynamic_parameters.len() + 1);

    if let Some(this_parameter) = signature.this_parameter {
        dynamic_parameters.push(this_parameter);
    }

    dynamic_parameters.extend(signature.dynamic_parameters.iter().copied());
    dynamic_parameters
}

/// Write one method parameter list and return type.
fn write_method_parameters_and_return_type<'ast, N>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    signature: &FunctionSignature,
    dynamic_parameters: &[LocalNodeId<destack_ast::Parameter>],
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<N>,
{
    let format_parameters_and_return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let should_expand_parameters =
            should_break_function_parameters(f.context(), dynamic_parameters);
        if dynamic_parameters.is_empty() {
            write_empty_parameter_list_with_interior_comments(f, node_id)?;
        } else if dynamic_parameters.len() == 1
            && !should_expand_parameters
            && single_parameter_should_hug(f.context(), dynamic_parameters[0])
        {
            write_signature_hug_parameter_list(f, dynamic_parameters)?;
        } else {
            let disallow_trailing_parameter_separator = dynamic_parameters
                .last()
                .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id));
            write_signature_dynamic_parameter_list(
                f,
                dynamic_parameters,
                should_expand_parameters,
                disallow_trailing_parameter_separator,
            )?;
        }

        if let Some(return_type) = signature.return_type {
            write!(f, [token(":"), space()])?;
            write_type_expression_with_inline_prefix_annotations(f, return_type)?;
        }

        Ok(())
    });

    write!(f, [group(&format_parameters_and_return_type)])
}

/// Write one method body after the signature boundary.
fn write_method_signature_boundary_and_body<'ast, N>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    signature: &FunctionSignature,
    body: LocalNodeId<Expression>,
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<N>,
{
    write!(
        f,
        [postfix_annotations_without_line_suffix_boundary(
            f.context(),
            node_id
        )]
    )?;

    if let Some(return_type) = signature.return_type {
        write!(
            f,
            [
                postfix_annotations_without_line_suffix_boundary::<Expression>(
                    f.context(),
                    return_type
                )
            ]
        )?;
    }

    let has_signature_line_boundary_annotation =
        f.context()
            .annotation_ids(node_id)
            .iter()
            .any(|annotation_id| {
                matches!(
                    f.context().annotation(*annotation_id).position(),
                    AnnotationPosition::LinePostfixBoundary
                )
            })
            || signature_return_type_has_line_suffix_boundary_annotation(
                f.context(),
                signature.return_type,
            );

    write!(f, [line_suffix_boundary_annotations(f.context(), node_id)])?;

    if let Some(return_type) = signature.return_type {
        write!(
            f,
            [line_suffix_boundary_annotations::<Expression>(
                f.context(),
                return_type
            )]
        )?;
    }

    write_method_body(
        f,
        body,
        has_signature_line_boundary_annotation,
        signature.return_type,
    )
}

/// Return raw comments between one method signature and its body.
fn method_body_boundary_comment_nodes<'ast>(
    f: &DestackFormatter<'ast, '_>,
    body_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let body_span = f.context().span(body_id);
    let Some(previous_token) = f.context().previous_non_trivia_token_before_span(body_span) else {
        return Vec::new();
    };
    if previous_token.span.file != body_span.file || previous_token.span.end >= body_span.start {
        return Vec::new();
    }

    {
        let comments = f.context().comments();
        comments
            .comments_in_range(previous_token.span.end, body_span.start)
            .to_vec()
    }
}

/// Write one method body after the signature boundary has been resolved.
fn write_method_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    body: LocalNodeId<Expression>,
    force_break_before_body: bool,
    _return_type: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let is_block_body = matches!(f.context().node::<Expression>(body), Expression::Block(..));
    let block_boundary_comment_nodes = method_body_boundary_comment_nodes(f, body)
        .into_iter()
        .filter(|comment| !is_block_body || !comment.followed_by_newline())
        .collect::<Vec<_>>();

    if force_break_before_body {
        write!(f, [hard_line_break()])?;
    }

    if !block_boundary_comment_nodes.is_empty() {
        write!(f, [space()])?;

        for (index, comment) in block_boundary_comment_nodes.iter().copied().enumerate() {
            let comment_span = comment.span;
            format_raw_comment(f, comment)?;

            let is_last = index + 1 == block_boundary_comment_nodes.len();
            if !is_last
                || f.context()
                    .span_has_newline_before_next_non_whitespace_token(comment_span)
            {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [space()])?;
            }
        }

        if is_block_body {
            return write_expression_without_prefix_annotations(f, body);
        }

        return write!(f, [body]);
    }

    if expression_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }

    let body_block_id = match f.context().node::<Expression>(body) {
        Expression::Block(block_id) => Some(*block_id),
        _ => None,
    };

    if let Some(block_id) = body_block_id {
        write_block_body(f, block_id)?;
        write!(
            f,
            [infix_or_postfix_annotations::<Expression>(
                f.context(),
                body
            )]
        )?;
    } else {
        write!(f, [body])?;
    }

    Ok(())
}

/// Format one node with shared directive handling and trailing annotation ownership.
pub(crate) fn format_node_with_directive<'ast, T, F>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    owns_infix_annotations: bool,
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
    F: FnMut(&mut DestackFormatter<'ast, '_>) -> FormatResult<()>,
{
    let is_ignored = node_has_ignore_directive(f.context(), node_id);
    write!(
        f,
        [prefix_annotations_without_decorators(f.context(), node_id)]
    )?;
    write!(f, [decorator_prefix_annotations(f.context(), node_id)])?;

    if is_ignored {
        write_ignored_node(f, node_id)?;
        if owns_infix_annotations {
            write!(f, [postfix_annotations(f.context(), node_id)])?;
        } else {
            write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;
        }
        return Ok(());
    }

    format_node(f)?;
    if owns_infix_annotations {
        write!(f, [postfix_annotations(f.context(), node_id)])?;
    } else {
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Property> for Property {
    fn format_node(
        &self,
        node_id: LocalNodeId<Property>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Property::Method {
            modifiers,
            key,
            signature,
            body,
        } = self
        {
            return format_node_with_directive(f, node_id, true, |f| {
                format_method_like(f, node_id, *modifiers, *key, signature, *body, false)
            });
        }

        format_node_with_directive(f, node_id, false, |f| {
            match self {
                Property::Field {
                    modifiers,
                    key,
                    value,
                    default,
                } => {
                    format_field_like(f, *modifiers, *key, *value, *default, false)?;
                }
                Property::Spread { modifiers, value } => {
                    // modifiers
                    format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                    // keyword
                    write!(f, [token("...")])?;
                    // value
                    write!(f, [value])?;
                    // modifiers
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                }
                Property::Method { .. } => {}
                Property::Error => {
                    write!(f, [token("/* ERROR */")])?;
                }
            }

            Ok(())
        })
    }
}
