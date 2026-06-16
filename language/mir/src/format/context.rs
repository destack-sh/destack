use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_fir::format::{
    Format, FormatContext, FormatError, FormatOptions, FormatResult, Formatter,
};
use destack_fir::prelude::*;
use destack_fir::print::PrintOptions;
use destack_fir::write;
use destack_source::{File, FileType, IndentStyle, LineEnding};

use crate::source::TokenType;
use crate::{
    Block, Function, Global, LifetimeParameter, LifetimeSlot, Local, LocalNodeId, Node, NodeType,
    Tree, TreeImpl, Type, TypeAlias, Value,
};

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
            trim_trailing_whitespace: false,
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
    pub tree: &'a Tree,
    /// The strings.
    pub strings: &'a StringPool,
    /// Dummy file for FIR compatibility.
    file: File,

    /// Map from local ID to its index in the current function's local list.
    pub local_indices: HashMap<LocalNodeId<Local>, usize>,
    /// Map from function ID to its unique display name.
    pub function_names: HashMap<LocalNodeId<Function>, String>,
    /// Map from block ID to its unique display name.
    pub block_names: HashMap<LocalNodeId<Block>, String>,
    /// Map from global ID to its unique display name.
    pub global_names: HashMap<LocalNodeId<Global>, String>,
    /// Map from type ID to its alias name (if any).
    pub type_alias_by_type: HashMap<LocalNodeId<Type>, String>,
    /// The function currently being formatted.
    pub current_function: Option<LocalNodeId<Function>>,
    /// Lifetime parameters currently in scope.
    pub current_lifetimes: Vec<LifetimeParameter>,
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
        tree: &'a Tree,
        strings: &'a StringPool,
        options: MirFormatOptions,
    ) -> FormatResult<Self> {
        // collect explicit type aliases
        let type_alias_by_type: HashMap<_, _> = tree
            .iter_nodes::<TypeAlias>()
            .filter_map(|(_, alias)| {
                let ty = alias.ty.ty()?;

                let name = strings.get(alias.name).to_string();
                Some((ty, name))
            })
            .collect();

        // assign unique function and global names
        let function_names = build_unique_function_names(tree, strings);
        let block_names = build_unique_block_names(tree, strings);
        let global_names = build_unique_global_names(tree, strings);

        // assemble the format context
        Ok(Self {
            options,
            tree,
            strings,
            file: File::empty_text(FileType::Destack),
            local_indices: HashMap::new(),
            function_names,
            block_names,
            global_names,
            type_alias_by_type,
            current_function: None,
            current_lifetimes: Vec::new(),
        })
    }

    /// Get the display name of a block in the current function.
    pub fn block_name(&self, id: LocalNodeId<Block>) -> String {
        self.block_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("b{}", id.id))
    }

    /// Get the index of a local in the current function.
    pub fn local_index(&self, id: LocalNodeId<Local>) -> usize {
        // resolve the cached local index when available
        if let Some(index) = self.local_indices.get(&id) {
            return *index;
        }

        // fall back to the local id
        id.id as usize
    }

    /// Get the display name for an SSA value in the current function.
    pub fn value_name(&self, value: Value) -> FormatResult<String> {
        let function_id = self.current_function.ok_or(FormatError::SyntaxError {
            message: "missing current function while formatting value",
        })?;
        let function = self.tree.get(function_id);
        let name = if let Some(name) = function.value_name(value) {
            self.strings.get(name).to_string()
        } else {
            format!("v{}", value.0)
        };

        Ok(name)
    }

    /// Get the unique function display name.
    pub fn function_name(&self, id: LocalNodeId<Function>) -> &str {
        // resolve cached unique name
        if let Some(name) = self.function_names.get(&id) {
            return name.as_str();
        }

        // fall back to stored name
        let function = self.tree.get(id);
        self.strings.get(function.name)
    }

    /// Get the unique global display name.
    pub fn global_name(&self, id: LocalNodeId<Global>) -> &str {
        // resolve cached unique name
        if let Some(name) = self.global_names.get(&id) {
            return name.as_str();
        }

        // fall back to stored name
        let global = self.tree.get(id);
        self.strings.get(global.name)
    }

    /// Get the alias name for a type, if one exists.
    pub fn type_alias_name(&self, ty: LocalNodeId<Type>) -> Option<&str> {
        // resolve the alias name when present
        self.type_alias_by_type.get(&ty).map(|name| name.as_str())
    }

    /// Get the lifetime parameter name for a slot, if one is in scope.
    pub fn lifetime_name(&self, slot: LifetimeSlot) -> Option<&str> {
        let lifetime = self.current_lifetimes.get(slot.0 as usize)?;
        let name = lifetime.name?;

        Some(self.strings.get(name))
    }

    /// Get the type for a value in the current function.
    pub fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        let function_id = self.current_function?;
        let function = self.tree.get(function_id);
        function.value_type(value)
    }
}

