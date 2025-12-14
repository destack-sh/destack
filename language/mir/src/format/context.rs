//! MIR format context.

use std::collections::HashMap;

use destack_base::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::print::PrintOptions;
use destack_fir::write;
use destack_source::{File, FileType, IndentStyle, LineEnding};

use crate::{Block, Function, Global, Local, LocalNodeId, Node, NodeTree, NodeTreeImpl};

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
    /// The strings.
    pub strings: &'a ImmutableStringPool,
    /// Dummy file for FIR compatibility.
    file: File,

    // local context (a little bit hacky but fine for now)
    /// Map from block ID to its index in the current function's block list.
    /// Used for formatting block references with stable indices.
    pub block_indices: HashMap<LocalNodeId<Block>, usize>,
    /// Map from local ID to its index in the current function's local list.
    pub local_indices: HashMap<LocalNodeId<Local>, usize>,
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
    pub fn new(
        tree: &'a NodeTree,
        strings: &'a ImmutableStringPool,
        options: MirFormatOptions,
    ) -> Self {
        Self {
            options,
            tree,
            strings,
            file: File::empty_text(FileType::Destack),
            block_indices: HashMap::new(),
            local_indices: HashMap::new(),
        }
    }

    /// Get the index of a block in the current function.
    pub fn block_index(&self, id: LocalNodeId<Block>) -> usize {
        self.block_indices
            .get(&id)
            .copied()
            .unwrap_or(id.id as usize)
    }

    /// Get the index of a local in the current function.
    pub fn local_index(&self, id: LocalNodeId<Local>) -> usize {
        self.local_indices
            .get(&id)
            .copied()
            .unwrap_or(id.id as usize)
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
pub fn format_mir(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    options: MirFormatOptions,
) -> String {
    let context = MirFormatContext::new(tree, strings, options);

    // format all globals and functions
    let formatted = destack_fir::format!(context, [FormatAllItems]);
    match formatted {
        Ok(doc) => match doc.print() {
            Ok(printed) => printed.as_str().to_string(),
            Err(_) => "<print error>".to_string(),
        },
        Err(_) => "<format error>".to_string(),
    }
}

/// Helper to format all module items (globals and functions).
struct FormatAllItems;

impl<'a> Format<MirFormatContext<'a>> for FormatAllItems {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let tree = f.context().tree;
        let global_ids: Vec<_> = tree.iter_nodes::<Global>().map(|(id, _)| id).collect();
        let function_ids: Vec<_> = tree.iter_nodes::<Function>().map(|(id, _)| id).collect();
        let mut first = true;

        // format globals first
        for global_id in &global_ids {
            if !first {
                write!(f, [hard_line_break()])?;
            }
            first = false;
            write!(f, [global_id, hard_line_break()])?;
        }

        // format functions
        for function_id in &function_ids {
            if !first {
                write!(f, [hard_line_break()])?;
            }
            first = false;
            write!(f, [function_id])?;
        }
        Ok(())
    }
}
