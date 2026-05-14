use crate::annotation::{
    FormatLeadingComments, block_infix_annotations, infix_or_postfix_annotations,
    prefix_annotations,
};
use crate::collection::{FormatSeparatedIter, TrailingSeparator, separated_entries};
use crate::context::{MemoizeFormatExt, MemoizedFormat};
use crate::expression::write_type_expression_node;
use crate::operator::{
    write_colon_prefixed_type_annotation, write_type_annotation_prefix,
    write_type_expression_with_inline_prefix_annotations,
};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_core::StringId;
use destack_dir::{
    Asynchrony, Expression, FunctionForm, FunctionRole, FunctionSignature, GenericParameter,
    Keyword, LocalNodeId, Node, Parameter, Pattern, TokenType, Tree, TreeStore, TypeExpression,
    VarianceModifier, Visibility, WhereClause,
};
use destack_fir::format::{FormatNodes, FormatResult};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::{NodeSpanRegion, NodeSpanType};
use destack_workspace::TrailingComma;

impl<'ast> Format<DestackFormatContext<'ast>> for Visibility {
    #[inline]
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Visibility::Public => write!(f, [Keyword::Public]),
            Visibility::Protected => write!(f, [Keyword::Protected]),
            Visibility::Private => write!(f, [Keyword::Private]),
        }
    }
}

/// Write one visibility prefix.
fn write_visibility_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    visibility: Option<Visibility>,
) -> FormatResult<()> {
    // visibility
    if let Some(visibility) = visibility {
        write!(f, [visibility, space()])?;
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

/// Write one variance prefix.
fn write_variance_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    variance: Option<VarianceModifier>,
) -> FormatResult<()> {
    // variance
    match variance {
        Some(VarianceModifier::In) => write!(f, [token("in"), space()])?,
        Some(VarianceModifier::Out) => write!(f, [token("out"), space()])?,
        Some(VarianceModifier::InOut) => {
            write!(f, [token("in"), space(), token("out"), space()])?;
        }
        None => {}
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

/// Write one type-parameter-like `extends` and `=` trailer sequence.
fn write_generic_parameter_constraint_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    if f.context().options.language_type.is_destack() {
        write!(f, [token(":")])
    } else {
        write!(f, [space(), token("extends")])
    }
}

/// Write one type-parameter-like constraint and `=` trailer sequence.
pub(crate) fn write_type_parameter_constraint_and_default<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<GenericParameter>,
    constraint: Option<LocalNodeId<TypeExpression>>,
    default: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    // constraint
    if let Some(constraint) = constraint {
        let type_span = f
            .context()
            .tree
            .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Type))
            .expect("generic parameter constraint should have a type span");
        let group_id = f.group_id("constraint");
        let leading_comments = f
            .context()
            .comments()
            .comments_before(type_span.start)
            .to_vec();

        if !leading_comments.is_empty() {
            write!(f, [space()])?;
            write!(f, [FormatLeadingComments::Comments(&leading_comments)])?;
        }

        write_generic_parameter_constraint_prefix(f)?;

        write!(
            f,
            [
                group(&indent(&format_args![
                    line_suffix_boundary(),
                    soft_line_break_or_space()
                ]))
                .with_id(Some(group_id)),
                indent_if_group_breaks(&constraint, group_id)
            ]
        )?;
    }

    // default
    if let Some(default) = default {
        let group_id = f.group_id("default");

        write!(
            f,
            [
                space(),
                token("="),
                group(&indent(&format_args![
                    line_suffix_boundary(),
                    soft_line_break_or_space()
                ]))
                .with_id(Some(group_id)),
                indent_if_group_breaks(&default, group_id)
            ]
        )?;
    }

    Ok(())
}

/// Write one grouped generic-parameter list.
pub(crate) fn write_generic_parameter_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<GenericParameter>],
    trailing_separator: TrailingSeparator,
) -> FormatResult<()> {
    // empty list
    if generic_parameters.is_empty() {
        return write!(f, [token("<>")]);
    }

    // grouped list
    let body = separated_entries(",", generic_parameters, trailing_separator, None);
    write!(
        f,
        [group(&format_args![
            token("<"),
            soft_block_indent(&body),
            token(">")
        ])]
    )
}

