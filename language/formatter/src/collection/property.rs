use crate::annotation::{
    block_infix_annotations, decorator_prefix_annotations, format_comment,
    infix_or_postfix_annotations, postfix_annotations, prefix_comments_before_decorators,
};
use crate::chain::transparent_inner_expression;
use crate::context::MemoizeFormatExt;
use crate::declaration::signature::{
    default_generic_parameter_trailing_separator, expression_body_requires_head_space,
    format_where_clause_with_break, parameter_is_variadic, should_break_function_parameters,
    should_hug_function_parameters, write_empty_parameter_list_with_interior_comments,
    write_function_header_prefix, write_generic_parameter_list,
    write_grouped_parameters_with_return_type, write_signature_hug_parameter_list,
    write_signature_parameter_list, write_signature_return_type,
};
use crate::declaration::statement::write_block_body;
use crate::expression::write_expression_without_prefix_annotations;
use crate::file::{node_has_ignore_directive, write_ignored_node};
use crate::operator::{
    is_poorly_breakable_member_or_call_chain, write_colon_prefixed_type_annotation,
    write_type_annotation_prefix, write_type_expression_with_inline_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_core::StringId;
use destack_dir::{
    BinaryOperator, Comment, Expression, FunctionSignature, Key, Keyword, LocalNodeId,
    MethodAbstraction, Mutability, Name, Node, NodeType, Parameter, Property, ScalarLiteral, Tree,
    TreeStore, TypeExpression, Visibility, is_identifier_compat,
};
use destack_fir::format::{FormatNodes, FormatResult, Formatter as FirFormatter, VecBuffer, text};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::{NodeSpanRegion, NodeSpanType};
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
    }

    Ok(())
}

/// Check whether a string is an identifier safe to leave unquoted as a property key.
pub(crate) fn is_identifier_for_quotes(content: &str) -> bool {
    is_identifier_compat(content) || content.parse::<Keyword>().is_ok()
}

/// Format a name key while applying quote rules.
pub(crate) fn format_name_with_quotes<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: Name,
    force_quote_keys: bool,
) -> FormatResult<()> {
    let context = f.context();
    let quote_props = context.options.quote_props;

    match name {
        Name::Identifier(string_id) => {
            let content = context.strings.get(string_id);
            if content.starts_with('#')
                && context.options.language_type.supports_private_identifiers()
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

            let should_quote = match quote_props {
                QuoteProperty::AsNeeded => force_quote_keys || !is_ident,
                QuoteProperty::Preserve => true,
                QuoteProperty::Consistent => force_quote_keys || !is_ident,
            };

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

/// Return whether one key requires quotes in a consistent quote group.
pub(crate) fn key_requires_quote_group(context: &DestackFormatContext<'_>, key: Key) -> bool {
    match key {
        Key::Name(Name::String(string_id)) => {
            !is_identifier_for_quotes(context.strings.get(string_id))
        }
        _ => false,
    }
}

/// Format a quoted key using the preferred quote character.
fn format_quoted_name<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    string_id: StringId,
) -> FormatResult<()> {
    let mut quote_style = f.context().options.quote_style;
    if quote_style == QuoteStyle::Semantic {
        quote_style = QuoteStyle::Double;
    }
    let content = f.context().strings.get(string_id);
    let quote_char = quote_style.char_for(content);
    let quote_str = if quote_char == '"' { "\"" } else { "'" };

    write!(f, [token(quote_str), string_id, token(quote_str)])
}

/// Write a field type annotation.
#[inline]
fn write_field_type_annotation<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    value: LocalNodeId<TypeExpression>,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
{
    if let Some(type_span) = f
        .context()
        .tree
        .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Type))
    {
        write_type_annotation_prefix(f, type_span.start)?;
        write_type_expression_with_inline_prefix_annotations(f, value)
    } else {
        write_colon_prefixed_type_annotation(f, value)
    }
}

/// The assignment-like layout used for field initializers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FieldLikeLayout {
    Fluid,
    BreakAfterOperator,
    NeverBreakAfterOperator,
}

