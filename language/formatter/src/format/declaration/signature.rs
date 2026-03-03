use crate::format::analysis::{
    next_non_whitespace_token_after_annotation, previous_non_whitespace_token_before_annotation,
};
use crate::format::call::{SeparatorLineCommentSource, write_separator_line_comment_after_comma};
use crate::format::collection::list_like;
use crate::format::collection::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AbstractionModifier, AnnotationPosition, Asynchrony, Comment, CommentStyle, Declaration,
    Expression, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature, Keyword, LocalNodeId, Member, Mutability, Node, NodeTree, NodeTreeImpl,
    NodeType, Parameter, Pattern, PatternField, Property, TokenType, WhereClause,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::Span;
use destack_workspace::TrailingComma;

// signature expansion thresholds
const CONSTRUCTOR_PARAMETER_EXPAND_MIN_COUNT: usize = 2;
const OBJECT_PATTERN_FORCE_EXPAND_MIN_FIELDS: usize = 3;
const OBJECT_PATTERN_INLINE_MAX_FIELDS: usize = 1;

/// Write one parameter type with annotation-aware infix spacing.
fn write_parameter_type_with_infix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    is_static_parameter: bool,
    ty: Option<LocalNodeId<Expression>>,
) -> FormatResult<bool> {
    let Some(ty) = ty else {
        return Ok(false);
    };

    write!(f, [f.context().block_infix_annotations(node_id)])?;
    if is_static_parameter {
        write!(f, [space(), Keyword::Extends, space(), ty])?;
    } else {
        let has_infix_annotations = f.context().has_infix_annotation(node_id);
        if has_infix_annotations {
            write!(f, [space(), token(":"), space(), ty])?;
        } else {
            write!(f, [token(":"), space(), ty])?;
        }
    }

    Ok(true)
}

/// Return whether all prefix annotations on one variadic parameter follow `...`.
fn parameter_prefix_annotations_follow_spread(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let Some(annotations) = context.annotations(parameter_id) else {
        return false;
    };

    let mut has_prefix_annotation = false;
    for annotation_id in annotations {
        let position = context.annotation(annotation_id).position();
        if !matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        ) {
            continue;
        }

        has_prefix_annotation = true;
        let previous_token =
            previous_non_whitespace_token_before_annotation(context, annotation_id);
        if !previous_token.is_some_and(|token| token.token.ty == TokenType::Spread) {
            return false;
        }
    }

    has_prefix_annotation
}

/// Format one parameter node with optional separator-boundary annotation suppression.
fn format_parameter_node<'ast>(
    parameter: &Parameter,
    node_id: LocalNodeId<Parameter>,
    f: &mut DestackFormatter<'ast, '_>,
    suppress_separator_boundary_annotations: bool,
) -> FormatResult<()> {
    let is_variadic_parameter = matches!(
        parameter,
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
    );
    let defer_prefix_annotations_after_spread =
        is_variadic_parameter && parameter_prefix_annotations_follow_spread(f.context(), node_id);
    if !defer_prefix_annotations_after_spread {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;
    }

    let is_typescript = f.context().options.language_type.is_typescript();
    let is_static_parameter = is_typescript && parameter_is_static(f.context(), node_id);
    let wrote_type_infix = match parameter {
        Parameter::Named {
            modifiers,
            name,
            ty,
            default,
        } => {
            // modifiers
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;

            // name
            write!(f, [name])?;

            // modifiers
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;

            // type
            let wrote_type_infix =
                write_parameter_type_with_infix(f, node_id, is_static_parameter, *ty)?;

            // default
            if let Some(default) = default {
                write!(f, [space(), token("="), space(), default])?;
            }
            wrote_type_infix
        }
        Parameter::Pattern {
            modifiers,
            pattern,
            ty,
            default,
        } => {
            // modifiers
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;

            // pattern
            write!(f, [pattern])?;

            // modifiers
            format_binding_modifiers_postfix_maybe(f, *modifiers)?;

            // type
            let wrote_type_infix =
                write_parameter_type_with_infix(f, node_id, is_static_parameter, *ty)?;

            // default
            if let Some(default) = default {
                write!(f, [space(), token("="), space(), default])?;
            }
            wrote_type_infix
        }
        Parameter::VariadicNamed {
            modifiers,
            name,
            ty,
        } => {
            // modifiers
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;

            // keyword
            write!(f, [token("...")])?;

            // spread seam prefix annotations
            if defer_prefix_annotations_after_spread {
                write!(f, [f.context().any_prefix_annotations(node_id)])?;
            }

            // name
            write!(f, [name])?;

            // type
            write_parameter_type_with_infix(f, node_id, is_static_parameter, *ty)?
        }
        Parameter::VariadicPattern {
            modifiers,
            pattern,
            ty,
        } => {
            // modifiers
            format_binding_modifiers_prefix_maybe(f, *modifiers)?;

            // keyword
            write!(f, [token("...")])?;

            // spread seam prefix annotations
            if defer_prefix_annotations_after_spread {
                write!(f, [f.context().any_prefix_annotations(node_id)])?;
            }

            // pattern
            write!(f, [pattern])?;

            // type
            write_parameter_type_with_infix(f, node_id, is_static_parameter, *ty)?
        }
    };

    if wrote_type_infix {
        if suppress_separator_boundary_annotations {
            write!(
                f,
                [f.context()
                    .any_postfix_except_line_postfix_boundary_annotations(node_id)]
            )?;
        } else {
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        }
    } else if suppress_separator_boundary_annotations {
        write!(
            f,
            [f.context()
                .any_infix_or_postfix_except_line_postfix_boundary_annotations(node_id)]
        )?;
    } else {
        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        node_id: LocalNodeId<Parameter>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        format_parameter_node(self, node_id, f, false)
    }
}

