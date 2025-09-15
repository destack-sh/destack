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

use crate::{
    Argument, ArrayLiteral, AssignOperator, BinaryOperator, Block, Break, Call, Cast, Coalesce,
    Continue, Defer, Doc, Enum, EnumField, Expression, FieldLiteral, FloatType, For, Function,
    FunctionStyle, If, Implement, Index, IntType, Let, LetInitialization, Loop, Match, MatchCase,
    Module, Mutability, Node, NodeId, NodeTree, NodeTreeStore, Parameter, PathId, PathPool,
    Pattern, PatternField, PrimitiveType, RangeLiteral, Return, Runtime, ScalarLiteral,
    SelfParameter, Statement, StringId, StringPool, Struct, StructField, StructLiteral, Trait, Try,
    Tuple, TupleField, TupleLiteral, Type, UnaryOperator, Union, UnionField, Use, UseClause,
    UseItem, Visibility, While, With, WithClause,
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

// todo!: also dump annotations somehow (docs/comments)

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

    /// Write a newline to the buffer.
    #[inline]
    fn write_newline(&mut self) {
        self.write_str("\n", None);
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
    pub fn node<'d>(&'d mut self, name: &str) -> StructDumper<'d, 'a> {
        StructDumper::begin(self, name)
    }

    /// Helper for dumping a single node that just wraps another node.
    #[inline]
    pub fn node_unwrap<T: Node + Clone + Dump>(
        &mut self,
        name: &str,
        wrapped_id: NodeId<T>,
    ) -> &mut Self
    where
        NodeTree: NodeTreeStore<T>,
    {
        let mut node_dumper = self.node(name);
        node_dumper.end();
        self.with_depth(|dumper| {
            dumper.dump_node(&wrapped_id, None);
        });
        self
    }

    /// Dump something as a new line.
    pub fn dump_node<T: Dump>(&mut self, thing: &T, label: Option<&str>) -> &mut Self {
        // default to last (no following siblings) when not part of a known group
        self.dump_node_branch(thing, label, false)
    }

    /// Dump many somethings as a new line (each).
    pub fn dump_nodes<T: Dump>(&mut self, things: &[T], label: Option<&str>) -> &mut Self {
        for (index, thing) in things.iter().enumerate() {
            let has_more = index + 1 < things.len();
            self.dump_node_branch(thing, label, has_more);
        }
        self
    }

    /// Dump a single thing but with explicit knowledge whether more siblings follow at this depth.
    #[inline]
    fn dump_node_branch<T: Dump>(
        &mut self,
        thing: &T,
        label: Option<&str>,
        has_more_siblings: bool,
    ) -> &mut Self {
        self.write_newline();
        // draw prefix depending on current structural depth
        if self.branch_stack.is_empty() {
            // no ancestor columns recorded, but we are nested: draw just the connector
            if has_more_siblings {
                self.write_str("├─ ", Some(Color::Cyan));
            } else {
                self.write_str("└─ ", Some(Color::Cyan));
            }
        } else {
            // we have ancestor columns; push our connector, render, then pop
            self.branch_stack.push(has_more_siblings);
            self.write_prefix();
            let _ = self.branch_stack.pop();
        }
        if let Some(label) = label {
            self.write_str("[", Some(Color::White));
            self.write_str(label.as_ref(), Some(Color::White));
            self.write_str("] ", Some(Color::White));
        }
        // set before dumping so nested with_depth sees correct parent branch info
        self.last_line_has_more = Some(has_more_siblings);
        thing.dump(self);
        self
    }

    /// Dump a list, but keep the branch open if there will be more siblings after the list ends.
    #[inline]
    fn dump_nodes_tail<T: Dump>(
        &mut self,
        things: &[T],
        label: Option<&str>,
        has_more_trail: bool,
    ) -> &mut Self {
        for (index, thing) in things.iter().enumerate() {
            let has_more = (index + 1) < things.len() || has_more_trail;
            self.dump_node_branch(thing, label, has_more);
        }
        self
    }
}

