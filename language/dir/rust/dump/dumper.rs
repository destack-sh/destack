#![allow(clippy::match_like_matches_macro)]

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
            self.dumper
                .write_str(format!(" :{node_id}").as_str(), Some(Color::White));
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
            self.dumper
                .write_str(format!(" :{node_id}").as_str(), Some(Color::White));
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

/// Dump a Visibility as a string.
impl Dump for Visibility {
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

/// Dump a ScopedMutability structure.
impl Dump for ScopedMutability {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ScopedMutability::Unscoped { mutability } => {
                dumper
                    .object("ScopedMutability::Unscoped")
                    .field("mutability", mutability)
                    .end();
            }
            ScopedMutability::Scoped { mutability, scopes } => {
                dumper
                    .object("ScopedMutability::Scoped")
                    .field("mutability", mutability)
                    .field("scopes", scopes)
                    .end();
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

/// Dump a Destination as a structured representation.
impl Dump for Destination {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Destination::LabelString(label) => {
                dumper
                    .object("Destination::LabelString")
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

/// Dump a Path as a structured representation.
impl Dump for Path {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Path::Intrinsic { intrinsic } => {
                dumper
                    .object("Path::Intrinsic")
                    .field("intrinsic", intrinsic)
                    .end();
            }
            Path::String { segments } => {
                dumper
                    .object("Path::String")
                    .field("segments", segments)
                    .end();
            }
            Path::Relative { segments, .. } => {
                dumper
                    .object("Path::Relative")
                    .field("segments", segments)
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

/// Dump a FloatType as a structured representation.
impl Dump for FloatType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let repr = self.as_str();
        dumper.object("FloatType").field("repr", &repr).end();
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
                dumper.object("TypeLiteral::Self_").end();
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
            Expression::Block(_) => {
                self.node("Expression::Block", id.id).end();
            }
            Expression::With {
                clauses: _,
                body: _,
            } => {
                self.node("Expression::With", id.id).end();
            }
            Expression::Use { items: _, body: _ } => {
                self.node("Expression::Use", id.id).end();
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

            Expression::Unary { operator, right: _ } => {
                self.node("Expression::Unary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Reference {
                mutability,
                right: _,
            } => {
                self.node("Expression::Reference", id.id)
                    .field("mutability", mutability)
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
                left: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::Call", id.id).end();
            }
            Expression::Index { left: _, right: _ } => {
                self.node("Expression::Index", id.id).end();
            }
            Expression::Cast { value: _, ty: _ } => {
                self.node("Expression::Cast", id.id).end();
            }
            Expression::Path { path } => {
                self.node("Expression::Path", id.id)
                    .field("path", path)
                    .end();
            }
            Expression::InlineDefinition { definition: _ } => {
                self.node("Expression::InlineDefinition", id.id).end();
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
            Expression::StructLiteral { ty: _, fields: _ } => {
                self.node("Expression::StructLiteral", id.id).end();
            }
            Expression::TupleLiteral { elements: _ } => {
                self.node("Expression::TupleLiteral", id.id).end();
            }
            Expression::ArrayLiteral { elements: _ } => {
                self.node("Expression::ArrayLiteral", id.id).end();
            }
            Expression::If {
                condition: _,
                then_block: _,
                else_block: _,
            } => {
                self.node("Expression::If", id.id).end();
            }
            Expression::Loop {
                condition: _,
                body: _,
                source,
            } => {
                self.node("Expression::Loop", id.id)
                    .field("source", source)
                    .end();
            }
            Expression::Match {
                value: _,
                cases: _,
                source,
            } => {
                self.node("Expression::Match", id.id)
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
            Expression::Defer {
                destination,
                body: _,
            } => {
                self.node("Expression::Defer", id.id)
                    .field("destination", destination)
                    .end();
            }
            Expression::Return {
                destination,
                value: _,
            } => {
                self.node("Expression::Return", id.id)
                    .field("destination", destination)
                    .end();
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
            Definition::Module {
                name,
                visibility,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Module", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Struct {
                name,
                visibility,
                static_parameters: _,
                super_types: _,
                variant: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Struct", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Enum {
                name,
                visibility,
                super_types: _,
                static_parameters: _,
                variant: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Enum", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Union {
                name,
                visibility,
                super_types: _,
                static_parameters: _,
                variants: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Union", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Trait {
                name,
                visibility,
                super_types: _,
                static_parameters: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Trait", id.id)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .end();
            }
            Definition::Function {
                name,
                visibility,
                runtime,
                static_parameters: _,
                self_parameter,
                dynamic_parameters: _,
                return_type: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Function", id.id)
                    .field("runtime", runtime)
                    .field_optional("name", name)
                    .field_optional("visibility", visibility)
                    .field_optional("self_parameter", self_parameter)
                    .end();
            }
            Definition::Implement {
                static_parameters: _,
                receiver: _,
                for_type: _,
                with_clauses: _,
                where_clauses: _,
                definitions: _,
            } => {
                self.node("Definition::Implement", id.id).end();
            }
            Definition::Let { name, visibility } => {
                self.node("Definition::Let", id.id)
                    .field("name", name)
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
            Type::Infer => {
                self.node("Type::Infer", id.id).end();
            }
            Type::Never => {
                self.node("Type::Never", id.id).end();
            }
            Type::Not(_) => {
                self.node("Type::Not", id.id).end();
            }
            Type::Maybe(_) => {
                self.node("Type::Maybe", id.id).end();
            }
            Type::Reference {
                mutability,
                target: _,
            } => {
                self.node("Type::Reference", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Type::Virtual(_) => {
                self.node("Type::Virtual", id.id).end();
            }
            Type::Variadic(_) => {
                self.node("Type::Variadic", id.id).end();
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
            Type::Union(_) => {
                self.node("Type::Union", id.id).end();
            }
            Type::Intersection(_) => {
                self.node("Type::Intersection", id.id).end();
            }
            Type::TypeLiteral(literal) => {
                self.node("Type::TypeLiteral", id.id)
                    .field("literal", literal)
                    .end();
            }
            Type::ScalarLiteral(literal) => {
                self.node("Type::ScalarLiteral", id.id)
                    .field("literal", literal)
                    .end();
            }
            Type::Self_ => {
                self.node("Type::Self_", id.id).end();
            }
            Type::Definition(_) => {
                self.node("Type::Definition", id.id).end();
            }
            Type::Expression(_) => {
                self.node("Type::Expression", id.id).end();
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
                representation_type: _,
                fields: _,
                value: _,
            } => {
                self.node("Variant::Struct", id.id)
                    .field_optional("name", name)
                    .end();
            }
            Variant::Tuple {
                name,
                representation_type: _,
                fields: _,
                value: _,
            } => {
                self.node("Variant::Tuple", id.id)
                    .field_optional("name", name)
                    .end();
            }
            Variant::Unit { name, value: _ } => {
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
            VariantField::Named { name, ty: _ } => {
                self.node("VariantField::Named", id.id)
                    .field("name", name)
                    .end();
            }
            VariantField::Positional { ty: _ } => {
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

    fn visit_use_item(&mut self, tree: &NodeTree, id: NodeId<UseItem>, use_item: &UseItem) {
        self.node("UseItem", id.id)
            .field("source", &use_item.source)
            .field("name", &use_item.name)
            .field_optional("alias", &use_item.alias)
            .end();
        self.with_depth(|dumper| {
            walk_use_item(dumper, tree, id, use_item);
        });
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        self.node("Parameter", id.id)
            .field("name", &parameter.name)
            .end();
        self.with_depth(|dumper| {
            walk_parameter(dumper, tree, id, parameter);
        });
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        match argument {
            Argument::Named { name, value: _ } => {
                self.node("Argument::Named", id.id)
                    .field("name", name)
                    .end();
            }
            Argument::Positional { value: _ } => {
                self.node("Argument::Positional", id.id).end();
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
                target: _,
                mutability,
            } => {
                self.node("Pattern::Reference", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Pattern::ScalarLiteral(value) => {
                self.node("Pattern::ScalarLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            Pattern::Binding { name } => {
                self.node("Pattern::Binding", id.id)
                    .field("name", name)
                    .end();
            }
            Pattern::Path(path) => {
                self.node("Pattern::Path", id.id).field("path", path).end();
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
            Pattern::Tuple { path, fields: _ } => {
                self.node("Pattern::Tuple", id.id)
                    .field_optional("path", path)
                    .end();
            }
            Pattern::Slice { fields: _ } => {
                self.node("Pattern::Slice", id.id).end();
            }
            Pattern::Struct { ty: _, fields: _ } => {
                self.node("Pattern::Struct", id.id).end();
            }
            Pattern::Union { fields: _ } => {
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
            Annotation::Blank { position, lines } => {
                self.node("Annotation::Blank", id.id)
                    .field("position", position)
                    .field("lines", lines)
                    .end();
            }
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
