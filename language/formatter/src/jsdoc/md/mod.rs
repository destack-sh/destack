mod collect;
mod detect;
mod nodes;
mod preprocess;

use std::borrow::Cow;

use markdown::{Constructs, ParseOptions, to_mdast};

use crate::DestackFormatOptions;
use destack_workspace::JsdocLineWrappingStyle;

use super::line::LineBuffer;
use super::wrap::{wrap_plain_paragraphs, wrap_plain_paragraphs_balance};

use detect::needs_markdown_parsing;
use nodes::serialize_children;
use preprocess::{
    convert_star_list_markers, escape_false_list_markers, normalize_legacy_ordered_list_markers,
    protect_jsdoc_links, restore_in_string,
};

/// Format a markdown description.
///
/// Parses the text into a markdown tree, then serializes it back to formatted
/// text with proper indentation, wrapping, and emphasis normalization.
/// This keeps structured markdown separate from plain paragraph wrapping.
pub(super) fn format_description_markdown(
    text: &str,
    max_width: usize,
    tag_string_length: usize,
    capitalize: bool,
    format_options: Option<&DestackFormatOptions>,
) -> String {
    if text.trim().is_empty() {
        return String::new();
    }

    let jsdoc_opts = format_options.and_then(|opts| opts.jsdoc.as_ref());
    let description_with_dot = jsdoc_opts.is_some_and(|o| o.description_with_dot);
    let prefer_code_fences = jsdoc_opts.is_some_and(|o| o.prefer_code_fences);
    let line_wrapping_style =
        jsdoc_opts.map_or(JsdocLineWrappingStyle::default(), |o| o.line_wrapping_style);

    // use the light paragraph wrapper when no markdown tree is needed
    if tag_string_length == 0 && !needs_markdown_parsing(text) {
        let result = if matches!(line_wrapping_style, JsdocLineWrappingStyle::Balance) {
            wrap_plain_paragraphs_balance(text, max_width)
        } else {
            wrap_plain_paragraphs(text, max_width)
        };
        if !capitalize && !description_with_dot {
            return result;
        }
        // capitalize the first word of each paragraph
        let mut out = String::with_capacity(result.len() + 1);
        let mut iter = result.split('\n').peekable();
        let mut at_paragraph_start = true;
        let mut first = true;
        while let Some(line) = iter.next() {
            if !first {
                out.push('\n');
            }
            first = false;
            // paragraph ends at the final line or before a blank line
            let is_last_in_para = iter.peek().is_none_or(|next| next.is_empty());
            if line.is_empty() {
                at_paragraph_start = true;
            } else if at_paragraph_start {
                let line = if capitalize {
                    super::normalize::capitalize_first(line)
                } else {
                    Cow::Borrowed(line)
                };
                if description_with_dot && is_last_in_para {
                    out.push_str(&super::normalize::append_trailing_dot(&line));
                } else {
                    out.push_str(&line);
                }
                at_paragraph_start = false;
                continue;
            } else if description_with_dot && is_last_in_para {
                out.push_str(&super::normalize::append_trailing_dot(line));
                continue;
            }
            out.push_str(line);
        }
        return out;
    }

    let text = normalize_legacy_ordered_list_markers(text);
    let text = convert_star_list_markers(&text);
    let text = escape_false_list_markers(&text);

    // protect inline tags before markdown parsing
    let (protected, placeholders) = protect_jsdoc_links(&text);

    // parse with tables left to the serializer
    let parse_opts = ParseOptions {
        constructs: Constructs {
            gfm_autolink_literal: false,
            gfm_footnote_definition: true,
            gfm_label_start_footnote: true,
            gfm_strikethrough: true,
            gfm_table: false,
            gfm_task_list_item: true,
            ..Constructs::default()
        },
        ..ParseOptions::default()
    };
    let Ok(root) = to_mdast(&protected, &parse_opts) else {
        // preserve text when parsing fails
        let mut out = String::new();
        for (i, line) in text.lines().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(line.trim());
        }
        return out;
    };

    let mut lines = LineBuffer::new();
    let opts = SerializeOptions {
        max_width,
        tag_string_length,
        capitalize,
        description_with_dot,
        prefer_code_fences,
        line_wrapping_style,
        source: &protected,
        format_options,
    };
    serialize_children(&root, 0, opts.tag_string_length, &opts, &mut lines);

    let output = lines.into_string();
    restore_in_string(&output, &placeholders).into_owned()
}

pub(super) struct SerializeOptions<'a> {
    pub(super) max_width: usize,
    pub(super) tag_string_length: usize,
    pub(super) capitalize: bool,
    pub(super) description_with_dot: bool,
    pub(super) prefer_code_fences: bool,
    pub(super) line_wrapping_style: JsdocLineWrappingStyle,
    pub(super) source: &'a str,
    pub(super) format_options: Option<&'a DestackFormatOptions>,
}