/// Helper for dumping a single struct-like type.
#[derive(Debug)]
pub struct StructDumper<'d, 'p> {
    dumper: &'d mut Dumper<'p>,
    has_fields: bool,
}

impl<'d, 'p> StructDumper<'d, 'p> {
    /// Begin a new struct-like dumper with some name.
    pub fn begin(dumper: &'d mut Dumper<'p>, name: &str) -> Self {
        dumper.write_str(name, Some(Color::BrightBlue));
        Self {
            dumper,
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
        self
    }

    /// Finish node and close the struct as exhaustive.
    pub fn end(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(" }", Some(Color::White));
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
        dumper.write_str(self.as_ref(), Some(Color::BrightYellow));
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

/// Dump a Visibility as a string.
impl Dump for Visibility {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::Yellow));
    }
}

/// Dump an IntType as a structured representation.
impl Dump for IntType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("IntType")
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
            PrimitiveType::Void => {
                dumper.node("PrimitiveType::Void").end();
            }
            PrimitiveType::Null => {
                dumper.node("PrimitiveType::Null").end();
            }
            PrimitiveType::Boolean => {
                dumper.node("PrimitiveType::Boolean").end();
            }
            PrimitiveType::Character => {
                dumper.node("PrimitiveType::Character").end();
            }
            PrimitiveType::Int(int_type) => {
                dumper.node("PrimitiveType::Int").value(int_type).end();
            }
            PrimitiveType::Float(float_type) => {
                dumper.node("PrimitiveType::Float").value(float_type).end();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

impl Dump for Block {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Block").end();
        dumper.with_depth(|dumper| {
            dumper.dump_nodes(&self.statements, None);
        });
    }
}

impl Dump for Statement {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Statement::Expression(node) => {
                dumper.node_unwrap("Statement::Expression", *node);
            }
            Statement::Module(node) => {
                dumper.node_unwrap("Statement::Module", *node);
            }
            Statement::Struct(node) => {
                dumper.node_unwrap("Statement::Struct", *node);
            }
            Statement::Enum(node) => {
                dumper.node_unwrap("Statement::Enum", *node);
            }
            Statement::Union(node) => {
                dumper.node_unwrap("Statement::Union", *node);
            }
            Statement::Trait(node) => {
                dumper.node_unwrap("Statement::Trait", *node);
            }
            Statement::Implement(node) => {
                dumper.node_unwrap("Statement::Implement", *node);
            }
            Statement::Function(node) => {
                dumper.node_unwrap("Statement::Function", *node);
            }
            Statement::With(node) => {
                dumper.node_unwrap("Statement::With", *node);
            }
            Statement::Use(node) => {
                dumper.node_unwrap("Statement::Use", *node);
            }
        }
    }
}

impl Dump for Expression {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Expression::Module(node) => {
                dumper.node_unwrap("Expression::Module", *node);
            }
            Expression::Struct(node) => {
                dumper.node_unwrap("Expression::Struct", *node);
            }
            Expression::Enum(node) => {
                dumper.node_unwrap("Expression::Enum", *node);
            }
            Expression::Union(node) => {
                dumper.node_unwrap("Expression::Union", *node);
            }
            Expression::Trait(node) => {
                dumper.node_unwrap("Expression::Trait", *node);
            }
            Expression::Implement(node) => {
                dumper.node_unwrap("Expression::Implement", *node);
            }
            Expression::Function(node) => {
                dumper.node_unwrap("Expression::Function", *node);
            }

