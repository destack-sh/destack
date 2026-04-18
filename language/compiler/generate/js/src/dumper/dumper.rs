#![allow(clippy::match_like_matches_macro)]

use destack_core::{Color, ImmutableStringPool, StringId, impl_dump_display, rebuild_tree_output};
use destack_js as js;
use smallvec::{Array, SmallVec};
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

/// A Dumper for dumping JS AST nodes.
#[derive(Debug)]
pub struct Dumper<'a> {
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
    /// The node tree.
    pub tree: &'a js::NodeTree,
    /// The dump options.
    pub options: DumperOptions,

    /// The visitor options.
    visitor_options: js::NodeVisitorOptions,
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
        tree: &'a js::NodeTree,
        options: DumperOptions,
    ) -> Self {
        Self {
            strings,
            tree,
            options,
            visitor_options: js::NodeVisitorOptions::default(),
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

    /// Write the path represented by one JS path.
    #[inline]
    pub fn write_path(&mut self, path: &js::Path) {
        for (index, segment) in path.segments.iter().enumerate() {
            let segment_str = self.strings.get(*segment);
            self.write_str(segment_str, Some(Color::Green));
            if index + 1 < path.segments.len() {
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
        if self.node_id.is_some() {
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
        dumper.write_str(self.as_ref(), Some(Color::BrightYellow));
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

/// Dump a StringId as a string.
impl Dump for StringId {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_string_id(*self);
    }
}

/// Dump a NodeId<T> as the node it points to.
impl<T: js::Node + Clone + Dump> Dump for js::LocalNodeId<T>
where
    js::NodeTree: js::NodeTreeImpl<T>,
{
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let node = dumper.tree.get(*self);
        node.dump(dumper);
    }
}

/// Dump one JS path as a dotted string.
impl Dump for js::Path {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_path(self);
    }
}

/// Dump one JS name as a string selector.
impl Dump for js::Name {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            js::Name::Identifier(id) => {
                id.dump(dumper);
            }
            js::Name::String(id) => {
                dumper.write_char('[', Some(Color::White));
                id.dump(dumper);
                dumper.write_char(']', Some(Color::White));
            }
        }
    }
}

/// Dump one JS key as a structured representation.
impl Dump for js::Key {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            js::Key::Name(name) => {
                dumper.object("js::Key::Name").value(name).end();
            }
            js::Key::Private(name) => {
                dumper.object("js::Key::Private").value(name).end();
            }
            js::Key::Expression(_) => {
                dumper.object("js::Key::Expression").end();
            }
            js::Key::NamedExpression { name, key: _ } => {
                dumper
                    .object("js::Key::NamedExpression")
                    .field("name", name)
                    .end();
            }
        }
    }
}

/// Dump one JS function signature.
impl Dump for js::FunctionSignature {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("js::FunctionSignature")
            .field("is_abstract", &self.is_abstract)
            .field("is_override", &self.is_override)
            .field("asynchrony", &self.asynchrony)
            .field("cardinality", &self.cardinality)
            .field_optional("mode", &self.mode)
            .field("kind", &self.kind)
            .end();
    }
}

impl_dump_display! {
    js::AccessorKind,
    js::AnnotationPosition,
    js::AssignOperator,
    js::Asynchrony,
    js::BinaryOperator,
    js::BindingKind,
    js::BindingOperator,
    js::BindingAnchor,
    js::DeclarationAbstraction,
    js::DeclarationKind,
    js::DependencyKind,
    js::DependencyMode,
    js::FunctionCardinality,
    js::FunctionKind,
    js::FunctionMode,
    js::Mutability,
    js::PostfixPosition,
    js::PrimitiveType,
    js::UnaryOperator,
    js::VarianceModifier,
    js::Visibility,
}

