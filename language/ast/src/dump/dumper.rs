#![allow(clippy::match_like_matches_macro)]

use crate::*;
use dyst_source::{Color, ImmutableStringPool, SmallVec, impl_dump_display, rebuild_tree_output};
use std::borrow::Cow;

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
    pub strings: &'a ImmutableStringPool,
    /// The node tree.
    pub tree: &'a MutableNodeTree,
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
        tree: &'a MutableNodeTree,
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
            let span = self.dumper.tree.source_map.get(node_id);
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
        dumper.write_str(self, Some(Color::BrightYellow));
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

/// Dump a SmallVec<T, N> as a slice.
impl<T: Dump, const N: usize> Dump for SmallVec<T, N> {
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

/// Dump a NodeId<T> as the node it points to.
impl<T: Node + Clone + Dump> Dump for NodeId<T>
where
    MutableNodeTree: MutableNodeTreeImpl<T>,
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

impl_dump_display! {
    AnnotationPosition,
    Asynchrony,
    AssignOperator,
    BindingKind,
    BindingOperator,
    BindingScope,
    BlockFormat,
    BinaryOperator,
    CommentStyle,
    DeclarationKind,
    DependencyKind,
    DocStyle,
    ExportType,
    ForEachKind,
    FunctionAbstraction,
    FunctionCardinality,
    FunctionKind,
    FunctionMode,
    IfKind,
    WhileKind,
    MatchKind,
    Mutability,
    PostfixPosition,
    ReferenceType,
    Runtime,
    StructKind,
    TypeBinaryOperator,
    TypeUnaryOperator,
    UnaryOperator,
    VariantFormat,
    VarianceBound,
    Visibility,
    YieldCardinality,
}

/// Dump a SelfParameter as a string.
impl Dump for SelfParameter {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("SelfParameter")
            .field("mutability", &self.mutability)
            .field("reference_type", &self.reference_type)
            .end();
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

/// Dump a DefinitionMeta as a string.
impl Dump for DefinitionMeta {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("DefinitionMeta")
            .field("kind", &self.kind)
            .field_optional("name", &self.name)
            .field_optional("visibility", &self.visibility)
            .field_optional("export", &self.export)
            .end();
    }
}
impl Dump for BindingModifier {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("BindingModifier")
            .field_optional("kind", &self.kind)
            .field_optional("mutability", &self.mutability)
            .field_optional("visibility", &self.visibility)
            .field_optional("operator", &self.operator)
            .end();
    }
}

/// Dump an IntType as a structured representation.
impl Dump for IntType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            IntType::Pointer { is_signed } => {
                dumper
                    .object("IntType::Pointer")
                    .field("is_signed", is_signed)
                    .end();
            }
            IntType::Arbitrary { width, is_signed } => {
                dumper
                    .object("IntType::Arbitrary")
                    .field("width", width)
                    .field("is_signed", is_signed)
                    .end();
            }
        }
    }
}

/// Dump a FloatType as a string.
impl Dump for FloatType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.object("FloatType").field("width", &self.width).end();
    }
}

