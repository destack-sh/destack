use crate::annotation::{
    declaration_body_boundary_prefix_annotations,
    declaration_expression_body_boundary_prefix_annotations, is_lambda_arrow_prefix_annotation,
};
use crate::argument::list_like;
use crate::block::format_block_of_statements;
use crate::expression::{is_expression_breakable, lambda_expression_should_break};
use crate::key::format_key_with_quote_policy;
use crate::property::format_block_of_members;
use crate::r#where::format_where_clause_with_break;
use crate::{
    DestackFormatContext, DestackFormatter, FormatNode, empty_block_with_infix_annotations,
};
use destack_ast::{
    Annotation, Argument, Asynchrony, Declaration, DeclarationAbstraction, DeclarationDescriptor,
    DeclarationKind, DependencyKind, DependencyMode, EnumField, EnumKind, Expression,
    FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode, Generics, Heritage,
    ImportAliasTarget, Key, Keyword, LocalNodeId, Member, Mutability, Name, NodeType, Parameter,
    Pattern, PatternField, TypeKind, Visibility, WhereClause,
};
use destack_fir::format::{BestFittingMode, FormatResult};
use destack_fir::prelude::*;
use destack_fir::{best_fitting, format_args, write};
use destack_source::Span;
use destack_workspace::ArrowParentheses;

/// Return whether this file is a module typescript source.
fn is_module_typescript_source(file_name: &str) -> bool {
    file_name.ends_with(".mts") || file_name.ends_with(".cts")
}

/// Return whether a lambda parameter should preserve multiline destructuring.
fn lambda_parameter_should_expand(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    let Parameter::Pattern { pattern, .. } = context.tree.get(parameter_id) else {
        return false;
    };

    let is_destructuring_pattern = matches!(
        context.tree.get(*pattern),
        Pattern::Object { .. } | Pattern::TaggedObject { .. } | Pattern::Array { .. }
    );
    if !is_destructuring_pattern {
        return false;
    }

    context.has_newline(context.get_span(*pattern))
}

/// Return whether a lambda tail parameter is a simple named binding.
fn lambda_parameter_is_simple_tail(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    matches!(
        context.tree.get(parameter_id),
        Parameter::Named {
            modifiers: None,
            ty: None,
            default: None,
            ..
        }
    )
}

/// Collect deferred prefix annotations that belong between lambda parameters and arrow token.
fn collect_lambda_arrow_prefix_annotations(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
    expression_id: Option<LocalNodeId<Expression>>,
    argument_id: Option<LocalNodeId<Argument>>,
) -> Vec<LocalNodeId<Annotation>> {
    fn append_node_annotations<T: destack_ast::Node + Clone>(
        context: &DestackFormatContext<'_>,
        node_id: LocalNodeId<T>,
        output: &mut Vec<LocalNodeId<Annotation>>,
    ) where
        destack_ast::NodeTree: destack_ast::NodeTreeImpl<T>,
    {
        let Some(annotations) = context.get_annotations(node_id) else {
            return;
        };

        for annotation_id in annotations {
            let annotation = context.tree.get::<Annotation>(annotation_id);
            let should_defer = is_lambda_arrow_prefix_annotation(
                context,
                node_id,
                annotation_id,
                annotation.position(),
            );
            if should_defer {
                output.push(annotation_id);
            }
        }
    }

    let mut annotations = Vec::new();
    append_node_annotations(context, declaration_id, &mut annotations);
    if let Some(expression_id) = expression_id {
        append_node_annotations(context, expression_id, &mut annotations);
    }
    if let Some(argument_id) = argument_id {
        append_node_annotations(context, argument_id, &mut annotations);
    }

    annotations.sort_by_key(|annotation_id| {
        let span = context.get_span::<Annotation>(*annotation_id);
        (span.start, span.end, annotation_id.id)
    });
    annotations.dedup_by_key(|annotation_id| annotation_id.id);
    annotations
}

/// Write deferred prefix annotations between lambda parameters and arrow token.
fn write_deferred_lambda_arrow_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    declaration_id: LocalNodeId<Declaration>,
    expression_id: Option<LocalNodeId<Expression>>,
    argument_id: Option<LocalNodeId<Argument>>,
) -> FormatResult<bool> {
    let annotations = collect_lambda_arrow_prefix_annotations(
        f.context(),
        declaration_id,
        expression_id,
        argument_id,
    );
    if annotations.is_empty() {
        return Ok(false);
    }

    for annotation_id in annotations {
        write!(f, [space()])?;
        let annotation = f.context().tree.get::<Annotation>(annotation_id);
        annotation.format_node(annotation_id, f)?;
    }
    write!(f, [space()])?;
    Ok(true)
}

