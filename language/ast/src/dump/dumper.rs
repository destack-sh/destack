#![allow(clippy::match_like_matches_macro)]

use crate::*;
use destack_core::{Color, ImmutableStringPool, impl_dump_display, rebuild_tree_output};
use smallvec::{Array, SmallVec};

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

/// Dump a usize as a numeric value.
impl Dump for usize {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(self.to_string(), Some(Color::Green));
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
impl<T: Node + Clone + Dump> Dump for LocalNodeId<T>
where
    NodeTree: NodeTreeImpl<T>,
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

/// Dump ambientness as a structured representation.
impl Dump for Ambientness {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Ambientness::Ambient => {
                dumper.object("Ambientness::Ambient").end();
            }
            Ambientness::Concrete => {
                dumper.object("Ambientness::Concrete").end();
            }
        }
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
            Name::Number(id) => {
                // numeric keys display without quotes
                id.dump(dumper);
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
    Asynchrony,
    AssignOperator,
    BlockFormat,
    BinaryOperator,
    CommentKind,
    NamespaceKind,
    DependencyKind,
    DependencyMode,
    ExportMode,
    DecoratorPosition,
    EnumKind,
    ForEachKind,
    FunctionCardinality,
    FunctionKind,
    FunctionMode,
    IfKind,
    LetKind,
    WhileKind,
    MatchKind,
    Mutability,
    PostfixPosition,
    ReferenceType,
    IntrinsicType,
    TypeBinaryOperator,
    TypeUnaryOperator,
    UnaryOperator,
    TypeKind,
    VarianceModifier,
    VarianceBound,
    Visibility,
    YieldCardinality,
}

/// Dump an ImportSource as a string.
impl Dump for ImportSource {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let name = match self {
            ImportSource::ImportStatement => "ImportStatement",
            ImportSource::ReferencePathDirective => "ReferencePathDirective",
            ImportSource::ReferenceTypesDirective => "ReferenceTypesDirective",
            ImportSource::ReferenceLibDirective => "ReferenceLibDirective",
            ImportSource::ImportEquals => "ImportEquals",
            ImportSource::ImportCall => "ImportCall",
        };

        dumper.write_str(name, Some(Color::White));
    }
}

/// Dump an ImportTarget as a structured representation.
impl Dump for ImportTarget {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ImportTarget::String(target) => {
                dumper
                    .object("ImportTarget::String")
                    .field("target", target)
                    .end();
            }
            ImportTarget::Expression { target } => {
                dumper
                    .object("ImportTarget::Expression")
                    .field("target_id", &target.id)
                    .end();
            }
        }
    }
}

/// Dump an import attribute clause kind as a string.
impl Dump for ImportAttributeClauseKind {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let name = match self {
            ImportAttributeClauseKind::With => "With",
            ImportAttributeClauseKind::Assert => "Assert",
        };

        dumper.write_str(name, Some(Color::White));
    }
}

/// Dump an import attribute clause as a structured representation.
impl Dump for ImportAttributeClause {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let attribute_count = self.attributes.len() as u32;

        dumper
            .object("ImportAttributeClause")
            .field("kind", &self.kind)
            .field("attribute_count", &attribute_count)
            .end();
    }
}

/// Dump an ImportAliasTarget as a structured representation.
impl Dump for ImportAliasTarget {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ImportAliasTarget::Require { target } => {
                dumper
                    .object("ImportAliasTarget::Require")
                    .field("target", target)
                    .end();
            }
            ImportAliasTarget::Path { path } => {
                dumper
                    .object("ImportAliasTarget::Path")
                    .field("path", path)
                    .end();
            }
        }
    }
}

/// Dump a Key as a structured representation.
impl Dump for Key {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Key::Name(name) => {
                dumper.object("Key::Name").value(name).end();
            }
            Key::Private(name) => {
                dumper.object("Key::Private").value(name).end();
            }
            Key::Expression(_) => {
                dumper.object("Key::Expression").end();
            }
        }
    }
}

