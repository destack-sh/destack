use super::node::write_tree_child;
use super::whitespace::{
    is_jsx_whitespace_char, tree_child_is_jsx_space_expression, tree_text_child_text,
    tree_text_is_whitespace_only,
};
use crate::{DestackFormatContext, DestackFormatter};
use destack_dir::{LocalNodeId, TreeChild};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::prelude::{
    empty_line, format_with, hard_line_break, if_group_breaks, if_group_fits_on_line,
    soft_line_break, soft_line_break_or_space, space, text, token,
};
use destack_fir::{format_args, write};
use destack_repository::QuoteStyle;

/// One tree text chunk.
#[derive(Clone, Copy, Debug)]
enum TreeTextChunk<'a> {
    /// A whitespace run.
    Whitespace(&'a str),
    /// A non-whitespace word.
    Word(&'a str),
}

/// One split tree child used by the JSX child-list fill layout.
#[derive(Clone, Debug)]
enum TreeSplitChild {
    /// One text word.
    Word(String),
    /// One JSX whitespace separator.
    Whitespace,
    /// One source newline separator.
    Newline,
    /// One source empty-line separator.
    EmptyLine,
    /// One non-text child.
    NonText(LocalNodeId<TreeChild>),
}

/// Separator before one split tree child in fill layout.
#[derive(Clone, Copy, Debug)]
enum TreeSplitSeparator {
    /// No separator.
    None,
    /// JSX whitespace.
    Whitespace,
    /// Soft word separator.
    SoftOrSpace,
    /// Soft child separator.
    Soft,
    /// Hard line break.
    Hard,
    /// Empty line.
    Empty,
}

/// Split one text child into JSX text chunks.
fn tree_text_chunks(text: &str) -> Vec<TreeTextChunk<'_>> {
    let mut chunks = Vec::new();
    let mut chunk_start = 0usize;
    let mut chunk_is_whitespace = None::<bool>;

    for (index, character) in text.char_indices() {
        let is_whitespace = is_jsx_whitespace_char(character);
        let Some(previous_is_whitespace) = chunk_is_whitespace else {
            chunk_is_whitespace = Some(is_whitespace);
            continue;
        };

        if previous_is_whitespace == is_whitespace {
            continue;
        }

        let chunk = &text[chunk_start..index];
        if previous_is_whitespace {
            chunks.push(TreeTextChunk::Whitespace(chunk));
        } else {
            chunks.push(TreeTextChunk::Word(chunk));
        }

        chunk_start = index;
        chunk_is_whitespace = Some(is_whitespace);
    }

    let Some(is_whitespace) = chunk_is_whitespace else {
        return chunks;
    };

    let chunk = &text[chunk_start..];
    if is_whitespace {
        chunks.push(TreeTextChunk::Whitespace(chunk));
    } else {
        chunks.push(TreeTextChunk::Word(chunk));
    }

    chunks
}

/// Split one tree text string into printable words.
fn tree_text_words(text: &str) -> Vec<&str> {
    tree_text_chunks(text)
        .into_iter()
        .filter_map(|chunk| match chunk {
            TreeTextChunk::Word(word) => Some(word),
            TreeTextChunk::Whitespace(_) => None,
        })
        .collect()
}

/// Write tree text words in one child-line position.
pub(crate) fn write_tree_text_words<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    source: &str,
) -> FormatResult<bool> {
    let words = tree_text_words(source);
    if words.is_empty() {
        return Ok(false);
    }

    let mut wrote_word = false;
    for word in words {
        if wrote_word {
            write!(f, [space()])?;
        }

        write!(f, [text(word)])?;
        wrote_word = true;
    }

    Ok(true)
}

/// Return whether one whitespace-only child run contains inline or newline spacing.
fn tree_whitespace_run_spacing(
    context: &DestackFormatContext<'_>,
    children: &[LocalNodeId<TreeChild>],
) -> (bool, bool) {
    let mut has_inline_whitespace = false;
    let mut has_newline_whitespace = false;

    for child_id in children {
        let Some((is_whitespace_only, has_newline)) =
            tree_text_is_whitespace_only(context, *child_id)
        else {
            continue;
        };
        if !is_whitespace_only {
            continue;
        }

        if has_newline {
            has_newline_whitespace = true;
        } else {
            has_inline_whitespace = true;
        }
    }

    if has_newline_whitespace {
        return (true, false);
    }

    if has_inline_whitespace {
        return (false, true);
    }

    (false, false)
}

