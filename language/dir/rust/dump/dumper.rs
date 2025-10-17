#![allow(clippy::match_like_matches_macro)]

use dyst_container::SmallVec;

use crate::*;

use std::ops::Range;

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

    fn end_postfix(&mut self, node_id: u32) {
        let (source_id, source_ast_id) = self.dumper.tree.get_source(node_id);
        let source_id = source_id.0;
        if let Some(source_ast_id) = source_ast_id {
            self.dumper.write_str(
                format!(" :{node_id} [{source_id:?}/{source_ast_id}]").as_str(),
                Some(Color::White),
            );
        } else {
            self.dumper.write_str(
                format!(" :{node_id} [{source_id:?}]").as_str(),
                Some(Color::White),
            );
        }
    }

    /// Finish node and mark the struct as non-exhaustive (with a ..)
    pub fn end_non_exhaustive(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(", .. }", Some(Color::White));
        } else {
            self.dumper.write_str(" { .. }", Some(Color::White));
        }
        if let Some(node_id) = self.node_id {
            self.end_postfix(node_id);
        }
        self
    }

    /// Finish node and close the struct as exhaustive.
    pub fn end(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(" }", Some(Color::White));
        }
        if let Some(node_id) = self.node_id {
            self.end_postfix(node_id);
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

/// Dump a SmallVec<T, N> as a slice.
impl<T: Dump, const N: usize> Dump for SmallVec<T, N> {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        self.as_slice().dump(dumper)
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

/// Dump a Visibility as a string.
impl Dump for Visibility {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a ExportMode as a string.
impl Dump for ExportMode {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ExportMode::Item => dumper.write_str("ExportMode::Item", Some(Color::Yellow)),
            ExportMode::Default => dumper.write_str("ExportMode::Default", Some(Color::Yellow)),
        }
    }
}

/// Dump a Runtime as a string.
impl Dump for Runtime {
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

/// Dump an EmbeddedDefinition as a string.
impl Dump for EmbeddedDefinition {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            EmbeddedDefinition::Super { ty: _ } => {
                dumper.object("EmbeddedDefinition::Super").end();
            }
            EmbeddedDefinition::Include { ty: _ } => {
                dumper.object("EmbeddedDefinition::Include").end();
            }
        }
    }
}

/// Dump an ArgumentSlot as a string.
impl Dump for ArgumentSlot {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ArgumentSlot::Parameter { parameter: _ } => {
                dumper.object("ArgumentSlot::Parameter").end();
            }
            ArgumentSlot::Field { field: _ } => {
                dumper.object("ArgumentSlot::Field").end();
            }
        }
    }
}

/// Dump a SelfParameter as a string.
impl Dump for SelfParameter {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("SelfParameter")
            .field("mutability", &self.mutability)
            .field("is_reference", &self.is_reference)
            .end();
    }
}

/// Dump a LoopSource as a string.
impl Dump for LoopSource {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a MatchSource as a string.
impl Dump for MatchSource {
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

/// Dump a Destination as a structured representation.
impl Dump for Destination {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Destination::UnevaluatedString { label } => {
                dumper
                    .object("Destination::UnevaluatedString")
                    .field("label", label)
                    .end();
            }
            Destination::Definition { .. } => {
                dumper.object("Destination::Definition").end();
            }
            Destination::Error => {
                dumper.object("Destination::Error").end();
            }
        }
    }
}

/// Dump an AnnotationPosition as a string.
impl Dump for AnnotationPosition {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump a PathBase as a string.
impl Dump for PathBase {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            PathBase::SelfValue => dumper.write_str("PathBase::SelfValue", Some(Color::Yellow)),
            PathBase::SelfType => dumper.write_str("PathBase::SelfType", Some(Color::Yellow)),
            PathBase::Module => dumper.write_str("PathBase::Module", Some(Color::Yellow)),
            PathBase::Package => dumper.write_str("PathBase::Package", Some(Color::Yellow)),
        }
    }
}

