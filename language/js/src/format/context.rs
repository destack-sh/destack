use crate::{
    Annotation, Argument, ArrayElement, AssignPattern, AssignPatternField, Block, CatchClause,
    Declaration, Declarator, DependencyItem, EnumField, Expression, GenericParameter, LocalNodeId,
    LocalNodeIdAny, Member, Node, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern,
    PatternField, Property, Statement, SwitchCase, TupleElement, TypeExpression, TypeMember,
};
use destack_core::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::print::PrintOptions as FirPrintOptions;
use destack_source::{File, FileType, IndentStyle, LineEnding, NodeSpanType, Span};

/// The formatter type for one JS formatting pass.
pub type JsFormatter<'context, 'buffer> = Formatter<'buffer, JsFormatContext<'context>>;

/// One source span provider for JS formatting and printing.
pub trait JsSourceMap: std::fmt::Debug {
    /// Return one source span for one lowered node when one exists.
    fn source_span(&self, tree: &NodeTree, node_id: u32) -> Option<Span>;

    /// Return one source part span for one lowered node when one exists.
    fn source_part_span(
        &self,
        tree: &NodeTree,
        node_id: u32,
        span_type: NodeSpanType,
    ) -> Option<Span>;
}

/// One no-op source span provider.
#[derive(Debug, Default)]
pub struct NoopJsSourceMap;

impl JsSourceMap for NoopJsSourceMap {
    fn source_span(&self, tree: &NodeTree, node_id: u32) -> Option<Span> {
        let _ = tree;
        let _ = node_id;

        None
    }

    fn source_part_span(
        &self,
        tree: &NodeTree,
        node_id: u32,
        span_type: NodeSpanType,
    ) -> Option<Span> {
        let _ = tree;
        let _ = node_id;
        let _ = span_type;

        None
    }
}

/// One reusable no-op source span provider.
pub static NOOP_JS_SOURCE_MAP: NoopJsSourceMap = NoopJsSourceMap;

/// The formatting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FormatMode {
    /// Pretty.
    #[default]
    Pretty,
    /// Minimal.
    Minimal,
}

/// JS/TS format options.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct JsFormatOptions {
    /// The formatting mode.
    pub mode: FormatMode = FormatMode::Pretty,
    /// The output file type.
    pub file_type: FileType = FileType::TypeScript,
    /// The line ending to apply to printed output.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// The best effort maximum line length.
    pub line_width: u8 = 100,
}

impl JsFormatOptions {
    /// Create pretty-printing options.
    pub fn pretty() -> Self {
        Self {
            mode: FormatMode::Pretty,
            ..Self::default()
        }
    }

    /// Create minimal formatting options.
    pub fn minimal() -> Self {
        Self {
            mode: FormatMode::Minimal,
            indent_width: 0,
            ..Self::default()
        }
    }

    /// Set the file type.
    pub fn with_file_type(mut self, file_type: FileType) -> Self {
        self.file_type = file_type;
        self
    }

    /// Set the line ending.
    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
        self.line_ending = line_ending;
        self
    }

    /// Set the indent style.
    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
        self.indent_style = indent_style;
        self
    }

    /// Set the indent width.
    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
        self.indent_width = indent_width;
        self
    }

    /// Set the line width.
    pub fn with_line_width(mut self, line_width: u8) -> Self {
        self.line_width = line_width;
        self
    }

    /// Convert into FIR print options.
    #[inline]
    pub fn as_print_options(&self) -> FirPrintOptions {
        FirPrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
            trim_trailing_whitespace: true,
        }
    }

    /// Return whether type syntax should be emitted.
    #[inline]
    pub fn include_types(&self) -> bool {
        matches!(
            self.file_type,
            FileType::TypeScript | FileType::TypeScriptXml | FileType::TypeScriptDeclaration
        )
    }

    /// Return whether annotations should be emitted.
    #[inline]
    pub fn include_annotations(&self) -> bool {
        matches!(
            self.file_type,
            FileType::TypeScript | FileType::TypeScriptXml | FileType::TypeScriptDeclaration
        )
    }

    /// Return whether declarations only should be emitted.
    #[inline]
    pub fn is_declaration(&self) -> bool {
        matches!(self.file_type, FileType::TypeScriptDeclaration)
    }
}

/// One explicit JS print configuration.
pub type PrintOptions = JsFormatOptions;

impl FormatOptions for JsFormatOptions {
    #[inline]
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    #[inline]
    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    #[inline]
    fn line_width(&self) -> u8 {
        self.line_width
    }

    #[inline]
    fn as_print_options(&self) -> FirPrintOptions {
        self.as_print_options()
    }
}

