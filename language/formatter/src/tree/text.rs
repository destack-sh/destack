use super::node::write_tree_child;
use super::whitespace::{
    is_tree_whitespace_char, tree_child_is_space_expression, tree_text_child_text,
    tree_text_is_whitespace_only,
};
use crate::{TsppFormatContext, TsppFormatter};
use tspp_dir::{LocalNodeId, TreeChild};
use tspp_fir::format::FormatResult;
use tspp_fir::prelude::{
    copied_text, empty_line, format_with, hard_line_break, if_group_breaks, if_group_fits_on_line,
    soft_line_break, soft_line_break_or_space, space, text, token,
};
use tspp_fir::{format_args, write};
use tspp_repository::QuoteStyle;

/// One tree text chunk.
#[derive(Clone, Copy, Debug)]
enum TreeTextChunk<'a> {
    /// A whitespace run.
    Whitespace(&'a str),
    /// A non-whitespace word.
    Word(&'a str),
}

/// One embedded tree child in an inline content stream.
#[derive(Debug)]
struct TreeInlineEmbedded {
    /// The embedded tree child.
    child_id: LocalNodeId<TreeChild>,
    /// Whether trailing inline punctuation can attach to this child.
    allows_trailing_punctuation: bool,
    /// Inline punctuation that belongs to the embedded child.
    trailing_punctuation: Option<String>,
}

/// One item in a mixed tree child inline content stream.
#[derive(Debug)]
enum TreeInlineItem {
    /// One text word.
    Word(String),
    /// One punctuation run.
    Punctuation(String),
    /// One tree whitespace separator.
    Whitespace,
    /// One source newline separator.
    Newline,
    /// One source empty-line separator.
    EmptyLine,
    /// One embedded non-text child.
    Embedded(TreeInlineEmbedded),
}

/// Separator before one inline tree item in fill layout.
#[derive(Clone, Copy, Debug)]
enum TreeInlineSeparator {
    /// No separator.
    None,
    /// Tree whitespace.
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

/// Split one text child into tree text chunks.
fn tree_text_chunks(text: &str) -> Vec<TreeTextChunk<'_>> {
    let mut chunks = Vec::new();
    let mut chunk_start = 0usize;
    let mut chunk_is_whitespace = None::<bool>;

    for (index, character) in text.char_indices() {
        let is_whitespace = is_tree_whitespace_char(character);
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
    f: &mut TsppFormatter<'ast, '_>,
    source: &'ast str,
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
    context: &TsppFormatContext<'_>,
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

/// Return whether one word is inline punctuation.
fn tree_text_word_is_inline_punctuation(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|character| matches!(character, '.' | ',' | ':' | ';' | '!' | '?'))
}

/// Return whether one text string is only inline punctuation.
pub(crate) fn tree_text_is_inline_punctuation(text: &str) -> bool {
    let chunks = tree_text_chunks(text);

    match chunks.as_slice() {
        [TreeTextChunk::Word(word)] => tree_text_word_is_inline_punctuation(word),
        _ => false,
    }
}

/// Return whether one tree child can own following inline punctuation.
pub(crate) fn tree_child_allows_trailing_inline_punctuation(
    context: &TsppFormatContext<'_>,
    child_id: LocalNodeId<TreeChild>,
) -> bool {
    match context.tree.get(child_id) {
        TreeChild::Expression { .. } => true,
        TreeChild::Tree { .. } => true,
        TreeChild::Text { .. } | TreeChild::Empty | TreeChild::Spread { .. } | TreeChild::Error => {
            false
        }
    }
}

/// Push one inline item with whitespace coalescing and punctuation attachment rules.
fn push_tree_inline_item(items: &mut Vec<TreeInlineItem>, item: TreeInlineItem) {
    match item {
        // coalesce whitespace layout separators
        TreeInlineItem::Whitespace => match items.last_mut() {
            Some(
                last @ (TreeInlineItem::EmptyLine
                | TreeInlineItem::Newline
                | TreeInlineItem::Whitespace),
            ) => {
                *last = TreeInlineItem::Whitespace;
            }
            _ => items.push(TreeInlineItem::Whitespace),
        },

        // attach punctuation to the previous inline box
        TreeInlineItem::Punctuation(punctuation) => match items.last_mut() {
            Some(TreeInlineItem::Word(word)) => {
                word.push_str(&punctuation);
            }
            Some(TreeInlineItem::Embedded(embedded)) if embedded.allows_trailing_punctuation => {
                if let Some(trailing_punctuation) = &mut embedded.trailing_punctuation {
                    trailing_punctuation.push_str(&punctuation);
                } else {
                    embedded.trailing_punctuation = Some(punctuation);
                }
            }
            _ => items.push(TreeInlineItem::Punctuation(punctuation)),
        },

        TreeInlineItem::Word(_)
        | TreeInlineItem::Embedded(_)
        | TreeInlineItem::Newline
        | TreeInlineItem::EmptyLine => items.push(item),
    }
}

/// Push one text word with tree inline text rules.
fn push_tree_inline_word(items: &mut Vec<TreeInlineItem>, word: &str) {
    let item = if tree_text_word_is_inline_punctuation(word) {
        TreeInlineItem::Punctuation(word.to_string())
    } else {
        TreeInlineItem::Word(word.to_string())
    };

    push_tree_inline_item(items, item);
}

/// Split tree children into an inline content stream.
fn tree_inline_items(
    context: &TsppFormatContext<'_>,
    children: &[LocalNodeId<TreeChild>],
) -> Vec<TreeInlineItem> {
    let mut items = Vec::new();

    for child_id in children {
        if tree_child_is_space_expression(context, *child_id) {
            push_tree_inline_item(&mut items, TreeInlineItem::Whitespace);
            continue;
        }

        let Some(text) = tree_text_child_text(context, *child_id) else {
            push_tree_inline_item(
                &mut items,
                TreeInlineItem::Embedded(TreeInlineEmbedded {
                    child_id: *child_id,
                    allows_trailing_punctuation: tree_child_allows_trailing_inline_punctuation(
                        context, *child_id,
                    ),
                    trailing_punctuation: None,
                }),
            );
            continue;
        };

        let mut chunks = tree_text_chunks(text).into_iter().peekable();
        if let Some(TreeTextChunk::Whitespace(whitespace)) = chunks.peek().copied() {
            chunks.next();

            if whitespace.contains('\n') {
                if chunks.peek().is_none() {
                    let newline_count = whitespace.bytes().filter(|byte| *byte == b'\n').count();
                    if newline_count > 1 {
                        push_tree_inline_item(&mut items, TreeInlineItem::EmptyLine);
                    }

                    continue;
                }

                push_tree_inline_item(&mut items, TreeInlineItem::Newline);
            } else {
                push_tree_inline_item(&mut items, TreeInlineItem::Whitespace);
            }
        }

        while let Some(chunk) = chunks.next() {
            match chunk {
                TreeTextChunk::Word(word) => {
                    push_tree_inline_word(&mut items, word);
                }
                TreeTextChunk::Whitespace(whitespace) => {
                    if chunks.peek().is_none() {
                        if whitespace.contains('\n') {
                            push_tree_inline_item(&mut items, TreeInlineItem::Newline);
                        } else {
                            push_tree_inline_item(&mut items, TreeInlineItem::Whitespace);
                        }
                    }
                }
            }
        }
    }

    if matches!(
        items.last(),
        Some(TreeInlineItem::EmptyLine | TreeInlineItem::Newline)
    ) {
        items.pop();
    }

    if matches!(
        items.first(),
        Some(TreeInlineItem::EmptyLine | TreeInlineItem::Newline)
    ) {
        items.remove(0);
    }

    items
}

/// Return the separator before one visible inline item.
fn tree_inline_separator(
    previous_visible: Option<&TreeInlineItem>,
    pending_separator: TreeInlineSeparator,
    current: &TreeInlineItem,
) -> TreeInlineSeparator {
    if !matches!(pending_separator, TreeInlineSeparator::None) {
        return pending_separator;
    }

    match (previous_visible, current) {
        (Some(TreeInlineItem::Word(_)), TreeInlineItem::Word(_))
        | (
            Some(TreeInlineItem::Embedded(TreeInlineEmbedded {
                trailing_punctuation: Some(_),
                ..
            })),
            TreeInlineItem::Word(_),
        )
        | (Some(TreeInlineItem::Punctuation(_)), TreeInlineItem::Word(_)) => {
            TreeInlineSeparator::SoftOrSpace
        }
        (Some(TreeInlineItem::Word(_)), TreeInlineItem::Embedded(_))
        | (Some(TreeInlineItem::Embedded(_)), TreeInlineItem::Word(_)) => TreeInlineSeparator::Soft,
        (Some(TreeInlineItem::Embedded(_)), TreeInlineItem::Embedded(_)) => {
            TreeInlineSeparator::Hard
        }
        (
            Some(TreeInlineItem::Embedded(TreeInlineEmbedded {
                allows_trailing_punctuation: false,
                ..
            })),
            TreeInlineItem::Punctuation(_),
        ) => TreeInlineSeparator::Hard,
        (_, TreeInlineItem::Punctuation(_)) => TreeInlineSeparator::None,
        _ => TreeInlineSeparator::None,
    }
}

/// Format one inline item separator.
fn write_tree_inline_separator<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    separator: TreeInlineSeparator,
) -> FormatResult<()> {
    match separator {
        TreeInlineSeparator::None => {}
        TreeInlineSeparator::Whitespace => write_tree_whitespace_separator(f)?,
        TreeInlineSeparator::SoftOrSpace => write!(f, [soft_line_break_or_space()])?,
        TreeInlineSeparator::Soft => write!(f, [soft_line_break()])?,
        TreeInlineSeparator::Hard => write!(f, [hard_line_break()])?,
        TreeInlineSeparator::Empty => write!(f, [empty_line()])?,
    }

    Ok(())
}

/// Return one tree space token that matches the configured quote style.
fn tree_space_token(context: &TsppFormatContext<'_>) -> &'static str {
    match context.options.quote_style {
        QuoteStyle::Single => "{' '}",
        QuoteStyle::Double | QuoteStyle::Semantic => "{\" \"}",
    }
}

/// Emit one tree whitespace separator that stays inline in flat mode.
fn write_tree_whitespace_separator<'ast>(f: &mut TsppFormatter<'ast, '_>) -> FormatResult<()> {
    let tree_space = token(tree_space_token(f.context()));
    write!(
        f,
        [
            if_group_breaks(&format_args![tree_space, soft_line_break()]),
            if_group_fits_on_line(&space())
        ]
    )
}

/// Write one embedded inline tree item.
fn write_tree_inline_embedded<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    embedded: &TreeInlineEmbedded,
) -> FormatResult<()> {
    write_tree_child(f, embedded.child_id, None)?;
    if let Some(punctuation) = embedded.trailing_punctuation.as_deref() {
        write!(f, [copied_text(punctuation)])?;
    }

    Ok(())
}