/// Dump a Path as a structured representation.
impl Dump for Path {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Path::UnevaluatedBase { base } => {
                dumper
                    .object("Path::UnevaluatedBase")
                    .field("base", base)
                    .end();
            }
            Path::UnevaluatedRelativeString { base, segments } => {
                dumper
                    .object("Path::UnevaluatedRelativeString")
                    .field("base", base)
                    .value(segments)
                    .end();
            }
            Path::UnevaluatedAbsoluteString { segments } => {
                dumper
                    .object("Path::UnevaluatedAbsoluteString")
                    .value(segments)
                    .end();
            }

            Path::Intrinsic { intrinsic } => {
                dumper
                    .object("Path::Intrinsic")
                    .field("intrinsic", intrinsic)
                    .end();
            }
            Path::Definition { .. } => {
                dumper.object("Path::Definition").end();
            }
            Path::Error => {
                dumper.object("Path::Error").end();
            }
        }
    }
}

/// Dump an Intrinsic. This cannot occur yet, but keep the match exhaustive.
impl Dump for Intrinsic {
    fn dump<'a>(&self, _dumper: &mut Dumper<'a>) {
        match *self {}
    }
}

/// Dump an ImportTarget as a string.
impl Dump for ImportTarget {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ImportTarget::Virtual(path) => {
                dumper.object("ImportTarget::Virtual").value(path).end();
            }
            ImportTarget::Physical(string) => {
                dumper.object("ImportTarget::Physical").value(string).end();
            }
        }
    }
}

/// Dump an IntType as a structured representation.
impl Dump for IntType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            IntType::Int8 => dumper.object("IntType::Int8").end(),
            IntType::Int16 => dumper.object("IntType::Int16").end(),
            IntType::Int32 => dumper.object("IntType::Int32").end(),
            IntType::Int64 => dumper.object("IntType::Int64").end(),
            IntType::Int128 => dumper.object("IntType::Int128").end(),
            IntType::Int256 => dumper.object("IntType::Int256").end(),
            IntType::Uint8 => dumper.object("IntType::Uint8").end(),
            IntType::Uint16 => dumper.object("IntType::Uint16").end(),
            IntType::Uint32 => dumper.object("IntType::Uint32").end(),
            IntType::Uint64 => dumper.object("IntType::Uint64").end(),
            IntType::Uint128 => dumper.object("IntType::Uint128").end(),
            IntType::Uint256 => dumper.object("IntType::Uint256").end(),
            IntType::Variable { width, is_signed } => dumper
                .object("IntType::Variable")
                .field("width", width)
                .field("is_signed", is_signed)
                .end(),
        };
    }
}

/// Dump a FloatType as a structured representation.
impl Dump for FloatType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            FloatType::Float16 => dumper.object("FloatType::Float16").end(),
            FloatType::Float32 => dumper.object("FloatType::Float32").end(),
            FloatType::Float64 => dumper.object("FloatType::Float64").end(),
            FloatType::Float80 => dumper.object("FloatType::Float80").end(),
            FloatType::Float128 => dumper.object("FloatType::Float128").end(),
            FloatType::Variable { width } => dumper
                .object("FloatType::Variable")
                .field("width", width)
                .end(),
        };
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

