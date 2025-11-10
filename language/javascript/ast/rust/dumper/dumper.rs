#![allow(clippy::match_like_matches_macro)]

use crate::*;
use dyst_source::{Color, SmallVec, StringId, StringPool, impl_dump_display, rebuild_tree_output};
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

/// A Dumper for dumping JS nodes.
#[derive(Debug)]
pub struct Dumper<'a> {
    /// The string pool.
    pub strings: &'a StringPool,
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
    pub fn new(strings: &'a StringPool, tree: &'a NodeTree, options: DumperOptions) -> Self {
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

    /// Write the path represented by a Path.
    #[inline]
    pub fn write_path(&mut self, path: &Path) {
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

/// Dump a SmallVec<T, N> as a slice.
impl<T: Dump, const N: usize> Dump for SmallVec<T, N> {
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
impl<T: Node + Clone + Dump> Dump for NodeId<T>
where
    NodeTree: NodeTreeImpl<T>,
{
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let node = dumper.tree.get(*self);
        node.dump(dumper);
    }
}

/// Dump a Path as a dotted string.
impl Dump for Path {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper.write_path(self);
    }
}

/// Dump a Name as a string selector.
impl Dump for Name {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            Name::Identifier(id) => {
                id.dump(dumper);
            }
            Name::String(id) => {
                dumper.write_char('[', Some(Color::White));
                id.dump(dumper);
                dumper.write_char(']', Some(Color::White));
            }
        }
    }
}

impl_dump_display! {
    AssignOperator,
    BinaryOperator,
    BindingKind,
    BindingOperator,
    BindingScope,
    PostfixPosition,
    DeclarationKind,
    DependencyKind,
    ExportType,
    Mutability,
    PrimitiveType,
    TypeBinaryOperator,
    TypeUnaryOperator,
    UnaryOperator,
    Visibility,
}

/// Dump a ScalarLiteral.
impl Dump for ScalarLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            ScalarLiteral::Boolean(value) => {
                dumper.object("ScalarLiteral::Boolean").value(value).end();
            }
            ScalarLiteral::Number(value) => {
                dumper.object("ScalarLiteral::Number").value(value).end();
            }
            ScalarLiteral::Bigint(value) => {
                dumper.object("ScalarLiteral::Bigint").value(value).end();
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

/// Dump a TemplateLiteral.
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
                template: _,
                expressions: _,
            } => {
                dumper.object("TemplateLiteral::InterpolatedString").end();
            }
            TemplateLiteral::TaggedInterpolatedString {
                tag,
                template: _,
                expressions: _,
            } => {
                dumper
                    .object("TemplateLiteral::TaggedInterpolatedString")
                    .field("tag", tag)
                    .end();
            }
        }
    }
}

/// Dump a TypeLiteral.
impl Dump for TypeLiteral {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        match self {
            TypeLiteral::Never => {
                dumper.object("TypeLiteral::Never").end();
            }
            TypeLiteral::Any => {
                dumper.object("TypeLiteral::Any").end();
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
            TypeLiteral::ScalarLiteral(scalar_literal) => {
                dumper
                    .object("TypeLiteral::ScalarLiteral")
                    .value(scalar_literal)
                    .end();
            }
        }
    }
}

/// Dump a BindingModifier.
impl Dump for BindingModifier {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        dumper
            .object("BindingModifier")
            .field_optional("kind", &self.kind)
            .field_optional("scope", &self.scope)
            .field_optional("mutability", &self.mutability)
            .field_optional("visibility", &self.visibility)
            .field_optional("operator", &self.operator)
            .end();
    }
}

