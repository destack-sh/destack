use std::ops::Range;

/// The console colors.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Black,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    /// Get the ANSI color code for this color.
    pub fn code(&self) -> &'static str {
        match self {
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::White => "37",
            Color::Black => "30",
            Color::BrightRed => "91",
            Color::BrightGreen => "92",
            Color::BrightYellow => "93",
            Color::BrightBlue => "94",
            Color::BrightMagenta => "95",
            Color::BrightCyan => "96",
            Color::BrightWhite => "97",
        }
    }

    /// Apply this color to a string.
    pub fn apply(&self, text: &str) -> String {
        format!("\x1b[{}m{}\x1b[0m", self.code(), text)
    }
}

/// Metadata for a dumped line.
#[derive(Debug)]
struct LineMetadata {
    depth: usize,
    text_range: Range<usize>,
    has_connector: bool,
}

/// Structural information extracted from a raw line.
#[derive(Debug)]
struct LineLayout {
    depth: usize,
    text_start: usize,
    has_connector: bool,
}

/// Rebuild the tree drawing based on recorded lines.
pub fn rebuild_tree_output(buffer: String, use_colors: bool) -> String {
    // if the buffer is empty, early return
    if buffer.is_empty() {
        return buffer;
    }

    // collect metadata for each line before rewriting prefixes
    let mut metas: Vec<LineMetadata> = Vec::new();
    let mut line_start: usize = 0;

    // iterate through the lines and extract prefix/structure info
    for line in buffer.split('\n') {
        let current_start = line_start;
        let analysis = analyze_line(line);
        let text_start = current_start + analysis.text_start;
        let text_end = current_start + line.len();
        metas.push(LineMetadata {
            depth: analysis.depth,
            text_range: text_start..text_end,
            has_connector: analysis.has_connector,
        });
        line_start = current_start + line.len() + 1;
    }

    // if there are no lines, return the buffer as is
    let line_count = metas.len();
    if line_count == 0 {
        return buffer;
    }

    // reconstruct parent/child relationships for lines based on depth
    let mut parents: Vec<Option<usize>> = vec![None; line_count];
    let mut is_last: Vec<bool> = vec![true; line_count];
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); line_count];
    let mut stack: Vec<usize> = Vec::new();

    // determine parents and populate children for each line
    for (idx, meta) in metas.iter().enumerate() {
        // pop stack to match current depth
        while stack.len() > meta.depth {
            stack.pop();
        }
        if meta.depth > 0 {
            // set parent and register as a child
            if let Some(&parent_idx) = stack.last() {
                parents[idx] = Some(parent_idx);
                children[parent_idx].push(idx);
            }
        }
        // push current idx for future children
        stack.push(idx);
    }

    // determine which siblings are not the last at their level
    for siblings in &children {
        if siblings.len() <= 1 {
            continue;
        }
        // mark all but the last as not the last
        for child in siblings.iter().take(siblings.len() - 1) {
            is_last[*child] = false;
        }
    }

    // build the output with correct prefixes for each line
    let mut result = String::with_capacity(buffer.len());
    for (idx, meta) in metas.iter().enumerate() {
        // if depth > 0 or the line starts with a connector, add prefix
        if meta.depth > 0 || meta.has_connector {
            let prefix = build_prefix(idx, &parents, &is_last, use_colors);
            result.push_str(&prefix);
        }
        // append the content of the line after the prefix
        let line_segment = &buffer[meta.text_range.clone()];
        result.push_str(line_segment);
        // add line break except for final line
        if idx + 1 < line_count {
            result.push('\n');
        }
    }

    result
}