/// Build unique display names for functions.
fn build_unique_function_names(
    tree: &Tree,
    strings: &StringPool,
) -> HashMap<LocalNodeId<Function>, String> {
    // collect function names
    let names = tree
        .iter_nodes::<Function>()
        .map(|(id, function)| (id, strings.get(function.name).to_string()));

    build_unique_names(names)
}

/// Build unique display names for blocks.
fn build_unique_block_names(
    tree: &Tree,
    strings: &StringPool,
) -> HashMap<LocalNodeId<Block>, String> {
    let mut names_by_id = HashMap::new();

    // build names independently per function
    for (_, function) in tree.iter_nodes::<Function>() {
        let names = function.blocks.iter().enumerate().map(|(index, block_id)| {
            let block = tree.get(*block_id);
            let name = if let Some(name) = block.name {
                strings.get(name).to_string()
            } else if index == 0 {
                "entry".to_string()
            } else {
                format!("b{index}")
            };

            (*block_id, name)
        });

        names_by_id.extend(build_unique_names(names));
    }

    names_by_id
}

/// Build unique display names for globals.
fn build_unique_global_names(
    tree: &Tree,
    strings: &StringPool,
) -> HashMap<LocalNodeId<Global>, String> {
    // collect global names
    let names = tree
        .iter_nodes::<Global>()
        .map(|(id, global)| (id, strings.get(global.name).to_string()));

    build_unique_names(names)
}

/// Build unique display names for nodes.
fn build_unique_names<T>(
    items: impl Iterator<Item = (LocalNodeId<T>, String)>,
) -> HashMap<LocalNodeId<T>, String>
where
    T: Node,
{
    // track used names and suffixes
    let mut used_names = HashSet::new();
    let mut next_suffix: HashMap<String, usize> = HashMap::new();
    let mut names_by_id = HashMap::new();

    // assign stable unique names
    for (id, base) in items {
        // resolve the base name or suffix
        let name = if used_names.contains(&base) {
            // use a suffixed variant
            // seed the suffix counter
            let entry = next_suffix.entry(base.clone()).or_insert(1);

            // find the next available suffix
            loop {
                let candidate = format!("{base}_{entry}");
                *entry += 1;
                if !used_names.contains(&candidate) {
                    break candidate;
                }
            }
        } else {
            // use the base name
            base.clone()
        };

        // record the final name
        used_names.insert(name.clone());
        names_by_id.insert(id, name);
    }

    // return the final name map
    names_by_id
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
    Tree: TreeImpl<T>,
    T: FormatMirNode<'a, T>,
{
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let node = f.context().tree.get(*self);
        node.format_node(*self, f)
    }
}

/// Format a MIR tree to a string.
pub fn format_mir(
    tree: &Tree,
    strings: &StringPool,
    options: MirFormatOptions,
) -> FormatResult<String> {
    let context = MirFormatContext::new(tree, strings, options)?;

    // format all globals and functions
    let document = destack_fir::format!(context, [FormatAllItems])?;

    // print the formatted document
    let printed = document.print()?;

    Ok(printed.as_str().to_string())
}

/// One normalized comment line.
#[derive(Debug, Clone)]
struct CommentLine {
    /// The number of blank lines before this comment.
    blank_lines_before: usize,
    /// The exact comment text.
    text: String,
}

/// One normalized comment block.
struct CommentBlock {
    /// The comment lines.
    lines: Vec<CommentLine>,
    /// The blank lines after the final comment.
    trailing_blank_lines: usize,
}