/// Write deferred declaration boundary prefix annotations between headers and `{`.
fn write_deferred_declaration_body_boundary_prefix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    declaration_id: LocalNodeId<Declaration>,
    declaration_expression_id: Option<LocalNodeId<Expression>>,
) -> FormatResult<bool> {
    let mut annotations = declaration_body_boundary_prefix_annotations(f.context(), declaration_id);
    if let Some(expression_id) = declaration_expression_id {
        annotations.extend(declaration_expression_body_boundary_prefix_annotations(
            f.context(),
            expression_id,
        ));
    }
    annotations.sort_by_key(|annotation_id| {
        let span = f.context().get_span::<Annotation>(*annotation_id);
        (span.start, span.end, annotation_id.id)
    });
    annotations.dedup_by_key(|annotation_id| annotation_id.id);
    if annotations.is_empty() {
        return Ok(false);
    }

    for annotation_id in annotations {
        write!(f, [space()])?;
        let annotation = f.context().tree.get::<Annotation>(annotation_id);
        annotation.format_node(annotation_id, f)?;
    }
    write!(f, [space()])?;
    Ok(true)
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

/// Return whether this parameter declares any modifiers.
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

    if fields.len() >= 3 {
        return true;
    }

    if fields.len() <= 1 {
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
fn parameter_should_force_expand_in_signature(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
) -> bool {
    if context.has_newline(context.get_span(parameter_id)) {
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

    if context.has_newline(context.get_span(pattern_id)) {
        return true;
    }

    parameter_object_pattern_should_expand(context, pattern_id)
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

/// Format a super type clause.
pub(crate) fn format_super_type_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    keyword: Keyword,
    types: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    assert!(!types.is_empty());

    write!(
        f,
        [
            space(),
            keyword,
            group(&indent(&format_args![
                soft_line_break_or_space(),
                format_with(|f| {
                    f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                        .entries(types)
                        .finish()
                })
            ])),
        ]
    )
}

/// Format shared export and declaration modifiers for declarations.
fn format_declaration_header_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    descriptor: &DeclarationDescriptor,
) -> FormatResult<()> {
    if let Some(export) = descriptor.export {
        write!(f, [export, space()])?;
    }
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }
    Ok(())
}

/// Format declaration static parameters when present.
fn format_declaration_static_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generics: &Generics,
) -> FormatResult<()> {
    if let Some(static_parameters) = generics.static_parameters.as_ref()
        && !static_parameters.is_empty()
    {
        write!(f, [list_like("<", ">", ",", static_parameters)])?;
    }
    Ok(())
}

/// Format declaration where clauses when present.
fn format_declaration_where_clauses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    where_clauses: Option<&[LocalNodeId<WhereClause>]>,
) -> FormatResult<()> {
    if let Some(where_clauses) = where_clauses
        && !where_clauses.is_empty()
    {
        format_where_clause_with_break(f, where_clauses)?;
    }
    Ok(())
}

/// Format declaration extends and optional implements clauses.
fn format_declaration_heritage<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    heritage: &Heritage,
    include_implements: bool,
) -> FormatResult<()> {
    if let Some(extends_types) = heritage.extends_types.as_ref()
        && !extends_types.is_empty()
    {
        format_super_type_clause(f, Keyword::Extends, extends_types)?;
    }
    if include_implements
        && let Some(implements_types) = heritage.implements_types.as_ref()
        && !implements_types.is_empty()
    {
        format_super_type_clause(f, Keyword::Implements, implements_types)?;
    }
    Ok(())
}

