use crate::format::directive::{node_has_ignore_directive, write_ignored_node};
use crate::{DestackFormatContext, DestackFormatter};
use destack_ast as ast;
use destack_ast::{Expression, Keyword, LocalNodeId};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    format_with, group, hard_line_break, if_group_breaks, soft_block_indent,
    soft_space_or_block_indent, space, token,
};
use destack_fir::write;

/// Return whether one mapped type has a newline immediately after `{`.
fn mapped_type_has_newline_after_opening_brace(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let source = context.span_str(context.span(node_id));
    let mut iter = source.as_bytes().iter().copied().skip(1).peekable();

    while let Some(byte) = iter.next() {
        match byte {
            b'\n' | b'\r' => return true,
            b' ' | b'\t' => {}
            b'/' => match iter.peek() {
                Some(&b'/') => {
                    iter.next();
                    return iter.any(|byte| matches!(byte, b'\n' | b'\r'));
                }
                Some(&b'*') => {
                    iter.next();
                    while let Some(byte) = iter.next() {
                        if matches!(byte, b'\n' | b'\r') {
                            return true;
                        }
                        if byte == b'*' && matches!(iter.peek(), Some(&b'/')) {
                            iter.next();
                            break;
                        }
                    }
                }
                _ => return false,
            },
            _ => return false,
        }
    }

    false
}

/// Write the readonly modifier of one mapped type.
fn write_mapped_type_readonly_modifier<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: ast::TypeMappedModifiers,
) -> FormatResult<()> {
    match modifiers.readonly {
        ast::TypeModifier::Add => write!(f, [token("readonly"), space()]),
        ast::TypeModifier::Remove => write!(f, [token("-readonly"), space()]),
        ast::TypeModifier::None => Ok(()),
    }
}

/// Write the optional modifier suffix of one mapped type.
fn write_mapped_type_optional_modifier<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    modifiers: ast::TypeMappedModifiers,
) -> FormatResult<()> {
    match modifiers.optional {
        ast::TypeModifier::Add => write!(f, [token("?")]),
        ast::TypeModifier::Remove => write!(f, [token("-?")]),
        ast::TypeModifier::None => Ok(()),
    }
}

/// Write the bracketed key clause of one mapped type.
fn write_mapped_type_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    parameter: &ast::TypeMappedParameter,
    modifiers: ast::TypeMappedModifiers,
) -> FormatResult<()> {
    write!(f, [token("[")])?;
    write!(f, [parameter.name])?;
    write!(f, [space(), Keyword::In, space(), parameter.constraint])?;

    if let Some(key_remap) = parameter.key_remap {
        write!(f, [space(), Keyword::As, space(), key_remap])?;
    }

    write!(f, [token("]")])?;
    write_mapped_type_optional_modifier(f, modifiers)
}

/// Write raw comments between one mapped field terminator and `}`.
fn write_mapped_type_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let node_span = f.context().span(node_id);
    let value_span = f.context().span(value_id);
    let Some(close_brace_token) = f.context().last_non_trivia_token_in_span(node_span) else {
        return Ok(());
    };
    if close_brace_token.span.file != node_span.file
        || close_brace_token.span.start <= value_span.end
    {
        return Ok(());
    }

    let comment_nodes = f
        .context()
        .comment_nodes_in_range(value_span.end, close_brace_token.span.start);
    if comment_nodes.is_empty() {
        return Ok(());
    }

    write!(f, [hard_line_break()])?;
    for (index, comment_id) in comment_nodes.iter().copied().enumerate() {
        if index > 0 {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [comment_id])?;
    }

    Ok(())
}

/// Format one mapped type using an OXC-like shell.
pub(crate) fn format_type_mapped_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    parameter: &ast::TypeMappedParameter,
    modifiers: ast::TypeMappedModifiers,
    value: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // local suppression path
    if node_has_ignore_directive(f.context(), node_id) {
        write_ignored_node(f, node_id)?;
        return Ok(());
    }

    let should_expand = mapped_type_has_newline_after_opening_brace(f.context(), node_id);
    let should_insert_space_around_brackets = f.context().options.bracket_spacing;
    let field_terminator = token(";");

    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        // comments immediately after `{` belong inside the mapped-type shell
        if should_expand
            && (f.context().has_infix_annotation(node_id)
                || f.context().has_postfix_annotation(node_id))
        {
            write!(
                f,
                [crate::format::annotation::infix_or_postfix_annotations(
                    f.context(),
                    node_id
                )]
            )?;
        }

        write_mapped_type_readonly_modifier(f, modifiers)?;

        let format_clause = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write_mapped_type_clause(f, parameter, modifiers)
        });
        write!(f, [group(&format_clause)])?;

        write!(f, [token(":"), space(), value])?;
        write!(f, [if_group_breaks(&field_terminator)])?;
        write_mapped_type_trailing_comments(f, node_id, value)?;

        Ok(())
    });

    if should_insert_space_around_brackets {
        let format_body_inner = soft_space_or_block_indent(&format_inner);
        let format_body = group(&format_body_inner).should_expand(should_expand);
        write!(f, [token("{"), format_body, token("}")])
    } else {
        let format_body_inner = soft_block_indent(&format_inner);
        let format_body = group(&format_body_inner).should_expand(should_expand);
        write!(f, [token("{"), format_body, token("}")])
    }
}
