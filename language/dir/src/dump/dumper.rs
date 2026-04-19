#![allow(clippy::match_like_matches_macro)]

use destack_core::{Color, ImmutableStringPool, impl_dump_display, rebuild_tree_output};
use destack_source::ModuleId;
use smallvec::{Array, SmallVec};

use crate::*;

#[derive(Debug, Clone, Copy, Default)]
pub struct DumperOptions {
    /// Use colors.
    pub use_colors: bool = true,
    /// Include node ids in output.
    pub include_node_ids: bool = true,
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
        if let Some(node_id) = self.node_id
            && self.dumper.options.include_node_ids
        {
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

/// Dump a Name as a string.
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
        dumper.write_str(self.to_string(), Some(Color::White));
    }
}

impl_dump_display! {
    Asynchrony,
    AssignOperator,
    BinaryOperator,
    DecoratorPosition,
    ImportAttributeClauseKind,
    ImportSource,
    NamespaceKind,
    DependencyKind,
    DependencyMode,
    ExportMode,
    EnumKind,
    ForEachKind,
    FunctionCardinality,
    FunctionKind,
    FunctionMode,
    IfKind,
    LoopKind,
    MatchKind,
    MatchSource,
    Mutability,
    ReferenceType,
    UnaryOperator,
    TypeKind,
    VarianceModifier,
    VarianceBound,
    Visibility,
    WhileKind,
    YieldCardinality,
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

/// Dump a MappedTypeModifier as a structured representation.
impl Dump for MappedTypeModifier {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            MappedTypeModifier::Present => dumper.object("MappedTypeModifier::Present").end(),
            MappedTypeModifier::Add => dumper.object("MappedTypeModifier::Add").end(),
            MappedTypeModifier::Remove => dumper.object("MappedTypeModifier::Remove").end(),
            MappedTypeModifier::None => dumper.object("MappedTypeModifier::None").end(),
        };
    }
}

/// Dump a MappedTypeModifiers as a structured representation.
impl Dump for MappedTypeModifiers {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("MappedTypeModifiers")
            .field("readonly", &self.readonly)
            .field("optional", &self.optional)
            .end();
    }
}