/// Return whether one logical rhs can stay inline in the assignment-like layout.
fn field_like_can_inline_logical_rhs(expression: &Expression) -> bool {
    match expression {
        Expression::ObjectExpression { properties, .. } => !properties.is_empty(),
        Expression::ArrayExpression { elements } => !elements.is_empty(),
        Expression::TreeExpression { .. } => true,
        _ => false,
    }
}

/// Return the assignment-like layout for one field initializer.
fn field_like_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    right: LocalNodeId<Expression>,
    is_left_short: bool,
    left_may_break: bool,
) -> FormatResult<FieldLikeLayout> {
    let right_id = transparent_inner_expression(f.context(), right);
    let right_expression = f.context().tree.get(right_id);

    if matches!(right_expression, Expression::SequenceExpression { .. }) {
        return Ok(FieldLikeLayout::BreakAfterOperator);
    }

    if let Expression::Binary {
        operator, right, ..
    } = right_expression
    {
        let is_logical_expression = matches!(
            operator,
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Coalesce
        );

        if !is_logical_expression
            || !field_like_can_inline_logical_rhs(f.context().tree.get(*right))
        {
            return Ok(FieldLikeLayout::BreakAfterOperator);
        }
    }

    if is_poorly_breakable_member_or_call_chain(f, right_id)? && !is_left_short {
        return Ok(FieldLikeLayout::BreakAfterOperator);
    }

    if !left_may_break
        && matches!(
            f.context().tree.get(right_id),
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
        return Ok(FieldLikeLayout::NeverBreakAfterOperator);
    }

    Ok(FieldLikeLayout::Fluid)
}

/// Write one visibility prefix.
fn write_visibility_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    visibility: Option<Visibility>,
) -> FormatResult<()> {
    // visibility
    if let Some(visibility) = visibility {
        let keyword = match visibility {
            Visibility::Public => Keyword::Public,
            Visibility::Protected => Keyword::Protected,
            Visibility::Private => Keyword::Private,
        };
        write!(f, [keyword, space()])?;
    }

    Ok(())
}

/// Write one is_ambient prefix.
fn write_ambient_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_ambient: bool,
) -> FormatResult<()> {
    // is_ambient
    if is_ambient {
        write!(f, [Keyword::Declare, space()])?;
    }

    Ok(())
}

/// Write one static prefix.
fn write_static_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_static: bool,
) -> FormatResult<()> {
    // static
    if is_static {
        write!(f, [Keyword::Static, space()])?;
    }

    Ok(())
}

/// Write one readonly prefix.
fn write_readonly_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_readonly: bool,
) -> FormatResult<()> {
    // readonly
    if is_readonly {
        write!(f, [Keyword::Readonly, space()])?;
    }

    Ok(())
}

/// Write one mutability prefix.
fn write_mutability_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    mutability: Option<Mutability>,
) -> FormatResult<()> {
    // mutability
    if let Some(mutability) = mutability {
        match mutability {
            Mutability::Immutable => write!(f, [token("readonly"), space()])?,
            Mutability::Exclusive => write!(f, [token("exclusive"), space()])?,
            Mutability::Mutable => {}
        }
    }

    Ok(())
}

/// Write one accessor prefix.
fn write_accessor_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_accessor: bool,
) -> FormatResult<()> {
    // accessor
    if is_accessor {
        write!(f, [token("accessor"), space()])?;
    }

    Ok(())
}

/// Write one optional suffix.
fn write_optional_suffix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_optional: bool,
) -> FormatResult<()> {
    // optional
    if is_optional {
        write!(f, [token("?")])?;
    }

    Ok(())
}

