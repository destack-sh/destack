use crate::format::chain::transparent_inner_expression;
use crate::format::collection::list_like;
use crate::format::declaration::signature::{
    expression_body_requires_head_space, format_where_clause_with_break, parameter_is_variadic,
    signature_parameters_should_expand, signature_return_type_has_line_postfix_boundary_annotation,
    signature_should_elide_space_before_body, single_parameter_should_hug,
    write_empty_parameter_list_with_interior_annotations, write_function_header_prefix,
    write_signature_dynamic_parameter_list,
};
use crate::format::declaration::statement::format_block;
use crate::format::directive::{
    ignore_ranges_for_nodes, node_has_ignore_directive, write_ignored_node, write_ignored_span,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AbstractionModifier, AccessorKind, Argument, BinaryOperator, BindingAnchor, BindingKind,
    BindingModifier, BindingOperator, Comment, Declaration, DeclarationKind, Expression,
    FunctionSignature, Key, Keyword, LocalNodeId, Member, Mutability, Name, Node, NodeTree,
    NodeTreeImpl, NodeType, Parameter, Property, Timing, VarianceModifier, is_identifier,
};
use destack_core::StringId;
use destack_fir::format::{FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;
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

/// Format binding modifiers that appear before a name.
#[inline]
pub(crate) fn format_binding_modifiers_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // variance
    if let Some(variance) = modifiers.variance {
        match variance {
            VarianceModifier::In => write!(f, [token("in"), space()])?,
            VarianceModifier::Out => write!(f, [token("out"), space()])?,
            VarianceModifier::InOut => {
                write!(f, [token("in"), space(), token("out"), space()])?;
            }
        }
    }
    // visibility
    if let Some(visibility) = modifiers.visibility {
        write!(f, [visibility, space()])?;
    }
    // declaration
    if modifiers.declaration == Some(DeclarationKind::Declaration) {
        write!(f, [Keyword::Declare, space()])?;
    }
    // scope
    if modifiers.anchor == Some(BindingAnchor::Static) {
        write!(f, [Keyword::Static, space()])?;
    }
    // abstraction
    if let Some(abstraction) = modifiers.abstraction {
        match abstraction {
            AbstractionModifier::Abstract => write!(f, [Keyword::Abstract, space()])?,
            AbstractionModifier::Override => write!(f, [Keyword::Override, space()])?,
            AbstractionModifier::AbstractOverride => {
                write!(f, [Keyword::Abstract, space()])?;
                write!(f, [Keyword::Override, space()])?;
            }
        }
    }
    // mutability
    if modifiers.mutability == Some(Mutability::Immutable) {
        write!(f, [Keyword::Readonly, space()])?;
    }
    // operator
    if modifiers.operator == Some(BindingOperator::AsConst) {
        write!(f, [Keyword::Const, space()])?;
    }
    // accessor
    if modifiers.accessor == Some(AccessorKind::Accessor) {
        write!(f, [Keyword::Accessor, space()])?;
    }
    // timing
    if modifiers.timing == Some(Timing::Comptime) {
        write!(f, [Keyword::Comptime, space()])?;
    }
    Ok(())
}

/// Format optional binding modifiers before a name.
#[inline]
pub(crate) fn format_binding_modifiers_prefix_maybe<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_prefix(f, modifiers)?;
    }
    Ok(())
}

/// Format binding modifiers that appear after a name.
#[inline]
pub(crate) fn format_binding_modifiers_postfix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // kind
    if modifiers.kind == Some(BindingKind::Must) {
        write!(f, [token("!")])?;
    } else if modifiers.kind == Some(BindingKind::Maybe) {
        write!(f, [token("?")])?;
    }
    Ok(())
}

/// Format optional binding modifiers after a name.
#[inline]
pub(crate) fn format_binding_modifiers_postfix_maybe<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: Option<BindingModifier>,
) -> FormatResult<()> {
    if let Some(modifiers) = modifiers {
        format_binding_modifiers_postfix(f, modifiers)?;
    }
    Ok(())
}

/// Return whether method signature source spans multiple lines before the body.
fn method_signature_is_multiline_before_body(
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

/// Format a block of properties with empty-annotation and ignore-range handling.
#[allow(unused)]
pub(crate) fn format_block_of_properties<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    properties: &[LocalNodeId<Property>],
    separator: &'static str,
) -> FormatResult<()> {
    format_block_nodes_with_ignore_ranges(f, properties, |f, property_id| {
        let property = f.context().tree.get(property_id);
        property_id.format(f)?;

        // separators
        if matches!(
            property,
            Property::Field { .. } | Property::Method { .. } | Property::Spread { .. }
        ) {
            write!(f, [token(separator)])?;
        }

        Ok(())
    })
}

