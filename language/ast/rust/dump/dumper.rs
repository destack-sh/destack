//! The Dumper is a helper for ugly-printing AST nodes for debugging and inspection.
//! Unlike the pretty Printer, Dumper makes no attempt to look like source code;
//!  instead, Dumper is optimized for checking parse trees.
//!
//! Because the AST is a tree, we print any children as a tree.
//! If there is more than one group of children, we prefix a label for the group name (like `left`).
//! The target output is something like:
//! ```
//!  Expression::Binary { operator: Add }
//!  ├─ [left] Expression::Path { path: a }
//!  └─ [right] Expression::Binary { operator: Divide }
//!     ├─ [left] Expression::Binary { operator: Multiply }
//!     │  ├─ [left] Expression::Path { path: b }
//!     │  └─ [right] Expression::ScalarLiteral
//!     │     └─ ScalarLiteral::Integer { value: 2 }
//!     └─ [right] Expression::Unary { operator: Negate }
//!        └─ Expression::ScalarLiteral
//!           └─ ScalarLiteral::Integer { value: 4 }
//! ```

#![allow(clippy::match_like_matches_macro)]

use std::borrow::Cow;

use crate::{
    Annotation, AnnotationPosition, Argument, ArrayLiteral, AssignOperator, BinaryOperator, Blank, Block, BlockFormat, Break, Call, Cast, Coalesce, Comment, CommentStyle, Continue, Decorator, Defer, Doc, DocStyle, Enum, EnumField, Expression, FieldLiteral, FloatType, For, Function, FunctionStyle, If, Implement, Index, IntType, Let, Loop, Match, MatchCase, Module, Mutability, Node, NodeId, NodeTree, NodeTreeStore, NodeType, NodeVisitor, Parameter, PathId, PathPool, Pattern, PatternField, PrimitiveType, RangeLiteral, Return, Runtime, ScalarLiteral, ScopedMutability, StringId, StringPool, Struct, StructField, StructLiteral, Tag, Trait, Try, Tuple, TupleField, TupleLiteral, Type, UnaryOperator, Union, UnionField, Use, UseClause, UseItem, Visibility, While, With, WithClause, walk_annotation, walk_argument, walk_array_literal, walk_blank, walk_block, walk_break, walk_call, walk_cast, walk_coalesce, walk_comment, walk_continue, walk_decorator, walk_defer, walk_doc, walk_enum, walk_enum_field, walk_expression, walk_field_literal, walk_for, walk_function, walk_if, walk_implement, walk_index, walk_let, walk_loop, walk_match, walk_match_case, walk_module, walk_parameter, walk_pattern, walk_pattern_field, walk_range_literal, walk_return, walk_scalar_literal, walk_struct, walk_struct_field, walk_struct_literal, walk_tag, walk_trait, walk_try, walk_tuple, walk_tuple_field, walk_tuple_literal, walk_type, walk_union, walk_union_field, walk_use, walk_use_clause, walk_use_item, walk_while, walk_with, walk_with_clause
};

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
        let float_str = match self {
            FloatType::Float32 => "float32",
            FloatType::Float64 => "float64",
        };
        dumper.write_str(float_str, Some(Color::Yellow));
    }
}