/// Return whether one property container should quote all eligible keys.
fn property_should_force_quote_keys<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Property>,
) -> bool {
    if f.context().options.quote_props != QuoteProperty::Consistent {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::ObjectExpression { properties, .. } = f.context().tree.get(parent_id) else {
        return false;
    };

    properties.iter().copied().any(|property_id| {
        let key = match f.context().tree.get(property_id) {
            Property::Field { key, .. } => Some(*key),
            Property::Method { key, .. } => *key,
            Property::Spread { .. } | Property::Error => None,
        };

        key.is_some_and(|key| key_requires_quote_group(f.context(), key))
    })
}

/// Format one runtime object property value using the assignment-like layout.
fn format_object_property_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Property>,
    key: Key,
    value: LocalNodeId<Expression>,
    is_shorthand: bool,
    force_quote_keys: bool,
) -> FormatResult<()> {
    // shorthand
    if is_shorthand {
        return format_key_with_quotes(f, key, force_quote_keys);
    }

    // left side
    let mut buffer = VecBuffer::new(f.state_mut());
    write_field_like_left(
        &mut FirFormatter::new(&mut buffer),
        node_id,
        key,
        None,
        None,
        false,
        false,
        false,
        false,
        false,
        None,
        false,
        false,
        force_quote_keys,
    )?;
    let left_nodes = buffer.into_vec();
    let left_may_break = left_nodes.will_break();
    let is_left_short = left_nodes
        .single_line_width()
        .is_some_and(|width| width < (u32::from(f.context().options.indent_width) + 3));
    let layout = field_like_layout(f, value, is_left_short, left_may_break)?;

    let left = f.intern_vec(left_nodes);
    let left = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        if let Some(left) = &left {
            f.write_node(left.clone());
        }

        Ok(())
    });

    let right = format_with(|f: &mut DestackFormatter<'ast, '_>| write!(f, [value]));
    let inner_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if left_may_break {
            write!(f, [left])?;
        } else {
            write!(f, [group(&left)])?;
        }

        write!(f, [token(":")])?;

        match layout {
            FieldLikeLayout::Fluid => {
                let group_id = f.group_id("object_property_rhs");
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

/// Write the shared left side of one field-like assignment layout.
fn write_field_like_left<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    key: Key,
    value: Option<LocalNodeId<TypeExpression>>,
    visibility: Option<Visibility>,
    is_ambient: bool,
    is_static: bool,
    is_abstract: bool,
    is_override: bool,
    is_readonly: bool,
    mutability: Option<Mutability>,
    is_accessor: bool,
    is_optional: bool,
    force_quote_keys: bool,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
{
    let force_quote_keys =
        force_quote_keys || should_preserve_class_field_quote(f.context(), node_id, key);

    // prefixes
    write_ambient_prefix(f, is_ambient)?;
    write_visibility_prefix(f, visibility)?;
    write_static_prefix(f, is_static)?;

    // abstraction
    if is_abstract {
        write!(f, [Keyword::Abstract, space()])?;
    }

    // override
    if is_override {
        write!(f, [Keyword::Override, space()])?;
    }

    // storage and accessor
    write_readonly_prefix(f, is_readonly)?;
    write_mutability_prefix(f, mutability)?;
    write_accessor_prefix(f, is_accessor)?;

    // key
    format_key_with_quotes(f, key, force_quote_keys)?;

    // key suffixes
    write_optional_suffix(f, is_optional)?;

    // value
    if let Some(value) = value {
        write_field_type_annotation(f, node_id, value)?;
    }

    Ok(())
}

/// Return whether a class field string key should preserve quotes.
fn should_preserve_class_field_quote<T>(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<T>,
    key: Key,
) -> bool
where
    T: Node + Clone,
    Tree: TreeStore<T>,
{
    if !matches!(key, Key::Name(Name::String(_))) {
        return false;
    }

    if !context.options.language_type.is_destack() && !context.options.language_type.is_typescript()
    {
        return false;
    }

    let Some((_, parent_type)) = context.parent(node_id) else {
        return false;
    };

    parent_type == NodeType::Declaration
}

/// Format shared property or member field output.
pub(crate) fn format_field_like<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    key: Key,
    value: Option<LocalNodeId<TypeExpression>>,
    visibility: Option<Visibility>,
    is_ambient: bool,
    is_static: bool,
    is_abstract: bool,
    is_override: bool,
    is_readonly: bool,
    mutability: Option<Mutability>,
    is_accessor: bool,
    is_optional: bool,
    default: Option<LocalNodeId<Expression>>,
    force_quote_keys: bool,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
{
    // no initializer
    let Some(default) = default else {
        write_field_like_left(
            f,
            node_id,
            key,
            value,
            visibility,
            is_ambient,
            is_static,
            is_abstract,
            is_override,
            is_readonly,
            mutability,
            is_accessor,
            is_optional,
            force_quote_keys,
        )?;
        return Ok(());
    };

    // left side
    let mut buffer = VecBuffer::new(f.state_mut());
    write_field_like_left(
        &mut FirFormatter::new(&mut buffer),
        node_id,
        key,
        value,
        visibility,
        is_ambient,
        is_static,
        is_abstract,
        is_override,
        is_readonly,
        mutability,
        is_accessor,
        is_optional,
        force_quote_keys,
    )?;
    let left_nodes = buffer.into_vec();
    let left_may_break = left_nodes.will_break();
    let is_left_short = left_nodes
        .single_line_width()
        .is_some_and(|width| width < (u32::from(f.context().options.indent_width) + 3));
    let layout = field_like_layout(f, default, is_left_short, left_may_break)?;

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
    key: Option<Key>,
    signature: &FunctionSignature,
    abstraction: MethodAbstraction,
    body: Option<LocalNodeId<Expression>>,
    visibility: Option<Visibility>,
    is_ambient: bool,
    is_static: bool,
    is_accessor: bool,
    is_optional: bool,
    force_quote_keys: bool,
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    Tree: TreeStore<N>,
{
    let parameters = method_parameters(signature);

    // prefixes
    write_ambient_prefix(f, is_ambient)?;
    write_visibility_prefix(f, visibility)?;
    write_static_prefix(f, is_static)?;
    if abstraction == MethodAbstraction::Virtual {
        write!(f, [Keyword::Virtual, space()])?;
    }
    write_accessor_prefix(f, is_accessor)?;

    // shared function header prefix
    write_function_header_prefix(f, signature, false, key.is_some())?;

    // key
    if let Some(key) = key {
        format_key_with_quotes(f, key, force_quote_keys)?;
    }

    // optional
    write_optional_suffix(f, is_optional)?;

    // generic parameters
    if !signature.generic_parameters.is_empty() {
        write_generic_parameter_list(
            f,
            &signature.generic_parameters,
            default_generic_parameter_trailing_separator(f),
        )?;
    }

    // key to parameter separator
    write!(f, [block_infix_annotations(f.context(), node_id)])?;
    write_method_parameters_and_return_type(f, node_id, signature, &parameters)?;

    // where clauses
    if !signature.where_clauses.is_empty() {
        format_where_clause_with_break(f, &signature.where_clauses)?;
    }

    // body
    if let Some(body) = body {
        write_method_signature_and_body(f, node_id, signature, body)?;
    }

    Ok(())
}

/// Collect method parameters, including `this`.
fn method_parameters(signature: &FunctionSignature) -> Vec<LocalNodeId<Parameter>> {
    let mut parameters = Vec::with_capacity(signature.parameters.len() + 1);

    if let Some(this_parameter) = signature.this_parameter {
        parameters.push(this_parameter);
    }

    parameters.extend(signature.parameters.iter().copied());
    parameters
}

/// Write one method parameter list and return type.
fn write_method_parameters_and_return_type<'ast, N>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    signature: &FunctionSignature,
    parameters: &[LocalNodeId<Parameter>],
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    Tree: TreeStore<N>,
{
    let format_parameters_and_return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let format_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if parameters.is_empty() {
                write_empty_parameter_list_with_interior_comments(f, node_id)?;
            } else if should_hug_function_parameters(f.context(), parameters, false) {
                write_signature_hug_parameter_list(f, parameters)?;
            } else {
                let disallow_trailing_parameter_separator = parameters
                    .last()
                    .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id));
                write_signature_parameter_list(
                    f,
                    parameters,
                    disallow_trailing_parameter_separator,
                )?;
            }

            Ok(())
        })
        .memoized();

        let format_return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if let Some(return_type) = signature.return_type {
                write_signature_return_type(f, node_id, return_type)?;
            }

            Ok(())
        })
        .memoized();

        let format_parameter_head =
            format_with(|_f: &mut DestackFormatter<'ast, '_>| Ok(())).memoized();
        let should_break_parameters = should_break_function_parameters(f.context(), parameters);
        write_grouped_parameters_with_return_type(
            f,
            &signature.generic_parameters,
            parameters.len(),
            signature.return_type,
            &format_parameter_head,
            &format_parameters,
            &format_return_type,
            should_break_parameters,
            false,
        )?;

        Ok(())
    });

    write!(f, [group(&format_parameters_and_return_type)])
}