/// Return whether a parameter is declared in one static parameter list.
fn parameter_is_static(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let Some((parent_id, parent_type)) = context.parent(parameter_id) else {
        return false;
    };

    match parent_type {
        NodeType::Declaration => {
            let declaration = context.tree.get(LocalNodeId::<Declaration>::new(parent_id));
            declaration
                .static_parameters()
                .is_some_and(|parameters| parameters.contains(&parameter_id))
        }
        NodeType::Property => {
            let property = context.tree.get(LocalNodeId::<Property>::new(parent_id));
            let Property::Method { signature, .. } = property else {
                return false;
            };

            signature
                .generics
                .as_ref()
                .and_then(|generics| generics.static_parameters.as_ref())
                .is_some_and(|parameters| parameters.contains(&parameter_id))
        }
        NodeType::Member => {
            let member = context.tree.get(LocalNodeId::<Member>::new(parent_id));
            match member {
                Member::Type {
                    static_parameters, ..
                } => static_parameters
                    .as_ref()
                    .is_some_and(|parameters| parameters.contains(&parameter_id)),
                Member::Method { signature, .. } => signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    .is_some_and(|parameters| parameters.contains(&parameter_id)),
                Member::Field { .. }
                | Member::ComptimeConst { .. }
                | Member::Embed { .. }
                | Member::StaticBlock { .. }
                | Member::ComptimeBlock { .. } => false,
            }
        }
        _ => false,
    }
}

/// Return whether this parameter is variadic.
pub(crate) fn parameter_is_variadic(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    matches!(
        context.tree.get(parameter_id),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
    )
}

/// Return whether this parameter declares any modifiers.
fn parameter_has_constructor_property_modifier(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let modifiers = match context.tree.get(parameter_id) {
        Parameter::Named { modifiers, .. }
        | Parameter::Pattern { modifiers, .. }
        | Parameter::VariadicNamed { modifiers, .. }
        | Parameter::VariadicPattern { modifiers, .. } => *modifiers,
    };

    let Some(modifiers) = modifiers else {
        return false;
    };

    let has_visibility_modifier = modifiers.visibility.is_some();
    let has_readonly_modifier = modifiers.mutability == Some(Mutability::Immutable);
    let has_override_modifier = matches!(
        modifiers.abstraction,
        Some(AbstractionModifier::Override | AbstractionModifier::AbstractOverride)
    );
    has_visibility_modifier || has_readonly_modifier || has_override_modifier
}

/// Return whether parameter lists with modifier parameters should break by default.
pub(crate) fn parameters_with_modifiers_should_expand(
    context: &DestackFormatContext<'_>,
    parameters: &[LocalNodeId<Parameter>],
) -> bool {
    parameters.len() >= CONSTRUCTOR_PARAMETER_EXPAND_MIN_COUNT
        && parameters
            .iter()
            .any(|parameter_id| parameter_has_constructor_property_modifier(context, *parameter_id))
}