/// Return the default trailing separator for one generic-parameter list.
pub(crate) fn default_generic_parameter_trailing_separator(
    f: &DestackFormatter<'_, '_>,
) -> TrailingSeparator {
    match f.context().options.trailing_comma {
        TrailingComma::None => TrailingSeparator::Omit,
        TrailingComma::Es5 | TrailingComma::All => TrailingSeparator::Allowed,
    }
}

/// Write one parameter type annotation.
fn write_parameter_type<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    declared_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
{
    // declared type
    if let Some(declared_type) = declared_type {
        if let Some(type_span) = f
            .context()
            .tree
            .get_side_span(node_id, NodeSpanType::Region(NodeSpanRegion::Type))
        {
            write_type_annotation_prefix(f, type_span.start)?;
            write_type_expression_with_inline_prefix_annotations(f, declared_type)?;
        } else {
            write_colon_prefixed_type_annotation(f, declared_type)?;
        }
    }

    Ok(())
}

/// Write one parameter default value.
fn write_parameter_default<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    default: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // default
    if let Some(default) = default {
        write!(f, [space(), token("="), space(), default])?;
    }

    Ok(())
}

/// Write one signature return type annotation.
pub(crate) fn write_signature_return_type<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
    return_type: LocalNodeId<TypeExpression>,
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
        let return_type_span = f.context().span::<TypeExpression>(return_type);
        let separator_comments = f
            .context()
            .comments()
            .comments_in_range(type_span.start + 1, return_type_span.start);

        write_type_annotation_prefix(f, type_span.start)?;

        if !separator_comments.is_empty() {
            write!(f, [FormatLeadingComments::Comments(separator_comments)])?;
        }

        let return_type_expression = f.context().tree.get::<TypeExpression>(return_type);
        write_type_expression_node(f, return_type, return_type_expression, true)
    } else {
        write_colon_prefixed_type_annotation(f, return_type)
    }
}

/// Return whether one pattern parameter is destructuring.
fn parameter_pattern_is_destructuring(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let pattern_id = match context.tree.get(parameter_id) {
        Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => *pattern,
        Parameter::Named { .. } | Parameter::VariadicNamed { .. } | Parameter::Error => {
            return false;
        }
    };

    matches!(
        context.tree.get(pattern_id),
        Pattern::Object { .. }
            | Pattern::TaggedObject { .. }
            | Pattern::Sequence { .. }
            | Pattern::Tuple { .. }
            | Pattern::TaggedTuple { .. }
    )
}

/// Return whether one parameter default is simple enough to hug.
fn parameter_default_is_huggable(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let default = match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => *default,
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } | Parameter::Error => {
            return false;
        }
    };

    let Some(default) = default else {
        return true;
    };

    match context.tree.get(default) {
        Expression::Identifier { .. }
        | Expression::QualifiedReference { .. }
        | Expression::ScalarLiteral(_)
        | Expression::This
        | Expression::Super => true,

        Expression::ObjectExpression { properties, .. } => properties.is_empty(),
        Expression::ArrayExpression { elements } => elements.is_empty(),

        _ => false,
    }
}

/// Return whether one parameter uses any modifiers.
fn parameter_has_modifier(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    match context.tree.get(parameter_id) {
        Parameter::Named {
            visibility,
            is_readonly,
            ..
        }
        | Parameter::VariadicNamed {
            visibility,
            is_readonly,
            ..
        } => visibility.is_some() || *is_readonly,
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } | Parameter::Error => false,
    }
}

/// Return whether one parameter is a plain binding identifier.
fn parameter_is_binding_identifier(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    matches!(
        context.tree.get(parameter_id),
        Parameter::Named { .. } | Parameter::VariadicNamed { .. }
    )
}