/// Return one node's leading trivia byte bounds.
fn leading_trivia_bounds<T>(tree: &Tree, id: LocalNodeId<T>) -> Option<(u32, u32)>
where
    T: Node,
{
    let span = tree.leading_comment_span(id)?;
    Some((span.start, span.end))
}

/// Return the source start used to order one top level item.
fn top_level_item_start<T>(tree: &Tree, id: LocalNodeId<T>) -> u32
where
    T: Node,
{
    // prefer leading trivia so leading comments stay attached to the item order
    if let Some(span) = tree.leading_comment_span(id) {
        return span.start;
    }

    // fall back to the node enclosing span
    tree.get_span(id).map(|span| span.start).unwrap_or(u32::MAX)
}

/// Collect normalized comments between byte offsets.
fn collect_comments_between(tree: &Tree, start: u32, end: u32) -> CommentBlock {
    let mut comments = Vec::new();
    let mut newline_count = 0usize;

    for token in tree.tokens() {
        if token.span.end <= start {
            continue;
        }

        if token.span.start >= end {
            break;
        }

        match token.ty {
            TokenType::Comment => {
                comments.push(CommentLine {
                    blank_lines_before: newline_count.saturating_sub(1),
                    text: tree.source_text(token.span).to_string(),
                });
                newline_count = 0;
            }
            TokenType::Newline => {
                newline_count += 1;
            }
            TokenType::Whitespace => {}
            _ => {}
        }
    }

    CommentBlock {
        lines: comments,
        trailing_blank_lines: newline_count.saturating_sub(1),
    }
}

/// Write one normalized blank-line gap.
fn write_blank_line_gap<'a>(count: usize, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    for _ in 0..count {
        write!(f, [empty_line()])?;
    }

    Ok(())
}

/// Write one normalized comment block.
fn write_comment_block<'a>(
    block: &CommentBlock,
    first_blank_lines_to_skip: usize,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    if block.lines.is_empty() {
        return Ok(false);
    }

    for (index, comment) in block.lines.iter().enumerate() {
        let blank_lines_before = if index == 0 {
            comment
                .blank_lines_before
                .saturating_sub(first_blank_lines_to_skip)
        } else {
            comment.blank_lines_before
        };

        if index > 0 {
            if blank_lines_before > 0 {
                write_blank_line_gap(blank_lines_before, f)?;
            } else {
                write!(f, [hard_line_break()])?;
            }
        } else if blank_lines_before > 0 {
            write_blank_line_gap(blank_lines_before, f)?;
        }

        write!(f, [text(&comment.text)])?;
    }

    if block.trailing_blank_lines > 0 {
        write_blank_line_gap(block.trailing_blank_lines, f)?;
    } else {
        write!(f, [hard_line_break()])?;
    }

    Ok(true)
}

/// Write comment lines after one canonical separator.
fn write_comments_after_separator<'a>(
    tree: &Tree,
    start: u32,
    end: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    let comments = collect_comments_between(tree, start, end);
    write_comment_block(&comments, 1, f)
}

/// Write comment lines between byte offsets as standalone lines.
pub(crate) fn write_comments_before<'a>(
    tree: &Tree,
    start: u32,
    end: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    let comments = collect_comments_between(tree, start, end);
    write_comment_block(&comments, 0, f)
}

/// Write comments after one anchor, keeping inline comments inline.
pub(crate) fn write_inline_comment_after<'a>(
    tree: &Tree,
    start: u32,
    end: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    let inline_comment = tree.inline_comment_between(start, end);

    if let Some(comment) = inline_comment {
        write!(f, [space(), text(&comment.text)])?;
        return Ok(true);
    }

    Ok(false)
}

/// Write comments after one anchor, keeping inline comments inline.
pub(crate) fn write_comments_after<'a>(
    tree: &Tree,
    start: u32,
    end: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    let mut block_start = None;
    let mut saw_newline = false;
    let wrote_inline_comment = write_inline_comment_after(tree, start, end, f)?;

    for token in tree.tokens() {
        if token.span.end <= start {
            continue;
        }

        if token.span.start >= end {
            break;
        }

        if token.ty == TokenType::Newline && !saw_newline {
            saw_newline = true;
            block_start = Some(token.span.start);
        }
    }

    let mut wrote_comment = wrote_inline_comment;

    if let Some(block_start) = block_start
        && write_comments_before(tree, block_start, end, f)?
    {
        wrote_comment = true;
    }

    Ok(wrote_comment)
}

