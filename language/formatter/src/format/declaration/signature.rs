use crate::format::annotation::{
    block_infix_annotations, format_raw_comment, infix_or_postfix_annotations,
    infix_or_postfix_annotations_without_line_suffix_boundary, postfix_annotations,
    postfix_annotations_without_line_suffix_boundary, prefix_annotations,
};
use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::format::operator::{
    write_type_expression_with_inline_prefix_annotations,
    write_type_expression_with_inline_prefix_annotations_from,
};
use crate::{Annotation, DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    AbstractionModifier, AccessorKind, AnnotationPosition, Asynchrony, BindingAnchor, BindingKind,
    BindingModifier, BindingOperator, Comment, Declaration, DeclarationKind, Expression,
    FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode, FunctionSignature,
    Keyword, LocalNodeId, Member, Mutability, Node, NodeTree, NodeTreeImpl, NodeType, Parameter,
    Pattern, Property, StringId, Timing, TokenType, VarianceModifier, WhereClause,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::NodeSpanType;
use destack_workspace::TrailingComma;

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

/// Write one type-parameter-like `extends` and `=` trailer sequence.
pub(crate) fn write_type_parameter_constraint_and_default<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    constraint: Option<LocalNodeId<Expression>>,
    default: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    if let Some(constraint) = constraint {
        let group_id = f.group_id("type_parameter_constraint");

        write!(
            f,
            [
                space(),
                Keyword::Extends,
                group(&indent(&format_with(|f| {
                    write!(f, [line_suffix_boundary(), soft_line_break_or_space()])
                })))
                .with_id(Some(group_id)),
                indent_if_group_breaks(&constraint, group_id)
            ]
        )?;
    }

    if let Some(default) = default {
        let group_id = f.group_id("type_parameter_default");

        write!(
            f,
            [
                space(),
                token("="),
                group(&indent(&soft_line_break_or_space())).with_id(Some(group_id)),
                line_suffix_boundary(),
                indent_if_group_breaks(&default, group_id)
            ]
        )?;
    }

    Ok(())
}

/// Write one grouped type-parameter declaration list with local `<...>` flow.
pub(crate) fn write_static_parameter_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    static_parameters: &[LocalNodeId<Parameter>],
    trailing_separator: TrailingSeparator,
) -> FormatResult<()> {
    let group_id = f.group_id("type_parameters");
    let format_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let separator = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [token(","), soft_line_break_or_space()])
        });

        f.join_with(separator)
            .entries(static_parameters.iter().copied())
            .finish()?;

        match trailing_separator {
            TrailingSeparator::Allowed => write!(f, [if_group_breaks(&token(","))])?,
            TrailingSeparator::Mandatory => write!(f, [token(",")])?,
            TrailingSeparator::Omit => {}
        }

        Ok(())
    });

    write!(
        f,
        [group(&format_with(|f| {
            write!(
                f,
                [
                    token("<"),
                    soft_block_indent(&format_parameters),
                    token(">")
                ]
            )
        }))
        .with_id(Some(group_id))]
    )
}

/// Return the default trailing separator for one type-parameter list.
pub(crate) fn default_static_parameter_trailing_separator(
    f: &DestackFormatter<'_, '_>,
) -> TrailingSeparator {
    match f.context().options.trailing_comma {
        TrailingComma::None => TrailingSeparator::Omit,
        TrailingComma::Es5 | TrailingComma::All => TrailingSeparator::Allowed,
    }
}

/// Write one parameter type with local infix spacing.
fn write_parameter_type_with_infix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    is_static_parameter: bool,
    ty: Option<LocalNodeId<Expression>>,
) -> FormatResult<bool> {
    let Some(ty) = ty else {
        return Ok(false);
    };

    write!(f, [block_infix_annotations(f.context(), node_id)])?;
    if is_static_parameter {
        write!(f, [space(), Keyword::Extends, space(), ty])?;
    } else {
        let has_infix_annotations = f.context().has_infix_annotation(node_id);
        if has_infix_annotations {
            write!(f, [space(), token(":"), space()])?;
        } else {
            write!(f, [token(":"), space()])?;
        }

        if let Some(start) = parameter_type_comment_start(f.context(), node_id, ty) {
            write_type_expression_with_inline_prefix_annotations_from(f, ty, start)?;
        } else {
            write_type_expression_with_inline_prefix_annotations(f, ty)?;
        }
    }

    Ok(true)
}

