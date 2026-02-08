use std::collections::HashMap;

use crate::argument::list_like;
use crate::directive::{
    FormatterDirectiveKind, FormatterDirectivePosition, collect_comment_tokens, directive_for_node,
    ignore_range_for_node, ignored_node_source, write_ignored_span,
};
use crate::format::block::format_block_of_statements;
use crate::key::{format_key_with_quote_policy, is_identifier_for_quotes};
use crate::r#where::format_where_clause_with_break;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AbstractionModifier, AccessorKind, Annotation, AnnotationPosition, Asynchrony, BindingAnchor,
    BindingKind, BindingModifier, BindingOperator, Comment, CommentStyle, Declaration, Expression,
    FunctionAbstraction, FunctionCardinality, FunctionMode, Key, Keyword, LocalNodeId, Member,
    Mutability, Name, NodeType, Parameter, Property, Timing, VarianceModifier,
};
use destack_fir::format::{FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_source::Span;
use destack_workspace::QuoteProperty;

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
    // scope
    if modifiers.anchor == Some(BindingAnchor::Static) {
        write!(f, [Keyword::Static, space()])?;
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

#[inline]
pub(crate) fn format_binding_modifiers_postfix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: BindingModifier,
) -> FormatResult<()> {
    // kind
    if modifiers.kind == Some(BindingKind::Must)
        && modifiers.accessor != Some(AccessorKind::Accessor)
    {
        write!(f, [token("!")])?;
    } else if modifiers.kind == Some(BindingKind::Maybe) {
        write!(f, [token("?")])?;
    }
    Ok(())
}

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

/// Return whether a parameter declares any modifiers.
fn parameter_has_modifier(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    match context.tree.get(parameter_id) {
        Parameter::Named { modifiers, .. }
        | Parameter::Pattern { modifiers, .. }
        | Parameter::VariadicNamed { modifiers, .. }
        | Parameter::VariadicPattern { modifiers, .. } => modifiers.is_some(),
    }
}

/// Return whether constructor parameter lists should break by default.
fn constructor_parameters_should_expand(
    context: &DestackFormatContext<'_>,
    mode: Option<FunctionMode>,
    parameters: &[LocalNodeId<Parameter>],
) -> bool {
    matches!(mode, Some(FunctionMode::Constructor | FunctionMode::New))
        && parameters.len() > 1
        && parameters
            .iter()
            .any(|parameter_id| parameter_has_modifier(context, *parameter_id))
}

/// Return whether this parameter is variadic.
fn parameter_is_variadic(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    matches!(
        context.tree.get(parameter_id),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
    )
}

/// Return whether a single parameter should keep compact outer parentheses.
fn single_parameter_should_hug(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    if parameter_is_variadic(context, parameter_id) {
        return false;
    }

    let has_multiline_collection_default = match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => default
            .is_some_and(|default_id| {
                matches!(
                    context.tree.get(default_id),
                    Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. }
                ) && context.has_newline(context.get_span(default_id))
            }),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => false,
    };
    if has_multiline_collection_default {
        return false;
    }

    let has_newline = context.has_newline(context.get_span(parameter_id));
    match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } => !(has_newline && default.is_some()),
        Parameter::VariadicNamed { .. } => !has_newline,
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => true,
    }
}

/// Return whether spacing before a function body should be emitted by annotations.
fn should_elide_space_before_body(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    return_type.is_some_and(|return_type| context.has_postfix_annotation(return_type))
}

/// Return whether a signature return type is already multiline in source.
fn return_type_is_multiline(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    return_type.is_some_and(|return_type| context.has_newline(context.get_span(return_type)))
}

/// Return whether property method signature source spans multiple lines before the body.
fn property_method_signature_source_is_multiline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Property>,
    body: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(body_id) = body else {
        return false;
    };

    let node_span = context.get_span(node_id);
    let body_span = context.get_span(body_id);
    if node_span.file != body_span.file || node_span.start >= body_span.start {
        return false;
    }

    let signature_span = Span::new(node_span.file, node_span.start, body_span.start);
    context.get_span_str(signature_span).contains('\n')
}

