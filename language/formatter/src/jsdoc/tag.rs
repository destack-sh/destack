use std::borrow::Cow;

use super::embedded::{
    fenced_code_file_type, format_embedded_code, format_embedded_code_as, update_template_depth,
};
use super::normalize::{capitalize_first, normalize_markdown_emphasis};
use super::parse::JsdocTag;
use super::serialize::{
    JsdocFormatter, format_default_value, is_known_tag, is_named_generic_tag,
    should_preserve_description_verbatim, should_skip_description_formatting,
    strip_default_is_suffix,
};
use super::wrap::{str_width, wrap_text};

/// Strip the minimum common leading whitespace from all non-empty lines.
/// The base indent is the indent of context not included in `text`.
fn dedent_lines(text: &str, base_indent: Option<usize>) -> String {
    // find source indentation from non-empty lines
    let min_indent = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // clamp by context indentation when the tag line has inline text
    let min_indent = base_indent.map_or(min_indent, |base_indent| min_indent.min(base_indent));

    // copy lines with their shared indentation removed
    let mut result = String::with_capacity(text.len());
    for (i, line) in text.lines().enumerate() {
        // restore separators between copied lines
        if i > 0 {
            result.push('\n');
        }

        // blank continuation lines stay empty
        if line.trim().is_empty() {
            continue;
        }

        // strip the computed shared indentation
        if min_indent > 0 && line.len() >= min_indent {
            result.push_str(&line[min_indent..]);
        }
        // preserve original whitespace when there is no common indent
        else if min_indent == 0 {
            result.push_str(line);
        }
        // fall back to trimming malformed short lines
        else {
            result.push_str(line.trim_start());
        }
    }

    result
}