/// Return whether this parameter has any slash comment annotation.
fn parameter_has_line_comment_annotation(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    if !context.has_annotation(parameter_id) {
        return false;
    }

    context
        .visit_annotations(parameter_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let Annotation::Comment { node, .. } = context.annotation(*annotation_id) else {
                    return false;
                };

                let comment = context.tree.get::<Comment>(node);
                comment.style == CommentStyle::Slash
            })
        })
        .unwrap_or(false)
}

/// Return separator-comment facts for one parameter annotation.
fn parameter_separator_line_comment_annotation_info(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> Option<(LocalNodeId<Comment>, bool, bool)> {
    let Annotation::Comment { node, .. } = context.annotation(annotation_id) else {
        return None;
    };

    let comment = context.tree.get::<Comment>(node);
    // separator detachment supports slash comments and own-line block comments
    let supports_separator_detachment = match comment.style {
        CommentStyle::Slash => true,
        CommentStyle::Star => context.annotation_starts_on_own_line(annotation_id),
    };
    if !supports_separator_detachment {
        return None;
    }

    let annotation_span = context.annotation_span(annotation_id);
    let previous_token = previous_non_whitespace_token_before_annotation(context, annotation_id);
    let next_token = next_non_whitespace_token_after_annotation(context, annotation_id);
    let has_preceding_separator =
        previous_token.is_some_and(|token| token.token.ty == TokenType::Comma);
    let has_following_separator_before_close_parenthesis = next_token
        .is_some_and(|token| token.token.ty == TokenType::Comma)
        && next_token.is_some_and(|token| {
            context
                .next_non_whitespace_token_after_span(token.span)
                .is_some_and(|after_separator| {
                    after_separator.token.ty == TokenType::CloseParenthesis
                })
        });
    let has_following_close_parenthesis =
        next_token.is_some_and(|token| token.token.ty == TokenType::CloseParenthesis);
    let has_virtual_trailing_separator = !has_preceding_separator
        && !has_following_separator_before_close_parenthesis
        && has_following_close_parenthesis
        && context.annotation_starts_on_own_line(annotation_id);
    if !has_preceding_separator
        && !has_following_separator_before_close_parenthesis
        && !has_virtual_trailing_separator
    {
        return None;
    }

    let is_own_line = if has_virtual_trailing_separator {
        context.annotation_starts_on_own_line(annotation_id)
    } else if let Some(separator_token) = previous_token {
        if separator_token.token.ty != TokenType::Comma {
            false
        } else {
            let before_comment_span = Span::new(
                annotation_span.file,
                separator_token.span.end,
                annotation_span.start,
            );
            context.has_newline(before_comment_span)
        }
    } else if has_following_separator_before_close_parenthesis {
        if let Some(separator_token) = next_token {
            let after_comment_span = Span::new(
                annotation_span.file,
                annotation_span.end,
                separator_token.span.start,
            );
            context.has_newline(after_comment_span)
        } else {
            false
        }
    } else {
        context.annotation_starts_on_own_line(annotation_id)
    };

    let has_blank_line_before_first_comment = if has_virtual_trailing_separator {
        if let Some(previous_token) = previous_token {
            let before_comment_span = Span::new(
                annotation_span.file,
                previous_token.span.end,
                annotation_span.start,
            );
            context.has_blank_line(before_comment_span)
        } else {
            false
        }
    } else if let Some(separator_token) = previous_token {
        if separator_token.token.ty != TokenType::Comma {
            false
        } else {
            let before_comment_span = Span::new(
                annotation_span.file,
                separator_token.span.end,
                annotation_span.start,
            );
            context.has_blank_line(before_comment_span)
        }
    } else if has_following_separator_before_close_parenthesis {
        if let Some(separator_token) = next_token {
            let before_separator_span = Span::new(
                annotation_span.file,
                annotation_span.end,
                separator_token.span.start,
            );
            context.has_blank_line(before_separator_span)
        } else {
            false
        }
    } else {
        false
    };

    Some((node, is_own_line, has_blank_line_before_first_comment))
}

/// Return one separator line-comment source for one parameter.
fn parameter_separator_line_comment_source(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> Option<SeparatorLineCommentSource> {
    let annotations = context.annotations(parameter_id)?;
    for (index, annotation_id) in annotations.iter().copied().enumerate() {
        let Some((comment_id, is_own_line, has_blank_line_before_first_comment)) =
            parameter_separator_line_comment_annotation_info(context, annotation_id)
        else {
            continue;
        };

        let mut comment_ids = vec![comment_id];
        for next_annotation_id in annotations.iter().skip(index + 1).copied() {
            if matches!(
                context.annotation(next_annotation_id),
                Annotation::Blank { .. }
            ) {
                continue;
            }

            let Some((next_comment_id, _, _)) =
                parameter_separator_line_comment_annotation_info(context, next_annotation_id)
            else {
                break;
            };

            comment_ids.push(next_comment_id);
        }

        return Some(SeparatorLineCommentSource {
            comment_ids,
            is_own_line,
            has_blank_line_before_first_comment,
            detached_from_following_prefix: false,
        });
    }

    None
}

/// Return whether one parameter has boundary-postfix annotations that are not separator comments.
fn parameter_has_non_separator_boundary_postfix_annotation(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    context
        .visit_annotations(parameter_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let position = context.annotation(*annotation_id).position();
                if position != AnnotationPosition::LinePostfixBoundary {
                    return false;
                }

                if matches!(context.annotation(*annotation_id), Annotation::Blank { .. }) {
                    return false;
                }

                parameter_separator_line_comment_annotation_info(context, *annotation_id).is_none()
            })
        })
        .unwrap_or(false)
}

