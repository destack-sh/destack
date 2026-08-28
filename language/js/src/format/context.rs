use std::collections::HashMap;

use crate::{Identifier, LocalNodeId, Node, SymbolId, SymbolTable, Tree, TreeStore};
use destack_core::{StringId, StringPool};
use destack_fir::format::{self, Format, FormatElement, FormatResult, FormatTag};
use destack_fir::print::{MAX_OUTPUT_BYTES, PrintOptions as FirPrintOptions};
use destack_source::{File, IndentStyle, LineEnding, ProvenanceId, TextNameId};

/// The formatter for one JavaScript formatting pass.
pub(crate) type Formatter<'context, 'state> =
    format::Formatter<'state, 'context, Context<'context>>;

/// JavaScript formatting options.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct FormatOptions {
    /// The line ending to apply to printed output.
    pub line_ending: LineEnding = LineEnding::LineFeed,
    /// The indent style.
    pub indent_style: IndentStyle = IndentStyle::Space,
    /// Spaces per indent.
    pub indent_width: u8 = 4,
    /// The best effort maximum line length.
    pub line_width: u8 = 100,
}

impl FormatOptions {
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
}

impl format::FormatOptions for FormatOptions {
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

/// One JavaScript formatting pass.
#[derive(Debug)]
pub(crate) struct Context<'a> {
    /// The format options.
    pub options: FormatOptions,
    /// The source file for line ending and print integration.
    pub file: &'a File,
    /// The JavaScript tree.
    pub tree: &'a Tree,
    /// The string pool.
    pub strings: &'a StringPool,
    /// The module symbols.
    pub symbols: &'a SymbolTable,
    /// Output name ids keyed by interned JavaScript names.
    pub text_names: &'a HashMap<StringId, TextNameId>,
}

/// Format content under one provenance attribution.
pub(crate) fn format_attributed<'ast>(
    provenance: ProvenanceId,
    name: Option<TextNameId>,
    f: &mut Formatter<'ast, '_>,
    content: impl FnOnce(&mut Formatter<'ast, '_>) -> FormatResult<()>,
) -> FormatResult<()> {
    f.write_element(FormatElement::Tag(FormatTag::StartProvenance {
        provenance,
        name,
    }));
    content(f)?;
    f.write_element(FormatElement::Tag(FormatTag::EndProvenance));

    Ok(())
}

/// Format one identifier with its occurrence and symbol provenance.
pub(crate) fn format_identifier<'ast>(
    identifier: Identifier,
    f: &mut Formatter<'ast, '_>,
) -> FormatResult<()> {
    let text_name = text_name(identifier.original_name, f.context())?;

    format_attributed(identifier.provenance, Some(text_name), f, |f| {
        format_symbol(identifier.symbol, f)
    })
}

/// Return the output name id for one interned JavaScript name.
pub(crate) fn text_name(name: StringId, context: &Context<'_>) -> FormatResult<TextNameId> {
    let Some(name) = context.text_names.get(&name).copied() else {
        return Err(format::FormatError::SyntaxError {
            message: "JavaScript name is absent from the output name table",
        });
    };

    Ok(name)
}

/// Format one symbol through its complete link chain.
fn format_symbol<'ast>(mut id: SymbolId, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
    let mut depth = 0;

    // open every symbol attribution from the occurrence symbol to its canonical symbol
    loop {
        let symbol = f.context().symbols.get(id);
        f.write_element(FormatElement::Tag(FormatTag::StartProvenance {
            provenance: symbol.provenance,
            name: None,
        }));
        depth += 1;

        if symbol.link == id {
            symbol.name.format(f)?;
            break;
        }

        id = symbol.link;
    }

    // close the nested symbol attributions
    for _ in 0..depth {
        f.write_element(FormatElement::Tag(FormatTag::EndProvenance));
    }

    Ok(())
}

impl<'a> format::FormatContext for Context<'a> {
    type Options = FormatOptions;

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
pub(crate) trait FormatNode<'a>: Node
where
    Context<'a>: format::FormatContext,
{
    /// Format one node.
    fn format_node(&self, f: &mut Formatter<'a, '_>) -> FormatResult<()>;
}

impl<'a, T> Format<'a, Context<'a>> for LocalNodeId<T>
where
    T: FormatNode<'a>,
    Tree: TreeStore<T>,
{
    #[inline]
    fn format(&self, f: &mut Formatter<'a, '_>) -> FormatResult<()> {
        let node = f.context().tree.get(*self);

        let provenance = f.context().tree.provenance(*self);

        format_attributed(provenance, None, f, |f| node.format_node(f))
    }
}
