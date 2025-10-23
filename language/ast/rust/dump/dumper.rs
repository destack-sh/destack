#![allow(clippy::match_like_matches_macro)]

use std::borrow::Cow;

use std::ops::Range;

use crate::*;

/// The console colors.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum Color {
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
    fn code(&self) -> &'static str {
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
    fn apply(&self, text: &str) -> String {
        format!("\x1b[{}m{}\x1b[0m", self.code(), text)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DumperOptions {
    /// Spaces per indent.
    pub indent: usize,
    /// Maximum depth to dump. 0 = unlimited.
    pub max_depth: usize,
    /// Use colors.
    pub use_colors: bool,
    /// If true, show NodeId<T> raw index.
    pub show_id: bool,
    /// If true, print source Spans.
    pub show_span: bool,
}

impl Default for DumperOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            max_depth: 0,
            use_colors: true,
            show_id: false,
            show_span: false,
        }
    }
}

/// A Dumper for dumping AST nodes.
#[derive(Debug)]
pub struct Dumper<'a> {
    /// The string pool.
    pub strings: &'a StringPool,
    /// The node tree.
    pub tree: &'a NodeTree,
    /// The dump options.
    pub options: DumperOptions,

    /// The buffer we're writing to.
    buffer: String,
    /// The current depth (see with_depth).
    depth: usize,
    /// Tracks, for each depth level, whether there are more siblings after the current one
    /// at that level. Used to render tree branches with gaps correctly.
    branch_stack: Vec<bool>,
    /// Tracks has_more flag of the most recent printed line at current level.
    last_line_has_more: Option<bool>,
}

impl<'a> Dumper<'a> {
    /// Create a new Dumper.
    pub fn new(strings: &'a StringPool, tree: &'a NodeTree, options: DumperOptions) -> Self {
        Self {
            strings,
            tree,
            options,
            buffer: String::new(),
            depth: 0,
            branch_stack: Vec::new(),
            last_line_has_more: None,
        }
    }

    /// Finish dumping and return the result.
    pub fn finish(self) -> String {
        rebuild_tree_output(self.buffer, self.options.use_colors)
    }

    /// Write a string to the buffer with a new depth context.
    pub fn with_depth(&mut self, lambda: impl FnOnce(&mut Self)) {
        self.depth += 1;
        let parent_flag = self.last_line_has_more;
        if let Some(parent_has_more) = parent_flag {
            self.branch_stack.push(parent_has_more);
        }
        lambda(self);
        if parent_flag.is_some() {
            let _ = self.branch_stack.pop();
        }
        self.depth -= 1;
    }

    /// Write the prefix for the current depth.
    #[inline]
    fn write_prefix(&mut self) {
        let depth = self.branch_stack.len();
        if depth == 0 {
            return;
        }
        // draw ancestor columns
        for i in 0..(depth - 1) {
            if let Some(has_more) = self.branch_stack.get(i) {
                if *has_more {
                    self.write_str("│  ", Some(Color::Cyan));
                } else {
                    self.write_str("   ", Some(Color::Cyan));
                }
            }
        }
        // draw connector for current level
        let has_more_here = self.branch_stack[depth - 1];
        if has_more_here {
            self.write_str("├─ ", Some(Color::Cyan));
        } else {
            self.write_str("└─ ", Some(Color::Cyan));
        }
    }

    /// Write a string to the buffer.
    #[inline]
    fn write_str(&mut self, s: &str, color: Option<Color>) {
        if self.options.use_colors {
            if let Some(color) = color {
                self.buffer.push_str(color.apply(s).as_str());
            } else {
                self.buffer.push_str(s);
            }
        } else {
            self.buffer.push_str(s);
        }
    }

    /// Write a char to the buffer.
    #[inline]
    fn write_char(&mut self, c: char, color: Option<Color>) {
        if self.options.use_colors {
            if let Some(color) = color {
                self.buffer.push_str(color.apply(&c.to_string()).as_str());
            } else {
                self.buffer.push(c);
            }
        } else {
            self.buffer.push(c);
        }
    }

    /// Write the string behind a StringId.
    #[inline]
    pub fn write_string_id(&mut self, id: StringId) {
        self.write_char('"', Some(Color::White));
        self.write_str(self.strings.get(id), Some(Color::Yellow));
        self.write_char('"', Some(Color::White));
    }

    /// Write the path behind a PathId.
    #[inline]
    pub fn write_path(&mut self, path: &Path) {
        for (i, string_id) in path.segments.iter().enumerate() {
            let string = self.strings.get(*string_id);
            self.write_str(string, Some(Color::Green));
            if i + 1 < path.segments.len() {
                self.write_str(".", Some(Color::White));
            }
        }
    }