/// Return whether one parameter supports separator-comment detachment rendering.
fn parameter_can_render_without_separator_line_comment(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    !parameter_has_non_separator_boundary_postfix_annotation(context, parameter_id)
}

/// Write one parameter without separator-boundary comments when detachment is allowed.
fn write_parameter_without_separator_line_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameter_id: LocalNodeId<Parameter>,
) -> FormatResult<bool> {
    if !parameter_can_render_without_separator_line_comment(f.context(), parameter_id) {
        return Ok(false);
    }

    let parameter = f.context().tree.get(parameter_id);
    format_parameter_node(parameter, parameter_id, f, true)?;

    Ok(true)
}

/// Collect separator-comment sources for parameters in source order.
fn collect_parameter_separator_line_comment_sources(
    context: &DestackFormatContext<'_>,
    parameters: &[LocalNodeId<Parameter>],
) -> Vec<Option<SeparatorLineCommentSource>> {
    parameters
        .iter()
        .copied()
        .map(|parameter_id| parameter_separator_line_comment_source(context, parameter_id))
        .collect::<Vec<_>>()
}

/// Return whether one blank line should be preserved between adjacent parameters.
fn preserve_blank_line_before_parameter_with_separator_comments(
    context: &DestackFormatContext<'_>,
    parameters: &[LocalNodeId<Parameter>],
    separator_line_comment_sources: &[Option<SeparatorLineCommentSource>],
    parameter_index: usize,
) -> bool {
    let left_parameter_id = parameters[parameter_index - 1];
    let right_parameter_id = parameters[parameter_index];

    // when separator comments are rendered after the previous comma,
    // preserve blank lines between that cluster and the next parameter
    if let Some(previous_separator_source) =
        separator_line_comment_sources[parameter_index - 1].as_ref()
        && previous_separator_source.is_own_line
        && let Some(last_comment_id) = previous_separator_source.comment_ids.last().copied()
    {
        let last_comment_span = context.span(last_comment_id);
        let right_parameter_span = context.span(right_parameter_id);
        if let Some(between_span) = last_comment_span.gap_to(right_parameter_span) {
            return context.has_blank_line(between_span);
        }
    }

    let left_parameter_span = context.span(left_parameter_id);
    let right_parameter_span = context.span(right_parameter_id);
    let Some(between_span) = left_parameter_span.gap_to(right_parameter_span) else {
        return false;
    };

    context.has_blank_line(between_span)
}