/// Format a struct or class declaration body and return whether it ended early.
fn format_struct_or_class_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration_expression_id: Option<LocalNodeId<Expression>>,
    descriptor: &DeclarationDescriptor,
    generics: &Generics,
    heritage: &Heritage,
    members: &[LocalNodeId<Member>],
    is_class: bool,
) -> FormatResult<bool> {
    format_declaration_header_prefix(f, descriptor)?;

    if descriptor.abstraction == DeclarationAbstraction::Abstract {
        write!(f, [Keyword::Abstract, space()])?;
    }

    if is_class {
        write!(f, [Keyword::Class])?;
    } else {
        write!(f, [Keyword::Struct])?;
    }

    if let Some(name) = descriptor.name {
        write!(f, [space(), name])?;
    }

    format_declaration_static_parameters(f, generics)?;
    format_declaration_heritage(f, heritage, true)?;
    format_declaration_where_clauses(f, generics.where_clauses.as_deref())?;

    let has_deferred_declaration_boundary_annotations =
        write_deferred_declaration_body_boundary_prefix_annotations(
            f,
            node_id,
            declaration_expression_id,
        )?;
    if !has_deferred_declaration_boundary_annotations {
        write!(f, [space()])?;
    }

    if members.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(true);
    }

    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            format_block_of_members(f, members)
        })),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;
    Ok(false)
}

/// Shared enum declaration inputs.
struct EnumDeclarationFormatData<'a> {
    descriptor: &'a DeclarationDescriptor,
    kind: EnumKind,
    generics: &'a Generics,
    heritage: &'a Heritage,
    fields: &'a [LocalNodeId<EnumField>],
    members: &'a [LocalNodeId<Member>],
}

/// Format an enum declaration body and return whether it ended early.
fn format_enum_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    data: EnumDeclarationFormatData<'_>,
) -> FormatResult<bool> {
    let EnumDeclarationFormatData {
        descriptor,
        kind,
        generics,
        heritage,
        fields,
        members,
    } = data;

    format_declaration_header_prefix(f, descriptor)?;

    if kind == EnumKind::Const {
        write!(f, [Keyword::Const, space()])?;
    }

    write!(f, [Keyword::Enum])?;

    if let Some(name) = descriptor.name {
        write!(f, [space(), name])?;
    }

    format_declaration_static_parameters(f, generics)?;
    format_declaration_heritage(f, heritage, true)?;
    format_declaration_where_clauses(f, generics.where_clauses.as_deref())?;

    write!(f, [space()])?;

    if fields.is_empty() && members.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(true);
    }

    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| f
            .join_with(&format_args![&hard_line_break()])
            .entries(fields)
            .finish())),])]
    )?;

    if !fields.is_empty() && !members.is_empty() {
        write!(f, [hard_line_break()])?;
        if !f.context().has_blank_prefix_annotation(members[0]) {
            write!(f, [empty_line()])?;
        }
    }

    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            format_block_of_members(f, members)
        })),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;
    Ok(false)
}

/// Format an interface declaration body and return whether it ended early.
fn format_interface_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: TypeKind,
    generics: &Generics,
    heritage: &Heritage,
    members: &[LocalNodeId<Member>],
) -> FormatResult<bool> {
    format_declaration_header_prefix(f, descriptor)?;

    if kind == TypeKind::Nominal {
        write!(f, [Keyword::Newtype, space()])?;
    }
    write!(f, [Keyword::Interface])?;

    if let Some(name) = descriptor.name {
        write!(f, [space(), name])?;
    }

    format_declaration_static_parameters(f, generics)?;
    format_declaration_heritage(f, heritage, false)?;
    format_declaration_where_clauses(f, generics.where_clauses.as_deref())?;

    write!(f, [space()])?;

    if members.is_empty() {
        write!(f, [empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(true);
    }

    write!(f, [token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            format_block_of_members(f, members)
        })),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;
    Ok(false)
}

impl<'ast> Format<DestackFormatContext<'ast>> for Visibility {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            Visibility::Public => write!(f, [Keyword::Public]),
            Visibility::Protected => write!(f, [Keyword::Protected]),
            Visibility::Private => write!(f, [Keyword::Private]),
        }
    }
}

impl<'ast> Format<DestackFormatContext<'ast>> for DependencyMode {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            DependencyMode::Item => write!(f, [Keyword::Export]),
            DependencyMode::Default => write!(f, [Keyword::Export, space(), Keyword::Default]),
            DependencyMode::Namespace => write!(f, [Keyword::Export]),
        }
    }
}

