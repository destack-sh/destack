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
//!  ├─ [right] Expression::Binary { operator: Divide }
//!  |  ├─ [left] Expression::Binary { operator: Multiply }
//!  |  │  ├─ [left] Expression::Path { path: b }
//!  |  │  ├─ [right] Expression::ScalarLiteral
//!  |  │  |  ├─ ScalarLiteral::Integer { value: 2 }
//!  |  ├─ [right] Expression::Unary { operator: Negate }
//!  |  |  ├─ Expression::ScalarLiteral
//!  |  |  |  ├─ ScalarLiteral::Integer { value: 4 }
//! ```

#![allow(clippy::match_like_matches_macro)]

use crate::{
    Argument, ArrayLiteral, AssignOperator, BinaryOperator, Block, Break, Call, Cast, Continue,
    Defer, Doc, Enum, EnumField, Expression, FieldLiteral, FloatType, For, Function, If, Implement,
    Index, IntType, Let, LetInitialization, Loop, Match, MatchCase, Module, Mutability, Node,
    NodeId, NodeTree, NodeTreeStore, Parameter, PathId, PathPool, Pattern, PatternField,
    PrimitiveType, RangeLiteral, Return, Runtime, ScalarLiteral, Statement, StringPool, Struct,
    StructField, StructLiteral, Trait, Try, Tuple, TupleField, TupleLiteral, Type, UnaryOperator,
    Union, UnionField, Use, UseClause, UseItem, While, With, WithClause,
};
use dyst_language_arena::StringId;

// The console colors.
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
        }
    }

    /// Finish dumping and return the result.
    pub fn finish(self) -> String {
        self.buffer
    }

    /// Write a string to the buffer with a new depth context.
    pub fn with_depth(&mut self, lambda: impl FnOnce(&mut Self)) {
        self.depth += 1;
        lambda(self);
        self.depth -= 1;
    }

    /// Write the prefix for the current depth.
    #[inline]
    fn write_prefix(&mut self) {
        if self.depth > 0 {
            for _ in 0..self.depth - 1 {
                self.write_str("|  ", Some(Color::Cyan));
            }
            self.write_str("├─ ", Some(Color::Cyan));
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
    pub fn node<'d>(&'d mut self, name: &str) -> NodeDumper<'d, 'a> {
        NodeDumper::new(self, name)
    }

    /// Helper for dumping a single node that just wraps another node.
    #[inline]
    pub fn node_wrapper<T: Node + Clone + Dump>(
        &mut self,
        name: &str,
        wrapped_id: NodeId<T>,
    ) -> &mut Self
    where
        NodeTree: NodeTreeStore<T>,
    {
        let mut node_dumper = self.node(name);
        node_dumper.finish();
        self.with_depth(|dumper| {
            dumper.dump(&wrapped_id, None);
        });
        self
    }

    /// Dump something as a new line.
    pub fn dump<T: Dump>(&mut self, thing: &T, label: Option<&str>) -> &mut Self {
        self.write_newline();
        self.write_prefix();
        if let Some(label) = label {
            self.write_str("[", Some(Color::White));
            self.write_str(label.as_ref(), Some(Color::White));
            self.write_str("] ", Some(Color::White));
        }
        thing.dump(self);
        self
    }

    /// Dump many somethings as a new line (each).
    pub fn dump_many<T: Dump>(&mut self, things: &[T], label: Option<&str>) -> &mut Self {
        for thing in things {
            self.write_newline();
            self.write_prefix();
            if let Some(label) = label {
                self.write_str("[", Some(Color::White));
                self.write_str(label.as_ref(), Some(Color::White));
                self.write_str("] ", Some(Color::White));
            }
            thing.dump(self);
        }
        self
    }
}

/// Helper for dumping a single node.
#[derive(Debug)]
pub struct NodeDumper<'d, 'p> {
    dumper: &'d mut Dumper<'p>,
    has_fields: bool,
}

impl<'d, 'p> NodeDumper<'d, 'p> {
    pub fn new(dumper: &'d mut Dumper<'p>, name: &str) -> Self {
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

    /// Finish node and mark the struct as non-exhaustive (with a ..)
    pub fn finish_non_exhaustive(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(", .. }", Some(Color::White));
        } else {
            self.dumper.write_str(" { .. }", Some(Color::White));
        }
        self
    }