/// Dump a DefinitionMeta.
impl Dump for DefinitionMeta {
    fn dump<'a>(&self, dumper: &mut Dumper<'a>) {
        let has_key = self.key.is_some();
        dumper
            .object("DefinitionMeta")
            .field("kind", &self.kind)
            .field("scope", &self.scope)
            .field_optional("name", &self.name)
            .field_optional("visibility", &self.visibility)
            .field_optional("export", &self.export)
            .field("has_key", &has_key)
            .end();
    }
}

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

    fn visit_block(&mut self, tree: &NodeTree, id: NodeId<Block>, block: &Block) {
        let statement_count = block.statements.len() as u32;
        self.node("Block", id.id)
            .field_optional("label", &block.label)
            .field("statement_count", &statement_count)
            .end();
        self.with_depth(|dumper| {
            walk_block(dumper, tree, id, block);
        });
    }

    fn visit_statement(&mut self, tree: &NodeTree, id: NodeId<Statement>, statement: &Statement) {
        match statement {
            Statement::Import {
                kind,
                target,
                alias,
                items: _,
                arguments: _,
            } => {
                self.node("Statement::Import", id.id)
                    .field("kind", kind)
                    .field("target", target)
                    .field_optional("alias", alias)
                    .end();
            }
            Statement::Export {
                mode,
                kind,
                target,
                alias,
                value: _,
                items: _,
            } => {
                self.node("Statement::Export", id.id)
                    .field("mode", mode)
                    .field("kind", kind)
                    .field_optional("target", target)
                    .field_optional("alias", alias)
                    .end();
            }
            Statement::Block { block: _ } => {
                self.node("Statement::Block", id.id).end();
            }
            Statement::Let {
                mutability,
                pattern: _,
                ty: _,
                value: _,
            } => {
                self.node("Statement::Let", id.id)
                    .field("mutability", mutability)
                    .end();
            }
            Statement::LetType {
                name,
                static_parameters: _,
                value: _,
            } => {
                self.node("Statement::LetType", id.id)
                    .field("name", name)
                    .end();
            }
            Statement::Assign {
                operator,
                left: _,
                right: _,
            } => {
                self.node("Statement::Assign", id.id)
                    .field("operator", operator)
                    .end();
            }
            Statement::Expression { expression: _ } => {
                self.node("Statement::Expression", id.id).end();
            }
            Statement::If {
                condition: _,
                then_block: _,
                else_block: _,
            } => {
                self.node("Statement::If", id.id).end();
            }
            Statement::While {
                condition: _,
                body: _,
            } => {
                self.node("Statement::While", id.id).end();
            }
            Statement::For {
                initialization: _,
                increment: _,
                condition: _,
                body: _,
            } => {
                self.node("Statement::For", id.id).end();
            }
            Statement::ForIn { name, .. } => {
                self.node("Statement::ForIn", id.id)
                    .field("name", name)
                    .end();
            }
            Statement::ForOf {
                pattern: _,
                iterator: _,
                body: _,
            } => {
                self.node("Statement::ForOf", id.id).end();
            }
            Statement::Try {
                catch_pattern: _,
                finally_block: _,
                try_block: _,
                catch_block: _,
            } => {
                self.node("Statement::Try", id.id).end();
            }
            Statement::Await { value: _ } => {
                self.node("Statement::Await", id.id).end();
            }
            Statement::Yield { value: _ } => {
                self.node("Statement::Yield", id.id).end();
            }
            Statement::Throw { value: _ } => {
                self.node("Statement::Throw", id.id).end();
            }
            Statement::Continue { label } => {
                self.node("Statement::Continue", id.id)
                    .field_optional("label", label)
                    .end();
            }
            Statement::Break { label } => {
                self.node("Statement::Break", id.id)
                    .field_optional("label", label)
                    .end();
            }
            Statement::Return { value: _ } => {
                self.node("Statement::Return", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_statement(dumper, tree, id, statement);
        });
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::Definition { definition: _ } => {
                self.node("Expression::Definition", id.id).end();
            }
            Expression::ArrowFunction {
                dynamic_parameters: _,
                return_type: _,
                body: _,
            } => {
                self.node("Expression::ArrowFunction", id.id).end();
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
                    .value(value)
                    .end();
            }
            Expression::TemplateLiteral { value } => {
                self.node("Expression::TemplateLiteral", id.id)
                    .value(value)
                    .end();
            }
            Expression::ArrayLiteral { elements: _ } => {
                self.node("Expression::ArrayLiteral", id.id).end();
            }
            Expression::ObjectLiteral { fields: _ } => {
                self.node("Expression::ObjectLiteral", id.id).end();
            }
            Expression::Parenthesized { .. } => {
                self.node("Expression::Parenthesized", id.id).end();
            }
            Expression::TypeUnary {
                operator,
                expression: _,
            } => {
                self.node("Expression::TypeUnary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::TypeBinary {
                operator,
                left: _,
                right: _,
            } => {
                self.node("Expression::TypeBinary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Unary {
                operator,
                expression: _,
            } => {
                self.node("Expression::Unary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Binary {
                operator,
                left: _,
                right: _,
            } => {
                self.node("Expression::Binary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Maybe { position, left: _ } => {
                self.node("Expression::Maybe", id.id)
                    .field("position", position)
                    .end();
            }
            Expression::Must { position, left: _ } => {
                self.node("Expression::Must", id.id)
                    .field("position", position)
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
            Expression::Index {
                position,
                index: _,
                left: _,
            } => {
                self.node("Expression::Index", id.id)
                    .field("position", position)
                    .end();
            }
            Expression::Call {
                position,
                left: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::Call", id.id)
                    .field("position", position)
                    .end();
            }
            Expression::ImportCall { source } => {
                self.node("Expression::ImportCall", id.id)
                    .field("source", source)
                    .end();
            }
            Expression::New {
                left,
                static_arguments: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::New", id.id)
                    .field("path", left)
                    .end();
            }
            Expression::IfTernary {
                condition: _,
                then_expression: _,
                else_expression: _,
            } => {
                self.node("Expression::IfTernary", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_expression(dumper, tree, id, expression);
        });
    }

    fn visit_switch_case(
        &mut self,
        tree: &NodeTree,
        id: NodeId<SwitchCase>,
        switch_case: &SwitchCase,
    ) {
        self.node("SwitchCase", id.id).end();
        self.with_depth(|dumper| {
            walk_switch_case(dumper, tree, id, switch_case);
        });
    }

    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Definition>,
        definition: &Definition,
    ) {
        match definition {
            Definition::Namespace {
                meta,
                definitions: _,
            } => {
                self.node("Definition::Namespace", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Class {
                meta,
                static_parameters: _,
                fields: _,
                definitions: _,
            } => {
                self.node("Definition::Class", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Interface {
                meta,
                static_parameters: _,
                fields: _,
                definitions: _,
            } => {
                self.node("Definition::Interface", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Enum { meta, fields: _ } => {
                self.node("Definition::Enum", id.id)
                    .field("meta", meta)
                    .end();
            }
            Definition::Function {
                meta,
                static_parameters: _,
                dynamic_parameters: _,
                return_type: _,
                body: _,
            } => {
                self.node("Definition::Function", id.id)
                    .field("meta", meta)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_definition(dumper, tree, id, definition);
        });
    }

    fn visit_field(&mut self, tree: &NodeTree, id: NodeId<Field>, field: &Field) {
        match field {
            Field::Named {
                modifiers,
                name,
                ty: _,
                default: _,
            } => {
                self.node("Field::Named", id.id)
                    .field_optional("modifiers", modifiers)
                    .field("name", name)
                    .end();
            }
            Field::Dynamic {
                modifiers,
                name,
                ty: _,
                key: _,
                default: _,
            } => {
                self.node("Field::Dynamic", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_field(dumper, tree, id, field);
        });
    }

    fn visit_enum_field(&mut self, tree: &NodeTree, id: NodeId<EnumField>, field: &EnumField) {
        let has_value = field.value.is_some();
        self.node("EnumField", id.id)
            .field("name", &field.name)
            .field("has_value", &has_value)
            .end();
        self.with_depth(|dumper| {
            walk_enum_field(dumper, tree, id, field);
        });
    }

    fn visit_dependency_item(
        &mut self,
        tree: &NodeTree,
        id: NodeId<DependencyItem>,
        dependency_item: &DependencyItem,
    ) {
        self.node("DependencyItem", id.id)
            .field_optional("kind", &dependency_item.kind)
            .field("name", &dependency_item.name)
            .field_optional("alias", &dependency_item.alias)
            .end();
        self.with_depth(|dumper| {
            walk_dependency_item(dumper, tree, id, dependency_item);
        });
    }

    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        match parameter {
            Parameter::Named {
                modifiers,
                name,
                ty: _,
                default: _,
            } => {
                self.node("Parameter::Named", id.id)
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

    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        match argument {
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

    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
        match pattern {
            Pattern::Binding { mutability, name } => {
                self.node("Pattern::Binding", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }
            Pattern::Array { elements: _ } => {
                self.node("Pattern::Array", id.id).end();
            }
            Pattern::Object { fields: _ } => {
                self.node("Pattern::Object", id.id).end();
            }
            Pattern::Rest { name } => {
                self.node("Pattern::Rest", id.id)
                    .field_optional("name", name)
                    .end();
            }
            Pattern::Hole => {
                self.node("Pattern::Hole", id.id).end();
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
        field: &PatternField,
    ) {
        match field {
            PatternField::Named {
                mutability,
                name,
                alias,
                default: _,
            } => {
                self.node("PatternField::Named", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .field_optional("alias", alias)
                    .end();
            }
            PatternField::Pattern {
                mutability,
                pattern: _,
                default: _,
            } => {
                self.node("PatternField::Pattern", id.id)
                    .field_optional("mutability", mutability)
                    .end();
            }
            PatternField::Positional { pattern: _ } => {
                self.node("PatternField::Positional", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_pattern_field(dumper, tree, id, field);
        });
    }

    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, ty: &Type) {
        match ty {
            Type::Scalar(literal) => {
                self.node("Type::Scalar", id.id).value(literal).end();
            }
            Type::Definition(_) => {
                self.node("Type::Definition", id.id).end();
            }
            Type::Unary { operator, right: _ } => {
                self.node("Type::Unary", id.id)
                    .field("operator", operator)
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
        }
        self.with_depth(|dumper| {
            walk_type(dumper, tree, id, ty);
        });
    }

    fn visit_annotation(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Annotation>,
        annotation: &Annotation,
    ) {
        match annotation {
            Annotation::Doc { string } => {
                let string = truncate_string(self.strings.get(*string), 40, " ");
                self.node("Annotation::Doc", id.id)
                    .field("string", &string.as_ref())
                    .end();
            }
            Annotation::Comment { string } => {
                let string = truncate_string(self.strings.get(*string), 40, " ");
                self.node("Annotation::Comment", id.id)
                    .field("string", &string.as_ref())
                    .end();
            }
        }
        self.with_depth(|dumper| {
            walk_annotation(dumper, tree, id, annotation);
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
