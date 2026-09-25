use crate::annotation::{
    block_infix_annotations, decorator_prefix_annotations, format_comment,
    infix_or_postfix_annotations, postfix_annotations, prefix_comments_before_decorators,
};
use crate::chain::transparent_inner_expression;
use crate::declaration::signature::{
    ParameterList, default_generic_parameter_trailing_separator, format_where_clause,
    parameter_is_variadic, should_hug_function_parameters,
    write_empty_parameter_list_with_interior_comments, write_function_header_prefix,
    write_generic_parameter_list, write_grouped_parameters_with_return_type,
    write_signature_hug_parameter_list_with_this, write_signature_parameter_list_with_this,
    write_signature_return_type,
};
use crate::declaration::statement::write_block_body;
use crate::declaration::{
    write_keyword_prefix, write_mutability_prefix, write_token_prefix, write_token_suffix,
    write_visibility_prefix,
};
use crate::expression::write_expression_without_prefix_annotations;
use crate::file::{node_has_ignore_directive, write_ignored_node, write_source_span};
use crate::operator::{
    is_poorly_breakable_member_or_call_chain, write_colon_prefixed_type_annotation,
    write_type_annotation_prefix, write_type_expression_with_inline_prefix_annotations,
};
use crate::{FormatNode, TsppFormatContext, TsppFormatter};
use tspp_core::StringId;
use tspp_dir::{
    BinaryOperator, Comment, Expression, FunctionSignature, Keyword, Literal, LocalNodeId, Member,
    MethodAbstraction, Name, Node, Parameter, Property, Tree, TreeStore, TypeExpression,
    Visibility, is_identifier_compat,
};
use tspp_fir::format::{FormatError, FormatLayout, FormatResult, Formatter as FirFormatter, text};
use tspp_fir::prelude::*;
use tspp_fir::write;
use tspp_repository::{QuoteProperty, QuoteStyle};
use tspp_source::{NodeSpanRegion, NodeSpanType};

impl<'ast> Format<'ast, TsppFormatContext<'ast>> for StringId {
    #[inline]
    fn format(&self, f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
        let string = f.context().strings.get(*self);
        write!(f, [text(string)])
    }
}

impl<'ast> Format<'ast, TsppFormatContext<'ast>> for Name {
    #[inline]
    fn format(&self, f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
        format_name_with_quotes(f, *self, false)
    }
}

impl<'ast> Format<'ast, TsppFormatContext<'ast>> for Keyword {
    #[inline]
    fn format(&self, f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [text(self.as_str())])
    }
}

/// Return whether a string can be written as an unquoted property name.
pub(crate) fn can_unquote_name(content: &str) -> bool {
    is_identifier_compat(content) || content.parse::<Keyword>().is_ok()
}

/// Format a property name while applying quote rules.
pub(crate) fn format_name_with_quotes<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    name: Name,
    force_quotes: bool,
) -> FormatResult<()> {
    let context = f.context();
    let quote_props = context.options.quote_props;

    match name {
        Name::Identifier(string_id) => {
            let should_quote = quote_props == QuoteProperty::Consistent && force_quotes;
            if should_quote {
                format_quoted_name(f, string_id)?;
            } else {
                string_id.format(f)?;
            }
        }
        Name::String(string_id) => {
            let content = context.strings.get(string_id);
            let is_ident = can_unquote_name(content);

            let should_quote = match quote_props {
                QuoteProperty::AsNeeded => force_quotes || !is_ident,
                QuoteProperty::Preserve => true,
                QuoteProperty::Consistent => force_quotes || !is_ident,
            };

            if should_quote {
                format_quoted_name(f, string_id)?;
            } else {
                string_id.format(f)?;
            }
        }
        Name::Index(index) => {
            write!(f, [copied_text(&index.to_string())])?;
        }
    }

    Ok(())
}

/// Return whether one name requires quotes in a consistent quote group.
pub(crate) fn name_requires_quote_group(context: &TsppFormatContext<'_>, name: Name) -> bool {
    match name {
        Name::String(string_id) => !can_unquote_name(context.strings.get(string_id)),
        _ => false,
    }
}