/// Dump a TypeLiteral as a structured representation.
impl Dump for PrimitiveType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            PrimitiveType::Boolean => {
                dumper.object("TypeLiteral::Boolean").end();
            }
            PrimitiveType::Character => {
                dumper.object("TypeLiteral::Character").end();
            }
            PrimitiveType::String => {
                dumper.object("TypeLiteral::String").end();
            }
            PrimitiveType::Number => {
                dumper.object("TypeLiteral::Number").end();
            }
            PrimitiveType::Int(int_type) => {
                dumper.object("TypeLiteral::Int").value(int_type).end();
            }
            PrimitiveType::Float(float_type) => {
                dumper.object("TypeLiteral::Float").value(float_type).end();
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
            TypeLiteral::Void => {
                dumper.object("TypeLiteral::Void").end();
            }
            TypeLiteral::Null => {
                dumper.object("TypeLiteral::Null").end();
            }
            TypeLiteral::Primitive(primitive) => {
                dumper
                    .object("TypeLiteral::Primitive")
                    .value(primitive)
                    .end();
            }
            TypeLiteral::Composite(composite) => {
                dumper
                    .object("TypeLiteral::Composite")
                    .value(composite)
                    .end();
            }
            TypeLiteral::ScalarLiteral(scalar_literal) => {
                dumper
                    .object("TypeLiteral::ScalarLiteral")
                    .value(scalar_literal)
                    .end();
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
                    .field("value", value)
                    .end();
            }
            ScalarLiteral::Byte(value) => {
                dumper
                    .object("ScalarLiteral::Byte")
                    .field("value", value)
                    .end();
            }
            ScalarLiteral::Integer(value) => {
                dumper
                    .object("ScalarLiteral::Integer")
                    .field("value", value)
                    .end();
            }
            ScalarLiteral::Float(value) => {
                dumper
                    .object("ScalarLiteral::Float")
                    .field("value", value)
                    .end();
            }
            ScalarLiteral::Character(value) => {
                dumper
                    .object("ScalarLiteral::Character")
                    .field("value", value)
                    .end();
            }
            ScalarLiteral::String(value) => {
                dumper
                    .object("ScalarLiteral::String")
                    .field("value", value)
                    .end();
            }
            ScalarLiteral::ByteString(value) => {
                dumper
                    .object("ScalarLiteral::ByteString")
                    .field("value", value)
                    .end();
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
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Block { block: _ } => {
                self.node("Expression::Block", id.id).end();
            }
            Expression::Definition { definition: _ } => {
                self.node("Expression::Definition", id.id).end();
            }

            Expression::With {
                clauses: _,
                body: _,
            } => {
                self.node("Expression::With", id.id).end();
            }
            Expression::Import { items: _ } => {
                self.node("Expression::Use", id.id).end();
            }
            Expression::Export { mode, items: _ } => {
                self.node("Expression::Export", id.id)
                    .field("mode", mode)
                    .end();
            }
            Expression::Let {
                mutability,
                visibility,
                pattern: _,
                ty: _,
                value: _,
            } => {
                self.node("Expression::Let", id.id)
                    .field("mutability", mutability)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Expression::Type {
                name,
                static_parameters: _,
                visibility,
                value: _,
            } => {
                self.node("Expression::Type", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }

            Expression::Unary {
                operator,
                expression: _,
            } => {
                self.node("Expression::Unary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Reference {
                mutability,
                right: _,
            } => {
                self.node("Expression::Reference", id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Expression::Dynamic {
                mutability,
                right: _,
            } => {
                self.node("Expression::Dynamic", id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Expression::Binary {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::Binary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::AssignDirect { left: _, right: _ } => {
                self.node("Expression::AssignDirect", id.id).end();
            }
            Expression::AssignBinary {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::AssignBinary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Member { left: _, path } => {
                self.node("Expression::Member", id.id)
                    .field("path", path)
                    .end();
            }
            Expression::Call {
                runtime,
                left: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::Call", id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Index { left: _, right: _ } => {
                self.node("Expression::Index", id.id).end();
            }
            Expression::Maybe { left: _ } => {
                self.node("Expression::Maybe", id.id).end();
            }
            Expression::Must { left: _ } => {
                self.node("Expression::Must", id.id).end();
            }

            Expression::Path { path } => {
                self.node("Expression::Path", id.id)
                    .field("path", path)
                    .end();
            }
            Expression::ScalarLiteral { value } => {
                self.node("Expression::ScalarLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            Expression::TypeLiteral { value } => {
                self.node("Expression::TypeLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            Expression::RangeLiteral {
                start: _,
                end: _,
                is_inclusive,
            } => {
                self.node("Expression::RangeLiteral", id.id)
                    .field("is_inclusive", is_inclusive)
                    .end();
            }
            Expression::ArrayLiteral { elements: _ } => {
                self.node("Expression::ArrayLiteral", id.id).end();
            }
            Expression::TupleLiteral { ty: _, elements: _ } => {
                self.node("Expression::TupleLiteral", id.id).end();
            }
            Expression::StructLiteral { ty: _, fields: _ } => {
                self.node("Expression::StructLiteral", id.id).end();
            }
            Expression::TreeLiteral {
                path,
                arguments: _,
                elements: _,
            } => {
                self.node("Expression::TreeLiteral", id.id)
                    .field_optional("path", path)
                    .end();
            }
            Expression::If {
                runtime,
                condition: _,
                then_expression: _,
                else_expression: _,
            } => {
                self.node("Expression::If", id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Loop {
                runtime,
                condition: _,
                body: _,
                source,
            } => {
                self.node("Expression::Loop", id.id)
                    .field_optional("runtime", runtime)
                    .field("source", source)
                    .end();
            }
            Expression::Match {
                runtime,
                value: _,
                cases: _,
                source,
            } => {
                self.node("Expression::Match", id.id)
                    .field_optional("runtime", runtime)
                    .field("source", source)
                    .end();
            }
            Expression::Break {
                destination,
                value: _,
            } => {
                self.node("Expression::Break", id.id)
                    .field("destination", destination)
                    .end();
            }
            Expression::Continue { destination } => {
                self.node("Expression::Continue", id.id)
                    .field("destination", destination)
                    .end();
            }
            Expression::Defer { expression: _ } => {
                self.node("Expression::Defer", id.id).end();
            }
            Expression::Return { value: _ } => {
                self.node("Expression::Return", id.id).end();
            }
            Expression::Error => {
                self.node("Expression::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_expression(dumper, tree, id, expression);
        });
    }

    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, block: &Block) {
        self.node("Block", id.id).end();
        self.with_depth(|dumper| {
            walk_block(dumper, tree, id, block);
        });
    }

    // ------------------------------------------------------------
    // Definitions
    // ------------------------------------------------------------

    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Definition>,
        definition: &Definition,
    ) {
        match definition {
            Definition::Intrinsic { intrinsic } => {
                self.node("Definition::Intrinsic", id.id)
                    .field("intrinsic", intrinsic)
                    .end();
            }
            Definition::Import { items: _ } => {
                self.node("Definition::Import", id.id).end();
            }
            Definition::Export { mode, items: _ } => {
                self.node("Definition::Export", id.id)
                    .field("mode", mode)
                    .end();
            }
            Definition::Let {
                name,
                export,
                visibility,
            } => {
                self.node("Definition::Let", id.id)
                    .field("name", name)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Type {
                name,
                export,
                visibility,
                value: _,
            } => {
                self.node("Definition::Type", id.id)
                    .field_optional("name", name)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Module {
                name,
                export,
                visibility,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Module", id.id)
                    .field_optional("name", name)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Struct {
                name,
                export,
                visibility,
                static_parameters: _,
                embedded_definitions: _,
                variant: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Struct", id.id)
                    .field_optional("name", name)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Enum {
                name,
                export,
                visibility,
                static_parameters: _,
                embedded_definitions: _,
                variants: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Enum", id.id)
                    .field_optional("name", name)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Union {
                name,
                export,
                visibility,
                static_parameters: _,
                embedded_definitions: _,
                variants: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Union", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("export", export)
                    .end();
            }
            Definition::Interface {
                name,
                export,
                visibility,
                static_parameters: _,
                embedded_definitions: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                definitions: _,
            } => {
                self.node("Definition::Interface", id.id)
                    .field_optional("name", name)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Function {
                name,
                export,
                visibility,
                runtime,
                style,
                static_parameters: _,
                self_parameter,
                dynamic_parameters: _,
                return_type: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
                body: _,
            } => {
                self.node("Definition::Function", id.id)
                    .field("runtime", runtime)
                    .field("style", style)
                    .field_optional("name", name)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .field_optional("self_parameter", self_parameter)
                    .end();
            }
            Definition::Implement {
                export,
                visibility,
                static_parameters: _,
                target_type: _,
                super_type: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Implement", id.id)
                    .field_optional("export", export)
                    .field_optional("visibility", visibility)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_definition(dumper, tree, id, definition);
        });
    }

    // ------------------------------------------------------------
    // Types
    // ------------------------------------------------------------

    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, ty: &Type) {
        match ty {
            Type::Scalar(scalar) => {
                self.node("Type::Scalar", id.id)
                    .field("scalar", scalar)
                    .end();
            }
            Type::Definition(_) => {
                self.node("Type::Definition", id.id).end();
            }

            Type::Not(_) => {
                self.node("Type::Not", id.id).end();
            }
            Type::Maybe(_) => {
                self.node("Type::Maybe", id.id).end();
            }
            Type::Must(_) => {
                self.node("Type::Must", id.id).end();
            }
            Type::Reference {
                mutability,
                target: _,
            } => {
                self.node("Type::Reference", id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Type::Dynamic {
                mutability,
                target: _,
            } => {
                self.node("Type::Dynamic", id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }

            Type::Range {
                start: _,
                end: _,
                is_inclusive,
            } => {
                self.node("Type::Range", id.id)
                    .field("is_inclusive", is_inclusive)
                    .end();
            }
            Type::Array {
                element: _,
                count: _,
            } => {
                self.node("Type::Array", id.id).end();
            }
            Type::Slice { element: _ } => {
                self.node("Type::Slice", id.id).end();
            }
            Type::Tuple(_) => {
                self.node("Type::Tuple", id.id).end();
            }
            Type::Intersection(_) => {
                self.node("Type::Intersection", id.id).end();
            }

            Type::UnevaluatedExpression(_) => {
                self.node("Type::UnevaluatedExpression", id.id).end();
            }
            Type::UnevaluatedSelf => {
                self.node("Type::UnevaluatedSelf", id.id).end();
            }

            Type::Error => {
                self.node("Type::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_type(dumper, tree, id, ty);
        });
    }

    fn visit_variant(&mut self, tree: &NodeTree, id: NodeId<Variant>, variant: &Variant) {
        match variant {
            Variant::Struct {
                name,
                ty: _,
                fields: _,
                value: _,
            } => {
                self.node("Variant::Struct", id.id)
                    .field_optional("name", name)
                    .end();
            }
            Variant::Tuple {
                name,
                ty: _,
                fields: _,
                value: _,
            } => {
                self.node("Variant::Tuple", id.id)
                    .field_optional("name", name)
                    .end();
            }
            Variant::Unit {
                name,
                ty: _,
                value: _,
            } => {
                self.node("Variant::Unit", id.id)
                    .field_optional("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_variant(dumper, tree, id, variant);
        });
    }

    fn visit_variant_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<VariantField>,
        variant_field: &VariantField,
    ) {
        match variant_field {
            VariantField::Named {
                name,
                ty: _,
                default: _,
            } => {
                self.node("VariantField::Named", id.id)
                    .field("name", name)
                    .end();
            }
            VariantField::Positional { ty: _, default: _ } => {
                self.node("VariantField::Positional", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_variant_field(dumper, tree, id, variant_field);
        });
    }

    fn visit_where_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WhereClause>,
        where_clause: &WhereClause,
    ) {
        match where_clause {
            WhereClause::Assertion { left, right: _ } => {
                self.node("WhereClause::Assertion", id.id)
                    .field("left", left)
                    .end();
            }
            WhereClause::Guard { guard: _ } => {
                self.node("WhereClause::Guard", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_where_clause(dumper, tree, id, where_clause);
        });
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    fn visit_with_clause(
        &mut self,
        tree: &NodeTree,
        id: NodeId<WithClause>,
        with_clause: &WithClause,
    ) {
        self.node("WithClause", id.id)
            .field_optional("alias", &with_clause.alias)
            .end();
        self.with_depth(|dumper| {
            walk_with_clause(dumper, tree, id, with_clause);
        });
    }

    fn visit_import_item(
        &mut self,
        tree: &NodeTree,
        id: NodeId<ImportItem>,
        import_item: &ImportItem,
    ) {
        match import_item {
            ImportItem::Glob { target, alias } => {
                self.node("ImportItem::Glob", id.id)
                    .field("target", target)
                    .field_optional("alias", alias)
                    .end();
            }
            ImportItem::Scalar {
                target,
                name,
                alias,
            } => {
                self.node("ImportItem::Scalar", id.id)
                    .field_optional("target", target)
                    .field("name", name)
                    .field_optional("alias", alias)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_import_item(dumper, tree, id, import_item);
        });
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        match parameter {
            Parameter::Scalar {
                name,
                ty: _,
                default: _,
            } => {
                self.node("Parameter::Scalar", id.id)
                    .field("name", name)
                    .end();
            }
            Parameter::Variadic { name, ty: _ } => {
                self.node("Parameter::Variadic", id.id)
                    .field("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_parameter(dumper, tree, id, parameter);
        });
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        match argument {
            Argument::UnevaluatedNamed { name, value: _ } => {
                self.node("Argument::UnevaluatedNamed", id.id)
                    .field("name", name)
                    .end();
            }
            Argument::UnevaluatedPositional { value: _ } => {
                self.node("Argument::UnevaluatedPositional", id.id).end();
            }
            Argument::UnevaluatedSpread { value: _ } => {
                self.node("Argument::UnevaluatedSpread", id.id).end();
            }
            Argument::Direct {
                name,
                slot,
                value: _,
            } => {
                self.node("Argument::Direct", id.id)
                    .field("name", name)
                    .field("slot", slot)
                    .end();
            }
            Argument::Spread { slot, value: _ } => {
                self.node("Argument::Spread", id.id)
                    .field("slot", slot)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_argument(dumper, tree, id, argument);
        });
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {
                self.node("Pattern::Wildcard", id.id).end();
            }
            Pattern::Rest => {
                self.node("Pattern::Rest", id.id).end();
            }
            Pattern::Maybe(_) => {
                self.node("Pattern::Maybe", id.id).end();
            }
            Pattern::Reference {
                right: _,
                mutability,
            } => {
                self.node("Pattern::Reference", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Pattern::Binding {
                mutability,
                name,
                pattern: _,
            } => {
                self.node("Pattern::Binding", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }
            Pattern::Expression { value: _ } => {
                self.node("Pattern::Expression", id.id).end();
            }
            Pattern::Range {
                start: _,
                end: _,
                is_inclusive,
            } => {
                self.node("Pattern::Range", id.id)
                    .field("is_inclusive", is_inclusive)
                    .end();
            }
            Pattern::Tuple { ty: _, fields: _ } => {
                self.node("Pattern::Tuple", id.id).end();
            }
            Pattern::Slice { fields: _ } => {
                self.node("Pattern::Slice", id.id).end();
            }
            Pattern::Struct { ty: _, fields: _ } => {
                self.node("Pattern::Struct", id.id).end();
            }
            Pattern::Union { patterns: _ } => {
                self.node("Pattern::Union", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_pattern(dumper, tree, id, pattern);
        });
    }

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        match pattern_field {
            PatternField::Named {
                name,
                pattern: _,
                mutability,
            } => {
                self.node("PatternField::Named", id.id)
                    .field("name", name)
                    .field_optional("mutability", mutability)
                    .end();
            }
            PatternField::NamedAlias {
                name,
                alias,
                mutability,
            } => {
                self.node("PatternField::NamedAlias", id.id)
                    .field("name", name)
                    .field("alias", alias)
                    .field_optional("mutability", mutability)
                    .end();
            }
            PatternField::Positional { pattern: _ } => {
                self.node("PatternField::Positional", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_pattern_field(dumper, tree, id, pattern_field);
        });
    }

    fn visit_match_case(&mut self, tree: &NodeTree, id: NodeId<MatchCase>, match_case: &MatchCase) {
        match match_case {
            MatchCase::Expression {
                pattern: _,
                body: _,
                guard: _,
            } => {
                self.node("MatchCase::Expression", id.id).end();
            }
            MatchCase::Block {
                pattern: _,
                body: _,
                guard: _,
            } => {
                self.node("MatchCase::Block", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_match_case(dumper, tree, id, match_case);
        });
    }

    // ------------------------------------------------------------
    // Annotations
    // ------------------------------------------------------------

    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
        annotation: &Annotation,
    ) {
        match annotation {
            Annotation::Doc { position, string } => {
                self.node("Annotation::Doc", id.id)
                    .field("position", position)
                    .field("string", string)
                    .end();
            }
            Annotation::Comment { position, string } => {
                self.node("Annotation::Comment", id.id)
                    .field("position", position)
                    .field("string", string)
                    .end();
            }
            Annotation::Tag {
                position,
                receiver,
                arguments: _,
            } => {
                self.node("Annotation::Tag", id.id)
                    .field("position", position)
                    .field("receiver", receiver)
                    .end();
            }
            Annotation::Decorator {
                position,
                receiver,
                arguments: _,
            } => {
                self.node("Annotation::Decorator", id.id)
                    .field("position", position)
                    .field("receiver", receiver)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_annotation(dumper, tree, id, annotation);
        });
    }
}