/// Dump a FunctionSignature as a structured representation.
impl Dump for FunctionSignature {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("FunctionSignature")
            .field("is_abstract", &self.is_abstract)
            .field("is_override", &self.is_override)
            .field("asynchrony", &self.asynchrony)
            .field("cardinality", &self.cardinality)
            .field_optional("mode", &self.mode)
            .field("kind", &self.kind)
            .field("generic_parameter_count", &self.generic_parameters.len())
            .field("where_clause_count", &self.where_clauses.len())
            .field("has_this_parameter", &self.this_parameter.is_some())
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

/// Dump a ScalarLiteral as a structured representation.
impl Dump for ScalarLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ScalarLiteral::Null => {
                dumper.object("ScalarLiteral::Null").end();
            }
            ScalarLiteral::Boolean(value) => {
                dumper.object("ScalarLiteral::Boolean").value(value).end();
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
            TypeLiteral::Object => {
                dumper.object("TypeLiteral::Object").end();
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
            TypeLiteral::Symbol => {
                dumper.object("TypeLiteral::Symbol").end();
            }
            TypeLiteral::UniqueSymbol => {
                dumper.object("TypeLiteral::UniqueSymbol").end();
            }
            TypeLiteral::Intrinsic(intrinsic) => {
                dumper
                    .object("TypeLiteral::Intrinsic")
                    .field("intrinsic", intrinsic)
                    .end();
            }
        }
    }
}

/// Dump a TypeModifier as a structured representation.
impl Dump for TypeModifier {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            TypeModifier::Present => {
                dumper.object("TypeModifier::Present").end();
            }
            TypeModifier::Add => {
                dumper.object("TypeModifier::Add").end();
            }
            TypeModifier::Remove => {
                dumper.object("TypeModifier::Remove").end();
            }
            TypeModifier::None => {
                dumper.object("TypeModifier::None").end();
            }
        }
    }
}

/// Dump a TypePredicateSubject as a structured representation.
impl Dump for TypePredicateSubject {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            TypePredicateSubject::Identifier(name) => {
                dumper
                    .object("TypePredicateSubject::Identifier")
                    .field("name", name)
                    .end();
            }
            TypePredicateSubject::This => {
                dumper.object("TypePredicateSubject::This").end();
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
        let decorators = tree.get_decorators(id);
        for decorator_id in decorators {
            let decorator = tree.get(decorator_id);
            self.visit_decorator(tree, decorator_id, decorator);
        }
    }

