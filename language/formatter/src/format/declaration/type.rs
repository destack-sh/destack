use crate::format::annotation::{infix_or_postfix_annotations, prefix_annotations};
use crate::format::collection::member::format_block_of_members;
use crate::format::declaration::declaration::{
    format_declaration_export_modifier, format_super_type_clause,
    format_super_type_clause_with_expand,
};
use crate::format::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    write_generic_parameter_list,
};
use crate::format::expression::{format_block_of_type_members, format_expression};
use crate::{
    Decorator, DestackFormatContext, DestackFormatter, FormatNode,
    empty_block_with_infix_annotations,
};
use destack_ast::{
    ClassDeclaration, Declaration, EnumDeclaration, EnumField, EnumKind, Expression,
    InterfaceDeclaration, Keyword, LocalNodeId, LocalNodeIdAny, Member, StructDeclaration,
    TypeMember,
};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

/// Return whether a declaration expression is a decorated class declaration.
pub(crate) fn expression_is_decorated_class_declaration(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Expression::Declaration(declaration_id) = context.tree.get(expression_id) else {
        return false;
    };
    let Declaration::Class(_) = context.tree.get(*declaration_id) else {
        return false;
    };

    // expression decorators
    if context
        .annotation_ids(expression_id)
        .iter()
        .any(|annotation_id| matches!(context.annotation(*annotation_id), Decorator { .. }))
    {
        return true;
    }

    // declaration decorators
    context
        .annotation_ids(*declaration_id)
        .iter()
        .any(|annotation_id| matches!(context.annotation(*annotation_id), Decorator { .. }))
}

/// Write one declaration generic parameter list.
fn write_declaration_generic_parameters<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    generic_parameters: &[LocalNodeId<destack_ast::GenericParameter>],
) -> FormatResult<()> {
    // generic parameters
    if !generic_parameters.is_empty() {
        write_generic_parameter_list(
            f,
            generic_parameters,
            default_generic_parameter_trailing_separator(f),
        )?;
    }

    Ok(())
}

/// Write one declaration where clause list.
fn write_declaration_where_clauses<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    where_clauses: &[LocalNodeId<destack_ast::WhereClause>],
) -> FormatResult<()> {
    // where clauses
    if !where_clauses.is_empty() {
        format_where_clause_with_break(f, where_clauses)?;
    }

    Ok(())
}

/// Write one declaration member body.
fn write_member_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<Member>],
    break_before_body: bool,
) -> FormatResult<()> {
    // empty body
    if members.is_empty() {
        if break_before_body {
            write!(
                f,
                [
                    hard_line_break(),
                    empty_block_with_infix_annotations(node_id)
                ]
            )?;
        } else {
            write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        }

        return Ok(());
    }

    // member body
    if break_before_body {
        write!(f, [hard_line_break(), token("{"), hard_line_break()])?;
    } else {
        write!(f, [space(), token("{"), hard_line_break()])?;
    }

    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            format_block_of_members(f, members)
        })))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Write one declaration type-member body.
fn write_type_member_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    members: &[LocalNodeId<TypeMember>],
    break_before_body: bool,
) -> FormatResult<()> {
    // empty body
    if members.is_empty() {
        if break_before_body {
            write!(
                f,
                [
                    hard_line_break(),
                    empty_block_with_infix_annotations(node_id)
                ]
            )?;
        } else {
            write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        }

        return Ok(());
    }

    // member body
    if break_before_body {
        write!(f, [hard_line_break(), token("{"), hard_line_break()])?;
    } else {
        write!(f, [space(), token("{"), hard_line_break()])?;
    }

    write!(
        f,
        [group(&block_indent(&format_with(move |f| {
            format_block_of_type_members(f, members)
        })))]
    )?;
    write!(f, [hard_line_break(), token("}")])
}

/// Return whether an anonymous class expression should start heritage on the next line.
fn class_heritage_starts_on_new_line(
    declaration_expression_id: Option<LocalNodeId<Expression>>,
    declaration: &ClassDeclaration,
) -> bool {
    declaration.name.is_none()
        && declaration_expression_id.is_some()
        && (declaration.extends_expression.is_some() || !declaration.implements_types.is_empty())
}

