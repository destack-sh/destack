use super::property::{
    format_field_like, format_method_like, format_node_with_directive,
    method_signature_is_multiline_before_body,
};
use crate::format::collection::{
    TrailingSeparator, format_block_nodes_with_ignore_ranges, separated_entries,
};
use crate::format::declaration::signature::{
    format_binding_modifiers_postfix_maybe, format_binding_modifiers_prefix,
    format_binding_modifiers_prefix_maybe, write_static_parameter_list,
};
use crate::format::directive::{node_has_ignore_directive, write_ignored_node};
use crate::format::operator::write_expression_with_inline_prefix_annotations;
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Comment, Declaration, Expression, Keyword, LocalNodeId, Member};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{hard_line_break, space, token};
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
    matches!(f.context().tree.get(parent_id), Declaration::Class { .. })
}

/// Return raw end-of-line comments after one field type and before the member terminator.
fn field_type_trailing_comment_nodes<'ast>(
    f: &DestackFormatter<'ast, '_>,
    value: Option<LocalNodeId<Expression>>,
    default: Option<LocalNodeId<Expression>>,
) -> Vec<LocalNodeId<Comment>> {
    if default.is_some() {
        return Vec::new();
    }

    let Some(value_id) = value else {
        return Vec::new();
    };

    let value_span = f.context().span(value_id);
    f.context().end_of_line_comment_nodes_after(value_span.end)
}

impl<'ast> FormatNode<'ast, Member> for Member {
    fn format_node(
        &self,
        node_id: LocalNodeId<Member>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        if let Member::Method {
            modifiers,
            key,
            signature,
            body,
        } = self
        {
            return format_node_with_directive(f, node_id, true, |f| {
                let force_quote_keys = class_member_should_force_quote_keys(f, node_id);
                let signature_is_multiline_before_body = method_signature_is_multiline_before_body(
                    f.context(),
                    f.context().span(node_id),
                    *body,
                );
                format_method_like(
                    f,
                    node_id,
                    *modifiers,
                    *key,
                    signature,
                    *body,
                    force_quote_keys,
                    signature_is_multiline_before_body,
                )?;

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
                [crate::format::annotation::prefix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
            write_ignored_node(f, node_id)?;
            write!(
                f,
                [crate::format::annotation::infix_or_postfix_annotations(
                    f.context(),
                    node_id
                )]
            )?;

            // ignored class fields still get one formatter-owned terminator
            if matches!(self, Member::Field { .. }) {
                write!(f, [token(";")])?;
            }

            return Ok(());
        }

        format_node_with_directive(f, node_id, false, |f| {
            match self {
                Member::Type {
                    modifiers,
                    name,
                    static_parameters,
                    where_clauses,
                    ty,
                    value,
                } => {
                    // modifiers
                    format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                    // keyword
                    write!(f, [Keyword::Type, space()])?;

                    // name
                    write!(f, [name])?;

                    // static parameters
                    if let Some(static_parameters) = static_parameters
                        && !static_parameters.is_empty()
                    {
                        write_static_parameter_list(
                            f,
                            static_parameters,
                            TrailingSeparator::Disallowed,
                        )?;
                    }

                    // where clauses
                    if let Some(where_clauses) = where_clauses
                        && !where_clauses.is_empty()
                    {
                        write!(f, [space(), Keyword::Where, space()])?;
                        write!(
                            f,
                            [separated_entries(
                                ",",
                                where_clauses,
                                TrailingSeparator::Omit,
                                None,
                            )]
                        )?;
                    }

                    // type bound
                    if let Some(ty) = ty {
                        write!(f, [token(":"), space()])?;
                        write_expression_with_inline_prefix_annotations(f, *ty)?;
                    }

                    // value
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), value])?;
                    }
                }
                Member::ComptimeConst {
                    modifiers,
                    name,
                    ty,
                    value,
                } => {
                    // keep non comptime modifiers before the associated keyword pair
                    if let Some(mut modifiers) = *modifiers {
                        modifiers.timing = None;
                        modifiers.operator = None;
                        format_binding_modifiers_prefix(f, modifiers)?;
                    }

                    // associated comptime constants are always emitted in canonical order
                    write!(
                        f,
                        [Keyword::Comptime, space(), Keyword::Const, space(), name]
                    )?;

                    // optional type annotation
                    if let Some(ty) = ty {
                        write!(f, [token(":"), space()])?;
                        write_expression_with_inline_prefix_annotations(f, *ty)?;
                    }

                    // optional initializer
                    if let Some(value) = value {
                        write!(f, [space(), token("="), space(), value])?;
                    }
                }
                Member::Field {
                    modifiers,
                    key,
                    value,
                    default,
                } => {
                    let force_quote_keys = class_member_should_force_quote_keys(f, node_id);
                    format_field_like(f, *modifiers, *key, *value, *default, force_quote_keys)?;
                }
                Member::Embed { modifiers, value } => {
                    // modifiers
                    format_binding_modifiers_prefix_maybe(f, *modifiers)?;

                    // keyword
                    write!(f, [token("...")])?;

                    // value
                    write!(f, [value])?;

                    // modifiers
                    format_binding_modifiers_postfix_maybe(f, *modifiers)?;
                }
                Member::StaticBlock { body, .. } => {
                    // keyword
                    write!(f, [Keyword::Static, space()])?;

                    // body
                    write!(f, [body])?;
                }
                Member::ComptimeBlock { modifiers, body } => {
                    // modifiers prefix
                    if let Some(mut modifiers) = *modifiers {
                        modifiers.timing = None;
                        format_binding_modifiers_prefix(f, modifiers)?;
                    }

                    // keyword
                    write!(f, [Keyword::Comptime, space()])?;

                    // body
                    write!(f, [body])?;
                }
                Member::Method { .. } => {}
                Member::Error => {
                    write!(f, [token("/* ERROR */")])?;
                }
            }

            let needs_semicolon = matches!(self, Member::Field { .. });
            if needs_semicolon {
                write!(f, [token(";")])?;

                // field type seams
                if let Member::Field { value, default, .. } = self {
                    let trailing_comment_nodes =
                        field_type_trailing_comment_nodes(f, *value, *default);
                    for comment_id in trailing_comment_nodes {
                        let comment_span = f.context().span(comment_id);
                        if f.context().span_starts_on_own_line(comment_span) {
                            write!(f, [hard_line_break(), comment_id])?;
                        } else {
                            write!(f, [space(), comment_id])?;
                        }
                    }
                }
            }

            Ok(())
        })
    }
}
