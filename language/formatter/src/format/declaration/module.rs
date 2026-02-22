use crate::format::collection::property::{format_block_of_members, format_key_with_quote_policy};
use crate::format::declaration::dispatch::{
    format_declaration_export_modifier, format_super_type_clause,
};
use crate::format::declaration::signature::format_where_clause_with_break;
use crate::format::declaration::statement::format_block_of_statements;
use crate::{DestackFormatter, empty_block_with_infix_annotations};
use destack_ast::{
    Declaration, DeclarationDescriptor, DeclarationKind, DependencyKind, Expression, Generics,
    Heritage, ImportAliasTarget, Key, Keyword, LocalNodeId, Member, Name, NamespaceKind,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

/// Format a global augmentation declaration.
pub(crate) fn format_global_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

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
                format_block_of_statements(f, expressions, false)
            })))]
        )?;
        write!(
            f,
            [f.context().block_infix_annotations(node_id), token("}")]
        )?;
    }

    Ok(())
}

/// Format a namespace declaration.
pub(crate) fn format_namespace_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: NamespaceKind,
    generics: &Generics,
    expressions: &[LocalNodeId<Expression>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // keyword
    if kind == NamespaceKind::Module {
        write!(f, [token("module")])?;
    } else {
        write!(f, [Keyword::Namespace])?;
    }

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
                format_block_of_statements(f, expressions, false)
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

    Ok(())
}

/// Format an import alias declaration.
pub(crate) fn format_import_alias_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    kind: DependencyKind,
    target: &ImportAliasTarget,
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // keyword
    write!(f, [Keyword::Import])?;
    if kind == DependencyKind::Type {
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
    Ok(())
}

/// Format an extension declaration.
pub(crate) fn format_extension_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    descriptor: &DeclarationDescriptor,
    generics: &Generics,
    target_type: LocalNodeId<Expression>,
    heritage: &Heritage,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    // export
    format_declaration_export_modifier(f, node_id, descriptor)?;

    // kind
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // keyword
    write!(f, [Keyword::Extension])?;

    // named extensions: `extension Name<T> of Target`
    // anonymous extensions: `extension<T> of Target`
    if let Some(name) = descriptor.name {
        // named: name first, then generics
        write!(f, [space(), name])?;

        if let Some(static_arguments) = generics.static_parameters.as_ref()
            && !static_arguments.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("<"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                            .entries(static_arguments)
                            .finish()
                    })),
                    token(">")
                ])]
            )?;
        }
    } else {
        // anonymous: generics first, no name
        if let Some(static_arguments) = generics.static_parameters.as_ref()
            && !static_arguments.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("<"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
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

    Ok(())
}