/// Write one separator-comment aware multiline parameter list body.
fn write_signature_separator_comment_multiline_parameter_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameters: &[LocalNodeId<Parameter>],
    separator_line_comment_sources: &[Option<SeparatorLineCommentSource>],
    should_emit_trailing_separator: bool,
) -> FormatResult<()> {
    for (index, parameter_id) in parameters.iter().enumerate() {
        if index > 0 {
            if preserve_blank_line_before_parameter_with_separator_comments(
                f.context(),
                parameters,
                separator_line_comment_sources,
                index,
            ) {
                write!(f, [empty_line()])?;
            } else {
                write!(f, [hard_line_break()])?;
            }
        }

        if let Some(comment_source) = separator_line_comment_sources[index].as_ref()
            && !parameter_is_variadic(f.context(), *parameter_id)
            && write_parameter_without_separator_line_comment(f, *parameter_id)?
        {
            write_separator_line_comment_after_comma(f, comment_source)?;
            continue;
        }

        write!(f, [group(parameter_id)])?;
        if index + 1 < parameters.len() || should_emit_trailing_separator {
            write!(f, [token(",")])?;
        }
    }

    Ok(())
}

/// Return whether a single parameter should keep compact outer parentheses.
pub(crate) fn single_parameter_should_hug(
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
                ) && context.node_has_newline(default_id)
            }),
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => false,
    };
    if has_multiline_collection_default {
        return false;
    }

    let has_newline = context.node_has_newline(parameter_id);
    match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } => !(has_newline && default.is_some()),
        Parameter::VariadicNamed { .. } => !has_newline,
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => true,
    }
}

/// Write the function abstraction prefix.
pub(crate) fn write_function_abstraction_prefix(
    f: &mut DestackFormatter<'_, '_>,
    abstraction: FunctionAbstraction,
) -> FormatResult<()> {
    match abstraction {
        FunctionAbstraction::Abstract => {
            write!(f, [Keyword::Abstract, space()])?;
        }
        FunctionAbstraction::AbstractOverride => {
            write!(f, [Keyword::Abstract, space(), Keyword::Override, space()])?;
        }
        FunctionAbstraction::ConcreteOverride => {
            write!(f, [Keyword::Override, space()])?;
        }
        FunctionAbstraction::Concrete => {}
    }

    Ok(())
}

/// Write the async keyword prefix.
pub(crate) fn write_function_asynchrony_prefix(
    f: &mut DestackFormatter<'_, '_>,
    asynchrony: Asynchrony,
) -> FormatResult<()> {
    if asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Async, space()])?;
    }

    Ok(())
}

/// Represent shared function header styles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FunctionHeaderStyle {
    /// The declaration-style function header.
    Declaration,
    /// The method-like function header.
    MethodLike,
}

/// Write shared function header keywords and generator markers.
pub(crate) fn write_function_header_prefix(
    f: &mut DestackFormatter<'_, '_>,
    signature: &FunctionSignature,
    style: FunctionHeaderStyle,
    has_name_or_key: bool,
) -> FormatResult<()> {
    // abstraction
    write_function_abstraction_prefix(f, signature.abstraction)?;

    // asynchrony
    write_function_asynchrony_prefix(f, signature.asynchrony)?;

    // mode
    if let Some(mode) = signature.mode {
        if let Some(keyword) = mode.to_keyword() {
            write!(f, [keyword])?;
        }
        if has_name_or_key || mode == FunctionMode::New {
            write!(f, [space()])?;
        }
    }

    // keyword and cardinality
    if style == FunctionHeaderStyle::Declaration
        && signature.kind == FunctionKind::Function
        && signature.mode != Some(FunctionMode::Constructor)
        && signature.mode != Some(FunctionMode::New)
    {
        if signature.cardinality == FunctionCardinality::Generator {
            write!(f, [Keyword::Function, token("*"), space()])?;
        } else {
            write!(f, [Keyword::Function, space()])?;
        }
    } else if signature.cardinality == FunctionCardinality::Generator {
        if style == FunctionHeaderStyle::Declaration {
            write!(f, [token("*"), space()])?;
        } else {
            write!(f, [token("*")])?;
        }
    }

    Ok(())
}