/// Dump a DefinitionType as a structured representation.
impl Dump for DefinitionType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            DefinitionType::Type => {
                dumper.object("DefinitionType::Type").end();
            }
            DefinitionType::Namespace => {
                dumper.object("DefinitionType::Namespace").end();
            }
            DefinitionType::Struct => {
                dumper.object("DefinitionType::Struct").end();
            }
            DefinitionType::Class => {
                dumper.object("DefinitionType::Class").end();
            }
            DefinitionType::Enum => {
                dumper.object("DefinitionType::Enum").end();
            }
            DefinitionType::Union => {
                dumper.object("DefinitionType::Union").end();
            }
            DefinitionType::Interface => {
                dumper.object("DefinitionType::Interface").end();
            }
            DefinitionType::Extension => {
                dumper.object("DefinitionType::Extension").end();
            }
            DefinitionType::Function => {
                dumper.object("DefinitionType::Function").end();
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
            TypeLiteral::Bigint => {
                dumper.object("TypeLiteral::Bigint").end();
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
            TypeLiteral::Symbol => {
                dumper.object("TypeLiteral::Symbol").end();
            }
            TypeLiteral::UniqueSymbol => {
                dumper.object("TypeLiteral::UniqueSymbol").end();
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

    fn visit_any(&mut self, tree: &MutableNodeTree, _ty: NodeType, id: u32) {
        let annotations = tree.get_annotations(id);
        for annotation_id in annotations {
            let annotation = tree.get(annotation_id);
            self.visit_annotation(tree, annotation_id, annotation);
        }
    }

    fn visit_expression(
        &mut self,
        _tree: &MutableNodeTree,
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
                kind,
                target,
                alias,
                items: _,
                arguments: _,
            } => {
                self.node("Expression::Import", _id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .field_optional("alias", alias)
                    .end();
            }
            Expression::Export {
                mode,
                kind,
                target,
                alias,
                items: _,
                value: _,
            } => {
                self.node("Expression::Export", _id.id)
                    .field("mode", mode)
                    .field("kind", kind)
                    .field_optional("target", target)
                    .field_optional("alias", alias)
                    .end();
            }
            Expression::Let {
                mutability,
                meta,
                pattern: _,
                ty: _,
                value: _,
            } => {
                self.node("Expression::Let", _id.id)
                    .field("mutability", mutability)
                    .field("meta", meta)
                    .end();
            }
            Expression::LetType {
                mutability,
                meta,
                static_parameters: _,
                value: _,
            } => {
                self.node("Expression::LetType", _id.id)
                    .field_optional("mutability", mutability)
                    .field("meta", meta)
                    .end();
            }
            Expression::If {
                kind,
                condition: _,
                then_expression: _,
                else_expression: _,
            } => {
                self.node("Expression::If", _id.id)
                    .field("kind", kind)
                    .end();
            }
            Expression::While {
                kind,
                condition: _,
                body: _,
            } => {
                self.node("Expression::While", _id.id)
                    .field("kind", kind)
                    .end();
            }
            Expression::ForEach {
                asynchrony,
                kind,
                pattern: _,
                iterator: _,
                body: _,
            } => {
                self.node("Expression::ForEach", _id.id)
                    .field("asynchrony", asynchrony)
                    .field("kind", kind)
                    .end();
            }
            Expression::For {
                initialization: _,
                condition: _,
                increment: _,
                body: _,
            } => {
                self.node("Expression::For", _id.id).end();
            }
            Expression::Loop { body: _ } => {
                self.node("Expression::Loop", _id.id).end();
            }
            Expression::Try {
                try_expression: _,
                catch_pattern: _,
                catch_expression: _,
                finally_expression: _,
            } => {
                self.node("Expression::Try", _id.id).end();
            }
            Expression::Match {
                kind: style,
                value: _,
                cases: _,
            } => {
                self.node("Expression::Match", _id.id)
                    .field("style", style)
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
            Expression::Defer { expression: _ } => {
                self.node("Expression::Defer", _id.id).end();
            }
            Expression::Await { expression: _ } => {
                self.node("Expression::Await", _id.id).end();
            }
            Expression::Yield {
                cardinality,
                value: _,
            } => {
                self.node("Expression::Yield", _id.id)
                    .field("cardinality", cardinality)
                    .end();
            }
            Expression::Throw { value: _ } => {
                self.node("Expression::Throw", _id.id).end();
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
            Expression::Unary { operator, right: _ } => {
                self.node("Expression::Unary", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::TypeUnary { operator, right: _ } => {
                self.node("Expression::TypeUnary", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::ValueOf {
                mutability,
                variance,
                right: _,
            } => {
                self.node("Expression::ValueOf", _id.id)
                    .field_optional("mutability", mutability)
                    .field_optional("variance", variance)
                    .end();
            }
            Expression::ReferenceOf {
                mutability,
                variance,
                right: _,
            } => {
                self.node("Expression::ReferenceOf", _id.id)
                    .field_optional("mutability", mutability)
                    .field_optional("variance", variance)
                    .end();
            }
            Expression::Member {
                left: _,
                path,
                static_arguments: _,
            } => {
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
                left: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::Call", _id.id)
                    .field("position", position)
                    .end();
            }
            Expression::New {
                left,
                static_arguments: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::New", _id.id)
                    .field("left", left)
                    .end();
            }
            Expression::Delete { value: _ } => {
                self.node("Expression::Delete", _id.id).end();
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
            Expression::TypeBinary {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::TypeBinary", _id.id)
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

    fn visit_block(&mut self, _tree: &MutableNodeTree, _id: NodeId<Block>, block: &Block) {
        self.node("Block", _id.id)
            .field("format", &block.format)
            .field_optional("label", &block.label)
            .end();
        self.with_depth(|dumper| {
            walk_block(dumper, _tree, _id, block);
        });
    }

    fn visit_definition(
        &mut self,
        tree: &MutableNodeTree,
        id: NodeId<crate::Definition>,
        definition: &crate::Definition,
    ) {
        match definition {
            Definition::Namespace {
                meta,
                expressions: _,
                with_clauses: _,
                where_clauses: _,
            } => {
                self.node("Definition::Namespace", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Struct {
                meta,
                kind,
                format,
                extends_types: _,
                implements_types: _,
                representation_type: _,
                static_parameters: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                expressions: _,
            } => {
                self.node("Definition::Struct", id.id)
                    .field("meta", meta)
                    .field("format", format)
                    .field("kind", kind)
                    .end();
            }
            Definition::Enum {
                meta,
                tag_type: _,
                static_parameters: _,
                extends_types: _,
                implements_types: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                expressions: _,
            } => {
                self.node("Definition::Enum", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Union {
                meta,
                tag_type: _,
                representation_type: _,
                static_parameters: _,
                extends_types: _,
                implements_types: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                expressions: _,
            } => {
                self.node("Definition::Union", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Interface {
                meta,
                extends_types: _,
                static_parameters: _,
                with_clauses: _,
                where_clauses: _,
                fields: _,
                expressions: _,
            } => {
                self.node("Definition::Interface", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Extension {
                meta,
                static_parameters: _,
                target_type: _,
                implements_types: _,
                with_clauses: _,
                where_clauses: _,
                expressions: _,
            } => {
                self.node("Definition::Extension", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Function {
                meta,
                abstraction,
                asynchrony,
                cardinality,
                mode: kind,
                kind: style,
                static_parameters: _,
                self_parameter,
                dynamic_parameters: _,
                return_type: _,
                with_clauses: _,
                where_clauses: _,
                body: _,
            } => {
                self.node("Definition::Function", id.id)
                    .field("meta", meta)
                    .field("abstraction", abstraction)
                    .field("asynchrony", asynchrony)
                    .field("cardinality", cardinality)
                    .field_optional("kind", kind)
                    .field("style", style)
                    .field_optional("self_parameter", self_parameter)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_definition(dumper, tree, id, definition);
        });
    }

    fn visit_field(&mut self, _tree: &MutableNodeTree, _id: NodeId<Field>, field: &Field) {
        match field {
            Field::Named {
                modifiers,
                name,
                ty: _,
                default: _,
            } => {
                self.node("Field::Named", _id.id)
                    .field("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
            Field::Positional {
                modifiers,
                ty: _,
                default: _,
            } => {
                self.node("Field::Positional", _id.id)
                    .field("modifiers", modifiers)
                    .end();
            }
            Field::Dynamic {
                modifiers,
                name,
                ty: _,
                key: _,
                default: _,
            } => {
                self.node("Field::Dynamic", _id.id)
                    .field("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_field(dumper, _tree, _id, field);
        });
    }

    fn visit_enum_field(&mut self, _tree: &MutableNodeTree, _id: NodeId<EnumField>, field: &EnumField) {
        self.node("EnumField", _id.id)
            .field("name", &field.name)
            .end();
        self.with_depth(|dumper| {
            walk_enum_field(dumper, _tree, _id, field);
        });
    }

    fn visit_union_field(&mut self, _tree: &MutableNodeTree, _id: NodeId<UnionField>, field: &UnionField) {
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

    fn visit_with_clause(
        &mut self,
        _tree: &MutableNodeTree,
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
        _tree: &MutableNodeTree,
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

    fn visit_dependency_item(
        &mut self,
        _tree: &MutableNodeTree,
        _id: NodeId<DependencyItem>,
        item: &DependencyItem,
    ) {
        self.node("DependencyItem", _id.id)
            .field("name", &item.name)
            .field_optional("alias", &item.alias)
            .end();
        self.with_depth(|dumper| {
            walk_dependency_item(dumper, _tree, _id, item);
        });
    }

    fn visit_parameter(&mut self, _tree: &MutableNodeTree, _id: NodeId<Parameter>, param: &Parameter) {
        match param {
            Parameter::Named {
                modifiers,
                name,
                ty: _,
                default: _,
            } => {
                self.node("Parameter::Scalar", _id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
            Parameter::Pattern {
                modifiers,
                pattern: _,
                ty: _,
                default: _,
            } => {
                self.node("Parameter::Pattern", _id.id)
                    .field_optional("modifiers", modifiers)
                    .end();
            }
            Parameter::Variadic {
                modifiers,
                name,
                ty: _,
            } => {
                self.node("Parameter::Variadic", _id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_parameter(dumper, _tree, _id, param);
        });
    }

    fn visit_argument(&mut self, _tree: &MutableNodeTree, _id: NodeId<Argument>, arg: &Argument) {
        match arg {
            Argument::Named {
                modifiers,
                name,
                value: _,
            } => {
                self.node("Argument::Named", _id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
            Argument::Shorthand { modifiers, name } => {
                self.node("Argument::Shorthand", _id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
            Argument::Positional {
                modifiers,
                value: _,
            } => {
                self.node("Argument::Positional", _id.id)
                    .field_optional("modifiers", modifiers)
                    .end();
            }
            Argument::Spread {
                modifiers,
                name,
                value: _,
            } => {
                self.node("Argument::Spread", _id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
            Argument::Dynamic {
                modifiers,
                name,
                key: _,
                value: _,
            } => {
                self.node("Argument::Dynamic", _id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
            Argument::Function {
                modifiers,
                name,
                value: _,
            } => {
                self.node("Argument::Function", _id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
            Argument::DynamicFunction {
                modifiers,
                name,
                key: _,
                value: _,
            } => {
                self.node("Argument::DynamicFunction", _id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_argument(dumper, _tree, _id, arg);
        });
    }

    fn visit_match_case(&mut self, _tree: &MutableNodeTree, _id: NodeId<MatchCase>, case: &MatchCase) {
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

    fn visit_pattern(&mut self, _tree: &MutableNodeTree, _id: NodeId<Pattern>, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {
                self.node("Pattern::Wildcard", _id.id).end();
            }
            Pattern::Rest { name } => {
                self.node("Pattern::Rest", _id.id)
                    .field_optional("name", name)
                    .end();
            }
            Pattern::Maybe(_) => {
                self.node("Pattern::Unwrap", _id.id).end();
            }
            Pattern::ReferenceOf {
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
        _tree: &MutableNodeTree,
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
            PatternField::Alias {
                mutability,
                name,
                alias,
                default: _,
            } => {
                self.node("PatternField::Alias", _id.id)
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

    fn visit_annotation(
        &mut self,
        _tree: &MutableNodeTree,
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

    fn visit_blank(&mut self, _tree: &MutableNodeTree, _id: NodeId<Blank>, blank: &Blank) {
        self.node("Blank", _id.id)
            .field("lines", &blank.lines)
            .end();
        self.with_depth(|dumper| {
            walk_blank(dumper, _tree, _id, blank);
        });
    }

    fn visit_doc(&mut self, _tree: &MutableNodeTree, _id: NodeId<Doc>, doc: &Doc) {
        let string = truncate_string(self.strings.get(doc.string), 40, "...");
        self.node("Doc", _id.id)
            .field("string", &string.as_ref())
            .field("style", &doc.style)
            .end();
        self.with_depth(|dumper| {
            walk_doc(dumper, _tree, _id, doc);
        });
    }

    fn visit_comment(&mut self, _tree: &MutableNodeTree, _id: NodeId<Comment>, comment: &Comment) {
        let string = truncate_string(self.strings.get(comment.string), 40, "...");
        self.node("Comment", _id.id)
            .field("string", &string.as_ref())
            .field("style", &comment.style)
            .end();
        self.with_depth(|dumper| {
            walk_comment(dumper, _tree, _id, comment);
        });
    }

    fn visit_tag(&mut self, _tree: &MutableNodeTree, _id: NodeId<Tag>, tag: &Tag) {
        self.node("Tag", _id.id)
            .field("receiver", &tag.receiver)
            .end();
        self.with_depth(|dumper| {
            walk_tag(dumper, _tree, _id, tag);
        });
    }

    fn visit_decorator(&mut self, _tree: &MutableNodeTree, _id: NodeId<Decorator>, decorator: &Decorator) {
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