/// Write one method body after the signature.
fn write_method_signature_and_body<'ast, N>(
    f: &mut DestackFormatter<'ast, '_>,
    _node_id: LocalNodeId<N>,
    signature: &FunctionSignature,
    body: LocalNodeId<Expression>,
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    Tree: TreeStore<N>,
{
    write_method_body(f, body, false, signature.return_type)
}

/// Return comments between one method signature and its body.
fn method_body_separator_comments<'ast>(
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

/// Write one method body after the signature has been resolved.
fn write_method_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    body: LocalNodeId<Expression>,
    force_break_before_body: bool,
    _return_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    let body_block_id = match f.context().node::<Expression>(body) {
        Expression::Block(block_id) => Some(*block_id),
        _ => None,
    };

    // block-leading line comments belong to the block owner
    let block_separator_comments = method_body_separator_comments(f, body)
        .into_iter()
        .filter(|comment| body_block_id.is_none() || comment.is_block())
        .collect::<Vec<_>>();

    if force_break_before_body {
        write!(f, [hard_line_break()])?;
    }

    if !block_separator_comments.is_empty() {
        write!(f, [space()])?;

        for (index, comment) in block_separator_comments.iter().copied().enumerate() {
            let comment_span = comment.span;
            format_comment(f, comment)?;

            let is_last = index + 1 == block_separator_comments.len();
            if !is_last
                || f.context()
                    .span_has_newline_before_next_non_whitespace_token(comment_span)
            {
                write!(f, [hard_line_break()])?;
            } else {
                write!(f, [space()])?;
            }
        }

        if body_block_id.is_some() {
            return write_expression_without_prefix_annotations(f, body);
        }

        return write!(f, [body]);
    }

    if expression_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }

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