            Expression::Let(node) => {
                dumper.node_unwrap("Expression::Let", *node);
            }
            Expression::Block(node) => {
                dumper.node_unwrap("Expression::Block", *node);
            }
            Expression::If(node) => {
                dumper.node_unwrap("Expression::If", *node);
            }
            Expression::While(node) => {
                dumper.node_unwrap("Expression::While", *node);
            }
            Expression::For(node) => {
                dumper.node_unwrap("Expression::For", *node);
            }
            Expression::Loop(node) => {
                dumper.node_unwrap("Expression::Loop", *node);
            }
            Expression::Break(node) => {
                dumper.node_unwrap("Expression::Break", *node);
            }
            Expression::Continue(node) => {
                dumper.node_unwrap("Expression::Continue", *node);
            }
            Expression::Defer(node) => {
                dumper.node_unwrap("Expression::Defer", *node);
            }
            Expression::Return(node) => {
                dumper.node_unwrap("Expression::Return", *node);
            }
            Expression::Try(node) => {
                dumper.node_unwrap("Expression::Try", *node);
            }
            Expression::Match(node) => {
                dumper.node_unwrap("Expression::Match", *node);
            }

            Expression::Path(path) => {
                let mut node_dumper = dumper.node("Expression::Path");
                node_dumper.field("path", path);
                node_dumper.end();
            }
            Expression::ScalarLiteral(node) => {
                dumper.node_unwrap("Expression::ScalarLiteral", *node);
            }
            Expression::RangeLiteral(node) => {
                dumper.node_unwrap("Expression::RangeLiteral", *node);
            }
            Expression::ArrayLiteral(node) => {
                dumper.node_unwrap("Expression::ArrayLiteral", *node);
            }
            Expression::TupleLiteral(node) => {
                dumper.node_unwrap("Expression::TupleLiteral", *node);
            }
            Expression::StructLiteral(node) => {
                dumper.node_unwrap("Expression::StructLiteral", *node);
            }

            Expression::Unary { operator, right } => {
                let mut node_dumper = dumper.node("Expression::Unary");
                node_dumper.field("operator", operator);
                node_dumper.end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(right, None);
                });
            }
            Expression::Reference { mutability, right } => {
                let mut node_dumper = dumper.node("Expression::Reference");
                node_dumper.field("mutability", mutability);
                node_dumper.end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(right, None);
                });
            }
            Expression::Member { receiver, path } => {
                let mut node_dumper = dumper.node("Expression::Member");
                node_dumper.field("path", path);
                node_dumper.end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(receiver, None);
                });
            }
            Expression::Index(node) => {
                dumper.node_unwrap("Expression::Index", *node);
            }
            Expression::Call(node) => {
                dumper.node_unwrap("Expression::Call", *node);
            }
            Expression::Cast(node) => {
                dumper.node_unwrap("Expression::Cast", *node);
            }
            Expression::Coalesce(node) => {
                dumper.node_unwrap("Expression::Coalesce", *node);
            }
            Expression::Unwrap(node) => {
                dumper.node_unwrap("Expression::Unwrap", *node);
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let mut node_dumper = dumper.node("Expression::Binary");
                node_dumper.field("operator", operator);
                node_dumper.end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(left, Some("left"), true);
                    dumper.dump_node_branch(right, Some("right"), false);
                });
            }
            Expression::Assign {
                left,
                operator,
                right,
            } => {
                let mut node_dumper = dumper.node("Expression::Assign");
                node_dumper.field("operator", operator);
                node_dumper.end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(left, Some("left"), true);
                    dumper.dump_node_branch(right, Some("right"), false);
                });
            }

            Expression::Error(_) => {
                dumper.node("Expression::Error").end();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

impl Dump for Module {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("Module")
            .field_optional("name", &self.name)
            .field_optional("visibility", &self.visibility)
            .end();
        dumper.with_depth(|dumper| {
            dumper.dump_nodes(&self.statements, None);
        });
    }
}

impl Dump for Struct {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("Struct")
            .field_optional("name", &self.name)
            .field_optional("visibility", &self.visibility)
            .end();
        dumper.with_depth(|dumper| {
            let has_more_after_fields = !self.statements.is_empty();
            if let Some(super_types) = &self.super_types {
                dumper.dump_nodes_tail(super_types, Some("super"), has_more_after_fields);
            }
            dumper.dump_nodes_tail(&self.fields, None, has_more_after_fields);
            dumper.dump_nodes(&self.statements, None);
        });
    }
}