/// Return the declared type annotation for one parameter.
fn parameter_declared_type(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> Option<LocalNodeId<TypeExpression>> {
    match context.tree.get(parameter_id) {
        Parameter::Named { declared_type, .. }
        | Parameter::Pattern { declared_type, .. }
        | Parameter::VariadicNamed { declared_type, .. }
        | Parameter::VariadicPattern { declared_type, .. } => *declared_type,
        Parameter::Error => None,
    }
}

/// Return whether one parameter has a default value.
fn parameter_has_default(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => default.is_some(),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } | Parameter::Error => {
            false
        }
    }
}

/// Return whether one type annotation is object-like for parameter hugging.
fn type_expression_is_object_like(
    context: &DestackFormatContext<'_>,
    type_id: LocalNodeId<TypeExpression>,
) -> bool {
    matches!(
        context.tree.get(type_id),
        TypeExpression::Object { .. } | TypeExpression::Mapped { .. }
    )
}

/// Return whether one generic parameter permits parameter grouping.
pub(crate) fn function_grouping_generic_parameter_is_plain(
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

/// Return whether parameters should group separately from the return type.
pub(crate) fn should_group_parameters_with_return_type<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<GenericParameter>],
    parameter_count: usize,
    return_type: Option<LocalNodeId<TypeExpression>>,
    formatted_return_type: &MemoizedFormat<T>,
) -> FormatResult<bool>
where
    T: Format<DestackFormatContext<'ast>>,
{
    match generic_parameters {
        [] => {}
        [generic_parameter_id]
            if function_grouping_generic_parameter_is_plain(f.context(), *generic_parameter_id) => {
        }
        _ => return Ok(false),
    }

    let Some(return_type) = return_type else {
        return Ok(false);
    };
    if parameter_count != 1 {
        return Ok(false);
    }

    if type_expression_is_object_like(f.context(), return_type) {
        return Ok(true);
    }

    let will_break = formatted_return_type
        .inspect(f)?
        .is_some_and(|return_type| return_type.will_break());

    Ok(will_break)
}

/// Write type parameters, parameters, and return type using grouped signature layout.
pub(crate) fn write_grouped_parameters_with_return_type<'ast, H, P, R>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<GenericParameter>],
    parameter_count: usize,
    return_type: Option<LocalNodeId<TypeExpression>>,
    format_parameter_head: &MemoizedFormat<H>,
    format_parameters: &MemoizedFormat<P>,
    format_return_type: &MemoizedFormat<R>,
    should_expand_parameters: bool,
    should_group_return_type: bool,
) -> FormatResult<()>
where
    H: Format<DestackFormatContext<'ast>>,
    P: Format<DestackFormatContext<'ast>>,
    R: Format<DestackFormatContext<'ast>>,
{
    format_parameter_head.inspect(f)?;
    format_parameters.inspect(f)?;

    let should_group_parameters = should_expand_parameters
        || should_group_parameters_with_return_type(
            f,
            generic_parameters,
            parameter_count,
            return_type,
            format_return_type,
        )?;

    if should_group_parameters {
        write!(
            f,
            [
                group(&format_args![format_parameter_head, format_parameters])
                    .should_expand(should_expand_parameters)
            ]
        )?;
    } else {
        write!(f, [format_parameter_head, format_parameters])?;
    }

    if should_group_return_type {
        write!(f, [group(format_return_type)])?;
    } else {
        write!(f, [format_return_type])?;
    }

    Ok(())
}

/// Return whether comments surround the only parameter inside its parentheses.
fn single_parameter_has_paren_comments(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let parameter_span = context.span(parameter_id);

    let has_open_paren_comments = context
        .previous_non_trivia_token_before_span(parameter_span)
        .filter(|token| token.token.ty == TokenType::OpenParenthesis)
        .is_some_and(|token| {
            context
                .comments()
                .has_comment_in_range(token.span.end, parameter_span.start)
        });
    if has_open_paren_comments {
        return true;
    }

    context
        .next_non_trivia_token_after_span(parameter_span)
        .filter(|token| token.token.ty == TokenType::CloseParenthesis)
        .is_some_and(|token| {
            context
                .comments()
                .has_comment_in_range(parameter_span.end, token.span.start)
        })
}

