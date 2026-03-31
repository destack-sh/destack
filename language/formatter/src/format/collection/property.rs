use crate::format::chain::transparent_inner_expression;
use crate::format::collection::TrailingSeparator;
use crate::format::declaration::signature::{
    expression_body_requires_head_space, format_binding_modifiers_postfix_maybe,
    format_binding_modifiers_prefix_maybe, format_where_clause_with_break, parameter_is_variadic,
    signature_parameters_should_expand, signature_return_type_has_line_postfix_boundary_annotation,
    signature_should_elide_space_before_body, single_parameter_should_hug,
    write_empty_parameter_list_with_interior_comments, write_function_header_prefix,
    write_signature_dynamic_parameter_list, write_static_parameter_list,
};
use crate::format::declaration::statement::format_block;
use crate::format::directive::{node_has_ignore_directive, write_ignored_node};
use crate::format::expression::{
    is_complex_expression, is_expression_breakable, is_trivial_expression,
};
use crate::format::operator::write_expression_with_inline_prefix_annotations;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AccessorKind, Argument, BinaryOperator, BindingModifier, Comment, Expression,
    FunctionSignature, Key, Keyword, LocalNodeId, Name, Node, NodeTree, NodeTreeImpl, Property,
    is_identifier,
};
use destack_core::StringId;
use destack_fir::format::{FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::{QuoteProperty, QuoteStyle};

/// Return whether one property value is complex enough to expand.
pub(crate) fn property_has_complex_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    // annotations on the property force complexity
    if context.has_annotation(property_id) {
        return true;
    }

    let property = tree.get(property_id);

    // field values and defaults can be complex
    if let Property::Field { value, default, .. } = property {
        let value_is_complex = value.is_some_and(|value_id| {
            let value_expr = tree.get(value_id);
            is_complex_expression(tree, value_expr) || context.has_annotation(value_id)
        });

        let default_is_complex = default.is_some_and(|default_id| {
            let default_expr = tree.get(default_id);
            is_complex_expression(tree, default_expr) || context.has_annotation(default_id)
        });

        return value_is_complex || default_is_complex;
    }

    // methods with bodies are always complex in object literals
    if let Property::Method { body, .. } = property {
        return body.is_some();
    }

    // spread properties inherit complexity from their value
    if let Property::Spread { value, .. } = property {
        let value_expr = tree.get(*value);
        return is_complex_expression(tree, value_expr) || context.has_annotation(*value);
    }

    false
}

/// Return whether one property contains a complex type value.
pub(crate) fn property_has_complex_type_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    if context.has_annotation(property_id) {
        return true;
    }

    match tree.get(property_id) {
        Property::Field { value, default, .. } => {
            value.is_some_and(|expression_id| {
                let expression = tree.get(expression_id);
                context.has_annotation(expression_id)
                    || is_expression_breakable(tree, expression)
                    || !is_trivial_expression(tree, expression)
            }) || default.is_some_and(|expression_id| {
                let expression = tree.get(expression_id);
                context.has_annotation(expression_id)
                    || is_expression_breakable(tree, expression)
                    || !is_trivial_expression(tree, expression)
            })
        }
        Property::Method { body, .. } => body.is_some(),
        Property::Spread { value, .. } => {
            let expression = tree.get(*value);
            context.has_annotation(*value)
                || is_expression_breakable(tree, expression)
                || !is_trivial_expression(tree, expression)
        }
        Property::Error => true,
    }
}

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
    if contains_katakana_middle_dot(content) {
        return false;
    }
    is_identifier(content)
}

