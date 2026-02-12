use crate::argument::list_like;
use crate::block::format_block_of_statements;
use crate::scan::next_non_whitespace_after_annotation;
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{
    Annotation, AnnotationPosition, Asynchrony, Comment, CommentStyle, Expression,
    FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode, FunctionSignature,
    Keyword, LocalNodeId, Parameter, Pattern, PatternField,
};
use destack_fir::format::{FormatResult, text};
use destack_fir::prelude::*;
use destack_fir::write;

// signature expansion thresholds
const CONSTRUCTOR_PARAMETER_EXPAND_MIN_COUNT: usize = 2;
const OBJECT_PATTERN_FORCE_EXPAND_MIN_FIELDS: usize = 3;
const OBJECT_PATTERN_INLINE_MAX_FIELDS: usize = 1;

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
pub(crate) fn parameter_has_modifier(
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
pub(crate) fn constructor_parameters_should_expand(
    context: &DestackFormatContext<'_>,
    mode: Option<FunctionMode>,
    parameters: &[LocalNodeId<Parameter>],
) -> bool {
    matches!(mode, Some(FunctionMode::Constructor | FunctionMode::New))
        && parameters.len() >= CONSTRUCTOR_PARAMETER_EXPAND_MIN_COUNT
        && parameters
            .iter()
            .any(|parameter_id| parameter_has_modifier(context, *parameter_id))
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
        return true;
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

/// Return whether a signature return type is already multiline in source.
pub(crate) fn signature_return_type_is_multiline(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    return_type.is_some_and(|return_type| context.node_has_newline(return_type))
}

/// Return whether spacing before a function body should be emitted by annotations.
pub(crate) fn signature_should_elide_space_before_body(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    return_type.is_some_and(|return_type| context.has_postfix_annotation(return_type))
}

/// Return whether dynamic parameters should force multiline signature formatting.
pub(crate) fn signature_parameters_should_expand(
    context: &DestackFormatContext<'_>,
    mode: Option<FunctionMode>,
    parameters: &[LocalNodeId<Parameter>],
    return_type: Option<LocalNodeId<Expression>>,
    include_parameter_shape_expansion: bool,
) -> bool {
    let should_expand_parameter_shapes = include_parameter_shape_expansion
        && parameters
            .iter()
            .copied()
            .any(|parameter_id| parameter_should_force_expand_in_signature(context, parameter_id));
    let should_break_constructor_parameters =
        constructor_parameters_should_expand(context, mode, parameters);
    let should_expand_single_for_multiline_return_type = parameters.len() == 1
        && !parameter_is_variadic(context, parameters[0])
        && signature_return_type_is_multiline(context, return_type);

    should_expand_parameter_shapes
        || should_break_constructor_parameters
        || should_expand_single_for_multiline_return_type
}

/// Write a dynamic parameter list with shared expansion controls.
pub(crate) fn write_signature_dynamic_parameter_list(
    f: &mut DestackFormatter<'_, '_>,
    parameters: &[LocalNodeId<Parameter>],
    should_expand: bool,
    disallow_trailing_separator: bool,
) -> FormatResult<()> {
    let mut parameters_list = list_like("(", ")", ",", parameters);
    parameters_list.should_expand(should_expand);
    if disallow_trailing_separator {
        parameters_list.disallow_trailing_separator();
    }

    write!(f, [parameters_list])?;
    Ok(())
}

/// Collect deferred function boundary line comments from return type and body.
pub(crate) fn collect_deferred_function_boundary_line_comments(
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

/// Return whether a function body has deferred boundary line comments.
pub(crate) fn function_body_has_deferred_boundary_line_comments(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
    body: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(body) = body else {
        return false;
    };
    matches!(context.tree.get(body), Expression::Block(_))
        && !collect_deferred_function_boundary_line_comments(context, return_type, body).is_empty()
}

/// Format a function body block with deferred boundary line comments inside braces.
pub(crate) fn format_function_body_block_with_deferred_boundary_line_comments(
    f: &mut DestackFormatter<'_, '_>,
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
            soft_block_indent(&format_with(|f| {
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