/// Write leading comments for one node.
pub(crate) fn write_node_leading_comments<'a, T>(
    tree: &Tree,
    id: LocalNodeId<T>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some((start, end)) = leading_trivia_bounds(tree, id) else {
        return Ok(false);
    };

    write_comments_before(tree, start, end, f)
}

/// Write leading comments after one canonical sibling separator.
pub(crate) fn write_node_leading_comments_after_separator<'a, T>(
    tree: &Tree,
    id: LocalNodeId<T>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some((start, end)) = leading_trivia_bounds(tree, id) else {
        return Ok(false);
    };

    write_comments_after_separator(tree, start, end, f)
}

/// Write trailing comments after the final top level item.
fn write_top_level_comments_after<'a, T>(
    tree: &Tree,
    id: LocalNodeId<T>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some(span) = tree.get_span(id) else {
        return Ok(false);
    };

    write!(f, [hard_line_break()])?;

    let scope_end = tree
        .tokens()
        .last()
        .map(|token| token.span.end)
        .unwrap_or(span.end);
    write_comments_after_separator(tree, span.end, scope_end, f)
}

/// Helper to format all module items.
struct FormatAllItems;

impl<'a> Format<MirFormatContext<'a>> for FormatAllItems {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let tree = f.context().tree;
        let mut has_output = false;

        // explicit top level items
        let mut item_ids = Vec::new();

        for (id, _) in tree.iter_nodes::<TypeAlias>() {
            let start = top_level_item_start(tree, id);
            item_ids.push((start, id.id, NodeType::TypeAlias));
        }

        for (id, _) in tree.iter_nodes::<Global>() {
            let start = top_level_item_start(tree, id);
            item_ids.push((start, id.id, NodeType::Global));
        }

        for (id, _) in tree.iter_nodes::<Function>() {
            let start = top_level_item_start(tree, id);
            item_ids.push((start, id.id, NodeType::Function));
        }

        item_ids.sort_by_key(|(start, node_id, _)| (*start, *node_id));

        for (index, (_, node_id, node_type)) in item_ids.iter().enumerate() {
            let next_boundary = item_ids.get(index + 1).map(|(start, _, _)| *start);

            match node_type {
                NodeType::TypeAlias => format_top_level_item(
                    tree,
                    LocalNodeId::<TypeAlias>::new(*node_id),
                    next_boundary,
                    has_output,
                    f,
                )?,
                NodeType::Global => format_top_level_item(
                    tree,
                    LocalNodeId::<Global>::new(*node_id),
                    next_boundary,
                    has_output,
                    f,
                )?,
                NodeType::Function => format_top_level_item(
                    tree,
                    LocalNodeId::<Function>::new(*node_id),
                    next_boundary,
                    has_output,
                    f,
                )?,
                _ => unreachable!(),
            }

            has_output = true;
        }

        Ok(())
    }
}

/// Format one explicit top level item.
fn format_top_level_item<'a, T>(
    tree: &Tree,
    id: LocalNodeId<T>,
    next_boundary: Option<u32>,
    has_output: bool,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()>
where
    Tree: TreeImpl<T>,
    T: FormatMirNode<'a, T> + Node + Clone,
{
    // item spacing
    if has_output {
        write!(f, [empty_line()])?;
    }

    // leading comments
    if has_output {
        write_node_leading_comments_after_separator(tree, id, f)?;
    } else {
        write_node_leading_comments(tree, id, f)?;
    }

    // item and trailing comments
    write!(f, [id])?;

    if let Some(next_boundary) = next_boundary {
        if let Some(span) = tree.get_span(id) {
            write_inline_comment_after(tree, span.end, next_boundary, f)?;
        }
    } else {
        write_top_level_comments_after(tree, id, f)?;
    }

    Ok(())
}
