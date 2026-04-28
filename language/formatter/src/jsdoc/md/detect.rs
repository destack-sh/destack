/// Check if the text contains markdown constructs that require full AST parsing.
/// Returns `false` only for pure plain-text paragraphs that `wrap_plain_paragraphs()`
/// can handle directly (no lists, tables, code fences, headings, blockquotes, or
/// inline markdown like emphasis/links).
pub(super) fn needs_markdown_parsing(text: &str) -> bool {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        match bytes[i] {
            // markdown markers
            b'~' | b'\\' | b'<' => return true,

            // inline emphasis and list markers
            b'*' | b'_' => {
                let next = if i + 1 < len { bytes[i + 1] } else { b' ' };
                let prev = if i > 0 { bytes[i - 1] } else { b' ' };

                // emphasis
                if !next.is_ascii_whitespace() || !prev.is_ascii_whitespace() {
                    return true;
                }

                // unordered list marker
                if bytes[i] == b'*' && next == b' ' && (i == 0 || prev == b'\n') {
                    return true;
                }
            }

            // links and references
            b'[' => {
                if i + 1 < len && bytes[i + 1] == b'^' {
                    return true;
                }

                // scan one line for link syntax
                let mut j = i + 1;
                let mut has_content = false;
                while j < len && bytes[j] != b'\n' {
                    if bytes[j] == b']' {
                        if has_content
                            && j + 1 < len
                            && (bytes[j + 1] == b'(' || bytes[j + 1] == b'[')
                        {
                            return true;
                        }
                        break;
                    }
                    if !bytes[j].is_ascii_whitespace() {
                        has_content = true;
                    }
                    j += 1;
                }
            }

            // block markers
            b' ' | b'#' | b'>' | b'-' | b'0'..=b'9' | b'|' | b'+'
                if i == 0 || bytes[i - 1] == b'\n' =>
            {
                // distinguish block starts from paragraph continuations
                let is_block_start = i == 0
                    || (i >= 2 && bytes[i - 1] == b'\n' && bytes[i - 2] == b'\n')
                    || (i >= 3
                        && bytes[i - 1] == b'\n'
                        && bytes[i - 2] == b' '
                        && bytes[i - 3] == b'\n');

                // count leading spaces
                let mut spaces = 0;
                while i + spaces < len && bytes[i + spaces] == b' ' {
                    spaces += 1;
                }

                // indented code block
                if spaces >= 4 && is_block_start {
                    return true;
                }

                // inspect line marker
                if i + spaces < len {
                    match bytes[i + spaces] {
                        // headings and blockquotes
                        b'#' | b'>' => return true,

                        // ordered lists and legacy markers
                        b'0'..=b'9' => {
                            // only trigger when the marker is complete
                            let mut j = i + spaces;
                            while j < len && bytes[j].is_ascii_digit() {
                                j += 1;
                            }
                            if j < len && j + 1 < len && bytes[j + 1] == b' ' {
                                match bytes[j] {
                                    b'.' | b')' if is_block_start => return true,
                                    b'-' => return true, // legacy marker always
                                    _ => {}
                                }
                            }
                        }
                        b'|' => {
                            // table rows require leading and trailing pipes
                            let line_start = i + spaces;
                            if bytes[line_start] == b'|' {
                                // find line end
                                let mut line_end = line_start + 1;
                                while line_end < len && bytes[line_end] != b'\n' {
                                    line_end += 1;
                                }

                                // check trailing pipe
                                let mut end = line_end;
                                while end > line_start + 1 && bytes[end - 1].is_ascii_whitespace() {
                                    end -= 1;
                                }
                                if end > line_start + 1 && bytes[end - 1] == b'|' {
                                    return true;
                                }
                            }
                        }

                        // unordered list markers
                        b'-' | b'+' | b'*'
                            if is_block_start
                                && i + spaces + 1 < len
                                && bytes[i + spaces + 1] == b' ' =>
                        {
                            return true;
                        }
                        _ => {}
                    }
                }
            }

            // code fences
            b'`' if i + 2 < len && bytes[i + 1] == b'`' && bytes[i + 2] == b'`' => {
                return true;
            }
            _ => {}
        }
        i += 1;
    }
    false
}
