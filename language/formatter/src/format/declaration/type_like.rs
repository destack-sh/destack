use super::dispatch::format_super_type_clause;
use crate::argument::list_like;
use crate::expression::expression_has_static_type_arguments;
use crate::property::format_block_of_members;
use crate::r#where::format_where_clause_with_break;
use crate::{DestackFormatter, empty_block_with_infix_annotations};
use destack_ast::{
    Annotation, Declaration, DeclarationAbstraction, DeclarationDescriptor, DeclarationKind,
    EnumField, EnumKind, Expression, Generics, Heritage, Keyword, LocalNodeId, Member, NodeType,
    TypeKind, WhereClause,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::{format_args, write};

/// Return whether heritage clauses include a line-postfix-boundary comment.
fn heritage_has_line_postfix_boundary_annotation(
    f: &DestackFormatter<'_, '_>,
    heritage: &Heritage,
) -> bool {
    let mut heritage_types: Vec<LocalNodeId<Expression>> = Vec::new();
    if let Some(extends_types) = heritage.extends_types.as_ref() {
        heritage_types.extend(extends_types.iter().copied());
    }
    if let Some(implements_types) = heritage.implements_types.as_ref() {
        heritage_types.extend(implements_types.iter().copied());
    }

    heritage_types.into_iter().any(|expression_id| {
        let Some(annotation_ids) = f.context().get_annotations(expression_id) else {
            return false;
        };

        annotation_ids.iter().any(|annotation_id| {
            matches!(
                f.context().tree.get::<Annotation>(*annotation_id),
                Annotation::Comment {
                    position: destack_ast::AnnotationPosition::LinePostfixBoundary,
                    ..
                }
            )
        })
    })
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

/// Format anonymous class heritage, preserving oxfmt style for generic extends and implements.
fn format_anonymous_class_heritage<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    heritage: &Heritage,
) -> FormatResult<()> {
    if let Some(extends_types) = heritage.extends_types.as_ref()
        && !extends_types.is_empty()
    {
        let has_generic_extends = extends_types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(f.context(), type_id));
        if has_generic_extends {
            write!(f, [space(), Keyword::Extends, space(), token("(")])?;
            write!(
                f,
                [group(&indent(&format_args![
                    soft_line_break_or_space(),
                    format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                            .entries(extends_types)
                            .finish()
                    })
                ]))]
            )?;
            write!(f, [soft_line_break_or_space(), token(")")])?;
        } else {
            format_super_type_clause(f, Keyword::Extends, extends_types)?;
        }
    }

    if let Some(implements_types) = heritage.implements_types.as_ref()
        && !implements_types.is_empty()
    {
        let has_generic_implements = implements_types
            .iter()
            .copied()
            .any(|type_id| expression_has_static_type_arguments(f.context(), type_id));
        if has_generic_implements {
            write!(f, [space(), Keyword::Implements, space()])?;
            write!(
                f,
                [format_with(|f| {
                    f.join_with(&format_args![&token(","), space()])
                        .entries(implements_types)
                        .finish()
                })]
            )?;
        } else {
            format_super_type_clause(f, Keyword::Implements, implements_types)?;
        }
    }

    Ok(())
}

/// Format a struct or class declaration and return whether it ended early.
#[allow(clippy::too_many_arguments)]
pub(super) fn format_struct_or_class_declaration<'ast>(
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

    let is_assignment_rhs_anonymous_class = if is_class && descriptor.name.is_none() {
        declaration_expression_id.is_some_and(|expression_id| {
            let Some((parent_id, parent_type)) = f.context().get_parent(expression_id) else {
                return false;
            };
            if parent_type != NodeType::Expression {
                return false;
            }

            let parent_expression = LocalNodeId::<Expression>::new(parent_id);
            matches!(
                f.context().tree.get(parent_expression),
                Expression::Assign { right, .. } if *right == expression_id
            )
        })
    } else {
        false
    };

    format_declaration_static_parameters(f, generics)?;
    if is_assignment_rhs_anonymous_class {
        format_anonymous_class_heritage(f, heritage)?;
    } else {
        format_declaration_heritage(f, heritage, true)?;
    }
    format_declaration_where_clauses(f, generics.where_clauses.as_deref())?;
    let has_heritage_line_boundary_annotation =
        heritage_has_line_postfix_boundary_annotation(f, heritage);

    if has_heritage_line_boundary_annotation {
        write!(f, [hard_line_break()])?;
    } else {
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
pub(super) struct EnumDeclarationFormatData<'a> {
    /// The shared declaration descriptor.
    pub descriptor: &'a DeclarationDescriptor,
    /// The enum kind.
    pub kind: EnumKind,
    /// The enum generics.
    pub generics: &'a Generics,
    /// The enum heritage clauses.
    pub heritage: &'a Heritage,
    /// The enum fields.
    pub fields: &'a [LocalNodeId<EnumField>],
    /// The enum members.
    pub members: &'a [LocalNodeId<Member>],
}

/// Format an enum declaration and return whether it ended early.
pub(super) fn format_enum_declaration<'ast>(
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
        [group(&format_args![block_indent(&format_with(|f| {
            f.join_with(&format_args![&hard_line_break()])
                .entries(fields)
                .finish()
        })),])]
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

/// Format an interface declaration and return whether it ended early.
pub(super) fn format_interface_declaration<'ast>(
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