/// Check whether a string contains katakana middle dot characters.
fn contains_katakana_middle_dot(content: &str) -> bool {
    content
        .chars()
        .any(|c| matches!(c, '\u{30FB}' | '\u{FF65}'))
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

/// Return whether method signature source spans multiple lines before the body.
pub(crate) fn method_signature_is_multiline_before_body(
    context: &DestackFormatContext<'_>,
    node_span: Span,
    body: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(body_id) = body else {
        return false;
    };

    let body_span = context.span(body_id);
    if node_span.file != body_span.file || node_span.start >= body_span.start {
        return false;
    }

    let signature_span = Span::new(node_span.file, node_span.start, body_span.start);
    context.has_newline(signature_span)
}

/// Return whether one expression subtree contains a type union or intersection binary.
fn expression_contains_type_binary(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut pending = vec![expression_id];
    while let Some(next_expression_id) = pending.pop() {
        let next_expression_id = transparent_inner_expression(context, next_expression_id);
        match context.tree.get(next_expression_id) {
            Expression::Binary {
                operator: BinaryOperator::ElementwiseOr | BinaryOperator::ElementwiseAnd,
                ..
            } => return true,
            Expression::Binary { left, right, .. } => {
                pending.push(*left);
                pending.push(*right);
            }
            Expression::Path {
                static_arguments: Some(static_arguments),
                ..
            }
            | Expression::Member {
                static_arguments: Some(static_arguments),
                ..
            }
            | Expression::TypeImport {
                static_arguments: Some(static_arguments),
                ..
            } => {
                for argument_id in static_arguments.iter().copied() {
                    let argument_value_id = match context.tree.get(argument_id) {
                        Argument::Named { value, .. }
                        | Argument::Labeled { value, .. }
                        | Argument::Positional { value, .. }
                        | Argument::Spread { value, .. } => *value,
                        Argument::Error => continue,
                    };
                    pending.push(argument_value_id);
                }
            }
            Expression::TypeConditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                pending.push(*left);
                pending.push(*right);
                pending.push(*then_type);
                pending.push(*else_type);
            }
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                pending.push(*expression);
            }
            _ => {}
        }
    }

    false
}

/// Return whether one method return type contains any type union or intersection.
fn signature_return_type_is_union_or_intersection(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(return_type_id) = return_type else {
        return false;
    };

    expression_contains_type_binary(context, return_type_id)
}

/// Write a field type annotation.
#[inline]
fn write_field_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    write!(f, [token(":"), space()])?;
    write_expression_with_inline_prefix_annotations(f, value)
}

/// Return whether one expression carries any static generic arguments.
fn field_expression_has_static_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_deref()
            .is_some_and(|arguments| !arguments.is_empty()),
        Expression::Call {
            left,
            static_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            ..
        }
        | Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            field_expression_has_static_arguments(context, *left)
                || static_arguments
                    .as_deref()
                    .is_some_and(|arguments| !arguments.is_empty())
        }
        Expression::Index { left, .. } => field_expression_has_static_arguments(context, *left),
        Expression::Instantiation {
            left,
            static_arguments,
        } => field_expression_has_static_arguments(context, *left) || !static_arguments.is_empty(),
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            field_expression_has_static_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether one expression contains multiple chained call-like operations.
fn field_expression_has_nested_call_chain(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id;
    let mut call_like_count = 0usize;

    loop {
        match context.tree.get(current_id) {
            Expression::Call { left, .. }
            | Expression::New { left, .. }
            | Expression::Instantiation { left, .. } => {
                call_like_count += 1;
                if call_like_count >= 2 {
                    return true;
                }

                current_id = *left;
            }
            Expression::Member { left, .. }
            | Expression::PrivateMember { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. }
            | Expression::Must { left, .. } => {
                current_id = *left;
            }
            Expression::Parenthesized { expression } | Expression::Statement(expression) => {
                current_id = *expression;
            }
            _ => return false,
        }
    }
}

/// Return whether one expression is a single call-like value whose callee receiver is another member chain.
fn field_expression_is_single_call_with_member_chain_callee(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let callee_id = match context.tree.get(expression_id) {
        Expression::Call { left, .. } | Expression::New { left, .. } => *left,
        _ => return false,
    };

    let receiver_id = match context.tree.get(callee_id) {
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. } => *left,
        _ => return false,
    };

    matches!(
        context.tree.get(receiver_id),
        Expression::Member { .. }
            | Expression::PrivateMember { .. }
            | Expression::Index { .. }
            | Expression::Maybe { .. }
            | Expression::Must { .. }
    )
}