impl Dump for StructField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("StructField").field("name", &self.name).end();
        dumper.with_depth(|dumper| {
            let has_default = self.default.is_some();
            dumper.dump_node_branch(&self.r#type, None, has_default);
            if let Some(default) = self.default {
                dumper.dump_node_branch(&default, None, false);
            }
        });
    }
}

impl Dump for Enum {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("Enum")
            .field_optional("name", &self.name)
            .field_optional("visibility", &self.visibility)
            .end();
        dumper.with_depth(|dumper| {
            if let Some(super_types) = &self.super_types {
                dumper.dump_nodes_tail(super_types, Some("super"), true);
            }
            dumper.dump_nodes(&self.fields, None);
        });
    }
}

impl Dump for EnumField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("EnumField").field("name", &self.name).end();
    }
}

impl Dump for Union {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("Union")
            .field_optional("name", &self.name)
            .field_optional("visibility", &self.visibility)
            .end();
        dumper.with_depth(|dumper| {
            if let Some(super_types) = &self.super_types {
                dumper.dump_nodes_tail(super_types, Some("super"), true);
            }
            dumper.dump_nodes(&self.fields, None);
        });
    }
}

impl Dump for UnionField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("UnionField").field("name", &self.name).end();
        dumper.with_depth(|dumper| {
            dumper.dump_node(&self.r#type, None);
            if let Some(value) = self.value {
                dumper.dump_node(&value, None);
            }
        });
    }
}

impl Dump for Trait {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("Trait")
            .field_optional("name", &self.name)
            .field_optional("visibility", &self.visibility)
            .end();
        dumper.with_depth(|dumper| {
            if let Some(super_types) = &self.super_types {
                dumper.dump_nodes_tail(super_types, Some("super"), true);
            }
            dumper.dump_nodes(&self.withs, None);
            dumper.dump_nodes(&self.statements, None);
        });
    }
}

impl Dump for Implement {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Implement").end();
        dumper.with_depth(|dumper| {
            if let Some(static_arguments) = &self.static_arguments {
                dumper.dump_nodes_tail(static_arguments, None, true);
            }
            let has_after_receiver = self.for_trait.is_some() || !self.statements.is_empty();
            dumper.dump_node_branch(&self.receiver, Some("receiver"), has_after_receiver);
            if let Some(for_trait) = &self.for_trait {
                let has_after_for = !self.statements.is_empty();
                dumper.dump_node_branch(for_trait, Some("for"), has_after_for);
            }
            dumper.dump_nodes(&self.statements, None);
        });
    }
}

impl Dump for Type {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Type::Infer => {
                dumper.node("Type::Infer").end();
            }
            Type::Maybe(inner) => {
                dumper.node("Type::Maybe").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(inner, None);
                });
            }
            Type::Not(inner) => {
                dumper.node("Type::Not").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(inner, None);
                });
            }
            Type::Never => {
                dumper.node("Type::Never").end();
            }
            Type::Self_ => {
                dumper.node("Type::Self").end();
            }
            Type::Primitive(primitive) => {
                dumper.node("Type::Primitive").value(primitive).end();
            }
            Type::Path {
                path,
                static_arguments,
            } => {
                dumper.node("Type::Path").value(path).end();
                dumper.with_depth(|dumper| {
                    if let Some(static_arguments) = static_arguments {
                        dumper.dump_nodes(static_arguments, None);
                    }
                });
            }
            Type::Pointer { mutability, target } => {
                dumper
                    .node("Type::Pointer")
                    .field("mutability", mutability)
                    .end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(target, None);
                });
            }
            Type::Virtual(inner) => {
                dumper.node("Type::Virtual").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(inner, None);
                });
            }
            Type::Variadic(inner) => {
                dumper.node("Type::Variadic").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(inner, None);
                });
            }
            Type::Array {
                element: element_type,
                count,
            } => {
                dumper.node("Type::Array").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(element_type, None, true);
                    dumper.dump_node_branch(count, Some("count"), false);
                });
            }
            Type::Slice { element } => {
                dumper.node("Type::Slice").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(element, None);
                });
            }
            Type::Tuple(tuple) => {
                dumper.node("Type::Tuple").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(tuple, None);
                });
            }
            Type::Struct(struct_node) => {
                dumper.node("Type::Struct").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(struct_node, None);
                });
            }
            Type::Enum(enum_node) => {
                dumper.node("Type::Enum").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(enum_node, None);
                });
            }
            Type::Union(union_node) => {
                dumper.node("Type::Union").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(union_node, None);
                });
            }
            Type::Function(function) => {
                dumper.node("Type::Function").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(function, None);
                });
            }
        }
    }
}

