use crate::{
    Annotation, Argument, ArrayElement, AssignPattern, AssignPatternField, Block, CatchClause,
    Declaration, Declarator, DependencyItem, Expression, LocalNodeId, LocalNodeIdAny, Member, Node,
    NodeType, Parameter, Pattern, PatternField, Property, Statement, SwitchCase, Tree, TreeImpl,
};
use tspp_core::StringPool;
use tspp_dir as dir;
use tspp_fir::format::{self, Format, FormatResult};
use tspp_fir::prelude::{hard_line_break, source_position};
use tspp_fir::print::{MAX_OUTPUT_BYTES, PrintOptions as FirPrintOptions};
use tspp_source::{File, IndentStyle, LineEnding, NodeSpanType, Span};

/// The formatter for one JavaScript formatting pass.
pub type Formatter<'context, 'state> = format::Formatter<'state, 'context, Context<'context>>;

/// The formatting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FormatMode {
    /// Pretty.
    #[default]
    Pretty,
    /// Minimal.
    Minimal,
}

/// JavaScript formatting options.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Options {
    /// The formatting mode.
    pub mode: FormatMode = FormatMode::Pretty,
    /// The line ending to apply to printed output.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// The best effort maximum line length.
    pub line_width: u8 = 100,
}

impl Options {
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
            max_output_bytes: MAX_OUTPUT_BYTES,
        }
    }
}

impl format::FormatOptions for Options {
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

/// One JavaScript formatting pass.
#[derive(Debug)]
pub struct Context<'a> {
    /// The format options.
    pub options: Options,
    /// The source file for line ending and print integration.
    pub file: &'a File,
    /// The JavaScript tree.
    pub tree: &'a Tree,
    /// Root nodes to format.
    pub roots: &'a [LocalNodeIdAny],
    /// The string pool.
    pub strings: &'a StringPool,
    /// The originating DIR tree when source markers are requested.
    pub source: Option<&'a dir::Tree>,
}

impl Context<'_> {
    /// Return the source id for one lowered node when one exists.
    fn source_id(&self, node_id: u32) -> Option<u32> {
        let source = self.source?;
        let origin = self.tree.get_origin(node_id)?;

        // ignore nodes originating outside this source tree
        if origin.module_id != source.module_id || !source.has_node_id(origin.node_id) {
            return None;
        }

        Some(source.get_source(origin.node_id))
    }

    /// Return one source span for one lowered node when one exists.
    #[inline]
    pub fn source_span(&self, node_id: u32) -> Option<Span> {
        let source = self.source?;
        let source_id = self.source_id(node_id)?;

        source.get_span_by_id(source_id)
    }

    /// Return one source part span when one exists.
    #[inline]
    pub fn source_part_span(&self, node_id: u32, span_type: NodeSpanType) -> Option<Span> {
        let source = self.source?;
        let source_id = self.source_id(node_id)?;

        match span_type {
            NodeSpanType::Enclosing => source.get_span_by_id(source_id),
            NodeSpanType::Main => source.get_main_span_by_id(source_id),
            other => source.get_side_span_by_id(source_id, other),
        }
    }
}

impl<'a> format::FormatContext for Context<'a> {
    type Options = Options;

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
    Context<'a>: format::FormatContext,
{
    /// Format one node.
    fn format_node(&self, node_id: LocalNodeId<T>, f: &mut Formatter<'a, '_>) -> FormatResult<()>;
}

impl<'a, T: Node> Format<'a, Context<'a>> for LocalNodeId<T>
where
    T: Node + Clone,
    Tree: TreeImpl<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'a, '_>) -> FormatResult<()> {
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

impl<'a> Format<'a, Context<'a>> for LocalNodeIdAny {
    #[inline]
    fn format(&self, f: &mut Formatter<'a, '_>) -> FormatResult<()> {
        match self.ty {
            NodeType::Block => LocalNodeId::<Block>::new(self.id).format(f),
            NodeType::CatchClause => LocalNodeId::<CatchClause>::new(self.id).format(f),
            NodeType::Statement => LocalNodeId::<Statement>::new(self.id).format(f),
            NodeType::Expression => LocalNodeId::<Expression>::new(self.id).format(f),
            NodeType::ArrayElement => LocalNodeId::<ArrayElement>::new(self.id).format(f),
            NodeType::Declaration => LocalNodeId::<Declaration>::new(self.id).format(f),
            NodeType::Property => LocalNodeId::<Property>::new(self.id).format(f),
            NodeType::Member => LocalNodeId::<Member>::new(self.id).format(f),
            NodeType::DependencyItem => LocalNodeId::<DependencyItem>::new(self.id).format(f),
            NodeType::SwitchCase => LocalNodeId::<SwitchCase>::new(self.id).format(f),
            NodeType::Pattern => LocalNodeId::<Pattern>::new(self.id).format(f),
            NodeType::PatternField => LocalNodeId::<PatternField>::new(self.id).format(f),
            NodeType::AssignPattern => LocalNodeId::<AssignPattern>::new(self.id).format(f),
            NodeType::AssignPatternField => {
                LocalNodeId::<AssignPatternField>::new(self.id).format(f)
            }
            NodeType::Parameter => LocalNodeId::<Parameter>::new(self.id).format(f),
            NodeType::Argument => LocalNodeId::<Argument>::new(self.id).format(f),
            NodeType::Annotation => LocalNodeId::<Annotation>::new(self.id).format(f),
            NodeType::Declarator => LocalNodeId::<Declarator>::new(self.id).format(f),
        }
    }
}

impl<'a> Format<'a, Context<'a>> for Context<'a> {
    #[inline]
    fn format(&self, f: &mut Formatter<'a, '_>) -> FormatResult<()> {
        f.join_with(hard_line_break())
            .entries(self.roots)
            .finish()?;
        Ok(())
    }
}
