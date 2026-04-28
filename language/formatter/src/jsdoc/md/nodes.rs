use std::borrow::Cow;

use destack_workspace::JsdocLineWrappingStyle;
use markdown::mdast::Node;

use super::super::embedded::{
    fenced_code_file_type, format_embedded_code, format_embedded_code_as,
};
use super::super::line::LineBuffer;
use super::super::normalize::{append_trailing_dot, capitalize_first};
use super::super::wrap::{format_table_block, indent_str, str_width, wrap_paragraph};
use super::SerializeOptions;
use super::collect::{
    collect_inline_recursive, collect_inline_text, collect_inline_text_from_children,
};

/// Serialize children of a parent node, inserting blank lines between block-level nodes.
///
/// Consecutive Paragraph and inline-like Html nodes are merged into a single
/// paragraph so that HTML tag references like `<option>` or `<div>` that the
/// markdown parser extracted as separate Html block nodes don't create spurious
/// blank lines in the middle of what should be a single paragraph.
pub(super) fn serialize_children(
    node: &Node,
    indent: usize,
    first_para_offset: usize,
    opts: &SerializeOptions<'_>,
    lines: &mut LineBuffer,
) {
    let Some(children) = node.children() else {
        return;
    };

    let mut i = 0;
    while i < children.len() {
        let child = &children[i];

        // merge paragraph and inline html runs
        if matches!(child, Node::Paragraph(_) | Node::Html(_)) {
            let run_start = i;
            let mut run_end = i + 1;
            let mut has_html = matches!(child, Node::Html(_));

            // extend run
            while run_end < children.len() {
                match &children[run_end] {
                    Node::Paragraph(_) => {
                        run_end += 1;
                    }
                    Node::Html(html) if is_inline_html(&html.value) => {
                        has_html = true;
                        run_end += 1;
                    }
                    _ => break,
                }
            }

            // merge mixed runs
            if has_html && run_end - run_start > 1 {
                if run_start > 0 && !lines.last_is_empty() {
                    lines.push_empty();
                }

                let mut merged_text = String::new();
                for child in children.iter().take(run_end).skip(run_start) {
                    match child {
                        Node::Paragraph(para) => {
                            let text = collect_inline_text_from_children(&para.children);
                            if !merged_text.is_empty() && !merged_text.ends_with(' ') {
                                merged_text.push(' ');
                            }
                            merged_text.push_str(text.trim());
                        }
                        Node::Html(html) => {
                            if !merged_text.is_empty() && !merged_text.ends_with(' ') {
                                merged_text.push(' ');
                            }
                            merged_text.push_str(html.value.trim());
                        }
                        _ => {}
                    }
                }

                // wrap merged paragraph
                let offset = if run_start == 0 { first_para_offset } else { 0 };
                let effective_width = opts.max_width.saturating_sub(indent);
                let ind = indent_str(indent);
                let mut para_buf = LineBuffer::new();
                wrap_paragraph(&merged_text, effective_width, offset, 0, &mut para_buf);
                let para_str = para_buf.into_string();

                let mut para_iter = para_str.split('\n').peekable();
                let mut li = 0usize;
                while let Some(line) = para_iter.next() {
                    let is_last = para_iter.peek().is_none();
                    if indent > 0 {
                        if line.is_empty() {
                            lines.push_empty();
                        } else if opts.description_with_dot && is_last {
                            let dotted = append_trailing_dot(line);
                            let s = lines.begin_line();
                            s.push_str(&ind);
                            s.push_str(&dotted);
                        } else {
                            let s = lines.begin_line();
                            s.push_str(&ind);
                            s.push_str(line);
                        }
                    } else if opts.capitalize && li == 0 {
                        let cap = capitalize_first(line);
                        if opts.description_with_dot && is_last {
                            lines.push(append_trailing_dot(&cap));
                        } else {
                            lines.push(cap);
                        }
                    } else if opts.description_with_dot && is_last {
                        lines.push(append_trailing_dot(line));
                    } else {
                        lines.push(line);
                    }
                    li += 1;
                }

                i = run_end;
                continue;
            }
        }

        // separate block siblings
        if i > 0 && is_block_node(child) && !lines.last_is_empty() {
            lines.push_empty();
        }

        // apply tag prefix offset once
        let offset = if i == 0 { first_para_offset } else { 0 };
        serialize_node(child, indent, offset, opts, lines);
        i += 1;
    }
}