impl Dump for Tuple {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Tuple").end();
        dumper.with_depth(|dumper| {
            dumper.dump_nodes(&self.elements, None);
        });
    }
}

impl Dump for TupleField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            TupleField::Named { name, r#type } => {
                dumper.node("TupleField::Named").field("name", name).end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(r#type, None);
                });
            }
            TupleField::Positional { r#type } => {
                dumper.node("TupleField::Positional").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(r#type, None);
                });
            }
        }
    }
}

impl Dump for SelfParameter {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("SelfParameter")
            .field("mutability", &self.mutability)
            .field("is_pointer", &self.is_pointer)
            .end();
    }
}

impl Dump for Function {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("Function")
            .field_optional("name", &self.name)
            .field_optional("visibility", &self.visibility)
            .field_optional("self", &self.self_parameter)
            .field("runtime", &self.runtime)
            .end();
        dumper.with_depth(|dumper| {
            if let Some(static_parameters) = &self.static_parameters {
                let tail_after = !self.dynamic_parameters.is_empty()
                    || self.return_type.is_some()
                    || self.with.is_some()
                    || self.body.is_some();
                dumper.dump_nodes_tail(static_parameters, None, tail_after);
            }
            let tail_after_dynamic =
                self.return_type.is_some() || self.with.is_some() || self.body.is_some();
            dumper.dump_nodes_tail(
                &self.dynamic_parameters,
                Some("dynamic"),
                tail_after_dynamic,
            );
            if let Some(return_type) = &self.return_type {
                let has_more = self.with.is_some() || self.body.is_some();
                dumper.dump_node_branch(return_type, Some("return"), has_more);
            }
            if let Some(with) = &self.with {
                let has_more = self.body.is_some();
                dumper.dump_node_branch(with, None, has_more);
            }
            if let Some(body) = &self.body {
                dumper.dump_node_branch(body, None, false);
            }
        });
    }
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------
impl Dump for With {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("With").end();
        dumper.with_depth(|dumper| {
            dumper.dump_nodes(&self.clauses, None);
        });
    }
}

impl Dump for WithClause {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            WithClause::Declaration { target, alias } => {
                dumper
                    .node("WithClause::Declaration")
                    .field_optional("alias", alias)
                    .end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(target, None, false);
                });
            }
            WithClause::Assertion { target, assertion } => {
                dumper.node("WithClause::Assertion").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(target, None, true);
                    dumper.dump_node_branch(assertion, None, false);
                });
            }
        }
    }
}

impl Dump for Use {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("Use")
            .field_optional("visibility", &self.visibility)
            .end();
        dumper.with_depth(|dumper| {
            let has_body = self.body.is_some();
            dumper.dump_nodes_tail(&self.clauses, None, has_body);
            if let Some(body) = &self.body {
                dumper.dump_node_branch(body, None, false);
            }
        });
    }
}

impl Dump for UseClause {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("UseClause")
            .field_optional("alias", &self.alias)
            .end();
        dumper.with_depth(|dumper| {
            let has_items = self.items.as_ref().map(|v| !v.is_empty()).unwrap_or(false);
            dumper.dump_node_branch(&self.target, None, has_items);
            if let Some(items) = &self.items {
                dumper.dump_nodes(items, None);
            }
        });
    }
}