/// Format a quoted name using the preferred quote character.
fn format_quoted_name<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
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
    /// Indent the right side only when the left side breaks.
    Fluid,
    /// Break and indent directly after the separator.
    BreakAfterOperator,
    /// Keep the right side directly after the separator.
    NeverBreakAfterOperator,
}

/// The separator between one field name and value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FieldValueSeparator {
    /// An object property separator.
    Colon,
    /// A field initializer separator.
    Equal,
}

impl FieldValueSeparator {
    /// Write this separator.
    fn write<'ast>(self, f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Self::Colon => write!(f, [token(":")]),
            Self::Equal => write!(f, [space(), token("=")]),
        }
    }
}

/// Return whether one logical rhs can stay inline in the assignment-like layout.
fn field_like_can_inline_logical_rhs(expression: &Expression) -> bool {
    match expression {
        Expression::ObjectExpression { properties, .. }
        | Expression::StructExpression { properties, .. } => !properties.is_empty(),
        Expression::ArrayExpression { elements } => !elements.is_empty(),
        Expression::TreeExpression { .. } => true,
        _ => false,
    }
}

/// Return the assignment-like layout for one field initializer.
fn field_like_layout<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    right: LocalNodeId<Expression>,
    is_left_short: bool,
    left_may_break: bool,
) -> FormatResult<FieldLikeLayout> {
    let right_id = transparent_inner_expression(f.context(), right);
    let right_expression = f.context().tree.get(right_id);

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
                | Expression::Literal(
                    Literal::Boolean(_)
                        | Literal::Integer(_)
                        | Literal::Bigint(_)
                        | Literal::Float(_)
                        | Literal::String(_)
                )
        )
    {
        return Ok(FieldLikeLayout::NeverBreakAfterOperator);
    }

    Ok(FieldLikeLayout::Fluid)
}

/// Write one captured field left side and its value.
fn write_field_value<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    left_instructions: InstructionTape<'ast>,
    right_id: LocalNodeId<Expression>,
    separator: FieldValueSeparator,
) -> FormatResult<()> {
    let left_may_break = left_instructions.will_break();
    let is_left_short = left_instructions
        .single_line_width()
        .is_some_and(|width| width < (u32::from(f.context().options.indent_width) + 3));
    let layout = field_like_layout(f, right_id, is_left_short, left_may_break)?;
    let left = left_instructions.collapse();

    let left = format_with(move |f: &mut TsppFormatter<'ast, '_>| {
        if let Some(left) = &left {
            f.write_element(*left);
        }

        Ok(())
    });
    let right = format_with(|f: &mut TsppFormatter<'ast, '_>| write!(f, [right_id]));
    let content = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        if left_may_break {
            write!(f, [left])?;
        } else {
            write!(f, [group(&left)])?;
        }

        separator.write(f)?;

        match layout {
            FieldLikeLayout::Fluid => {
                let group_id = f.group_id();
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
            FieldLikeLayout::NeverBreakAfterOperator => write!(f, [space(), right]),
        }
    });

    write!(f, [group(&content)])
}

/// Return whether one property container should quote all eligible keys.
fn property_should_force_quotes<'ast>(
    f: &TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Property>,
) -> bool {
    if f.context().options.quote_props != QuoteProperty::Consistent {
        return false;
    }

    let Some(parent_id) = f.context().expression_parent(node_id) else {
        return false;
    };

    let (Expression::ObjectExpression { properties, .. }
    | Expression::StructExpression { properties, .. }) = f.context().tree.get(parent_id)
    else {
        return false;
    };

    properties.iter().copied().any(|property_id| {
        let name = f.context().tree.get(property_id).name();

        name.is_some_and(|name| name_requires_quote_group(f.context(), name))
    })
}