/// Check if an HTML node looks like an inline tag reference that the markdown
/// parser incorrectly extracted as a block-level element.
///
/// In Jsdoc descriptions, `<div>`, `<table>`, etc. are usually mentioned as
/// tag names (e.g., "renders a `<div>` element") rather than actual HTML blocks.
/// The CommonMark parser treats these as HTML block starts, absorbing subsequent
/// text into the Html node. This function detects such cases so they can be
/// merged back into the surrounding paragraph.
///
/// Returns `true` when the Html node content looks like an inline tag reference
/// (possibly followed by absorbed paragraph text), NOT a genuine HTML block
/// with structured content.
fn is_inline_html(html: &str) -> bool {
    let trimmed = html.trim();
    if !trimmed.starts_with('<') {
        return false;
    }

    // find the end of the first tag
    let Some(tag_end) = trimmed.find('>') else {
        return false;
    };

    // extract the tag name
    let tag_content = &trimmed[1..tag_end];
    let tag_name = tag_content
        .trim_start_matches('/')
        .split(|c: char| c.is_ascii_whitespace() || c == '/')
        .next()
        .unwrap_or("");

    if tag_name.is_empty() {
        return false;
    }

    // simple tag references are inline
    let after_tag = trimmed[tag_end + 1..].trim();
    if after_tag.is_empty() {
        return true;
    }

    // closing tags still represent inline prose mentions here
    let closing_tag = format!("</{tag_name}>");
    if after_tag.contains(&closing_tag) {
        return true;
    }

    true
}

fn is_block_node(node: &Node) -> bool {
    matches!(
        node,
        Node::Paragraph(_)
            | Node::Heading(_)
            | Node::List(_)
            | Node::Code(_)
            | Node::Blockquote(_)
            | Node::ThematicBreak(_)
            | Node::Definition(_)
            | Node::Html(_)
    )
}

fn serialize_node(
    node: &Node,
    indent: usize,
    first_para_offset: usize,
    opts: &SerializeOptions<'_>,
    lines: &mut LineBuffer,
) {
    match node {
        Node::Root(_) => {
            serialize_children(node, indent, 0, opts, lines);
        }
        Node::Paragraph(para) => {
            serialize_paragraph(para, indent, first_para_offset, opts, lines);
        }
        Node::Heading(heading) => {
            let text = collect_inline_text(node);
            let prefix = "#".repeat(heading.depth as usize);
            if !lines.is_empty() && !lines.last_is_empty() {
                lines.push_empty();
            }
            {
                let s = lines.begin_line();
                s.push_str(&prefix);
                s.push(' ');
                s.push_str(&text);
            }
        }
        Node::List(list) => {
            serialize_list(list, indent, opts, lines);
        }
        // list items are handled by serialize_list, thematic breaks are dropped
        Node::ListItem(_) | Node::ThematicBreak(_) => {}
        Node::Code(code) => {
            serialize_code(code, opts, lines);
        }
        Node::Blockquote(bq) => {
            serialize_blockquote(bq, opts, lines);
        }
        Node::Definition(def) => {
            let label = def.label.as_deref().unwrap_or(&def.identifier);
            {
                let s = lines.begin_line();
                s.push('[');
                s.push_str(label);
                s.push_str("]: ");
                s.push_str(&def.url);
            }
        }
        Node::Html(html) => {
            for line in html.value.lines() {
                lines.push(line);
            }
        }
        // block-level inline fallback
        _ => {
            let text = collect_inline_text(node);
            if !text.is_empty() {
                lines.push(text);
            }
        }
    }
}

// ================================================================================
// paragraph serialization
// ================================================================================