/// Format one named parameter.
fn write_named_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameter_id: LocalNodeId<Parameter>,
    name: StringId,
    visibility: Option<Visibility>,
    is_readonly: bool,
    is_optional: bool,
    declared_type: Option<LocalNodeId<TypeExpression>>,
    default: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let left = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // prefixes
        write_visibility_prefix(f, visibility)?;
        write_readonly_prefix(f, is_readonly)?;

        // name
        write!(f, [name])?;
        write_optional_suffix(f, is_optional)?;

        // trailers
        write_parameter_type(f, parameter_id, declared_type)
    })
    .memoized();

    if let Some(default) = default {
        left.inspect(f)?;

        let leading_comments = f
            .context()
            .comments()
            .own_line_comments_before(f.context().span(default).start)
            .to_vec();

        if !leading_comments.is_empty() {
            write!(f, [FormatLeadingComments::Comments(&leading_comments)])?;
        }

        write!(f, [group(&left), space(), token("="), space(), default])
    } else {
        write!(f, [left])
    }
}

/// Format one pattern parameter.
fn write_pattern_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameter_id: LocalNodeId<Parameter>,
    pattern: LocalNodeId<Pattern>,
    is_optional: bool,
    declared_type: Option<LocalNodeId<TypeExpression>>,
    default: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let left = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // pattern
        write!(f, [pattern])?;
        write_optional_suffix(f, is_optional)?;

        // trailers
        write_parameter_type(f, parameter_id, declared_type)
    })
    .memoized();

    if let Some(default) = default {
        left.inspect(f)?;

        let leading_comments = f
            .context()
            .comments()
            .own_line_comments_before(f.context().span(default).start)
            .to_vec();

        if !leading_comments.is_empty() {
            write!(f, [FormatLeadingComments::Comments(&leading_comments)])?;
        }

        write!(f, [group(&left), space(), token("="), space(), default])
    } else {
        write!(f, [left])
    }
}

/// Format one variadic named parameter.
fn write_variadic_named_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameter_id: LocalNodeId<Parameter>,
    name: StringId,
    visibility: Option<Visibility>,
    is_readonly: bool,
    declared_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    // prefixes
    write_visibility_prefix(f, visibility)?;
    write_readonly_prefix(f, is_readonly)?;

    // variadic name
    write!(f, [token("..."), name])?;

    // type
    write_parameter_type(f, parameter_id, declared_type)
}

/// Format one variadic pattern parameter.
fn write_variadic_pattern_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameter_id: LocalNodeId<Parameter>,
    pattern: LocalNodeId<Pattern>,
    declared_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    // variadic pattern
    write!(f, [token("..."), pattern])?;

    // type
    write_parameter_type(f, parameter_id, declared_type)
}

/// Format one parameter body.
fn format_parameter_node<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    parameter: &Parameter,
) -> FormatResult<()> {
    match parameter {
        Parameter::Named {
            name,
            visibility,
            is_readonly,
            is_optional,
            declared_type,
            default,
        } => write_named_parameter(
            f,
            node_id,
            *name,
            *visibility,
            *is_readonly,
            *is_optional,
            *declared_type,
            *default,
        ),
        Parameter::Pattern {
            pattern,
            is_optional,
            declared_type,
            default,
        } => write_pattern_parameter(f, node_id, *pattern, *is_optional, *declared_type, *default),
        Parameter::VariadicNamed {
            name,
            visibility,
            is_readonly,
            declared_type,
        } => write_variadic_named_parameter(
            f,
            node_id,
            *name,
            *visibility,
            *is_readonly,
            *declared_type,
        ),
        Parameter::VariadicPattern {
            pattern,
            declared_type,
        } => write_variadic_pattern_parameter(f, node_id, *pattern, *declared_type),
        Parameter::Error => write!(f, [token("/* ERROR */")]),
    }
}

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        node_id: LocalNodeId<Parameter>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            format_parameter_node(f, node_id, self)?;
            write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
        });

        write!(f, [group(&content)])
    }
}