impl Dump for UseItem {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("UseItem")
            .field("name", &self.name)
            .field_optional("alias", &self.alias)
            .end();
    }
}

// ----------------------------------------------------------------------------
// Control
// ----------------------------------------------------------------------------

impl Dump for If {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("If").end();
        match self {
            If::If {
                condition,
                then_block,
            } => {
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(condition, Some("condition"), true);
                    dumper.dump_node_branch(then_block, Some("then"), false);
                });
            }
            If::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(condition, Some("condition"), true);
                    dumper.dump_node_branch(then_block, Some("then"), true);
                    dumper.dump_node_branch(else_block, Some("else"), false);
                });
            }
            If::IfElseIf {
                condition,
                then_block,
                else_if,
            } => {
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(condition, Some("condition"), true);
                    dumper.dump_node_branch(then_block, Some("then"), true);
                    dumper.dump_node_branch(else_if, Some("else_if"), false);
                });
            }
        }
    }
}

impl Dump for While {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("While").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node_branch(&self.condition, Some("condition"), true);
            dumper.dump_node_branch(&self.body, Some("body"), false);
        });
    }
}

impl Dump for For {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("For").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node_branch(&self.pattern, Some("pattern"), true);
            dumper.dump_node_branch(&self.iterator, Some("iterator"), true);
            dumper.dump_node_branch(&self.body, Some("body"), false);
        });
    }
}

impl Dump for Loop {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Loop").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node(&self.body, Some("body"));
        });
    }
}

impl Dump for Break {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Break").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node(&self.label, Some("label"));
        });
    }
}

impl Dump for Continue {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Continue").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node(&self.label, Some("label"));
        });
    }
}

impl Dump for Defer {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Defer::Expression(expression) => {
                dumper.node("Defer::Expression").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(expression, Some("expression"));
                });
            }
            Defer::Block(block) => {
                dumper.node("Defer::Block").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(block, Some("block"));
                });
            }
        }
    }
}

impl Dump for Return {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Return").end();
        dumper.with_depth(|dumper| {
            if let Some(value) = self.value {
                dumper.dump_node(&value, Some("value"));
            }
        });
    }
}

impl Dump for Try {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Try").end();
        match self {
            Try::Expression { try_expression } => {
                dumper.with_depth(|dumper| {
                    dumper.dump_node(try_expression, Some("expression"));
                });
            }
            Try::Block { try_block } => {
                dumper.with_depth(|dumper| {
                    dumper.dump_node(try_block, Some("block"));
                });
            }
            Try::BlockWithCatch {
                try_block,
                catch_match,
            } => {
                dumper.with_depth(|dumper| {
                    dumper.dump_node(try_block, Some("block"));
                    dumper.dump_node(catch_match, Some("catch"));
                });
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Bindings
// ----------------------------------------------------------------------------

impl Dump for LetInitialization {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}").as_str(), Some(Color::White));
    }
}

impl Dump for Let {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("Let")
            .field("mutability", &self.mutability)
            .field_optional("visibility", &self.visibility)
            .field("initialization", &self.initialization)
            .end();
        dumper.with_depth(|dumper| {
            dumper.dump_node(&self.pattern, None);
            if let Some(r#type) = self.r#type {
                dumper.dump_node(&r#type, None);
            }
            if let Some(value) = self.value {
                dumper.dump_node(&value, None);
            }
        });
    }
}

impl Dump for Parameter {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.with_depth(|dumper| {
            dumper.node("Parameter").field("name", &self.name).end();
            if let Some(r#type) = self.r#type {
                dumper.dump_node(&r#type, Some("type"));
            }
            if let Some(default) = self.default {
                dumper.dump_node(&default, Some("default"));
            }
        });
    }
}

impl Dump for Argument {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.with_depth(|dumper| match self {
            Argument::Named { name, value } => {
                dumper.node("Argument::Named").field("name", name).end();
                dumper.dump_node(value, None);
            }
            Argument::NamedShorthand { name } => {
                dumper
                    .node("Argument::NamedShorthand")
                    .field("name", name)
                    .end();
            }
            Argument::Positional { value } => {
                dumper.node("Argument::Positional").end();
                dumper.dump_node(value, None);
            }
        });
    }
}