/// Analyze a line to locate prefixes and determine structural depth.
fn analyze_line(line: &str) -> LineLayout {
    // handle empty lines trivially
    if line.is_empty() {
        return LineLayout {
            depth: 0,
            text_start: 0,
            has_connector: false,
        };
    }

    // decode display characters while skipping over ANSI escape sequences
    let mut display_chars: Vec<char> = Vec::new();
    let mut byte_indices: Vec<usize> = Vec::new();
    let mut idx = 0;
    let bytes = line.as_bytes();
    while idx < bytes.len() {
        if bytes[idx] == b'\x1b' {
            // skip ANSI escape sequence
            idx += 1;
            if idx < bytes.len() && bytes[idx] == b'[' {
                idx += 1;
                while idx < bytes.len() && bytes[idx] != b'm' {
                    idx += 1;
                }
                if idx < bytes.len() {
                    idx += 1;
                }
            }
            continue;
        }
        // add display character and its byte index
        let ch = line[idx..].chars().next().unwrap();
        display_chars.push(ch);
        byte_indices.push(idx);
        idx += ch.len_utf8();
    }

    // parse prefix before text starts
    let mut depth = 0;
    let mut pos = 0;
    let mut has_connector = false;
    let mut text_display_idx = 0;

    // scan triples of chars to count tree guides and connectors
    while pos + 2 < display_chars.len() {
        let triple = (
            display_chars[pos],
            display_chars[pos + 1],
            display_chars[pos + 2],
        );
        // count depth for guide lines and spaces
        if triple == ('│', ' ', ' ') || triple == (' ', ' ', ' ') {
            depth += 1;
            pos += 3;
            continue;
        }
        // detect connector (├─ or └─ at start of a line)
        if (display_chars[pos] == '└' || display_chars[pos] == '├')
            && display_chars.get(pos + 1) == Some(&'─')
            && display_chars.get(pos + 2) == Some(&' ')
        {
            has_connector = true;
            text_display_idx = pos + 3;
        }
        break;
    }

    // if no connector, treat as text at depth 0
    if !has_connector {
        return LineLayout {
            depth: 0,
            text_start: 0,
            has_connector,
        };
    }

    // find byte index marking start of main text (skipping ANSI)
    let text_start = find_text_remainder_start(line, text_display_idx);

    LineLayout {
        depth,
        text_start,
        has_connector,
    }
}

/// Skip display characters while preserving ANSI codes for the text portion.
fn find_text_remainder_start(line: &str, display_to_skip: usize) -> usize {
    let bytes = line.as_bytes();
    let mut idx = 0;
    let mut skipped = 0;

    // skip display characters (including possible multi-byte) and count how many have been skipped
    while idx < bytes.len() && skipped < display_to_skip {
        // skip ANSI sequence entirely
        if bytes[idx] == b'\x1b' {
            idx += 1;
            if idx < bytes.len() && bytes[idx] == b'[' {
                idx += 1;
                while idx < bytes.len() && bytes[idx] != b'm' {
                    idx += 1;
                }
                if idx < bytes.len() {
                    idx += 1;
                }
            }
        }
        // move past a single char
        else {
            let ch = line[idx..].chars().next().unwrap();
            idx += ch.len_utf8();
            skipped += 1;
        }
    }

    // skip any trailing escape codes after display chars (reset or style remainers)
    loop {
        if idx >= bytes.len() || bytes[idx] != b'\x1b' {
            break;
        }
        let mut lookahead = idx + 1;
        if lookahead >= bytes.len() || bytes[lookahead] != b'[' {
            break;
        }
        lookahead += 1;
        let code_start = lookahead;
        while lookahead < bytes.len() && bytes[lookahead] != b'm' {
            lookahead += 1;
        }
        if lookahead >= bytes.len() {
            break;
        }
        let code = &line[code_start..lookahead];
        lookahead += 1;

        // skip reset codes (restore previous styles)
        if code == "0" {
            idx = lookahead;
            continue;
        }

        break;
    }

    idx
}

/// Build the prefix for a line using parent/last-sibling information.
/// Constructs the tree ASCII (or Unicode) lines for each depth level.
fn build_prefix(
    index: usize,
    parents: &[Option<usize>],
    is_last: &[bool],
    use_colors: bool,
) -> String {
    // collect full ancestry path for the current line
    let mut path: Vec<usize> = Vec::new();
    let mut current = Some(index);
    while let Some(idx) = current {
        path.push(idx);
        current = parents[idx];
    }
    path.reverse();

    // root node is not prefixed
    if path.len() <= 1 {
        return String::new();
    }

    let mut prefix = String::new();
    // for all levels but last, use guides/branches according to sibling position
    for level in 0..(path.len() - 1) {
        // for the direct parent, use branch or corner
        let segment = if level == path.len() - 2 {
            if is_last[path[level + 1]] {
                "└─ "
            } else {
                "├─ "
            }
        }
        // only pad if last at this level
        else if is_last[path[level + 1]] {
            "   "
        }
        // draw vertical guide if there are siblings deeper
        else {
            "│  "
        };
        append_segment(&mut prefix, segment, use_colors);
    }

    prefix
}

/// Append a segment to the output with optional coloring.
fn append_segment(buffer: &mut String, segment: &str, use_colors: bool) {
    if use_colors {
        buffer.push_str(Color::Cyan.apply(segment).as_str());
    } else {
        buffer.push_str(segment);
    }
}