/// Return the earliest offset that may own comments before one parameter type.
fn parameter_type_comment_start(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Parameter>,
    ty: LocalNodeId<Expression>,
) -> Option<u32> {
    let type_span = context.tree.get_side_span(node_id, NodeSpanType::Type)?;
    let expression_start = context.type_expression_token_start(ty);

    let gap_start = match context.tree.get(node_id) {
        Parameter::Named { .. } | Parameter::VariadicNamed { .. } => {
            context.tree.get_main_span(node_id)?.end
        }
        Parameter::Pattern { pattern, .. } | Parameter::VariadicPattern { pattern, .. } => {
            context.tree.get_span(*pattern).end
        }
        Parameter::Error => return None,
    };

    if gap_start >= expression_start || type_span.start > expression_start {
        return None;
    }

    Some(gap_start)
}

/// Write one static parameter trailer sequence after its name.
fn write_static_parameter_trailers<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    constraint: Option<LocalNodeId<Expression>>,
    default: Option<LocalNodeId<Expression>>,
) -> FormatResult<bool> {
    if constraint.is_none() && default.is_none() {
        return Ok(false);
    }

    write!(f, [block_infix_annotations(f.context(), node_id)])?;
    write_type_parameter_constraint_and_default(f, constraint, default)?;

    Ok(true)
}

/// Write one parameter type and default trailer sequence.
fn write_parameter_type_and_default<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    is_static_parameter: bool,
    ty: Option<LocalNodeId<Expression>>,
    default: Option<LocalNodeId<Expression>>,
) -> FormatResult<bool> {
    if is_static_parameter {
        return write_static_parameter_trailers(f, node_id, ty, default);
    }

    let wrote_type_infix = write_parameter_type_with_infix(f, node_id, false, ty)?;

    if let Some(default) = default {
        write!(f, [space(), token("="), space(), default])?;
    }

    Ok(wrote_type_infix)
}

/// Write one parameter's trailing annotations after its core syntax.
fn write_parameter_trailing_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    wrote_type_infix: bool,
    suppress_separator_boundary_annotations: bool,
) -> FormatResult<()> {
    if wrote_type_infix {
        if suppress_separator_boundary_annotations {
            return write!(
                f,
                [postfix_annotations_without_line_suffix_boundary(
                    f.context(),
                    node_id
                )]
            );
        }

        return write!(f, [postfix_annotations(f.context(), node_id)]);
    }

    if suppress_separator_boundary_annotations {
        return write!(
            f,
            [infix_or_postfix_annotations_without_line_suffix_boundary(
                f.context(),
                node_id
            )]
        );
    }

    write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
}

/// Return whether all prefix annotations on one variadic parameter follow `...`.
fn parameter_prefix_annotations_follow_spread(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let annotations = context.annotation_ids(parameter_id);
    if annotations.is_empty() {
        return false;
    }

    let mut has_prefix_annotation = false;
    for annotation_id in annotations.iter().copied() {
        let position = context.annotation(annotation_id).position();
        if !matches!(
            position,
            AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix
        ) {
            continue;
        }

        has_prefix_annotation = true;
        let previous_token = context.annotation_previous_non_whitespace_token(annotation_id);
        if !previous_token.is_some_and(|token| token.token.ty == TokenType::Spread) {
            return false;
        }
    }

    has_prefix_annotation
}

/// Format one parameter node with optional separator-boundary annotation suppression.
fn write_named_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    modifiers: Option<BindingModifier>,
    name: StringId,
    ty: Option<LocalNodeId<Expression>>,
    default: Option<LocalNodeId<Expression>>,
    is_static_parameter: bool,
) -> FormatResult<bool> {
    // defaulted runtime parameters
    if !is_static_parameter && let Some(default) = default {
        let format_left = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            // modifiers
            format_binding_modifiers_prefix_maybe(f, modifiers)?;

            // name
            write!(f, [name])?;

            // modifiers
            format_binding_modifiers_postfix_maybe(f, modifiers)?;

            // type
            write_parameter_type_with_infix(f, node_id, false, ty)?;

            Ok(())
        });

        write!(
            f,
            [group(&format_left), space(), token("="), space(), default]
        )?;
        return Ok(ty.is_some());
    }

    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;

    // name
    write!(f, [name])?;

    // modifiers
    format_binding_modifiers_postfix_maybe(f, modifiers)?;

    // type and default
    write_parameter_type_and_default(f, node_id, is_static_parameter, ty, default)
}