impl<'ast> FormatNode<'ast, Declaration> for Declaration {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declaration>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let is_lambda_declaration = matches!(
            self,
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
        );
        let declaration_expression_id =
            if let Some((parent_id, parent_type)) = f.context().get_parent(node_id) {
                if parent_type == NodeType::Expression {
                    let expression_id = LocalNodeId::<Expression>::new(parent_id);
                    match f.context().tree.get(expression_id) {
                        Expression::Declaration(parent_declaration_id)
                            if *parent_declaration_id == node_id =>
                        {
                            Some(expression_id)
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };
        let deferred_lambda_expression = if is_lambda_declaration {
            declaration_expression_id
        } else {
            None
        };
        let deferred_lambda_argument = if let Some(expression_id) = deferred_lambda_expression {
            if let Some((parent_id, parent_type)) = f.context().get_parent(expression_id) {
                if parent_type == NodeType::Argument {
                    Some(LocalNodeId::<Argument>::new(parent_id))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let lambda_is_nested_in_lambda_body = if is_lambda_declaration {
            if let Some(expression_id) = deferred_lambda_expression {
                if let Some((container_id, container_type)) = f.context().get_parent(expression_id)
                {
                    if container_type == NodeType::Declaration {
                        let container_declaration_id =
                            LocalNodeId::<Declaration>::new(container_id);
                        matches!(
                            f.context().tree.get(container_declaration_id),
                            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
                        )
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        let lambda_has_argument_ancestor = is_lambda_declaration
            && f.context()
                .get_ancestors(node_id)
                .into_iter()
                .any(|(_, node_type)| node_type == NodeType::Argument);
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        match self {
            // global augmentation
            Declaration::Global {
                descriptor,
                expressions,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [token("global")])?;

                // body
                write!(f, [space()])?;
                if expressions.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                } else {
                    write!(f, [token("{"), hard_line_break()])?;
                    write!(
                        f,
                        [group(&block_indent(&format_with(|f| {
                            format_block_of_statements(f, expressions)
                        })))]
                    )?;
                    write!(
                        f,
                        [f.context().block_infix_annotations(node_id), token("}")]
                    )?;
                }
            }
            // module
            Declaration::Namespace {
                descriptor,
                generics,
                expressions,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Namespace])?;

                // name / key
                if let Some(name) = descriptor.name {
                    write!(f, [space()])?;
                    if matches!(name, Name::String(_)) {
                        format_key_with_quote_policy(f, Key::Name(name), true)?;
                    } else {
                        write!(f, [name])?;
                    }
                }

                // where
                if let Some(where_clauses) = generics.where_clauses.as_ref()
                    && !where_clauses.is_empty()
                {
                    format_where_clause_with_break(f, where_clauses)?;
                }

                // body
                write!(f, [space()])?;
                if expressions.is_empty() {
                    write!(f, [empty_block_with_infix_annotations(node_id)])?;
                    write!(f, [f.context().any_postfix_annotations(node_id)])?;
                } else {
                    write!(f, [token("{"), hard_line_break()])?;
                    write!(
                        f,
                        [group(&block_indent(&format_with(|f| {
                            format_block_of_statements(f, expressions)
                        })))]
                    )?;
                    write!(
                        f,
                        [
                            hard_line_break(),
                            f.context().block_infix_annotations(node_id),
                            token("}")
                        ]
                    )?;
                }
            }

            // type alias
            Declaration::Type {
                descriptor,
                kind,
                mutability,
                static_parameters,
                value: value_id,
            } => {
                let header = format_with(|f| {
                    // export
                    if let Some(export) = descriptor.export {
                        write!(f, [export, space()])?;
                    }
                    // keyword
                    if *mutability == Some(Mutability::Immutable) {
                        // for readonly type expression
                        write!(f, [Keyword::Readonly])?;
                    } else if *kind == TypeKind::Structural {
                        write!(f, [Keyword::Type])?;
                    } else {
                        write!(f, [Keyword::Newtype])?;
                    }
                    // name
                    if let Some(name) = descriptor.name {
                        write!(f, [space(), name])?;
                    }
                    // static parameters
                    if let Some(static_parameters) = static_parameters {
                        write!(f, [list_like("<", ">", ",", static_parameters)])?;
                    }
                    Ok(())
                });

                // prefer keeping the value on a single line
                let format_inline = format_with(|f| {
                    write!(f, [header, space(), token("="), space(), value_id])?;
                    Ok(())
                });
                let format_soft_break = format_with(|f| {
                    write!(
                        f,
                        [group(&format_args![
                            header,
                            space(),
                            token("="),
                            indent(&format_args![soft_line_break_or_space(), value_id])
                        ])]
                    )
                });
                // expand inline if breakable (like let x = [\n ... ])
                let format_inline_expanded = format_with(|f| {
                    write!(
                        f,
                        [
                            header,
                            space(),
                            token("="),
                            space(),
                            fits_expanded(&group(value_id).should_expand(true)),
                        ]
                    )
                });
                // expand and indent the value
                let format_indented = format_with(|f| {
                    group(&format_args![
                        header,
                        space(),
                        token("="),
                        block_indent(value_id)
                    ])
                    .format(f)
                });

                let tree = f.context().tree;
                let value_expression = tree.get(*value_id);
                let value_has_prefix_annotation = f.context().has_prefix_annotation(*value_id);
                let declaration_span = f.context().get_span(node_id);
                let value_span = f.context().get_span(*value_id);
                let leading_value_span =
                    Span::new(value_span.file, declaration_span.start, value_span.start);
                let has_comment_before_value =
                    f.context().get_span_str(leading_value_span).contains("/*")
                        || f.context().get_span_str(leading_value_span).contains("//");
                let should_break_after_equals = match value_expression {
                    Expression::TypeConditional { left, .. } => {
                        !matches!(tree.get(*left), Expression::Parenthesized { .. })
                    }
                    _ => false,
                };

                if should_break_after_equals || has_comment_before_value {
                    format_soft_break.format(f)?;
                } else if is_expression_breakable(tree, tree.get(*value_id)) {
                    if value_has_prefix_annotation {
                        best_fitting![format_inline, format_soft_break, format_indented]
                            .with_mode(BestFittingMode::AllLines)
                            .format(f)?;
                    } else {
                        best_fitting![format_inline, format_inline_expanded, format_indented]
                            .with_mode(BestFittingMode::AllLines)
                            .format(f)?;
                    }
                } else {
                    best_fitting![format_inline, format_indented]
                        .with_mode(BestFittingMode::AllLines)
                        .format(f)?;
                }
                // type alias declarations need trailing semicolon (like const/let)
                write!(f, [token(";")])?;
            }

            // import alias
            Declaration::ImportAlias {
                descriptor,
                kind,
                target,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // keyword
                write!(f, [Keyword::Import])?;
                if *kind == DependencyKind::Type {
                    write!(f, [space(), Keyword::Type])?;
                }

                // name
                if let Some(name) = descriptor.name {
                    write!(f, [space(), name])?;
                }

                // target
                write!(f, [space(), token("="), space()])?;
                match target {
                    ImportAliasTarget::Require { target } => {
                        write!(
                            f,
                            [
                                token("require"),
                                token("("),
                                token("\""),
                                target,
                                token("\""),
                                token(")")
                            ]
                        )?;
                    }
                    ImportAliasTarget::Path { value } => {
                        write!(f, [*value])?;
                    }
                }

                write!(f, [token(";")])?;
            }

            // struct or class
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                members,
            }
            | Declaration::Class {
                descriptor,
                generics,
                heritage,
                members,
            } => {
                let is_class = matches!(self, Declaration::Class { .. });
                if format_struct_or_class_declaration(
                    f,
                    node_id,
                    declaration_expression_id,
                    descriptor,
                    generics,
                    heritage,
                    members,
                    is_class,
                )? {
                    return Ok(());
                }
            }

            // enum
            Declaration::Enum {
                descriptor,
                kind,
                generics,
                heritage,
                fields,
                members,
            } => {
                let data = EnumDeclarationFormatData {
                    descriptor,
                    kind: *kind,
                    generics,
                    heritage,
                    fields,
                    members,
                };
                if format_enum_declaration(f, node_id, data)? {
                    return Ok(());
                }
            }

            // interface
            Declaration::Interface {
                descriptor,
                kind,
                generics,
                heritage,
                members,
            } => {
                if format_interface_declaration(
                    f, node_id, descriptor, *kind, generics, heritage, members,
                )? {
                    return Ok(());
                }
            }

            // extension
            Declaration::Extension {
                descriptor,
                generics,
                target_type,
                heritage,
                members,
            } => {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // keyword
                write!(f, [Keyword::Extension])?;

                // For named extensions: `extension Name<T> of Target`
                // For anonymous extensions: `extension<T> of Target`
                if let Some(name) = descriptor.name {
                    // Named: name first, then generics
                    write!(f, [space(), name])?;

                    if let Some(static_arguments) = generics.static_parameters.as_ref()
                        && !static_arguments.is_empty()
                    {
                        write!(
                            f,
                            [group(&format_args![
                                token("<"),
                                soft_block_indent(&format_with(|f| {
                                    f.join_with(&format_args![
                                        &token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(static_arguments)
                                    .finish()
                                })),
                                token(">")
                            ])]
                        )?;
                    }
                } else {
                    // Anonymous: generics first (no name)
                    if let Some(static_arguments) = generics.static_parameters.as_ref()
                        && !static_arguments.is_empty()
                    {
                        write!(
                            f,
                            [group(&format_args![
                                token("<"),
                                soft_block_indent(&format_with(|f| {
                                    f.join_with(&format_args![
                                        &token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(static_arguments)
                                    .finish()
                                })),
                                token(">")
                            ])]
                        )?;
                    }
                }

                // for keyword + target type
                write!(f, [space(), Keyword::For, space(), target_type])?;

                // implements types
                if let Some(implements_types) = heritage.implements_types.as_ref()
                    && !implements_types.is_empty()
                {
                    format_super_type_clause(f, Keyword::Implements, implements_types)?;
                }

                // where
                if let Some(where_clauses) = generics.where_clauses.as_ref()
                    && !where_clauses.is_empty()
                {
                    format_where_clause_with_break(f, where_clauses)?;
                }

                // body
                if members.is_empty() {
                    write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
                    return Ok(());
                }

                // body
                write!(f, [space(), token("{"), hard_line_break()])?;
                write!(
                    f,
                    [group(&format_args![block_indent(&format_with(|f| {
                        format_block_of_members(f, members)
                    })),])]
                )?;
                write!(f, [hard_line_break(), token("}")])?;
            }

            // function
            Declaration::Function {
                descriptor,
                signature,
                body,
            } => {
                let generics = signature.generics.as_ref();

                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }

                // kind
                if descriptor.kind == DeclarationKind::Declaration {
                    write!(f, [Keyword::Declare, space()])?;
                }

                // abstraction
                match signature.abstraction {
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

                // asynchrony
                if signature.asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Async, space()])?;
                }

                // kind
                if let Some(kind) = signature.mode {
                    write!(f, [kind.to_keyword()])?;
                    if descriptor.name.is_some() || kind == FunctionMode::New {
                        write!(f, [space()])?;
                    }
                }

                // keyword
                if signature.kind == FunctionKind::Function
                    && signature.mode != Some(FunctionMode::Constructor)
                    && signature.mode != Some(FunctionMode::New)
                {
                    // function keyword
                    if signature.cardinality == FunctionCardinality::Generator {
                        write!(f, [Keyword::Function, token("*"), space()])?;
                    } else {
                        write!(f, [Keyword::Function, space()])?;
                    }
                } else {
                    // lambda (no keyword, maybe star)
                    if signature.cardinality == FunctionCardinality::Generator {
                        write!(f, [token("*"), space()])?;
                    }
                }

                // name / key
                if signature.kind == FunctionKind::Function
                    && let Some(name) = descriptor.name
                {
                    write!(f, [name])?;
                }

                // static parameters
                let has_static_parameters = generics
                    .and_then(|generics| generics.static_parameters.as_ref())
                    .is_some_and(|params| !params.is_empty());
                if let Some(static_parameters) =
                    generics.and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    let needs_jsx_disambiguation = signature.kind == FunctionKind::Lambda
                        && f.context().options.language_type.supports_jsx()
                        && static_parameters.len() == 1;
                    let needs_module_typescript_trailing_comma = signature.kind
                        == FunctionKind::Lambda
                        && static_parameters.len() == 1
                        && is_module_typescript_source(&f.context().file.name);
                    let mut static_params_list = list_like("<", ">", ",", static_parameters);
                    if needs_jsx_disambiguation || needs_module_typescript_trailing_comma {
                        static_params_list.force_trailing_separator();
                    }
                    write!(f, [static_params_list])?;
                }

                let mut dynamic_parameters =
                    Vec::with_capacity(signature.dynamic_parameters.len() + 1);
                if let Some(this_parameter) = signature.this_parameter {
                    dynamic_parameters.push(this_parameter);
                }
                dynamic_parameters.extend(signature.dynamic_parameters.iter().copied());

                // omit arrow function parentheses for simple single param lambdas
                let can_omit_parens = signature.kind == FunctionKind::Lambda
                    && signature.this_parameter.is_none()
                    && !has_static_parameters
                    && signature.cardinality != FunctionCardinality::Generator
                    && dynamic_parameters.len() == 1
                    && matches!(
                        f.context().options.arrow_parentheses,
                        ArrowParentheses::Avoid
                    )
                    && {
                        let param = f.context().tree.get(dynamic_parameters[0]);
                        matches!(
                            param,
                            Parameter::Named {
                                modifiers: None,
                                ty: None,
                                default: None,
                                ..
                            }
                        )
                    };

                // dynamic parameters
                if can_omit_parens {
                    write!(f, [&dynamic_parameters[0]])?;
                } else if dynamic_parameters.len() == 1
                    && single_parameter_should_hug(f.context(), dynamic_parameters[0])
                {
                    write!(f, [token("("), dynamic_parameters[0], token(")")])?;
                } else if signature.kind == FunctionKind::Lambda
                    && dynamic_parameters.len() == 2
                    && lambda_parameter_should_expand(f.context(), dynamic_parameters[0])
                    && lambda_parameter_is_simple_tail(f.context(), dynamic_parameters[1])
                {
                    // keep callback parameters compact: `({ ... }, tail)`
                    write!(
                        f,
                        [
                            token("("),
                            group(&dynamic_parameters[0]).should_expand(true),
                            token(","),
                            space(),
                            dynamic_parameters[1],
                            token(")")
                        ]
                    )?;
                } else {
                    let force_expand_parameters =
                        dynamic_parameters.iter().copied().any(|parameter_id| {
                            parameter_should_force_expand_in_signature(f.context(), parameter_id)
                        }) || constructor_parameters_should_expand(
                            f.context(),
                            signature.mode,
                            &dynamic_parameters,
                        );
                    let mut dynamic_parameters_list = list_like("(", ")", ",", &dynamic_parameters);
                    dynamic_parameters_list.should_expand(force_expand_parameters);

                    let disallow_trailing_parameter_separator =
                        dynamic_parameters.last().is_some_and(|parameter_id| {
                            parameter_is_variadic(f.context(), *parameter_id)
                        }) || (signature.kind == FunctionKind::Lambda
                            && dynamic_parameters.len() == 1);
                    if disallow_trailing_parameter_separator {
                        dynamic_parameters_list.disallow_trailing_separator();
                    }

                    write!(f, [dynamic_parameters_list])?;
                }

                // block and line prefix comments between parameters and arrow are emitted here
                let has_deferred_lambda_arrow_prefix_annotations = if is_lambda_declaration {
                    write_deferred_lambda_arrow_prefix_annotations(
                        f,
                        node_id,
                        deferred_lambda_expression,
                        deferred_lambda_argument,
                    )?
                } else {
                    false
                };

                let has_deferred_lambda_arrow_boundary_annotations =
                    has_deferred_lambda_arrow_prefix_annotations;

                // return type
                if let Some(return_type) = signature.return_type {
                    if signature.kind == FunctionKind::Lambda && body.is_none() {
                        if has_deferred_lambda_arrow_boundary_annotations {
                            write!(f, [token("=>"), space(), return_type])?;
                        } else {
                            write!(f, [space(), token("=>"), space(), return_type])?;
                        }
                    } else {
                        write!(f, [token(":"), space(), return_type])?;
                    }
                }

                // where clause
                if let Some(where_clauses) =
                    generics.and_then(|generics| generics.where_clauses.as_ref())
                    && !where_clauses.is_empty()
                {
                    format_where_clause_with_break(f, where_clauses)?;
                }

                // body
                if let Some(body) = body {
                    if signature.kind == FunctionKind::Lambda {
                        let body_expression = f.context().tree.get(*body);
                        let body_is_block = matches!(body_expression, Expression::Block(_));
                        let body_is_tree =
                            matches!(body_expression, Expression::TreeExpression { .. });
                        let body_is_parenthesized_tree = matches!(
                            body_expression,
                            Expression::Parenthesized { expression: inner_id }
                                if matches!(
                                    f.context().tree.get(*inner_id),
                                    Expression::TreeExpression { .. }
                                )
                        );
                        let body_is_lambda_declaration = matches!(
                            body_expression,
                            Expression::Declaration(nested_declaration_id)
                                if matches!(
                                    f.context().tree.get(*nested_declaration_id),
                                    Declaration::Function { signature, .. }
                                        if signature.kind == FunctionKind::Lambda
                                )
                        );
                        let force_break = lambda_expression_should_break(f.context(), node_id);

                        // arrow is fine since lambdas can only have return type or body
                        if body_is_block {
                            if has_deferred_lambda_arrow_boundary_annotations
                                && lambda_is_nested_in_lambda_body
                                && lambda_has_argument_ancestor
                            {
                                write!(f, [token("=>"), space(), dedent(&body)])?;
                            } else if has_deferred_lambda_arrow_boundary_annotations {
                                write!(f, [token("=>"), space(), body])?;
                            } else {
                                write!(f, [space(), token("=>"), space(), body])?;
                            }
                        } else if body_is_tree {
                            // tree bodies need conditional parentheses when they break
                            let body_group_id = f.group_id("lambda_body");
                            let parenthesized_body = format_with(|f| {
                                write!(f, [token("("), soft_block_indent(&body), token(")")])
                            });
                            let body_break = format_with(|f| {
                                write!(
                                    f,
                                    [
                                        if_group_breaks(&parenthesized_body)
                                            .with_group_id(Some(body_group_id)),
                                        if_group_fits_on_line(&body)
                                            .with_group_id(Some(body_group_id))
                                    ]
                                )?;
                                Ok(())
                            });

                            if has_deferred_lambda_arrow_boundary_annotations {
                                write!(
                                    f,
                                    [group(&format_args![token("=>"), space(), body_break])
                                        .with_id(Some(body_group_id))
                                        .should_expand(force_break)]
                                )?;
                            } else {
                                write!(
                                    f,
                                    [group(&format_args![
                                        space(),
                                        token("=>"),
                                        space(),
                                        body_break
                                    ])
                                    .with_id(Some(body_group_id))
                                    .should_expand(force_break)]
                                )?;
                            }
                        } else if body_is_lambda_declaration
                            && has_deferred_lambda_arrow_boundary_annotations
                            && lambda_is_nested_in_lambda_body
                        {
                            // nested lambda chains keep body lambda aligned with current indent
                            let body_break = format_with(|f| write!(f, [body]));

                            write!(
                                f,
                                [group(&format_args![
                                    token("=>"),
                                    soft_line_break_or_space(),
                                    body_break
                                ])
                                .should_expand(force_break)]
                            )?;
                        } else {
                            // default expression body formatting
                            let body_break = format_with(|f| write!(f, [body]));

                            if has_deferred_lambda_arrow_boundary_annotations {
                                if body_is_parenthesized_tree {
                                    write!(
                                        f,
                                        [group(&format_args![token("=>"), space(), body_break])
                                            .should_expand(force_break)]
                                    )?;
                                } else {
                                    write!(
                                        f,
                                        [group(&format_args![
                                            token("=>"),
                                            indent(&format_args![
                                                soft_line_break_or_space(),
                                                body_break
                                            ])
                                        ])
                                        .should_expand(force_break)]
                                    )?;
                                }
                            } else {
                                if body_is_parenthesized_tree {
                                    write!(
                                        f,
                                        [group(&format_args![
                                            space(),
                                            token("=>"),
                                            space(),
                                            body_break
                                        ])
                                        .should_expand(force_break)]
                                    )?;
                                } else {
                                    write!(
                                        f,
                                        [group(&format_args![
                                            space(),
                                            token("=>"),
                                            indent(&format_args![
                                                soft_line_break_or_space(),
                                                body_break
                                            ])
                                        ])
                                        .should_expand(force_break)]
                                    )?;
                                }
                            }
                        }
                    } else {
                        write!(f, [space(), body])?;
                    }
                }

                // exported lambda declarations need trailing semicolon (they're expressions)
                // non-exported lambdas are part of another statement that adds the semicolon
                if signature.kind == FunctionKind::Lambda && descriptor.export.is_some() {
                    write!(f, [token(";")])?;
                }
            }
        }

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}
