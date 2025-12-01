#![allow(clippy::match_like_matches_macro)]

use destack_source::{Color, ImmutableStringPool, impl_dump_display, rebuild_tree_output};
use smallvec::{Array, SmallVec};

use crate::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct DumperOptions {
    /// Use colors.
    pub use_colors: bool = true,
}

/// A Dumper for dumping DIR nodes.
#[derive(Debug)]
pub struct Dumper<'a> {
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
    /// The node tree.
    pub tree: &'a NodeTree,
    /// The dump options.
    pub options: DumperOptions,
    /// The visitor options.
    visitor_options: NodeVisitorOptions,

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
        strings: &'a ImmutableStringPool,
        tree: &'a NodeTree,
        options: DumperOptions,
    ) -> Self {
        Self {
            strings,
            tree,
            options,
            visitor_options: NodeVisitorOptions::default(),
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
    fn write_str(&mut self, s: impl AsRef<str>, color: Option<Color>) {
        if self.options.use_colors {
            if let Some(color) = color {
                self.buffer.push_str(color.apply(s.as_ref()).as_str());
            } else {
                self.buffer.push_str(s.as_ref());
            }
        } else {
            self.buffer.push_str(s.as_ref());
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

    /// Write the path represented by a Path.
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
        let parent_id = self.tree.get_parent_id(id);
        self.node_like(name, Some(id), parent_id)
    }

    /// Helper for dumping a single node like thing.
    #[inline]
    pub fn node_like<'d>(
        &'d mut self,
        name: &str,
        id: Option<u32>,
        parent_id: Option<u32>,
    ) -> StructDumper<'d, 'a> {
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
        StructDumper::new(self, name, id, parent_id)
    }

    /// Helper for dumping a single struct.
    #[inline]
    pub fn object<'d>(&'d mut self, name: &str) -> StructDumper<'d, 'a> {
        StructDumper::new(self, name, None, None)
    }
}

/// Helper for dumping a single struct-like type.
#[derive(Debug)]
pub struct StructDumper<'d, 'p> {
    dumper: &'d mut Dumper<'p>,
    node_id: Option<u32>,
    _parent_id: Option<u32>,
    has_fields: bool,
}

impl<'d, 'p> StructDumper<'d, 'p> {
    /// Begin a new struct-like dumper with some name.
    pub fn new(
        dumper: &'d mut Dumper<'p>,
        name: &str,
        node_id: Option<u32>,
        parent_id: Option<u32>,
    ) -> Self {
        dumper.write_str(name, Some(Color::BrightBlue));
        Self {
            dumper,
            node_id,
            _parent_id: parent_id,
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
            self.dumper
                .write_str(format!(" :{node_id}").as_str(), Some(Color::White));
            self.dumper.write_str("\n", Some(Color::White));
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
        dumper.write_str(self, Some(Color::BrightYellow));
        dumper.write_char('"', Some(Color::White));
    }
}

/// Dump a String as a string.
impl Dump for String {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_char('"', Some(Color::White));
        dumper.write_str(self, Some(Color::BrightYellow));
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

/// Dump a Box<T> as a string.
impl<T: Dump> Dump for Box<T> {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        self.as_ref().dump(dumper)
    }
}

/// Dump a Vec<T> as a slice.
impl<T: Dump> Dump for Vec<T> {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        self.as_slice().dump(dumper)
    }
}

/// Dump a SmallVec<A> as a slice.
impl<A: Array> Dump for SmallVec<A>
where
    A::Item: Dump,
{
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        self.as_slice().dump(dumper)
    }
}

/// Dump a bool as a string.
impl Dump for bool {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(if *self { "true" } else { "false" }, Some(Color::Green))
    }
}

/// Dump a u8 as a string.
impl Dump for u8 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(self.to_string(), Some(Color::Green))
    }
}

/// Dump a u16 as a string.
impl Dump for u16 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(self.to_string(), Some(Color::Green))
    }
}

/// Dump a u32 as a string.
impl Dump for u32 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(self.to_string(), Some(Color::Green))
    }
}

/// Dump an i64 as a string.
impl Dump for i64 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(self.to_string(), Some(Color::Green))
    }
}

/// Dump an f64 as a string.
impl Dump for f64 {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(self.to_string(), Some(Color::Green))
    }
}

/// Dump a char as a quoted character.
impl Dump for char {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_char('\'', None);
        dumper.write_str(self.to_string(), Some(Color::Green));
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

/// Dump a StringId as a string.
impl Dump for StringId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_string_id(*self);
    }
}

