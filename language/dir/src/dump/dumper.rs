#![allow(clippy::match_like_matches_macro)]

use dyst_source::{Color, ImmutableStringPool, SmallVec, impl_dump_display, rebuild_tree_output};

use crate::*;

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
            let (module_id, source_ast_id) = self.dumper.tree.get_source(node_id);
            let module_id = module_id.0;
            if let Some(source_ast_id) = source_ast_id {
                self.dumper.write_str(
                    format!(" :{node_id} [{module_id:?}/{source_ast_id}]").as_str(),
                    Some(Color::White),
                );
            } else {
                self.dumper.write_str(
                    format!(" :{node_id} [{module_id:?}]").as_str(),
                    Some(Color::White),
                );
            }
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

/// Dump a StringId as a string.
impl Dump for StringId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_string_id(*self);
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

impl_dump_display! {
    AnnotationPosition,
    Asynchrony,
    AssignOperator,
    BindingKind,
    BindingOperator,
    BinaryOperator,
    DeclarationKind,
    DependencyKind,
    DependencySource,
    ExportType,
    ForEachKind,
    FunctionAbstraction,
    FunctionCardinality,
    FunctionKind,
    FunctionMode,
    IfKind,
    LoopFile,
    MatchFile,
    Mutability,
    ReferenceType,
    Runtime,
    StructKind,
    TypeBinaryOperator,
    TypeUnaryOperator,
    UnaryOperator,
    TypeKind,
    VarianceBound,
    Visibility,
    WhileKind,
}

/// Dump a BindingModifier as a string.
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

/// Dump an ArgumentSlot as a string.
impl Dump for ArgumentSlot {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ArgumentSlot::Parameter { parameter: _ } => {
                dumper.object("ArgumentSlot::Parameter").end();
            }
            ArgumentSlot::Property { property: _ } => {
                dumper.object("ArgumentSlot::Field").end();
            }
        }
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

/// Dump a BlockTarget as a structured representation.
impl Dump for BlockTarget {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            BlockTarget::UnevaluatedString { label } => {
                dumper
                    .object("BlockTarget::UnevaluatedString")
                    .field("label", label)
                    .end();
            }
            BlockTarget::Definition { .. } => {
                dumper.object("BlockTarget::Definition").end();
            }
        }
    }
}