/// Format one node with shared directive handling and trailing annotation control.
pub(crate) fn format_node_with_directive<'ast, T, F>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    owns_infix_annotations: bool,
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
    F: FnMut(&mut DestackFormatter<'ast, '_>) -> FormatResult<()>,
{
    let is_ignored = node_has_ignore_directive(f.context(), node_id);
    write!(f, [prefix_comments_before_decorators(f.context(), node_id)])?;
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
        let force_quote_keys = property_should_force_quote_keys(f, node_id);

        if let Property::Method {
            key,
            signature,
            body,
        } = self
        {
            return format_node_with_directive(f, node_id, true, |f| {
                format_method_like(
                    f,
                    node_id,
                    *key,
                    signature,
                    MethodAbstraction::Concrete,
                    *body,
                    None,
                    false,
                    false,
                    false,
                    false,
                    force_quote_keys,
                )
            });
        }

        format_node_with_directive(f, node_id, false, |f| {
            match self {
                Property::Field {
                    key,
                    value,
                    is_shorthand,
                } => {
                    format_object_property_value(
                        f,
                        node_id,
                        *key,
                        *value,
                        *is_shorthand,
                        force_quote_keys,
                    )?;
                }
                Property::Spread { value } => {
                    // keyword
                    write!(f, [token("...")])?;

                    // value
                    write!(f, [value])?;
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
