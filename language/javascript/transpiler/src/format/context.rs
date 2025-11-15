use dyst_ast::StringId;
use dyst_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use dyst_fir::print::PrintOptions;
use dyst_javascript_ast::{
    Annotation, Argument, Block, Definition, DependencyItem, EnumField, Expression,
    MutableNodeTree, MutableNodeTreeImpl, Node, NodeId, NodeIdAny, NodeType, Parameter, Pattern,
    PatternField, Property, Statement, SwitchCase, Type,
};
use dyst_source::{File, ImmutableStringPool, IndentStyle, LineEnding};

use crate::{TranspilerLanguage, TranspilerUnit};

pub type JavaScriptFormatter<'ast, 'buf> = Formatter<'buf, JavaScriptFormatContext<'ast>>;

/// The formatting mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatMode {
    /// Pretty.
    Pretty,
    /// Minimal.
    Minimal,
}

/// JS/TS format options (mostly for testing).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct JavaScriptFormatOptions {
    /// The formatting mode.
    pub mode: FormatMode = FormatMode::Pretty,
    /// The language target.
    pub language: TranspilerLanguage = TranspilerLanguage::TypeScript,
    /// The type of line ending to apply to the printed input.  
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// Maximum line length (best effort).
    pub line_width: u8 = 100,
}

impl JavaScriptFormatOptions {
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

    /// Set the language target.
    pub fn with_language(mut self, language: TranspilerLanguage) -> Self {
        self.language = language;
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
            self.language,
            TranspilerLanguage::TypeScript | TranspilerLanguage::TypeScriptDeclaration
        )
    }

    /// Whether we need annotations.
    #[inline]
    pub fn include_annotations(&self) -> bool {
        matches!(
            self.language,
            TranspilerLanguage::TypeScript | TranspilerLanguage::TypeScriptDeclaration
        )
    }
}

impl FormatOptions for JavaScriptFormatOptions {
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
pub struct JavaScriptFormatContext<'a> {
    /// The format options.
    pub options: JavaScriptFormatOptions,
    /// The file.
    pub file: &'a File,
    /// The unit.
    pub unit: &'a TranspilerUnit,
    /// The tree.
    pub tree: &'a MutableNodeTree,
    /// The string pool.
    pub strings: &'a ImmutableStringPool,
}

impl<'ast> JavaScriptFormatContext<'ast> {
    /// Get alias for a definition from a given node.
    #[inline]
    pub fn get_alias_to_definition(
        &self,
        from_id: NodeIdAny,
        to_id: NodeId<Definition>,
    ) -> StringId {
        self.unit.get_alias_to_definition(from_id, to_id)
    }

    /// Whether we need type annotations.
    #[inline]
    pub fn include_types(&self) -> bool {
        matches!(
            self.options.language,
            TranspilerLanguage::TypeScript | TranspilerLanguage::TypeScriptDeclaration
        )
    }

    /// Whether we need annotations.
    #[inline]
    pub fn include_annotations(&self) -> bool {
        matches!(
            self.options.language,
            TranspilerLanguage::TypeScript | TranspilerLanguage::TypeScriptDeclaration
        )
    }
}

impl<'a> FormatContext for JavaScriptFormatContext<'a> {
    type Options = JavaScriptFormatOptions;

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
    JavaScriptFormatContext<'a>: FormatContext,
{
    /// Format a node.
    fn format_node(
        &self,
        node_id: NodeId<T>,
        f: &mut JavaScriptFormatter<'a, '_>,
    ) -> FormatResult<()>;
}

/// Implement Format for FormatNode for NodeIds.
impl<'a, T: Node> Format<JavaScriptFormatContext<'a>> for NodeId<T>
where
    T: Node + Clone,
    MutableNodeTree: MutableNodeTreeImpl<T>,
    T: FormatNode<'a, T>,
{
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        let node = context.tree.get(*self);
        node.format_node(*self, f)
    }
}

/// Implement Format for FormatNode for NodeIdsAny.
impl<'a> Format<JavaScriptFormatContext<'a>> for NodeIdAny {
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'a, '_>) -> FormatResult<()> {
        let context = f.context();
        match self.ty {
            NodeType::Block => {
                let node_id = NodeId::<Block>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Statement => {
                let node_id = NodeId::<Statement>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Expression => {
                let node_id = NodeId::<Expression>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Definition => {
                let node_id = NodeId::<Definition>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Property => {
                let node_id = NodeId::<Property>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Type => {
                let node_id = NodeId::<Type>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::EnumField => {
                let node_id = NodeId::<EnumField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::DependencyItem => {
                let node_id = NodeId::<DependencyItem>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::SwitchCase => {
                let node_id = NodeId::<SwitchCase>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Pattern => {
                let node_id = NodeId::<Pattern>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::PatternField => {
                let node_id = NodeId::<PatternField>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Parameter => {
                let node_id = NodeId::<Parameter>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Argument => {
                let node_id = NodeId::<Argument>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
            NodeType::Annotation => {
                let node_id = NodeId::<Annotation>::new(self.id);
                let node = context.tree.get(node_id);
                node.format_node(node_id, f)
            }
        }
    }
}

impl<'a> Format<JavaScriptFormatContext<'a>> for TranspilerUnit {
    #[inline]
    fn format(&self, f: &mut JavaScriptFormatter<'a, '_>) -> FormatResult<()> {
        for root in &self.roots {
            root.format(f)?;
        }
        Ok(())
    }
}