/// Dump a PathBase as a string.
impl Dump for PathBase {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            PathBase::SelfValue => dumper.write_str("PathBase::SelfValue", Some(Color::Yellow)),
            PathBase::SelfType => dumper.write_str("PathBase::SelfType", Some(Color::Yellow)),
            PathBase::SuperValue => dumper.write_str("PathBase::SuperValue", Some(Color::Yellow)),
            PathBase::SuperType => dumper.write_str("PathBase::SuperType", Some(Color::Yellow)),
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
            FloatType::Float16 => dumper.object("FloatType::Float16").end(),
            FloatType::Float32 => dumper.object("FloatType::Float32").end(),
            FloatType::Float64 => dumper.object("FloatType::Float64").end(),
            FloatType::Float80 => dumper.object("FloatType::Float80").end(),
            FloatType::Float128 => dumper.object("FloatType::Float128").end(),
            FloatType::Arbitrary { width } => dumper
                .object("FloatType::Arbitrary")
                .field("width", width)
                .end(),
        };
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
                dumper.object("DefinitionType::Module").end();
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
            TemplateLiteral::String { template } => {
                dumper
                    .object("TemplateLiteral::String")
                    .field("template", template)
                    .end();
            }
            TemplateLiteral::TaggedString { tag, template } => {
                dumper
                    .object("TemplateLiteral::TaggedString")
                    .field("tag", tag)
                    .field("template", template)
                    .end();
            }
            TemplateLiteral::InterpolatedString {
                template,
                arguments: _,
            } => {
                dumper
                    .object("TemplateLiteral::InterpolatedString")
                    .field("template", template)
                    .end();
            }
            TemplateLiteral::TaggedInterpolatedString {
                tag,
                template,
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
        tree: &MutableNodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Definition { definition: _ } => {
                self.node("Expression::Definition", id.id).end();
            }
            Expression::Block { block: _ } => {
                self.node("Expression::Block", id.id).end();
            }
            Expression::Statement { statement: _ } => {
                self.node("Expression::Statement", id.id).end();
            }

            Expression::With {
                clauses: _,
                body: _,
            } => {
                self.node("Expression::With", id.id).end();
            }
            Expression::Import {
                kind,
                items: _,
                arguments: _,
            } => {
                self.node("Expression::Import", id.id)
                    .field("kind", kind)
                    .end();
            }
            Expression::Export {
                mode,
                kind,
                items: _,
                value: _,
            } => {
                self.node("Expression::Export", id.id)
                    .field("mode", mode)
                    .field("kind", kind)
                    .end();
            }
            Expression::Let {
                mutability,
                pattern: _,
                ty: _,
                value: _,
            } => {
                self.node("Expression::Let", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Expression::LetType {
                kind,
                mutability,
                name,
                static_parameters: _,
                value: _,
            } => {
                self.node("Expression::Type", id.id)
                    .field("kind", kind)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }

            Expression::UnevaluatedUnary { operator, right: _ } => {
                self.node("Expression::UnevaluatedUnary", id.id)
                    .field("operator", operator)
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
            Expression::UnevaluatedBinary {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::UnevaluatedBinary", id.id)
                    .field("operator", operator)
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
            Expression::AssignDirect { left: _, right: _ } => {
                self.node("Expression::AssignDirect", id.id).end();
            }
            Expression::UnevaluatedAssignBinary {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Expression::UnevaluatedAssignBinary", id.id)
                    .field("operator", operator)
                    .end();
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
                path,
                static_arguments: _,
            } => {
                self.node("Expression::Member", id.id)
                    .field("path", path)
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
                left,
                static_arguments: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::New", id.id)
                    .field("left", left)
                    .end();
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

            Expression::Path {
                path,
                static_arguments: _,
            } => {
                self.node("Expression::Path", id.id)
                    .field("path", path)
                    .end();
            }
            Expression::ScalarLiteral { value } => {
                self.node("Expression::ScalarLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            Expression::TemplateLiteral { value } => {
                self.node("Expression::TemplateLiteral", id.id)
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
            Expression::StructLiteral {
                ty: _,
                properties: _,
            } => {
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
                kind,
                condition: _,
                then_expression: _,
                else_expression: _,
            } => {
                self.node("Expression::If", id.id).field("kind", kind).end();
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
            Expression::ForEach {
                asynchrony,
                kind,
                pattern: _,
                iterator: _,
                body: _,
            } => {
                self.node("Expression::ForEach", id.id)
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
                self.node("Expression::For", id.id).end();
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
            Expression::Break { target, value: _ } => {
                self.node("Expression::Break", id.id)
                    .field("target", target)
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

    fn visit_block(&mut self, tree: &MutableNodeTree, id: NodeId<Block>, block: &Block) {
        self.node("Block", id.id).end();
        self.with_depth(|dumper| {
            walk_block(dumper, tree, id, block);
        });
    }

    fn visit_definition(
        &mut self,
        tree: &MutableNodeTree,
        id: NodeId<Definition>,
        definition: &Definition,
    ) {
        match definition {
            Definition::UnevaluatedExpression { expression: _ } => {
                self.node("Definition::UnevaluatedExpression", id.id).end();
            }
            Definition::Namespace {
                descriptor,
                generics,
                definitions: _,
            } => {
                self.node("Definition::Module", id.id)
                    .field("descriptor", descriptor)
                    .field_optional("generics", generics)
                    .end();
            }
            Definition::Struct {
                descriptor,
                kind,
                generics,
                heritage,
                properties: _,
            } => {
                self.node("Definition::Struct", id.id)
                    .field("descriptor", descriptor)
                    .field("kind", kind)
                    .field_optional("generics", generics)
                    .field_optional("heritage", heritage)
                    .end();
            }
            Definition::Enum {
                descriptor,
                generics,
                heritage,
                fields: _,
                properties: _,
            } => {
                self.node("Definition::Enum", id.id)
                    .field("descriptor", descriptor)
                    .field_optional("generics", generics)
                    .field_optional("heritage", heritage)
                    .end();
            }
            Definition::Interface {
                descriptor,
                generics,
                heritage,
                properties: _,
            } => {
                self.node("Definition::Interface", id.id)
                    .field("descriptor", descriptor)
                    .field_optional("generics", generics)
                    .field_optional("heritage", heritage)
                    .end();
            }
            Definition::Function {
                descriptor,
                signature,
                definitions: _,
                body: _,
            } => {
                self.node("Definition::Function", id.id)
                    .field("descriptor", descriptor)
                    .field("signature", signature)
                    .end();
            }
            Definition::Implement {
                descriptor,
                generics,
                target_type: _,
                heritage,
                properties: _,
            } => {
                self.node("Definition::Extension", id.id)
                    .field("descriptor", descriptor)
                    .field_optional("generics", generics)
                    .field_optional("heritage", heritage)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_definition(dumper, tree, id, definition);
        });
    }

    fn visit_type(&mut self, tree: &MutableNodeTree, id: NodeId<Type>, ty: &Type) {
        match ty {
            Type::Scalar(scalar) => {
                self.node("Type::Scalar", id.id)
                    .field("scalar", scalar)
                    .end();
            }
            Type::Definition(_) => {
                self.node("Type::Definition", id.id).end();
            }

            Type::Unary { operator, right: _ } => {
                self.node("Type::Unary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Type::Mutable {
                mutability,
                right: _,
            } => {
                self.node("Type::Mutable", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Type::ValueOf {
                mutability,
                variance,
                right: _,
            } => {
                self.node("Type::ValueOf", id.id)
                    .field_optional("mutability", mutability)
                    .field_optional("variance", variance)
                    .end();
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right: _,
            } => {
                self.node("Type::ReferenceOf", id.id)
                    .field_optional("mutability", mutability)
                    .field_optional("variance", variance)
                    .end();
            }
            Type::Binary {
                left: _,
                operator,
                right: _,
            } => {
                self.node("Type::Binary", id.id)
                    .field("operator", operator)
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
            Type::ArrayStatic {
                element: _,
                count: _,
            } => {
                self.node("Type::ArrayStatic", id.id).end();
            }
            Type::ArraySlice { element: _ } => {
                self.node("Type::ArraySlice", id.id).end();
            }
            Type::ArrayDynamic { elements: _ } => {
                self.node("Type::ArrayDynamic", id.id).end();
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
            Type::Function { signature } => {
                self.node("Type::Function", id.id).value(signature).end();
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

    fn visit_where_clause(
        &mut self,
        tree: &MutableNodeTree,
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

    fn visit_with_clause(
        &mut self,
        tree: &MutableNodeTree,
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

    fn visit_dependency_item(
        &mut self,
        tree: &MutableNodeTree,
        id: NodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        match dependency_item {
            DependencyItem::SideEffect { kind, target } => {
                self.node("DependencyItem::SideEffect", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .end();
            }
            DependencyItem::Namespace {
                kind,
                target,
                alias,
            } => {
                self.node("DependencyItem::Glob", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .field("alias", alias)
                    .end();
            }
            DependencyItem::Named {
                kind,
                target,
                name,
                alias,
            } => {
                self.node("DependencyItem::Scalar", id.id)
                    .field("kind", kind)
                    .field_optional("target", target)
                    .field("name", name)
                    .field_optional("alias", alias)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_dependency_item(dumper, tree, id, dependency_item);
        });
    }

    fn visit_parameter(
        &mut self,
        tree: &MutableNodeTree,
        id: NodeId<Parameter>,
        parameter: &Parameter,
    ) {
        match parameter {
            Parameter::Named {
                modifiers,
                name,
                ty: _,
                default: _,
            } => {
                self.node("Parameter::Scalar", id.id)
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
                self.node("Parameter::Pattern", id.id)
                    .field_optional("modifiers", modifiers)
                    .end();
            }
            Parameter::Variadic {
                modifiers,
                name,
                ty: _,
            } => {
                self.node("Parameter::Variadic", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_parameter(dumper, tree, id, parameter);
        });
    }

    fn visit_argument(
        &mut self,
        tree: &MutableNodeTree,
        id: NodeId<Argument>,
        argument: &Argument,
    ) {
        match argument {
            Argument::UnevaluatedNamed {
                modifiers,
                name,
                value: _,
            } => {
                self.node("Argument::UnevaluatedNamed", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
            Argument::UnevaluatedPositional {
                modifiers,
                value: _,
            } => {
                self.node("Argument::UnevaluatedPositional", id.id)
                    .field_optional("modifiers", modifiers)
                    .end();
            }
            Argument::UnevaluatedSpread {
                modifiers,
                name,
                value: _,
            } => {
                self.node("Argument::UnevaluatedSpread", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
            Argument::UnevaluatedDynamic {
                modifiers,
                name,
                key: _,
                value: _,
            } => {
                self.node("Argument::UnevaluatedDynamic", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
            Argument::Direct {
                modifiers,
                name,
                slot,
                value: _,
            } => {
                self.node("Argument::Direct", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .field("slot", slot)
                    .end();
            }
            Argument::Spread {
                modifiers,
                slot,
                value: _,
            } => {
                self.node("Argument::Spread", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("slot", slot)
                    .end();
            }
            Argument::Dynamic {
                modifiers,
                name,
                key: _,
                value: _,
            } => {
                self.node("Argument::Dynamic", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_argument(dumper, tree, id, argument);
        });
    }

    fn visit_pattern(&mut self, tree: &MutableNodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
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
            Pattern::Reference {
                right: _,
                mutability,
            } => {
                self.node("Pattern::ReferenceOf", id.id)
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
        tree: &MutableNodeTree,
        id: NodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        match pattern_field {
            PatternField::Named {
                mutability,
                name,
                default: _,
                pattern: _,
            } => {
                self.node("PatternField::Named", id.id)
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
                self.node("PatternField::Alias", id.id)
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

    fn visit_match_case(
        &mut self,
        tree: &MutableNodeTree,
        id: NodeId<MatchCase>,
        match_case: &MatchCase,
    ) {
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

    fn visit_annotation(
        &mut self,
        tree: &MutableNodeTree,
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