/// Return whether an object parameter pattern should expand for readability.
fn parameter_object_pattern_should_expand(
    context: &DestackFormatContext<'_>,
    pattern_id: LocalNodeId<Pattern>,
) -> bool {
    let fields = match context.tree.get(pattern_id) {
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => fields,
        _ => return false,
    };

    let has_nested_pattern = fields
        .iter()
        .any(|field_id| match context.tree.get(*field_id) {
            PatternField::Named {
                pattern: Some(_), ..
            }
            | PatternField::Computed {
                pattern: Some(_), ..
            }
            | PatternField::Positional { .. } => true,
            PatternField::Spread {
                pattern: Some(_), ..
            } => true,
            PatternField::Named { pattern: None, .. }
            | PatternField::Computed { pattern: None, .. }
            | PatternField::Alias { .. }
            | PatternField::Spread { pattern: None, .. }
            | PatternField::Elision => false,
        });
    if has_nested_pattern {
        return true;
    }

    if fields.len() >= OBJECT_PATTERN_FORCE_EXPAND_MIN_FIELDS {
        return true;
    }

    if fields.len() <= OBJECT_PATTERN_INLINE_MAX_FIELDS {
        return false;
    }

    fields
        .iter()
        .any(|field_id| match context.tree.get(*field_id) {
            PatternField::Named { default, .. }
            | PatternField::Computed { default, .. }
            | PatternField::Alias { default, .. } => default.is_some(),
            PatternField::Positional { .. }
            | PatternField::Spread { .. }
            | PatternField::Elision => false,
        })
}

/// Return whether this parameter should force multiline signature formatting.
pub(crate) fn parameter_should_force_expand_in_signature(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    if context.node_has_newline(parameter_id) {
        match context.tree.get(parameter_id) {
            Parameter::Named { default, .. } => {
                if default.is_some() {
                    return true;
                }
            }
            Parameter::VariadicNamed { .. }
            | Parameter::Pattern { .. }
            | Parameter::VariadicPattern { .. } => {
                return true;
            }
        }
    }

    let pattern_id = match context.tree.get(parameter_id) {
        Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => {
            Some(*pattern)
        }
        Parameter::Named { .. } | Parameter::VariadicNamed { .. } => None,
    };
    let Some(pattern_id) = pattern_id else {
        return false;
    };

    if context.node_has_newline(pattern_id) {
        return true;
    }

    parameter_object_pattern_should_expand(context, pattern_id)
}

/// Return whether a signature return type carries a boundary line-postfix annotation.
pub(crate) fn signature_return_type_has_line_postfix_boundary_annotation(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(return_type) = return_type else {
        return false;
    };
    let Some(annotations) = context.annotations(return_type) else {
        return false;
    };

    annotations.iter().any(|annotation_id| {
        matches!(
            context.annotation(*annotation_id),
            Annotation::Comment {
                position: AnnotationPosition::LinePostfixBoundary,
                ..
            }
        )
    })
}

/// Return whether spacing before a function body should be emitted by annotations.
pub(crate) fn signature_should_elide_space_before_body(
    _context: &DestackFormatContext<'_>,
    _return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    false
}

/// Return whether dynamic parameters should force multiline signature formatting.
pub(crate) fn signature_parameters_should_expand(
    context: &DestackFormatContext<'_>,
    _mode: Option<FunctionMode>,
    parameters: &[LocalNodeId<Parameter>],
    _return_type: Option<LocalNodeId<Expression>>,
    include_parameter_shape_expansion: bool,
) -> bool {
    let should_expand_parameter_shapes = include_parameter_shape_expansion
        && parameters.len() > 1
        && parameters
            .iter()
            .copied()
            .any(|parameter_id| parameter_should_force_expand_in_signature(context, parameter_id));
    let should_break_constructor_parameters =
        parameters_with_modifiers_should_expand(context, parameters);
    let should_expand_for_parameter_line_comments = parameters
        .iter()
        .copied()
        .any(|parameter_id| parameter_has_line_comment_annotation(context, parameter_id));

    should_expand_parameter_shapes
        || should_break_constructor_parameters
        || should_expand_for_parameter_line_comments
}