/// Format a block of members with empty-annotation and ignore-range handling.
#[allow(unused)]
pub(crate) fn format_block_of_members<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    format_block_nodes_with_ignore_ranges(f, members, |f, member_id| member_id.format(f))
}

/// Format a block of nodes while honoring ignore ranges and spacing.
fn format_block_nodes_with_ignore_ranges<'ast, T, F>(
    f: &mut DestackFormatter<'ast, '_>,
    node_ids: &[LocalNodeId<T>],
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Comment>,
    F: FnMut(&mut DestackFormatter<'ast, '_>, LocalNodeId<T>) -> FormatResult<()>,
{
    let comment_tokens = f.context().comment_tokens();
    let ignore_ranges = ignore_ranges_for_nodes(f.context(), node_ids, comment_tokens);

    let mut skip_until: Option<u32> = None;
    for (index, node_id) in node_ids.iter().copied().enumerate() {
        let node_span = f.context().span(node_id);

        if let Some(skip_end) = skip_until {
            if node_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between entries
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        if let Some(range_span) = ignore_ranges.get(&node_id.id) {
            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        format_node(f, node_id)?;
    }

    Ok(())
}

/// Return whether one property should force quoted keys.
#[inline]
fn should_force_quote_keys_for_property() -> bool {
    // object fields follow identifier quoting rules, even in consistent mode
    false
}

/// Return whether one member should force quoted keys.
#[inline]
fn should_force_quote_keys_for_member<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Member>,
) -> bool {
    if f.context().options.quote_props != QuoteProperty::Consistent {
        return false;
    }

    if f.context().options.language_type.is_destack() {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
        return false;
    }

    let parent_id = LocalNodeId::<Declaration>::new(parent_id);
    if !matches!(f.context().tree.get(parent_id), Declaration::Class { .. }) {
        return false;
    }

    true
}

/// Write a field type annotation.
#[inline]
fn write_field_type_annotation<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if f.context().has_prefix_annotation(value) {
        write!(f, [token(":"), indent(&format_args![space(), value])])
    } else {
        write!(f, [token(":"), space(), value])
    }
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
fn format_field_like<'ast>(
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
fn format_method_like<'ast, N>(
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
    write!(f, [f.context().block_infix_annotations(node_id)])?;

    // name postfix modifiers: `?` and `!` belong on the method name
    format_binding_modifiers_postfix_maybe(f, modifiers)?;

    // static parameters
    if let Some(static_parameters) =
        generics.and_then(|generics| generics.static_parameters.as_ref())
        && !static_parameters.is_empty()
    {
        write!(
            f,
            [list_like::<Parameter>("<", ">", ",", static_parameters)]
        )?;
    }

    // optional marker to parameter list seam
    write!(f, [f.context().block_infix_annotations(node_id)])?;

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
        write_empty_parameter_list_with_interior_annotations(f, node_id)?;
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
        write!(f, [token(":"), space(), return_type])?;
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

/// Format one method-like node with directive handling and method-local infix ownership.
fn format_method_node_with_directive<'ast, T, F>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Comment>,
    F: FnMut(&mut DestackFormatter<'ast, '_>) -> FormatResult<()>,
{
    let is_ignored = node_has_ignore_directive(f.context(), node_id);
    write!(f, [f.context().any_prefix_annotations(node_id)])?;

    if is_ignored {
        write_ignored_node(f, node_id)?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(());
    }

    format_node(f)?;
    write!(f, [f.context().any_postfix_annotations(node_id)])?;

    Ok(())
}

/// Format one node with shared directive handling and annotations.
fn format_node_with_directive<'ast, T, F>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T> + NodeTreeImpl<Comment>,
    F: FnMut(&mut DestackFormatter<'ast, '_>) -> FormatResult<()>,
{
    let is_ignored = node_has_ignore_directive(f.context(), node_id);
    write!(f, [f.context().any_prefix_annotations(node_id)])?;

    if is_ignored {
        write_ignored_node(f, node_id)?;
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        return Ok(());
    }

    format_node(f)?;
    write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

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
            return format_method_node_with_directive(f, node_id, |f| {
                let force_quote_keys = should_force_quote_keys_for_property();
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
                    force_quote_keys,
                    signature_is_multiline_before_body,
                )
            });
        }

        format_node_with_directive(f, node_id, |f| {
            match self {
                Property::Field {
                    modifiers,
                    key,
                    value,
                    default,
                } => {
                    let force_quote_keys = should_force_quote_keys_for_property();
                    format_field_like(f, *modifiers, *key, *value, *default, force_quote_keys)?;
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

impl<'ast> FormatNode<'ast, Member> for Member {
    fn format_node(
        &self,
        node_id: LocalNodeId<Member>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Member::Method {
            modifiers,
            key,
            signature,
            body,
        } = self
        {
            return format_method_node_with_directive(f, node_id, |f| {
                let force_quote_keys = should_force_quote_keys_for_member(f, node_id);
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
                    force_quote_keys,
                    signature_is_multiline_before_body,
                )?;

                if body.is_none() {
                    write!(f, [token(";")])?;
                }

                Ok(())
            });
        }

        let is_ignored = node_has_ignore_directive(f.context(), node_id);
        if is_ignored {
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
            write_ignored_node(f, node_id)?;
            write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

            // ignored class fields still get one formatter-owned terminator
            if matches!(self, Member::Field { .. }) {
                write!(f, [token(";")])?;
            }

            return Ok(());
        }

        format_node_with_directive(f, node_id, |f| {
            match self {
                Member::Type {
                    modifiers,
                    name,
                    static_parameters,
                    where_clauses,
                    ty,
                    value,
                } => {
                    // modifiers
                    format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                    // keyword
                    write!(f, [Keyword::Type, space()])?;
                    // name
                    write!(f, [name])?;
                    // static parameters
                    if let Some(static_parameters) = static_parameters
                        && !static_parameters.is_empty()
                    {
                        write!(f, [list_like("<", ">", ",", static_parameters)])?;
                    }
                    // where clauses
                    if let Some(where_clauses) = where_clauses
                        && !where_clauses.is_empty()
                    {
                        write!(f, [space(), Keyword::Where, space()])?;
                        write!(f, [list_like("", "", ",", where_clauses)])?;
                    }
                    // type bound
                    if let Some(ty) = ty {
                        write!(f, [token(":"), space(), ty])?;
                    }
                    // value
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), value])?;
                    }
                }
                Member::ComptimeConst {
                    modifiers,
                    name,
                    ty,
                    value,
                } => {
                    // keep non comptime modifiers before the associated keyword pair
                    if let Some(mut modifiers) = *modifiers {
                        modifiers.timing = None;
                        modifiers.operator = None;
                        format_binding_modifiers_prefix(f, modifiers)?;
                    }

                    // associated comptime constants are always emitted in canonical order
                    write!(
                        f,
                        [Keyword::Comptime, space(), Keyword::Const, space(), name]
                    )?;

                    // optional type annotation
                    if let Some(ty) = ty {
                        write!(f, [token(":"), space(), ty])?;
                    }

                    // optional initializer
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), value])?;
                    }
                }
                Member::Field {
                    modifiers,
                    key,
                    value,
                    default,
                } => {
                    let force_quote_keys = should_force_quote_keys_for_member(f, node_id);
                    format_field_like(f, *modifiers, *key, *value, *default, force_quote_keys)?;
                }
                Member::Embed { modifiers, value } => {
                    // modifiers
                    format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                    // keyword
                    write!(f, [token("...")])?;
                    // value
                    write!(f, [value])?;
                    // modifiers
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                }
                Member::StaticBlock { body, .. } => {
                    // keyword
                    write!(f, [Keyword::Static, space()])?;
                    // body
                    write!(f, [body])?;
                }
                Member::ComptimeBlock { modifiers, body } => {
                    // modifiers prefix
                    if let Some(mut modifiers) = *modifiers {
                        modifiers.timing = None;
                        format_binding_modifiers_prefix(f, modifiers)?;
                    }
                    // keyword
                    write!(f, [Keyword::Comptime, space()])?;
                    // body
                    write!(f, [body])?;
                }
                Member::Method { .. } => {}
                Member::Error => {
                    write!(f, [token("/* ERROR */")])?;
                }
            }

            let needs_semicolon = matches!(self, Member::Field { .. });
            if needs_semicolon {
                write!(f, [token(";")])?;
            }

            Ok(())
        })
    }
}