/// Push one split JSX child with whitespace coalescing rules.
fn push_tree_split_child(children: &mut Vec<TreeSplitChild>, child: TreeSplitChild) {
    match children.last_mut() {
        Some(
            last @ (TreeSplitChild::EmptyLine
            | TreeSplitChild::Newline
            | TreeSplitChild::Whitespace),
        ) => {
            if matches!(child, TreeSplitChild::Whitespace) {
                *last = child;
            } else if matches!(child, TreeSplitChild::NonText(_) | TreeSplitChild::Word(_)) {
                children.push(child);
            }
        }
        _ => children.push(child),
    }
}

/// Split tree children with JSX child-list rules.
fn split_tree_children(
    context: &DestackFormatContext<'_>,
    children: &[LocalNodeId<TreeChild>],
) -> Vec<TreeSplitChild> {
    let mut split_children = Vec::new();

    for child_id in children {
        if tree_child_is_jsx_space_expression(context, *child_id) {
            push_tree_split_child(&mut split_children, TreeSplitChild::Whitespace);
            continue;
        }

        let Some(text) = tree_text_child_text(context, *child_id) else {
            push_tree_split_child(&mut split_children, TreeSplitChild::NonText(*child_id));
            continue;
        };

        let mut chunks = tree_text_chunks(text).into_iter().peekable();
        if let Some(TreeTextChunk::Whitespace(_)) = chunks.peek() {
            let Some(TreeTextChunk::Whitespace(whitespace)) = chunks.next() else {
                unreachable!("peeked whitespace chunk should be whitespace");
            };

            if whitespace.contains('\n') {
                if chunks.peek().is_none() {
                    let newline_count = whitespace.bytes().filter(|byte| *byte == b'\n').count();
                    if newline_count > 1 {
                        push_tree_split_child(&mut split_children, TreeSplitChild::EmptyLine);
                    }

                    continue;
                }

                push_tree_split_child(&mut split_children, TreeSplitChild::Newline);
            } else {
                push_tree_split_child(&mut split_children, TreeSplitChild::Whitespace);
            }
        }

        while let Some(chunk) = chunks.next() {
            match chunk {
                TreeTextChunk::Word(word) => {
                    push_tree_split_child(
                        &mut split_children,
                        TreeSplitChild::Word(word.to_string()),
                    );
                }
                TreeTextChunk::Whitespace(whitespace) => {
                    if chunks.peek().is_none() {
                        if whitespace.contains('\n') {
                            push_tree_split_child(&mut split_children, TreeSplitChild::Newline);
                        } else {
                            push_tree_split_child(&mut split_children, TreeSplitChild::Whitespace);
                        }
                    }
                }
            }
        }
    }

    if matches!(
        split_children.last(),
        Some(TreeSplitChild::EmptyLine | TreeSplitChild::Newline)
    ) {
        split_children.pop();
    }

    if matches!(
        split_children.first(),
        Some(TreeSplitChild::EmptyLine | TreeSplitChild::Newline)
    ) {
        split_children.remove(0);
    }

    split_children
}

/// Return the separator before one visible split child.
fn tree_split_separator(
    previous_visible: Option<&TreeSplitChild>,
    pending_separator: TreeSplitSeparator,
    current: &TreeSplitChild,
) -> TreeSplitSeparator {
    if !matches!(pending_separator, TreeSplitSeparator::None) {
        return pending_separator;
    }

    match (previous_visible, current) {
        (Some(TreeSplitChild::Word(_)), TreeSplitChild::Word(_)) => TreeSplitSeparator::SoftOrSpace,
        (Some(TreeSplitChild::Word(_)), TreeSplitChild::NonText(_))
        | (Some(TreeSplitChild::NonText(_)), TreeSplitChild::Word(_)) => TreeSplitSeparator::Soft,
        (Some(TreeSplitChild::NonText(_)), TreeSplitChild::NonText(_)) => TreeSplitSeparator::Hard,
        _ => TreeSplitSeparator::None,
    }
}