/// Return whether one static argument list contains block-like type expressions.
fn field_static_argument_list_has_block_expressions(
    context: &DestackFormatContext<'_>,
    static_arguments: &[LocalNodeId<Argument>],
) -> bool {
    static_arguments.iter().copied().any(|argument_id| {
        let (Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. }) = context.tree.get(argument_id)
        else {
            return false;
        };

        field_expression_has_complex_nested_static_arguments(context, *value)
    })
}

/// Return whether one expression carries nested block-like static arguments.
fn field_expression_has_complex_nested_static_arguments(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. } => true,
        Expression::Path {
            static_arguments, ..
        } => static_arguments.as_deref().is_some_and(|arguments| {
            field_static_argument_list_has_block_expressions(context, arguments)
        }),
        Expression::Call {
            left,
            static_arguments,
            ..
        }
        | Expression::New {
            left,
            static_arguments,
            ..
        }
        | Expression::Member {
            left,
            static_arguments,
            ..
        }
        | Expression::PrivateMember {
            left,
            static_arguments,
            ..
        } => {
            field_expression_has_complex_nested_static_arguments(context, *left)
                || static_arguments.as_deref().is_some_and(|arguments| {
                    field_static_argument_list_has_block_expressions(context, arguments)
                })
        }
        Expression::Index { left, .. } => {
            field_expression_has_complex_nested_static_arguments(context, *left)
        }
        Expression::Instantiation {
            left,
            static_arguments,
        } => {
            field_expression_has_complex_nested_static_arguments(context, *left)
                || field_static_argument_list_has_block_expressions(context, static_arguments)
        }
        Expression::Parenthesized { expression } | Expression::Statement(expression) => {
            field_expression_has_complex_nested_static_arguments(context, *expression)
        }
        _ => false,
    }
}

