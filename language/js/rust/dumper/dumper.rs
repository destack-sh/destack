#![allow(clippy::match_like_matches_macro)]

use crate::*;
use dyst_container::SmallVec;
use dyst_source::{StringId, StringPool};
use dyst_tree::{Color, impl_dump_display, rebuild_tree_output};
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
    DeclarationKind,
    DependencyKind,
    ExportType,
    Mutability,
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
                source,
                items,
                arguments,
            } => {
                let item_count = items.len() as u32;
                let argument_count = arguments.as_ref().map(|args| args.len() as u32);
                self.node("Statement::Import", id.id)
                    .field("source", source)
                    .field("item_count", &item_count)
                    .field_optional("argument_count", &argument_count)
                    .end();
            }
            Statement::Export { items } => {
                let item_count = items.len() as u32;
                self.node("Statement::Export", id.id)
                    .field("item_count", &item_count)
                    .end();
            }
            Statement::Block { label, statements } => {
                let statement_count = statements.len() as u32;
                self.node("Statement::Block", id.id)
                    .field_optional("label", label)
                    .field("statement_count", &statement_count)
                    .end();
            }
            Statement::Let {
                mutability,
                ty,
                value,
            } => {
                let has_type = ty.is_some();
                let has_value = value.is_some();
                self.node("Statement::Let", id.id)
                    .field("mutability", mutability)
                    .field("has_type", &has_type)
                    .field("has_value", &has_value)
                    .end();
            }
            Statement::LetType {
                name,
                static_parameters,
                ..
            } => {
                let static_parameter_count =
                    static_parameters.as_ref().map(|params| params.len() as u32);
                self.node("Statement::LetType", id.id)
                    .field("name", name)
                    .field_optional("static_parameter_count", &static_parameter_count)
                    .end();
            }
            Statement::Assign { operator, .. } => {
                self.node("Statement::Assign", id.id)
                    .field("operator", operator)
                    .end();
            }
            Statement::Expression { .. } => {
                self.node("Statement::Expression", id.id).end();
            }
            Statement::If { else_block, .. } => {
                let has_else = else_block.is_some();
                self.node("Statement::If", id.id)
                    .field("has_else", &has_else)
                    .end();
            }
            Statement::While { .. } => {
                self.node("Statement::While", id.id).end();
            }
            Statement::For {
                initialization,
                increment,
                ..
            } => {
                let has_initialization = initialization.is_some();
                let has_increment = increment.is_some();
                self.node("Statement::For", id.id)
                    .field("has_initialization", &has_initialization)
                    .field("has_increment", &has_increment)
                    .end();
            }
            Statement::ForIn { name, .. } => {
                self.node("Statement::ForIn", id.id)
                    .field("name", name)
                    .end();
            }
            Statement::ForOf { .. } => {
                self.node("Statement::ForOf", id.id).end();
            }
            Statement::Try {
                catch_pattern,
                finally_block,
                ..
            } => {
                let has_catch_pattern = catch_pattern.is_some();
                let has_finally = finally_block.is_some();
                self.node("Statement::Try", id.id)
                    .field("has_catch_pattern", &has_catch_pattern)
                    .field("has_finally", &has_finally)
                    .end();
            }
            Statement::Await { .. } => {
                self.node("Statement::Await", id.id).end();
            }
            Statement::Yield { .. } => {
                self.node("Statement::Yield", id.id).end();
            }
            Statement::Throw { .. } => {
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
            Statement::Return { value } => {
                let has_value = value.is_some();
                self.node("Statement::Return", id.id)
                    .field("has_value", &has_value)
                    .end();
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
            Expression::Definition { .. } => {
                self.node("Expression::Definition", id.id).end();
            }
            Expression::ArrowFunction {
                dynamic_parameters,
                return_type,
                ..
            } => {
                let parameter_count = dynamic_parameters.len() as u32;
                let has_return_type = return_type.is_some();
                self.node("Expression::ArrowFunction", id.id)
                    .field("parameter_count", &parameter_count)
                    .field("has_return_type", &has_return_type)
                    .end();
            }
            Expression::Path {
                path,
                static_arguments,
            } => {
                let static_argument_count = static_arguments.as_ref().map(|args| args.len() as u32);
                self.node("Expression::Path", id.id)
                    .field("path", path)
                    .field_optional("static_argument_count", &static_argument_count)
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
            Expression::ArrayLiteral { elements } => {
                let element_count = elements.len() as u32;
                self.node("Expression::ArrayLiteral", id.id)
                    .field("element_count", &element_count)
                    .end();
            }
            Expression::ObjectLiteral { fields } => {
                let field_count = fields.len() as u32;
                self.node("Expression::ObjectLiteral", id.id)
                    .field("field_count", &field_count)
                    .end();
            }
            Expression::Parenthesized { .. } => {
                self.node("Expression::Parenthesized", id.id).end();
            }
            Expression::TypeUnary { operator, .. } => {
                self.node("Expression::TypeUnary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::TypeBinary { operator, .. } => {
                self.node("Expression::TypeBinary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Unary { operator, .. } => {
                self.node("Expression::Unary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Binary { operator, .. } => {
                self.node("Expression::Binary", id.id)
                    .field("operator", operator)
                    .end();
            }
            Expression::Member {
                left: _,
                path,
                kind,
                static_arguments: _,
            } => {
                self.node("Expression::Member", id.id)
                    .field("path", path)
                    .field("kind", kind)
                    .end();
            }
            Expression::Index {
                kind,
                index: _,
                left: _,
            } => {
                self.node("Expression::Index", id.id)
                    .field("kind", kind)
                    .end();
            }
            Expression::Call {
                kind,
                left: _,
                dynamic_arguments: _,
            } => {
                self.node("Expression::Call", id.id)
                    .field("kind", kind)
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
        self.node("Field", id.id).field("name", &field.name).end();
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
            Argument::Positional { modifiers, .. } => {
                self.node("Argument::Positional", id.id)
                    .field_optional("modifiers", modifiers)
                    .end();
            }
            Argument::Spread {
                modifiers, name, ..
            } => {
                self.node("Argument::Spread", id.id)
                    .field_optional("modifiers", modifiers)
                    .field_optional("name", name)
                    .end();
            }
            Argument::Dynamic {
                modifiers, name, ..
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

    fn visit_pattern(&mut self, tree: &NodeTree, id: NodeId<Pattern>, pattern: &Pattern) {
        match pattern {
            Pattern::Binding { mutability, name } => {
                self.node("Pattern::Binding", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }
            Pattern::Array { elements } => {
                let element_count = elements.len() as u32;
                self.node("Pattern::Array", id.id)
                    .field("element_count", &element_count)
                    .end();
            }
            Pattern::Object { fields } => {
                let field_count = fields.len() as u32;
                self.node("Pattern::Object", id.id)
                    .field("field_count", &field_count)
                    .end();
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
                mutability, name, ..
            } => {
                self.node("PatternField::Named", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .end();
            }
            PatternField::Alias {
                mutability,
                name,
                alias,
                ..
            } => {
                self.node("PatternField::Alias", id.id)
                    .field_optional("mutability", mutability)
                    .field("name", name)
                    .field("alias", alias)
                    .end();
            }
            PatternField::Positional { .. } => {
                self.node("PatternField::Positional", id.id).end();
            }
        }
        self.with_depth(|dumper| {
            walk_pattern_field(dumper, tree, id, field);
        });
    }

    fn visit_type(&mut self, tree: &NodeTree, id: NodeId<Type>, ty: &Type) {
        self.node("Type", id.id).end();
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
                let snippet = truncate_string(self.strings.get(*string), 40, " ");
                let snippet_ref = snippet.as_ref();
                self.node("Annotation::Doc", id.id)
                    .field("string", &snippet_ref)
                    .end();
            }
            Annotation::Comment { string } => {
                let snippet = truncate_string(self.strings.get(*string), 40, " ");
                let snippet_ref = snippet.as_ref();
                self.node("Annotation::Comment", id.id)
                    .field("string", &snippet_ref)
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
