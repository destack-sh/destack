pub mod argument;
pub mod buffer;
pub mod builder;
pub mod context;
pub mod document;
pub mod error;
pub mod formatter;
pub mod group;
pub mod label;
pub mod macros;
pub mod node;
pub mod options;
pub mod sizing;
pub mod source;
pub mod spacing;
pub mod tag;

pub use argument::{Argument, Arguments};
pub use buffer::{
    Buffer, BufferExtensions, BufferSnapshot, Inspect, Recorded, Recording, RemoveSoftLinesBuffer,
    VecBuffer,
};
pub use builder::{
    Align, BestFitParenthesize, BestFitting, BlockIndent, Dedent, ExpandParent, FileSliceBuilder,
    FillBuilder, FormatOnce, FormatWith, IfGroupBreaks, Indent, IndentIfGroupBreaks, JoinBuilder,
    Line, LinePostfix, LinePostfixBoundary, Space, Text, Token, align, best_fit_parenthesize,
    block_indent, conditional_group, dedent, dedent_to_root, empty_line, expand_parent,
    fits_expanded, format_once, format_with, group, hard_line_break, if_group_breaks,
    if_group_fits_on_line, indent, indent_if_group_breaks, line_postfix, line_postfix_boundary,
    soft_block_indent, soft_line_break, soft_line_break_or_space, soft_line_indent_or_space,
    soft_space_or_block_indent, source_text_slice, space, text, token,
};
pub use context::{FormatContext, FormatState, SimpleFormatContext};
pub use document::Document;
pub use error::{
    ActualStart, FormatError, FormatResult, InvalidDocumentError, PrintError, PrintResult,
};
pub use formatter::{Format, Formatted, Formatter, FormatterSnapshot, format, write};
pub use group::{ConditionalGroup, DebugGroupId, Group, GroupId, GroupMode, ReleaseGroupId};
pub use label::{LabelDefinition, LabelId};
pub use node::{FormatNode, Interned, LineMode};
pub use options::{FormatOptions, SimpleFormatOptions};
pub use sizing::{
    BestFittingMode, BestFittingVariants, BestFittingVariantsIter, FormatNodes, TextLen, TextWidth,
    Width,
};
pub use source::{FileMarker, LINE_TERMINATORS, normalize_newlines};
pub use spacing::{IndentStyle, Indentation, LineEnding};
pub use tag::{
    Condition, DedentMode, FitsExpanded, FormatTag, FormatTagKind, PrintMode, VerbatimKind,
};