    /// Finish node and close the struct as exhaustive.
    pub fn finish(&mut self) -> &mut Self {
        if self.has_fields {
            self.dumper.write_str(" }", Some(Color::White));
        }
        self
    }
}

pub trait Dump {
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

/// Dump a Mutability as a string.
impl Dump for Mutability {
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
            .finish();
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
                dumper.node("PrimitiveType::Void").finish();
            }
            PrimitiveType::Boolean => {
                dumper.node("PrimitiveType::Boolean").finish();
            }
            PrimitiveType::Character => {
                dumper.node("PrimitiveType::Character").finish();
            }
            PrimitiveType::Int(int_type) => {
                dumper.node("PrimitiveType::Int").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(int_type, None);
                });
            }
            PrimitiveType::Float(float_type) => {
                dumper.node("PrimitiveType::Float").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(float_type, None);
                });
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Groupings
// ----------------------------------------------------------------------------

impl Dump for Block {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Block").finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.statements, None);
        });
    }
}

impl Dump for Statement {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Statement::Expression(node) => {
                dumper.node_wrapper("Statement::Expression", *node);
            }
            Statement::Module(node) => {
                dumper.node_wrapper("Statement::Module", *node);
            }
            Statement::Struct(node) => {
                dumper.node_wrapper("Statement::Struct", *node);
            }
            Statement::Enum(node) => {
                dumper.node_wrapper("Statement::Enum", *node);
            }
            Statement::Union(node) => {
                dumper.node_wrapper("Statement::Union", *node);
            }
            Statement::Trait(node) => {
                dumper.node_wrapper("Statement::Trait", *node);
            }
            Statement::Implement(node) => {
                dumper.node_wrapper("Statement::Implement", *node);
            }
            Statement::Function(node) => {
                dumper.node_wrapper("Statement::Function", *node);
            }
            Statement::With(node) => {
                dumper.node_wrapper("Statement::With", *node);
            }
            Statement::Use(node) => {
                dumper.node_wrapper("Statement::Use", *node);
            }
            Statement::Doc(node) => {
                dumper.node_wrapper("Statement::Doc", *node);
            }
        }
    }
}

impl Dump for Expression {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Expression::Module(node) => {
                dumper.node_wrapper("Expression::Module", *node);
            }
            Expression::Struct(node) => {
                dumper.node_wrapper("Expression::Struct", *node);
            }
            Expression::Enum(node) => {
                dumper.node_wrapper("Expression::Enum", *node);
            }
            Expression::Union(node) => {
                dumper.node_wrapper("Expression::Union", *node);
            }
            Expression::Trait(node) => {
                dumper.node_wrapper("Expression::Trait", *node);
            }
            Expression::Implement(node) => {
                dumper.node_wrapper("Expression::Implement", *node);
            }
            Expression::Function(node) => {
                dumper.node_wrapper("Expression::Function", *node);
            }

            Expression::Let(node) => {
                dumper.node_wrapper("Expression::Let", *node);
            }
            Expression::Block(node) => {
                dumper.node_wrapper("Expression::Block", *node);
            }
            Expression::If(node) => {
                dumper.node_wrapper("Expression::If", *node);
            }
            Expression::While(node) => {
                dumper.node_wrapper("Expression::While", *node);
            }
            Expression::For(node) => {
                dumper.node_wrapper("Expression::For", *node);
            }
            Expression::Loop(node) => {
                dumper.node_wrapper("Expression::Loop", *node);
            }
            Expression::Break(node) => {
                dumper.node_wrapper("Expression::Break", *node);
            }
            Expression::Continue(node) => {
                dumper.node_wrapper("Expression::Continue", *node);
            }
            Expression::Defer(node) => {
                dumper.node_wrapper("Expression::Defer", *node);
            }
            Expression::Return(node) => {
                dumper.node_wrapper("Expression::Return", *node);
            }
            Expression::Try(node) => {
                dumper.node_wrapper("Expression::Try", *node);
            }
            Expression::Match(node) => {
                dumper.node_wrapper("Expression::Match", *node);
            }

