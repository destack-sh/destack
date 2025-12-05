//! MIR format context.

use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::print::PrintOptions;
use destack_fir::write;
use destack_source::{File, FileType, IndentStyle, LineEnding};

use crate::{Function, LocalNodeId, Node, NodeTree, NodeTreeImpl};

pub type MirFormatter<'a, 'buf> = Formatter<'buf, MirFormatContext<'a>>;

/// MIR format options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MirFormatOptions {
    /// Line ending style.
    pub line_ending: LineEnding,
    /// Indent style.
    pub indent_style: IndentStyle,
    /// Indent width.
    pub indent_width: u8,
    /// Maximum line width.
    pub line_width: u8,
}

impl Default for MirFormatOptions {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
        }
    }
}

impl MirFormatOptions {
    /// Convert to print options.
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }
}

impl FormatOptions for MirFormatOptions {
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    fn line_width(&self) -> u8 {
        self.line_width
    }

    fn as_print_options(&self) -> PrintOptions {
        self.as_print_options()
    }
}

/// MIR format context.
pub struct MirFormatContext<'a> {
    /// Format options.
    pub options: MirFormatOptions,
    /// The MIR tree.
    pub tree: &'a NodeTree,
    /// Dummy file for FIR compatibility.
    file: File,
}

impl<'a> std::fmt::Debug for MirFormatContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MirFormatContext")
            .field("options", &self.options)
            .finish()
    }
}

impl<'a> MirFormatContext<'a> {
    /// Create a new format context.
    pub fn new(tree: &'a NodeTree, options: MirFormatOptions) -> Self {
        Self {
            options,
            tree,
            file: File::empty_text_with_type(FileType::Destack),
        }
    }
}

impl<'a> FormatContext for MirFormatContext<'a> {
    type Options = MirFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn file(&self) -> &File {
        &self.file
    }
}

/// Trait for formatting MIR nodes.
pub trait FormatMirNode<'a, T: Node> {
    /// Format a node.
    fn format_node(&self, id: LocalNodeId<T>, f: &mut MirFormatter<'a, '_>) -> FormatResult<()>;
}

/// Implement Format for LocalNodeId<T> where T implements FormatMirNode.
impl<'a, T: Node + Clone> Format<MirFormatContext<'a>> for LocalNodeId<T>
where
    NodeTree: NodeTreeImpl<T>,
    T: FormatMirNode<'a, T>,
{
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let node = f.context().tree.get(*self);
        node.format_node(*self, f)
    }
}

/// Format a MIR tree to a string.
pub fn format_mir(tree: &NodeTree) -> String {
    format_mir_with_options(tree, MirFormatOptions::default())
}

/// Format a MIR tree to a string with options.
pub fn format_mir_with_options(tree: &NodeTree, options: MirFormatOptions) -> String {
    let context = MirFormatContext::new(tree, options);

    // format all functions
    let formatted = destack_fir::format!(context, [FormatAllFunctions]);
    match formatted {
        Ok(doc) => match doc.print() {
            Ok(printed) => printed.as_str().to_string(),
            Err(_) => "<print error>".to_string(),
        },
        Err(_) => "<format error>".to_string(),
    }
}

/// Helper to format all functions.
struct FormatAllFunctions;

impl<'a> Format<MirFormatContext<'a>> for FormatAllFunctions {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let tree = f.context().tree;
        let function_ids: Vec<_> = tree.iter_nodes::<Function>().map(|(id, _)| id).collect();

        for (i, func_id) in function_ids.iter().enumerate() {
            if i > 0 {
                write!(f, [hard_line_break(), hard_line_break()])?;
            }
            write!(f, [func_id])?;
        }
        Ok(())
    }
}