/// Serialize a paragraph node. Handles Break nodes (hard line breaks from `\` at EOL)
/// by splitting the paragraph into segments that are wrapped independently.
fn serialize_paragraph(
    para: &markdown::mdast::Paragraph,
    indent: usize,
    first_line_offset: usize,
    opts: &SerializeOptions<'_>,
    lines: &mut LineBuffer,
) {
    if serialize_pipe_prefixed_paragraph(para, indent, opts, lines) {
        return;
    }

    // hard line breaks
    let has_breaks = para.children.iter().any(|c| matches!(c, Node::Break(_)));

    if has_breaks {
        // split into break segments
        let indent_str = indent_str(indent);
        let mut current_segment = String::new();

        for child in &para.children {
            if matches!(child, Node::Break(_)) {
                // emit current segment
                let text = current_segment.trim();
                if indent > 0 {
                    {
                        let s = lines.begin_line();
                        s.push_str(&indent_str);
                        s.push_str(text);
                        s.push('\\');
                    }
                } else if opts.capitalize && lines.is_empty() {
                    let text = capitalize_first(text);
                    {
                        let s = lines.begin_line();
                        s.push_str(&text);
                        s.push('\\');
                    }
                } else {
                    {
                        let s = lines.begin_line();
                        s.push_str(text);
                        s.push('\\');
                    }
                }
                current_segment.clear();
            } else {
                collect_inline_recursive(child, &mut current_segment);
            }
        }

        // emit final segment
        if !current_segment.trim().is_empty() {
            let text = current_segment.trim();
            if indent > 0 {
                {
                    let s = lines.begin_line();
                    s.push_str(&indent_str);
                    s.push_str(text);
                }
            } else {
                lines.push(text);
            }
        }
        return;
    }

    // collect and wrap inline text
    let inline_text = collect_inline_text_from_children(&para.children);
    let effective_width = opts.max_width.saturating_sub(indent);
    let ind = indent_str(indent);

    // preserve balanced source lines when they fit
    if matches!(opts.line_wrapping_style, JsdocLineWrappingStyle::Balance)
        && let Some(position) = para.position.as_ref()
    {
        let raw = &opts.source[position.start.offset..position.end.offset];
        let original_lines: Vec<&str> = raw
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();

        if original_lines.len() > 1
            && original_lines
                .iter()
                .all(|l| str_width(l) <= effective_width)
        {
            let total = original_lines.len();
            for (i, orig_line) in original_lines.iter().enumerate() {
                let is_first = i == 0;
                let is_last = i == total - 1;

                let line: String = {
                    let mut s = (*orig_line).to_owned();
                    if is_first && opts.capitalize {
                        s = capitalize_first(&s).into_owned();
                    }
                    if is_last && opts.description_with_dot {
                        s = append_trailing_dot(&s).into_owned();
                    }
                    s
                };

                if indent > 0 {
                    let s = lines.begin_line();
                    s.push_str(&ind);
                    s.push_str(&line);
                } else {
                    lines.push(&line);
                }
            }
            return;
        }
    }

    let mut para_buf = LineBuffer::new();
    wrap_paragraph(
        &inline_text,
        effective_width,
        first_line_offset,
        0,
        &mut para_buf,
    );
    let para_str = para_buf.into_string();

    let mut para_iter = para_str.split('\n').peekable();
    let mut i = 0usize;
    while let Some(line) = para_iter.next() {
        let is_last = para_iter.peek().is_none();
        if indent > 0 {
            if line.is_empty() {
                lines.push_empty();
            } else if opts.description_with_dot && is_last {
                let dotted = append_trailing_dot(line);
                let s = lines.begin_line();
                s.push_str(&ind);
                s.push_str(&dotted);
            } else {
                let s = lines.begin_line();
                s.push_str(&ind);
                s.push_str(line);
            }
        } else if opts.capitalize && i == 0 {
            let cap = capitalize_first(line);
            if opts.description_with_dot && is_last {
                lines.push(append_trailing_dot(&cap));
            } else {
                lines.push(cap);
            }
        } else if opts.description_with_dot && is_last {
            lines.push(append_trailing_dot(line));
        } else {
            lines.push(line);
        }
        i += 1;
    }
}