            Expression::Path { path } => {
                let mut node_dumper = dumper.node("Expression::Path");
                node_dumper.field("path", path);
                node_dumper.finish();
            }
            Expression::ScalarLiteral(node) => {
                dumper.node_wrapper("Expression::ScalarLiteral", *node);
            }
            Expression::RangeLiteral(node) => {
                dumper.node_wrapper("Expression::RangeLiteral", *node);
            }
            Expression::ArrayLiteral(node) => {
                dumper.node_wrapper("Expression::ArrayLiteral", *node);
            }
            Expression::TupleLiteral(node) => {
                dumper.node_wrapper("Expression::TupleLiteral", *node);
            }
            Expression::StructLiteral(node) => {
                dumper.node_wrapper("Expression::StructLiteral", *node);
            }

            Expression::Unary { operator, right } => {
                let mut node_dumper = dumper.node("Expression::Unary");
                node_dumper.field("operator", operator);
                node_dumper.finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(right, None);
                });
            }
            Expression::Index(node) => {
                dumper.node_wrapper("Expression::Index", *node);
            }
            Expression::Call(node) => {
                dumper.node_wrapper("Expression::Call", *node);
            }
            Expression::Cast(node) => {
                dumper.node_wrapper("Expression::Cast", *node);
            }
            Expression::Unwrap(node) => {
                dumper.node_wrapper("Expression::Unwrap", *node);
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let mut node_dumper = dumper.node("Expression::Binary");
                node_dumper.field("operator", operator);
                node_dumper.finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(left, Some("left"));
                    dumper.dump(right, Some("right"));
                });
            }
            Expression::Assign {
                left,
                operator,
                right,
            } => {
                let mut node_dumper = dumper.node("Expression::Assign");
                node_dumper.field("operator", operator);
                node_dumper.finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(left, Some("left"));
                    dumper.dump(right, Some("right"));
                });
            }

            Expression::Error => {
                dumper.node("Expression::Error").finish();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Declarations
// ----------------------------------------------------------------------------

impl Dump for Module {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Module").field("name", &self.name).finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.body, None);
        });
    }
}

impl Dump for Struct {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Struct").field("name", &self.name).finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.fields, None);
            dumper.dump_many(&self.usings, None);
            dumper.dump_many(&self.lets, None);
        });
    }
}

impl Dump for StructField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("StructField")
            .field("name", &self.name)
            .finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.r#type, None);
            if let Some(default) = self.default {
                dumper.dump(&default, None);
            }
        });
    }
}

impl Dump for Enum {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Enum").field("name", &self.name).finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.fields, None);
        });
    }
}

impl Dump for EnumField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("EnumField").finish();
    }
}

impl Dump for Union {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Union").field("name", &self.name).finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.fields, None);
        });
    }
}

impl Dump for UnionField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("UnionField").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.r#type, None);
            if let Some(value) = self.value {
                dumper.dump(&value, None);
            }
        });
    }
}

impl Dump for Trait {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Trait").field("name", &self.name).finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.withs, None);
            dumper.dump_many(&self.usings, None);
            dumper.dump_many(&self.lets, None);
            dumper.dump_many(&self.functions, None);
        });
    }
}
impl Dump for Implement {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Implement").finish();
        dumper.with_depth(|dumper| {
            if let Some(static_arguments) = &self.static_arguments {
                dumper.dump_many(static_arguments, Some("static"));
            }
            dumper.dump(&self.receiver, Some("receiver"));
            if let Some(for_trait) = &self.for_trait {
                dumper.dump(for_trait, None);
            }
            dumper.dump_many(&self.lets, None);
            dumper.dump_many(&self.functions, None);
        });
    }
}