/// Format one pattern parameter node.
fn write_pattern_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    modifiers: Option<BindingModifier>,
    pattern: LocalNodeId<Pattern>,
    ty: Option<LocalNodeId<Expression>>,
    default: Option<LocalNodeId<Expression>>,
    is_static_parameter: bool,
) -> FormatResult<bool> {
    // defaulted runtime parameters
    if !is_static_parameter && let Some(default) = default {
        let format_left = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            // modifiers
            format_binding_modifiers_prefix_maybe(f, modifiers)?;

            // pattern
            write!(f, [pattern])?;

            // modifiers
            format_binding_modifiers_postfix_maybe(f, modifiers)?;

            // type
            write_parameter_type_with_infix(f, node_id, false, ty)?;

            Ok(())
        });

        write!(
            f,
            [group(&format_left), space(), token("="), space(), default]
        )?;
        return Ok(ty.is_some());
    }

    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;

    // pattern
    write!(f, [pattern])?;

    // modifiers
    format_binding_modifiers_postfix_maybe(f, modifiers)?;

    // type and default
    write_parameter_type_and_default(f, node_id, is_static_parameter, ty, default)
}

/// Format one variadic named parameter node.
fn write_variadic_named_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    modifiers: Option<BindingModifier>,
    name: StringId,
    ty: Option<LocalNodeId<Expression>>,
    is_static_parameter: bool,
    defer_prefix_annotations_after_spread: bool,
) -> FormatResult<bool> {
    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;

    // keyword
    write!(f, [token("...")])?;

    // spread boundary prefix annotations
    if defer_prefix_annotations_after_spread {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    // name
    write!(f, [name])?;

    // type
    write_parameter_type_with_infix(f, node_id, is_static_parameter, ty)
}

/// Format one variadic pattern parameter node.
fn write_variadic_pattern_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Parameter>,
    modifiers: Option<BindingModifier>,
    pattern: LocalNodeId<Pattern>,
    ty: Option<LocalNodeId<Expression>>,
    is_static_parameter: bool,
    defer_prefix_annotations_after_spread: bool,
) -> FormatResult<bool> {
    // modifiers
    format_binding_modifiers_prefix_maybe(f, modifiers)?;

    // keyword
    write!(f, [token("...")])?;

    // spread boundary prefix annotations
    if defer_prefix_annotations_after_spread {
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    // pattern
    write!(f, [pattern])?;

    // type
    write_parameter_type_with_infix(f, node_id, is_static_parameter, ty)
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
        write!(f, [prefix_annotations(f.context(), node_id)])?;
    }

    let is_typescript = f.context().options.language_type.is_typescript();
    let is_static_parameter = is_typescript && parameter_is_static(f.context(), node_id);
    let wrote_type_infix = match parameter {
        Parameter::Named {
            modifiers,
            name,
            ty,
            default,
        } => write_named_parameter(
            f,
            node_id,
            *modifiers,
            *name,
            *ty,
            *default,
            is_static_parameter,
        )?,
        Parameter::Pattern {
            modifiers,
            pattern,
            ty,
            default,
        } => write_pattern_parameter(
            f,
            node_id,
            *modifiers,
            *pattern,
            *ty,
            *default,
            is_static_parameter,
        )?,
        Parameter::VariadicNamed {
            modifiers,
            name,
            ty,
        } => write_variadic_named_parameter(
            f,
            node_id,
            *modifiers,
            *name,
            *ty,
            is_static_parameter,
            defer_prefix_annotations_after_spread,
        )?,
        Parameter::VariadicPattern {
            modifiers,
            pattern,
            ty,
        } => write_variadic_pattern_parameter(
            f,
            node_id,
            *modifiers,
            *pattern,
            *ty,
            is_static_parameter,
            defer_prefix_annotations_after_spread,
        )?,
        Parameter::Error => false,
    };

    write_parameter_trailing_annotations(
        f,
        node_id,
        wrote_type_infix,
        suppress_separator_boundary_annotations,
    )?;

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

/// Return whether a constructor parameter list should break.
pub(crate) fn should_break_function_parameters(
    context: &DestackFormatContext<'_>,
    parameters: &[LocalNodeId<Parameter>],
) -> bool {
    parameters.len() > 1
        && parameters
            .iter()
            .any(|parameter_id| parameter_has_constructor_property_modifier(context, *parameter_id))
}

/// Return whether one parameter pattern is object-like or array-like destructuring.
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
        Pattern::Object { .. } | Pattern::TaggedObject { .. } | Pattern::Array { .. }
    )
}

