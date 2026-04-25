use crate::format::context::FormatNodeWithoutTrailingComments;
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{Comment, CommentContent, LocalNodeId, Node, NodeTree, NodeTreeImpl};
use destack_fir::format::{Buffer, Format, FormatResult, Formatter, hard_line_break};
use destack_fir::prelude::{
    block_indent, empty_line, expand_parent, format_with, group, line_suffix, soft_block_indent,
    soft_line_break_or_space, space, text,
};
use destack_fir::write;
use destack_source::Span;

/// Return whether adjacent jsdoc comments should stay nestled together.
fn should_nestle_adjacent_doc_comments(current: Comment, next: Comment) -> bool {
    matches!(
        current.content,
        CommentContent::Jsdoc | CommentContent::JsdocLegal
    ) && matches!(
        next.content,
        CommentContent::Jsdoc | CommentContent::JsdocLegal
    ) && current.is_multiline_block()
        && next.is_multiline_block()
        && current.span.end == next.span.start
}

/// Format one comment.
pub(crate) fn format_comment<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comment: Comment,
) -> FormatResult<()> {
    f.context_mut().comments_mut().mark_comment_printed(comment);

    let comment_source = f.context().span_str(comment.span);

    // multiline block comments
    if comment.is_multiline_block() {
        if block_comment_is_alignable(comment_source) {
            let mut lines = comment_source.lines();
            let Some(first_line) = lines.next() else {
                return Ok(());
            };

            write!(f, [text(first_line.trim_end())])?;

            for line in lines {
                write!(
                    f,
                    [
                        hard_line_break(),
                        space(),
                        text(line.trim_end_matches('\r').trim())
                    ]
                )?;
            }

            return Ok(());
        }

        let mut normalized_comment = String::with_capacity(comment_source.len());
        let mut lines = comment_source.lines();
        let Some(first_line) = lines.next() else {
            return Ok(());
        };

        normalized_comment.push_str(first_line.trim_end());

        for line in lines {
            normalized_comment.push('\n');
            normalized_comment.push_str(line.trim_end_matches('\r'));
        }

        write!(f, [text(&normalized_comment)])?;
        return Ok(());
    }

    write!(f, [text(comment_source.trim_end())])
}

/// Return one leading comment formatter for one node span.
#[inline]
pub(crate) const fn format_leading_comments<'a>(span: Span) -> FormatLeadingComments<'a> {
    FormatLeadingComments::Node(span)
}

/// Format leading comments for one node or one explicit comment slice.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FormatLeadingComments<'a> {
    /// Leading comments before one node span.
    Node(Span),
    /// One explicit leading comment slice.
    Comments(&'a [Comment]),
}

impl<'a> Format<DestackFormatContext<'a>> for FormatLeadingComments<'_> {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'a>>) -> FormatResult<()> {
        fn format_leading_comments_impl<'ast>(
            comments: &[Comment],
            f: &mut DestackFormatter<'ast, '_>,
        ) -> FormatResult<()> {
            let source = f.context().source_text();
            let mut comments = comments.iter().copied().peekable();

            while let Some(comment) = comments.next() {
                format_comment(f, comment)?;

                if comment.is_block() {
                    match source.lines_after(comment.span.end) {
                        0 => {
                            let should_nestle = comments.peek().is_some_and(|next_comment| {
                                should_nestle_adjacent_doc_comments(comment, *next_comment)
                            });

                            if !should_nestle {
                                write!(f, [space()])?;
                            }
                        }
                        1 => {
                            let lines_before = {
                                let comment_cursor = f.context().comments();
                                source.get_lines_before(comment.span, comment_cursor)
                            };

                            if lines_before == 0 {
                                write!(f, [soft_line_break_or_space()])?;
                            } else {
                                write!(f, [hard_line_break()])?;
                            }
                        }
                        _ => {
                            write!(f, [empty_line()])?;
                        }
                    }
                } else {
                    match source.lines_after(comment.span.end) {
                        0 | 1 => {
                            write!(f, [hard_line_break()])?;
                        }
                        _ => {
                            write!(f, [empty_line()])?;
                        }
                    }
                }
            }

            Ok(())
        }

        match self {
            Self::Node(span) => {
                let comments = f.context().comments().comments_before(span.start);

                if comments.is_empty() {
                    return Ok(());
                }

                format_leading_comments_impl(comments, f)
            }
            Self::Comments(comments) => {
                if comments.is_empty() {
                    return Ok(());
                }

                format_leading_comments_impl(comments, f)
            }
        }
    }
}

