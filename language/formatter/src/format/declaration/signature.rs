use crate::analysis::scan::previous_non_whitespace_token_before_annotation;
use crate::collection::list_like;
use crate::collection::property::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix_maybe,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AnnotationPosition, Asynchrony, Comment, CommentStyle, Declaration, Doc, DocStyle, Expression,
    FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode, FunctionSignature,
    Keyword, LocalNodeId, Member, NodeType, Parameter, Pattern, PatternField, Property, TokenType,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

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

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        node_id: LocalNodeId<Parameter>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let is_variadic_parameter = matches!(
            self,
            Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. }
        );
        let defer_prefix_annotations_after_spread = is_variadic_parameter
            && parameter_prefix_annotations_follow_spread(f.context(), node_id);
        if !defer_prefix_annotations_after_spread {
            write!(f, [f.context().any_prefix_annotations(node_id)])?;
        }

        let is_typescript = f.context().options.language_type.is_typescript();
        let is_static_parameter = is_typescript && parameter_is_static(f.context(), node_id);
        let wrote_type_infix = match self {
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
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        } else {
            write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        }

        Ok(())
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

/// Return whether this parameter has any prefix annotation that should force multiline layout.
fn parameter_has_prefix_annotation(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    if !context.has_annotation(parameter_id) {
        return false;
    }

    let is_variadic_parameter = parameter_is_variadic(context, parameter_id);
    context
        .visit_annotations(parameter_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation = context.annotation(*annotation_id);
                let is_prefix = matches!(
                    annotation,
                    Annotation::Decorator { .. }
                        | Annotation::Comment {
                            position: AnnotationPosition::BlockPrefix
                                | AnnotationPosition::LinePrefix,
                            ..
                        }
                        | Annotation::Doc {
                            position: AnnotationPosition::BlockPrefix
                                | AnnotationPosition::LinePrefix,
                            ..
                        }
                );
                if !is_prefix {
                    return false;
                }

                // keep single-line decorators inline with parameters when they fit
                if matches!(annotation, Annotation::Decorator { .. }) {
                    let annotation_span = context.annotation_span(*annotation_id);
                    if !context.has_newline(annotation_span) {
                        return false;
                    }
                }

                // keep single variadic parameters compact for inline star-style rest seam comments
                if is_variadic_parameter {
                    let annotation_span = context.annotation_span(*annotation_id);
                    let is_single_line = !context.has_newline(annotation_span);
                    if is_single_line {
                        let is_star_style = match annotation {
                            Annotation::Comment { node, .. } => {
                                let comment = context.tree.get::<Comment>(node);
                                comment.style == CommentStyle::Star
                            }
                            Annotation::Doc { node, .. } => {
                                let doc = context.tree.get::<Doc>(node);
                                doc.style == DocStyle::Star
                            }
                            Annotation::Decorator { .. } | Annotation::Blank { .. } => false,
                        };
                        if is_star_style {
                            return false;
                        }
                    }
                }

                true
            })
        })
        .unwrap_or(false)
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
    let should_expand_for_parameter_line_comments = parameters
        .iter()
        .copied()
        .any(|parameter_id| parameter_has_line_comment_annotation(context, parameter_id));
    let should_expand_for_parameter_prefix_annotations = parameters
        .iter()
        .copied()
        .any(|parameter_id| parameter_has_prefix_annotation(context, parameter_id));
    let should_expand_single_for_multiline_return_type = parameters.len() == 1
        && !parameter_is_variadic(context, parameters[0])
        && signature_return_type_is_multiline(context, return_type);

    should_expand_parameter_shapes
        || should_break_constructor_parameters
        || should_expand_for_parameter_line_comments
        || should_expand_for_parameter_prefix_annotations
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

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

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
            "/* a */ x: number",
            |p| p.eat_parameter(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_optional_parameter_comment_between_name_and_type() {
        assert_format!(
            "x? /* a */ : number",
            "/* a */ x?: number",
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
}