fn serialize_pipe_prefixed_paragraph(
    para: &markdown::mdast::Paragraph,
    indent: usize,
    opts: &SerializeOptions<'_>,
    lines: &mut LineBuffer,
) -> bool {
    let Some(position) = para.position.as_ref() else {
        return false;
    };

    let raw = &opts.source[position.start.offset..position.end.offset];
    let raw_lines: Vec<&str> = raw.lines().collect();

    // detect real table rows
    if raw_lines.is_empty()
        || !raw_lines.iter().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2
        })
    {
        return false;
    }

    let ind = indent_str(indent);
    let mut index = 0;
    let mut emitted_segment = false;

    while index < raw_lines.len() {
        while index < raw_lines.len() && raw_lines[index].trim().is_empty() {
            index += 1;
        }
        if index >= raw_lines.len() {
            break;
        }

        if emitted_segment && !lines.last_is_empty() {
            lines.push_empty();
        }

        let is_table = {
            let trimmed = raw_lines[index].trim_start();
            trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2
        };
        if is_table {
            let start = index;
            while index < raw_lines.len() && {
                let t = raw_lines[index].trim_start();
                t.starts_with('|') && t.ends_with('|') && t.len() > 2
            } {
                index += 1;
            }

            let block_lines = format_table_block(&raw_lines[start..index]);

            for line in block_lines {
                if indent > 0 && !line.is_empty() {
                    {
                        let s = lines.begin_line();
                        s.push_str(&ind);
                        s.push_str(&line);
                    }
                } else {
                    lines.push(line);
                }
            }
        } else {
            let start = index;
            while index < raw_lines.len()
                && !{
                    let t = raw_lines[index].trim_start();
                    t.starts_with('|') && t.ends_with('|') && t.len() > 2
                }
            {
                index += 1;
            }

            let text_parts: Vec<&str> = raw_lines[start..index]
                .iter()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect();
            if text_parts.is_empty() {
                continue;
            }

            let joined = text_parts.join(" ");
            let effective_width = opts.max_width.saturating_sub(indent);
            let mut para_buf = LineBuffer::new();
            wrap_paragraph(&joined, effective_width, 0, 0, &mut para_buf);
            let para_str = para_buf.into_string();

            for (i, line) in para_str.split('\n').enumerate() {
                if indent > 0 {
                    let s = lines.begin_line();
                    s.push_str(&ind);
                    s.push_str(line);
                } else if opts.capitalize && i == 0 {
                    lines.push(capitalize_first(line));
                } else {
                    lines.push(line);
                }
            }
        }

        emitted_segment = true;
    }

    true
}

// ================================================================================
// list serialization
// ================================================================================