impl Dump for Type {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Type::Infer => {
                dumper.node("Type::Infer").finish();
            }
            Type::Maybe(inner) => {
                dumper.node("Type::Maybe").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(inner, None);
                });
            }
            Type::Not(inner) => {
                dumper.node("Type::Not").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(inner, None);
                });
            }
            Type::Never => {
                dumper.node("Type::Never").finish();
            }
            Type::Self_ => {
                dumper.node("Type::Self_").finish();
            }
            Type::Primitive(primitive) => {
                dumper
                    .node("Type::Primitive")
                    .field("primitive", primitive)
                    .finish();
            }
            Type::Path {
                path,
                static_arguments,
            } => {
                dumper.node("Type::Path").field("path", path).finish();
                dumper.with_depth(|dumper| {
                    if let Some(static_arguments) = static_arguments {
                        dumper.dump_many(static_arguments, Some("static"));
                    }
                });
            }
            Type::Pointer { mutability, target } => {
                dumper
                    .node("Type::Pointer")
                    .field("mutability", mutability)
                    .finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(target, None);
                });
            }
            Type::Array {
                element_type,
                count,
            } => {
                dumper.node("Type::Array").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(element_type, None);
                    dumper.dump(count, Some("count"));
                });
            }
            Type::Slice { element } => {
                dumper.node("Type::Slice").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(element, None);
                });
            }
            Type::Tuple(tuple) => {
                dumper.node("Type::Tuple").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(tuple, None);
                });
            }
            Type::Struct(struct_node) => {
                dumper.node("Type::Struct").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(struct_node, None);
                });
            }
            Type::Enum(enum_node) => {
                dumper.node("Type::Enum").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(enum_node, None);
                });
            }
            Type::Intersection(types) => {
                dumper.node("Type::Intersection").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump_many(types, None);
                });
            }
            Type::Union(union_node) => {
                dumper.node("Type::Union").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(union_node, None);
                });
            }
            Type::Function(function) => {
                dumper.node("Type::Function").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(function, None);
                });
            }
        }
    }
}

impl Dump for Tuple {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Tuple").finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.elements, None);
        });
    }
}

impl Dump for TupleField {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            TupleField::Named { name, r#type } => {
                dumper
                    .node("TupleField::Named")
                    .field("name", name)
                    .finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(r#type, None);
                });
            }
            TupleField::Positional { r#type } => {
                dumper.node("TupleField::Positional").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(r#type, None);
                });
            }
        }
    }
}

impl Dump for Function {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Function").field("name", &self.name).finish();
        dumper.with_depth(|dumper| {
            if let Some(static_parameters) = &self.static_parameters {
                dumper.dump_many(static_parameters, Some("static"));
            }
            dumper.dump_many(&self.dynamic_parameters, Some("dynamic"));
            if let Some(return_type) = &self.return_type {
                dumper.dump(return_type, Some("return"));
            }
            if let Some(with) = &self.with {
                dumper.dump(with, None);
            }
            if let Some(body) = &self.body {
                dumper.dump(body, None);
            }
        });
    }
}

// ----------------------------------------------------------------------------
// Context
// ----------------------------------------------------------------------------
impl Dump for With {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("With").finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.clauses, None);
        });
    }
}

impl Dump for WithClause {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            WithClause::Declaration { target, alias } => {
                dumper
                    .node("WithClause::Declaration")
                    .field("alias", alias)
                    .finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(target, None);
                });
            }
            WithClause::Assertion { target, assertion } => {
                dumper.node("WithClause::Assertion").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(target, None);
                    dumper.dump(assertion, None);
                });
            }
        }
    }
}

impl Dump for Use {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Use").finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.clauses, None);
            if let Some(body) = &self.body {
                dumper.dump(body, None);
            }
        });
    }
}

impl Dump for UseClause {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("UseClause")
            .field("alias", &self.alias)
            .finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.target, None);
            if let Some(items) = &self.items {
                dumper.dump_many(items, None);
            }
        });
    }
}

impl Dump for UseItem {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .node("UseItem")
            .field("name", &self.name)
            .field("alias", &self.alias)
            .finish();
    }
}

// ----------------------------------------------------------------------------
// Control
// ----------------------------------------------------------------------------

impl Dump for If {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("If").finish();
        match self {
            If::If {
                condition,
                then_block,
            } => {
                dumper.with_depth(|dumper| {
                    dumper.dump(condition, Some("condition"));
                    dumper.dump(then_block, Some("then"));
                });
            }
            If::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                dumper.with_depth(|dumper| {
                    dumper.dump(condition, Some("condition"));
                    dumper.dump(then_block, Some("then"));
                    dumper.dump(else_block, Some("else"));
                });
            }
            If::IfElseIf {
                condition,
                then_block,
                else_if,
            } => {
                dumper.with_depth(|dumper| {
                    dumper.dump(condition, Some("condition"));
                    dumper.dump(then_block, Some("then"));
                    dumper.dump(else_if, Some("else_if"));
                });
            }
        }
    }
}

