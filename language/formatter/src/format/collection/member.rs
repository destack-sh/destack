use super::property::{
    format_field_like, format_method_like, format_node_with_directive, key_requires_quote_group,
};
use crate::format::annotation::{
    decorator_prefix_annotations, infix_or_postfix_annotations,
    prefix_annotations_without_decorators,
};
use crate::format::collection::format_block_nodes_with_ignore_ranges;
use crate::format::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    write_generic_parameter_list,
};
use crate::format::declaration::write_statement_terminator_after_anchor;
use crate::format::file::{node_has_ignore_directive, write_ignored_node};
use crate::{DestackFormatter, FormatNode};
use destack_ast::{
    Ambientness, Declaration, Keyword, LocalNodeId, Member, TypeExpression, Visibility,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{space, token};
use destack_fir::write;

/// Format a block of members with empty-annotation and ignore-range handling.
pub(crate) fn format_block_of_members<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    format_block_nodes_with_ignore_ranges(f, members, |f, member_id| write!(f, [member_id]))
}

/// Return whether one class member should force quoted keys.
#[inline]
fn class_member_should_force_quote_keys<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Member>,
) -> bool {
    if f.context().options.quote_props != destack_workspace::QuoteProperty::Consistent {
        return false;
    }

    if f.context().options.language_type.is_destack() {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().parent(node_id) else {
        return false;
    };
    if parent_type != destack_ast::NodeType::Declaration {
        return false;
    }

    let parent_id = LocalNodeId::<Declaration>::new(parent_id);
    let Declaration::Class(class) = f.context().tree.get(parent_id) else {
        return false;
    };

    class.members.iter().copied().any(|member_id| {
        let key = f.context().tree.get(member_id).key().copied();

        key.is_some_and(|key| key_requires_quote_group(f.context(), key))
    })
}

/// Write one visibility prefix.
fn write_visibility_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    visibility: Option<Visibility>,
) -> FormatResult<()> {
    // visibility
    if let Some(visibility) = visibility {
        let keyword = match visibility {
            Visibility::Public => Keyword::Public,
            Visibility::Protected => Keyword::Protected,
            Visibility::Private => Keyword::Private,
        };
        write!(f, [keyword, space()])?;
    }

    Ok(())
}

/// Write one ambient prefix.
fn write_ambient_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    ambient: Ambientness,
) -> FormatResult<()> {
    // ambient
    if ambient.is_ambient() {
        write!(f, [Keyword::Declare, space()])?;
    }

    Ok(())
}

/// Write one static prefix.
fn write_static_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_static: bool,
) -> FormatResult<()> {
    // static
    if is_static {
        write!(f, [Keyword::Static, space()])?;
    }

    Ok(())
}

/// Write one abstract prefix.
fn write_abstract_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_abstract: bool,
) -> FormatResult<()> {
    // abstract
    if is_abstract {
        write!(f, [Keyword::Abstract, space()])?;
    }

    Ok(())
}

/// Write one override prefix.
fn write_override_prefix<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    is_override: bool,
) -> FormatResult<()> {
    // override
    if is_override {
        write!(f, [Keyword::Override, space()])?;
    }

    Ok(())
}

/// Write one type annotation.
fn write_declared_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    declared_type: Option<LocalNodeId<TypeExpression>>,
) -> FormatResult<()> {
    // declared type
    if let Some(declared_type) = declared_type {
        write!(f, [token(":"), space(), declared_type])?;
    }

    Ok(())
}

/// Write a class-like member terminator and any same-line trailing comments.
fn write_member_terminator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Member>,
) -> FormatResult<()> {
    write_statement_terminator_after_anchor(f, f.context().span(node_id).end)
}