    fn visit_expression(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Declaration(_) => {
                self.node("Expression::Declaration", _id.id).end();
            }
            Expression::Block(_) => {
                self.node("Expression::Block", _id.id).end();
            }
            Expression::Labelled { label, body: _ } => {
                self.node("Expression::Labelled", _id.id)
                    .field("label", label)
                    .end();
            }
            Expression::Import {
                source,
                kind,
                target,
                items: _,
                attributes,
                arguments: _,
            } => {
                self.node("Expression::Import", _id.id)
                    .field("source", source)
                    .field("kind", kind)
                    .field("target", target)
                    .field_optional("attributes", attributes)
                    .end();
            }
            Expression::Export {
                kind,
                target,
                items: _,
                attributes,
            } => {
                self.node("Expression::Export", _id.id)
                    .field("kind", kind)
                    .field_optional("target", target)
                    .field_optional("attributes", attributes)
                    .end();
            }
            Expression::ExportNamespace { name } => {
                self.node("Expression::ExportNamespace", _id.id)
                    .field("name", name)
                    .end();
            }
            Expression::Let {
                kind,
                mutability,
                export,
                ambient,
                declarators: _,
            } => {
                self.node("Expression::Let", _id.id)
                    .field("kind", kind)
                    .field("mutability", mutability)
                    .field_optional("export", export)
                    .field("ambient", ambient)
                    .end();
            }
            Expression::Using {
                asynchrony,
                export,
                ambient,
                declarators: _,
            } => {
                self.node("Expression::Using", _id.id)
                    .field("asynchrony", asynchrony)
                    .field_optional("export", export)
                    .field("ambient", ambient)
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
                binding: _,
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
                catch_ty: _,
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
            Expression::Await { expression: _ } => {
                self.node("Expression::Await", _id.id).end();
            }
            Expression::AwaitMaybe { expression: _ } => {
                self.node("Expression::AwaitMaybe", _id.id).end();
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
            Expression::Identifier { name } => {
                self.node("Expression::Identifier", _id.id)
                    .field("name", name)
                    .end();
            }
            Expression::QualifiedReference {
                path,
                generic_arguments: _,
            } => {
                self.node("Expression::QualifiedReference", _id.id)
                    .field("path", path)
                    .end();
            }
            Expression::PrivateIdentifier { name } => {
                self.node("Expression::PrivateIdentifier", _id.id)
                    .field("name", name)
                    .end();
            }
            Expression::This => {
                self.node("Expression::This", _id.id).end();
            }
            Expression::Super => {
                self.node("Expression::Super", _id.id).end();
            }
            Expression::ImportMeta => {
                self.node("Expression::ImportMeta", _id.id).end();
            }
            Expression::NewTarget => {
                self.node("Expression::NewTarget", _id.id).end();
            }
            Expression::ScalarLiteral(value) => {
                self.node("Expression::ScalarLiteral", _id.id)
                    .value(value)
                    .end();
            }
            Expression::TemplateExpression { value } => {
                self.node("Expression::TemplateExpression", _id.id)
                    .value(value)
                    .end();
            }
            Expression::TaggedTemplateExpression { tag: _, value } => {
                self.node("Expression::TaggedTemplateExpression", _id.id)
                    .value(value)
                    .end();
            }
            Expression::ArrayExpression { elements: _ } => {
                self.node("Expression::ArrayExpression", _id.id).end();
            }
            Expression::TupleExpression { elements: _ } => {
                self.node("Expression::TupleExpression", _id.id).end();
            }
            Expression::SequenceExpression { expressions: _ } => {
                self.node("Expression::SequenceExpression", _id.id).end();
            }
            Expression::ObjectExpression {
                ty: _,
                properties: _,
            } => {
                self.node("Expression::ObjectExpression", _id.id).end();
            }
            Expression::TreeExpression {
                left: _,
                arguments: _,
                elements: _,
            } => {
                self.node("Expression::TreeExpression", _id.id).end();
            }
            Expression::Parenthesized { expression: _ } => {
                self.node("Expression::Parenthesized", _id.id).end();
            }
            Expression::Type { value: _ } => {
                self.node("Expression::Type", _id.id).end();
            }
            Expression::Comptime { body: _ } => {
                self.node("Expression::Comptime", _id.id).end();
            }
            Expression::Unary { operator, right: _ } => {
                self.node("Expression::Unary", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::As {
                expression: _,
                target_type: _,
            } => {
                self.node("Expression::As", _id.id).end();
            }
            Expression::Satisfies {
                expression: _,
                target_type: _,
            } => {
                self.node("Expression::Satisfies", _id.id).end();
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
            Expression::PointerOf {
                mutability,
                right: _,
            } => {
                self.node("Expression::PointerOf", _id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Expression::Member {
                left: _,
                name,
                generic_arguments: _,
            } => {
                self.node("Expression::Member", _id.id)
                    .field("name", name)
                    .end();
            }
            Expression::PrivateMember {
                left: _,
                name,
                generic_arguments: _,
            } => {
                self.node("Expression::PrivateMember", _id.id)
                    .field("name", name)
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
            Expression::Instantiation {
                left: _,
                generic_arguments: _,
            } => {
                self.node("Expression::Instantiation", _id.id).end();
            }
            Expression::Call {
                position,
                generic_arguments: _,
                left: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::Call", _id.id)
                    .field("position", position)
                    .end();
            }
            Expression::New {
                left: _,
                generic_arguments: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::New", _id.id).end();
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
            Expression::Assign {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::Assign", _id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Debugger => {
                self.node("Expression::Debugger", _id.id).end();
            }
            Expression::Missing => {
                self.node("Expression::Missing", _id.id).end();
            }
            Expression::Stub => {
                self.node("Expression::Stub", _id.id).end();
            }
            Expression::Error => {
                self.node("Expression::Error", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_expression(dumper, _tree, _id, expression);
        });
    }

    fn visit_type_expression(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TypeExpression>,
        type_expression: &TypeExpression,
    ) {
        match type_expression {
            TypeExpression::Parenthesized { expression: _ } => {
                self.node("TypeExpression::Parenthesized", id.id).end();
            }
            TypeExpression::ScalarLiteral { value } => {
                self.node("TypeExpression::ScalarLiteral", id.id)
                    .value(value)
                    .end();
            }
            TypeExpression::Literal { value } => {
                self.node("TypeExpression::Literal", id.id)
                    .value(value)
                    .end();
            }
            TypeExpression::Tuple { elements: _ } => {
                self.node("TypeExpression::Tuple", id.id).end();
            }
            TypeExpression::Array { element: _ } => {
                self.node("TypeExpression::Array", id.id).end();
            }
            TypeExpression::Object { properties: _ } => {
                self.node("TypeExpression::Object", id.id).end();
            }
            TypeExpression::Declaration {
                declaration: declaration_id,
            } => {
                self.node("TypeExpression::Declaration", id.id)
                    .field("declaration_id", &declaration_id.id)
                    .end();
            }
            TypeExpression::Reference {
                path,
                generic_arguments: _,
            } => {
                self.node("TypeExpression::Reference", id.id)
                    .value(path)
                    .end();
            }
            TypeExpression::Member {
                left: _,
                name,
                generic_arguments: _,
            } => {
                self.node("TypeExpression::Member", id.id)
                    .field("name", name)
                    .end();
            }
            TypeExpression::Const => {
                self.node("TypeExpression::Const", id.id).end();
            }
            TypeExpression::This => {
                self.node("TypeExpression::This", id.id).end();
            }
            TypeExpression::Import {
                target: _,
                arguments: _,
                qualifier,
                generic_arguments: _,
            } => {
                self.node("TypeExpression::Import", id.id)
                    .field_optional("qualifier", qualifier)
                    .end();
            }
            TypeExpression::Readonly { target_type: _ } => {
                self.node("TypeExpression::Readonly", id.id).end();
            }
            TypeExpression::KeyOf { target_type: _ } => {
                self.node("TypeExpression::KeyOf", id.id).end();
            }
            TypeExpression::TypeOfValue { value: _ } => {
                self.node("TypeExpression::TypeOfValue", id.id).end();
            }
            TypeExpression::Must { target_type: _ } => {
                self.node("TypeExpression::Must", id.id).end();
            }
            TypeExpression::AsComptime { target_type: _ } => {
                self.node("TypeExpression::AsComptime", id.id).end();
            }
            TypeExpression::Not { target_type: _ } => {
                self.node("TypeExpression::Not", id.id).end();
            }
            TypeExpression::ValueOf {
                mutability,
                variance,
                target_type: _,
            } => {
                self.node("TypeExpression::ValueOf", id.id)
                    .field("mutability", mutability)
                    .field("variance", variance)
                    .end();
            }
            TypeExpression::ReferenceOf {
                mutability,
                variance,
                target_type: _,
            } => {
                self.node("TypeExpression::ReferenceOf", id.id)
                    .field("mutability", mutability)
                    .field("variance", variance)
                    .end();
            }
            TypeExpression::PointerOf {
                mutability,
                target_type: _,
            } => {
                self.node("TypeExpression::PointerOf", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            TypeExpression::Union { elements: _ } => {
                self.node("TypeExpression::Union", id.id).end();
            }
            TypeExpression::Intersection { elements: _ } => {
                self.node("TypeExpression::Intersection", id.id).end();
            }
            TypeExpression::Conditional {
                left: _,
                extends_type: _,
                then_type: _,
                else_type: _,
            } => {
                self.node("TypeExpression::Conditional", id.id).end();
            }
            TypeExpression::Mapped {
                parameter: _,
                readonly,
                optional,
                value: _,
            } => {
                self.node("TypeExpression::Mapped", id.id)
                    .field("readonly", readonly)
                    .field("optional", optional)
                    .end();
            }
            TypeExpression::Index { left: _, index: _ } => {
                self.node("TypeExpression::Index", id.id).end();
            }
            TypeExpression::TemplateLiteral {
                strings: _,
                spans: _,
            } => {
                self.node("TypeExpression::TemplateLiteral", id.id).end();
            }
            TypeExpression::Infer {
                name,
                constraint: _,
            } => {
                self.node("TypeExpression::Infer", id.id)
                    .field("name", name)
                    .end();
            }
            TypeExpression::Predicate {
                asserts,
                subject,
                target: _,
            } => {
                self.node("TypeExpression::Predicate", id.id)
                    .field("asserts", asserts)
                    .field("subject", subject)
                    .end();
            }
            TypeExpression::Missing => {
                self.node("TypeExpression::Missing", id.id).end();
            }
            TypeExpression::Error => {
                self.node("TypeExpression::Error", id.id).end();
            }
        }

        self.with_depth(|dumper| {
            walk_type_expression(dumper, tree, id, type_expression);
        });
    }

    fn visit_block(&mut self, _tree: &NodeTree, _id: LocalNodeId<Block>, block: &Block) {
        self.node("Block", _id.id)
            .field("format", &block.format)
            .end();
        self.with_depth(|dumper| {
            walk_block(dumper, _tree, _id, block);
        });
    }

    fn visit_declaration(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declaration>,
        declaration: &Declaration,
    ) {
        match declaration {
            Declaration::Global(declaration) => {
                self.node("Declaration::Global", id.id)
                    .field("ambient", &declaration.ambient)
                    .end();
            }
            Declaration::Namespace(declaration) => {
                self.node("Declaration::Namespace", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("kind", &declaration.kind)
                    .end();
            }
            Declaration::Type(declaration) => {
                self.node("Declaration::Type", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("is_nominal", &declaration.is_nominal)
                    .field_optional("mutability", &declaration.mutability)
                    .end();
            }
            Declaration::ImportAlias(declaration) => {
                self.node("Declaration::ImportAlias", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("kind", &declaration.kind)
                    .field("target", &declaration.target)
                    .end();
            }
            Declaration::Struct(declaration) => {
                self.node("Declaration::Struct", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .end();
            }
            Declaration::Class(declaration) => {
                self.node("Declaration::Class", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("is_abstract", &declaration.is_abstract)
                    .end();
            }
            Declaration::Enum(declaration) => {
                self.node("Declaration::Enum", id.id)
                    .field_optional("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("kind", &declaration.kind)
                    .end();
            }
            Declaration::Interface(declaration) => {
                self.node("Declaration::Interface", id.id)
                    .field_optional("name", &declaration.name)
                    .field("is_nominal", &declaration.is_nominal)
                    .end();
            }
            Declaration::Extension(declaration) => {
                self.node("Declaration::Extension", id.id)
                    .field_optional("name", &declaration.name)
                    .end();
            }
            Declaration::Function(declaration) => {
                self.node("Declaration::Function", id.id)
                    .field_optional("name", &declaration.name)
                    .field("signature", &declaration.signature)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_declaration(dumper, tree, id, declaration);
        });
    }

    fn visit_property(&mut self, tree: &NodeTree, id: LocalNodeId<Property>, property: &Property) {
        match property {
            Property::Field { key, value: _ } => {
                self.node("Property::Field", id.id).field("key", key).end();
            }
            Property::Method {
                key,
                signature,
                body: _,
            } => {
                self.node("Property::Method", id.id)
                    .field("key", key)
                    .field("signature", signature)
                    .end();
            }
            Property::Spread { value: _ } => {
                self.node("Property::Spread", id.id).end();
            }
            Property::Error => {
                self.node("Property::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_property(dumper, tree, id, property);
        });
    }

    fn visit_member(&mut self, tree: &NodeTree, id: LocalNodeId<Member>, member: &Member) {
        match member {
            Member::Type {
                name: _,
                generic_parameters: _,
                where_clauses: _,
                declared_type: _,
                value: _,
                visibility,
                ambient,
                is_abstract,
                is_override,
                is_static,
            } => {
                self.node("Member::Type", id.id)
                    .field_optional("visibility", visibility)
                    .field("ambient", ambient)
                    .field("is_abstract", is_abstract)
                    .field("is_override", is_override)
                    .field("is_static", is_static)
                    .end();
            }
            Member::ComptimeConst {
                name: _,
                declared_type: _,
                value: _,
                visibility,
                ambient,
                is_static,
            } => {
                self.node("Member::ComptimeConst", id.id)
                    .field_optional("visibility", visibility)
                    .field("ambient", ambient)
                    .field("is_static", is_static)
                    .end();
            }
            Member::Field {
                key,
                declared_type: _,
                default: _,
                is_optional,
                is_readonly,
                mutability,
                visibility,
                ambient,
                is_abstract,
                is_override,
                is_static,
                is_const_asserted,
                is_accessor,
                is_comptime,
            } => {
                self.node("Member::Field", id.id)
                    .field("key", key)
                    .field("is_optional", is_optional)
                    .field("is_readonly", is_readonly)
                    .field_optional("mutability", mutability)
                    .field_optional("visibility", visibility)
                    .field("ambient", ambient)
                    .field("is_abstract", is_abstract)
                    .field("is_override", is_override)
                    .field("is_static", is_static)
                    .field("is_const_asserted", is_const_asserted)
                    .field("is_accessor", is_accessor)
                    .field("is_comptime", is_comptime)
                    .end();
            }
            Member::Method {
                key,
                signature,
                body: _,
                visibility,
                ambient,
                is_abstract,
                is_override,
                is_static,
                is_accessor,
                is_comptime,
            } => {
                self.node("Member::Method", id.id)
                    .field("key", key)
                    .field("signature", signature)
                    .field_optional("visibility", visibility)
                    .field("ambient", ambient)
                    .field("is_abstract", is_abstract)
                    .field("is_override", is_override)
                    .field("is_static", is_static)
                    .field("is_accessor", is_accessor)
                    .field("is_comptime", is_comptime)
                    .end();
            }
            Member::Embed { value: _, .. } => {
                self.node("Member::Embed", id.id).end();
            }
            Member::StaticBlock { body: _ } => {
                self.node("Member::StaticBlock", id.id).end();
            }
            Member::ComptimeBlock { body: _ } => {
                self.node("Member::ComptimeBlock", id.id).end();
            }
            Member::Error => {
                self.node("Member::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_member(dumper, tree, id, member);
        });
    }

    fn visit_enum_field(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<EnumField>,
        field: &EnumField,
    ) {
        self.node("EnumField", _id.id)
            .field("name", &field.name)
            .end();
        self.with_depth(|dumper| {
            walk_enum_field(dumper, _tree, _id, field);
        });
    }

    fn visit_where_clause(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<WhereClause>,
        clause: &WhereClause,
    ) {
        self.node("WhereClause", _id.id)
            .field("left", &clause.left)
            .end();
        self.with_depth(|dumper| {
            walk_where_clause(dumper, _tree, _id, clause);
        });
    }

    fn visit_dependency_item(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<DependencyItem>,
        item: &DependencyItem,
    ) {
        match item {
            DependencyItem::Item {
                mode,
                kind,
                name,
                alias,
                value: _,
            } => {
                self.node("DependencyItem::Item", _id.id)
                    .field("mode", mode)
                    .field_optional("kind", kind)
                    .field_optional("name", name)
                    .field_optional("alias", alias)
                    .end();
            }
            DependencyItem::Error => {
                self.node("DependencyItem::Error", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_dependency_item(dumper, _tree, _id, item);
        });
    }

    fn visit_parameter(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<Parameter>,
        param: &Parameter,
    ) {
        match param {
            Parameter::Named {
                name,
                visibility,
                is_readonly,
                is_optional,
                declared_type: _,
                default: _,
            } => {
                self.node("Parameter::Named", _id.id)
                    .field("name", name)
                    .field("visibility", visibility)
                    .field("is_readonly", is_readonly)
                    .field("is_optional", is_optional)
                    .end();
            }
            Parameter::Pattern {
                pattern: _,
                is_optional,
                declared_type: _,
                default: _,
            } => {
                self.node("Parameter::Pattern", _id.id)
                    .field("is_optional", is_optional)
                    .end();
            }
            Parameter::VariadicNamed {
                name,
                visibility,
                is_readonly,
                declared_type: _,
            } => {
                self.node("Parameter::VariadicNamed", _id.id)
                    .field("name", name)
                    .field("visibility", visibility)
                    .field("is_readonly", is_readonly)
                    .end();
            }
            Parameter::VariadicPattern {
                pattern: _,
                declared_type: _,
            } => {
                self.node("Parameter::VariadicPattern", _id.id).end();
            }
            Parameter::Error => {
                self.node("Parameter::Error", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_parameter(dumper, _tree, _id, param);
        });
    }

    fn visit_tuple_element(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<TupleElement>,
        tuple_element: &TupleElement,
    ) {
        match tuple_element {
            TupleElement::Element {
                label,
                value: _,
                is_optional,
                is_readonly,
            } => {
                self.node("TupleElement::Element", _id.id)
                    .field_optional("label", label)
                    .field("is_optional", is_optional)
                    .field("is_readonly", is_readonly)
                    .end();
            }
            TupleElement::Spread { label, value: _ } => {
                self.node("TupleElement::Spread", _id.id)
                    .field_optional("label", label)
                    .end();
            }
            TupleElement::Error => {
                self.node("TupleElement::Error", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_tuple_element(dumper, _tree, _id, tuple_element);
        });
    }

    fn visit_argument(&mut self, _tree: &NodeTree, _id: LocalNodeId<Argument>, arg: &Argument) {
        match arg {
            Argument::Named { name, value: _ } => {
                self.node("Argument::Named", _id.id)
                    .field("name", name)
                    .end();
            }
            Argument::Labeled { label, value: _ } => {
                self.node("Argument::Labeled", _id.id)
                    .field("label", label)
                    .end();
            }
            Argument::Positional { value: _ } => {
                self.node("Argument::Positional", _id.id).end();
            }
            Argument::Spread { label, value: _ } => {
                self.node("Argument::Spread", _id.id)
                    .field_optional("label", label)
                    .end();
            }
            Argument::Error => {
                self.node("Argument::Error", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_argument(dumper, _tree, _id, arg);
        });
    }

    fn visit_match_case(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<MatchCase>,
        case: &MatchCase,
    ) {
        match case {
            MatchCase::Expression {
                selector: _,
                body: _,
            } => {
                self.node("MatchCase::Expression", _id.id).end();
            }
            MatchCase::Block {
                selector: _,
                body: _,
            } => {
                self.node("MatchCase::Block", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_match_case(dumper, _tree, _id, case);
        });
    }

    fn visit_declarator(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        let Declarator {
            pattern: _,
            ty: _,
            value: _,
        } = declarator;
        self.node("Declarator", _id.id).end();
        self.with_depth(|dumper| {
            walk_declarator(dumper, _tree, _id, declarator);
        });
    }

    fn visit_pattern(&mut self, _tree: &NodeTree, _id: LocalNodeId<Pattern>, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => {
                self.node("Pattern::Wildcard", _id.id).end();
            }
            Pattern::Must(_) => {
                self.node("Pattern::Must", _id.id).end();
            }
            Pattern::ReferenceOf {
                mutability,
                right: _,
            } => {
                self.node("Pattern::ReferenceOf", _id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            Pattern::ValueOf {
                mutability,
                right: _,
            } => {
                self.node("Pattern::ValueOf", _id.id)
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
            Pattern::TypeExpression { value: _ } => {
                self.node("Pattern::TypeExpression", _id.id).end();
            }
            Pattern::Tuple { fields: _ } => {
                self.node("Pattern::Tuple", _id.id).end();
            }
            Pattern::TaggedTuple { ty: _, fields: _ } => {
                self.node("Pattern::TaggedTuple", _id.id).end();
            }
            Pattern::Array { fields: _ } => {
                self.node("Pattern::Array", _id.id).end();
            }
            Pattern::Object { fields: _ } => {
                self.node("Pattern::Object", _id.id).end();
            }
            Pattern::TaggedObject { ty: _, fields: _ } => {
                self.node("Pattern::TaggedObject", _id.id).end();
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
        _id: LocalNodeId<PatternField>,
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
            PatternField::Computed {
                mutability,
                key: _,
                pattern: _,
                default: _,
            } => {
                self.node("PatternField::Computed", _id.id)
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
            PatternField::Positional {
                pattern: _,
                default: _,
            } => {
                self.node("PatternField::Positional", _id.id).end();
            }
            PatternField::Spread {
                mutability,
                pattern: _,
            } => {
                self.node("PatternField::Spread", _id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            PatternField::Elision => {
                self.node("PatternField::Elision", _id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_pattern_field(dumper, _tree, _id, field);
        });
    }

    fn visit_decorator(
        &mut self,
        _tree: &NodeTree,
        _id: LocalNodeId<Decorator>,
        decorator: &Decorator,
    ) {
        self.node("Decorator", _id.id)
            .field("expression", &decorator.expression.id)
            .field("position", &decorator.position)
            .end();
        self.with_depth(|dumper| {
            walk_decorator(dumper, _tree, _id, decorator);
        });
    }
}