/// Return whether one parameter is variadic.
pub(crate) fn parameter_is_variadic(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    matches!(
        context.tree.get(parameter_id),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
    )
}

/// Return whether one parameter list should prefer a multi-line layout.
pub(crate) fn should_break_function_parameters(
    context: &DestackFormatContext<'_>,
    parameters: &[LocalNodeId<Parameter>],
) -> bool {
    parameters.len() > 1
        && parameters
            .iter()
            .copied()
            .any(|parameter_id| parameter_has_modifier(context, parameter_id))
}

/// Return whether one single-parameter list should hug.
pub(crate) fn should_hug_function_parameters(
    context: &DestackFormatContext<'_>,
    parameters: &[LocalNodeId<Parameter>],
    can_omit_parentheses: bool,
) -> bool {
    if parameters.len() != 1 {
        return false;
    }

    let parameter_id = parameters[0];

    if parameter_is_variadic(context, parameter_id) {
        return false;
    }

    if parameter_has_modifier(context, parameter_id) {
        return false;
    }

    if single_parameter_has_paren_comments(context, parameter_id) {
        return false;
    }

    if parameter_pattern_is_destructuring(context, parameter_id) {
        return parameter_default_is_huggable(context, parameter_id);
    }

    if !parameter_is_binding_identifier(context, parameter_id)
        || parameter_has_default(context, parameter_id)
    {
        return false;
    }

    can_omit_parentheses
        || parameter_declared_type(context, parameter_id)
            .is_some_and(|type_id| type_expression_is_object_like(context, type_id))
}

/// Write one function abstraction prefix.
pub(crate) fn write_function_abstraction_prefix(
    f: &mut DestackFormatter<'_, '_>,
    is_abstract: bool,
    is_override: bool,
) -> FormatResult<()> {
    // abstraction
    if is_abstract {
        write!(f, [Keyword::Abstract, space()])?;
    }

    // override
    if is_override {
        write!(f, [Keyword::Override, space()])?;
    }

    Ok(())
}

/// Write the async keyword prefix.
pub(crate) fn write_function_asynchrony_prefix(
    f: &mut DestackFormatter<'_, '_>,
    asynchrony: Asynchrony,
) -> FormatResult<()> {
    // asynchrony
    if asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Async, space()])?;
    }

    Ok(())
}

/// Write shared function header keywords and generator markers.
pub(crate) fn write_function_header_prefix(
    f: &mut DestackFormatter<'_, '_>,
    signature: &FunctionSignature,
    is_declaration_style: bool,
    has_name_or_key: bool,
) -> FormatResult<()> {
    // abstraction
    write_function_abstraction_prefix(f, signature.is_abstract, signature.is_override)?;

    // asynchrony
    write_function_asynchrony_prefix(f, signature.asynchrony)?;

    // role
    if let Some(role) = signature.role {
        if let Some(keyword) = role.to_keyword() {
            write!(f, [keyword])?;
        }

        if has_name_or_key || role == FunctionRole::New {
            write!(f, [space()])?;
        }
    }

    // keyword and generator
    if is_declaration_style
        && signature.form == FunctionForm::Function
        && signature.role != Some(FunctionRole::Constructor)
        && signature.role != Some(FunctionRole::New)
    {
        write!(f, [Keyword::Function])?;

        if signature.is_generator {
            write!(f, [token("*")])?;
        }

        write!(f, [space()])?;
    } else if signature.is_generator {
        write!(f, [token("*")])?;

        if is_declaration_style {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Return whether one following expression should receive one separating space.
pub(crate) fn expression_body_requires_head_space(
    _context: &DestackFormatContext<'_>,
    _body: LocalNodeId<Expression>,
) -> bool {
    true
}

/// Write one grouped parameter list.
pub(crate) fn write_signature_parameter_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameters: &[LocalNodeId<Parameter>],
    disallow_trailing_parameter_separator: bool,
) -> FormatResult<()> {
    let trailing_separator = if disallow_trailing_parameter_separator {
        TrailingSeparator::Omit
    } else {
        default_generic_parameter_trailing_separator(f)
    };

    if should_break_function_parameters(f.context(), parameters) {
        let body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            for (index, parameter_id) in parameters.iter().copied().enumerate() {
                let is_last = index + 1 == parameters.len();

                write!(f, [parameter_id])?;

                if !is_last {
                    write!(f, [token(","), hard_line_break()])?;
                    continue;
                }

                match trailing_separator {
                    TrailingSeparator::Allowed | TrailingSeparator::Mandatory => {
                        write!(f, [token(",")])?;
                    }
                    TrailingSeparator::Omit => {}
                }
            }

            Ok(())
        });

        return write!(f, [token("("), block_indent(&body), token(")")]);
    }

    let body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let entries = FormatSeparatedIter::new(parameters.iter().copied(), ",")
            .with_trailing_separator(trailing_separator);

        for (index, entry) in entries.enumerate() {
            if index > 0 {
                let parameter_span = f.context().span(entry.element());
                let has_lines_before = f
                    .context()
                    .source_text()
                    .get_lines_before(parameter_span, f.context().comments())
                    > 1;

                if has_lines_before {
                    write!(f, [empty_line()])?;
                } else {
                    write!(f, [soft_line_break_or_space()])?;
                }
            }

            write!(f, [entry])?;
        }

        Ok(())
    });
    write!(f, [token("("), soft_block_indent(&body), token(")")])
}