/// Return whether member method signature source spans multiple lines before the body.
fn member_method_signature_source_is_multiline(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Member>,
    body: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(body_id) = body else {
        return false;
    };

    let node_span = context.get_span(node_id);
    let body_span = context.get_span(body_id);
    if node_span.file != body_span.file || node_span.start >= body_span.start {
        return false;
    }

    let signature_span = Span::new(node_span.file, node_span.start, body_span.start);
    context.get_span_str(signature_span).contains('\n')
}

/// Return the first non-whitespace character after an annotation span.
fn next_non_whitespace_after_annotation(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<char> {
    let span = context.get_span::<Annotation>(annotation_id);
    if span.end >= context.file.len {
        return None;
    }

    let tail_span = Span::new(span.file, span.end, context.file.len);
    let tail_source = context.file.get_span_str(tail_span)?;
    tail_source
        .chars()
        .find(|character: &char| !character.is_whitespace())
}

/// Collect deferred method boundary line comments from return type and body.
fn collect_deferred_method_boundary_line_comments(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
    body: LocalNodeId<Expression>,
) -> Vec<String> {
    let mut comments: Vec<(u32, String)> = Vec::new();

    if let Some(return_type) = return_type
        && let Some(annotations) = context.get_annotations(return_type)
    {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } = context.tree.get(annotation_id) else {
                continue;
            };
            if *position != AnnotationPosition::LinePostfixBoundary {
                continue;
            }
            let comment = context.tree.get::<Comment>(*node);
            if comment.style != CommentStyle::Slash {
                continue;
            }
            if next_non_whitespace_after_annotation(context, annotation_id) != Some('{') {
                continue;
            }

            let annotation_span = context.get_span::<Annotation>(annotation_id);
            let annotation_source = context.get_span_str(annotation_span).trim().to_string();
            comments.push((annotation_span.start, annotation_source));
        }
    }

    if let Some(annotations) = context.get_annotations(body) {
        for annotation_id in annotations {
            let Annotation::Comment { node, position } = context.tree.get(annotation_id) else {
                continue;
            };
            if *position != AnnotationPosition::BlockPrefix {
                continue;
            }
            let comment = context.tree.get::<Comment>(*node);
            if comment.style != CommentStyle::Slash {
                continue;
            }

            let annotation_span = context.get_span::<Annotation>(annotation_id);
            let annotation_source = context.get_span_str(annotation_span).trim().to_string();
            comments.push((annotation_span.start, annotation_source));
        }
    }

    comments.sort_by_key(|(start, _)| *start);
    comments
        .into_iter()
        .map(|(_, comment)| comment)
        .collect::<Vec<_>>()
}

/// Return whether a method body has deferred boundary line comments.
fn method_body_has_deferred_boundary_line_comments(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
    body: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(body) = body else {
        return false;
    };
    matches!(context.tree.get(body), Expression::Block(_))
        && !collect_deferred_method_boundary_line_comments(context, return_type, body).is_empty()
}

/// Format a method body block with deferred boundary line comments inside braces.
fn format_method_body_block_with_deferred_boundary_line_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    body: LocalNodeId<Expression>,
    comments: &[String],
) -> FormatResult<()> {
    let Expression::Block(block_id) = f.context().tree.get(body) else {
        return write!(f, [body]);
    };

    let block = f.context().tree.get(*block_id);
    let has_block_infix_annotations = f.context().has_infix_annotation(*block_id);
    write!(
        f,
        [
            token("{"),
            hard_line_break(),
            soft_block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let mut has_content = false;
                for (index, comment) in comments.iter().enumerate() {
                    if index > 0 {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [text(comment.as_str())])?;
                    has_content = true;
                }

                if !comments.is_empty() && !block.expressions.is_empty() {
                    write!(f, [hard_line_break()])?;
                }

                if !block.expressions.is_empty() {
                    format_block_of_statements(f, &block.expressions)?;
                    has_content = true;
                }

                if has_block_infix_annotations {
                    if has_content {
                        write!(f, [hard_line_break()])?;
                    }
                    write!(f, [f.context().block_infix_annotations(*block_id)])?;
                }

                Ok(())
            })),
            hard_line_break(),
            token("}")
        ]
    )
}