/// Return whether one parameter default expression is safe to hug.
fn parameter_default_is_huggable(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    match context.tree.get(expression_id) {
        Expression::ObjectExpression { properties, .. } => properties.is_empty(),
        Expression::ArrayExpression { elements } => elements.is_empty(),
        Expression::Identifier { .. } => true,
        _ => false,
    }
}

/// Return whether a single parameter should keep compact outer parentheses.
pub(crate) fn single_parameter_should_hug(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    if parameter_is_variadic(context, parameter_id) {
        return false;
    }

    if parameter_has_constructor_property_modifier(context, parameter_id) {
        return false;
    }

    match context.tree.get(parameter_id) {
        Parameter::Named { ty, default, .. } => {
            default.is_none()
                && ty.is_some_and(|ty| {
                    matches!(
                        context.tree.get(ty),
                        Expression::TypeLiteral(_) | Expression::TypeMapped { .. }
                    )
                })
        }
        Parameter::Pattern { default, .. } => {
            parameter_pattern_is_destructuring(context, parameter_id)
                && default
                    .is_none_or(|default_id| parameter_default_is_huggable(context, default_id))
        }
        Parameter::VariadicNamed { .. } | Parameter::VariadicPattern { .. } => false,
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

/// Return whether a signature return type carries a boundary line-postfix annotation.
pub(crate) fn signature_return_type_has_line_suffix_boundary_annotation(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<Expression>>,
) -> bool {
    let Some(return_type) = return_type else {
        return false;
    };
    let annotations = context.annotation_ids(return_type);
    !context
        .end_of_line_raw_doc_comments_after(context.span(return_type).end)
        .is_empty()
        || annotations.iter().any(|annotation_id| {
            matches!(
                context.annotation(*annotation_id).position(),
                AnnotationPosition::LinePostfixBoundary
            )
        })
}

/// Return whether one block body should be preceded by a space.
pub(crate) fn expression_body_requires_head_space(
    context: &DestackFormatContext<'_>,
    body_expression: LocalNodeId<Expression>,
) -> bool {
    fn annotations_require_head_spacing(
        context: &DestackFormatContext<'_>,
        annotations: &[LocalNodeId<Annotation>],
        raw_prefix_doc_comments: &[Comment],
    ) -> Option<bool> {
        if annotations.is_empty() {
            if raw_prefix_doc_comments.is_empty() {
                return None;
            }

            if raw_prefix_doc_comments
                .iter()
                .any(|comment: &Comment| comment.preceded_by_newline())
            {
                return Some(false);
            }

            return Some(true);
        }

        let has_block_prefix_annotation = annotations.iter().copied().any(|annotation_id| {
            matches!(
                context.annotation(annotation_id),
                Annotation::Decorator {
                    position: AnnotationPosition::BlockPrefix,
                    ..
                }
            )
        }) || raw_prefix_doc_comments
            .iter()
            .any(|comment: &Comment| comment.preceded_by_newline());
        if has_block_prefix_annotation {
            return Some(false);
        }

        let has_line_prefix_annotation = annotations.iter().copied().any(|annotation_id| {
            matches!(
                context.annotation(annotation_id),
                Annotation::Decorator {
                    position: AnnotationPosition::LinePrefix,
                    ..
                }
            )
        }) || raw_prefix_doc_comments
            .iter()
            .any(|comment: &Comment| !comment.preceded_by_newline());
        if has_line_prefix_annotation {
            return Some(true);
        }

        None
    }

    let Expression::Block(block_id) = context.tree.get(body_expression) else {
        return true;
    };
    let block_prefix_doc_comments = context.raw_prefix_doc_comments_for(*block_id);
    if let Some(requires_space) = annotations_require_head_spacing(
        context,
        context.annotation_ids(*block_id),
        &block_prefix_doc_comments,
    ) {
        return requires_space;
    }

    let block = context.tree.get(*block_id);
    if let Some(first_expression) = block.first_expression()
        && let first_expression_prefix_doc_comments =
            context.raw_prefix_doc_comments_for(first_expression)
        && let Some(requires_space) = annotations_require_head_spacing(
            context,
            context.annotation_ids(first_expression),
            &first_expression_prefix_doc_comments,
        )
    {
        return requires_space;
    }

    true
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
    let has_modifier_parameters = parameters
        .iter()
        .copied()
        .any(|parameter_id| parameter_has_constructor_property_modifier(f.context(), parameter_id));
    let should_emit_trailing_separator = !disallow_trailing_separator
        && f.context().options.trailing_comma == TrailingComma::All
        && !has_variadic_tail;

    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'_, '_>| {
            write!(
                f,
                [
                    token("("),
                    soft_block_indent(&format_with(|f: &mut DestackFormatter<'_, '_>| {
                        for (index, parameter_id) in parameters.iter().copied().enumerate() {
                            if index > 0 {
                                write!(f, [token(",")])?;

                                if has_modifier_parameters {
                                    write!(f, [hard_line_break()])?;
                                } else {
                                    write!(f, [soft_line_break_or_space()])?;
                                }
                            }

                            write!(f, [group(&parameter_id)])?;
                        }

                        if should_emit_trailing_separator {
                            write!(f, [if_group_breaks(&token(","))])?;
                        }

                        Ok(())
                    })),
                    token(")")
                ]
            )
        }))
        .should_expand(should_expand)]
    )?;
    Ok(())
}