/// Write one hugged parameter list.
pub(crate) fn write_signature_hug_parameter_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameters: &[LocalNodeId<Parameter>],
) -> FormatResult<()> {
    let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        for (index, parameter_id) in parameters.iter().copied().enumerate() {
            if index > 0 {
                write!(f, [token(","), space()])?;
            }

            write!(f, [parameter_id])?;
        }

        Ok(())
    });

    write!(f, [group(&format_args![token("("), content, token(")")])])
}

/// Write one empty parameter list with interior annotations.
pub(crate) fn write_empty_parameter_list_with_interior_comments<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    Tree: TreeStore<T>,
{
    // empty list
    if !f.context().has_infix_annotation(node_id) {
        return write!(f, [token("("), token(")")]);
    }

    // annotated empty list
    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&block_infix_annotations(f.context(), node_id)),
            token(")")
        ])]
    )
}

/// Format one where-clause list with break support.
pub(crate) fn format_where_clause_with_break<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    where_clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    let body = separated_entries(",", where_clauses, TrailingSeparator::Omit, None);
    let body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if where_clauses.len() > 1 {
            write!(
                f,
                [group(&format_args![
                    token("("),
                    soft_block_indent(&body),
                    token(")")
                ])]
            )
        } else {
            write!(f, [body])
        }
    });

    write!(
        f,
        [group(&format_args![
            soft_line_break_or_space(),
            Keyword::Where,
            space(),
            body
        ])]
    )
}

impl<'ast> FormatNode<'ast, WhereClause> for WhereClause {
    fn format_node(
        &self,
        node_id: LocalNodeId<WhereClause>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
        write!(f, [self.left, token(":"), space(), self.right])?;
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}

impl<'ast> FormatNode<'ast, GenericParameter> for GenericParameter {
    fn format_node(
        &self,
        node_id: LocalNodeId<GenericParameter>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        match self {
            GenericParameter::Type {
                name,
                is_const,
                variance,
                constraint,
                default,
            } => {
                // const
                if *is_const {
                    write!(f, [Keyword::Const, space()])?;
                }

                // variance
                write_variance_prefix(f, *variance)?;

                // name and trailers
                write!(f, [*name])?;
                write_type_parameter_constraint_and_default(f, node_id, *constraint, *default)?;
            }
            GenericParameter::Value {
                name,
                declared_type,
                default,
                is_comptime,
            } => {
                // comptime
                if *is_comptime {
                    write!(f, [Keyword::Comptime, space()])?;
                }

                // name and trailers
                write!(f, [*name])?;
                write_parameter_type(f, node_id, *declared_type)?;
                write_parameter_default(f, *default)?;
            }
            GenericParameter::Error => {
                write!(f, [token("/* ERROR */")])?;
            }
        }

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}
