use super::property::{
    format_member_field, format_method_like, format_node_with_directive, name_requires_quote_group,
};
use crate::annotation::{
    decorator_prefix_annotations, infix_or_postfix_annotations, prefix_comments_before_decorators,
};
use crate::collection::format_block_nodes_with_ignore_ranges_after;
use crate::declaration::signature::{
    default_generic_parameter_trailing_separator, format_where_clause, write_generic_parameter_list,
};
use crate::declaration::{
    write_keyword_prefix, write_statement_terminator_after_anchor, write_visibility_prefix,
};
use crate::file::{
    node_has_ignore_directive, node_has_trailing_ignore_directive, write_ignored_node,
    write_source_span,
};
use crate::{FormatNode, TsppFormatter};
use tspp_dir::{Declaration, Keyword, LocalNodeId, Member, NodeType, TypeExpression};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::{space, token};
use tspp_fir::write;
use tspp_repository::QuoteProperty;
use tspp_source::Span;

/// Return the initial comment range before the first class member.
fn member_block_initial_gap(
    f: &TsppFormatter<'_, '_>,
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
        .previous_token_before_span(first_member_start_span)?;
    let gap_start = f
        .context()
        .previous_token_before_span(open_brace_token.span)
        .filter(|token| token.span.file == first_member_span.file)
        .map_or(open_brace_token.span.end, |token| token.span.end);

    Some((gap_start, first_member_prefix_start))
}

/// Format a block of members with empty annotations and ignored ranges.
pub(crate) fn format_block_of_members<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    members: &[LocalNodeId<Member>],
) -> FormatResult<()> {
    let initial_gap = member_block_initial_gap(f, members);

    format_block_nodes_with_ignore_ranges_after(f, members, initial_gap, |f, member_id| {
        write!(f, [member_id])
    })
}

/// Return whether one class member should force quoted names.
#[inline]
fn class_member_should_force_quotes<'ast>(
    f: &TsppFormatter<'ast, '_>,
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
        let name = f.context().tree.get(member_id).name();

        name.is_some_and(|name| name_requires_quote_group(f.context(), name))
    })
}

/// Write one type annotation.
fn write_declared_type<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
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
    f: &mut TsppFormatter<'ast, '_>,
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
        f: &mut TsppFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Member::Method {
            name,
            signature,
            abstraction,
            body,
            visibility,
            is_optional,
            is_ambient,
            is_static,
            is_accessor,
            ..
        } = self
        {
            return format_node_with_directive(f, node_id, true, |f| {
                let force_quotes = class_member_should_force_quotes(f, node_id);

                format_method_like(
                    f,
                    node_id,
                    *name,
                    signature,
                    *abstraction,
                    *body,
                    *visibility,
                    *is_ambient,
                    *is_static,
                    *is_accessor,
                    *is_optional,
                    force_quotes,
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
                    is_ambient,
                    is_abstract,
                    is_override,
                } => {
                    // prefixes
                    write_keyword_prefix(f, Keyword::Declare, *is_ambient)?;
                    write_visibility_prefix(f, *visibility)?;
                    write_keyword_prefix(f, Keyword::Abstract, *is_abstract)?;
                    write_keyword_prefix(f, Keyword::Override, *is_override)?;

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
                        format_where_clause(f, where_clauses)?;
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
                    is_ambient,
                    is_abstract,
                    is_override,
                } => {
                    // prefixes
                    write_keyword_prefix(f, Keyword::Declare, *is_ambient)?;
                    write_visibility_prefix(f, *visibility)?;
                    write_keyword_prefix(f, Keyword::Abstract, *is_abstract)?;
                    write_keyword_prefix(f, Keyword::Override, *is_override)?;

                    // keyword and name
                    write!(f, [Keyword::Const, space(), *name])?;

                    // type and value
                    write_declared_type(f, *declared_type)?;
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), *value])?;
                    }
                }
                Member::Field { .. } => {
                    let force_quotes = class_member_should_force_quotes(f, node_id);

                    format_member_field(f, node_id, self, force_quotes)?;
                }
                Member::StaticBlock { body } => {
                    // keyword
                    write!(f, [Keyword::Static, space()])?;

                    // body
                    write!(f, [*body])?;
                }
                Member::ConstBlock { body } => {
                    // keyword
                    write!(f, [Keyword::Const, space()])?;

                    // body
                    write!(f, [*body])?;
                }
                Member::Method { .. } => {}
                Member::Error => {
                    write_source_span(f, f.context().span(node_id))?;
                }
            }

            // declaration-like members own one trailing terminator
            if matches!(
                self,
                Member::AssociatedType { .. }
                    | Member::AssociatedConst { .. }
                    | Member::Field { .. }
            ) {
                write_member_terminator(f, node_id)?;
            }

            Ok(())
        })
    }
}
