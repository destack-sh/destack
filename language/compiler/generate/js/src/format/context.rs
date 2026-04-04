use crate::{
    Annotation, Argument, Block, Declaration, Declarator, DependencyItem, EnumField, Expression,
    LocalNodeId, LocalNodeIdAny, Member, Name, Node, NodeTree, NodeTreeImpl, NodeType, Parameter,
    Pattern, PatternField, Property, Statement, SwitchCase, Type, TypeField,
};
use destack_artifact::{Ast, DirPatched};
use destack_core::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::print::PrintOptions;
use destack_source::{File, FileType, IndentStyle, LineEnding, Span};
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
            trim_trailing_whitespace: true,
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
    /// The original source file.
    pub file: &'a File,
    /// The original AST for source span lookup.
    pub ast: &'a Ast,
    /// The patched DIR artifact for source span lookup.
    pub dir: &'a DirPatched,
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

    /// Return the source span for one lowered JS node when one exists.
    pub fn source_span(&self, node_id: u32) -> Option<Span> {
        let (module_id, source_id) = self.tree.get_source(node_id);

        // skip nodes lowered from a different source module
        if module_id != self.ast.id {
            return None;
        }

        // js nodes carry dir ids, so resolve them back to ast ids first
        if !self.dir.tree.has_node_id(source_id) {
            return None;
        }

        let source_id = self.dir.tree.get_source(source_id);

        Some(self.ast.tree.get_span_by_id(source_id))
    }

    /// Return the exact static dependency target span for one statement when one exists.
    pub fn statement_dependency_target_span(
        &self,
        node_id: LocalNodeId<Statement>,
    ) -> Option<Span> {
        let source_span = self.source_span(node_id.id)?;
        let statement = self.tree.get(node_id);
        match statement {
            Statement::Import { items, .. } => {
                if items.is_none() {
                    self.source_string_literal_span_after(source_span, "import")
                } else {
                    self.source_string_literal_span_after(source_span, "from")
                }
            }
            Statement::Export {
                target: Some(_), ..
            } => self.source_string_literal_span_after(source_span, "from"),
            _ => None,
        }
    }

    /// Return the exact dynamic import target span for one import call when one exists.
    pub fn import_call_target_span(&self, node_id: LocalNodeId<Expression>) -> Option<Span> {
        let source_span = self.source_span(node_id.id)?;

        self.source_string_literal_span_after(source_span, "import")
    }

    /// Return the exact dependency item name span when one exists.
    pub fn dependency_item_name_span(&self, node_id: LocalNodeId<DependencyItem>) -> Option<Span> {
        let source_span = self.source_span(node_id.id)?;
        let item = self.tree.get(node_id);
        let name = item.name?;

        match name {
            Name::Identifier(name) => {
                let name = self.strings.get(name);

                self.source_identifier_span(source_span, name)
            }
            Name::String(name) => {
                let name = self.strings.get(name);

                self.source_string_literal_span(source_span, name)
            }
        }
    }

    /// Return the exact dependency item alias span when one exists.
    pub fn dependency_item_alias_span(&self, node_id: LocalNodeId<DependencyItem>) -> Option<Span> {
        let source_span = self.source_span(node_id.id)?;
        let item = self.tree.get(node_id);
        let alias = item.alias?;
        let alias = self.strings.get(alias);

        self.source_identifier_span_from_end(source_span, alias)
    }

    /// Return the first quoted string literal span after one anchor within one source span.
    fn source_string_literal_span_after(&self, source_span: Span, anchor: &str) -> Option<Span> {
        let source_text = self.file.get_span_str(source_span)?;
        let anchor_offset = source_text.find(anchor)?;
        let search_offset = anchor_offset + anchor.len();
        let (quote_offset, quote) =
            source_text[search_offset..]
                .char_indices()
                .find_map(|(offset, character)| match character {
                    '"' | '\'' => Some((search_offset + offset, character)),
                    _ => None,
                })?;
        let content_offset = quote_offset + quote.len_utf8();
        let mut is_escaped = false;

        for (offset, character) in source_text[content_offset..].char_indices() {
            if is_escaped {
                is_escaped = false;
                continue;
            }

            if character == '\\' {
                is_escaped = true;
                continue;
            }

            if character != quote {
                continue;
            }

            let end_offset = content_offset + offset + character.len_utf8();

            return Some(Span::new(
                source_span.file,
                source_span.start + quote_offset as u32,
                source_span.start + end_offset as u32,
            ));
        }

        None
    }

    /// Return the quoted string literal span for one exact string value within one source span.
    fn source_string_literal_span(&self, source_span: Span, value: &str) -> Option<Span> {
        let source_text = self.file.get_span_str(source_span)?;
        let quoted_values = [format!("\"{value}\""), format!("'{value}'")];

        for quoted_value in quoted_values {
            if let Some(literal_offset) = source_text.find(&quoted_value) {
                let literal_end = literal_offset + quoted_value.len();

                return Some(Span::new(
                    source_span.file,
                    source_span.start + literal_offset as u32,
                    source_span.start + literal_end as u32,
                ));
            }
        }

        None
    }

    /// Return the first exact identifier span within one source span.
    fn source_identifier_span(&self, source_span: Span, identifier: &str) -> Option<Span> {
        let source_text = self.file.get_span_str(source_span)?;

        self.identifier_span_in_text(source_span, source_text, identifier)
    }

    /// Return the last exact identifier span within one source span.
    fn source_identifier_span_from_end(&self, source_span: Span, identifier: &str) -> Option<Span> {
        let source_text = self.file.get_span_str(source_span)?;
        let mut offset = source_text.len();

        while let Some(relative_offset) = source_text[..offset].rfind(identifier) {
            let end_offset = relative_offset + identifier.len();

            if Self::is_identifier_boundary(source_text, relative_offset, end_offset) {
                return Some(Span::new(
                    source_span.file,
                    source_span.start + relative_offset as u32,
                    source_span.start + end_offset as u32,
                ));
            }

            offset = relative_offset;
        }

        None
    }

    /// Return the first exact identifier span in one source text slice.
    fn identifier_span_in_text(
        &self,
        source_span: Span,
        source_text: &str,
        identifier: &str,
    ) -> Option<Span> {
        let mut offset = 0;

        while let Some(relative_offset) = source_text[offset..].find(identifier) {
            let start_offset = offset + relative_offset;
            let end_offset = start_offset + identifier.len();

            if Self::is_identifier_boundary(source_text, start_offset, end_offset) {
                return Some(Span::new(
                    source_span.file,
                    source_span.start + start_offset as u32,
                    source_span.start + end_offset as u32,
                ));
            }

            offset = end_offset;
        }

        None
    }

    /// Return whether one source range sits on identifier boundaries.
    fn is_identifier_boundary(source_text: &str, start_offset: usize, end_offset: usize) -> bool {
        let previous = source_text[..start_offset].chars().next_back();
        let next = source_text[end_offset..].chars().next();

        !Self::is_identifier_character(previous) && !Self::is_identifier_character(next)
    }

    /// Return whether one character may be part of one identifier.
    fn is_identifier_character(character: Option<char>) -> bool {
        character.is_some_and(|character| {
            character == '_' || character == '$' || character.is_alphanumeric()
        })
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
        let source_span = context.source_span(self.id);
        let node = context.tree.get(*self);

        // mark the start and end of each lowered node against its original source span
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

/// Implement Format for FormatNode for NodeIdsAny.
impl<'a> Format<CodegenJsFormatContext<'a>> for LocalNodeIdAny {
    #[inline]
    fn format(&self, f: &mut CodegenJsFormatter<'a, '_>) -> FormatResult<()> {
        match self.ty {
            NodeType::Block => LocalNodeId::<Block>::new(self.id).format(f),
            NodeType::Statement => LocalNodeId::<Statement>::new(self.id).format(f),
            NodeType::Expression => LocalNodeId::<Expression>::new(self.id).format(f),
            NodeType::Declaration => LocalNodeId::<Declaration>::new(self.id).format(f),
            NodeType::Property => LocalNodeId::<Property>::new(self.id).format(f),
            NodeType::Member => LocalNodeId::<Member>::new(self.id).format(f),
            NodeType::Type => LocalNodeId::<Type>::new(self.id).format(f),
            NodeType::TypeField => LocalNodeId::<TypeField>::new(self.id).format(f),
            NodeType::EnumField => LocalNodeId::<EnumField>::new(self.id).format(f),
            NodeType::DependencyItem => LocalNodeId::<DependencyItem>::new(self.id).format(f),
            NodeType::SwitchCase => LocalNodeId::<SwitchCase>::new(self.id).format(f),
            NodeType::Pattern => LocalNodeId::<Pattern>::new(self.id).format(f),
            NodeType::PatternField => LocalNodeId::<PatternField>::new(self.id).format(f),
            NodeType::Parameter => LocalNodeId::<Parameter>::new(self.id).format(f),
            NodeType::Argument => LocalNodeId::<Argument>::new(self.id).format(f),
            NodeType::Annotation => LocalNodeId::<Annotation>::new(self.id).format(f),
            NodeType::Declarator => LocalNodeId::<Declarator>::new(self.id).format(f),
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