/// JS/TS format context.
#[derive(Debug)]
pub struct JsFormatContext<'a> {
    /// The format options.
    pub options: JsFormatOptions,
    /// The source file for line ending and print integration.
    pub file: &'a File,
    /// The JS AST tree.
    pub tree: &'a NodeTree,
    /// Root nodes to format.
    pub roots: &'a [LocalNodeIdAny],
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
    /// The source span provider.
    pub source_map: &'a dyn JsSourceMap,
}

impl<'context> JsFormatContext<'context> {
    /// Return whether type syntax should be emitted.
    #[inline]
    pub fn include_types(&self) -> bool {
        self.options.include_types()
    }

    /// Return whether annotations should be emitted.
    #[inline]
    pub fn include_annotations(&self) -> bool {
        self.options.include_annotations()
    }

    /// Return one source span for one lowered node when one exists.
    #[inline]
    pub fn source_span(&self, node_id: u32) -> Option<Span> {
        self.source_map.source_span(self.tree, node_id)
    }

    /// Return one source part span when one exists.
    #[inline]
    pub fn source_part_span(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        self.source_map
            .source_part_span(self.tree, node_id, span_type)
    }
}

impl<'a> FormatContext for JsFormatContext<'a> {
    type Options = JsFormatOptions;

    #[inline]
    fn options(&self) -> &Self::Options {
        &self.options
    }

    #[inline]
    fn file(&self) -> &File {
        self.file
    }
}

/// Format one node with extra tree context.
pub(crate) trait FormatNode<'a, T: Node>
where
    JsFormatContext<'a>: FormatContext,
{
    /// Format one node.
    fn format_node(&self, node_id: LocalNodeId<T>, f: &mut JsFormatter<'a, '_>)
    -> FormatResult<()>;
}

impl<'a, T: Node> Format<JsFormatContext<'a>> for LocalNodeId<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut JsFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        let source_span = context.source_span(self.id);
        let node = context.tree.get(*self);

        // carry source markers when one caller has attached them
        if let Some(source_span) = source_span {
            source_position(source_span.start).format(f)?;
        }

        node.format_node(*self, f)?;

        if let Some(source_span) = source_span {
            source_position(source_span.end).format(f)?;
        }

        Ok(())
    }
}

impl<'a> Format<JsFormatContext<'a>> for LocalNodeIdAny {
    #[inline]
    fn format(&self, f: &mut JsFormatter<'a, '_>) -> FormatResult<()> {
        match self.ty {
            NodeType::Block => LocalNodeId::<Block>::new(self.id).format(f),
            NodeType::CatchClause => LocalNodeId::<CatchClause>::new(self.id).format(f),
            NodeType::Statement => LocalNodeId::<Statement>::new(self.id).format(f),
            NodeType::Expression => LocalNodeId::<Expression>::new(self.id).format(f),
            NodeType::ArrayElement => LocalNodeId::<ArrayElement>::new(self.id).format(f),
            NodeType::Declaration => LocalNodeId::<Declaration>::new(self.id).format(f),
            NodeType::Property => LocalNodeId::<Property>::new(self.id).format(f),
            NodeType::Member => LocalNodeId::<Member>::new(self.id).format(f),
            NodeType::TypeExpression => LocalNodeId::<TypeExpression>::new(self.id).format(f),
            NodeType::TupleElement => LocalNodeId::<TupleElement>::new(self.id).format(f),
            NodeType::TypeMember => LocalNodeId::<TypeMember>::new(self.id).format(f),
            NodeType::EnumField => LocalNodeId::<EnumField>::new(self.id).format(f),
            NodeType::DependencyItem => LocalNodeId::<DependencyItem>::new(self.id).format(f),
            NodeType::SwitchCase => LocalNodeId::<SwitchCase>::new(self.id).format(f),
            NodeType::Pattern => LocalNodeId::<Pattern>::new(self.id).format(f),
            NodeType::PatternField => LocalNodeId::<PatternField>::new(self.id).format(f),
            NodeType::AssignPattern => LocalNodeId::<AssignPattern>::new(self.id).format(f),
            NodeType::AssignPatternField => {
                LocalNodeId::<AssignPatternField>::new(self.id).format(f)
            }
            NodeType::GenericParameter => LocalNodeId::<GenericParameter>::new(self.id).format(f),
            NodeType::Parameter => LocalNodeId::<Parameter>::new(self.id).format(f),
            NodeType::Argument => LocalNodeId::<Argument>::new(self.id).format(f),
            NodeType::Annotation => LocalNodeId::<Annotation>::new(self.id).format(f),
            NodeType::Declarator => LocalNodeId::<Declarator>::new(self.id).format(f),
        }
    }
}

impl<'a> Format<JsFormatContext<'a>> for JsFormatContext<'a> {
    #[inline]
    fn format(&self, f: &mut JsFormatter<'a, '_>) -> FormatResult<()> {
        f.join_with(hard_line_break())
            .entries(self.roots)
            .finish()?;
        Ok(())
    }
}