/// Write a dynamic parameter list with shared expansion controls.
pub(crate) fn write_signature_dynamic_parameter_list(
    f: &mut DestackFormatter<'_, '_>,
    parameters: &[LocalNodeId<Parameter>],
    should_expand: bool,
    disallow_trailing_separator: bool,
) -> FormatResult<()> {
    let has_variadic_tail = parameters
        .last()
        .is_some_and(|parameter_id| parameter_is_variadic(f.context(), *parameter_id));
    let should_emit_trailing_separator = !disallow_trailing_separator
        && f.context().options.trailing_comma == TrailingComma::All
        && !has_variadic_tail;
    let separator_line_comment_sources =
        collect_parameter_separator_line_comment_sources(f.context(), parameters);
    let use_separator_comment_multiline = should_expand
        && separator_line_comment_sources
            .iter()
            .zip(parameters.iter())
            .any(|(source, parameter_id)| {
                source.is_some()
                    && parameter_can_render_without_separator_line_comment(
                        f.context(),
                        *parameter_id,
                    )
            });
    if use_separator_comment_multiline {
        write!(f, [token("("), hard_line_break()])?;
        write!(
            f,
            [block_indent(&format_with(
                |f: &mut DestackFormatter<'_, '_>| {
                    write_signature_separator_comment_multiline_parameter_list(
                        f,
                        parameters,
                        &separator_line_comment_sources,
                        should_emit_trailing_separator,
                    )
                }
            ))]
        )?;
        write!(f, [hard_line_break(), token(")")])?;
        return Ok(());
    }

    let mut parameters_list = list_like("(", ")", ",", parameters);
    parameters_list.should_expand(should_expand);
    if disallow_trailing_separator {
        parameters_list.disallow_trailing_separator();
    }

    write!(f, [parameters_list])?;
    Ok(())
}

/// Write one empty parameter list and keep delimiter-interior comments inside `()`.
pub(crate) fn write_empty_parameter_list_with_interior_annotations<'ast, T: Node + Clone>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
) -> FormatResult<()>
where
    NodeTree: NodeTreeImpl<T>,
{
    if !f.context().has_delimited_interior_annotation(node_id) {
        write!(f, [token("()")])?;
        return Ok(());
    }

    write!(
        f,
        [
            token("("),
            soft_block_indent(&f.context().delimited_interior_annotations(node_id)),
            token(")")
        ]
    )?;
    Ok(())
}

/// Format a where clause list.
pub(crate) fn format_where_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    // keyword
    write!(f, [Keyword::Where])?;
    if clauses.is_empty() {
        return Ok(());
    }
    write!(f, [space()])?;

    // clauses
    if clauses.len() == 1 {
        write!(f, [&clauses[0]])?;
    } else {
        let clauses_vec = clauses.to_vec();
        write!(f, [list_like("(", ")", ",", &clauses_vec)])?;
    }

    Ok(())
}

/// Format a where clause list in a soft break group.
pub(crate) fn format_where_clause_with_break<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    clauses: &[LocalNodeId<WhereClause>],
) -> FormatResult<()> {
    write!(
        f,
        [group(&destack_fir::format_args![
            soft_line_break_or_space(),
            format_with(|f| format_where_clause(f, clauses)),
        ])]
    )?;

    Ok(())
}

impl<'ast> FormatNode<'ast, WhereClause> for WhereClause {
    fn format_node(
        &self,
        node_id: LocalNodeId<WhereClause>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        write!(f, [self.left, token(":"), space(), self.right])?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Annotation, DestackFormatArtifacts, DestackFormatContext, DestackFormatOptions,
        TestFormatter, assert_format, assert_format_program_idempotent_with_file_type,
    };
    use destack_ast::{LocalNodeId, NodeParentIndex, NodeType, Parameter};
    use destack_source::FileType;