    /// Helper for dumping a single node.
    #[inline]
    pub fn node<'d>(&'d mut self, name: &str, id: u32) -> StructDumper<'d, 'a> {
        let has_more = false;
        // draw prefix depending on current structural depth
        if self.branch_stack.is_empty() {
            // no ancestor columns recorded, but we are nested: draw just the connector
            if has_more {
                self.write_str("├─ ", Some(Color::Cyan));
            } else {
                self.write_str("└─ ", Some(Color::Cyan));
            }
        } else {
            // we have ancestor columns; push our connector, render, then pop
            self.branch_stack.push(has_more);
            self.write_prefix();
            let _ = self.branch_stack.pop();
        }
        // set before dumping so nested with_depth sees correct parent branch info
        self.last_line_has_more = Some(has_more);
        StructDumper::new(self, name, Some(id))
    }

    /// Helper for dumping a single struct.
    #[inline]
    pub fn object<'d>(&'d mut self, name: &str) -> StructDumper<'d, 'a> {
        StructDumper::new(self, name, None)
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
fn rebuild_tree_output(buffer: String, use_colors: bool) -> String {
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

/// Helper for dumping a single struct-like type.
#[derive(Debug)]
pub struct StructDumper<'d, 'p> {
    dumper: &'d mut Dumper<'p>,
    node_id: Option<u32>,
    has_fields: bool,
}

impl<'d, 'p> StructDumper<'d, 'p> {
    /// Begin a new struct-like dumper with some name.
    pub fn new(dumper: &'d mut Dumper<'p>, name: &str, node_id: Option<u32>) -> Self {
        dumper.write_str(name, Some(Color::BrightBlue));
        Self {
            dumper,
            node_id,
            has_fields: false,
        }
    }

    /// Add a new field to the generated struct output.
    pub fn field<T: Dump>(&mut self, name: &str, value: &T) -> &mut Self {
        let prefix = if self.has_fields { ", " } else { " { " };
        self.dumper.write_str(prefix, Some(Color::White));
        self.dumper.write_str(name, Some(Color::Magenta));
        self.dumper.write_str(": ", Some(Color::White));
        value.dump(self.dumper);
        self.has_fields = true;
        self
    }

    /// Add a field optional if it is Some.
    pub fn field_optional<T: Dump>(&mut self, name: &str, value: &Option<T>) -> &mut Self {
        if let Some(value) = value {
            self.field(name, value);
        }
        self
    }

    /// Add a new field to the generated struct output.
    pub fn value<T: Dump>(&mut self, value: &T) -> &mut Self {
        let prefix = if self.has_fields { ", " } else { " { " };
        self.dumper.write_str(prefix, Some(Color::White));
        value.dump(self.dumper);
        self.has_fields = true;
        self
    }

    /// Finish node and close the struct as exhaustive.
    pub fn end(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(" }", Some(Color::White));
        }
        if let Some(node_id) = self.node_id {
            let span = self.dumper.tree.spans.get_by_id(node_id);
            self.dumper.write_str(
                format!(" :{} [{}..{}]", node_id, span.start, span.end).as_str(),
                Some(Color::White),
            );
            self.dumper.write_char('\n', None);
        }
        self
    }
}

pub trait Dump {
    /// Dump self to the Dumper in some form.
    fn dump<'a>(&self, dumper: &mut Dumper<'a>);
}

// ----------------------------------------------------------------------------
// Blanket impls
// ----------------------------------------------------------------------------

/// Dump a &str as a string.
impl Dump for &str {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_char('"', Some(Color::White));
        dumper.write_str(self.as_ref(), Some(Color::BrightYellow));
        dumper.write_char('"', Some(Color::White));
    }
}

/// Dump a String as a string.
impl Dump for String {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_char('"', Some(Color::White));
        dumper.write_str(self.as_str(), Some(Color::BrightYellow));
        dumper.write_char('"', Some(Color::White));
    }
}

/// Dump an Option<T> as a string.
impl<T: Dump> Dump for Option<T> {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Some(v) => v.dump(dumper),
            None => dumper.write_str("<None>", Some(Color::White)),
        }
    }
}

/// Dump a Vec<T> as a slice.
impl<T: Dump> Dump for Vec<T> {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        self.as_slice().dump(dumper)
    }
}

/// Dump a bool as a string.
impl Dump for bool {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(if *self { "true" } else { "false" }, None)
    }
}

/// Dump a u8 as a string.
impl Dump for u8 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(&self.to_string(), None)
    }
}