/// Dump one JS scalar literal.
impl Dump for js::ScalarLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            js::ScalarLiteral::Null => {
                dumper.object("js::ScalarLiteral::Null").end();
            }
            js::ScalarLiteral::Undefined => {
                dumper.object("js::ScalarLiteral::Undefined").end();
            }
            js::ScalarLiteral::Boolean(value) => {
                dumper
                    .object("js::ScalarLiteral::Boolean")
                    .value(value)
                    .end();
            }
            js::ScalarLiteral::Number(value) => {
                dumper
                    .object("js::ScalarLiteral::Number")
                    .value(value)
                    .end();
            }
            js::ScalarLiteral::Bigint(value) => {
                dumper
                    .object("js::ScalarLiteral::Bigint")
                    .value(value)
                    .end();
            }
            js::ScalarLiteral::String(value) => {
                dumper
                    .object("js::ScalarLiteral::String")
                    .value(value)
                    .end();
            }
            js::ScalarLiteral::RegexString { content, flags } => {
                dumper
                    .object("js::ScalarLiteral::RegexString")
                    .field("content", content)
                    .field_optional("flags", flags)
                    .end();
            }
        }
    }
}

/// Dump one JS template literal.
impl Dump for js::TemplateLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            js::TemplateLiteral::String { template } => {
                dumper
                    .object("js::TemplateLiteral::String")
                    .field("template", template)
                    .end();
            }
            js::TemplateLiteral::TaggedString { tag, template } => {
                dumper
                    .object("js::TemplateLiteral::TaggedString")
                    .field("tag", tag)
                    .field("template", template)
                    .end();
            }
            js::TemplateLiteral::InterpolatedString {
                template: _,
                expressions: _,
            } => {
                dumper
                    .object("js::TemplateLiteral::InterpolatedString")
                    .end();
            }
            js::TemplateLiteral::TaggedInterpolatedString {
                tag,
                template: _,
                expressions: _,
            } => {
                dumper
                    .object("js::TemplateLiteral::TaggedInterpolatedString")
                    .field("tag", tag)
                    .end();
            }
        }
    }
}

/// Dump one JS type literal.
impl Dump for js::TypeLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            js::TypeLiteral::Never => {
                dumper.object("js::TypeLiteral::Never").end();
            }
            js::TypeLiteral::Any => {
                dumper.object("js::TypeLiteral::Any").end();
            }
            js::TypeLiteral::Undefined => {
                dumper.object("js::TypeLiteral::Undefined").end();
            }
            js::TypeLiteral::Unknown => {
                dumper.object("js::TypeLiteral::Unknown").end();
            }
            js::TypeLiteral::Object => {
                dumper.object("js::TypeLiteral::Object").end();
            }
            js::TypeLiteral::Void => {
                dumper.object("js::TypeLiteral::Void").end();
            }
            js::TypeLiteral::Null => {
                dumper.object("js::TypeLiteral::Null").end();
            }
            js::TypeLiteral::Primitive(primitive) => {
                dumper
                    .object("js::TypeLiteral::Primitive")
                    .value(primitive)
                    .end();
            }
            js::TypeLiteral::ScalarLiteral(scalar_literal) => {
                dumper
                    .object("js::TypeLiteral::ScalarLiteral")
                    .value(scalar_literal)
                    .end();
            }
        }
    }
}

/// Dump one JS binding modifier.
impl Dump for js::BindingModifier {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("js::BindingModifier")
            .field_optional("kind", &self.kind)
            .field_optional("variance", &self.variance)
            .field_optional("anchor", &self.anchor)
            .field_optional("mutability", &self.mutability)
            .field_optional("visibility", &self.visibility)
            .field_optional("operator", &self.operator)
            .field_optional("accessor", &self.accessor)
            .end();
    }
}

/// Dump one JS declaration descriptor.
impl Dump for js::DeclarationDescriptor {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("js::DeclarationDescriptor")
            .field("kind", &self.kind)
            .field("abstraction", &self.abstraction)
            .field("anchor", &self.anchor)
            .field_optional("name", &self.name)
            .field_optional("export", &self.export)
            .end();
    }
}