/// Format one split child separator.
fn write_tree_split_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    separator: TreeSplitSeparator,
) -> FormatResult<()> {
    match separator {
        TreeSplitSeparator::None => {}
        TreeSplitSeparator::Whitespace => write_tree_jsx_whitespace_separator(f)?,
        TreeSplitSeparator::SoftOrSpace => write!(f, [soft_line_break_or_space()])?,
        TreeSplitSeparator::Soft => write!(f, [soft_line_break()])?,
        TreeSplitSeparator::Hard => write!(f, [hard_line_break()])?,
        TreeSplitSeparator::Empty => write!(f, [empty_line()])?,
    }

    Ok(())
}

/// Return one JSX space token that matches the configured quote style.
fn tree_jsx_space_token(context: &DestackFormatContext<'_>) -> &'static str {
    match context.options.quote_style {
        QuoteStyle::Single => "{' '}",
        QuoteStyle::Double | QuoteStyle::Semantic => "{\" \"}",
    }
}

/// Emit one JSX whitespace separator that stays inline in flat mode.
fn write_tree_jsx_whitespace_separator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    let jsx_space = token(tree_jsx_space_token(f.context()));
    write!(
        f,
        [
            if_group_breaks(&format_args![jsx_space, soft_line_break()]),
            if_group_fits_on_line(&space())
        ]
    )
}

/// Format mixed tree children with fill separators.
pub(crate) fn format_tree_children_fill<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    children: &[LocalNodeId<TreeChild>],
    force_multiline: bool,
) -> FormatResult<()> {
    let split_children = split_tree_children(f.context(), children);
    if split_children.is_empty() {
        let (has_newline_spacing, has_inline_spacing) =
            tree_whitespace_run_spacing(f.context(), children);
        if has_newline_spacing {
            write!(f, [hard_line_break()])?;
        } else if has_inline_spacing {
            write_tree_jsx_whitespace_separator(f)?;
        }

        return Ok(());
    }

    let mut fill = f.fill();
    let mut previous_visible = None::<TreeSplitChild>;
    let mut pending_separator = TreeSplitSeparator::None;

    for child in &split_children {
        match child {
            TreeSplitChild::Whitespace => {
                if force_multiline {
                    pending_separator = TreeSplitSeparator::Whitespace;
                    continue;
                }

                let separator =
                    tree_split_separator(previous_visible.as_ref(), pending_separator, child);
                let separator = format_with(|f| write_tree_split_separator(f, separator));
                let whitespace = format_with(write_tree_jsx_whitespace_separator);
                fill.entry(&separator, &whitespace);
                previous_visible = Some(child.clone());
                pending_separator = TreeSplitSeparator::None;
            }
            TreeSplitChild::Newline => {
                pending_separator = TreeSplitSeparator::Hard;
            }
            TreeSplitChild::EmptyLine => {
                pending_separator = TreeSplitSeparator::Empty;
            }
            TreeSplitChild::Word(word) => {
                let separator =
                    tree_split_separator(previous_visible.as_ref(), pending_separator, child);
                let separator = format_with(|f| write_tree_split_separator(f, separator));
                fill.entry(&separator, &text(word.as_str()));
                previous_visible = Some(child.clone());
                pending_separator = TreeSplitSeparator::None;
            }
            TreeSplitChild::NonText(child_id) => {
                let separator =
                    tree_split_separator(previous_visible.as_ref(), pending_separator, child);
                let separator = format_with(|f| write_tree_split_separator(f, separator));
                fill.entry(
                    &separator,
                    &format_with(|f| write_tree_child(f, *child_id, None)),
                );
                previous_visible = Some(child.clone());
                pending_separator = TreeSplitSeparator::None;
            }
        }
    }

    fill.finish()
}