/// Write trailing comments with line-suffix behavior.
fn write_trailing_comments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    write_trailing_comments_with_options(f, comments, true)
}

/// Write trailing comments with configurable parent expansion for line comments.
fn write_trailing_comments_with_options<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
    expand_parent_for_line_comments: bool,
) -> FormatResult<()> {
    let source = f.context().source_text();
    let mut total_lines_before = 0usize;
    let mut previous_comment = None;

    for comment in comments.iter().copied() {
        f.context_mut().comments_mut().mark_comment_printed(comment);

        let lines_before = {
            let comment_cursor = f.context().comments();
            source.get_lines_before(comment.span, comment_cursor)
        };
        total_lines_before += lines_before;
        let should_nestle = previous_comment.is_some_and(|previous_comment| {
            should_nestle_adjacent_doc_comments(previous_comment, comment)
        });

        if total_lines_before > 0 || previous_comment.is_some_and(Comment::is_line) {
            write!(
                f,
                [line_suffix(&format_with(
                    move |f: &mut DestackFormatter<'ast, '_>| {
                        match lines_before {
                            _ if should_nestle => {}
                            0 => {
                                if previous_comment.is_some_and(Comment::is_line) {
                                    write!(f, [hard_line_break()])?;
                                } else {
                                    write!(f, [space()])?;
                                }
                            }
                            1 => {
                                write!(f, [hard_line_break()])?;
                            }
                            _ => {
                                write!(f, [empty_line()])?;
                            }
                        }

                        format_comment(f, comment)
                    }
                ))]
            )?;
        } else {
            let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
                if !should_nestle {
                    write!(f, [space()])?;
                }

                format_comment(f, comment)
            });

            if comment.is_line() {
                if expand_parent_for_line_comments {
                    write!(f, [line_suffix(&content), expand_parent()])?;
                } else {
                    write!(f, [line_suffix(&content)])?;
                }
            } else {
                write!(f, [content])?;
            }
        }

        previous_comment = Some(comment);
    }

    Ok(())
}

/// Write one comment slice with direct source-preserving separators.
pub(crate) fn write_comment_slice<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    comments: &[Comment],
) -> FormatResult<()> {
    let source = f.context().source_text();
    let mut previous_comment = None;

    for comment in comments.iter().copied() {
        let lines_before = {
            let comment_cursor = f.context().comments();
            source.get_lines_before(comment.span, comment_cursor)
        };
        let should_nestle = previous_comment.is_some_and(|previous_comment| {
            should_nestle_adjacent_doc_comments(previous_comment, comment)
        });

        match lines_before {
            0 if should_nestle => {}
            0 => {
                if previous_comment.is_some_and(Comment::is_line) {
                    write!(f, [hard_line_break()])?;
                } else {
                    write!(f, [space()])?;
                }
            }
            1 => {
                write!(f, [hard_line_break()])?;
            }
            _ => {
                write!(f, [empty_line()])?;
            }
        }

        format_comment(f, comment)?;
        previous_comment = Some(comment);
    }

    Ok(())
}

/// Return one dangling comment formatter for one enclosing span.
#[inline]
pub(crate) const fn format_dangling_comments<'a>(span: Span) -> FormatDanglingComments<'a> {
    FormatDanglingComments::Node {
        span,
        indent: DanglingIndentMode::None,
    }
}

/// Format dangling comments before one closing delimiter.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FormatDanglingComments<'a> {
    /// One enclosing span and indentation mode.
    Node {
        span: Span,
        indent: DanglingIndentMode,
    },
    /// One explicit dangling comment slice and indentation mode.
    Comments {
        comments: &'a [Comment],
        indent: DanglingIndentMode,
    },
}

/// The indentation mode for dangling comments.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum DanglingIndentMode {
    /// Indent dangling comments with one block indent.
    Block,
    /// Indent dangling comments with one soft block indent.
    Soft,
    /// Write dangling comments without extra indentation wrappers.
    None,
}