/// Format mixed tree children with inline fill layout.
pub(crate) fn format_tree_children_inline_fill<'ast>(
    f: &mut TsppFormatter<'ast, '_>,
    children: &[LocalNodeId<TreeChild>],
    force_multiline: bool,
) -> FormatResult<()> {
    let inline_items = tree_inline_items(f.context(), children);
    if inline_items.is_empty() {
        let (has_newline_spacing, has_inline_spacing) =
            tree_whitespace_run_spacing(f.context(), children);
        if has_newline_spacing {
            write!(f, [hard_line_break()])?;
        } else if has_inline_spacing {
            write_tree_whitespace_separator(f)?;
        }

        return Ok(());
    }

    let mut fill = f.fill();
    let mut previous_visible = None::<&TreeInlineItem>;
    let mut pending_separator = TreeInlineSeparator::None;

    for item in &inline_items {
        match item {
            TreeInlineItem::Whitespace => {
                if force_multiline {
                    pending_separator = TreeInlineSeparator::Whitespace;
                    continue;
                }

                let separator = tree_inline_separator(previous_visible, pending_separator, item);
                let separator = format_with(|f| write_tree_inline_separator(f, separator));
                let whitespace = format_with(write_tree_whitespace_separator);
                fill.entry(&separator, &whitespace);
                previous_visible = Some(item);
                pending_separator = TreeInlineSeparator::None;
            }
            TreeInlineItem::Newline => {
                // preserve visible whitespace before layout newlines
                if !matches!(pending_separator, TreeInlineSeparator::Whitespace) {
                    pending_separator = TreeInlineSeparator::Hard;
                }
            }
            TreeInlineItem::EmptyLine => {
                pending_separator = TreeInlineSeparator::Empty;
            }
            TreeInlineItem::Word(word) | TreeInlineItem::Punctuation(word) => {
                let separator = tree_inline_separator(previous_visible, pending_separator, item);
                let separator = format_with(|f| write_tree_inline_separator(f, separator));
                fill.entry(&separator, &copied_text(word.as_str()));
                previous_visible = Some(item);
                pending_separator = TreeInlineSeparator::None;
            }
            TreeInlineItem::Embedded(embedded) => {
                let separator = tree_inline_separator(previous_visible, pending_separator, item);
                let separator = format_with(|f| write_tree_inline_separator(f, separator));
                fill.entry(
                    &separator,
                    &format_with(|f| write_tree_inline_embedded(f, embedded)),
                );
                previous_visible = Some(item);
                pending_separator = TreeInlineSeparator::None;
            }
        }
    }

    fill.finish()
}