/// Format one runtime object property value using the assignment-like layout.
fn format_object_property_value<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    name: Name,
    value: LocalNodeId<Expression>,
    is_shorthand: bool,
    force_quotes: bool,
) -> FormatResult<()> {
    // shorthand
    if is_shorthand {
        return format_name_with_quotes(f, name, force_quotes);
    }

    // left side
    let mut formatter = FirFormatter::new(f.state_mut());
    format_name_with_quotes(&mut formatter, name, force_quotes)?;
    let left_instructions = formatter.into_tape();

    write_field_value(f, left_instructions, value, FieldValueSeparator::Colon)
}

/// Write one member field before its initializer.
fn write_member_field_left<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Member>,
    member: &Member,
    force_quotes: bool,
) -> FormatResult<()> {
    let Member::Field {
        name,
        declared_type,
        is_optional,
        is_readonly,
        mutability,
        visibility,
        is_ambient,
        is_abstract,
        is_override,
        is_static,
        is_accessor,
        ..
    } = member
    else {
        return Err(FormatError::SyntaxError {
            message: "member field formatter requires a field",
        });
    };
    let force_quotes = force_quotes || matches!(name, Name::String(_));

    // prefixes
    write_keyword_prefix(f, Keyword::Declare, *is_ambient)?;
    write_visibility_prefix(f, *visibility)?;
    write_keyword_prefix(f, Keyword::Static, *is_static)?;

    // abstraction
    if *is_abstract {
        write!(f, [Keyword::Abstract, space()])?;
    }

    // override
    if *is_override {
        write!(f, [Keyword::Override, space()])?;
    }

    // storage and accessor
    write_keyword_prefix(f, Keyword::Readonly, *is_readonly)?;
    write_mutability_prefix(f, *mutability)?;
    write_token_prefix(f, "accessor", *is_accessor)?;

    // name
    format_name_with_quotes(f, *name, force_quotes)?;

    // name suffixes
    write_token_suffix(f, "?", *is_optional)?;

    // declared type
    if let Some(declared_type) = declared_type {
        write_field_type_annotation(f, node_id, *declared_type)?;
    }

    Ok(())
}

/// Format one member field.
pub(crate) fn format_member_field<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<Member>,
    member: &Member,
    force_quotes: bool,
) -> FormatResult<()> {
    let Member::Field { default, .. } = member else {
        return Err(FormatError::SyntaxError {
            message: "member field formatter requires a field",
        });
    };

    // no initializer
    let Some(default) = *default else {
        return write_member_field_left(f, node_id, member, force_quotes);
    };

    // left side
    let mut formatter = FirFormatter::new(f.state_mut());
    write_member_field_left(&mut formatter, node_id, member, force_quotes)?;
    let left_instructions = formatter.into_tape();

    write_field_value(f, left_instructions, default, FieldValueSeparator::Equal)
}

/// Format shared property or member method output.
pub(crate) fn format_method_like<'ast, N>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    name: Option<Name>,
    signature: &FunctionSignature,
    abstraction: MethodAbstraction,
    body: Option<LocalNodeId<Expression>>,
    visibility: Option<Visibility>,
    is_ambient: bool,
    is_static: bool,
    is_accessor: bool,
    is_optional: bool,
    force_quotes: bool,
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    Tree: TreeStore<N>,
{
    let parameters = ParameterList::from_signature(signature);

    // prefixes
    write_keyword_prefix(f, Keyword::Declare, is_ambient)?;
    write_visibility_prefix(f, visibility)?;
    write_keyword_prefix(f, Keyword::Static, is_static)?;
    if abstraction == MethodAbstraction::Virtual {
        write!(f, [Keyword::Virtual, space()])?;
    }
    write_token_prefix(f, "accessor", is_accessor)?;

    // shared function header prefix
    write_function_header_prefix(f, signature, false, name.is_some())?;

    // name
    if let Some(name) = name {
        format_name_with_quotes(f, name, force_quotes)?;
    }

    // optional
    write_token_suffix(f, "?", is_optional)?;

    // generic parameters
    if !signature.generic_parameters.is_empty() {
        write_generic_parameter_list(
            f,
            &signature.generic_parameters,
            default_generic_parameter_trailing_separator(f),
        )?;
    }

    // name to parameter separator
    write!(f, [block_infix_annotations(f.context(), node_id)])?;
    write_method_parameters_and_return_type(f, node_id, signature, &parameters)?;

    // where clauses
    if !signature.where_clauses.is_empty() {
        format_where_clause(f, &signature.where_clauses)?;
    }

    // body
    if let Some(body) = body {
        write_method_body(f, body, false, signature.return_type)?;
    }

    Ok(())
}