/// Dump a PredicateSubject as a structured representation.
impl Dump for PredicateSubject {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            PredicateSubject::Unresolved(name) => {
                dumper
                    .object("PredicateSubject::Unresolved")
                    .value(name)
                    .end();
            }
            PredicateSubject::Symbol(symbol) => {
                dumper
                    .object("PredicateSubject::Symbol")
                    .value(symbol)
                    .end();
            }
            PredicateSubject::This => {
                dumper.object("PredicateSubject::This").end();
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

/// Dump a FunctionSignature as a structured object.
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
            .field(
                "generic_parameter_count",
                &(self.generic_parameters.len() as u32),
            )
            .field("where_clause_count", &(self.where_clauses.len() as u32))
            .field_optional("this_parameter", &self.this_parameter.map(|id| id.id))
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
            IntType::Isize => dumper.object("IntType::Isize").end(),
            IntType::Uint8 => dumper.object("IntType::Uint8").end(),
            IntType::Uint16 => dumper.object("IntType::Uint16").end(),
            IntType::Uint32 => dumper.object("IntType::Uint32").end(),
            IntType::Uint64 => dumper.object("IntType::Uint64").end(),
            IntType::Uint128 => dumper.object("IntType::Uint128").end(),
            IntType::Uint256 => dumper.object("IntType::Uint256").end(),
            IntType::Usize => dumper.object("IntType::Usize").end(),
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

/// Dump a TypeIntrinsic as a structured representation.
impl Dump for IntrinsicType {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            IntrinsicType::Uppercase => {
                dumper.object("TypeIntrinsic::Uppercase").end();
            }
            IntrinsicType::Lowercase => {
                dumper.object("TypeIntrinsic::Lowercase").end();
            }
            IntrinsicType::Capitalize => {
                dumper.object("TypeIntrinsic::Capitalize").end();
            }
            IntrinsicType::Uncapitalize => {
                dumper.object("TypeIntrinsic::Uncapitalize").end();
            }
            IntrinsicType::NoInfer => {
                dumper.object("TypeIntrinsic::NoInfer").end();
            }
            IntrinsicType::BuiltinIteratorReturn => {
                dumper.object("TypeIntrinsic::BuiltinIteratorReturn").end();
            }
        };
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
            TypeLiteral::Primitive(primitive) => {
                dumper
                    .object("TypeLiteral::Primitive")
                    .value(primitive)
                    .end();
            }
            TypeLiteral::Intrinsic(intrinsic) => {
                dumper
                    .object("TypeLiteral::Intrinsic")
                    .value(intrinsic)
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
                    .value(content)
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
        tree: &NodeTree,
        id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Declaration(_) => {
                self.node("Expression::Declaration", id.id).end();
            }
            Expression::Block(_) => {
                self.node("Expression::Block", id.id).end();
            }
            Expression::Labelled {
                label,
                body: _,
                symbol,
            } => {
                self.node("Expression::Labelled", id.id)
                    .field("label", label)
                    .field("symbol", symbol)
                    .end();
            }

            Expression::UnresolvedImport {
                source,
                kind,
                target,
                items: _,
                attributes: _,
                arguments: _,
            } => {
                self.node("Expression::UnresolvedImport", id.id)
                    .field("source", source)
                    .field("kind", kind)
                    .field("target", target)
                    .end();
            }
            Expression::Import {
                source,
                kind,
                target,
                target_module,
                items: _,
                attributes: _,
                arguments: _,
            } => {
                self.node("Expression::Import", id.id)
                    .field("source", source)
                    .field("kind", kind)
                    .field("target", target)
                    .field("target_module", target_module)
                    .end();
            }
            Expression::UnresolvedReExport {
                target,
                kind,
                items: _,
                attributes: _,
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
                attributes: _,
            } => {
                self.node("Expression::ReExport", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .field("target_module", target_module)
                    .end();
            }
            Expression::Export {
                kind,
                items: _,
                attributes: _,
            } => {
                self.node("Expression::Export", id.id)
                    .field("kind", kind)
                    .end();
            }
            Expression::ExportNamespace { name } => {
                self.node("Expression::ExportNamespace", id.id)
                    .field("name", name)
                    .end();
            }
            Expression::Let {
                export,
                ambient,
                mutability,
                declarators: _,
            } => {
                self.node("Expression::Let", id.id)
                    .field_optional("export", export)
                    .field("ambient", ambient)
                    .field("mutability", mutability)
                    .end();
            }
            Expression::Using {
                asynchrony,
                export,
                ambient,
                declarators: _,
            } => {
                self.node("Expression::Using", id.id)
                    .field("asynchrony", asynchrony)
                    .field_optional("export", export)
                    .field("ambient", ambient)
                    .end();
            }
            Expression::As {
                operator: _,
                source: _,
                expression: _,
                target_type: _,
            } => {
                self.node("Expression::As", id.id).end();
            }
            Expression::Satisfies {
                expression: _,
                target_type: _,
            } => {
                self.node("Expression::Satisfies", id.id).end();
            }
            Expression::Is {
                value: _,
                target_type: _,
            } => {
                self.node("Expression::Is", id.id).end();
            }
            Expression::InstanceOf {
                value: _,
                target: _,
            } => {
                self.node("Expression::InstanceOf", id.id).end();
            }
            Expression::Unary { operator, right: _ } => {
                self.node("Expression::Unary", id.id)
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
            Expression::PointerOf {
                mutability,
                right: _,
            } => {
                self.node("Expression::PointerOf", id.id)
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
            Expression::Member { left: _, name } => {
                self.node("Expression::Member", id.id)
                    .field("name", name)
                    .end();
            }
            Expression::PrivateMember { left: _, name } => {
                self.node("Expression::PrivateMember", id.id)
                    .field("name", name)
                    .end();
            }
            Expression::Instantiation {
                left: _,
                generic_arguments: _,
            } => {
                self.node("Expression::Instantiation", id.id).end();
            }
            Expression::Call {
                left: _,
                generic_arguments: _,
                arguments: _,
            } => {
                self.node("Expression::Call", id.id).end();
            }
            Expression::New {
                left: _,
                generic_arguments: _,
                arguments: _,
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

            Expression::UnresolvedPath {
                path,
                generic_arguments: _,
                space_order: _,
            } => {
                self.node("Expression::UnresolvedPath", id.id)
                    .field("path", path)
                    .end();
            }
            Expression::LocalReference {
                path,
                generic_arguments: _,
                target_symbol,
            } => {
                self.node("Expression::LocalReference", id.id)
                    .field("path", path)
                    .field("target_symbol", target_symbol)
                    .end();
            }
            Expression::ModuleReference {
                path,
                generic_arguments: _,
                target_symbol,
            } => {
                self.node("Expression::ModuleReference", id.id)
                    .field("path", path)
                    .field("target_symbol", target_symbol)
                    .end();
            }
            Expression::GlobalReference {
                path,
                generic_arguments: _,
                target_symbol,
            } => {
                self.node("Expression::GlobalReference", id.id)
                    .field("path", path)
                    .field("target_symbol", target_symbol)
                    .end();
            }
            Expression::PrivateIdentifier { name } => {
                self.node("Expression::PrivateIdentifier", id.id)
                    .field("name", name)
                    .end();
            }
            Expression::ImportMeta => {
                self.node("Expression::ImportMeta", id.id).end();
            }
            Expression::NewTarget => {
                self.node("Expression::NewTarget", id.id).end();
            }
            Expression::This => {
                self.node("Expression::This", id.id).end();
            }
            Expression::Super => {
                self.node("Expression::Super", id.id).end();
            }

            Expression::Type {
                value: _,
                resolved_type: _,
            } => {
                self.node("Expression::Type", id.id).end();
            }
            Expression::ScalarLiteral { value } => {
                self.node("Expression::ScalarLiteral", id.id)
                    .field("value", value)
                    .end();
            }
            Expression::TaggedTemplateExpression {
                tag: _,
                generic_arguments: _,
                value,
            } => {
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
            Expression::ArrayExpression { elements: _ } => {
                self.node("Expression::ArrayExpression", id.id).end();
            }
            Expression::TupleExpression { elements: _ } => {
                self.node("Expression::TupleExpression", id.id).end();
            }
            Expression::SequenceExpression { expressions: _ } => {
                self.node("Expression::SequenceExpression", id.id).end();
            }
            Expression::ObjectExpression {
                ty: _,
                properties: _,
            } => {
                self.node("Expression::ObjectExpression", id.id).end();
            }
            Expression::TreeExpression {
                left: _,
                generic_arguments: _,
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
                binding: _,
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
                catch_ty: _,
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
                kind,
                value: _,
                cases: _,
                source,
                scope,
                symbol,
            } => {
                self.node("Expression::Match", id.id)
                    .field("kind", kind)
                    .field("source", source)
                    .field("scope", scope)
                    .field("symbol", symbol)
                    .end();
            }
            Expression::UnresolvedBreak { target, value: _ } => {
                self.node("Expression::UnresolvedBreak", id.id)
                    .field("target", target)
                    .end();
            }
            Expression::Break {
                target,
                target_symbol,
                value: _,
            } => {
                self.node("Expression::Break", id.id)
                    .field_optional("target", target)
                    .field_optional("target_symbol", target_symbol)
                    .end();
            }
            Expression::UnresolvedContinue { target } => {
                self.node("Expression::UnresolvedContinue", id.id)
                    .field("target", target)
                    .end();
            }
            Expression::Continue {
                target,
                target_symbol,
            } => {
                self.node("Expression::Continue", id.id)
                    .field_optional("target", target)
                    .field_optional("target_symbol", target_symbol)
                    .end();
            }
            Expression::Throw { value: _ } => {
                self.node("Expression::Throw", id.id).end();
            }
            Expression::Await { expression: _ } => {
                self.node("Expression::Await", id.id).end();
            }
            Expression::AwaitMaybe { expression: _ } => {
                self.node("Expression::AwaitMaybe", id.id).end();
            }
            Expression::Comptime { body: _ } => {
                self.node("Expression::Comptime", id.id).end();
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
            Expression::Debugger => {
                self.node("Expression::Debugger", id.id).end();
            }
            Expression::Missing => {
                self.node("Expression::Missing", id.id).end();
            }
            Expression::Stub => {
                self.node("Expression::Stub", id.id).end();
            }
            Expression::Error => {
                self.node("Expression::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_expression(dumper, tree, id, expression);
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
            Declaration::Global(declaration) => {
                self.node("Declaration::Global", id.id)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field("scope", &declaration.scope)
                    .end();
            }
            Declaration::Namespace(declaration) => {
                self.node("Declaration::Namespace", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field("kind", &declaration.kind)
                    .field(
                        "generic_parameter_count",
                        &(declaration.generic_parameters.len() as u32),
                    )
                    .field(
                        "where_clause_count",
                        &(declaration.where_clauses.len() as u32),
                    )
                    .field("scope", &declaration.scope)
                    .end();
            }
            Declaration::Type(declaration) => {
                self.node("Declaration::Type", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field("is_nominal", &declaration.is_nominal)
                    .field_optional("mutability", &declaration.mutability)
                    .end();
            }
            Declaration::ImportAlias(declaration) => {
                self.node("Declaration::ImportAlias", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field("kind", &declaration.kind)
                    .field("target", &declaration.target)
                    .end();
            }
            Declaration::Struct(declaration) => {
                self.node("Declaration::Struct", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field(
                        "generic_parameter_count",
                        &(declaration.generic_parameters.len() as u32),
                    )
                    .field(
                        "where_clause_count",
                        &(declaration.where_clauses.len() as u32),
                    )
                    .field("scope", &declaration.scope)
                    .end();
            }
            Declaration::Class(declaration) => {
                self.node("Declaration::Class", id.id)
                    .field("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field_optional("self_symbol", &declaration.self_symbol)
                    .field("is_abstract", &declaration.is_abstract)
                    .field(
                        "generic_parameter_count",
                        &(declaration.generic_parameters.len() as u32),
                    )
                    .field(
                        "where_clause_count",
                        &(declaration.where_clauses.len() as u32),
                    )
                    .field("scope", &declaration.scope)
                    .end();
            }
            Declaration::Enum(declaration) => {
                self.node("Declaration::Enum", id.id)
                    .field_optional("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field("kind", &declaration.kind)
                    .field(
                        "generic_parameter_count",
                        &(declaration.generic_parameters.len() as u32),
                    )
                    .field(
                        "where_clause_count",
                        &(declaration.where_clauses.len() as u32),
                    )
                    .field("scope", &declaration.scope)
                    .end();
            }
            Declaration::Interface(declaration) => {
                self.node("Declaration::Interface", id.id)
                    .field_optional("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field("is_nominal", &declaration.is_nominal)
                    .field(
                        "generic_parameter_count",
                        &(declaration.generic_parameters.len() as u32),
                    )
                    .field(
                        "where_clause_count",
                        &(declaration.where_clauses.len() as u32),
                    )
                    .field("scope", &declaration.scope)
                    .end();
            }
            Declaration::Extension(declaration) => {
                self.node("Declaration::Extension", id.id)
                    .field_optional("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field(
                        "generic_parameter_count",
                        &(declaration.generic_parameters.len() as u32),
                    )
                    .field(
                        "where_clause_count",
                        &(declaration.where_clauses.len() as u32),
                    )
                    .field_optional("target_symbol", &declaration.target_symbol)
                    .field("scope", &declaration.scope)
                    .end();
            }
            Declaration::Function(declaration) => {
                self.node("Declaration::Function", id.id)
                    .field_optional("name", &declaration.name)
                    .field_optional("export", &declaration.export)
                    .field("ambient", &declaration.ambient)
                    .field("symbol", &declaration.symbol)
                    .field_optional("self_symbol", &declaration.self_symbol)
                    .field("signature", &declaration.signature)
                    .field("scope", &declaration.scope)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_declaration(dumper, tree, id, declaration);
        });
    }

    fn visit_declarator(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Declarator>,
        declarator: &Declarator,
    ) {
        let Declarator {
            pattern: _,
            ty: _,
            value: _,
        } = declarator;
        self.node("Declarator", id.id).end();
        self.with_depth(|dumper| {
            walk_declarator(dumper, tree, id, declarator);
        });
    }

    fn visit_property(&mut self, tree: &NodeTree, id: LocalNodeId<Property>, property: &Property) {
        match property {
            Property::Field {
                key,
                value: _,
                symbol,
            } => {
                self.node("Property::Field", id.id)
                    .field("key", key)
                    .field("symbol", symbol)
                    .end();
            }
            Property::Method {
                key,
                signature,
                body: _,
                symbol,
            } => {
                self.node("Property::Method", id.id)
                    .field("key", key)
                    .field("signature", signature)
                    .field("symbol", symbol)
                    .end();
            }
            Property::Spread { value: _, symbol } => {
                self.node("Property::Spread", id.id)
                    .field("symbol", symbol)
                    .end();
            }
            Property::Error { symbol } => {
                self.node("Property::Error", id.id)
                    .field("symbol", symbol)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_property(dumper, tree, id, property);
        });
    }

    fn visit_member(&mut self, tree: &NodeTree, id: LocalNodeId<Member>, member: &Member) {
        match member {
            Member::AssociatedType {
                name: _,
                generic_parameters: _,
                where_clauses: _,
                constraint: _,
                value: _,
                visibility,
                ambient,
                is_abstract,
                is_override,
                is_static,
                symbol,
            } => {
                self.node("Member::AssociatedType", id.id)
                    .field_optional("visibility", visibility)
                    .field("ambient", ambient)
                    .field("is_abstract", is_abstract)
                    .field("is_override", is_override)
                    .field("is_static", is_static)
                    .field("symbol", symbol)
                    .end();
            }
            Member::AssociatedConst {
                name: _,
                declared_type: _,
                value: _,
                visibility,
                ambient,
                is_static,
                symbol,
            } => {
                self.node("Member::AssociatedConst", id.id)
                    .field_optional("visibility", visibility)
                    .field("ambient", ambient)
                    .field("is_static", is_static)
                    .field("symbol", symbol)
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
                is_definite,
                is_accessor,
                is_comptime,
                symbol,
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
                    .field("is_definite", is_definite)
                    .field("is_accessor", is_accessor)
                    .field("is_comptime", is_comptime)
                    .field("symbol", symbol)
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
                symbol,
            } => {
                self.node("Member::Method", id.id)
                    .field_optional("key", key)
                    .field("signature", signature)
                    .field_optional("visibility", visibility)
                    .field("ambient", ambient)
                    .field("is_abstract", is_abstract)
                    .field("is_override", is_override)
                    .field("is_static", is_static)
                    .field("is_accessor", is_accessor)
                    .field("is_comptime", is_comptime)
                    .field("symbol", symbol)
                    .end();
            }
            Member::Embed {
                value: _,
                visibility,
                ambient,
                is_static,
                symbol,
            } => {
                self.node("Member::Embed", id.id)
                    .field_optional("visibility", visibility)
                    .field("ambient", ambient)
                    .field("is_static", is_static)
                    .field("symbol", symbol)
                    .end();
            }
            Member::StaticBlock { body: _, symbol } => {
                self.node("Member::StaticBlock", id.id)
                    .field("symbol", symbol)
                    .end();
            }
            Member::ComptimeBlock { body: _, symbol } => {
                self.node("Member::ComptimeBlock", id.id)
                    .field("symbol", symbol)
                    .end();
            }
            Member::Error { symbol } => {
                self.node("Member::Error", id.id)
                    .field("symbol", symbol)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_member(dumper, tree, id, member);
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
        self.node("WhereClause", id.id)
            .field("left", &where_clause.left)
            .end();
        self.with_depth(|dumper| {
            walk_where_clause(dumper, tree, id, where_clause);
        });
    }

    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        match dependency_item {
            DependencyItem::Error => {
                self.node("DependencyItem::Error", id.id).end();
            }
            DependencyItem::UnresolvedRemote {
                source,
                mode,
                kind,
                name,
                alias,
                target,
                target_module,
                symbol,
            } => {
                self.node("DependencyItem::UnresolvedRemote", id.id)
                    .field("source", source)
                    .field("mode", mode)
                    .field("kind", kind)
                    .field_optional("name", name)
                    .field_optional("alias", alias)
                    .field("target", target)
                    .field_optional("modules", target_module)
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
            DependencyItem::Value { mode, value: _ } => {
                self.node("DependencyItem::Value", id.id)
                    .field("mode", mode)
                    .end();
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
                name,
                visibility,
                is_readonly,
                is_optional,
                declared_type: _,
                symbol,
                default: _,
            } => {
                self.node("Parameter::Scalar", id.id)
                    .field("name", name)
                    .field("visibility", visibility)
                    .field("is_readonly", is_readonly)
                    .field("is_optional", is_optional)
                    .field("symbol", symbol)
                    .end();
            }
            Parameter::Pattern {
                pattern: _,
                is_optional,
                declared_type: _,
                symbol,
                default: _,
            } => {
                self.node("Parameter::Pattern", id.id)
                    .field("is_optional", is_optional)
                    .field("symbol", symbol)
                    .end();
            }
            Parameter::VariadicNamed {
                name,
                visibility,
                is_readonly,
                declared_type: _,
                symbol,
            } => {
                self.node("Parameter::VariadicNamed", id.id)
                    .field("name", name)
                    .field("visibility", visibility)
                    .field("is_readonly", is_readonly)
                    .field("symbol", symbol)
                    .end();
            }
            Parameter::VariadicPattern {
                pattern: _,
                declared_type: _,
                symbol,
            } => {
                self.node("Parameter::VariadicPattern", id.id)
                    .field("symbol", symbol)
                    .end();
            }
            Parameter::Error { symbol } => {
                self.node("Parameter::Error", id.id)
                    .field("symbol", symbol)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_parameter(dumper, tree, id, parameter);
        });
    }

    fn visit_tuple_element(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<TupleElement>,
        tuple_element: &TupleElement,
    ) {
        match tuple_element {
            TupleElement::Element {
                label,
                value: _,
                is_optional,
                is_readonly,
            } => {
                self.node("TupleElement::Element", id.id)
                    .field_optional("label", label)
                    .field("is_optional", is_optional)
                    .field("is_readonly", is_readonly)
                    .end();
            }
            TupleElement::Spread { label, value: _ } => {
                self.node("TupleElement::Spread", id.id)
                    .field_optional("label", label)
                    .end();
            }
            TupleElement::Error => {
                self.node("TupleElement::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_tuple_element(dumper, tree, id, tuple_element);
        });
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: LocalNodeId<Argument>, argument: &Argument) {
        match argument {
            Argument::Named { name, value: _, .. } => {
                self.node("Argument::Named", id.id)
                    .field("name", name)
                    .end();
            }
            Argument::Positional { value: _, .. } => {
                self.node("Argument::Positional", id.id).end();
            }
            Argument::Spread {
                label, value: _, ..
            } => {
                self.node("Argument::Spread", id.id)
                    .field_optional("label", label)
                    .end();
            }
            Argument::Labeled {
                label, value: _, ..
            } => {
                self.node("Argument::Labeled", id.id)
                    .field("label", label)
                    .end();
            }
            Argument::Error { value: _ } => {
                self.node("Argument::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_argument(dumper, tree, id, argument);
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
                selector: _,
                body: _,
                scope,
            } => {
                self.node("MatchCase::Expression", id.id)
                    .field("scope", scope)
                    .end();
            }
            MatchCase::Block {
                selector: _,
                body: _,
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
            Pattern::Must(_) => {
                self.node("Pattern::Must", id.id).end();
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
            Pattern::TypeExpression { value: _ } => {
                self.node("Pattern::TypeExpression", id.id).end();
            }
            Pattern::Tuple { fields: _ } => {
                self.node("Pattern::Tuple", id.id).end();
            }
            Pattern::TaggedTuple { ty: _, fields: _ } => {
                self.node("Pattern::TaggedTuple", id.id).end();
            }
            Pattern::Array { fields: _ } => {
                self.node("Pattern::Array", id.id).end();
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
            } => {
                self.node("PatternField::Named", id.id)
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
                self.node("PatternField::Computed", id.id)
                    .field_optional("mutability", mutability)
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
            PatternField::Positional {
                pattern: _,
                default: _,
            } => {
                self.node("PatternField::Positional", id.id).end();
            }
            PatternField::Spread {
                mutability,
                pattern: _,
            } => {
                self.node("PatternField::Spread", id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            PatternField::Elision => {
                self.node("PatternField::Elision", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_pattern_field(dumper, tree, id, pattern_field);
        });
    }

    fn visit_decorator(
        &mut self,
        tree: &NodeTree,
        id: LocalNodeId<Decorator>,
        decorator: &Decorator,
    ) {
        self.node("Decorator", id.id)
            .field("position", &decorator.position)
            .field("expression", &decorator.expression.id)
            .end();

        self.with_depth(|dumper| {
            walk_decorator(dumper, tree, id, decorator);
        });
    }
}

// ----------------------------------------------------------------------------
// Meta
// ----------------------------------------------------------------------------

impl_dump_display! {
    ModuleTarget,
    ScopeKind,
    SymbolSpace,
    SymbolKind,
    NodeType,
}

impl Dump for ModuleResolution {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("{self:?}"), Some(Color::Green));
    }
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
            format!(
                "{}:{}:{:?}",
                self.module_id, self.local_id.id, self.local_id.ty
            ),
            Some(Color::Green),
        );
    }
}

impl Dump for LocalSymbolId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(format!("#{}", self.id), Some(Color::Green));
    }
}

impl Dump for GlobalScopeId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_str(
            format!("{}:#{}", self.module_id, self.local_id.0),
            Some(Color::Green),
        );
    }
}

impl Dump for StaticKey {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            StaticKey::Name(name) => {
                dumper.object("StaticKey::Name").value(name).end();
            }
            StaticKey::Number(name) => {
                dumper.object("StaticKey::Number").value(name).end();
            }
            StaticKey::Symbol(symbol) => {
                dumper.object("StaticKey::Symbol").value(symbol).end();
            }
        }
    }
}

impl Dump for SymbolKey {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            SymbolKey::Unique(symbol) => {
                dumper.object("SymbolKey::Unique").value(symbol).end();
            }
            SymbolKey::WellKnown(symbol) => {
                let name = symbol.global_symbol_name();
                dumper.object("SymbolKey::WellKnown").value(&name).end();
            }
            SymbolKey::Registry(name) => {
                dumper.object("SymbolKey::Registry").value(name).end();
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
            for (_key, symbol_id) in symbols.active_named_symbols(scope) {
                let symbol = symbols.get_symbol(symbol_id);
                dumper.visit_symbol(tree, symbols, symbol_id, symbol);
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
        self.node_like("Symbol", Some(id.id), Some(symbol.scope.0.0))
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
