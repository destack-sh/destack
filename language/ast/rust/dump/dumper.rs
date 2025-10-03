//! The Dumper is a helper for ugly-printing AST nodes for debugging and inspection.
//! Unlike the pretty Printer, Dumper makes no attempt to look like source code;
//!  instead, Dumper is optimized for checking parse trees.
//! ```

#![allow(clippy::match_like_matches_macro)]

use std::borrow::Cow;

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
    /// The path pool.
    pub paths: &'a PathPool,
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
    pub fn new(
        strings: &'a StringPool,
        paths: &'a PathPool,
        tree: &'a NodeTree,
        options: DumperOptions,
    ) -> Self {
        Self {
            strings,
            paths,
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
        self.buffer
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
    pub fn write_path_id(&mut self, id: PathId) {
        let path = self.paths.get(id);
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

    /// Finish node and mark the struct as non-exhaustive (with a ..)
    pub fn end_non_exhaustive(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(", .. }", Some(Color::White));
        } else {
            self.dumper.write_str(" { .. }", Some(Color::White));
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

/// Dump a PathId as a string.
impl Dump for PathId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_path_id(*self);
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

/// Dump a StructStyle as a string.
impl Dump for StructStyle {
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
            CompositeType::Trait => {
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
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Byte(value) => {
                dumper
                    .object("ScalarLiteral::Byte")
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Integer(value) => {
                dumper
                    .object("ScalarLiteral::Integer")
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Float(value) => {
                dumper
                    .object("ScalarLiteral::Float")
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Character(value) => {
                dumper
                    .object("ScalarLiteral::Character")
                    .field("value", &value.to_string().as_str())
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
                dumper
                    .object("TypeLiteral::Int")
                    .field("type", int_type)
                    .end();
            }
            TypeLiteral::Float(float_type) => {
                dumper
                    .object("TypeLiteral::Float")
                    .field("type", float_type)
                    .end();
            }
            TypeLiteral::Composite(composite_type) => {
                dumper
                    .object("TypeLiteral::Composite")
                    .field("type", composite_type)
                    .end();
            }
            TypeLiteral::Self_ => {
                dumper.object("TypeLiteral::Self_").end();
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
        for annotation in annotations {
            self.visit_annotation(tree, annotation, tree.get(annotation));
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
            Expression::With { .. } => {
                self.node("Expression::With", _id.id).end();
            }
            Expression::Use { visibility, .. } => {
                self.node("Expression::Use", _id.id)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Expression::Let {
                mutability,
                visibility,
                ..
            } => {
                self.node("Expression::Let", _id.id)
                    .field("mutability", mutability)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Expression::If { runtime, .. } => {
                self.node("Expression::If", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::While { runtime, .. } => {
                self.node("Expression::While", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::For { runtime, .. } => {
                self.node("Expression::For", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Loop { runtime, .. } => {
                self.node("Expression::Loop", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Try { runtime, .. } => {
                self.node("Expression::Try", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Match { runtime, .. } => {
                self.node("Expression::Match", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Break { label, .. } => {
                self.node("Expression::Break", _id.id)
                    .field_optional("label", label)
                    .end();
            }
            Expression::Continue { label } => {
                self.node("Expression::Continue", _id.id)
                    .field_optional("label", label)
                    .end();
            }
            Expression::Defer { .. } => {
                self.node("Expression::Defer", _id.id).end();
            }
            Expression::Return { .. } => {
                self.node("Expression::Return", _id.id).end();
            }
            Expression::Path {
                path,
                static_arguments: _,
            } => {
                self.node("Expression::Path", _id.id)
                    .field("path", path)
                    .end();
            }
            Expression::ScalarLiteral(lit) => {
                self.node("Expression::ScalarLiteral", _id.id)
                    .field("value", lit)
                    .end();
            }
            Expression::TypeLiteral(lit) => {
                self.node("Expression::TypeLiteral", _id.id)
                    .field("value", lit)
                    .end();
            }
            Expression::RangeLiteral { is_inclusive, .. } => {
                self.node("Expression::RangeLiteral", _id.id)
                    .field("is_inclusive", is_inclusive)
                    .end();
            }
            Expression::ArrayLiteral { .. } => {
                self.node("Expression::ArrayLiteral", _id.id).end();
            }
            Expression::TupleLiteral { .. } => {
                self.node("Expression::TupleLiteral", _id.id).end();
            }
            Expression::StructLiteral { .. } => {
                self.node("Expression::StructLiteral", _id.id).end();
            }
            Expression::Parenthesized { .. } => {
                self.node("Expression::Parenthesized", _id.id).end();
            }
            Expression::Unary { operator, .. } => {
                self.node("Expression::Unary", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Reference { mutability, .. } => {
                self.node("Expression::Reference", _id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Expression::Member { path, .. } => {
                self.node("Expression::Member", _id.id)
                    .field("path", path)
                    .end();
            }
            Expression::Index { .. } => {
                self.node("Expression::Index", _id.id).end();
            }
            Expression::Call { runtime, .. } => {
                self.node("Expression::Call", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            Expression::Maybe(_) => {
                self.node("Expression::Maybe", _id.id).end();
            }
            Expression::Must(_) => {
                self.node("Expression::Must", _id.id).end();
            }
            Expression::Binary { operator, .. } => {
                self.node("Expression::Binary", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Assign { operator, .. } => {
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
                format,
                name,
                visibility,
                ..
            } => {
                self.node("Definition::Module", id.id)
                    .field("format", format)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Struct {
                name,
                visibility,
                style,
                ..
            } => {
                self.node("Definition::Struct", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field("style", style)
                    .end();
            }
            Definition::Enum {
                name, visibility, ..
            } => {
                self.node("Definition::Enum", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Union {
                name, visibility, ..
            } => {
                self.node("Definition::Union", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Trait {
                name, visibility, ..
            } => {
                self.node("Definition::Trait", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Implement { .. } => {
                self.node("Definition::Implement", id.id).end();
            }
            Definition::Function {
                name,
                visibility,
                runtime,
                style,
                ..
            } => {
                self.node("Definition::Function", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field("runtime", runtime)
                    .field("style", style)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_definition(dumper, tree, id, definition);
        });
    }

    fn visit_struct_field(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<StructField>,
        field: &StructField,
    ) {
        self.node("StructField", _id.id)
            .field("name", &field.name)
            .end();
        self.with_depth(|dumper| {
            walk_struct_field(dumper, _tree, _id, field);
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
        self.node("UnionField", _id.id)
            .field("name", &field.name)
            .end();
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
        match clause {
            WithClause::Declaration { target: _ } => {
                self.node("WithClause::Declaration", _id.id).end();
            }
            WithClause::Assertion {
                target: _,
                assertion: _,
            } => {
                self.node("WithClause::Assertion", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_with_clause(dumper, _tree, _id, clause);
        });
    }

    fn visit_use_clause(&mut self, _tree: &NodeTree, _id: NodeId<UseClause>, clause: &UseClause) {
        self.node("UseClause", _id.id)
            .field_optional("alias", &clause.alias)
            .end();
        self.with_depth(|dumper| {
            walk_use_clause(dumper, _tree, _id, clause);
        });
    }

    fn visit_use_item(&mut self, _tree: &NodeTree, _id: NodeId<UseItem>, item: &UseItem) {
        self.node("UseItem", _id.id)
            .field("name", &item.name)
            .field_optional("alias", &item.alias)
            .end();
        self.with_depth(|dumper| {
            walk_use_item(dumper, _tree, _id, item);
        });
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_parameter(&mut self, _tree: &NodeTree, _id: NodeId<Parameter>, param: &Parameter) {
        self.node("Parameter", _id.id)
            .field("name", &param.name)
            .end();
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
            Argument::Positional { value: _ } => {
                self.node("Argument::Positional", _id.id).end();
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
                target: _,
                mutability,
            } => {
                self.node("Pattern::Pointer", _id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Pattern::ScalarLiteral(_node) => {
                self.node("Pattern::Literal", _id.id).end();
            }
            Pattern::Binding { name } => {
                self.node("Pattern::Binding", _id.id)
                    .field("name", name)
                    .end();
            }
            Pattern::Path(path) => {
                self.node("Pattern::Path", _id.id).field("path", path).end();
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
            Pattern::Tuple { path, fields: _ } => {
                self.node("Pattern::Tuple", _id.id)
                    .field_optional("path", path)
                    .end();
            }
            Pattern::Slice { fields: _ } => {
                self.node("Pattern::Slice", _id.id).end();
            }
            Pattern::Struct {
                r#type: _,
                fields: _,
            } => {
                self.node("Pattern::Struct", _id.id).end();
            }
            Pattern::Union { fields: _ } => {
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
                name,
                pattern: _,
                mutability,
            } => {
                self.node("PatternField::Named", _id.id)
                    .field("name", name)
                    .field_optional("mutability", mutability)
                    .end();
            }
            PatternField::NamedAlias {
                name,
                alias,
                mutability,
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
            Annotation::Blank { position, .. } => {
                self.node("Annotation::Blank", _id.id)
                    .field("position", position)
                    .end();
            }
            Annotation::Doc { position, .. } => {
                self.node("Annotation::Doc", _id.id)
                    .field("position", position)
                    .end();
            }
            Annotation::Comment { position, .. } => {
                self.node("Annotation::Comment", _id.id)
                    .field("position", position)
                    .end();
            }
            Annotation::Tag { position, .. } => {
                self.node("Annotation::Tag", _id.id)
                    .field("position", position)
                    .end();
            }
            Annotation::Decorator { position, .. } => {
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