/// Dump a u16 as a string.
impl Dump for u16 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(&self.to_string(), None)
    }
}

/// Dump a u32 as a string.
impl Dump for u32 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(&self.to_string(), None)
    }
}

/// Dump an i64 as a string.
impl Dump for i64 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(&self.to_string(), None)
    }
}

/// Dump an f64 as a string.
impl Dump for f64 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(&self.to_string(), None)
    }
}

/// Dump a char as a quoted character.
impl Dump for char {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_char('\'', None);
        dumper.write_str(&self.to_string(), None);
        dumper.write_char('\'', None);
    }
}

/// Dump a &[T] as a string.
impl<T: Dump> Dump for &[T] {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str("[", Some(Color::White));
        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                dumper.write_str(", ", Some(Color::White));
            }
            v.dump(dumper);
        }
        dumper.write_str("]", Some(Color::White));
    }
}

/// Dump a NodeId<T> as the node it points to.
impl<T: Node + Clone + Dump> Dump for NodeId<T>
where
    NodeTree: NodeTreeStore<T>,
{
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let node = dumper.tree.get(*self);
        node.dump(dumper);
    }
}

/// Dump a StringId as a string.
impl Dump for StringId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_string_id(*self);
    }
}

/// Dump an Identifier as a string.
impl Dump for Name {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Name::Identifier(id) => id.dump(dumper),
            Name::String(id) => {
                dumper.write_char('[', Some(Color::White));
                id.dump(dumper);
                dumper.write_char(']', Some(Color::White));
            }
        }
    }
}

/// Dump a Path as a string.
impl Dump for Path {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_path(self);
    }
}

