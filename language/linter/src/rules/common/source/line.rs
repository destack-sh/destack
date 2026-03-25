use destack_source::{File, Span};

/// Count file lines with optional comment and blank line skipping.
pub fn count_file_lines(file: &File, skip_comments: bool, skip_blank_lines: bool) -> usize {
    let full_span = Span::new(file.id, 0, file.len);

    count_file_span_lines(file, full_span, skip_comments, skip_blank_lines)
}

/// Count lines inside one file span with optional comment and blank line skipping.
pub fn count_file_span_lines(
    file: &File,
    span: Span,
    skip_comments: bool,
    skip_blank_lines: bool,
) -> usize {
    if span.start >= span.end {
        return 0;
    }

    // resolve inclusive line bounds for the span
    let Some((start_line_index, _)) = file.get_position(span.start) else {
        return 0;
    };
    let Some((end_line_index, _)) = file.get_position(span.end.saturating_sub(1)) else {
        return 0;
    };

    // count relevant lines while tracking block comments
    let mut counted_lines = 0usize;
    let mut in_block_comment = false;

    for line_index in start_line_index..=end_line_index {
        let Some(line_span) = file.get_line_span(line_index) else {
            continue;
        };
        let Some(line_text) = file.get_line_str(line_index) else {
            continue;
        };

        // slice the line down to the requested span
        let slice_start = if line_index == start_line_index {
            span.start.saturating_sub(line_span.start) as usize
        } else {
            0
        }
        .min(line_text.len());
        let slice_end = if line_index == end_line_index {
            span.end.saturating_sub(line_span.start) as usize
        } else {
            line_text.len()
        }
        .min(line_text.len());
        let line_text = &line_text[slice_start..slice_end];

        // skip comment only lines when configured
        if skip_comments && !line_has_code_outside_comments(line_text, &mut in_block_comment) {
            continue;
        }

        // skip blank lines when configured
        if skip_blank_lines && line_text.trim().is_empty() {
            continue;
        }

        counted_lines += 1;
    }

    counted_lines
}

/// Return true when one line contains non-comment code.
pub fn line_has_code_outside_comments(line_text: &str, in_block_comment: &mut bool) -> bool {
    // track whether this line contains any code token
    let mut has_code = false;
    let bytes = line_text.as_bytes();
    let mut index = 0usize;

    // scan one line while tracking block comment state
    while index < bytes.len() {
        // consume an active block comment until close token or line end
        if *in_block_comment {
            let Some(close_offset) = line_text[index..].find("*/") else {
                return has_code;
            };

            index += close_offset + 2;
            *in_block_comment = false;
            continue;
        }

        // skip leading and interstitial whitespace
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }

        // stop at one line comment marker
        if line_text[index..].starts_with("//") {
            break;
        }

        // start block comment mode when block opener is found
        if line_text[index..].starts_with("/*") {
            *in_block_comment = true;
            index += 2;
            continue;
        }

        // mark code and continue scanning to track trailing block comments
        has_code = true;
        index += 1;
    }

    has_code
}