/// Format a block of properties with appropriate empty annotations.
#[allow(unused)]
pub(crate) fn format_block_of_properties<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    properties: &[LocalNodeId<Property>],
    separator: &'static str,
) -> FormatResult<()> {
    let comment_tokens = collect_comment_tokens(f.context());
    let mut ignore_ranges: HashMap<u32, Span> = HashMap::new();
    for &property_id in properties {
        if let Some(range_span) = ignore_range_for_node(f.context(), property_id, &comment_tokens) {
            ignore_ranges.insert(property_id.id, range_span);
        }
    }

    let mut skip_until: Option<u32> = None;
    for (i, &property_id) in properties.iter().enumerate() {
        let property = f.context().tree.get(property_id);
        let property_span = f.context().get_span(property_id);

        if let Some(skip_end) = skip_until {
            if property_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between properties
        if i > 0 {
            write!(f, [hard_line_break()])?;
        }

        if let Some(range_span) = ignore_ranges.get(&property_id.id) {
            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        property_id.format(f)?;
        // comma after field properties
        if matches!(
            property,
            Property::Field { .. } | Property::Method { .. } | Property::Spread { .. }
        ) {
            write!(f, [token(separator)])?;
        }
    }
    Ok(())
}

/// Format a block of members with appropriate empty annotations.
#[allow(unused)]
pub(crate) fn format_block_of_members<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    let comment_tokens = collect_comment_tokens(f.context());
    let mut ignore_ranges: HashMap<u32, Span> = HashMap::new();
    for &member_id in members {
        if let Some(range_span) = ignore_range_for_node(f.context(), member_id, &comment_tokens) {
            ignore_ranges.insert(member_id.id, range_span);
        }
    }

    let mut skip_until: Option<u32> = None;
    for (i, &member_id) in members.iter().enumerate() {
        let member_span = f.context().get_span(member_id);

        if let Some(skip_end) = skip_until {
            if member_span.start < skip_end {
                continue;
            }
            skip_until = None;
        }

        // blank line between members
        if i > 0 {
            write!(f, [hard_line_break()])?;
        }

        if let Some(range_span) = ignore_ranges.get(&member_id.id) {
            write_ignored_span(f, *range_span)?;
            skip_until = Some(range_span.end);
            continue;
        }

        member_id.format(f)?;
    }
    Ok(())
}

/// Check whether a key requires quotes under identifier rules.
#[inline]
fn key_requires_quotes<'ast>(f: &DestackFormatter<'ast, '_>, key: Key) -> bool {
    let strings = f.context().strings;
    match key {
        Key::Name(Name::String(string_id)) => {
            let content = strings.get(string_id);
            !is_identifier_for_quotes(content)
        }
        _ => false,
    }
}

/// Check whether any object key forces consistent quoting.
#[inline]
fn force_quote_keys_for_object<'ast>(
    f: &DestackFormatter<'ast, '_>,
    properties: &[LocalNodeId<Property>],
) -> bool {
    properties.iter().any(|property_id| {
        let property = f.context().tree.get(*property_id);
        match property {
            Property::Field { key, .. } | Property::Method { key, .. } => {
                key.is_some_and(|key| key_requires_quotes(f, key))
            }
            Property::Spread { .. } => false,
        }
    })
}

/// Check whether any type member key forces consistent quoting.
#[inline]
fn force_quote_keys_for_members<'ast>(
    f: &DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> bool {
    members.iter().any(|member_id| {
        let member = f.context().tree.get(*member_id);
        match member {
            Member::Field { key, .. } | Member::Method { key, .. } => {
                key.is_some_and(|key| key_requires_quotes(f, key))
            }
            _ => false,
        }
    })
}

/// Decide whether this property should force consistent key quoting.
#[inline]
fn should_force_quote_keys_for_property<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Property>,
) -> bool {
    if f.context().options.quote_props != QuoteProperty::Consistent {
        return false;
    }

    if f.context().options.language_type.is_destack() {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().get_parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    let Expression::ObjectExpression { properties, .. } = f.context().tree.get(parent_id) else {
        return false;
    };

    force_quote_keys_for_object(f, properties)
}

/// Decide whether this member should force consistent key quoting.
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

    let Some((parent_id, parent_type)) = f.context().get_parent(node_id) else {
        return false;
    };

    if parent_type != NodeType::Declaration {
        return false;
    }

    let parent_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Class { members, .. } = f.context().tree.get(parent_id) else {
        return false;
    };

    force_quote_keys_for_members(f, members)
}