impl<'ast> FormatNode<'ast, Member> for Member {
    fn format_node(
        &self,
        node_id: LocalNodeId<Member>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Member::Method {
            key,
            signature,
            body,
            visibility,
            ambient,
            is_static,
            is_accessor,
            is_comptime,
            ..
        } = self
        {
            return format_node_with_directive(f, node_id, true, |f| {
                let force_quote_keys = class_member_should_force_quote_keys(f, node_id);

                format_method_like(
                    f,
                    node_id,
                    *visibility,
                    *ambient,
                    *is_static,
                    *is_accessor,
                    *is_comptime,
                    *key,
                    signature,
                    *body,
                    force_quote_keys,
                )?;

                // abstract and signature-only methods own their terminator
                if body.is_none() {
                    write!(f, [token(";")])?;
                }

                Ok(())
            });
        }

        let is_ignored = node_has_ignore_directive(f.context(), node_id);
        if is_ignored {
            write!(
                f,
                [prefix_annotations_without_decorators(f.context(), node_id)]
            )?;
            write!(f, [decorator_prefix_annotations(f.context(), node_id)])?;
            write_ignored_node(f, node_id)?;
            write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

            // ignored declaration-like members still own one terminator
            if matches!(
                self,
                Member::AssociatedType { .. }
                    | Member::AssociatedConst { .. }
                    | Member::Field { .. }
                    | Member::Embed { .. }
            ) {
                write_member_terminator(f, node_id)?;
            }

            return Ok(());
        }

        format_node_with_directive(f, node_id, false, |f| {
            match self {
                Member::AssociatedType {
                    name,
                    generic_parameters,
                    where_clauses,
                    constraint,
                    value,
                    visibility,
                    ambient,
                    is_abstract,
                    is_override,
                    is_static,
                } => {
                    // prefixes
                    write_ambient_prefix(f, *ambient)?;
                    write_visibility_prefix(f, *visibility)?;
                    write_static_prefix(f, *is_static)?;
                    write_abstract_prefix(f, *is_abstract)?;
                    write_override_prefix(f, *is_override)?;

                    // head
                    write!(f, [Keyword::Type, space(), *name])?;

                    // generic parameters
                    if !generic_parameters.is_empty() {
                        write_generic_parameter_list(
                            f,
                            generic_parameters,
                            default_generic_parameter_trailing_separator(f),
                        )?;
                    }

                    // where clauses
                    if !where_clauses.is_empty() {
                        format_where_clause_with_break(f, where_clauses)?;
                    }

                    // type bound
                    write_declared_type(f, *constraint)?;

                    // value
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), *value])?;
                    }
                }
                Member::AssociatedConst {
                    name,
                    declared_type,
                    value,
                    visibility,
                    ambient,
                    is_static,
                } => {
                    // prefixes
                    write_ambient_prefix(f, *ambient)?;
                    write_visibility_prefix(f, *visibility)?;
                    write_static_prefix(f, *is_static)?;

                    // keyword pair
                    write!(
                        f,
                        [Keyword::Comptime, space(), Keyword::Const, space(), *name]
                    )?;

                    // type and value
                    write_declared_type(f, *declared_type)?;
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), *value])?;
                    }
                }
                Member::Field {
                    key,
                    declared_type,
                    default,
                    is_optional,
                    is_readonly,
                    mutability,
                    visibility,
                    ambient,
                    is_abstract,
                    is_override,
                    is_static,
                    is_const_asserted,
                    is_accessor,
                    ..
                } => {
                    let force_quote_keys = class_member_should_force_quote_keys(f, node_id);

                    format_field_like(
                        f,
                        node_id,
                        *key,
                        *declared_type,
                        *visibility,
                        *ambient,
                        *is_static,
                        *is_abstract,
                        *is_override,
                        *is_readonly,
                        *mutability,
                        *is_accessor,
                        *is_optional,
                        *is_const_asserted,
                        *default,
                        force_quote_keys,
                    )?;
                }
                Member::Embed {
                    value,
                    visibility,
                    ambient,
                    is_static,
                } => {
                    // prefixes
                    write_ambient_prefix(f, *ambient)?;
                    write_visibility_prefix(f, *visibility)?;
                    write_static_prefix(f, *is_static)?;

                    // embedded type
                    write!(f, [token("..."), *value])?;
                }
                Member::StaticBlock { body } => {
                    // keyword
                    write!(f, [Keyword::Static, space()])?;

                    // body
                    write!(f, [*body])?;
                }
                Member::ComptimeBlock { body } => {
                    // keyword
                    write!(f, [Keyword::Comptime, space()])?;

                    // body
                    write!(f, [*body])?;
                }
                Member::Method { .. } => {}
                Member::Error => {
                    write!(f, [token("/* ERROR */")])?;
                }
            }

            // declaration-like members own one trailing terminator
            if matches!(
                self,
                Member::AssociatedType { .. }
                    | Member::AssociatedConst { .. }
                    | Member::Field { .. }
                    | Member::Embed { .. }
            ) {
                write_member_terminator(f, node_id)?;
            }

            Ok(())
        })
    }
}