// ----------------------------------------------------------------------------
// Literals
// ----------------------------------------------------------------------------

impl Dump for ScalarLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ScalarLiteral::Void => {
                dumper.node("ScalarLiteral::Void").end();
            }
            ScalarLiteral::Null => {
                dumper.node("ScalarLiteral::Null").end();
            }
            ScalarLiteral::Boolean(value) => {
                dumper
                    .node("ScalarLiteral::Boolean")
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Byte(value) => {
                dumper
                    .node("ScalarLiteral::Byte")
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Integer(value, _) => {
                dumper
                    .node("ScalarLiteral::Integer")
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Float(value, _) => {
                dumper
                    .node("ScalarLiteral::Float")
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::Character(value) => {
                dumper
                    .node("ScalarLiteral::Character")
                    .field("value", &value.to_string().as_str())
                    .end();
            }
            ScalarLiteral::String(value) => {
                dumper
                    .node("ScalarLiteral::String")
                    .field("value", value)
                    .end();
            }
            ScalarLiteral::ByteString(value) => {
                dumper
                    .node("ScalarLiteral::ByteString")
                    .field("value", value)
                    .end();
            }
        }
    }
}

impl Dump for RangeLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("RangeLiteral").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node_branch(&self.start, Some("start"), true);
            dumper.dump_node_branch(&self.end, Some("end"), false);
        });
    }
}

impl Dump for ArrayLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ArrayLiteral::Fixed { elements } => {
                dumper.node("ArrayLiteral::Fixed").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_nodes(elements, Some("element"));
                });
            }
        }
    }
}

impl Dump for TupleLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("TupleLiteral").end();
        dumper.with_depth(|dumper| {
            dumper.dump_nodes(&self.elements, Some("element"));
        });
    }
}

impl Dump for StructLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("StructLiteral").end();
        dumper.with_depth(|dumper| {
            let has_fields = !self.fields.is_empty();
            dumper.dump_node_branch(&self.r#type, None, has_fields);
            dumper.dump_nodes(&self.fields, None);
        });
    }
}

impl Dump for FieldLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            FieldLiteral::Named { name, value } => {
                dumper.node("FieldLiteral::Named").field("name", name).end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(value, None);
                });
            }
            FieldLiteral::NamedShorthand { name } => {
                dumper
                    .node("FieldLiteral::NamedShorthand")
                    .field("name", name)
                    .end();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Calls
// ----------------------------------------------------------------------------

impl Dump for Index {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Index").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node_branch(&self.receiver, Some("receiver"), true);
            dumper.dump_node_branch(&self.index, Some("index"), false);
        });
    }
}

impl Dump for Call {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Call").field("runtime", &self.runtime).end();
        dumper.with_depth(|dumper| {
            let mut after_receiver_more = !self.dynamic_arguments.is_empty();
            if let Some(static_arguments) = &self.static_arguments {
                after_receiver_more = after_receiver_more || !static_arguments.is_empty();
            }
            dumper.dump_node_branch(&self.receiver, Some("receiver"), after_receiver_more);
            if let Some(static_arguments) = &self.static_arguments {
                let tail_has_more = !self.dynamic_arguments.is_empty();
                dumper.dump_nodes_tail(static_arguments, Some("static"), tail_has_more);
            }
            dumper.dump_nodes(&self.dynamic_arguments, Some("dynamic"));
        });
    }
}

impl Dump for Cast {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Cast").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node_branch(&self.receiver, Some("receiver"), true);
            dumper.dump_node_branch(&self.r#type, Some("type"), false);
        });
    }
}

impl Dump for Coalesce {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Coalesce").end();
        dumper.with_depth(|dumper| {
            dumper.dump_node_branch(&self.receiver, Some("receiver"), true);
            dumper.dump_node_branch(&self.default, Some("default"), false);
        });
    }
}