impl JsdocFormatter<'_> {
    /// Normalize leading tabs in multiline tag types to the configured indent.
    fn normalize_multiline_type_indent<'a>(&self, raw_type: &'a str) -> Cow<'a, str> {
        if self.format_options.indent_style.is_tab()
            || !raw_type.lines().skip(1).any(|line| line.starts_with('\t'))
        {
            return Cow::Borrowed(raw_type);
        }

        let indent = self.code_indent();
        let mut result = String::with_capacity(raw_type.len());

        for (line_index, line) in raw_type.lines().enumerate() {
            if line_index > 0 {
                result.push('\n');
            }

            let tab_count = line.bytes().take_while(|byte| *byte == b'\t').count();

            for _ in 0..tab_count {
                result.push_str(indent);
            }

            result.push_str(&line[tab_count..]);
        }

        Cow::Owned(result)
    }

    pub(super) fn format_example_tag(&mut self, normalized_kind: &str, tag: &JsdocTag<'_>) {
        let description = tag.comment();
        let raw_text = description.parsed_preserving_whitespace();
        let trimmed = raw_text.trim();
        // keep leading captions inline with @example
        if let Some(rest) = trimmed.strip_prefix("<caption>")
            && let Some(end_pos) = rest.find("</caption>")
        {
            let caption = &rest[..end_pos];
            let after_caption = rest[end_pos + "</caption>".len()..].trim();
            {
                let s = self.content_lines.begin_line();
                s.push('@');
                s.push_str(normalized_kind);
                s.push_str(" <caption>");
                s.push_str(caption);
                s.push_str("</caption>");
            }
            self.format_example_code(after_caption);
            return;
        }

        {
            let s = self.content_lines.begin_line();
            s.push('@');
            s.push_str(normalized_kind);
        }
        self.format_example_code(trimmed);
    }

    /// Push formatted code lines with continuation indent, tracking template literal depth
    /// so that content inside template literals is not re-indented.
    fn push_formatted_code_lines(&mut self, code: &str, indent: &str) {
        let mut template_depth: u32 = 0;
        for line in code.lines() {
            if line.is_empty() {
                self.content_lines.push_empty();
            } else if template_depth == 0 {
                let s = self.content_lines.begin_line();
                s.push_str(indent);
                s.push_str(line);
            } else {
                self.content_lines.push(line);
            }
            template_depth = update_template_depth(line, template_depth);
        }
    }

    /// Push raw code lines with continuation indent, optionally preserving original indentation.
    fn push_raw_code_lines(&mut self, code: &str, indent: &str) {
        for line in code.lines() {
            let content = if self.options.keep_unparsable_example_indent {
                line
            } else {
                line.trim()
            };
            if content.is_empty() {
                self.content_lines.push_empty();
            } else {
                let s = self.content_lines.begin_line();
                s.push_str(indent);
                s.push_str(content);
            }
        }
    }

    /// Format example code content with continuation indent.
    fn format_example_code(&mut self, code: &str) {
        if code.is_empty() {
            return;
        }

        // fence markers must be handled before embedded code parsing
        if let Some((first_line, rest)) = code.split_once('\n') {
            let first_line = first_line.trim_start();
            if first_line.starts_with("```") {
                if let Some(closing_fence) = rest
                    .lines()
                    .next_back()
                    .filter(|line| line.trim_start().starts_with("```"))
                {
                    let closing_pos = rest.rfind(closing_fence).unwrap_or(rest.len());
                    let inner_code = rest[..closing_pos].trim_end_matches('\n');
                    let closing_fence = closing_fence.trim();

                    self.format_example_fenced_block(first_line, inner_code, closing_fence);
                    return;
                } else if rest.trim() == "```" {
                    // empty fenced block
                    self.format_example_fenced_block(first_line, "", rest.trim());
                    return;
                }
            }
        }

        let indent = self.code_indent();

        // use the remaining width after code indentation
        let effective_width = self.wrap_width.saturating_sub(self.code_indent_width());
        if let Some(formatted) = format_embedded_code(code, effective_width, self.format_options) {
            self.push_formatted_code_lines(&formatted, indent);
            return;
        }

        self.push_raw_code_lines(code, indent);
    }

    /// Handle fenced code blocks inside @example tags.
    /// Strips the ``` markers, formats the inner code, and re-adds fences
    /// with proper continuation indentation.
    fn format_example_fenced_block(
        &mut self,
        lang_line: &str,
        inner_code: &str,
        closing_fence: &str,
    ) {
        let indent = self.code_indent();
        let effective_width = self.wrap_width.saturating_sub(self.code_indent_width());

        // opening fence
        {
            let s = self.content_lines.begin_line();
            s.push_str(indent);
            s.push_str(lang_line);
        }

        if !inner_code.is_empty() {
            let lang = lang_line[3..].trim();
            if let Some(file_type) = fenced_code_file_type(lang) {
                if let Some(formatted) = format_embedded_code_as(
                    inner_code,
                    effective_width,
                    self.format_options,
                    file_type,
                ) {
                    self.push_formatted_code_lines(&formatted, indent);
                } else {
                    self.push_raw_code_lines(inner_code, indent);
                }
            } else {
                self.push_raw_code_lines(inner_code, indent);
            }
        }

        // closing fence
        {
            let s = self.content_lines.begin_line();
            s.push_str(indent);
            s.push_str(closing_fence);
        }
    }

    pub(super) fn format_type_name_comment_tag(
        &mut self,
        normalized_kind: &str,
        tag: &JsdocTag<'_>,
        should_capitalize: bool,
        has_no_space_before_type: bool,
    ) {
        let (tag_type, tag_name, description) = tag.type_name_comment();

        let tag_prefix_len = 1 + normalized_kind.len();
        let mut tag_line = String::with_capacity(tag_prefix_len + 32);
        tag_line.push('@');
        tag_line.push_str(normalized_kind);

        if let Some(tag_type) = &tag_type {
            let raw_type = self.normalize_multiline_type_indent(tag_type.raw());
            let raw_type = raw_type.trim();
            if !raw_type.is_empty() {
                if !has_no_space_before_type {
                    tag_line.push(' ');
                }
                tag_line.push_str(raw_type);
            }
        }

        // name and default value
        let mut name_str = Cow::Borrowed("");
        let mut default_value: Option<&str> = None;
        if let Some(tag_name) = &tag_name {
            let name_raw = tag_name.raw();
            if name_raw.starts_with('[') && name_raw.ends_with(']') {
                if let Some(eq_pos) = name_raw.find('=') {
                    let name_inner = &name_raw[1..eq_pos];
                    let val = name_raw[eq_pos + 1..name_raw.len() - 1].trim();
                    if val.is_empty() {
                        name_str = Cow::Owned(format!("[{name_inner}]"));
                    } else {
                        default_value = Some(val);
                        name_str = Cow::Owned(format!("[{name_inner}={val}]"));
                    }
                } else {
                    name_str = Cow::Borrowed(name_raw);
                }
            } else {
                name_str = Cow::Borrowed(name_raw);
            }
        }

        if !name_str.is_empty() {
            tag_line.push(' ');
            tag_line.push_str(&name_str);
        }

        // multiline types keep their type block shape
        if tag_line.contains('\n') {
            let desc_raw = description.parsed_preserving_whitespace();
            let desc_raw = desc_raw.trim();
            let desc_normalized = normalize_markdown_emphasis(desc_raw);
            let desc_raw = desc_normalized.trim();

            let mut lines_iter = tag_line.split('\n');
            if let Some(first) = lines_iter.next() {
                self.content_lines.push(first);
            }
            for line in lines_iter {
                self.content_lines.push(line);
            }
            if !desc_raw.is_empty() {
                let indent = Self::continuation_indent();
                let indent_width = self
                    .wrap_width
                    .saturating_sub(Self::continuation_indent_width());
                let desc = wrap_text(desc_raw, indent_width, 0, false, Some(self.format_options));
                self.push_indented_desc(indent, desc);
            }
            return;
        }

        let desc_ws_raw = description.parsed_preserving_whitespace();
        // source blank line after the name keeps the description separate
        let has_desc_blank_line = {
            let t = desc_ws_raw.trim_start_matches(' ');
            t.starts_with("\n\n") || t.starts_with("\n \n")
        };
        let desc_raw = desc_ws_raw.trim();
        let desc_normalized = normalize_markdown_emphasis(desc_raw);
        let desc_raw = desc_normalized.trim();

        // strip existing default text before appending the normalized suffix
        let desc_raw = if default_value.is_some() && self.options.add_default_to_description {
            strip_default_is_suffix(desc_raw)
        } else {
            Cow::Borrowed(desc_raw)
        };
        let desc_raw = desc_raw.trim();

        if desc_raw.is_empty() && default_value.is_none() {
            self.content_lines.push(tag_line);
            return;
        }

        // separate the description when the source had a blank line
        if has_desc_blank_line && !desc_raw.is_empty() {
            self.content_lines.push(tag_line);
            self.content_lines.push_empty();
            let indent = Self::continuation_indent();
            let indent_width = self
                .wrap_width
                .saturating_sub(Self::continuation_indent_width());
            let desc = wrap_text(desc_raw, indent_width, 0, false, Some(self.format_options));
            self.push_indented_desc(indent, desc);
            return;
        }

        // split without collecting all lines
        let (first_text_line, rest_of_desc) = match desc_raw.split_once('\n') {
            Some((first, rest)) => (first.trim(), Some(rest)),
            None => (desc_raw.trim(), None),
        };

        // code fences stay separated from the tag line
        if first_text_line.starts_with("```") {
            self.content_lines.push(tag_line);
            self.content_lines.push_empty();
            let indent = Self::continuation_indent();
            let indent_width = self
                .wrap_width
                .saturating_sub(Self::continuation_indent_width());
            let mut desc = wrap_text(desc_raw, indent_width, 0, false, Some(self.format_options));
            // skip the blank line already emitted above
            if desc.starts_with('\n') {
                desc.remove(0);
            }
            self.push_indented_desc(indent, desc);
            return;
        }

        // optional dash separator
        let (has_dash, first_text) = if let Some(rest) = first_text_line.strip_prefix("- ") {
            (true, rest)
        } else if first_text_line == "-" {
            (true, "")
        } else {
            (false, first_text_line)
        };

        let first_text: Cow<'_, str> = if should_capitalize {
            capitalize_first(first_text)
        } else {
            Cow::Borrowed(first_text)
        };

        let default_value_for_desc = if self.options.add_default_to_description {
            default_value
        } else {
            None
        };

        // suffix length: "default is `" + value + "`"
        let default_suffix_len: Option<usize> = default_value_for_desc.map(|dv| 13 + dv.len());

        if first_text.is_empty() && default_suffix_len.is_none() && rest_of_desc.is_none() {
            self.content_lines.push(tag_line);
            return;
        }

        // separator between the tag head and description
        let separator = if has_dash { " - " } else { " " };

        // remaining text keeps relative indentation
        let remaining_desc = if let Some(rest) = rest_of_desc {
            let base_indent = (!first_text_line.is_empty()).then_some(0);
            dedent_lines(rest, base_indent)
        } else {
            String::new()
        };
        let has_remaining = !remaining_desc.trim().is_empty();

        // compute one-line width without allocating
        let prefix_len = str_width(&tag_line) + str_width(separator);
        let one_liner_len = if has_remaining {
            prefix_len + str_width(&first_text)
        } else if let Some(ds_len) = default_suffix_len {
            if first_text.is_empty() {
                prefix_len + ds_len
            } else {
                prefix_len + str_width(&first_text) + 2 + ds_len
            }
        } else {
            prefix_len + str_width(&first_text)
        };

        if !has_remaining && one_liner_len <= self.wrap_width {
            // write one-line descriptions directly
            let s = self.content_lines.begin_line();
            s.push_str(&tag_line);
            s.push_str(separator);
            if let Some(dv) = default_value_for_desc {
                if first_text.is_empty() {
                    s.push_str("Default is `");
                    s.push_str(dv);
                    s.push('`');
                } else {
                    s.push_str(&first_text);
                    let last_char = first_text.as_bytes().last().copied().unwrap_or(b' ');
                    if matches!(last_char, b'.' | b'!' | b'?') {
                        s.push(' ');
                    } else {
                        s.push_str(". ");
                    }
                    s.push_str("Default is `");
                    s.push_str(dv);
                    s.push('`');
                }
            } else if self.options.description_with_dot {
                let dotted = super::normalize::append_trailing_dot(&first_text);
                s.push_str(&dotted);
            } else {
                s.push_str(&first_text);
            }
        } else {
            // multiline descriptions wrap with the tag prefix counted on the first line
            let indent = Self::continuation_indent();
            let indent_width = self
                .wrap_width
                .saturating_sub(Self::continuation_indent_width());

            // keep a standalone dash on the tag line
            if has_dash && first_text.is_empty() && has_remaining {
                let s = self.content_lines.begin_line();
                s.push_str(&tag_line);
                s.push_str(" -");
                let desc = wrap_text(
                    &remaining_desc,
                    indent_width,
                    0,
                    false,
                    Some(self.format_options),
                );
                self.push_indented_desc(indent, desc);
                return;
            }

            // build full description text before wrapping
            let full_desc = {
                let mut s = if has_remaining {
                    let mut s = String::with_capacity(first_text.len() + 1 + remaining_desc.len());
                    s.push_str(&first_text);
                    s.push('\n');
                    s.push_str(&remaining_desc);
                    s
                } else {
                    String::from(first_text.as_ref())
                };
                if let Some(dv) = default_value_for_desc {
                    let last = s.as_bytes().last().copied().unwrap_or(b' ');
                    if matches!(last, b'.' | b'!' | b'?') {
                        s.push(' ');
                    } else {
                        s.push_str(". ");
                    }
                    s.push_str("Default is `");
                    s.push_str(dv);
                    s.push('`');
                }
                s
            };

            let tag_str_len = prefix_len.saturating_sub(Self::continuation_indent_width());

            let first_word_w = full_desc.split_whitespace().next().map_or(0, str_width);
            if prefix_len + first_word_w > self.wrap_width {
                // move the description when the first word cannot fit after the tag
                let mut line = tag_line;
                if has_dash {
                    line.push_str(" -");
                }
                self.content_lines.push(line);
                let desc = wrap_text(
                    &full_desc,
                    indent_width,
                    0,
                    false,
                    Some(self.format_options),
                );
                self.push_indented_desc(indent, desc);
            } else {
                // wrap the description with the tag prefix counted on the first line
                let desc = wrap_text(
                    &full_desc,
                    indent_width,
                    tag_str_len,
                    false,
                    Some(self.format_options),
                );
                let mut iter = desc.split('\n');
                if let Some(first) = iter.next() {
                    let s = self.content_lines.begin_line();
                    s.push_str(&tag_line);
                    s.push_str(separator);
                    s.push_str(first);
                }
                for line in iter {
                    if line.is_empty() {
                        self.content_lines.push_empty();
                    } else {
                        let s = self.content_lines.begin_line();
                        s.push_str(indent);
                        s.push_str(line);
                    }
                }
            }
        }
    }

    pub(super) fn format_type_comment_tag(
        &mut self,
        normalized_kind: &str,
        tag: &JsdocTag<'_>,
        should_capitalize: bool,
        has_no_space_before_type: bool,
    ) {
        let (tag_type, description) = tag.type_comment();

        let tag_prefix_len = 1 + normalized_kind.len();
        let mut tag_line = String::with_capacity(tag_prefix_len + 32);
        tag_line.push('@');
        tag_line.push_str(normalized_kind);

        if let Some(tag_type) = &tag_type {
            let raw_type = self.normalize_multiline_type_indent(tag_type.raw());
            let raw_type = raw_type.trim();
            if !raw_type.is_empty() {
                if !has_no_space_before_type {
                    tag_line.push(' ');
                }
                tag_line.push_str(raw_type);
            }
        }

        let desc_text = description.parsed();
        let desc_text = normalize_markdown_emphasis(desc_text.trim());
        let desc_text = desc_text.trim();

        if desc_text.is_empty() {
            self.content_lines.push(tag_line);
            return;
        }

        let desc_text: Cow<'_, str> = if should_capitalize {
            capitalize_first(desc_text)
        } else {
            Cow::Borrowed(desc_text)
        };

        // multiline types keep a single-token description on the last type line
        if tag_line.contains('\n') {
            let last_line_width = tag_line.rsplit('\n').next().map_or(0, str_width);
            let desc_is_single_token = !desc_text.contains(' ');
            let desc_fits_on_last_line = desc_is_single_token
                && last_line_width + 1 + str_width(&desc_text) <= self.wrap_width;

            if desc_fits_on_last_line {
                // append the description to the final type line
                let mut lines: Vec<&str> = tag_line.split('\n').collect();
                if let Some(last) = lines.last_mut() {
                    let combined = format!("{last} {desc_text}");
                    for line in &lines[..lines.len() - 1] {
                        self.content_lines.push(*line);
                    }
                    self.content_lines.push(combined);
                }
            } else {
                let indent = Self::continuation_indent();
                let indent_width = self
                    .wrap_width
                    .saturating_sub(Self::continuation_indent_width());
                let mut lines_iter = tag_line.split('\n');
                if let Some(first) = lines_iter.next() {
                    self.content_lines.push(first);
                }
                for line in lines_iter {
                    self.content_lines.push(line);
                }
                let desc = wrap_text(
                    &desc_text,
                    indent_width,
                    0,
                    false,
                    Some(self.format_options),
                );
                self.push_indented_desc(indent, desc);
            }
            return;
        }

        // strip dash separators before wrapping so they are not parsed as markdown lists
        let (has_dash, desc_text_no_dash): (bool, Cow<'_, str>) =
            if let Some(rest) = desc_text.strip_prefix("- ") {
                (true, Cow::Borrowed(rest))
            } else if *desc_text == *"-" {
                (true, Cow::Borrowed(""))
            } else {
                (false, Cow::Borrowed(&*desc_text))
            };
        let separator = if has_dash { " - " } else { " " };
        let sep_len = separator.len();

        let prefix_len = str_width(&tag_line) + sep_len;
        let one_liner_len = prefix_len + str_width(&desc_text_no_dash);
        if one_liner_len <= self.wrap_width {
            let s = self.content_lines.begin_line();
            s.push_str(&tag_line);
            s.push_str(separator);
            s.push_str(&desc_text_no_dash);
        } else if desc_text_no_dash.is_empty() {
            // only a dash, no description text
            let s = self.content_lines.begin_line();
            s.push_str(&tag_line);
            s.push_str(separator);
        } else {
            // wrap the description with the tag prefix counted on the first line
            let indent = Self::continuation_indent();
            let indent_width = self
                .wrap_width
                .saturating_sub(Self::continuation_indent_width());
            let tag_str_len = prefix_len.saturating_sub(Self::continuation_indent_width());

            let first_word_w = desc_text_no_dash
                .split_whitespace()
                .next()
                .map_or(0, str_width);
            if prefix_len + first_word_w > self.wrap_width {
                self.content_lines.push(tag_line);
                let desc = wrap_text(
                    &desc_text_no_dash,
                    indent_width,
                    0,
                    false,
                    Some(self.format_options),
                );
                self.push_indented_desc(indent, desc);
            } else {
                let desc = wrap_text(
                    &desc_text_no_dash,
                    indent_width,
                    tag_str_len,
                    false,
                    Some(self.format_options),
                );
                let mut iter = desc.split('\n');
                if let Some(first) = iter.next() {
                    let s = self.content_lines.begin_line();
                    s.push_str(&tag_line);
                    s.push_str(separator);
                    s.push_str(first);
                }
                for line in iter {
                    if line.is_empty() {
                        self.content_lines.push_empty();
                    } else {
                        let s = self.content_lines.begin_line();
                        s.push_str(indent);
                        s.push_str(line);
                    }
                }
            }
        }
    }

    pub(super) fn format_generic_tag(
        &mut self,
        normalized_kind: &str,
        tag: &JsdocTag<'_>,
        should_capitalize: bool,
    ) {
        let mut tag_line = String::with_capacity(normalized_kind.len() + 1);
        tag_line.push('@');
        tag_line.push_str(normalized_kind);

        // detect a blank line between the tag and description:
        //   @internal
        //
        //   some description
        let raw_ws = tag.comment().parsed_preserving_whitespace();
        let has_leading_blank_line = {
            let trimmed_start = raw_ws.trim_start_matches(' ');
            trimmed_start.starts_with("\n\n") || trimmed_start.starts_with("\n \n")
        };
        // unknown tags preserve source line structure
        let desc_starts_on_new_line = {
            let trimmed_start = raw_ws.trim_start_matches(' ');
            trimmed_start.starts_with('\n')
        };

        let desc_text = tag.comment().parsed();
        let desc_text = normalize_markdown_emphasis(desc_text.trim());
        let desc_text = desc_text.trim();
        // whitespace-preserving description for verbatim tags
        let raw_ws_trimmed = raw_ws.trim();
        let raw_ws_normalized = normalize_markdown_emphasis(raw_ws_trimmed);
        let raw_ws_desc = raw_ws_normalized.trim();

        if desc_text.is_empty() {
            self.content_lines.push(tag_line);
            return;
        }

        let quote_style = self.quote_style();

        // format single-line default values
        let desc_text: Cow<'_, str> = if matches!(normalized_kind, "default" | "defaultValue") {
            if raw_ws_desc.contains('\n') {
                Cow::Borrowed(desc_text)
            } else {
                format_default_value(desc_text, quote_style)
            }
        } else if should_capitalize
            && is_named_generic_tag(normalized_kind)
            && !has_leading_blank_line
        {
            // named tags keep the first word as the name and capitalize the rest
            let name_end = if desc_text.starts_with('{') {
                find_balanced_brace_end(desc_text)
            } else {
                desc_text.find(|c: char| c.is_ascii_whitespace())
            };
            if let Some(name_end_idx) = name_end {
                let name = &desc_text[..name_end_idx];
                let description = desc_text[name_end_idx..].trim_start();
                if description.is_empty() {
                    Cow::Borrowed(desc_text)
                } else {
                    let capitalized = capitalize_first(description);
                    let mut s = String::with_capacity(name.len() + 1 + capitalized.len());
                    s.push_str(name);
                    s.push(' ');
                    s.push_str(&capitalized);
                    Cow::Owned(s)
                }
            } else {
                Cow::Borrowed(desc_text)
            }
        } else if should_capitalize {
            capitalize_first_skip_type(desc_text)
        } else {
            Cow::Borrowed(desc_text)
        };

        // preserve a blank line between the tag and the description
        if has_leading_blank_line {
            self.content_lines.push(tag_line);
            self.content_lines.push_empty();
            let skip_fmt = should_skip_description_formatting(normalized_kind);
            if skip_fmt && raw_ws_desc.contains('\n') {
                for line in raw_ws_desc.split('\n') {
                    if line.trim().is_empty() {
                        self.content_lines.push_empty();
                    } else {
                        self.content_lines.push(line);
                    }
                }
            } else if skip_fmt {
                let mut desc = wrap_text(
                    raw_ws_desc,
                    self.wrap_width,
                    0,
                    false,
                    Some(self.format_options),
                );
                if desc.starts_with('\n') {
                    desc.remove(0);
                }
                self.push_indented_desc("", desc);
            } else {
                let indent = Self::continuation_indent();
                let indent_width = self
                    .wrap_width
                    .saturating_sub(Self::continuation_indent_width());
                let mut desc = wrap_text(
                    &desc_text,
                    indent_width,
                    0,
                    false,
                    Some(self.format_options),
                );
                // skip the leading blank line since it was already emitted
                if desc.starts_with('\n') {
                    desc.remove(0);
                }
                self.push_indented_desc(indent, desc);
            }
            return;
        }

        // remarks always move their description to the next line
        if matches!(normalized_kind, "remarks" | "privateRemarks") {
            self.content_lines.push(tag_line);
            let indent = Self::continuation_indent();
            let indent_width = self
                .wrap_width
                .saturating_sub(Self::continuation_indent_width());
            let desc = wrap_text(
                &desc_text,
                indent_width,
                0,
                false,
                Some(self.format_options),
            );
            self.push_indented_desc(indent, desc);
            return;
        }

        // named generic tags preserve a source line break before the description
        if is_named_generic_tag(normalized_kind)
            && desc_starts_on_new_line
            && !should_skip_description_formatting(normalized_kind)
        {
            if let Some(space_idx) = desc_text.find(|c: char| c.is_ascii_whitespace()) {
                let name = &desc_text[..space_idx];
                let description = desc_text[space_idx..].trim_start();
                if !description.is_empty() {
                    let s = self.content_lines.begin_line();
                    s.push_str(&tag_line);
                    s.push(' ');
                    s.push_str(name);
                    let indent = Self::continuation_indent();
                    let indent_width = self
                        .wrap_width
                        .saturating_sub(Self::continuation_indent_width());
                    let desc = wrap_text(
                        description,
                        indent_width,
                        0,
                        false,
                        Some(self.format_options),
                    );
                    self.push_indented_desc(indent, desc);
                    return;
                }
            }
            let s = self.content_lines.begin_line();
            s.push_str(&tag_line);
            s.push(' ');
            s.push_str(&desc_text);
            return;
        }

        let prefix_len = str_width(&tag_line) + 1;
        let is_unknown = !is_known_tag(normalized_kind);
        let skip_wrapping = should_skip_description_formatting(normalized_kind) || is_unknown;

        // preserve source line breaks for unknown and verbatim tags
        let skip_formatting = should_skip_description_formatting(normalized_kind);
        if (is_unknown || skip_formatting) && desc_starts_on_new_line {
            self.content_lines.push(tag_line);
            // keep internal indentation in verbatim descriptions
            for line in raw_ws_desc.split('\n') {
                if line.trim().is_empty() {
                    self.content_lines.push_empty();
                } else {
                    self.content_lines.push(line);
                }
            }
            return;
        }

        let fits_on_one_line = prefix_len + str_width(&desc_text) <= self.wrap_width;
        if fits_on_one_line || skip_wrapping {
            // preserve blank lines when a verbatim description contains newlines
            if skip_wrapping && raw_ws_desc.contains('\n') {
                let mut lines = raw_ws_desc.split('\n');
                if let Some(first) = lines.next() {
                    let s = self.content_lines.begin_line();
                    s.push_str(&tag_line);
                    s.push(' ');
                    s.push_str(first);
                }
                for line in lines {
                    if line.is_empty() {
                        self.content_lines.push_empty();
                    } else {
                        self.content_lines.push(line);
                    }
                }
            } else if skip_wrapping && !fits_on_one_line {
                if should_preserve_description_verbatim(normalized_kind) || is_unknown {
                    let s = self.content_lines.begin_line();
                    s.push_str(&tag_line);
                    s.push(' ');
                    s.push_str(raw_ws_desc);
                } else {
                    // some tags skip capitalization but still wrap
                    let indent = Self::continuation_indent();
                    let indent_width = self
                        .wrap_width
                        .saturating_sub(Self::continuation_indent_width());
                    let tag_str_len = prefix_len.saturating_sub(Self::continuation_indent_width());
                    let desc = wrap_text(
                        raw_ws_desc,
                        indent_width,
                        tag_str_len,
                        false,
                        Some(self.format_options),
                    );
                    let mut iter = desc.split('\n');
                    if let Some(first) = iter.next() {
                        let s = self.content_lines.begin_line();
                        s.push_str(&tag_line);
                        s.push(' ');
                        s.push_str(first);
                    }
                    for line in iter {
                        if line.is_empty() {
                            self.content_lines.push_empty();
                        } else {
                            let s = self.content_lines.begin_line();
                            s.push_str(indent);
                            s.push_str(line);
                        }
                    }
                }
            } else {
                let s = self.content_lines.begin_line();
                s.push_str(&tag_line);
                s.push(' ');
                s.push_str(&desc_text);
            }
        } else {
            // wrap with the tag prefix counted on the first line
            let indent = Self::continuation_indent();
            let indent_width = self
                .wrap_width
                .saturating_sub(Self::continuation_indent_width());
            let tag_str_len = prefix_len.saturating_sub(Self::continuation_indent_width());

            let first_word_w = desc_text.split_whitespace().next().map_or(0, str_width);
            if prefix_len + first_word_w > self.wrap_width {
                self.content_lines.push(tag_line);
                let desc = wrap_text(
                    &desc_text,
                    indent_width,
                    0,
                    false,
                    Some(self.format_options),
                );
                self.push_indented_desc(indent, desc);
            } else {
                let desc = wrap_text(
                    &desc_text,
                    indent_width,
                    tag_str_len,
                    false,
                    Some(self.format_options),
                );
                let mut iter = desc.split('\n');
                if let Some(first) = iter.next() {
                    let s = self.content_lines.begin_line();
                    s.push_str(&tag_line);
                    s.push(' ');
                    s.push_str(first);
                }
                for line in iter {
                    if line.is_empty() {
                        self.content_lines.push_empty();
                    } else {
                        let s = self.content_lines.begin_line();
                        s.push_str(indent);
                        s.push_str(line);
                    }
                }
            }
        }
    }
}

/// Find the index just past the balanced closing `}` for a string that starts with `{`.
/// Returns the index after the matching `}`, accounting for nested braces.
/// Returns `None` if no balanced closing brace is found.
fn find_balanced_brace_end(s: &str) -> Option<usize> {
    debug_assert!(s.starts_with('{'));
    let mut depth: u32 = 0;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// Capitalize after a leading type expression.
fn capitalize_first_skip_type(s: &str) -> Cow<'_, str> {
    if !s.starts_with('{') {
        return capitalize_first(s);
    }
    if let Some(end) = find_balanced_brace_end(s) {
        let type_text = &s[..end];
        let rest = s[end..].trim_start();
        if rest.is_empty() {
            return Cow::Borrowed(s);
        }
        let capitalized = capitalize_first(rest);
        if matches!(capitalized, Cow::Borrowed(_)) {
            return Cow::Borrowed(s);
        }
        let mut result = String::with_capacity(type_text.len() + 1 + capitalized.len());
        result.push_str(type_text);
        result.push(' ');
        result.push_str(&capitalized);
        Cow::Owned(result)
    } else {
        capitalize_first(s)
    }
}