/// Write one method parameter list and return type.
fn write_method_parameters_and_return_type<'ast, N>(
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<N>,
    signature: &FunctionSignature,
    parameters: &[LocalNodeId<Parameter>],
) -> FormatResult<()>
where
    N: Node + Clone + 'ast,
    Tree: TreeStore<N>,
{
    let format_parameters_and_return_type = format_with(|f: &mut TsppFormatter<'ast, '_>| {
        let format_parameters = format_with(|f: &mut TsppFormatter<'ast, '_>| {
            if parameters.is_empty() {
                write_empty_parameter_list_with_interior_comments(f, node_id)?;
            } else if should_hug_function_parameters(f.context(), parameters, false) {
                write_signature_hug_parameter_list_with_this(
                    f,
                    signature.this_form,
                    signature.this_parameter,
                    &signature.parameters,
                )?;
            } else {
                let disallow_trailing_parameter_separator = parameters
                    .last()
                    .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id));
                write_signature_parameter_list_with_this(
                    f,
                    signature.this_form,
                    signature.this_parameter,
                    &signature.parameters,
                    disallow_trailing_parameter_separator,
                )?;
            }

            Ok(())
        });

        let format_return_type = format_with(|f: &mut TsppFormatter<'ast, '_>| {
            if let Some(return_type) = signature.return_type {
                write_signature_return_type(f, node_id, return_type)?;
            }

            Ok(())
        });

        let format_parameter_head = format_with(|_f: &mut TsppFormatter<'ast, '_>| Ok(()));
        write_grouped_parameters_with_return_type(
            f,
            &signature.generic_parameters,
            parameters.len(),
            signature.return_type,
            format_parameter_head,
            format_parameters,
            format_return_type,
            false,
            false,
        )?;

        Ok(())
    });

    write!(f, [group(&format_parameters_and_return_type)])
}

/// Return comments between one method signature and its body.
fn method_body_separator_comments<'ast>(
    f: &TsppFormatter<'ast, '_>,
    body_id: LocalNodeId<Expression>,
) -> Vec<Comment> {
    let body_span = f.context().span(body_id);
    let Some(previous_token) = f.context().previous_token_before_span(body_span) else {
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
    f: &mut TsppFormatter<'ast, '_>,
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
            if !is_last || f.context().has_newline_before_next_token(comment_span) {
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

    write!(f, [space()])?;

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
    f: &mut TsppFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    owns_infix_annotations: bool,
    mut format_node: F,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
    F: FnMut(&mut TsppFormatter<'ast, '_>) -> FormatResult<()>,
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
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let force_quotes = property_should_force_quotes(f, node_id);

        if let Property::Method {
            name,
            signature,
            body,
        } = self
        {
            return format_node_with_directive(f, node_id, true, |f| {
                format_method_like(
                    f,
                    node_id,
                    *name,
                    signature,
                    MethodAbstraction::Concrete,
                    *body,
                    None,
                    false,
                    false,
                    false,
                    false,
                    force_quotes,
                )
            });
        }

        format_node_with_directive(f, node_id, false, |f| {
            match self {
                Property::Field {
                    name,
                    value,
                    is_shorthand,
                } => {
                    format_object_property_value(f, *name, *value, *is_shorthand, force_quotes)?;
                }
                Property::Spread { value } => {
                    // keyword
                    write!(f, [token("...")])?;

                    // value
                    write!(f, [value])?;
                }
                Property::Method { .. } => {}
                Property::Error => {
                    write_source_span(f, f.context().span(node_id))?;
                }
            }

            Ok(())
        })
    }
}