fn serialize_list(
    list: &markdown::mdast::List,
    indent: usize,
    opts: &SerializeOptions<'_>,
    lines: &mut LineBuffer,
) {
    let ind = indent_str(indent);
    let mut counter = list.start.unwrap_or(1);

    for child in &list.children {
        let Node::ListItem(item) = child else {
            continue;
        };

        // build marker
        let (marker, marker_width) = if list.ordered {
            let num_str = counter.to_string();
            let mut m = String::with_capacity(num_str.len() + 2);
            m.push_str(&num_str);
            m.push_str(". ");
            let width = m.len();
            counter += 1;
            (Cow::Owned(m), width)
        } else {
            (Cow::Borrowed("- "), 2)
        };

        // serialize children
        let mut first_child = true;
        for item_child in &item.children {
            if first_child {
                // prepend first child marker
                let child_str = serialize_node_for_list_item(item_child, marker_width, true, opts);

                for (line_idx, line) in child_str.split('\n').enumerate() {
                    if line_idx == 0 {
                        let text = if opts.capitalize {
                            capitalize_first(line)
                        } else {
                            std::borrow::Cow::Borrowed(line)
                        };
                        {
                            let s = lines.begin_line();
                            s.push_str(&ind);
                            s.push_str(&marker);
                            s.push_str(&text);
                        }
                    } else if line.is_empty() {
                        lines.push_empty();
                    } else {
                        {
                            let s = lines.begin_line();
                            s.push_str(&ind);
                            s.push_str(line);
                        }
                    }
                }
                first_child = false;
            } else {
                // separate subsequent block children
                if is_block_node(item_child) && !lines.last_is_empty() {
                    lines.push_empty();
                }

                if matches!(item_child, Node::Definition(_)) {
                    serialize_node(item_child, 0, 0, opts, lines);
                }
                // align nested lists under the content block
                else if matches!(item_child, Node::List(_)) {
                    let nested_indent = if indent == 0 {
                        indent + marker_width
                    } else {
                        indent + marker_width + marker_width
                    };
                    serialize_node(item_child, nested_indent, 0, opts, lines);
                } else {
                    let child_str =
                        serialize_node_for_list_item(item_child, marker_width, false, opts);
                    let child_ind = indent_str(indent + marker_width);
                    for line in child_str.split('\n') {
                        if line.is_empty() {
                            lines.push_empty();
                        } else {
                            {
                                let s = lines.begin_line();
                                s.push_str(&child_ind);
                                s.push_str(line);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Serialize a node that is a child of a list item.
/// For paragraphs, we collect text, restore placeholders, then wrap.
///
/// `is_first_child`: If true, the first child of the list item. The first line wraps
/// at `max_width` and the marker is prepended by the caller. Continuation lines get
/// `marker_width` indent from wrap_paragraph.
///
/// If false, a subsequent child. The paragraph still wraps at `max_width`; the caller
/// prepends `marker_width` spaces afterward so continuation blocks stay aligned to the
/// list item's content column.
fn serialize_node_for_list_item(
    node: &Node,
    marker_width: usize,
    is_first_child: bool,
    opts: &SerializeOptions<'_>,
) -> String {
    if let Node::Paragraph(para) = node {
        let inline_text = collect_inline_text_from_children(&para.children);
        let mut buf = LineBuffer::new();
        if is_first_child {
            wrap_paragraph(&inline_text, opts.max_width, 0, marker_width, &mut buf);
        } else {
            wrap_paragraph(&inline_text, opts.max_width, 0, 0, &mut buf);
        }
        buf.into_string()
    } else {
        let mut buf = LineBuffer::new();
        serialize_node(node, 0, 0, opts, &mut buf);
        buf.into_string()
    }
}

// ================================================================================
// code block serialization
// ================================================================================

fn serialize_code(
    code: &markdown::mdast::Code,
    opts: &SerializeOptions<'_>,
    lines: &mut LineBuffer,
) {
    // separate code block
    if !lines.is_empty() && !lines.last_is_empty() {
        lines.push_empty();
    }

    let code_width = opts.max_width.saturating_sub(4);
    let formatted_value = format_code_value(&code.value, code.lang.as_deref(), code_width, opts);

    let has_lang = code.lang.as_ref().is_some_and(|l| !l.is_empty());
    let use_fence = has_lang || opts.prefer_code_fences;

    if use_fence {
        // fenced code block
        {
            let s = lines.begin_line();
            s.push_str("```");
            if let Some(lang) = &code.lang {
                s.push_str(lang);
            }
        }
        for line in formatted_value.lines() {
            lines.push(line);
        }
        lines.push("```");
    } else {
        // indented code block
        let min_indent = formatted_value
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start().len())
            .min();
        for line in formatted_value.lines() {
            if line.is_empty() {
                lines.push_empty();
            } else {
                {
                    let s = lines.begin_line();
                    s.push_str("    ");
                    if let Some(min_indent) = min_indent
                        && min_indent > 0
                        && line.len() >= min_indent
                    {
                        s.push_str(&line[min_indent..]);
                    } else {
                        s.push_str(line);
                    }
                }
            }
        }
    }
}

/// Format code block content in Jsdoc descriptions.
///
/// Known code fences use their language, unlabeled fences use the surrounding
/// formatter language, and unknown code fences are preserved verbatim.
fn format_code_value<'a>(
    code: &'a str,
    lang: Option<&str>,
    width: usize,
    opts: &SerializeOptions<'_>,
) -> Cow<'a, str> {
    if let Some(format_options) = opts.format_options {
        // explicit code fences use their own language
        if let Some(lang) = lang {
            let Some(file_type) = fenced_code_file_type(lang) else {
                return Cow::Borrowed(code);
            };

            if let Some(formatted) = format_embedded_code_as(code, width, format_options, file_type)
            {
                return Cow::Owned(formatted);
            }
        }
        // unlabeled code blocks inherit the surrounding language
        else if let Some(formatted) = format_embedded_code(code, width, format_options) {
            return Cow::Owned(formatted);
        }
    }
    Cow::Borrowed(code)
}

// ================================================================================
// blockquote serialization
// ================================================================================

fn serialize_blockquote(
    bq: &markdown::mdast::Blockquote,
    opts: &SerializeOptions<'_>,
    lines: &mut LineBuffer,
) {
    // serialize blockquote sections
    for (i, child) in bq.children.iter().enumerate() {
        if i > 0 {
            lines.push_empty();
        }
        let mut inner_buf = LineBuffer::new();
        serialize_node(child, 0, 0, opts, &mut inner_buf);
        for line in inner_buf.into_string().split('\n') {
            if line.is_empty() {
                lines.push(">");
            } else {
                {
                    let s = lines.begin_line();
                    s.push_str("> ");
                    s.push_str(line);
                }
            }
        }
    }
}
