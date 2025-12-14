use std::ops::Range;

use crate::Color;

#[macro_export]
macro_rules! impl_dump_display {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl Dump for $ty {
                fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
                    dumper.write_str(format!("{self:?}").as_str(), Some($crate::Color::Yellow));
                }
            }
        )+
    };
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

/// Trait for types that can be dumped to a tree representation.
pub trait Dump {
    /// Dump this value to the dumper.
    fn dump<'a>(&self, dumper: &mut Dumper<'a>);
}

/// A dumper for creating tree representations of data structures.
#[derive(Debug)]
pub struct Dumper<'a> {
    /// The output buffer.
    buffer: &'a mut String,
    /// Current indentation depth.
    depth: usize,
    /// Whether to use colors.
    use_colors: bool,
}

impl<'a> Dumper<'a> {
    /// Create a new dumper.
    pub fn new(buffer: &'a mut String, use_colors: bool) -> Self {
        Self {
            buffer,
            depth: 0,
            use_colors,
        }
    }

    /// Write a string with optional color.
    pub fn write_str(&mut self, s: &str, color: Option<Color>) {
        if self.use_colors {
            if let Some(c) = color {
                self.buffer.push_str(&c.apply(s));
            } else {
                self.buffer.push_str(s);
            }
        } else {
            self.buffer.push_str(s);
        }
    }

    /// Write a string with optional color in bold.
    pub fn write_str_bold(&mut self, s: &str, color: Option<Color>) {
        if self.use_colors {
            if let Some(c) = color {
                self.buffer.push_str(&c.apply_bold(s));
            } else {
                self.buffer.push_str(s);
            }
        } else {
            self.buffer.push_str(s);
        }
    }

    /// Start a new line with proper indentation.
    pub fn newline(&mut self) {
        self.buffer.push('\n');
        for _ in 0..self.depth {
            self.buffer.push_str("├─ ");
        }
    }

    /// Increase indentation depth.
    pub fn indent(&mut self) {
        self.depth += 1;
    }

    /// Decrease indentation depth.
    pub fn dedent(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Get whether colors are enabled.
    pub fn use_colors(&self) -> bool {
        self.use_colors
    }
}
