use super::property::{
    format_field_like, format_method_like, format_node_with_directive, key_requires_quote_group,
};
use crate::annotation::{
    decorator_prefix_annotations, infix_or_postfix_annotations, prefix_comments_before_decorators,
};
use crate::collection::format_block_nodes_with_ignore_ranges_after;
use crate::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause_with_break,
    write_generic_parameter_list,
};
use crate::declaration::write_statement_terminator_after_anchor;
use crate::file::{
    node_has_ignore_directive, node_has_trailing_ignore_directive, write_ignored_node,
};
use crate::{DestackFormatter, FormatNode};
use destack_ast::{
    Ambientness, Declaration, Keyword, LocalNodeId, Member, NodeType, TypeExpression, Visibility,
};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{space, token};
use destack_fir::write;
use destack_source::Span;
use destack_workspace::QuoteProperty;

/// Return the initial comment range before the first class member.
fn member_block_initial_gap(
    f: &DestackFormatter<'_, '_>,
    members: &[LocalNodeId<Member>],
) -> Option<(u32, u32)> {
    let first_member_id = members.first().copied()?;
    let first_member_span = f.context().span(first_member_id);
    let first_member_prefix_start = f
        .context()
        .annotation_ids(first_member_id)
        .iter()
        .copied()
        .map(|annotation_id| f.context().annotation_span(annotation_id).start)
        .next()
        .unwrap_or_else(|| f.context().node_token_start(first_member_id));
    let first_member_start_span = Span::new(
        first_member_span.file,
        first_member_prefix_start,
        first_member_prefix_start,
    );
    let open_brace_token = f
        .context()
        .previous_non_trivia_token_before_span(first_member_start_span)?;
    let gap_start = f
        .context()
        .previous_non_trivia_token_before_span(open_brace_token.span)
        .filter(|token| token.span.file == first_member_span.file)
        .map_or(open_brace_token.span.end, |token| token.span.end);

    Some((gap_start, first_member_prefix_start))
}

/// Format a block of members with empty annotations and ignored ranges.
pub(crate) fn format_block_of_members<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    let initial_gap = member_block_initial_gap(f, members);

    format_block_nodes_with_ignore_ranges_after(f, members, initial_gap, |f, member_id| {
        write!(f, [member_id])
    })
}

/// Return whether one class member should force quoted keys.
#[inline]
fn class_member_should_force_quote_keys<'ast>(
    f: &DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Member>,
) -> bool {
    if f.context().options.quote_props != QuoteProperty::Consistent {
        return false;
    }

    let Some((parent_id, parent_type)) = f.context().parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Declaration {
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
    if node_has_trailing_ignore_directive(f.context(), node_id) {
        return Ok(());
    }

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
            is_optional,
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
                    *is_optional,
                    *key,
                    signature,
                    *body,
                    force_quote_keys,
                )?;

                // abstract and signature-only methods own their terminator
                if body.is_none() {
                    write_member_terminator(f, node_id)?;
                }

                Ok(())
            });
        }

        let is_ignored = node_has_ignore_directive(f.context(), node_id);
        if is_ignored {
            write!(f, [prefix_comments_before_decorators(f.context(), node_id)])?;
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
                    is_definite,
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
                        *is_definite,
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