/// Format one struct declaration.
pub(crate) fn format_struct_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &StructDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    if declaration.ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    // head
    write!(f, [Keyword::Struct, space(), declaration.name])?;
    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

    // heritage
    format_super_type_clause(f, Keyword::Extends, &declaration.embedded_types)?;
    format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;

    // where clauses
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_member_body(f, node_id, &declaration.members, false)
}

/// Format one class declaration.
pub(crate) fn format_class_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration_expression_id: Option<LocalNodeId<Expression>>,
    declaration: &ClassDeclaration,
) -> FormatResult<()> {
    let starts_heritage_on_new_line =
        class_heritage_starts_on_new_line(declaration_expression_id, declaration);

    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    if declaration.ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    if declaration.is_abstract {
        write!(f, [Keyword::Abstract, space()])?;
    }

    // head
    write!(f, [Keyword::Class])?;
    if let Some(name) = declaration.name {
        write!(f, [space(), name])?;
    }

    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;

    // heritage
    if let Some(extends_expression) = declaration.extends_expression {
        if starts_heritage_on_new_line {
            write!(f, [soft_line_break_or_space()])?;
        } else {
            write!(f, [space()])?;
        }

        write!(f, [Keyword::Extends, space()])?;

        let extends_expression_node = f.context().tree.get(extends_expression);
        format_expression(f, extends_expression, extends_expression_node, false)?;
    }

    format_super_type_clause_with_expand(
        f,
        Keyword::Implements,
        &declaration.implements_types,
        false,
        starts_heritage_on_new_line && declaration.extends_expression.is_none(),
    )?;

    // where clauses
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_member_body(
        f,
        node_id,
        &declaration.members,
        starts_heritage_on_new_line,
    )
}

/// Return the ordered enum body nodes.
fn ordered_enum_body_nodes(
    context: &DestackFormatContext<'_>,
    fields: &[LocalNodeId<EnumField>],
    members: &[LocalNodeId<Member>],
) -> Vec<LocalNodeIdAny> {
    let mut nodes = Vec::with_capacity(fields.len() + members.len());

    // field nodes
    for field_id in fields {
        nodes.push((*field_id).into_any());
    }

    // member nodes
    for member_id in members {
        nodes.push((*member_id).into_any());
    }

    // source order
    nodes.sort_by_key(|node_id| context.span_by_id(node_id.id).start);
    nodes
}

/// Write one enum body.
fn write_enum_body<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    fields: &[LocalNodeId<EnumField>],
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    let nodes = ordered_enum_body_nodes(f.context(), fields, members);

    // empty body
    if nodes.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        return Ok(());
    }

    // body
    write!(f, [space(), token("{"), hard_line_break()])?;

    for (index, node_id) in nodes.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [node_id])?;
    }

    write!(f, [hard_line_break(), token("}")])
}

/// Format one enum declaration.
pub(crate) fn format_enum_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &EnumDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    if declaration.ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    if declaration.kind == EnumKind::Const {
        write!(f, [Keyword::Const, space()])?;
    }

    // head
    write!(f, [Keyword::Enum])?;
    if let Some(name) = declaration.name {
        write!(f, [space(), name])?;
    }

    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;
    format_super_type_clause(f, Keyword::Implements, &declaration.implements_types)?;
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_enum_body(f, node_id, &declaration.fields, &declaration.members)
}

/// Format one interface declaration.
pub(crate) fn format_interface_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    declaration: &InterfaceDeclaration,
) -> FormatResult<()> {
    // prefixes
    format_declaration_export_modifier(f, node_id, declaration.export)?;
    if declaration.ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    if declaration.is_nominal {
        write!(f, [Keyword::Newtype, space()])?;
    }

    // head
    write!(f, [Keyword::Interface])?;
    if let Some(name) = declaration.name {
        write!(f, [space(), name])?;
    }

    write_declaration_generic_parameters(f, &declaration.generic_parameters)?;
    format_super_type_clause(f, Keyword::Extends, &declaration.extends_types)?;
    write_declaration_where_clauses(f, &declaration.where_clauses)?;

    // body
    write_type_member_body(f, node_id, &declaration.members, false)
}

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        node_id: LocalNodeId<EnumField>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        // name
        write!(f, [self.name])?;

        // value
        if let Some(value) = self.value {
            write!(f, [space(), token("="), space(), value])?;
        }

        // separator
        write!(f, [token(",")])?;
        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])
    }
}