impl FormatDanglingComments<'_> {
    /// Indent dangling comments with one block indent.
    pub(crate) fn with_block_indent(self) -> Self {
        self.with_indent_mode(DanglingIndentMode::Block)
    }

    /// Indent dangling comments with one soft block indent.
    pub(crate) fn with_soft_block_indent(self) -> Self {
        self.with_indent_mode(DanglingIndentMode::Soft)
    }

    /// Set the indentation mode for one dangling comment formatter.
    fn with_indent_mode(mut self, indent: DanglingIndentMode) -> Self {
        match &mut self {
            Self::Node {
                indent: current_indent,
                ..
            }
            | Self::Comments {
                indent: current_indent,
                ..
            } => *current_indent = indent,
        }

        self
    }
}

impl<'a> Format<DestackFormatContext<'a>> for FormatDanglingComments<'_> {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'a>>) -> FormatResult<()> {
        fn write_dangling_comments<'ast>(
            f: &mut DestackFormatter<'ast, '_>,
            comments: &[Comment],
            indent: DanglingIndentMode,
        ) -> FormatResult<()> {
            let content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                let mut previous_comment = None;

                for comment in comments.iter().copied() {
                    if previous_comment.is_some() {
                        let should_nestle = previous_comment.is_some_and(|previous_comment| {
                            should_nestle_adjacent_doc_comments(previous_comment, comment)
                        });

                        if !should_nestle {
                            write!(f, [hard_line_break()])?;
                        }
                    }

                    format_comment(f, comment)?;
                    previous_comment = Some(comment);
                }

                if indent == DanglingIndentMode::Soft
                    && previous_comment.is_some_and(Comment::is_line)
                {
                    write!(f, [hard_line_break()])?;
                }

                Ok(())
            });

            match indent {
                DanglingIndentMode::Block => write!(f, [block_indent(&content)]),
                DanglingIndentMode::Soft => write!(f, [group(&soft_block_indent(&content))]),
                DanglingIndentMode::None => write!(f, [content]),
            }
        }

        match self {
            Self::Node { span, indent } => {
                let comments = f.context().comments().comments_before(span.end);

                if comments.is_empty() {
                    return Ok(());
                }

                write_dangling_comments(f, comments, *indent)
            }
            Self::Comments { comments, indent } => {
                if comments.is_empty() {
                    return Ok(());
                }

                write_dangling_comments(f, comments, *indent)
            }
        }
    }
}

/// Return one trailing comment formatter for one node relationship.
#[inline]
pub(crate) const fn format_trailing_comments(
    enclosing_span: Span,
    preceding_span: Span,
    following_span_start: u32,
) -> FormatTrailingComments<'static> {
    FormatTrailingComments::Node((enclosing_span, preceding_span, following_span_start))
}

/// Format one node with generated-node-style trailing comment boundaries.
pub(crate) fn format_node_with_trailing_comments<'ast, T>(
    enclosing_span: Span,
    node_id: LocalNodeId<T>,
    following_span_start: u32,
) -> impl Format<DestackFormatContext<'ast>> + use<'ast, T>
where
    T: FormatNode<'ast, T> + Node + Clone + 'ast,
    NodeTree: NodeTreeImpl<T>,
{
    format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        let node_span = f.context().span(node_id);

        write!(
            f,
            [
                FormatNodeWithoutTrailingComments(node_id),
                format_trailing_comments(enclosing_span, node_span, following_span_start)
            ]
        )
    })
}

/// Format trailing comments for one node relationship.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FormatTrailingComments<'a> {
    /// The enclosing span, preceding span, and following sibling start.
    Node((Span, Span, u32)),
    /// One explicit trailing comment slice.
    Comments(&'a [Comment]),
}

impl<'a> Format<DestackFormatContext<'a>> for FormatTrailingComments<'_> {
    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'a>>) -> FormatResult<()> {
        match self {
            Self::Node((enclosing_span, preceding_span, following_span_start)) => {
                let comments = f.context().comments().get_trailing_comments(
                    *enclosing_span,
                    *preceding_span,
                    *following_span_start,
                );

                if comments.is_empty() {
                    return Ok(());
                }

                write_trailing_comments(f, comments)
            }
            Self::Comments(comments) => {
                if comments.is_empty() {
                    return Ok(());
                }

                write_trailing_comments(f, comments)
            }
        }
    }
}

/// Return whether one multiline block comment is alignable on `*` prefixes.
fn block_comment_is_alignable(comment_source: &str) -> bool {
    comment_source
        .lines()
        .skip(1)
        .all(|line| line.trim_start_matches('\r').trim_start().starts_with('*'))
}