/// Dump a NodeId<T> as the node it points to.
impl<T: Node + Clone + Dump> Dump for LocalNodeId<T>
where
    NodeTree: NodeTreeImpl<T>,
{
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let node = dumper.tree.get(*self);
        node.dump(dumper);
    }
}

/// Dump a ModuleId as a string.
impl Dump for ModuleId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(self.0.to_string(), Some(Color::White));
    }
}

impl_dump_display! {
    AnnotationPosition,
    Asynchrony,
    AssignOperator,
    BindingKind,
    BindingOperator,
    BindingAnchor,
    BinaryOperator,
    DeclarationKind,
    DependencyKind,
    DependencySource,
    DependencyMode,
    ForEachKind,
    FunctionAbstraction,
    FunctionCardinality,
    FunctionKind,
    FunctionMode,
    IfKind,
    LoopKind,
    MatchSource,
    Mutability,
    ReferenceType,
    Runtime,
    TypeBinaryOperator,
    TypeUnaryOperator,
    UnaryOperator,
    TypeKind,
    VarianceBound,
    Visibility,
    WhileKind,
    YieldCardinality,
}

/// Dump a BindingModifier as a string.
impl Dump for BindingModifier {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("BindingModifier")
            .field_optional("kind", &self.kind)
            .field_optional("anchor", &self.anchor)
            .field_optional("mutability", &self.mutability)
            .field_optional("visibility", &self.visibility)
            .field_optional("operator", &self.operator)
            .end();
    }
}

/// Dump a DeclarationDescriptor as a structured object.
impl Dump for DeclarationDescriptor {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("DeclarationDescriptor")
            .field("kind", &self.kind)
            .field_optional("name", &self.name)
            .field_optional("export", &self.export)
            .field("symbol", &self.symbol)
            .end();
    }
}

/// Dump a Key as a structured representation.
impl Dump for Key {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Key::Name(name) => {
                dumper.object("Key::Name").value(name).end();
            }
            Key::Expression(_) => {
                dumper.object("Key::Expression").end();
            }
            Key::NamedExpression { name, key: _ } => {
                dumper
                    .object("Key::NamedExpression")
                    .field("name", name)
                    .end();
            }
        }
    }
}

/// Dump a Generics as a structured object.
impl Dump for Generics {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.object("Generics").end();
    }
}

/// Dump a Heritage as a structured object.
impl Dump for Heritage {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.object("Heritage").end();
    }
}

/// Dump a FunctionSignature as a structured object.
impl Dump for FunctionSignature {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("FunctionSignature")
            .field("abstraction", &self.abstraction)
            .field("asynchrony", &self.asynchrony)
            .field("cardinality", &self.cardinality)
            .field_optional("mode", &self.mode)
            .field("kind", &self.kind)
            .end();
    }
}

/// Dump a Path as a structured representation.
impl Dump for Path {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_path(self);
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
            IntType::IntP => dumper.object("IntType::IntSize").end(),
            IntType::Uint8 => dumper.object("IntType::Uint8").end(),
            IntType::Uint16 => dumper.object("IntType::Uint16").end(),
            IntType::Uint32 => dumper.object("IntType::Uint32").end(),
            IntType::Uint64 => dumper.object("IntType::Uint64").end(),
            IntType::Uint128 => dumper.object("IntType::Uint128").end(),
            IntType::Uint256 => dumper.object("IntType::Uint256").end(),
            IntType::UintP => dumper.object("IntType::UintSize").end(),
            IntType::Arbitrary { width, is_signed } => dumper
                .object("IntType::Arbitrary")
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
            FloatType::Float32 => dumper.object("FloatType::Float32").end(),
            FloatType::Float64 => dumper.object("FloatType::Float64").end(),
            FloatType::Arbitrary { width } => dumper
                .object("FloatType::Arbitrary")
                .field("width", width)
                .end(),
        };
    }
}

