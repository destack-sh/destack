use crate::format::analysis::previous_non_whitespace_token_before_annotation;
use crate::format::annotation::annotation_render_items_matching;
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
        Parameter::Error => false,
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
                | Member::ComptimeBlock { .. }
                | Member::Error => false,
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
        Parameter::Error => None,
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
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } | Parameter::Error => {
            false
        }
    };
    if has_multiline_collection_default {
        return false;
    }

    let has_newline = context.node_has_newline(parameter_id);
    match context.tree.get(parameter_id) {
        Parameter::Named { default, .. } => !(has_newline && default.is_some()),
        Parameter::VariadicNamed { .. } => !has_newline,
        Parameter::Pattern { .. } | Parameter::VariadicPattern { .. } => true,
        Parameter::Error => false,
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

/// Write shared function header keywords and generator markers.
pub(crate) fn write_function_header_prefix(
    f: &mut DestackFormatter<'_, '_>,
    signature: &FunctionSignature,
    is_declaration_style: bool,
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
    if is_declaration_style
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
        if is_declaration_style {
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
            | Parameter::VariadicPattern { .. }
            | Parameter::Error => {
                return true;
            }
        }
    }

    let pattern_id = match context.tree.get(parameter_id) {
        Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => {
            Some(*pattern)
        }
        Parameter::Named { .. } | Parameter::VariadicNamed { .. } | Parameter::Error => None,
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

/// Return whether one block body should be preceded by a space.
pub(crate) fn expression_body_requires_head_space(
    context: &DestackFormatContext<'_>,
    body_expression: LocalNodeId<Expression>,
) -> bool {
    fn annotations_require_head_spacing(
        context: &DestackFormatContext<'_>,
        annotations: Option<Vec<LocalNodeId<Annotation>>>,
    ) -> Option<bool> {
        let annotations = annotations?;

        let has_block_prefix_annotation = annotations.iter().copied().any(|annotation_id| {
            matches!(
                context.annotation(annotation_id),
                Annotation::Blank {
                    position: AnnotationPosition::BlockPrefix,
                    ..
                } | Annotation::Doc {
                    position: AnnotationPosition::BlockPrefix,
                    ..
                } | Annotation::Comment {
                    position: AnnotationPosition::BlockPrefix,
                    ..
                } | Annotation::Decorator {
                    position: AnnotationPosition::BlockPrefix,
                    ..
                }
            )
        });
        if has_block_prefix_annotation {
            return Some(false);
        }

        let has_line_prefix_annotation = annotations.iter().copied().any(|annotation_id| {
            matches!(
                context.annotation(annotation_id),
                Annotation::Blank {
                    position: AnnotationPosition::LinePrefix,
                    ..
                } | Annotation::Doc {
                    position: AnnotationPosition::LinePrefix,
                    ..
                } | Annotation::Comment {
                    position: AnnotationPosition::LinePrefix,
                    ..
                } | Annotation::Decorator {
                    position: AnnotationPosition::LinePrefix,
                    ..
                }
            )
        });
        if has_line_prefix_annotation {
            return Some(true);
        }

        None
    }

    let Expression::Block(block_id) = context.tree.get(body_expression) else {
        return true;
    };
    if let Some(requires_space) =
        annotations_require_head_spacing(context, context.annotations(*block_id))
    {
        return requires_space;
    }

    let block = context.tree.get(*block_id);
    if let Some(first_expression) = block.expressions.first().copied()
        && let Some(requires_space) =
            annotations_require_head_spacing(context, context.annotations(first_expression))
    {
        return requires_space;
    }

    true
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

    let mut parameters_list = list_like("(", ")", ",", parameters);
    parameters_list.should_expand(should_expand);
    if should_emit_trailing_separator {
        parameters_list.force_trailing_separator();
    }
    if disallow_trailing_separator {
        parameters_list.disallow_trailing_separator();
    }

    write!(f, [parameters_list])?;
    Ok(())
}

/// Write one empty parameter list and keep delimiter-interior comments inside `()`.
pub(crate) fn write_empty_parameter_list_with_interior_annotations<'ast, T: Node + Clone + 'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
) -> FormatResult<()>
where
    NodeTree: NodeTreeImpl<T>,
{
    let interior_items = annotation_render_items_matching(f.context(), node_id, |position| {
        position == AnnotationPosition::BlockInfix
    });
    if interior_items.is_empty() {
        write!(f, [token("()")])?;
        return Ok(());
    }

    // keep one inline block comment compact: `(/* comment */)`
    if interior_items.len() == 1
        && matches!(
            f.context().annotation(interior_items[0]),
            Annotation::Comment {
                position: AnnotationPosition::BlockInfix,
                ..
            }
        )
        && let Annotation::Comment { node, .. } = f.context().annotation(interior_items[0])
    {
        let comment = f.context().tree.get::<Comment>(node);
        if comment.style == CommentStyle::Star
            && !f.context().annotation_starts_on_own_line(interior_items[0])
        {
            write!(
                f,
                [
                    token("("),
                    f.context().block_infix_annotations(node_id),
                    token(")")
                ]
            )?;
            return Ok(());
        }
    }

    write!(
        f,
        [
            token("("),
            soft_block_indent(&f.context().block_infix_annotations(node_id)),
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
