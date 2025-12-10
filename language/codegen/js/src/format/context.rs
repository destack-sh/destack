use crate::{
    Annotation, Argument, Block, Declaration, Declarator, DependencyItem, EnumField, Expression,
    LocalNodeId, LocalNodeIdAny, Node, NodeTree, NodeTreeImpl, NodeType, Parameter, Pattern,
    PatternField, Property, Statement, SwitchCase, Type, TypeField,
};
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::print::PrintOptions;
use destack_source::{File, FileType, ImmutableStringPool, IndentStyle, LineEnding};
use destack_workspace::Target;

pub type CodegenJsFormatter<'ast, 'buf> = Formatter<'buf, CodegenJsFormatContext<'ast>>;

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
pub struct CodegenJsFormatOptions {
    /// The formatting mode.
    pub mode: FormatMode = FormatMode::Pretty,
    /// The output file type (determines whether to emit types).
    pub file_type: FileType = FileType::TypeScript,
    /// The type of line ending to apply to the printed input.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl CodegenJsFormatOptions {
    /// Create options from a Target and the specific file type being generated.
    pub fn from_target(_target: &Target, file_type: FileType) -> Self {
        // NOTE #Broken: use target settings for formatting options
        Self {
            mode: FormatMode::Pretty,
            file_type,
            ..Self::default()
        }
    }

    /// Pretty options.
    pub fn pretty() -> Self {
        Self {
            mode: FormatMode::Pretty,
            ..Self::default()
        }
    }

    /// Pretty options with a given line width.
    pub fn pretty_with_line_width(line_width: u8) -> Self {
        Self {
            mode: FormatMode::Pretty,
            line_width,
            ..Self::default()
        }
    }

    /// Default options with tab indent style.
    pub fn pretty_tab() -> Self {
        Self {
            mode: FormatMode::Pretty,
            indent_style: IndentStyle::Tab,
            ..Self::default()
        }
    }

    /// Minimal options.
    pub fn minimal() -> Self {
        Self {
            mode: FormatMode::Minimal,
            indent_width: 0,
            ..Self::default()
        }
    }

    /// Minimal options with a given line width.
    pub fn minimal_with_line_width(line_width: u8) -> Self {
        Self {
            mode: FormatMode::Minimal,
            line_width,
            ..Self::default()
        }
    }

    /// Minimal options with tab indent style.
    pub fn minimal_tab() -> Self {
        Self {
            mode: FormatMode::Minimal,
            indent_style: IndentStyle::Tab,
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

    /// Convert to print options.
    #[inline]
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }

    /// Whether we need type annotations.
    #[inline]
    pub fn include_types(&self) -> bool {
        matches!(
            self.file_type,
            FileType::TypeScript | FileType::TypeScriptXml | FileType::TypeScriptDeclaration
        )
    }

    /// Whether we need annotations/decorators.
    #[inline]
    pub fn include_annotations(&self) -> bool {
        matches!(
            self.file_type,
            FileType::TypeScript | FileType::TypeScriptXml | FileType::TypeScriptDeclaration
        )
    }

    /// Whether this is generating declarations only (.d.ts).
    #[inline]
    pub fn is_declaration(&self) -> bool {
        matches!(self.file_type, FileType::TypeScriptDeclaration)
    }
}

impl FormatOptions for CodegenJsFormatOptions {
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
    fn as_print_options(&self) -> PrintOptions {
        self.as_print_options()
    }
}

/// JS/TS format context.
#[derive(Debug)]
pub struct CodegenJsFormatContext<'a> {
    /// The format options.
    pub options: CodegenJsFormatOptions,
    /// The file (for span information).
    pub file: &'a File,
    /// The JS AST tree.
    pub tree: &'a NodeTree,
    /// Root nodes to format.
    pub roots: &'a [LocalNodeIdAny],
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
}

impl<'ast> CodegenJsFormatContext<'ast> {
    /// Whether we need type annotations.
    #[inline]
    pub fn include_types(&self) -> bool {
        self.options.include_types()
    }

    /// Whether we need annotations.
    #[inline]
    pub fn include_annotations(&self) -> bool {
        self.options.include_annotations()
    }
}

impl<'a> FormatContext for CodegenJsFormatContext<'a> {
    type Options = CodegenJsFormatOptions;

    #[inline]
    fn options(&self) -> &Self::Options {
        &self.options
    }

    #[inline]
    fn file(&self) -> &File {
        self.file
    }
}

/// Format Nodes with more information.
pub(crate) trait FormatNode<'a, T: Node>
where
    CodegenJsFormatContext<'a>: FormatContext,
{
    /// Format a node.
    fn format_node(
        &self,
        node_id: LocalNodeId<T>,
        f: &mut CodegenJsFormatter<'a, '_>,
    ) -> FormatResult<()>;
}

/// Implement Format for FormatNode for NodeIds.
impl<'a, T: Node> Format<CodegenJsFormatContext<'a>> for LocalNodeId<T>
where
    T: Node + Clone,
    NodeTree: NodeTreeImpl<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        let node = context.tree.get(*self);
        node.format_node(*self, f)
    }
}

/// Implement Format for FormatNode for NodeIdsAny.
impl<'a> Format<CodegenJsFormatContext<'a>> for LocalNodeIdAny {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        match self.ty {
            NodeType::Block => {
                let node_id = LocalNodeId::<Block>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Statement => {
                let node_id = LocalNodeId::<Statement>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Expression => {
                let node_id = LocalNodeId::<Expression>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Declaration => {
                let node_id = LocalNodeId::<Declaration>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Property => {
                let node_id = LocalNodeId::<Property>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Type => {
                let node_id = LocalNodeId::<Type>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::TypeField => {
                let node_id = LocalNodeId::<TypeField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::EnumField => {
                let node_id = LocalNodeId::<EnumField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::DependencyItem => {
                let node_id = LocalNodeId::<DependencyItem>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::SwitchCase => {
                let node_id = LocalNodeId::<SwitchCase>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Pattern => {
                let node_id = LocalNodeId::<Pattern>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::PatternField => {
                let node_id = LocalNodeId::<PatternField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Parameter => {
                let node_id = LocalNodeId::<Parameter>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Argument => {
                let node_id = LocalNodeId::<Argument>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Annotation => {
                let node_id = LocalNodeId::<Annotation>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Declarator => {
                let node_id = LocalNodeId::<Declarator>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
        }
    }
}

/// Implement Format for the context itself (formats all roots).
impl<'a> Format<CodegenJsFormatContext<'a>> for CodegenJsFormatContext<'a> {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'a, '_>) -> FormatResult<()> {
        f.join_with(hard_line_break())
            .entries(self.roots)
            .finish()?;
        Ok(())
    }
}