// ----------------------------------------------------------------------------
// Matching
// ----------------------------------------------------------------------------

impl Dump for Match {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Match").end();
        dumper.with_depth(|dumper| {
            let has_cases = !self.cases.is_empty();
            dumper.dump_node_branch(&self.value, None, has_cases);
            dumper.dump_nodes(&self.cases, None);
        });
    }
}

impl Dump for MatchCase {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("MatchCase").end();
        dumper.with_depth(|dumper| match self {
            MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                let has_guard = guard.is_some();
                dumper.dump_node_branch(pattern, None, true);
                dumper.dump_node_branch(body, None, has_guard);
                if let Some(guard) = guard {
                    dumper.dump_node_branch(guard, None, false);
                }
            }
            MatchCase::Block {
                pattern,
                body,
                guard,
            } => {
                let has_guard = guard.is_some();
                dumper.dump_node_branch(pattern, None, true);
                dumper.dump_node_branch(body, None, has_guard);
                if let Some(guard) = guard {
                    dumper.dump_node_branch(guard, None, false);
                }
            }
        });
    }
}

impl Dump for Pattern {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Pattern::Wildcard => {
                dumper.node("Pattern::Wildcard").end();
            }
            Pattern::Rest => {
                dumper.node("Pattern::Rest").end();
            }
            Pattern::Pointer { target, mutability } => {
                let mut node_dumper = dumper.node("Pattern::Pointer");
                node_dumper.field("mutability", mutability);
                node_dumper.end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(target, Some("target"));
                });
            }
            Pattern::Literal(node) => {
                dumper.node_unwrap("Pattern::Literal", *node);
            }
            Pattern::Identifier(string_id) => {
                let mut node_dumper = dumper.node("Pattern::Identifier");
                node_dumper.field("identifier", string_id);
                node_dumper.end();
            }
            Pattern::Path(path) => {
                let mut node_dumper = dumper.node("Pattern::Path");
                node_dumper.field("path", path);
                node_dumper.end();
            }
            Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let mut node_dumper = dumper.node("Pattern::Range");
                node_dumper.field("is_inclusive", is_inclusive);
                node_dumper.end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node_branch(start, Some("start"), true);
                    dumper.dump_node_branch(end, Some("end"), false);
                });
            }
            Pattern::Tuple { path, fields } => {
                dumper
                    .node("Pattern::Tuple")
                    .field_optional("path", path)
                    .end();
                dumper.with_depth(|dumper| {
                    dumper.dump_nodes(fields, None);
                });
            }
            Pattern::Slice { fields } => {
                dumper.node("Pattern::Slice").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_nodes(fields, None);
                });
            }
            Pattern::Struct { r#type, fields } => {
                dumper.node("Pattern::Struct").end();
                dumper.with_depth(|dumper| {
                    let has_fields = !fields.is_empty();
                    dumper.dump_node_branch(r#type, None, has_fields);
                    dumper.dump_nodes(fields, None);
                });
            }
            Pattern::Union { fields } => {
                dumper.node("Pattern::Union").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_nodes(fields, None);
                });
            }
        }
    }
}

impl Dump for PatternField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            PatternField::Named { name, pattern } => {
                let mut node_dumper = dumper.node("PatternField::Named");
                node_dumper.field("name", name);
                node_dumper.end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(pattern, None);
                });
            }
            PatternField::NamedAlias { name, alias } => {
                let mut node_dumper = dumper.node("PatternField::NamedAlias");
                node_dumper.field("name", name);
                node_dumper.field("alias", alias);
                node_dumper.end();
            }
            PatternField::Positional { pattern } => {
                dumper.node("PatternField::Positional").end();
                dumper.with_depth(|dumper| {
                    dumper.dump_node(pattern, None);
                });
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Annotations
// ----------------------------------------------------------------------------

impl Dump for Doc {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Doc").field("string", &self.string).end();
    }
}