/// Dump a DeclarationType as a structured representation.
impl Dump for DeclarationType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            DeclarationType::Type => {
                dumper.object("DeclarationType::Type").end();
            }
            DeclarationType::Namespace => {
                dumper.object("DeclarationType::Module").end();
            }
            DeclarationType::Struct => {
                dumper.object("DeclarationType::Struct").end();
            }
            DeclarationType::Class => {
                dumper.object("DeclarationType::Class").end();
            }
            DeclarationType::Enum => {
                dumper.object("DeclarationType::Enum").end();
            }
            DeclarationType::Union => {
                dumper.object("DeclarationType::Union").end();
            }
            DeclarationType::Interface => {
                dumper.object("DeclarationType::Interface").end();
            }
            DeclarationType::Extension => {
                dumper.object("DeclarationType::Extension").end();
            }
            DeclarationType::Function => {
                dumper.object("DeclarationType::Function").end();
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
            PrimitiveType::Bigint => {
                dumper.object("TypeLiteral::Bigint").end();
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
            PrimitiveType::Symbol => {
                dumper.object("TypeLiteral::Symbol").end();
            }
            PrimitiveType::UniqueSymbol => {
                dumper.object("TypeLiteral::UniqueSymbol").end();
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
                dumper.object("ScalarLiteral::Boolean").value(value).end();
            }
            ScalarLiteral::Byte(value) => {
                dumper.object("ScalarLiteral::Byte").value(value).end();
            }
            ScalarLiteral::Integer(value) => {
                dumper.object("ScalarLiteral::Integer").value(value).end();
            }
            ScalarLiteral::Bigint(value) => {
                dumper.object("ScalarLiteral::Bigint").value(value).end();
            }
            ScalarLiteral::Float(value) => {
                dumper.object("ScalarLiteral::Float").value(value).end();
            }
            ScalarLiteral::Character(value) => {
                dumper.object("ScalarLiteral::Character").value(value).end();
            }
            ScalarLiteral::String(value) => {
                dumper.object("ScalarLiteral::String").value(value).end();
            }
            ScalarLiteral::RegexString { content, flags } => {
                dumper
                    .object("ScalarLiteral::RegexString")
                    .value(content)
                    .field_optional("flags", flags)
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
            TemplateLiteral::InterpolatedString {
                strings: template,
                arguments: _,
            } => {
                dumper
                    .object("TemplateLiteral::InterpolatedString")
                    .field("template", template)
                    .end();
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Nodes
// ----------------------------------------------------------------------------

impl<'a> NodeVisitor for Dumper<'a> {
    #[inline]
    fn options(&self) -> &NodeVisitorOptions {
        &self.visitor_options
    }

    fn visit_any(&mut self, tree: &NodeTree, _ty: NodeType, id: u32) {
        let annotations = tree.get_annotations(id);
        for annotation_id in annotations {
            let annotation = tree.get(annotation_id);
            self.visit_annotation(tree, annotation_id, annotation);
        }
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Declaration { declaration: _ } => {
                self.node("Expression::Declaration", id.id).end();
            }
            Expression::Block { block: _ } => {
                self.node("Expression::Block", id.id).end();
            }
            Expression::Statement { statement: _ } => {
                self.node("Expression::Statement", id.id).end();
            }

            Expression::With {
                clauses: _,
                scope,
                symbol,
                body: _,
            } => {
                self.node("Expression::With", id.id)
                    .field("scope", scope)
                    .field("symbol", symbol)
                    .end();
            }
            Expression::UnresolvedImport {
                kind,
                target,
                items: _,
                arguments: _,
            } => {
                self.node("Expression::UnresolvedImport", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .end();
            }
            Expression::Import {
                kind,
                target,
                target_module,
                items: _,
                arguments: _,
            } => {
                self.node("Expression::Import", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .field("target_module", target_module)
                    .end();
            }
            Expression::UnresolvedReExport {
                target,
                kind,
                items: _,
            } => {
                self.node("Expression::UnresolvedReExport", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .end();
            }
            Expression::ReExport {
                target,
                target_module,
                kind,
                items: _,
            } => {
                self.node("Expression::ReExport", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .field("target_module", target_module)
                    .end();
            }
            Expression::Export { kind, items: _ } => {
                self.node("Expression::Export", id.id)
                    .field("kind", kind)
                    .end();
            }
            Expression::Let {
                descriptor,
                mutability,
                pattern: _,
                value: _,
            } => {
                self.node("Expression::Let", id.id)
                    .field("descriptor", descriptor)
                    .field("mutability", mutability)
                    .end();
            }
            Expression::Unary { operator, right: _ } => {
                self.node("Expression::Unary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::TypeUnary { operator, right: _ } => {
                self.node("Expression::TypeUnary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::ValueOf {
                mutability,
                variance,
                right: _,
            } => {
                self.node("Expression::ValueOf", id.id)
                    .field_optional("mutability", mutability)
                    .field_optional("variance", variance)
                    .end();
            }
            Expression::ReferenceOf {
                mutability,
                variance,
                right: _,
            } => {
                self.node("Expression::ReferenceOf", id.id)
                    .field_optional("mutability", mutability)
                    .field_optional("variance", variance)
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
            Expression::TypeBinary {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::TypeBinary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Assign { left: _, right: _ } => {
                self.node("Expression::Assign", id.id).end();
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
            Expression::Member {
                left: _,
                name,
                static_arguments: _,
            } => {
                self.node("Expression::Member", id.id)
                    .field("name", name)
                    .end();
            }
            Expression::Call {
                left: _,
                static_arguments: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::Call", id.id).end();
            }
            Expression::New {
                left: _,
                static_arguments: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::New", id.id).end();
            }
            Expression::Delete { value: _ } => {
                self.node("Expression::Delete", id.id).end();
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

            Expression::UnresolvedAbsolutePath {
                path,
                static_arguments: _,
            } => {
                self.node("Expression::UnresolvedAbsolutePath", id.id)
                    .field("path", path)
                    .end();
            }
            Expression::UnresolvedRelativePath {
                remaining_path,
                target_symbol,
                path,
                static_arguments: _,
            } => {
                self.node("Expression::UnresolvedRelativePath", id.id)
                    .field("remaining_path", remaining_path)
                    .field("target_symbol", target_symbol)
                    .field("path", path)
                    .end();
            }
            Expression::LocalReference {
                path,
                static_arguments: _,
                target_symbol,
            } => {
                self.node("Expression::LocalReference", id.id)
                    .field("path", path)
                    .field("target_symbol", target_symbol)
                    .end();
            }
            Expression::ModuleReference {
                path,
                static_arguments: _,
                target_symbol,
            } => {
                self.node("Expression::ModuleReference", id.id)
                    .field("path", path)
                    .field("target_symbol", target_symbol)
                    .end();
            }
            Expression::GlobalReference {
                path,
                static_arguments: _,
                target_symbol,
            } => {
                self.node("Expression::GlobalReference", id.id)
                    .field("path", path)
                    .field("target_symbol", target_symbol)
                    .end();
            }

            Expression::Type { ty: _ } => {
                self.node("Expression::Type", id.id).end();
            }
            Expression::ScalarLiteral { value } => {
                self.node("Expression::ScalarLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            Expression::TaggedTemplateExpression { tag: _, value } => {
                self.node("Expression::TaggedTemplateExpression", id.id)
                    .field("value", value)
                    .end();
            }
            Expression::TemplateExpression { value } => {
                self.node("Expression::TemplateExpression", id.id)
                    .field("value", value)
                    .end();
            }
            Expression::TypeLiteral { value } => {
                self.node("Expression::TypeLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            Expression::RangeExpression {
                start: _,
                end: _,
                is_inclusive,
            } => {
                self.node("Expression::RangeExpression", id.id)
                    .field("is_inclusive", is_inclusive)
                    .end();
            }
            Expression::ArrayExpression { elements: _ } => {
                self.node("Expression::ArrayExpression", id.id).end();
            }
            Expression::TupleExpression { elements: _ } => {
                self.node("Expression::TupleExpression", id.id).end();
            }
            Expression::ObjectExpression { properties: _ } => {
                self.node("Expression::ObjectExpression", id.id).end();
            }
            Expression::TreeExpression {
                left: _,
                arguments: _,
                elements: _,
            } => {
                self.node("Expression::TreeExpression", id.id).end();
            }
            Expression::TaggedScalarExpression { ty: _, value: _ } => {
                self.node("Expression::TaggedScalarExpression", id.id).end();
            }
            Expression::TaggedTupleExpression { ty: _, elements: _ } => {
                self.node("Expression::TaggedTupleExpression", id.id).end();
            }
            Expression::TaggedObjectExpression {
                ty: _,
                properties: _,
            } => {
                self.node("Expression::TaggedObjectExpression", id.id).end();
            }
            Expression::Parenthesized { expression: _ } => {
                self.node("Expression::Parenthesized", id.id).end();
            }

            Expression::If {
                kind,
                condition: _,
                then_expression: _,
                else_expression: _,
            } => {
                self.node("Expression::If", id.id).field("kind", kind).end();
            }
            Expression::Loop {
                kind,
                condition: _,
                body: _,
                symbol,
                scope,
            } => {
                self.node("Expression::Loop", id.id)
                    .field("kind", kind)
                    .field("scope", scope)
                    .field("symbol", symbol)
                    .end();
            }
            Expression::ForEach {
                asynchrony,
                kind,
                pattern: _,
                iterator: _,
                body: _,
                scope,
                symbol,
            } => {
                self.node("Expression::ForEach", id.id)
                    .field("asynchrony", asynchrony)
                    .field("kind", kind)
                    .field("scope", scope)
                    .field("symbol", symbol)
                    .end();
            }
            Expression::For {
                initialization: _,
                condition: _,
                increment: _,
                body: _,
                scope,
                symbol,
            } => {
                self.node("Expression::For", id.id)
                    .field("scope", scope)
                    .field("symbol", symbol)
                    .end();
            }
            Expression::Try {
                try_expression: _,
                catch_pattern: _,
                catch_expression: _,
                finally_expression: _,
                scope,
                symbol,
            } => {
                self.node("Expression::Try", id.id)
                    .field("scope", scope)
                    .field("symbol", symbol)
                    .end();
            }
            Expression::Match {
                value: _,
                cases: _,
                source,
                scope,
                symbol,
            } => {
                self.node("Expression::Match", id.id)
                    .field("source", source)
                    .field("scope", scope)
                    .field("symbol", symbol)
                    .end();
            }
            Expression::UnresolvedBreak { target, value: _ } => {
                self.node("Expression::UnresolvedBreak", id.id)
                    .field_optional("target", target)
                    .end();
            }
            Expression::Break { target, value: _ } => {
                self.node("Expression::Break", id.id)
                    .field("target", target)
                    .end();
            }
            Expression::UnresolvedContinue { target } => {
                self.node("Expression::UnresolvedContinue", id.id)
                    .field_optional("target", target)
                    .end();
            }
            Expression::Continue { target } => {
                self.node("Expression::Continue", id.id)
                    .field("target", target)
                    .end();
            }
            Expression::Defer { expression: _ } => {
                self.node("Expression::Defer", id.id).end();
            }
            Expression::Throw { value: _ } => {
                self.node("Expression::Throw", id.id).end();
            }
            Expression::Await { expression: _ } => {
                self.node("Expression::Await", id.id).end();
            }
            Expression::Yield {
                cardinality,
                value: _,
            } => {
                self.node("Expression::Yield", id.id)
                    .field("cardinality", cardinality)
                    .end();
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

    fn visit_static_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<StaticExpression>,
        static_expression: &StaticExpression,
    ) {
        match static_expression {
            StaticExpression::Unevaluated { node: _ } => {
                self.node("StaticExpression::Unevaluated", id.id).end();
            }

            StaticExpression::Declaration {
                declaration: _,
                static_arguments: _,
            } => {
                self.node("StaticExpression::Declaration", id.id).end();
            }
            StaticExpression::Type { ty: _ } => {
                self.node("StaticExpression::Type", id.id).end();
            }

            StaticExpression::TypeLiteral { value } => {
                self.node("StaticExpression::TypeLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            StaticExpression::ScalarLiteral { value } => {
                self.node("StaticExpression::ScalarLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            StaticExpression::RangeExpression {
                start: _,
                end: _,
                is_inclusive,
            } => {
                self.node("StaticExpression::RangeExpression", id.id)
                    .field("is_inclusive", is_inclusive)
                    .end();
            }
            StaticExpression::ArrayExpression { elements: _ } => {
                self.node("StaticExpression::ArrayExpression", id.id).end();
            }
            StaticExpression::TupleExpression { elements: _ } => {
                self.node("StaticExpression::TupleExpression", id.id).end();
            }
            StaticExpression::ObjectExpression { properties: _ } => {
                self.node("StaticExpression::ObjectExpression", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_static_expression(dumper, tree, id, static_expression);
        });
    }

    fn visit_block(&mut self, tree: &NodeTree, id: LocalNodeId<Block>, block: &Block) {
        self.node("Block", id.id).end();
        self.with_depth(|dumper| {
            walk_block(dumper, tree, id, block);
        });
    }

    fn visit_declaration(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        match declaration {
            Declaration::Namespace {
                descriptor,
                generics,
                expressions: _,
                scope,
            } => {
                self.node("Declaration::Module", id.id)
                    .field("descriptor", descriptor)
                    .field("generics", generics)
                    .field("scope", scope)
                    .end();
            }
            Declaration::Type {
                descriptor,
                kind,
                mutability,
                static_parameters: _,
                value: _,
            } => {
                self.node("Declaration::Type", id.id)
                    .field("descriptor", descriptor)
                    .field("kind", kind)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Declaration::Struct {
                descriptor,
                generics,
                heritage,
                properties: _,
                scope,
            } => {
                self.node("Declaration::Struct", id.id)
                    .field("descriptor", descriptor)
                    .field("generics", generics)
                    .field("heritage", heritage)
                    .field("scope", scope)
                    .end();
            }
            Declaration::Class {
                descriptor,
                generics,
                heritage,
                properties: _,
                scope,
            } => {
                self.node("Declaration::Class", id.id)
                    .field("descriptor", descriptor)
                    .field("generics", generics)
                    .field("heritage", heritage)
                    .field("scope", scope)
                    .end();
            }
            Declaration::Enum {
                descriptor,
                generics,
                heritage,
                fields: _,
                properties: _,
                scope,
            } => {
                self.node("Declaration::Enum", id.id)
                    .field("descriptor", descriptor)
                    .field("generics", generics)
                    .field("heritage", heritage)
                    .field("scope", scope)
                    .end();
            }
            Declaration::Interface {
                descriptor,
                generics,
                heritage,
                properties: _,
                scope,
            } => {
                self.node("Declaration::Interface", id.id)
                    .field("descriptor", descriptor)
                    .field("generics", generics)
                    .field("heritage", heritage)
                    .field("scope", scope)
                    .end();
            }
            Declaration::Function {
                descriptor,
                signature,
                body: _,
                scope,
            } => {
                self.node("Declaration::Function", id.id)
                    .field("descriptor", descriptor)
                    .field("signature", signature)
                    .field("scope", scope)
                    .end();
            }
            Declaration::Extension {
                descriptor,
                generics,
                target_type: _,
                target_symbol,
                heritage,
                properties: _,
                scope,
            } => {
                self.node("Declaration::Extension", id.id)
                    .field("descriptor", descriptor)
                    .field("generics", generics)
                    .field_optional("target_symbol", target_symbol)
                    .field("heritage", heritage)
                    .field("scope", scope)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_declaration(dumper, tree, id, declaration);
        });
    }

    fn visit_property(&mut self, tree: &NodeTree, id: LocalNodeId<Property>, property: &Property) {
        match property {
            Property::Field {
                modifiers,
                key,
                value: _,
                default: _,
                symbol,
            } => {
                self.node("Property::Field", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("key", key)
                    .field("symbol", symbol)
                    .end();
            }
            Property::Method {
                modifiers,
                key,
                signature,
                body: _,
                symbol,
            } => {
                self.node("Property::Method", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("key", key)
                    .field("signature", signature)
                    .field("symbol", symbol)
                    .end();
            }
            Property::Spread {
                modifiers,
                value: _,
                symbol,
            } => {
                self.node("Property::Spread", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("symbol", symbol)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_property(dumper, tree, id, property);
        });
    }

    fn visit_static_property(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<StaticProperty>,
        static_property: &StaticProperty,
    ) {
        match static_property {
            StaticProperty::Unevaluated { node: _ } => {
                self.node("StaticProperty::Unevaluated", id.id).end();
            }
            StaticProperty::Field {
                modifiers,
                key,
                value: _,
                default: _,
                symbol,
            } => {
                self.node("StaticProperty::Field", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("key", key)
                    .field("symbol", symbol)
                    .end();
            }
            StaticProperty::Method {
                modifiers,
                key,
                signature,
                body: _,
                symbol,
            } => {
                self.node("StaticProperty::Method", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("key", key)
                    .field("signature", signature)
                    .field("symbol", symbol)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_static_property(dumper, tree, id, static_property);
        });
    }

    fn visit_enum_field(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<EnumField>,
        enum_field: &EnumField,
    ) {
        self.node("EnumField", id.id)
            .field("name", &enum_field.name)
            .field("symbol", &enum_field.symbol)
            .end();
        self.with_depth(|dumper| {
            walk_enum_field(dumper, tree, id, enum_field);
        });
    }

    fn visit_where_clause(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<WhereClause>,
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

    fn visit_with_clause(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<WithClause>,
        with_clause: &WithClause,
    ) {
        self.node("WithClause", id.id)
            .field_optional("alias", &with_clause.alias)
            .end();
        self.with_depth(|dumper| {
            walk_with_clause(dumper, tree, id, with_clause);
        });
    }

    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        match dependency_item {
            DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                alias,
                target,
                target_module: module,
                symbol,
            } => {
                self.node("DependencyItem::UnresolvedRemote", id.id)
                    .field("source", source)
                    .field("mode", mode)
                    .field("kind", kind)
                    .field_optional("name", name)
                    .field_optional("alias", alias)
                    .field("target", target)
                    .field_optional("module", module)
                    .field("symbol", symbol)
                    .end();
            }
            DependencyItem::UnresolvedLocal {
                mode,
                kind,
                name,
                alias,
                symbol,
            } => {
                self.node("DependencyItem::UnresolvedLocal", id.id)
                    .field("mode", mode)
                    .field("kind", kind)
                    .field("name", name)
                    .field_optional("alias", alias)
                    .field("symbol", symbol)
                    .end();
            }
            DependencyItem::Value { value: _ } => {
                self.node("DependencyItem::Value", id.id).end();
            }
            DependencyItem::Local {
                mode,
                kind,
                name,
                alias,
                symbol,
                target_symbol,
            } => {
                self.node("DependencyItem::Declaration", id.id)
                    .field("mode", mode)
                    .field("kind", kind)
                    .field("name", name)
                    .field_optional("alias", alias)
                    .field("symbol", symbol)
                    .field("target_symbol", target_symbol)
                    .end();
            }
            DependencyItem::Remote {
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol,
                target_symbol,
            } => {
                self.node("DependencyItem::Remote", id.id)
                    .field("mode", mode)
                    .field("kind", kind)
                    .field("name", name)
                    .field_optional("alias", alias)
                    .field("target", target)
                    .field("target_module", target_module)
                    .field("symbol", symbol)
                    .field("target_symbol", target_symbol)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_dependency_item(dumper, tree, id, dependency_item);
        });
    }

    fn visit_parameter(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Parameter>,
        parameter: &Parameter,
    ) {
        match parameter {
            Parameter::Named {
                modifiers,
                name,
                symbol,
                default: _,
            } => {
                self.node("Parameter::Scalar", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .field("symbol", symbol)
                    .end();
            }
            Parameter::Pattern {
                modifiers,
                pattern: _,
                symbol,
                default: _,
            } => {
                self.node("Parameter::Pattern", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("symbol", symbol)
                    .end();
            }
            Parameter::Variadic {
                modifiers,
                name,
                symbol,
            } => {
                self.node("Parameter::Variadic", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .field("symbol", symbol)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_parameter(dumper, tree, id, parameter);
        });
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: LocalNodeId<Argument>, argument: &Argument) {
        match argument {
            Argument::Named { name, value: _ } => {
                self.node("Argument::Named", id.id)
                    .field("name", name)
                    .end();
            }
            Argument::Positional { value: _ } => {
                self.node("Argument::Positional", id.id).end();
            }
            Argument::Spread { value: _ } => {
                self.node("Argument::Spread", id.id).end();
            }
            Argument::Dynamic { key: _, value: _ } => {
                self.node("Argument::Dynamic", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_argument(dumper, tree, id, argument);
        });
    }

    fn visit_static_argument(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<StaticArgument>,
        static_argument: &StaticArgument,
    ) {
        match static_argument {
            StaticArgument::Unevaluated { node: _ } => {
                self.node("StaticArgument::Unevaluated", id.id).end();
            }
            StaticArgument::Evaluated { name, value: _ } => {
                self.node("StaticArgument::Evaluated", id.id)
                    .field_optional("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_static_argument(dumper, tree, id, static_argument);
        });
    }

    fn visit_match_case(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<MatchCase>,
        match_case: &MatchCase,
    ) {
        match match_case {
            MatchCase::Expression {
                pattern: _,
                body: _,
                guard: _,
                scope,
            } => {
                self.node("MatchCase::Expression", id.id)
                    .field("scope", scope)
                    .end();
            }
            MatchCase::Block {
                pattern: _,
                body: _,
                guard: _,
                scope,
            } => {
                self.node("MatchCase::Block", id.id)
                    .field("scope", scope)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_match_case(dumper, tree, id, match_case);
        });
    }

    fn visit_pattern(&mut self, tree: &NodeTree, id: LocalNodeId<Pattern>, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {
                self.node("Pattern::Wildcard", id.id).end();
            }
            Pattern::Rest { name } => {
                self.node("Pattern::Rest", id.id)
                    .field_optional("name", name)
                    .end();
            }
            Pattern::Maybe(_) => {
                self.node("Pattern::Maybe", id.id).end();
            }
            Pattern::ReferenceOf {
                right: _,
                mutability,
            } => {
                self.node("Pattern::ReferenceOf", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Pattern::ValueOf {
                right: _,
                mutability,
            } => {
                self.node("Pattern::ValueOf", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Pattern::Binding {
                mutability,
                name,
                pattern: _,
                symbol,
            } => {
                self.node("Pattern::Binding", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .field("symbol", symbol)
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
            Pattern::Tuple { fields: _ } => {
                self.node("Pattern::Tuple", id.id).end();
            }
            Pattern::TaggedTuple { ty: _, fields: _ } => {
                self.node("Pattern::TaggedTuple", id.id).end();
            }
            Pattern::Slice { fields: _ } => {
                self.node("Pattern::Slice", id.id).end();
            }
            Pattern::Object { fields: _ } => {
                self.node("Pattern::Object", id.id).end();
            }
            Pattern::TaggedObject { ty: _, fields: _ } => {
                self.node("Pattern::TaggedObject", id.id).end();
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
        id: LocalNodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        match pattern_field {
            PatternField::Named {
                mutability,
                name,
                default: _,
                pattern: _,
                symbol,
            } => {
                self.node("PatternField::Named", id.id)
                    .field("name", name)
                    .field_optional("mutability", mutability)
                    .field("symbol", symbol)
                    .end();
            }
            PatternField::Alias {
                mutability,
                name,
                alias,
                default: _,
                symbol,
            } => {
                self.node("PatternField::Alias", id.id)
                    .field("name", name)
                    .field("alias", alias)
                    .field_optional("mutability", mutability)
                    .field("symbol", symbol)
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

    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Annotation>,
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
            Annotation::UnresolvedTag {
                position,
                left,
                arguments: _,
            } => {
                self.node("Annotation::Tag", id.id)
                    .field("position", position)
                    .field("left", left)
                    .end();
            }
            Annotation::UnresolvedDecorator {
                position,
                left,
                arguments: _,
            } => {
                self.node("Annotation::Decorator", id.id)
                    .field("position", position)
                    .field("left", left)
                    .end();
            }
            Annotation::Tag {
                position,
                left,
                target_symbol,
                arguments: _,
            } => {
                self.node("Annotation::Tag", id.id)
                    .field("position", position)
                    .field("left", left)
                    .field("target_symbol", target_symbol)
                    .end();
            }
            Annotation::Decorator {
                position,
                left,
                target_symbol,
                arguments: _,
            } => {
                self.node("Annotation::Decorator", id.id)
                    .field("position", position)
                    .field("left", left)
                    .field("target_symbol", target_symbol)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_annotation(dumper, tree, id, annotation);
        });
    }
}

// ----------------------------------------------------------------------------
// Meta
// ----------------------------------------------------------------------------

impl_dump_display! {
    ScopeKind,
    SymbolSpace,
    SymbolKind,
    NodeType,
}

impl Dump for LocalScopeId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("#{}", self.0), Some(Color::Green));
    }
}

impl Dump for LocalScopeMark {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        if self.0 == u32::MAX {
            dumper.write_str(".END", Some(Color::Green));
        } else {
            dumper.write_str(format!(".{}", self.0), Some(Color::Green));
        }
    }
}

impl Dump for GlobalSymbolId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(
            format!("#{}/{}", self.module_id.0, self.local_id.0),
            Some(Color::Green),
        );
    }
}

impl Dump for LocalSymbolId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("#{}", self.0), Some(Color::Green));
    }
}

impl Dump for GlobalScopeId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(
            format!("#{}/#{}", self.module_id.0, self.local_id.0),
            Some(Color::Green),
        );
    }
}

impl Dump for SymbolKey {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            SymbolKey::Name(name) => {
                dumper.object("SymbolKey::Name").value(name).end();
            }
            SymbolKey::UniqueSymbol(unique_symbol) => {
                dumper
                    .object("SymbolKey::UniqueSymbol")
                    .value(&unique_symbol.id)
                    .end();
            }
            SymbolKey::GlobalSymbol(global_symbol) => {
                dumper
                    .object("SymbolKey::GlobalSymbol")
                    .value(global_symbol)
                    .end();
            }
        }
    }
}

impl<'a> Dumper<'a> {
    pub fn visit_scope(
        &mut self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        id: LocalScopeId,
        scope: &Scope,
    ) {
        self.node_like("Scope", Some(id.0), scope.parent.map(|(id, _)| id.0))
            .field("id", &id)
            .field_optional(
                "parent",
                &scope.parent.map(|(id, mark)| format!("{id}{mark}")),
            )
            .field("kind", &scope.kind)
            .field_optional("owner", &scope.owner_id)
            .end();
        self.with_depth(|dumper| {
            // symbols
            for (_key, symbol_id) in scope.named_symbols.iter() {
                let symbol = symbols.get_symbol(*symbol_id);
                dumper.visit_symbol(tree, symbols, *symbol_id, symbol);
            }
            // children
            for child_id in scope.children.iter() {
                let child = symbols.get_scope_by_id(*child_id);
                dumper.visit_scope(tree, symbols, *child_id, child);
            }
        });
    }

    pub fn visit_symbol(
        &mut self,
        _tree: &NodeTree,
        _symbols: &SymbolTable,
        id: LocalSymbolId,
        symbol: &Symbol,
    ) {
        self.node_like("Symbol", Some(id.0), Some(symbol.scope.0.0))
            .field("id", &id)
            .field("kind", &symbol.kind)
            .field("space", &symbol.space)
            .field_optional(
                "declaration",
                &symbol.primary_declaration.map(|id| id.local_id.ty),
            )
            .field_optional("key", &symbol.key)
            .field("scope", &format!("{}{}", symbol.scope.0, symbol.scope.1))
            .field_optional("export", &symbol.export)
            .end();
    }
}