/// Write one hug-style parameter list with space separated entries.
pub(crate) fn write_signature_hug_parameter_list(
    f: &mut DestackFormatter<'_, '_>,
    parameters: &[LocalNodeId<Parameter>],
) -> FormatResult<()> {
    write!(
        f,
        [
            token("("),
            format_with(|f: &mut DestackFormatter<'_, '_>| {
                f.join_with(&space())
                    .entries(parameters.iter().copied())
                    .finish()
            }),
            token(")")
        ]
    )?;

    Ok(())
}

/// Write one empty parameter list and keep delimiter-interior comments inside `()`.
pub(crate) fn write_empty_parameter_list_with_interior_comments<'ast, T: Node + Clone + 'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
) -> FormatResult<()>
where
    NodeTree: NodeTreeImpl<T>,
{
    let node_span = f.context().span(node_id);
    let node_text = f.context().file.span_str(node_span).as_bytes();
    let open_parenthesis_offset = node_text.iter().position(|byte| *byte == b'(');
    let close_parenthesis_offset = open_parenthesis_offset.and_then(|open_offset| {
        node_text[open_offset.saturating_add(1)..]
            .iter()
            .position(|byte| *byte == b')')
            .map(|close_offset| open_offset + 1 + close_offset)
    });

    let Some(open_parenthesis_offset) = open_parenthesis_offset else {
        write!(f, [token("()")])?;
        return Ok(());
    };
    let Some(close_parenthesis_offset) = close_parenthesis_offset else {
        write!(f, [token("()")])?;
        return Ok(());
    };

    let interior_start = node_span
        .start
        .saturating_add(open_parenthesis_offset as u32 + 1);
    let interior_end = node_span
        .start
        .saturating_add(close_parenthesis_offset as u32);
    let interior_comment_nodes = {
        let comments = f.context().comments();
        comments
            .comments_in_range(interior_start, interior_end)
            .to_vec()
    };
    if interior_comment_nodes.is_empty() {
        write!(f, [token("()")])?;
        return Ok(());
    }

    // keep one inline block comment compact: `(/* comment */)`
    if interior_comment_nodes.len() == 1 {
        let comment_id = interior_comment_nodes[0];
        let comment_span = comment_id.span;
        let comment = comment_id;
        if comment.is_block() && !f.context().span_starts_on_own_line(comment_span) {
            write!(f, [token("(")])?;
            format_raw_comment(f, comment)?;
            write!(f, [token(")")])?;
            return Ok(());
        }
    }

    write!(
        f,
        [
            token("("),
            soft_block_indent(&format_with(|f| {
                for (index, comment_id) in interior_comment_nodes.iter().copied().enumerate() {
                    if index > 0 {
                        write!(f, [hard_line_break()])?;
                    }

                    format_raw_comment(f, comment_id)?;
                }

                Ok(())
            })),
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
        write!(
            f,
            [group(&format_with(|f: &mut DestackFormatter<'_, '_>| {
                write!(
                    f,
                    [
                        token("("),
                        soft_block_indent(&separated_entries(
                            ",",
                            &clauses_vec,
                            TrailingSeparator::Omit,
                            None,
                        )),
                        token(")")
                    ]
                )
            }))]
        )?;
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
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        write!(f, [self.left, token(":"), space()])?;
        write_type_expression_with_inline_prefix_annotations(f, self.right)?;

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

        Ok(())
    }
}