impl<'a> js::NodeVisitor for Dumper<'a> {
    #[inline]
    fn options(&self) -> &js::NodeVisitorOptions {
        &self.visitor_options
    }

    fn visit_any(&mut self, tree: &js::NodeTree, _ty: js::NodeType, id: u32) {
        let annotations = tree.get_annotations(id);
        for annotation_id in annotations {
            let annotation = tree.get(annotation_id);
            self.visit_annotation(tree, annotation_id, annotation);
        }
    }

    fn visit_block(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Block>,
        block: &js::Block,
    ) {
        let statement_count = block.statements.len() as u32;
        self.node("js::Block", id.id)
            .field("statement_count", &statement_count)
            .end();
        self.with_depth(|dumper| {
            js::walk_block(dumper, tree, id, block);
        });
    }

    fn visit_statement(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Statement>,
        statement: &js::Statement,
    ) {
        match statement {
            js::Statement::Import {
                kind,
                target,
                target_module,
                items: _,
                attributes: _,
            } => {
                self.node("js::Statement::Import", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .field_optional(
                        "target_module",
                        &target_module.map(|module| format!("{module:?}")),
                    )
                    .end();
            }
            js::Statement::Export {
                kind,
                target,
                target_module,
                items: _,
                attributes: _,
            } => {
                self.node("js::Statement::Export", id.id)
                    .field("kind", kind)
                    .field_optional("target", target)
                    .field_optional(
                        "target_module",
                        &target_module.map(|module| format!("{module:?}")),
                    )
                    .end();
            }
            js::Statement::ExportValue { value: _ } => {
                self.node("js::Statement::ExportValue", id.id).end();
            }
            js::Statement::Declaration { declaration: _ } => {
                self.node("js::Statement::Declaration", id.id).end();
            }
            js::Statement::Block { block: _ } => {
                self.node("js::Statement::Block", id.id).end();
            }
            js::Statement::Labelled { label, body: _ } => {
                self.node("js::Statement::Labelled", id.id)
                    .field("label", label)
                    .end();
            }
            js::Statement::Let {
                descriptor,
                mutability,
                declarators: _,
            } => {
                self.node("js::Statement::Let", id.id)
                    .field("descriptor", descriptor)
                    .field("mutability", mutability)
                    .end();
            }
            js::Statement::Var {
                descriptor,
                declarators: _,
            } => {
                self.node("js::Statement::Var", id.id)
                    .field("descriptor", descriptor)
                    .end();
            }
            js::Statement::Using {
                asynchrony,
                descriptor,
                declarators: _,
            } => {
                self.node("js::Statement::Using", id.id)
                    .field("asynchrony", asynchrony)
                    .field("descriptor", descriptor)
                    .end();
            }
            js::Statement::Assign {
                operator,
                left: _,
                right: _,
            } => {
                self.node("js::Statement::Assign", id.id)
                    .field("operator", operator)
                    .end();
            }
            js::Statement::Expression { expression: _ } => {
                self.node("js::Statement::Expression", id.id).end();
            }
            js::Statement::If {
                condition: _,
                then_block: _,
                else_block: _,
            } => {
                self.node("js::Statement::If", id.id).end();
            }
            js::Statement::While {
                condition: _,
                body: _,
            } => {
                self.node("js::Statement::While", id.id).end();
            }
            js::Statement::DoWhile {
                body: _,
                condition: _,
            } => {
                self.node("js::Statement::DoWhile", id.id).end();
            }
            js::Statement::For {
                initialization: _,
                increment: _,
                condition: _,
                body: _,
            } => {
                self.node("js::Statement::For", id.id).end();
            }
            js::Statement::ForIn { pattern: _, .. } => {
                self.node("js::Statement::ForIn", id.id).end();
            }
            js::Statement::ForOf {
                asynchrony,
                pattern: _,
                iterator: _,
                body: _,
                declaration_kind: _,
            } => {
                self.node("js::Statement::ForOf", id.id)
                    .field("asynchrony", asynchrony)
                    .end();
            }
            js::Statement::Switch { value: _, cases: _ } => {
                self.node("js::Statement::Switch", id.id).end();
            }
            js::Statement::Try {
                try_block: _,
                catch_clause: _,
                finally_block: _,
            } => {
                self.node("js::Statement::Try", id.id).end();
            }
            js::Statement::Throw { value: _ } => {
                self.node("js::Statement::Throw", id.id).end();
            }
            js::Statement::Continue { label } => {
                self.node("js::Statement::Continue", id.id)
                    .field_optional("label", label)
                    .end();
            }
            js::Statement::Break { label } => {
                self.node("js::Statement::Break", id.id)
                    .field_optional("label", label)
                    .end();
            }
            js::Statement::Return { value: _ } => {
                self.node("js::Statement::Return", id.id).end();
            }
            js::Statement::Debugger => {
                self.node("js::Statement::Debugger", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_statement(dumper, tree, id, statement);
        });
    }

    fn visit_expression(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Expression>,
        expression: &js::Expression,
    ) {
        match expression {
            js::Expression::Declaration { declaration: _ } => {
                self.node("js::Expression::Declaration", id.id).end();
            }
            js::Expression::ArrowFunction { signature, body: _ } => {
                self.node("js::Expression::ArrowFunction", id.id)
                    .field("signature", signature)
                    .end();
            }
            js::Expression::Path {
                path,
                generic_arguments: _,
            } => {
                self.node("js::Expression::Path", id.id)
                    .field("path", path)
                    .end();
            }
            js::Expression::ImportMeta => {
                self.node("js::Expression::ImportMeta", id.id).end();
            }
            js::Expression::This => {
                self.node("js::Expression::This", id.id).end();
            }
            js::Expression::Super => {
                self.node("js::Expression::Super", id.id).end();
            }
            js::Expression::NewTarget => {
                self.node("js::Expression::NewTarget", id.id).end();
            }
            js::Expression::PrivateIdentifier { name } => {
                self.node("js::Expression::PrivateIdentifier", id.id)
                    .field("name", name)
                    .end();
            }
            js::Expression::ScalarLiteral { value } => {
                self.node("js::Expression::ScalarLiteral", id.id)
                    .value(value)
                    .end();
            }
            js::Expression::TemplateLiteral { value } => {
                self.node("js::Expression::TemplateLiteral", id.id)
                    .value(value)
                    .end();
            }
            js::Expression::ArrayLiteral { elements: _ } => {
                self.node("js::Expression::ArrayLiteral", id.id).end();
            }
            js::Expression::SequenceExpression { expressions } => {
                self.node("js::Expression::SequenceExpression", id.id)
                    .field("count", &(expressions.len() as u32))
                    .end();
            }
            js::Expression::ObjectLiteral { properties: _ } => {
                self.node("js::Expression::ObjectLiteral", id.id).end();
            }
            js::Expression::Parenthesized { .. } => {
                self.node("js::Expression::Parenthesized", id.id).end();
            }
            js::Expression::As {
                expression: _,
                target_type: _,
            } => {
                self.node("js::Expression::As", id.id).end();
            }
            js::Expression::Satisfies {
                expression: _,
                target_type: _,
            } => {
                self.node("js::Expression::Satisfies", id.id).end();
            }
            js::Expression::InstanceOf {
                value: _,
                target: _,
            } => {
                self.node("js::Expression::InstanceOf", id.id).end();
            }
            js::Expression::Await { value: _ } => {
                self.node("js::Expression::Await", id.id).end();
            }
            js::Expression::Yield {
                is_delegate,
                value: _,
            } => {
                self.node("js::Expression::Yield", id.id)
                    .field("is_delegate", is_delegate)
                    .end();
            }
            js::Expression::Unary { operator, right: _ } => {
                self.node("js::Expression::Unary", id.id)
                    .field("operator", operator)
                    .end();
            }
            js::Expression::Binary {
                operator,
                left: _,
                right: _,
            } => {
                self.node("js::Expression::Binary", id.id)
                    .field("operator", operator)
                    .end();
            }
            js::Expression::Assign { left: _, right: _ } => {
                self.node("js::Expression::Assign", id.id).end();
            }
            js::Expression::AssignBinary {
                operator,
                left: _,
                right: _,
            } => {
                self.node("js::Expression::AssignBinary", id.id)
                    .field("operator", operator)
                    .end();
            }
            js::Expression::Maybe { position, left: _ } => {
                self.node("js::Expression::Maybe", id.id)
                    .field("position", position)
                    .end();
            }
            js::Expression::Must { position, left: _ } => {
                self.node("js::Expression::Must", id.id)
                    .field("position", position)
                    .end();
            }
            js::Expression::Member { left: _, name } => {
                self.node("js::Expression::Member", id.id)
                    .field("name", name)
                    .end();
            }
            js::Expression::PrivateMember { left: _, name } => {
                self.node("js::Expression::PrivateMember", id.id)
                    .field("name", name)
                    .end();
            }
            js::Expression::Index {
                position,
                right: _,
                left: _,
            } => {
                self.node("js::Expression::Index", id.id)
                    .field("position", position)
                    .end();
            }
            js::Expression::Instantiation {
                left: _,
                generic_arguments: _,
            } => {
                self.node("js::Expression::Instantiation", id.id).end();
            }
            js::Expression::Call {
                position,
                left: _,
                generic_arguments: _,
                arguments: _,
            } => {
                self.node("js::Expression::Call", id.id)
                    .field("position", position)
                    .end();
            }
            js::Expression::ImportCall {
                target: _,
                target_module,
                arguments: _,
            } => {
                self.node("js::Expression::ImportCall", id.id)
                    .field(
                        "target_module",
                        &target_module.map(|module| format!("{module:?}")),
                    )
                    .end();
            }
            js::Expression::New {
                left: _,
                generic_arguments: _,
                arguments: _,
            } => {
                self.node("js::Expression::New", id.id).end();
            }
            js::Expression::IfTernary {
                condition: _,
                then_expression: _,
                else_expression: _,
            } => {
                self.node("js::Expression::IfTernary", id.id).end();
            }
            js::Expression::Missing => {
                self.node("js::Expression::Missing", id.id).end();
            }
            js::Expression::Stub => {
                self.node("js::Expression::Stub", id.id).end();
            }
            js::Expression::Error => {
                self.node("js::Expression::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_expression(dumper, tree, id, expression);
        });
    }

    fn visit_switch_case(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::SwitchCase>,
        switch_case: &js::SwitchCase,
    ) {
        self.node("js::SwitchCase", id.id).end();
        self.with_depth(|dumper| {
            js::walk_switch_case(dumper, tree, id, switch_case);
        });
    }

    fn visit_catch_clause(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::CatchClause>,
        catch_clause: &js::CatchClause,
    ) {
        self.node("js::CatchClause", id.id).end();
        self.with_depth(|dumper| {
            js::walk_catch_clause(dumper, tree, id, catch_clause);
        });
    }

    fn visit_declaration(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Declaration>,
        declaration: &js::Declaration,
    ) {
        match declaration {
            js::Declaration::Global(js::GlobalDeclaration {
                descriptor,
                statements: _,
            }) => {
                self.node("js::Declaration::Global", id.id)
                    .field("descriptor", descriptor)
                    .end();
            }
            js::Declaration::Namespace(js::NamespaceDeclaration {
                descriptor,
                statements: _,
            }) => {
                self.node("js::Declaration::Namespace", id.id)
                    .field("descriptor", descriptor)
                    .end();
            }
            js::Declaration::Type(js::TypeDeclaration {
                descriptor,
                generic_parameters: _,
                value: _,
            }) => {
                self.node("js::Declaration::Type", id.id)
                    .field("descriptor", descriptor)
                    .end();
            }
            js::Declaration::Class(js::ClassDeclaration {
                descriptor,
                generic_parameters: _,
                extends_expression: _,
                extends_generic_arguments: _,
                implements_types: _,
                members: _,
            }) => {
                self.node("js::Declaration::Class", id.id)
                    .field("descriptor", descriptor)
                    .end();
            }
            js::Declaration::Interface(js::InterfaceDeclaration {
                descriptor,
                generic_parameters: _,
                extends_types: _,
                members: _,
            }) => {
                self.node("js::Declaration::Interface", id.id)
                    .field("descriptor", descriptor)
                    .end();
            }
            js::Declaration::Enum(js::EnumDeclaration {
                descriptor,
                fields: _,
            }) => {
                self.node("js::Declaration::Enum", id.id)
                    .field("descriptor", descriptor)
                    .end();
            }
            js::Declaration::Function(js::FunctionDeclaration {
                descriptor,
                signature,
                body: _,
            }) => {
                self.node("js::Declaration::Function", id.id)
                    .field("descriptor", descriptor)
                    .field("signature", signature)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_declaration(dumper, tree, id, declaration);
        });
    }

    fn visit_property(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Property>,
        property: &js::Property,
    ) {
        match property {
            js::Property::Field {
                modifiers,
                key,
                value: _,
                is_shorthand: _,
            } => {
                self.node("js::Property::Field", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("key", key)
                    .end();
            }
            js::Property::Method {
                modifiers,
                key,
                signature,
                body: _,
            } => {
                self.node("js::Property::Method", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("key", key)
                    .field("signature", signature)
                    .end();
            }
            js::Property::Spread {
                modifiers,
                value: _,
            } => {
                self.node("js::Property::Spread", id.id)
                    .field_optional("modifiers", modifiers)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_property(dumper, tree, id, property);
        });
    }

    fn visit_member(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Member>,
        member: &js::Member,
    ) {
        match member {
            js::Member::Field {
                modifiers,
                key,
                value: _,
                default: _,
            } => {
                self.node("js::Member::Field", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("key", key)
                    .end();
            }
            js::Member::Method {
                modifiers,
                key,
                signature,
                body: _,
            } => {
                self.node("js::Member::Method", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("key", key)
                    .field("signature", signature)
                    .end();
            }
            js::Member::StaticBlock { body: _ } => {
                self.node("js::Member::StaticBlock", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_member(dumper, tree, id, member);
        });
    }

    fn visit_enum_field(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::EnumField>,
        field: &js::EnumField,
    ) {
        let has_value = field.value.is_some();
        self.node("js::EnumField", id.id)
            .field("name", &field.name)
            .field("has_value", &has_value)
            .end();
        self.with_depth(|dumper| {
            js::walk_enum_field(dumper, tree, id, field);
        });
    }

    fn visit_dependency_item(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::DependencyItem>,
        dependency_item: &js::DependencyItem,
    ) {
        self.node("js::DependencyItem", id.id)
            .field("mode", &dependency_item.mode)
            .field_optional("kind", &dependency_item.kind)
            .field_optional("name", &dependency_item.name)
            .field_optional("alias", &dependency_item.alias)
            .end();
        self.with_depth(|dumper| {
            js::walk_dependency_item(dumper, tree, id, dependency_item);
        });
    }

    fn visit_parameter(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Parameter>,
        parameter: &js::Parameter,
    ) {
        match parameter {
            js::Parameter::Named {
                modifiers,
                name,
                ty: _,
                default: _,
            } => {
                self.node("js::Parameter::Named", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
            js::Parameter::Pattern {
                modifiers,
                pattern: _,
                ty: _,
                default: _,
            } => {
                self.node("js::Parameter::Pattern", id.id)
                    .field_optional("modifiers", modifiers)
                    .end();
            }
            js::Parameter::VariadicNamed {
                modifiers,
                name,
                ty: _,
            } => {
                self.node("js::Parameter::VariadicNamed", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
            js::Parameter::VariadicPattern {
                modifiers,
                pattern: _,
                ty: _,
            } => {
                self.node("js::Parameter::VariadicPattern", id.id)
                    .field_optional("modifiers", modifiers)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_parameter(dumper, tree, id, parameter);
        });
    }

    fn visit_argument(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Argument>,
        argument: &js::Argument,
    ) {
        match argument {
            js::Argument::Positional { value: _ } => {
                self.node("js::Argument::Positional", id.id).end();
            }
            js::Argument::Spread { value: _ } => {
                self.node("js::Argument::Spread", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_argument(dumper, tree, id, argument);
        });
    }

    fn visit_array_element(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::ArrayElement>,
        array_element: &js::ArrayElement,
    ) {
        match array_element {
            js::ArrayElement::Expression { value: _ } => {
                self.node("js::ArrayElement::Expression", id.id).end();
            }
            js::ArrayElement::Spread { value: _ } => {
                self.node("js::ArrayElement::Spread", id.id).end();
            }
            js::ArrayElement::Elision => {
                self.node("js::ArrayElement::Elision", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_array_element(dumper, tree, id, array_element);
        });
    }

    fn visit_pattern(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Pattern>,
        pattern: &js::Pattern,
    ) {
        match pattern {
            js::Pattern::Binding { mutability, name } => {
                self.node("js::Pattern::Binding", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }
            js::Pattern::Array { fields: _ } => {
                self.node("js::Pattern::Array", id.id).end();
            }
            js::Pattern::Object { fields: _ } => {
                self.node("js::Pattern::Object", id.id).end();
            }
            js::Pattern::Hole => {
                self.node("js::Pattern::Hole", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_pattern(dumper, tree, id, pattern);
        });
    }

    fn visit_pattern_field(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::PatternField>,
        field: &js::PatternField,
    ) {
        match field {
            js::PatternField::Named {
                mutability,
                name,
                pattern: _,
                default: _,
            } => {
                self.node("js::PatternField::Named", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }
            js::PatternField::Computed {
                mutability,
                key: _,
                pattern: _,
                default: _,
            } => {
                self.node("js::PatternField::Computed", id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            js::PatternField::Alias {
                mutability,
                name,
                alias,
                default: _,
            } => {
                self.node("js::PatternField::Alias", id.id)
                    .field("name", name)
                    .field("alias", alias)
                    .field_optional("mutability", mutability)
                    .end();
            }
            js::PatternField::Positional {
                pattern: _,
                default: _,
            } => {
                self.node("js::PatternField::Positional", id.id).end();
            }
            js::PatternField::Spread {
                mutability,
                pattern: _,
            } => {
                self.node("js::PatternField::Spread", id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            js::PatternField::Elision => {
                self.node("js::PatternField::Elision", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_pattern_field(dumper, tree, id, field);
        });
    }

    fn visit_type_expression(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::TypeExpression>,
        type_expression: &js::TypeExpression,
    ) {
        match type_expression {
            js::TypeExpression::Scalar(literal) => {
                self.node("js::TypeExpression::Scalar", id.id)
                    .value(literal)
                    .end();
            }
            js::TypeExpression::This => {
                self.node("js::TypeExpression::This", id.id).end();
            }
            js::TypeExpression::Path {
                path,
                generic_arguments: _,
            } => {
                self.node("js::TypeExpression::Path", id.id)
                    .field("path", path)
                    .end();
            }
            js::TypeExpression::Readonly { target_type: _ } => {
                self.node("js::TypeExpression::Readonly", id.id).end();
            }
            js::TypeExpression::KeyOf { target_type: _ } => {
                self.node("js::TypeExpression::KeyOf", id.id).end();
            }
            js::TypeExpression::Must { target_type: _ } => {
                self.node("js::TypeExpression::Must", id.id).end();
            }
            js::TypeExpression::AsComptime { target_type: _ } => {
                self.node("js::TypeExpression::AsComptime", id.id).end();
            }
            js::TypeExpression::Not { target_type: _ } => {
                self.node("js::TypeExpression::Not", id.id).end();
            }
            js::TypeExpression::In { left: _, right: _ } => {
                self.node("js::TypeExpression::In", id.id).end();
            }
            js::TypeExpression::Extends { left: _, right: _ } => {
                self.node("js::TypeExpression::Extends", id.id).end();
            }
            js::TypeExpression::Implements { left: _, right: _ } => {
                self.node("js::TypeExpression::Implements", id.id).end();
            }
            js::TypeExpression::Conditional {
                left: _,
                right: _,
                then_type: _,
                else_type: _,
            } => {
                self.node("js::TypeExpression::Conditional", id.id).end();
            }
            js::TypeExpression::Mapped {
                parameter: _,
                modifiers: _,
                value: _,
            } => {
                self.node("js::TypeExpression::Mapped", id.id).end();
            }
            js::TypeExpression::Index { left: _, index: _ } => {
                self.node("js::TypeExpression::Index", id.id).end();
            }
            js::TypeExpression::TemplateLiteral(_) => {
                self.node("js::TypeExpression::TemplateLiteral", id.id)
                    .end();
            }
            js::TypeExpression::Import {
                target: _,
                qualifier: _,
                generic_arguments: _,
            } => {
                self.node("js::TypeExpression::Import", id.id).end();
            }
            js::TypeExpression::Infer {
                name: _,
                constraint: _,
            } => {
                self.node("js::TypeExpression::Infer", id.id).end();
            }
            js::TypeExpression::Predicate {
                asserts: _,
                subject: _,
                target: _,
            } => {
                self.node("js::TypeExpression::Predicate", id.id).end();
            }
            js::TypeExpression::Array { .. } => {
                self.node("js::TypeExpression::Array", id.id).end();
            }
            js::TypeExpression::Tuple { elements: _ } => {
                self.node("js::TypeExpression::Tuple", id.id).end();
            }
            js::TypeExpression::Object { members: _ } => {
                self.node("js::TypeExpression::Object", id.id).end();
            }
            js::TypeExpression::Union { elements: _ } => {
                self.node("js::TypeExpression::Union", id.id).end();
            }
            js::TypeExpression::Intersection { elements: _ } => {
                self.node("js::TypeExpression::Intersection", id.id).end();
            }
            js::TypeExpression::Function { signature } => {
                self.node("js::TypeExpression::Function", id.id)
                    .field("signature", signature)
                    .end();
            }

            js::TypeExpression::Error => {
                self.node("js::TypeExpression::Error", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_type_expression(dumper, tree, id, type_expression);
        });
    }

    fn visit_annotation(
        &mut self,
        tree: &js::NodeTree,
        id: js::LocalNodeId<js::Annotation>,
        annotation: &js::Annotation,
    ) {
        match annotation {
            js::Annotation::Comment { position, string } => {
                let string = truncate_string(self.strings.get(*string), 40, " ");
                self.node("js::Annotation::Comment", id.id)
                    .field("position", position)
                    .field("string", &string.as_ref())
                    .end();
            }
        }
        self.with_depth(|dumper| {
            js::walk_annotation(dumper, tree, id, annotation);
        });
    }
}

fn truncate_string<'a>(string: &'a str, max_len: usize, newline_replacement: &str) -> Cow<'a, str> {
    if string.len() > max_len || string.contains('\n') {
        let truncated = if string.len() > max_len {
            &string[..max_len]
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
