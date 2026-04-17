use crate::format::annotation::{
    block_infix_annotations, infix_or_postfix_annotations, prefix_annotations,
    write_raw_leading_comments,
};
use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Asynchrony, DecoratorPosition, Expression, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature, GenericParameter, Keyword, LocalNodeId, Node, NodeTree, NodeTreeImpl,
    Parameter, Pattern, TypeExpression, VarianceModifier, Visibility, WhereClause,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
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
pub(crate) fn write_type_parameter_constraint_and_default<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    constraint: Option<LocalNodeId<TypeExpression>>,
    default: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    // constraint
    if let Some(constraint) = constraint {
        let group_id = f.group_id("constraint");

        write!(
            f,
            [
                space(),
                Keyword::Extends,
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
fn write_parameter_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    declared_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    // declared type
    if let Some(declared_type) = declared_type {
        write!(f, [token(":"), space(), declared_type])?;
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

/// Write one return type annotation after boundary comments.
pub(crate) fn write_signature_return_type_with_boundary_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    return_type: LocalNodeId<TypeExpression>,
) -> FormatResult<()> {
    let trailing_comments = f.context().raw_comments_in_trailing_for(return_type);

    // boundary comments
    if !trailing_comments.is_empty() {
        write!(f, [space()])?;
        write_raw_leading_comments(f, &trailing_comments)?;
        write!(f, [space()])?;
    }

    write!(f, [return_type])
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
            | Pattern::Array { .. }
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

    matches!(
        context.tree.get(default),
        Expression::Identifier { .. }
            | Expression::QualifiedReference { .. }
            | Expression::ScalarLiteral(_)
            | Expression::This
            | Expression::Super
    )
}

/// Format one named parameter.
fn write_named_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: destack_core::StringId,
    visibility: Option<Visibility>,
    is_readonly: bool,
    is_optional: bool,
    declared_type: Option<LocalNodeId<TypeExpression>>,
    default: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // prefixes
    write_visibility_prefix(f, visibility)?;
    write_readonly_prefix(f, is_readonly)?;

    // name
    write!(f, [name])?;
    write_optional_suffix(f, is_optional)?;

    // trailers
    write_parameter_type(f, declared_type)?;
    write_parameter_default(f, default)
}

/// Format one pattern parameter.
fn write_pattern_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern: LocalNodeId<Pattern>,
    is_optional: bool,
    declared_type: Option<LocalNodeId<TypeExpression>>,
    default: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // pattern
    write!(f, [pattern])?;
    write_optional_suffix(f, is_optional)?;

    // trailers
    write_parameter_type(f, declared_type)?;
    write_parameter_default(f, default)
}

/// Format one variadic named parameter.
fn write_variadic_named_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: destack_core::StringId,
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
    write_parameter_type(f, declared_type)
}

/// Format one variadic pattern parameter.
fn write_variadic_pattern_parameter<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    pattern: LocalNodeId<Pattern>,
    declared_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    // variadic pattern
    write!(f, [token("..."), pattern])?;

    // type
    write_parameter_type(f, declared_type)
}

/// Format one parameter body.
fn format_parameter_node<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
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
        } => write_pattern_parameter(f, *pattern, *is_optional, *declared_type, *default),
        Parameter::VariadicNamed {
            name,
            visibility,
            is_readonly,
            declared_type,
        } => write_variadic_named_parameter(f, *name, *visibility, *is_readonly, *declared_type),
        Parameter::VariadicPattern {
            pattern,
            declared_type,
        } => write_variadic_pattern_parameter(f, *pattern, *declared_type),
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
        format_parameter_node(f, self)?;
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
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

/// Return whether one parameter list should break.
pub(crate) fn should_break_function_parameters(
    context: &DestackFormatContext<'_>,
    parameters: &[LocalNodeId<Parameter>],
) -> bool {
    parameters.iter().copied().any(|parameter_id| {
        context.node_has_newline(parameter_id)
            || context.has_annotation(parameter_id)
            || parameter_pattern_is_destructuring(context, parameter_id)
    })
}

/// Return whether one single parameter should hug.
pub(crate) fn single_parameter_should_hug(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    if parameter_is_variadic(context, parameter_id) {
        return false;
    }

    parameter_pattern_is_destructuring(context, parameter_id)
        && parameter_default_is_huggable(context, parameter_id)
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

    // mode
    if let Some(mode) = signature.mode {
        if let Some(keyword) = mode.to_keyword() {
            write!(f, [keyword])?;
        }

        if has_name_or_key || mode == FunctionMode::New {
            write!(f, [space()])?;
        }
    }

    // keyword and generator
    if is_declaration_style
        && signature.kind == FunctionKind::Function
        && signature.mode != Some(FunctionMode::Constructor)
        && signature.mode != Some(FunctionMode::New)
    {
        write!(f, [Keyword::Function])?;

        if signature.cardinality == FunctionCardinality::Generator {
            write!(f, [token("*")])?;
        }

        write!(f, [space()])?;
    } else if signature.cardinality == FunctionCardinality::Generator {
        write!(f, [token("*")])?;

        if is_declaration_style {
            write!(f, [space()])?;
        }
    }

    Ok(())
}

/// Return whether a signature return type carries a boundary line annotation.
pub(crate) fn signature_return_type_has_line_suffix_boundary_annotation(
    context: &DestackFormatContext<'_>,
    return_type: Option<LocalNodeId<TypeExpression>>,
) -> bool {
    return_type.is_some_and(|return_type| {
        context
            .annotation_ids(return_type)
            .iter()
            .copied()
            .any(|annotation_id| {
                matches!(
                    context.annotation(annotation_id).position,
                    DecoratorPosition::LinePostfixBoundary
                )
            })
    })
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
    should_expand: bool,
    disallow_trailing_parameter_separator: bool,
) -> FormatResult<()> {
    let trailing_separator = if disallow_trailing_parameter_separator {
        TrailingSeparator::Omit
    } else {
        default_generic_parameter_trailing_separator(f)
    };

    let body = separated_entries(",", parameters, trailing_separator, None);
    write!(
        f,
        [group(&format_args![
            token("("),
            soft_block_indent(&body),
            token(")")
        ])
        .should_expand(should_expand)]
    )
}

/// Write one hugged parameter list.
pub(crate) fn write_signature_hug_parameter_list<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameters: &[LocalNodeId<Parameter>],
) -> FormatResult<()> {
    write!(
        f,
        [group(&format_args![token("("), parameters[0], token(")")])]
    )
}

/// Write one empty parameter list with interior annotations.
pub(crate) fn write_empty_parameter_list_with_interior_comments<'ast, T>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<T>,
) -> FormatResult<()>
where
    T: Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
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
    write!(
        f,
        [group(&format_args![
            soft_line_break_or_space(),
            Keyword::Where,
            indent(&format_args![soft_line_break_or_space(), body])
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
                variance,
                constraint,
                default,
            } => {
                // variance
                write_variance_prefix(f, *variance)?;

                // name and trailers
                write!(f, [*name])?;
                write_type_parameter_constraint_and_default(f, *constraint, *default)?;
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
                write_parameter_type(f, *declared_type)?;
                write_parameter_default(f, *default)?;
            }
            GenericParameter::Error => {
                write!(f, [token("/* ERROR */")])?;
            }
        }

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}