impl Dump for While {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("While").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.condition, Some("condition"));
            dumper.dump(&self.body, Some("body"));
        });
    }
}

impl Dump for For {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("For").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.pattern, Some("pattern"));
            dumper.dump(&self.iterator, Some("iterator"));
            dumper.dump(&self.body, Some("body"));
        });
    }
}

impl Dump for Loop {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Loop").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.body, Some("body"));
        });
    }
}

impl Dump for Break {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Break").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.label, Some("label"));
        });
    }
}

impl Dump for Continue {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Continue").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.label, Some("label"));
        });
    }
}

impl Dump for Defer {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Defer::Expression(expression) => {
                dumper.node("Defer::Expression").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(expression, Some("expression"));
                });
            }
            Defer::Block(block) => {
                dumper.node("Defer::Block").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(block, Some("block"));
                });
            }
        }
    }
}

impl Dump for Return {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Return").finish();
        dumper.with_depth(|dumper| {
            if let Some(value) = self.value {
                dumper.dump(&value, Some("value"));
            }
        });
    }
}

impl Dump for Try {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Try").finish();
        match self {
            Try::Expression { try_expression } => {
                dumper.with_depth(|dumper| {
                    dumper.dump(try_expression, Some("expression"));
                });
            }
            Try::Block { try_block } => {
                dumper.with_depth(|dumper| {
                    dumper.dump(try_block, Some("block"));
                });
            }
            Try::BlockWithCatch {
                try_block,
                catch_match,
            } => {
                dumper.with_depth(|dumper| {
                    dumper.dump(try_block, Some("block"));
                    dumper.dump(catch_match, Some("catch"));
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
            .field("initialization", &self.initialization)
            .finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.pattern, None);
            if let Some(r#type) = self.r#type {
                dumper.dump(&r#type, None);
            }
            if let Some(value) = self.value {
                dumper.dump(&value, None);
            }
        });
    }
}

impl Dump for Parameter {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.with_depth(|dumper| {
            dumper.node("Parameter").field("name", &self.name).finish();
            if let Some(r#type) = self.r#type {
                dumper.dump(&r#type, Some("type"));
            }
            if let Some(default) = self.default {
                dumper.dump(&default, Some("default"));
            }
        });
    }
}

impl Dump for Argument {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.with_depth(|dumper| match self {
            Argument::Named { name, value } => {
                let mut node_dumper = dumper.node("Argument::Named");
                node_dumper.field("name", name);
                node_dumper.finish();
                dumper.dump(value, Some("value"));
            }
            Argument::NamedShorthand { name } => {
                dumper
                    .node("Argument::NamedShorthand")
                    .field("name", name)
                    .finish();
            }
            Argument::Positional { value } => {
                dumper.node("Argument::Positional").finish();
                dumper.dump(value, Some("value"));
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
            ScalarLiteral::Boolean(value) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Boolean");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::Byte(value) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Byte");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::Integer(value, _) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Integer");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::Float(value, _) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Float");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::Character(value) => {
                let mut node_dumper = dumper.node("ScalarLiteral::Character");
                node_dumper.field("value", &value.to_string().as_str());
                node_dumper.finish();
            }
            ScalarLiteral::String(value) => {
                let mut node_dumper = dumper.node("ScalarLiteral::String");
                node_dumper.field("value", value);
                node_dumper.finish();
            }
            ScalarLiteral::ByteString(value) => {
                let mut node_dumper = dumper.node("ScalarLiteral::ByteString");
                node_dumper.field("value", value);
                node_dumper.finish();
            }
        }
    }
}

impl Dump for RangeLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("RangeLiteral").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.start, Some("start"));
            dumper.dump(&self.end, Some("end"));
        });
    }
}

impl Dump for ArrayLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ArrayLiteral::Fixed { elements } => {
                dumper.node("ArrayLiteral::Fixed").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump_many(elements, Some("element"));
                });
            }
            ArrayLiteral::Repeated { element, count } => {
                dumper.node("ArrayLiteral::Repeated").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(element, Some("element"));
                    dumper.dump(count, Some("count"));
                });
            }
        }
    }
}

impl Dump for TupleLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("TupleLiteral").finish();
        dumper.with_depth(|dumper| {
            dumper.dump_many(&self.elements, Some("element"));
        });
    }
}