/// Decide whether a field default should stay inline after `=`.
fn should_keep_field_default_inline<'ast>(
    f: &DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if f.context().has_prefix_annotation(expression_id) {
        return false;
    }

    let is_call_like = matches!(
        f.context().tree.get(expression_id),
        Expression::Call { .. } | Expression::New { .. } | Expression::Instantiation { .. }
    );
    if !is_call_like {
        return false;
    }

    // preserve inline `= <expr>` when source already uses multiline rhs structure
    if f.context().has_newline(f.context().get_span(expression_id)) {
        return true;
    }

    // single-line call-like defaults should only stay inline when short
    let line_width = usize::from(f.context().options.line_width);
    let expression_len = f
        .context()
        .get_span_str(f.context().get_span(expression_id))
        .chars()
        .count();

    expression_len <= line_width / 2
}

/// Write a field-like type annotation after `:`.
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

impl<'ast> FormatNode<'ast, Property> for Property {
    fn format_node(
        &self,
        node_id: LocalNodeId<Property>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // ignore formatting when requested
        let directive = directive_for_node(f.context(), node_id);

        // prefix annotations
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // raw node formatting for ignore directives
        if let Some(directive) = directive
            && directive.kind == FormatterDirectiveKind::IgnoreFormat
        {
            let raw_property = ignored_node_source(f.context(), node_id, directive);
            write!(f, [text(&raw_property)])?;

            if !matches!(
                directive.position,
                FormatterDirectivePosition::Postfix { .. }
            ) {
                write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
            }

            return Ok(());
        }

        match self {
            Property::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                let force_quote_keys = should_force_quote_keys_for_property(f, node_id);

                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                if let Some(key) = key {
                    format_key_with_quote_policy(f, *key, force_quote_keys)?;
                }
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                if let Some(value) = value {
                    write_field_type_annotation(f, *value)?;
                }
                // default
                if let Some(default) = default {
                    if should_keep_field_default_inline(f, *default) {
                        write!(
                            f,
                            [group(&format_args![space(), token("="), space(), default])]
                        )?;
                    } else {
                        write!(
                            f,
                            [group(&format_args![
                                space(),
                                token("="),
                                indent(&format_args![soft_line_break_or_space(), default]),
                            ])]
                        )?;
                    }
                }
            }
            Property::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                let generics = signature.generics.as_ref();
                let force_quote_keys = should_force_quote_keys_for_property(f, node_id);
                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                // abstraction
                match signature.abstraction {
                    FunctionAbstraction::Abstract => {
                        write!(f, [Keyword::Abstract, space()])?;
                    }
                    FunctionAbstraction::AbstractOverride => {
                        write!(f, [Keyword::Abstract, space()])?;
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::ConcreteOverride => {
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::Concrete => {}
                }

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // mode
                if let Some(mode) = signature.mode {
                    if let Some(keyword) = mode.to_keyword() {
                        write!(f, [keyword])?;
                    }
                    if key.is_some() || matches!(mode, FunctionMode::New) {
                        write!(f, [space()])?;
                    }
                }

                // cardinality
                if signature.cardinality == FunctionCardinality::Generator {
                    write!(f, [token("*")])?;
                }

                // key
                if let Some(key) = key {
                    format_key_with_quote_policy(f, *key, force_quote_keys)?;
                }

                // static parameters
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // dynamic parameters
                let signature_source_is_multiline_at_80 =
                    usize::from(f.context().options.line_width) <= 80
                        && property_method_signature_source_is_multiline(
                            f.context(),
                            node_id,
                            *body,
                        );
                if signature.dynamic_parameters.len() == 1
                    && single_parameter_should_hug(f.context(), signature.dynamic_parameters[0])
                    && !return_type_is_multiline(f.context(), signature.return_type)
                    && !signature_source_is_multiline_at_80
                {
                    write!(f, [token("("), signature.dynamic_parameters[0], token(")")])?;
                } else {
                    let should_break_constructor_parameters = constructor_parameters_should_expand(
                        f.context(),
                        signature.mode,
                        &signature.dynamic_parameters,
                    );
                    let should_expand_single_for_multiline_return_type =
                        signature.dynamic_parameters.len() == 1
                            && !parameter_is_variadic(f.context(), signature.dynamic_parameters[0])
                            && return_type_is_multiline(f.context(), signature.return_type);
                    let should_expand_parameters = should_break_constructor_parameters
                        || should_expand_single_for_multiline_return_type;
                    let mut dynamic_parameters_list =
                        list_like("(", ")", ",", &signature.dynamic_parameters);
                    dynamic_parameters_list.should_expand(should_expand_parameters);
                    write!(f, [dynamic_parameters_list])?;
                }

                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;

                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [token(":"), space(), return_type])?;
                }

                // where clauses
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    format_where_clause_with_break(f, where_clauses)?;
                }

