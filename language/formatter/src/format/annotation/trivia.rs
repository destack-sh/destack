use crate::{DestackFormatter, FormatNode};
use destack_ast::{Blank, Comment, CommentStyle, Doc, DocStyle, LocalNodeId};
use destack_fir::format::{Buffer, FormatResult, hard_line_break};
use destack_fir::prelude::{empty_line, space, text};
use destack_fir::write;

impl<'ast> FormatNode<'ast, Blank> for Blank {
    /// Format one blank annotation node.
    fn format_node(
        &self,
        _node_id: LocalNodeId<Blank>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // reduce any number of blank lines to a single one
        write!(f, [empty_line()])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Doc> for Doc {
    /// Format one documentation comment node.
    fn format_node(
        &self,
        node_id: LocalNodeId<Doc>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let raw_comment = f.context().doc_raw_text(node_id);
        let is_block_comment = self.style == DocStyle::Star;
        format_comment_like_raw_text(f, raw_comment, is_block_comment)
    }
}

impl<'ast> FormatNode<'ast, Comment> for Comment {
    /// Format one comment node.
    fn format_node(
        &self,
        node_id: LocalNodeId<Comment>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let raw_comment = f.context().comment_raw_text(node_id);
        let is_block_comment = self.style == CommentStyle::Star;
        format_comment_like_raw_text(f, raw_comment, is_block_comment)
    }
}

/// Format one raw comment or documentation token.
fn format_comment_like_raw_text<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
    is_block_comment: bool,
) -> FormatResult<()> {
    let is_multiline_comment = raw_comment.contains('\n');

    // render one-line comments with trailing whitespace normalized
    if !is_multiline_comment {
        write!(f, [text(raw_comment.trim_end())])?;
        return Ok(());
    }

    // preserve star-aligned block comments in conventional form
    if is_block_comment && block_comment_is_alignable(raw_comment) {
        format_alignable_block_comment(f, raw_comment)?;
        return Ok(());
    }

    format_multiline_comment_raw(f, raw_comment)
}

/// Format one multiline raw comment with explicit line breaks.
fn format_multiline_comment_raw<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
) -> FormatResult<()> {
    let mut lines = raw_comment.lines();
    let Some(first_line) = lines.next() else {
        return Ok(());
    };

    write!(f, [text(first_line.trim_end_matches('\r').trim_end())])?;

    let remaining_lines = lines.collect::<Vec<_>>();
    let common_indent = common_multiline_comment_indent(&remaining_lines);
    for line in remaining_lines {
        let line = line.trim_end_matches('\r');
        let line = if common_indent == 0 {
            line
        } else {
            let mut end_index = 0;
            for byte in line.as_bytes().iter().take(common_indent) {
                if !matches!(*byte, b' ' | b'\t') {
                    break;
                }

                end_index += 1;
            }

            &line[end_index..]
        };
        write!(f, [hard_line_break(), text(line)])?;
    }

    Ok(())
}

/// Return the common leading indentation width for multiline comment lines.
fn common_multiline_comment_indent(lines: &[&str]) -> usize {
    let mut common_indent = usize::MAX;

    for line in lines {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        let line_indent = line
            .as_bytes()
            .iter()
            .take_while(|byte| matches!(**byte, b' ' | b'\t'))
            .count();
        common_indent = common_indent.min(line_indent);
    }

    if common_indent == usize::MAX {
        return 0;
    }

    common_indent
}

/// Format one alignable multiline block comment.
fn format_alignable_block_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    raw_comment: &str,
) -> FormatResult<()> {
    let mut lines = raw_comment.lines();
    let Some(first_line) = lines.next() else {
        return Ok(());
    };

    write!(f, [text(first_line.trim_end_matches('\r').trim_end())])?;
    for line in lines {
        let trimmed_line = line.trim_end_matches('\r').trim();
        let normalized_line = if let Some(prefix) = trimmed_line.strip_suffix("*/") {
            let prefix = prefix.trim_end();
            if prefix.is_empty() {
                "*/".to_string()
            } else {
                format!("{prefix} */")
            }
        } else {
            trimmed_line.to_string()
        };
        write!(f, [hard_line_break(), space(), text(&normalized_line)])?;
    }

    Ok(())
}

/// Return whether one multiline block comment is alignable on `*` prefixes.
fn block_comment_is_alignable(raw_comment: &str) -> bool {
    raw_comment
        .lines()
        .skip(1)
        .all(|line| line.trim_start_matches('\r').trim_start().starts_with('*'))
}