    /// Build a formatter context for signature separator-comment assertions.
    fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
        DestackFormatContext::new(
            DestackFormatOptions::default(),
            DestackFormatArtifacts {
                file: &formatter.file,
                tree: &formatter.tree,
                tokens: &formatter.tokens,
                side_tokens: &formatter.side_tokens,
                side_span: &formatter.side_span,
                strings: &formatter.strings,
                parents: NodeParentIndex::from_tree(&formatter.tree),
            },
        )
    }

    /// Find one annotation id by marker fragment.
    fn find_annotation_by_fragment(
        context: &DestackFormatContext<'_>,
        marker: &str,
    ) -> Option<LocalNodeId<Annotation>> {
        for (entry_index, _) in context.formatter_annotation_entries.iter().enumerate() {
            let annotation_id = LocalNodeId::<Annotation>::new(entry_index as u32);
            let matches_marker = match context.annotation(annotation_id) {
                Annotation::Comment { node, .. } => context.comment_text(node).contains(marker),
                Annotation::Doc { node, .. } => {
                    let document = context.tree.get(node);
                    context.strings.get(document.string).contains(marker)
                }
                Annotation::Blank { .. } | Annotation::Decorator { .. } => false,
            };
            if matches_marker {
                return Some(annotation_id);
            }
        }

        None
    }

    /// Find one annotation owner parameter id.
    fn find_annotation_owner_parameter_id(
        context: &DestackFormatContext<'_>,
        annotation_id: LocalNodeId<Annotation>,
    ) -> Option<LocalNodeId<Parameter>> {
        context
            .formatter_annotation_ids_by_node_id
            .iter()
            .enumerate()
            .find_map(|(node_id, annotation_ids)| {
                if !annotation_ids
                    .iter()
                    .any(|candidate| candidate.id == annotation_id.id)
                {
                    return None;
                }

                (context.tree.get_node_type(node_id as u32) == NodeType::Parameter)
                    .then_some(LocalNodeId::<Parameter>::new(node_id as u32))
            })
    }

    #[test]
    fn test_format_parameter() {
        assert_format!(
            "x: int32",
            "x: int32",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_parameter_with_default() {
        assert_format!(
            "x: int32 = 1",
            "x: int32 = 1",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_parameter_comment_between_name_and_type() {
        assert_format!(
            "x /* a */ : number",
            "x /* a */ : number",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_optional_parameter_comment_between_name_and_type() {
        assert_format!(
            "x? /* a */ : number",
            "x? /* a */ : number",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_parameter_comment_before_name() {
        assert_format!(
            "/* a */ x: number",
            "/* a */ x: number",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    /// Trailing separator line comments in parameter lists should stay idempotent.
    #[test]
    fn test_format_signature_trailing_separator_line_comment_is_idempotent() {
        let source = "f2 = (
  currentRequest: {a: number},
  // TODO this is a very very very very long comment that makes it go > 80 columns
): number => {};
";
        assert_format_program_idempotent_with_file_type(
            source,
            FileType::TypeScript,
            DestackFormatOptions::default(),
        );
    }

    /// Own-line block separator comments in parameter lists should stay idempotent.
    #[test]
    fn test_format_signature_trailing_separator_block_comment_is_idempotent() {
        let source = r#"var x = {
  getSectionMode(
    pageMetaData: PageMetaData,
    sectionMetaData: SectionMetaData
    /* $FlowFixMe This error was exposed while converting keyMirror
     * to keyMirrorRecursive */
  ): $Enum<SectionMode> {
  }
}

class X2 {
  getSectionMode(
    pageMetaData: PageMetaData,
    sectionMetaData: SectionMetaData = ['unknown']
    /* $FlowFixMe This error was exposed while converting keyMirror
     * to keyMirrorRecursive */
  ): $Enum<SectionMode> {
  }
}
"#;
        assert_format_program_idempotent_with_file_type(
            source,
            FileType::TypeScript,
            DestackFormatOptions::default(),
        );
    }

    /// Signature separator block comments should resolve to one detachable own-line source.
    #[test]
    fn test_signature_separator_block_comment_source_is_detected_for_flow_style_fixture() {
        let source = r#"class X2 {
  getSectionMode(
    pageMetaData: PageMetaData,
    sectionMetaData: SectionMetaData = ["unknown"]
    /* $FlowFixMe This error was exposed while converting keyMirror
     * to keyMirrorRecursive */
    ,
  ): $Enum<SectionMode> {
  }
}
"#;
        let (formatter, _) =
            TestFormatter::parse_with_file_type(source, FileType::TypeScript, |p| Ok(p.parse()))
                .expect("parse signature block comment source");
        let context = context_from_formatter(&formatter);
        let annotation_id = find_annotation_by_fragment(&context, "$FlowFixMe")
            .expect("expected flow-fixme separator block annotation");

        let source_info =
            super::parameter_separator_line_comment_annotation_info(&context, annotation_id);
        assert!(source_info.is_some(), "expected separator annotation info");

        let parameter_id = find_annotation_owner_parameter_id(&context, annotation_id)
            .expect("expected parameter owner for separator annotation");
        let source = super::parameter_separator_line_comment_source(&context, parameter_id);
        assert!(source.is_some(), "expected separator source for parameter");
    }
}