/// Dump a PrimitiveType as a structured representation.
impl Dump for PrimitiveType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            PrimitiveType::Undefined => {
                dumper.object("PrimitiveType::Undefined").end();
            }
            PrimitiveType::Void => {
                dumper.object("PrimitiveType::Void").end();
            }
            PrimitiveType::Null => {
                dumper.object("PrimitiveType::Null").end();
            }
            PrimitiveType::Boolean => {
                dumper.object("PrimitiveType::Boolean").end();
            }
            PrimitiveType::Character => {
                dumper.object("PrimitiveType::Character").end();
            }
            PrimitiveType::Int(int_type) => {
                dumper.object("PrimitiveType::Int").value(int_type).end();
            }
            PrimitiveType::Float(float_type) => {
                dumper
                    .object("PrimitiveType::Float")
                    .value(float_type)
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
        for annotation in annotations {
            self.visit_annotation(tree, annotation, tree.get(annotation));
        }
    }

    // ------------------------------------------------------------
    // Groupings
    // ------------------------------------------------------------

    fn visit_block(&mut self, _tree: &NodeTree, _id: NodeId<Block>, block: &Block) {
        self.node("Block", _id.id)
            .field("format", &block.format)
            .field_optional("label", &block.label)
            .end();
        self.with_depth(|dumper| {
            walk_block(dumper, _tree, _id, block);
        });
    }

    fn visit_expression(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Module(..) => {
                self.node("Expression::Module", _id.id).end();
            }
            Expression::Struct(_node) => {
                self.node("Expression::Struct", _id.id).end();
            }
            Expression::Enum(_node) => {
                self.node("Expression::Enum", _id.id).end();
            }
            Expression::Union(_node) => {
                self.node("Expression::Union", _id.id).end();
            }
            Expression::Trait(_node) => {
                self.node("Expression::Trait", _id.id).end();
            }
            Expression::Implement(_node) => {
                self.node("Expression::Implement", _id.id).end();
            }
            Expression::Function(_node) => {
                self.node("Expression::Function", _id.id).end();
            }
            Expression::Block(_node) => {
                self.node("Expression::Block", _id.id).end();
            }

            Expression::With(_node) => {
                self.node("Expression::With", _id.id).end();
            }
            Expression::Use(_node) => {
                self.node("Expression::Use", _id.id).end();
            }
            Expression::Let(_node) => {
                self.node("Expression::Let", _id.id).end();
            }
            Expression::If(_node) => {
                self.node("Expression::If", _id.id).end();
            }
            Expression::While(_node) => {
                self.node("Expression::While", _id.id).end();
            }
            Expression::For(_node) => {
                self.node("Expression::For", _id.id).end();
            }
            Expression::Loop(_node) => {
                self.node("Expression::Loop", _id.id).end();
            }
            Expression::Try(_node) => {
                self.node("Expression::Try", _id.id).end();
            }
            Expression::Match(_node) => {
                self.node("Expression::Match", _id.id).end();
            }
            Expression::Break(_node) => {
                self.node("Expression::Break", _id.id).end();
            }
            Expression::Continue(_node) => {
                self.node("Expression::Continue", _id.id).end();
            }
            Expression::Defer(_node) => {
                self.node("Expression::Defer", _id.id).end();
            }
            Expression::Return(_node) => {
                self.node("Expression::Return", _id.id).end();
            }

            Expression::Path(path) => {
                self.node("Expression::Path", _id.id)
                    .field("path", path)
                    .end();
            }
            Expression::ScalarLiteral(_node) => {
                self.node("Expression::ScalarLiteral", _id.id).end();
            }
            Expression::RangeLiteral(_node) => {
                self.node("Expression::RangeLiteral", _id.id).end();
            }
            Expression::ArrayLiteral(_node) => {
                self.node("Expression::ArrayLiteral", _id.id).end();
            }
            Expression::TupleLiteral(_node) => {
                self.node("Expression::TupleLiteral", _id.id).end();
            }
            Expression::StructLiteral(_node) => {
                self.node("Expression::StructLiteral", _id.id).end();
            }

            Expression::Unary { operator, right: _ } => {
                self.node("Expression::Unary", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Reference {
                mutability,
                right: _,
            } => {
                self.node("Expression::Reference", _id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Expression::Member { receiver: _, path } => {
                self.node("Expression::Member", _id.id)
                    .field("path", path)
                    .end();
            }
            Expression::Index(_node) => {
                self.node("Expression::Index", _id.id).end();
            }
            Expression::Call(_node) => {
                self.node("Expression::Call", _id.id).end();
            }
            Expression::Cast(_node) => {
                self.node("Expression::Cast", _id.id).end();
            }
            Expression::Unwrap(_node) => {
                self.node("Expression::Unwrap", _id.id).end();
            }
            Expression::UnwrapOrPanic(_node) => {
                self.node("Expression::UnwrapOrPanic", _id.id).end();
            }
            Expression::Coalesce(_node) => {
                self.node("Expression::Coalesce", _id.id).end();
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

    // ------------------------------------------------------------
    // Declarations
    // ------------------------------------------------------------

    fn visit_module(&mut self, _tree: &NodeTree, _id: NodeId<Module>, module: &Module) {
        self.node("Module", _id.id)
            .field_optional("name", &module.name)
            .field_optional("visibility", &module.visibility)
            .end();
        self.with_depth(|dumper| {
            walk_module(dumper, _tree, _id, module);
        });
    }

    fn visit_struct(&mut self, _tree: &NodeTree, _id: NodeId<Struct>, struct_node: &Struct) {
        self.node("Struct", _id.id)
            .field_optional("name", &struct_node.name)
            .field_optional("visibility", &struct_node.visibility)
            .end();
        self.with_depth(|dumper| {
            walk_struct(dumper, _tree, _id, struct_node);
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

    fn visit_enum(&mut self, _tree: &NodeTree, _id: NodeId<Enum>, enum_node: &Enum) {
        self.node("Enum", _id.id)
            .field_optional("name", &enum_node.name)
            .field_optional("visibility", &enum_node.visibility)
            .end();
        self.with_depth(|dumper| {
            walk_enum(dumper, _tree, _id, enum_node);
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

    fn visit_union(&mut self, _tree: &NodeTree, _id: NodeId<Union>, union_node: &Union) {
        self.node("Union", _id.id)
            .field_optional("name", &union_node.name)
            .field_optional("visibility", &union_node.visibility)
            .end();
        self.with_depth(|dumper| {
            walk_union(dumper, _tree, _id, union_node);
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

    fn visit_trait(&mut self, _tree: &NodeTree, _id: NodeId<Trait>, trait_node: &Trait) {
        self.node("Trait", _id.id)
            .field_optional("name", &trait_node.name)
            .field_optional("visibility", &trait_node.visibility)
            .end();
        self.with_depth(|dumper| {
            walk_trait(dumper, _tree, _id, trait_node);
        });
    }

    fn visit_implement(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<Implement>,
        _implement: &Implement,
    ) {
        self.node("Implement", _id.id).end();
        self.with_depth(|dumper| {
            walk_implement(dumper, _tree, _id, _implement);
        });
    }

    fn visit_type(&mut self, _tree: &NodeTree, _id: NodeId<Type>, type_node: &Type) {
        match type_node {
            Type::Infer => {
                self.node("Type::Infer", _id.id).end();
            }
            Type::Maybe(..) => {
                self.node("Type::Maybe", _id.id).end();
            }
            Type::Not(..) => {
                self.node("Type::Not", _id.id).end();
            }
            Type::Never => {
                self.node("Type::Never", _id.id).end();
            }
            Type::Self_ => {
                self.node("Type::Self", _id.id).end();
            }
            Type::Primitive(primitive) => {
                self.node("Type::Primitive", _id.id).value(primitive).end();
            }
            Type::Path { path, .. } => {
                self.node("Type::Path", _id.id).value(path).end();
            }
            Type::Reference {
                mutability,
                target: _,
            } => {
                self.node("Type::Pointer", _id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Type::Virtual(..) => {
                self.node("Type::Virtual", _id.id).end();
            }
            Type::Variadic(..) => {
                self.node("Type::Variadic", _id.id).end();
            }
            Type::Array { .. } => {
                self.node("Type::Array", _id.id).end();
            }
            Type::Slice { element: _ } => {
                self.node("Type::Slice", _id.id).end();
            }
            Type::Tuple(_tuple) => {
                self.node("Type::Tuple", _id.id).end();
            }
            Type::InlineStruct(_struct_node) => {
                self.node("Type::InlineStruct", _id.id).end();
            }
            Type::InlineEnum(_enum_node) => {
                self.node("Type::InlineEnum", _id.id).end();
            }
            Type::InlineUnion(_union_node) => {
                self.node("Type::InlineUnion", _id.id).end();
            }
            Type::Union(_) => {
                self.node("Type::Union", _id.id).end();
            }
            Type::Intersection(_) => {
                self.node("Type::Intersection", _id.id).end();
            }
            Type::Function(_function) => {
                self.node("Type::Function", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_type(dumper, _tree, _id, type_node);
        });
    }

    fn visit_tuple(&mut self, _tree: &NodeTree, _id: NodeId<Tuple>, _tuple: &Tuple) {
        self.node("Tuple", _id.id).end();
        self.with_depth(|dumper| {
            walk_tuple(dumper, _tree, _id, _tuple);
        });
    }

    fn visit_tuple_field(&mut self, _tree: &NodeTree, _id: NodeId<TupleField>, field: &TupleField) {
        match field {
            TupleField::Named { name, r#type: _ } => {
                self.node("TupleField::Named", _id.id)
                    .field("name", name)
                    .end();
            }
            TupleField::Positional { r#type: _ } => {
                self.node("TupleField::Positional", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_tuple_field(dumper, _tree, _id, field);
        });
    }

    fn visit_function(&mut self, _tree: &NodeTree, _id: NodeId<Function>, function: &Function) {
        self.node("Function", _id.id)
            .field_optional("name", &function.name)
            .field_optional("visibility", &function.visibility)
            .field("runtime", &function.runtime)
            .end();
        self.with_depth(|dumper| {
            walk_function(dumper, _tree, _id, function);
        });
    }

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------

    fn visit_with(&mut self, _tree: &NodeTree, _id: NodeId<With>, _with: &With) {
        self.node("With", _id.id).end();
        self.with_depth(|dumper| {
            walk_with(dumper, _tree, _id, _with);
        });
    }

    fn visit_with_clause(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<WithClause>,
        clause: &WithClause,
    ) {
        match clause {
            WithClause::Declaration { target: _, alias } => {
                self.node("WithClause::Declaration", _id.id)
                    .field_optional("alias", alias)
                    .end();
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

    fn visit_use(&mut self, _tree: &NodeTree, _id: NodeId<Use>, use_node: &Use) {
        self.node("Use", _id.id)
            .field_optional("visibility", &use_node.visibility)
            .end();
        self.with_depth(|dumper| {
            walk_use(dumper, _tree, _id, use_node);
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
    // Control
    // ------------------------------------------------------------

    fn visit_if(&mut self, _tree: &NodeTree, _id: NodeId<If>, if_node: &If) {
        match if_node {
            If::If {
                runtime,
                condition: _,
                then_block: _,
            } => {
                self.node("If::If", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            If::IfElse {
                runtime,
                condition: _,
                then_block: _,
                else_block: _,
            } => {
                self.node("If::IfElse", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
            If::IfElseIf {
                runtime,
                condition: _,
                then_block: _,
                else_if: _,
            } => {
                self.node("If::IfElseIf", _id.id)
                    .field_optional("runtime", runtime)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_if(dumper, _tree, _id, if_node);
        });
    }

    fn visit_while(&mut self, _tree: &NodeTree, _id: NodeId<While>, while_node: &While) {
        self.node("While", _id.id)
            .field_optional("runtime", &while_node.runtime)
            .end();
        self.with_depth(|dumper| {
            walk_while(dumper, _tree, _id, while_node);
        });
    }

    fn visit_for(&mut self, _tree: &NodeTree, _id: NodeId<For>, for_node: &For) {
        self.node("For", _id.id)
            .field_optional("runtime", &for_node.runtime)
            .end();
        self.with_depth(|dumper| {
            walk_for(dumper, _tree, _id, for_node);
        });
    }

    fn visit_loop(&mut self, _tree: &NodeTree, _id: NodeId<Loop>, loop_node: &Loop) {
        self.node("Loop", _id.id)
            .field_optional("runtime", &loop_node.runtime)
            .end();
        self.with_depth(|dumper| {
            walk_loop(dumper, _tree, _id, loop_node);
        });
    }

    fn visit_break(&mut self, _tree: &NodeTree, _id: NodeId<Break>, _break_node: &Break) {
        self.node("Break", _id.id).end();
        self.with_depth(|dumper| {
            walk_break(dumper, _tree, _id, _break_node);
        });
    }

    fn visit_continue(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<Continue>,
        _continue_node: &Continue,
    ) {
        self.node("Continue", _id.id).end();
        self.with_depth(|dumper| {
            walk_continue(dumper, _tree, _id, _continue_node);
        });
    }

    fn visit_defer(&mut self, _tree: &NodeTree, _id: NodeId<Defer>, defer_node: &Defer) {
        match defer_node {
            Defer::Expression(_expression) => {
                self.node("Defer::Expression", _id.id).end();
            }
            Defer::Block(_block) => {
                self.node("Defer::Block", _id.id).end();
            }
            Defer::Catch(_match) => {
                self.node("Defer::Catch", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_defer(dumper, _tree, _id, defer_node);
        });
    }

    fn visit_return(&mut self, _tree: &NodeTree, _id: NodeId<Return>, _return_node: &Return) {
        self.node("Return", _id.id).end();
        self.with_depth(|dumper| {
            walk_return(dumper, _tree, _id, _return_node);
        });
    }

    fn visit_try(&mut self, _tree: &NodeTree, _id: NodeId<Try>, try_node: &Try) {
        match try_node {
            Try::Expression { try_expression: _ } => {
                self.node("Try::Expression", _id.id).end();
            }
            Try::Block { try_block: _ } => {
                self.node("Try::Block", _id.id).end();
            }
            Try::BlockWithCatch {
                try_block: _,
                catch_match: _,
            } => {
                self.node("Try::BlockWithCatch", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_try(dumper, _tree, _id, try_node);
        });
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_let(&mut self, _tree: &NodeTree, _id: NodeId<Let>, let_node: &Let) {
        self.node("Let", _id.id)
            .field("mutability", &let_node.mutability)
            .field_optional("visibility", &let_node.visibility)
            .end();
        self.with_depth(|dumper| {
            walk_let(dumper, _tree, _id, let_node);
        });
    }

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
    // Literals
    // ------------------------------------------------------------

    fn visit_scalar_literal(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<ScalarLiteral>,
        literal: &ScalarLiteral,
    ) {
        match literal {
            ScalarLiteral::Undefined => {
                self.node("ScalarLiteral::Undefined", _id.id).end();
            }
            ScalarLiteral::Void => {
                self.node("ScalarLiteral::Void", _id.id).end();
            }
            ScalarLiteral::Null => {
                self.node("ScalarLiteral::Null", _id.id).end();
            }
            ScalarLiteral::Boolean(value) => {
                self.node("ScalarLiteral::Boolean", _id.id)
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Byte(value) => {
                self.node("ScalarLiteral::Byte", _id.id)
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Integer(value, _) => {
                self.node("ScalarLiteral::Integer", _id.id)
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Float(value, _) => {
                self.node("ScalarLiteral::Float", _id.id)
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Character(value) => {
                self.node("ScalarLiteral::Character", _id.id)
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::String(value) => {
                self.node("ScalarLiteral::String", _id.id)
                    .field("value", value)
                    .end();
            }
            ScalarLiteral::ByteString(value) => {
                self.node("ScalarLiteral::ByteString", _id.id)
                    .field("value", value)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_scalar_literal(dumper, _tree, _id, literal);
        });
    }

    fn visit_range_literal(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<RangeLiteral>,
        _literal: &RangeLiteral,
    ) {
        self.node("RangeLiteral", _id.id).end();
        self.with_depth(|dumper| {
            walk_range_literal(dumper, _tree, _id, _literal);
        });
    }

    fn visit_array_literal(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<ArrayLiteral>,
        literal: &ArrayLiteral,
    ) {
        match literal {
            ArrayLiteral::Fixed { elements: _ } => {
                self.node("ArrayLiteral::Fixed", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_array_literal(dumper, _tree, _id, literal);
        });
    }

    fn visit_tuple_literal(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<TupleLiteral>,
        _literal: &TupleLiteral,
    ) {
        self.node("TupleLiteral", _id.id).end();
        self.with_depth(|dumper| {
            walk_tuple_literal(dumper, _tree, _id, _literal);
        });
    }

    fn visit_struct_literal(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<StructLiteral>,
        _literal: &StructLiteral,
    ) {
        self.node("StructLiteral", _id.id).end();
        self.with_depth(|dumper| {
            walk_struct_literal(dumper, _tree, _id, _literal);
        });
    }

    fn visit_field_literal(
        &mut self,
        _tree: &NodeTree,
        _id: NodeId<FieldLiteral>,
        literal: &FieldLiteral,
    ) {
        match literal {
            FieldLiteral::Named { name, value: _ } => {
                self.node("FieldLiteral::Named", _id.id)
                    .field("name", name)
                    .end();
            }
            FieldLiteral::NamedShorthand { name } => {
                self.node("FieldLiteral::NamedShorthand", _id.id)
                    .field("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_field_literal(dumper, _tree, _id, literal);
        });
    }

    // ------------------------------------------------------------
    // Calls
    // ------------------------------------------------------------

    fn visit_index(&mut self, _tree: &NodeTree, _id: NodeId<Index>, index: &Index) {
        match index {
            Index::Explicit {
                receiver: _,
                index: _,
            } => {
                self.node("Index::Explicit", _id.id).end();
            }
            Index::Implicit { receiver: _, index } => {
                self.node("Index::Implicit", _id.id)
                    .field("index", index)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_index(dumper, _tree, _id, index);
        });
    }

    fn visit_call(&mut self, _tree: &NodeTree, _id: NodeId<Call>, call: &Call) {
        self.node("Call", _id.id)
            .field("runtime", &call.runtime)
            .end();
        self.with_depth(|dumper| {
            walk_call(dumper, _tree, _id, call);
        });
    }

    fn visit_cast(&mut self, _tree: &NodeTree, _id: NodeId<Cast>, _cast: &Cast) {
        self.node("Cast", _id.id).end();
        self.with_depth(|dumper| {
            walk_cast(dumper, _tree, _id, _cast);
        });
    }

    fn visit_coalesce(&mut self, _tree: &NodeTree, _id: NodeId<Coalesce>, _coalesce: &Coalesce) {
        self.node("Coalesce", _id.id).end();
        self.with_depth(|dumper| {
            walk_coalesce(dumper, _tree, _id, _coalesce);
        });
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    fn visit_match(&mut self, _tree: &NodeTree, _id: NodeId<Match>, _match_node: &Match) {
        self.node("Match", _id.id).end();
        self.with_depth(|dumper| {
            walk_match(dumper, _tree, _id, _match_node);
        });
    }

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
            Pattern::Unwrap(_) => {
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
            Pattern::Literal(_node) => {
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