/// Return whether one field initializer should break after `=`.
fn field_default_should_break_after_operator(
    f: &DestackFormatter<'_, '_>,
    modifiers: Option<BindingModifier>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if f.context().has_prefix_annotation(expression_id) {
        return false;
    }

    let value_has_static_arguments =
        field_expression_has_static_arguments(f.context(), expression_id);
    let value_has_nested_call_chain =
        field_expression_has_nested_call_chain(f.context(), expression_id);
    let value_has_block_static_arguments =
        field_expression_has_complex_nested_static_arguments(f.context(), expression_id);
    let value_has_dynamic_arguments = matches!(
        f.context().tree.get(expression_id),
        Expression::Call {
            dynamic_arguments, ..
        } | Expression::New {
            dynamic_arguments, ..
        } if !dynamic_arguments.is_empty()
    );
    let is_accessor_field =
        modifiers.is_some_and(|modifiers| modifiers.accessor == Some(AccessorKind::Accessor));
    let line_width = f.context().options.line_width;

    if is_accessor_field
        && matches!(
            f.context().tree.get(expression_id),
            Expression::Call { .. } | Expression::New { .. }
        )
    {
        return value_has_dynamic_arguments;
    }

    if value_has_block_static_arguments && !value_has_dynamic_arguments {
        return false;
    }

    value_has_static_arguments && !value_has_nested_call_chain
        || line_width <= 80
            && field_expression_is_single_call_with_member_chain_callee(f.context(), expression_id)
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
    // default
    if let Some(default) = default {
        if f.context().has_prefix_annotation(default)
            || field_default_should_break_after_operator(f, modifiers, default)
        {
            write!(
                f,
                [group(&format_args![
                    space(),
                    token("="),
                    indent(&format_args![soft_line_break_or_space(), default]),
                ])]
            )?;
        } else {
            write!(
                f,
                [group(&format_args![space(), token("="), space(), default])]
            )?;
        }
    }

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
    signature_is_multiline_before_body: bool,
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<N>,
{
    let generics = signature.generics.as_ref();

    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;

    // shared function header prefix
    write_function_header_prefix(f, signature, false, key.is_some())?;

    // key
    if let Some(key) = key {
        format_key_with_quotes(f, key, force_quote_keys)?;
    }

    // name seam comments
    write!(
        f,
        [crate::format::annotation::block_infix_annotations(
            f.context(),
            node_id
        )]
    )?;

    // name postfix modifiers: `?` and `!` belong on the method name
    format_binding_modifiers_postfix_maybe(f, modifiers)?;

    // static parameters
    if let Some(static_parameters) =
        generics.and_then(|generics| generics.static_parameters.as_ref())
        && !static_parameters.is_empty()
    {
        write_static_parameter_list(f, static_parameters, TrailingSeparator::Disallowed)?;
    }

    // optional marker to parameter list seam
    write!(
        f,
        [crate::format::annotation::block_infix_annotations(
            f.context(),
            node_id
        )]
    )?;

    // dynamic parameters
    let mut dynamic_parameters = Vec::with_capacity(signature.dynamic_parameters.len() + 1);
    if let Some(this_parameter) = signature.this_parameter {
        dynamic_parameters.push(this_parameter);
    }
    dynamic_parameters.extend(signature.dynamic_parameters.iter().copied());

    // dynamic parameter rendering
    let should_expand_parameters = signature_parameters_should_expand(
        f.context(),
        signature.mode,
        &dynamic_parameters,
        signature.return_type,
        false,
    ) || (signature_is_multiline_before_body
        && signature_return_type_is_union_or_intersection(f.context(), signature.return_type));
    if dynamic_parameters.is_empty() {
        write_empty_parameter_list_with_interior_comments(f, node_id)?;
    } else if dynamic_parameters.len() == 1
        && !should_expand_parameters
        && single_parameter_should_hug(f.context(), dynamic_parameters[0])
    {
        write!(f, [token("("), dynamic_parameters[0], token(")")])?;
    } else {
        let disallow_trailing_parameter_separator = dynamic_parameters
            .last()
            .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id));
        write_signature_dynamic_parameter_list(
            f,
            &dynamic_parameters,
            should_expand_parameters,
            disallow_trailing_parameter_separator,
        )?;
    }

    // return type
    if let Some(return_type) = signature.return_type {
        write!(f, [token(":"), space()])?;
        write_expression_with_inline_prefix_annotations(f, return_type)?;
    }

    // where clauses
    if let Some(where_clauses) = generics.and_then(|generics| generics.where_clauses.as_ref())
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }

    // body
    if let Some(body) = body {
        if signature_return_type_has_line_postfix_boundary_annotation(
            f.context(),
            signature.return_type,
        ) {
            write!(f, [hard_line_break(), body])?;
        } else if signature_should_elide_space_before_body(f.context(), signature.return_type) {
            write!(f, [body])?;
        } else {
            if expression_body_requires_head_space(f.context(), body) {
                write!(f, [space()])?;
            }
            let body_expression = f.context().node::<Expression>(body);
            if let Expression::Block(block_id) = body_expression {
                format_block(f, *block_id)?;
            } else {
                write!(f, [body])?;
            }
        }
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
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Comment>,
    F: FnMut(&mut DestackFormatter<'ast, '_>) -> FormatResult<()>,
{
    let is_ignored = node_has_ignore_directive(f.context(), node_id);
    write!(
        f,
        [crate::format::annotation::prefix_annotations(
            f.context(),
            node_id
        )]
    )?;

    if is_ignored {
        write_ignored_node(f, node_id)?;
        if owns_infix_annotations {
            write!(
                f,
                [crate::format::annotation::postfix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        } else {
            write!(
                f,
                [crate::format::annotation::infix_or_postfix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        }
        return Ok(());
    }

    format_node(f)?;
    if owns_infix_annotations {
        write!(
            f,
            [crate::format::annotation::postfix_annotations(
                f.context(),
                node_id
            )]
        )?;
    } else {
        write!(
            f,
            [crate::format::annotation::infix_or_postfix_annotations(
                f.context(),
                node_id
            )]
        )?;
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
                let signature_is_multiline_before_body = method_signature_is_multiline_before_body(
                    f.context(),
                    f.context().span(node_id),
                    *body,
                );
                format_method_like(
                    f,
                    node_id,
                    *modifiers,
                    *key,
                    signature,
                    *body,
                    false,
                    signature_is_multiline_before_body,
                )
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