/// Dump a UnaryOperator as a string.
impl Dump for UnaryOperator {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a BinaryOperator as a string.
impl Dump for BinaryOperator {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump an AssignOperator as a string.
impl Dump for AssignOperator {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a Runtime as a string.
impl Dump for Runtime {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a FunctionCardinality as a string.
impl Dump for FunctionCardinality {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a FunctionAccessor as a string.
impl Dump for FunctionAccessor {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a MaybePosition as a string.
impl Dump for PostfixPosition {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a IfStyle as a string.
impl Dump for IfStyle {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a YieldCardinality as a string.
impl Dump for YieldCardinality {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a FunctionStyle as a string.
impl Dump for FunctionStyle {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a Mutability as a string.
impl Dump for Mutability {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a ScopedMutability as a string.
impl Dump for ScopedMutability {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ScopedMutability::Scoped { mutability, scopes } => {
                dumper
                    .object("ScopedMutability::Scoped")
                    .field("mutability", mutability)
                    .field("scopes", scopes)
                    .end();
            }
            ScopedMutability::Unscoped { mutability } => {
                dumper
                    .object("ScopedMutability::Unscoped")
                    .field("mutability", mutability)
                    .end();
            }
        }
    }
}

/// Dump an ExportMode as a string.
impl Dump for ExportMode {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a Visibility as a string.
impl Dump for Visibility {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a BlockFormat as a string.
impl Dump for BlockFormat {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a ModuleFormat as a string.
impl Dump for ModuleFormat {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a VariantStyle as a string.
impl Dump for VariantStyle {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump an AnnotationPosition as a string.
impl Dump for AnnotationPosition {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a CommentStyle as a string.
impl Dump for CommentStyle {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a DocStyle as a string.
impl Dump for DocStyle {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump an ImportTarget as a string.
impl Dump for ImportTarget {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ImportTarget::Path(path) => {
                dumper.object("ImportTarget::Path").value(path).end();
            }
            ImportTarget::Virtual(string) => {
                dumper.object("ImportTarget::Virtual").value(string).end();
            }
        }
    }
}

/// Dump an IntType as a structured representation.
impl Dump for IntType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("IntType")
            .field("width", &self.width)
            .field("is_signed", &self.is_signed)
            .end();
    }
}

/// Dump a FloatType as a string.
impl Dump for FloatType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.object("FloatType").field("width", &self.width).end();
    }
}

/// Dump a CompositeType as a structured representation.
impl Dump for CompositeType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            CompositeType::Type => {
                dumper.object("CompositeType::Type").end();
            }
            CompositeType::Struct => {
                dumper.object("CompositeType::Struct").end();
            }
            CompositeType::Enum => {
                dumper.object("CompositeType::Enum").end();
            }
            CompositeType::Union => {
                dumper.object("CompositeType::Union").end();
            }
            CompositeType::Tuple => {
                dumper.object("CompositeType::Tuple").end();
            }
            CompositeType::Interface => {
                dumper.object("CompositeType::Trait").end();
            }
            CompositeType::Function => {
                dumper.object("CompositeType::Function").end();
            }
        }
    }
}

/// Dump a ScalarLiteral as a structured representation.
impl Dump for ScalarLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ScalarLiteral::Boolean(value) => {
                dumper
                    .object("ScalarLiteral::Boolean")
                    .value(&value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Byte(value) => {
                dumper
                    .object("ScalarLiteral::Byte")
                    .value(&value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Integer(value) => {
                dumper
                    .object("ScalarLiteral::Integer")
                    .value(&value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Float(value) => {
                dumper
                    .object("ScalarLiteral::Float")
                    .value(&value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Character(value) => {
                dumper
                    .object("ScalarLiteral::Character")
                    .value(&value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::String(value) => {
                dumper.object("ScalarLiteral::String").value(value).end();
            }
            ScalarLiteral::RegexString { content, flags } => {
                dumper
                    .object("ScalarLiteral::RegexString")
                    .field("content", content)
                    .field_optional("flags", flags)
                    .end();
            }
            ScalarLiteral::ByteString(value) => {
                dumper
                    .object("ScalarLiteral::ByteString")
                    .value(value)
                    .end();
            }
        }
    }
}

/// Dump a TemplateLiteral as a structured representation.
impl Dump for TemplateLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            TemplateLiteral::String { string: template } => {
                dumper
                    .object("TemplateLiteral::String")
                    .field("template", template)
                    .end();
            }
            TemplateLiteral::TaggedString {
                tag,
                string: template,
            } => {
                dumper
                    .object("TemplateLiteral::TaggedString")
                    .field("tag", tag)
                    .field("template", template)
                    .end();
            }
            TemplateLiteral::InterpolatedString {
                strings: template,
                arguments: _,
            } => {
                dumper
                    .object("TemplateLiteral::InterpolatedString")
                    .field("template", template)
                    .end();
            }
            TemplateLiteral::TaggedInterpolatedString {
                tag,
                strings: template,
                arguments: _,
            } => {
                dumper
                    .object("TemplateLiteral::TaggedInterpolatedString")
                    .field("tag", tag)
                    .field("template", template)
                    .end();
            }
        }
    }
}

/// Dump a TypeLiteral as a structured representation.
impl Dump for TypeLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            TypeLiteral::Never => {
                dumper.object("TypeLiteral::Never").end();
            }
            TypeLiteral::Any => {
                dumper.object("TypeLiteral::Any").end();
            }
            TypeLiteral::Infer => {
                dumper.object("TypeLiteral::Infer").end();
            }
            TypeLiteral::Undefined => {
                dumper.object("TypeLiteral::Undefined").end();
            }
            TypeLiteral::Unknown => {
                dumper.object("TypeLiteral::Unknown").end();
            }
            TypeLiteral::Void => {
                dumper.object("TypeLiteral::Void").end();
            }
            TypeLiteral::Null => {
                dumper.object("TypeLiteral::Null").end();
            }
            TypeLiteral::Boolean => {
                dumper.object("TypeLiteral::Boolean").end();
            }
            TypeLiteral::Character => {
                dumper.object("TypeLiteral::Character").end();
            }
            TypeLiteral::String => {
                dumper.object("TypeLiteral::String").end();
            }
            TypeLiteral::Number => {
                dumper.object("TypeLiteral::Number").end();
            }
            TypeLiteral::Int(int_type) => {
                dumper.object("TypeLiteral::Int").value(int_type).end();
            }
            TypeLiteral::Float(float_type) => {
                dumper.object("TypeLiteral::Float").value(float_type).end();
            }
            TypeLiteral::Composite(composite_type) => {
                dumper
                    .object("TypeLiteral::Composite")
                    .value(composite_type)
                    .end();
            }
            TypeLiteral::Self_ => {
                dumper.object("TypeLiteral::Self").end();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Nodes
// ----------------------------------------------------------------------------

impl<'a> NodeVisitor for Dumper<'a> {
    fn visit_any(&mut self, tree: &NodeTree, _ty: NodeType, id: u32) {
        let annotations = tree.get_annotations_for(id);
        for annotation_id in annotations {
            let annotation = tree.get(annotation_id);
            self.visit_annotation(tree, annotation_id, annotation);
        }
    }

    // ------------------------------------------------------------
    // Groupings
    // ------------------------------------------------------------

    fn visit_expression(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Definition(_) => {
                self.node("Expression::Definition", _id.id).end();
            }
            Expression::Block(_) => {
                self.node("Expression::Block", _id.id).end();
            }
            Expression::With {
                clauses: _,
                body: _,
            } => {
                self.node("Expression::With", _id.id).end();
            }
            Expression::Import {
                target,
                alias,
                items: _,
            } => {
                self.node("Expression::Use", _id.id)
                    .field("target", target)
                    .field_optional("alias", alias)
                    .end();
            }
            Expression::Export {
                mode,
                target,
                alias,
                items: _,
            } => {
                self.node("Expression::Export", _id.id)
                    .field("mode", mode)
                    .field_optional("target", target)
                    .field_optional("alias", alias)
                    .end();
            }
            Expression::Let {
                mutability,
                visibility,
                export,
                pattern: _,
                ty: _,
                value: _,
            } => {
                self.node("Expression::Let", _id.id)
                    .field("mutability", mutability)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .end();
            }
            Expression::LetType {
                mutability,
                name,
                static_parameters: _,
                visibility,
                export,
                value: _,
            } => {
                self.node("Expression::LetType", _id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .end();
            }
            Expression::Type {
                mutability,
                value: _,
            } => {
                self.node("Expression::Type", _id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Expression::If {
                runtime,
                style,
                condition: _,
                then_expression: _,
                else_expression: _,
            } => {
                self.node("Expression::If", _id.id)
                    .field_optional("runtime", runtime)
                    .field("style", style)
                    .end();
            }
            Expression::While {
                runtime,
                condition: _,
                body: _,
            } => {
                self.node("Expression::While", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::For {
                runtime,
                pattern: _,
                iterator: _,
                body: _,
            } => {
                self.node("Expression::For", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Loop { runtime, body: _ } => {
                self.node("Expression::Loop", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Try {
                runtime,
                try_expression: _,
                catch_pattern: _,
                catch_expression: _,
                finally_expression: _,
            } => {
                self.node("Expression::Try", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Match {
                runtime,
                value: _,
                cases: _,
            } => {
                self.node("Expression::Match", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Break { label, value: _ } => {
                self.node("Expression::Break", _id.id)
                    .field_optional("label", label)
                    .end();
            }
            Expression::Continue { label } => {
                self.node("Expression::Continue", _id.id)
                    .field_optional("label", label)
                    .end();
            }
            Expression::Defer {
                expression: _,
                catch: _,
            } => {
                self.node("Expression::Defer", _id.id).end();
            }
            Expression::Await { expression: _ } => {
                self.node("Expression::Await", _id.id).end();
            }
            Expression::Yield { cardinality, value: _ } => {
                self.node("Expression::Yield", _id.id)
                    .field("cardinality", cardinality)
                    .end();
            }
            Expression::Return { value: _ } => {
                self.node("Expression::Return", _id.id).end();
            }
            Expression::Path {
                path,
                static_arguments: _,
            } => {
                self.node("Expression::Path", _id.id).value(path).end();
            }
            Expression::ScalarLiteral(value) => {
                self.node("Expression::ScalarLiteral", _id.id)
                    .value(value)
                    .end();
            }
            Expression::TemplateLiteral(value) => {
                self.node("Expression::TemplateLiteral", _id.id)
                    .value(value)
                    .end();
            }
            Expression::TypeLiteral(value) => {
                self.node("Expression::TypeLiteral", _id.id)
                    .value(value)
                    .end();
            }
            Expression::RangeLiteral {
                start: _,
                end: _,
                is_inclusive,
            } => {
                self.node("Expression::RangeLiteral", _id.id)
                    .field("is_inclusive", is_inclusive)
                    .end();
            }
            Expression::ArrayLiteral { elements: _ } => {
                self.node("Expression::ArrayLiteral", _id.id).end();
            }
            Expression::TupleLiteral { elements: _ } => {
                self.node("Expression::TupleLiteral", _id.id).end();
            }
            Expression::StructLiteral { ty: _, fields: _ } => {
                self.node("Expression::StructLiteral", _id.id).end();
            }
            Expression::TreeLiteral {
                path,
                arguments: _,
                elements: _,
            } => {
                self.node("Expression::TreeLiteral", _id.id)
                    .field_optional("path", path)
                    .end();
            }
            Expression::Parenthesized { expression: _ } => {
                self.node("Expression::Parenthesized", _id.id).end();
            }
            Expression::Unary {
                operator,
                expression: _,
            } => {
                self.node("Expression::Unary", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Reference {
                mutability,
                right: _,
            } => {
                self.node("Expression::Reference", _id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Expression::Dynamic {
                mutability,
                right: _,
            } => {
                self.node("Expression::Dynamic", _id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Expression::Member { left: _, path } => {
                self.node("Expression::Member", _id.id)
                    .field("path", path)
                    .end();
            }
            Expression::Index {
                position,
                left: _,
                index: _,
            } => {
                self.node("Expression::Index", _id.id)
                    .field("position", position)
                    .end();
            }
            Expression::Call {
                position,
                runtime,
                left: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::Call", _id.id)
                    .field("position", position)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Maybe { position, left: _ } => {
                self.node("Expression::Maybe", _id.id)
                    .field("position", position)
                    .end();
            }
            Expression::Must { position, left: _ } => {
                self.node("Expression::Must", _id.id)
                    .field("position", position)
                    .end();
            }
            Expression::Binary {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::Binary", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Assign {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::Assign", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Error => {
                self.node("Expression::Error", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_expression(dumper, _tree, _id, expression);
        });
    }

    fn visit_block(&mut self, _tree: &NodeTree, _id: NodeId<Block>, block: &Block) {
        self.node("Block", _id.id)
            .field("format", &block.format)
            .field_optional("label", &block.label)
            .end();
        self.with_depth(|dumper| {
            walk_block(dumper, _tree, _id, block);
        });
    }

    // ------------------------------------------------------------
    // Declarations
    // ------------------------------------------------------------

    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<crate::Definition>,
        definition: &crate::Definition,
    ) {
        match definition {
            Definition::Module {
                name,
                export,
                visibility,
                format,
                expressions: _,
                with_clauses: _,
                where_clauses: _,
            } => {
                self.node("Definition::Module", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .field("format", format)
                    .end();
            }
            Definition::Struct {
                name,
                visibility,
                style,
                export,
                super_types: _,
                representation_type: _,
                static_parameters: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                expressions: _,
            } => {
                self.node("Definition::Struct", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .field("style", style)
                    .end();
            }
            Definition::Enum {
                name,
                visibility,
                export,
                tag_type: _,
                static_parameters: _,
                super_types: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                expressions: _,
            } => {
                self.node("Definition::Enum", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .end();
            }
            Definition::Union {
                name,
                visibility,
                export,
                tag_type: _,
                representation_type: _,
                static_parameters: _,
                super_types: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                expressions: _,
            } => {
                self.node("Definition::Union", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .end();
            }
            Definition::Interface {
                name,
                visibility,
                export,
                super_types: _,
                static_parameters: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                expressions: _,
            } => {
                self.node("Definition::Interface", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .end();
            }
            Definition::Implement {
                export,
                visibility,
                static_parameters: _,
                target_type: _,
                super_types: _,
                with_clauses: _,
                where_clauses: _,
                expressions: _,
            } => {
                self.node("Definition::Implement", id.id)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Function {
                name,
                visibility,
                export,
                runtime,
                cardinality,
                accessor,
                style,
                static_parameters: _,
                self_parameter: _,
                dynamic_parameters: _,
                return_type: _,
                with_clauses: _,
                where_clauses: _,
                body: _,
            } => {
                self.node("Definition::Function", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .field("runtime", runtime)
                    .field("cardinality", cardinality)
                    .field_optional("accessor", accessor)
                    .field("style", style)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_definition(dumper, tree, id, definition);
        });
    }

    fn visit_variant_field(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<VariantField>,
        field: &VariantField,
    ) {
        match field {
            VariantField::Named {
                visibility,
                mutability,
                name,
                ty: _,
                default: _,
            } => {
                self.node("VariantField::Named", _id.id)
                    .field_optional("visibility", visibility)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }
            VariantField::Positional {
                visibility,
                mutability,
                ty: _,
                default: _,
            } => {
                self.node("VariantField::Positional", _id.id)
                    .field_optional("visibility", visibility)
                    .field_optional("mutability", mutability)
                    .end();
            }
            VariantField::Dynamic {
                visibility,
                mutability,
                name,
                ty: _,
                key: _,
                default: _,
            } => {
                self.node("VariantField::Dynamic", _id.id)
                    .field_optional("visibility", visibility)
                    .field_optional("mutability", mutability)
                    .field_optional("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_variant_field(dumper, _tree, _id, field);
        });
    }

    fn visit_enum_field(&mut self, _tree: &NodeTree, _id: NodeId<EnumField>, field: &EnumField) {
        self.node("EnumField", _id.id)
            .field("name", &field.name)
            .end();
        self.with_depth(|dumper| {
            walk_enum_field(dumper, _tree, _id, field);
        });
    }

    fn visit_union_field(&mut self, _tree: &NodeTree, _id: NodeId<UnionField>, field: &UnionField) {
        match field {
            UnionField::Unit { name, value: _ } => {
                self.node("UnionField::Unit", _id.id)
                    .field("name", name)
                    .end();
            }
            UnionField::Tuple {
                name,
                fields: _,
                value: _,
            } => {
                self.node("UnionField::Tuple", _id.id)
                    .field("name", name)
                    .end();
            }
            UnionField::Struct {
                name,
                fields: _,
                value: _,
            } => {
                self.node("UnionField::Struct", _id.id)
                    .field("name", name)
                    .end();
            }
        };
        self.with_depth(|dumper| {
            walk_union_field(dumper, _tree, _id, field);
        });
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    fn visit_with_clause(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<WithClause>,
        clause: &WithClause,
    ) {
        self.node("WithClause", _id.id)
            .field_optional("alias", &clause.alias)
            .end();
        self.with_depth(|dumper| {
            walk_with_clause(dumper, _tree, _id, clause);
        });
    }

    fn visit_where_clause(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<WhereClause>,
        clause: &WhereClause,
    ) {
        match clause {
            WhereClause::Assertion { left, .. } => {
                self.node("WhereClause::Assertion", _id.id)
                    .field("left", left)
                    .end();
            }
            WhereClause::Guard { .. } => {
                self.node("WhereClause::Guard", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_where_clause(dumper, _tree, _id, clause);
        });
    }

    fn visit_import_item(&mut self, _tree: &NodeTree, _id: NodeId<ImportItem>, item: &ImportItem) {
        self.node("UseItem", _id.id)
            .field("name", &item.name)
            .field_optional("alias", &item.alias)
            .end();
        self.with_depth(|dumper| {
            walk_import_item(dumper, _tree, _id, item);
        });
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_parameter(&mut self, _tree: &NodeTree, _id: NodeId<Parameter>, param: &Parameter) {
        match param {
            Parameter::Named {
                name,
                ty: _,
                default: _,
            } => {
                self.node("Parameter::Scalar", _id.id)
                    .field("name", name)
                    .end();
            }
            Parameter::Pattern {
                pattern: _,
                ty: _,
                default: _,
            } => {
                self.node("Parameter::Pattern", _id.id).end();
            }
            Parameter::Variadic { name, ty: _ } => {
                self.node("Parameter::Variadic", _id.id)
                    .field("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_parameter(dumper, _tree, _id, param);
        });
    }

    fn visit_argument(&mut self, _tree: &NodeTree, _id: NodeId<Argument>, arg: &Argument) {
        match arg {
            Argument::Named { name, value: _ } => {
                self.node("Argument::Named", _id.id)
                    .field("name", name)
                    .end();
            }
            Argument::NamedShorthand { name } => {
                self.node("Argument::NamedShorthand", _id.id)
                    .field("name", name)
                    .end();
            }
            Argument::NamedFunction { name, value: _ } => {
                self.node("Argument::NamedFunction", _id.id)
                    .field("name", name)
                    .end();
            }
            Argument::Positional { value: _ } => {
                self.node("Argument::Positional", _id.id).end();
            }
            Argument::Spread { value: _ } => {
                self.node("Argument::Spread", _id.id).end();
            }
            Argument::Dynamic {
                name,
                key: _,
                value: _,
            } => {
                self.node("Argument::Dynamic", _id.id)
                    .field_optional("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_argument(dumper, _tree, _id, arg);
        });
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    fn visit_match_case(&mut self, _tree: &NodeTree, _id: NodeId<MatchCase>, case: &MatchCase) {
        match case {
            MatchCase::Expression {
                pattern: _,
                body: _,
                guard: _,
            } => {
                self.node("MatchCase::Expression", _id.id).end();
            }
            MatchCase::Block {
                pattern: _,
                body: _,
                guard: _,
            } => {
                self.node("MatchCase::Block", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_match_case(dumper, _tree, _id, case);
        });
    }

    fn visit_pattern(&mut self, _tree: &NodeTree, _id: NodeId<Pattern>, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {
                self.node("Pattern::Wildcard", _id.id).end();
            }
            Pattern::Rest => {
                self.node("Pattern::Rest", _id.id).end();
            }
            Pattern::Maybe(_) => {
                self.node("Pattern::Unwrap", _id.id).end();
            }
            Pattern::Reference {
                mutability,
                right: _,
            } => {
                self.node("Pattern::Pointer", _id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Pattern::Binding {
                mutability,
                name,
                pattern: _,
            } => {
                self.node("Pattern::Binding", _id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }
            Pattern::Expression { value: _ } => {
                self.node("Pattern::Expression", _id.id).end();
            }
            Pattern::Range {
                start: _,
                end: _,
                is_inclusive,
            } => {
                self.node("Pattern::Range", _id.id)
                    .field("is_inclusive", is_inclusive)
                    .end();
            }
            Pattern::Tuple { ty: _, fields: _ } => {
                self.node("Pattern::Tuple", _id.id).end();
            }
            Pattern::Slice { fields: _ } => {
                self.node("Pattern::Slice", _id.id).end();
            }
            Pattern::Struct { ty: _, fields: _ } => {
                self.node("Pattern::Struct", _id.id).end();
            }
            Pattern::Union { patterns: _ } => {
                self.node("Pattern::Union", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_pattern(dumper, _tree, _id, pattern);
        });
    }

    fn visit_pattern_field(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<PatternField>,
        field: &PatternField,
    ) {
        match field {
            PatternField::Named {
                mutability,
                name,
                pattern: _,
                default: _,
            } => {
                self.node("PatternField::Named", _id.id)
                    .field("name", name)
                    .field_optional("mutability", mutability)
                    .end();
            }
            PatternField::NamedAlias {
                mutability,
                name,
                alias,
                default: _,
            } => {
                self.node("PatternField::NamedAlias", _id.id)
                    .field("name", name)
                    .field("alias", alias)
                    .field_optional("mutability", mutability)
                    .end();
            }
            PatternField::Positional { pattern: _ } => {
                self.node("PatternField::Positional", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_pattern_field(dumper, _tree, _id, field);
        });
    }

    // ------------------------------------------------------------
    // Annotations
    // ------------------------------------------------------------

    fn visit_annotation(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<Annotation>,
        annotation: &Annotation,
    ) {
        match annotation {
            Annotation::Blank { node: _, position } => {
                self.node("Annotation::Blank", _id.id)
                    .field("position", position)
                    .end();
            }
            Annotation::Doc { node: _, position } => {
                self.node("Annotation::Doc", _id.id)
                    .field("position", position)
                    .end();
            }
            Annotation::Comment { node: _, position } => {
                self.node("Annotation::Comment", _id.id)
                    .field("position", position)
                    .end();
            }
            Annotation::Tag { node: _, position } => {
                self.node("Annotation::Tag", _id.id)
                    .field("position", position)
                    .end();
            }
            Annotation::Decorator { node: _, position } => {
                self.node("Annotation::Decorator", _id.id)
                    .field("position", position)
                    .end();
            }
        };
        self.with_depth(|dumper| {
            walk_annotation(dumper, _tree, _id, annotation);
        });
    }

    fn visit_blank(&mut self, _tree: &NodeTree, _id: NodeId<Blank>, blank: &Blank) {
        self.node("Blank", _id.id)
            .field("lines", &blank.lines)
            .end();
        self.with_depth(|dumper| {
            walk_blank(dumper, _tree, _id, blank);
        });
    }

    fn visit_doc(&mut self, _tree: &NodeTree, _id: NodeId<Doc>, doc: &Doc) {
        let string = truncate_string(self.strings.get(doc.string), 40, "...");
        self.node("Doc", _id.id)
            .field("string", &string.as_ref())
            .field("style", &doc.style)
            .end();
        self.with_depth(|dumper| {
            walk_doc(dumper, _tree, _id, doc);
        });
    }

    fn visit_comment(&mut self, _tree: &NodeTree, _id: NodeId<Comment>, comment: &Comment) {
        let string = truncate_string(self.strings.get(comment.string), 40, "...");
        self.node("Comment", _id.id)
            .field("string", &string.as_ref())
            .field("style", &comment.style)
            .end();
        self.with_depth(|dumper| {
            walk_comment(dumper, _tree, _id, comment);
        });
    }

    fn visit_tag(&mut self, _tree: &NodeTree, _id: NodeId<Tag>, tag: &Tag) {
        self.node("Tag", _id.id)
            .field("receiver", &tag.receiver)
            .end();
        self.with_depth(|dumper| {
            walk_tag(dumper, _tree, _id, tag);
        });
    }

    fn visit_decorator(&mut self, _tree: &NodeTree, _id: NodeId<Decorator>, decorator: &Decorator) {
        self.node("Decorator", _id.id)
            .field("receiver", &decorator.receiver)
            .end();
        self.with_depth(|dumper| {
            walk_decorator(dumper, _tree, _id, decorator);
        });
    }
}

/// Truncate a string to n characters (with newlines replaced).
fn truncate_string<'a>(string: &'a str, n: usize, newline_replacement: &str) -> Cow<'a, str> {
    if string.len() > n || string.contains('\n') {
        let truncated = if string.len() > n {
            &string[..n]
        } else {
            string
        };
        Cow::Owned(format!(
            "{} ...",
            truncated.replace('\n', newline_replacement)
        ))
    } else {
        Cow::Borrowed(string)
    }
}