                // body
                if let Some(body) = body {
                    if method_body_has_deferred_boundary_line_comments(
                        f.context(),
                        signature.return_type,
                        Some(*body),
                    ) {
                        let comments = collect_deferred_method_boundary_line_comments(
                            f.context(),
                            signature.return_type,
                            *body,
                        );
                        write!(f, [space()])?;
                        format_method_body_block_with_deferred_boundary_line_comments(
                            f, *body, &comments,
                        )?;
                    } else if should_elide_space_before_body(f.context(), signature.return_type) {
                        write!(f, [body])?;
                    } else {
                        write!(f, [space(), body])?;
                    }
                }
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
        }

        // postfix annotations
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Member> for Member {
    fn format_node(
        &self,
        node_id: LocalNodeId<Member>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // ignore formatting when requested
        let directive = directive_for_node(f.context(), node_id);

        // prefix annotations
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // raw node formatting for ignore directives
        if let Some(directive) = directive
            && directive.kind == FormatterDirectiveKind::IgnoreFormat
        {
            let raw_member = ignored_node_source(f.context(), node_id, directive);
            write!(f, [text(&raw_member)])?;

            if !matches!(
                directive.position,
                FormatterDirectivePosition::Postfix { .. }
            ) {
                write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
            }

            return Ok(());
        }

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
                // keep non-comptime modifiers before the associated keyword pair
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

                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;
                // key
                if let Some(key) = key {
                    format_key_with_quote_policy(f, *key, force_quote_keys)?;
                }
                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                // value
                if let Some(value) = value {
                    write_field_type_annotation(f, *value)?;
                }
                // default
                if let Some(default) = default {
                    if should_keep_field_default_inline(f, *default) {
                        write!(
                            f,
                            [group(&format_args![space(), token("="), space(), default])]
                        )?;
                    } else {
                        write!(
                            f,
                            [group(&format_args![
                                space(),
                                token("="),
                                indent(&format_args![soft_line_break_or_space(), default]),
                            ])]
                        )?;
                    }
                }
            }
            Member::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                let generics = signature.generics.as_ref();
                let force_quote_keys = should_force_quote_keys_for_member(f, node_id);

                // modifiers
                format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                // abstraction
                match signature.abstraction {
                    FunctionAbstraction::Abstract => {
                        write!(f, [Keyword::Abstract, space()])?;
                    }
                    FunctionAbstraction::AbstractOverride => {
                        write!(f, [Keyword::Abstract, space()])?;
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::ConcreteOverride => {
                        write!(f, [Keyword::Override, space()])?;
                    }
                    FunctionAbstraction::Concrete => {}
                }

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // mode
                if let Some(mode) = signature.mode {
                    if let Some(keyword) = mode.to_keyword() {
                        write!(f, [keyword])?;
                    }
                    if key.is_some() || matches!(mode, FunctionMode::New) {
                        write!(f, [space()])?;
                    }
                }

                // cardinality
                if signature.cardinality == FunctionCardinality::Generator {
                    write!(f, [token("*")])?;
                }

                // key
                if let Some(key) = key {
                    format_key_with_quote_policy(f, *key, force_quote_keys)?;
                }

                // static parameters
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    write!(f, [list_like("<", ">", ",", static_parameters)])?;
                }

                // dynamic parameters
                let signature_source_is_multiline_at_80 =
                    usize::from(f.context().options.line_width) <= 80
                        && member_method_signature_source_is_multiline(f.context(), node_id, *body);
                if signature.dynamic_parameters.len() == 1
                    && single_parameter_should_hug(f.context(), signature.dynamic_parameters[0])
                    && !return_type_is_multiline(f.context(), signature.return_type)
                    && !signature_source_is_multiline_at_80
                {
                    write!(f, [token("("), signature.dynamic_parameters[0], token(")")])?;
                } else {
                    let should_break_constructor_parameters = constructor_parameters_should_expand(
                        f.context(),
                        signature.mode,
                        &signature.dynamic_parameters,
                    );
                    let should_expand_single_for_multiline_return_type =
                        signature.dynamic_parameters.len() == 1
                            && !parameter_is_variadic(f.context(), signature.dynamic_parameters[0])
                            && return_type_is_multiline(f.context(), signature.return_type);
                    let should_expand_parameters = should_break_constructor_parameters
                        || should_expand_single_for_multiline_return_type;
                    let mut dynamic_parameters_list =
                        list_like("(", ")", ",", &signature.dynamic_parameters);
                    dynamic_parameters_list.should_expand(should_expand_parameters);
                    write!(f, [dynamic_parameters_list])?;
                }

                // modifiers
                format_binding_modifiers_postfix_maybe(f, *modifiers)?;

                // return type
                if let Some(return_type) = signature.return_type {
                    write!(f, [token(":"), space(), return_type])?;
                }

                // where clauses
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    format_where_clause_with_break(f, where_clauses)?;
                }

                // body
                if let Some(body) = body {
                    if method_body_has_deferred_boundary_line_comments(
                        f.context(),
                        signature.return_type,
                        Some(*body),
                    ) {
                        let comments = collect_deferred_method_boundary_line_comments(
                            f.context(),
                            signature.return_type,
                            *body,
                        );
                        write!(f, [space()])?;
                        format_method_body_block_with_deferred_boundary_line_comments(
                            f, *body, &comments,
                        )?;
                    } else if should_elide_space_before_body(f.context(), signature.return_type) {
                        write!(f, [body])?;
                    } else {
                        write!(f, [space(), body])?;
                    }
                }
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
        }

        let needs_semicolon = matches!(
            self,
            Member::Field { .. } | Member::Method { body: None, .. }
        );
        if needs_semicolon {
            write!(f, [token(";")])?;
        }

        // postfix annotations
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::DeclarationDescriptor;
    use destack_source::LanguageType;

    #[test]
    fn test_format_struct_empty() {
        assert_format!(
            "struct Foo { }",
            "struct Foo {}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_fields() {
        assert_format!(
            "struct Foo { a: int32, b: boolean }",
            "struct Foo {\n\ta: int32;\n\tb: boolean;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_modified_fields() {
        assert_format!(
            "struct Foo { readonly a: int32, private b: boolean }",
            "struct Foo {\n\treadonly a: int32;\n\tprivate b: boolean;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_name() {
        assert_format!(
            "struct Foo { a: int32 }",
            "struct Foo {\n\ta: int32;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct Foo { a?: int32 = 42, b: boolean }",
            "struct Foo {\n\ta?: int32 = 42;\n\tb: boolean;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_class_with_abstract_override_field() {
        assert_format!(
            "class Foo { abstract override bar: int32 }",
            "class Foo {\n\tabstract override bar: int32;\n}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions {
                language_type: LanguageType::TypeScript,
                ..DestackFormatOptions::default_tab()
            }
        );
    }

    #[test]
    fn test_format_struct_with_static_parameters_and_inheritance() {
        assert_format!(
            "struct Foo<T: Numeric> extends Bar implements Baz { }",
            "struct Foo<T: Numeric> extends Bar implements Baz {}",
            |p| p.eat_struct_or_class(&p.mark(), DeclarationDescriptor::default(), false),
            DestackFormatOptions::default()
        );
    }
}