impl Dump for StructLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("StructLiteral").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.r#type, Some("type"));
            dumper.dump_many(&self.fields, Some("field"));
        });
    }
}

impl Dump for FieldLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            FieldLiteral::Named { name, value } => {
                dumper.node("FieldLiteral::Named").finish();
                dumper.dump(name, Some("name"));
                dumper.with_depth(|dumper| {
                    dumper.dump(value, Some("value"));
                });
            }
            FieldLiteral::NamedShorthand { name } => {
                dumper.dump(name, Some("name"));
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Calls
// ----------------------------------------------------------------------------

impl Dump for Index {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Index").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.receiver, Some("receiver"));
            dumper.dump(&self.index, Some("index"));
        });
    }
}

impl Dump for Call {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Call").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.receiver, Some("receiver"));
            if let Some(static_arguments) = &self.static_arguments {
                dumper.dump_many(static_arguments, Some("static"));
            }
            dumper.dump_many(&self.dynamic_arguments, Some("dynamic"));
        });
    }
}

impl Dump for Cast {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Cast").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.receiver, Some("receiver"));
            dumper.dump(&self.r#type, Some("type"));
        });
    }
}

// ----------------------------------------------------------------------------
// Matching
// ----------------------------------------------------------------------------

impl Dump for Match {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Match").finish();
        dumper.with_depth(|dumper| {
            dumper.dump(&self.value, Some("value"));
            dumper.dump_many(&self.cases, None);
        });
    }
}

impl Dump for MatchCase {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("MatchCase").finish();
        dumper.with_depth(|dumper| match self {
            MatchCase::Expression {
                pattern,
                body,
                guard,
            } => {
                dumper.dump(pattern, Some("pattern"));
                dumper.dump(body, Some("body"));
                dumper.dump(guard, Some("guard"));
            }
            MatchCase::Block {
                pattern,
                body,
                guard,
            } => {
                dumper.dump(pattern, Some("pattern"));
                dumper.dump(body, Some("body"));
                dumper.dump(guard, Some("guard"));
            }
        });
    }
}

impl Dump for Pattern {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Pattern::Wildcard => {
                dumper.node("Pattern::Wildcard").finish();
            }
            Pattern::Rest => {
                dumper.node("Pattern::Rest").finish();
            }
            Pattern::Pointer { target, mutability } => {
                let mut node_dumper = dumper.node("Pattern::Pointer");
                node_dumper.field("mutability", mutability);
                node_dumper.finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(target, Some("target"));
                });
            }
            Pattern::Literal(node) => {
                dumper.node_wrapper("Pattern::Literal", *node);
            }
            Pattern::Identifier(string_id) => {
                let mut node_dumper = dumper.node("Pattern::Identifier");
                node_dumper.field("identifier", string_id);
                node_dumper.finish();
            }
            Pattern::Range {
                start,
                end,
                is_inclusive,
            } => {
                let mut node_dumper = dumper.node("Pattern::Range");
                node_dumper.field("is_inclusive", is_inclusive);
                node_dumper.finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(start, Some("start"));
                    dumper.dump(end, Some("end"));
                });
            }
            Pattern::Tuple { fields } => {
                dumper.node("Pattern::Tuple").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump_many(fields, None);
                });
            }
            Pattern::Slice { fields } => {
                dumper.node("Pattern::Slice").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump_many(fields, None);
                });
            }
            Pattern::Struct { r#type, fields } => {
                dumper.node("Pattern::Struct").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(r#type, None);
                    dumper.dump_many(fields, None);
                });
            }
            Pattern::Union { fields } => {
                dumper.node("Pattern::Union").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump_many(fields, None);
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
                node_dumper.finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(pattern, None);
                });
            }
            PatternField::NamedAlias { name, alias } => {
                let mut node_dumper = dumper.node("PatternField::NamedAlias");
                node_dumper.field("name", name);
                node_dumper.field("alias", alias);
                node_dumper.finish();
            }
            PatternField::Positional { pattern } => {
                dumper.node("PatternField::Positional").finish();
                dumper.with_depth(|dumper| {
                    dumper.dump(pattern, None);
                });
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Documentation
// ----------------------------------------------------------------------------

impl Dump for Doc {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.node("Doc").field("string", &self.string).finish();
    }
}
