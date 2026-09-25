use tspp_dir::{Comment, Documentation};
use tspp_fir::format::{FormatError, FormatResult};

use crate::{TsppFormatContext, TsppFormatter};

use super::render::FormattedDocumentation;

/// Format the valid documentation group beginning with one source comment.
pub(crate) fn format_documentation_comment<'ast>(
    formatter: &mut TsppFormatter<'ast, '_>,
    comment: Comment,
) -> FormatResult<Option<tspp_source::Span>> {
    let Some(documentation) = documentation_for_comment(formatter.context(), comment)? else {
        return Ok(None);
    };

    // render from the parser owned documentation model
    let available_width = documentation_width(formatter.context(), comment);
    let formatted = FormattedDocumentation::new(
        documentation,
        available_width,
        formatter.context().tree,
        formatter.context().strings,
        &formatter.context().options,
    )?;
    let span = documentation.span;
    tspp_fir::write!(formatter, [formatted])?;

    Ok(Some(span))
}

/// Return the documentation attached to one leading comment.
fn documentation_for_comment<'a>(
    context: &'a TsppFormatContext<'_>,
    comment: Comment,
) -> FormatResult<Option<&'a Documentation>> {
    if !comment.is_documentation() || comment.is_legal() || !comment.is_leading() {
        return Ok(None);
    }

    // read the exact parser attachment
    let nodes = context
        .source_index
        .documentation_nodes_at(comment.span.start);
    if nodes.len() > 1 {
        return Err(FormatError::SyntaxError {
            message: "documentation comment has multiple owners",
        });
    }
    // format unowned documentation as an ordinary comment
    let Some((_, node_id)) = nodes.first() else {
        return Ok(None);
    };
    let Some(documentation) = context.tree.get_documentation(*node_id) else {
        return Ok(None);
    };

    Ok(Some(documentation))
}

/// Return the documentation width available at its canonical indentation.
fn documentation_width(context: &TsppFormatContext<'_>, comment: Comment) -> usize {
    // find the source line and its leading indentation
    let source = context.source_text().slice_to(comment.span.start);
    let line_start = match source.rfind(['\n', '\r']) {
        Some(index) => index + 1,
        None => 0,
    };
    let line = &source[line_start..];
    let tab_width = usize::from(context.options.indent_width);
    let indentation = line
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .map(|byte| if byte == b'\t' { tab_width } else { 1 })
        .sum::<usize>();

    // reserve one indentation step when moving inline documentation onto its own line
    let is_inline = line.bytes().any(|byte| !matches!(byte, b' ' | b'\t'));
    let output_indentation = if is_inline {
        indentation + tab_width
    } else {
        indentation
    };

    // reserve the documentation marker width
    usize::from(context.options.line_width).saturating_sub(output_indentation + 4)
}
