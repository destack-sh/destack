#![feature(default_field_values)]

pub mod argument;
pub mod buffer;
pub mod builder;
pub mod context;
pub mod document;
pub mod element;
pub mod error;
pub mod format;
pub mod formatter;
pub mod group;
pub mod label;
pub mod macros;
pub mod prelude;
pub mod printer;
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
    Align, BestFitParenthesize, BestFitting, BlockIndent, Dedent, ExpandParent, FillBuilder,
    FormatOnce, FormatWith, IfGroupBreaks, Indent, IndentIfGroupBreaks, JoinBuilder, Line,
    LineBoundary, LineSuffix, SourceSliceBuilder, Space, Text, Token, align,
    best_fit_parenthesize, block_indent, conditional_group, dedent, dedent_to_root, empty_line,
    expand_parent, fits_expanded, format_once, format_with, group, hard_line_break,
    if_group_breaks, if_group_fits_on_line, indent, indent_if_group_breaks, line_suffix,
    line_suffix_boundary, soft_block_indent, soft_line_break, soft_line_break_or_space,
    soft_line_indent_or_space, soft_space_or_block_indent, source_text_slice, space, text, token,
};
pub use context::{
    FormatContext, FormatOptions, FormatState, SimpleFormatContext, SimpleFormatOptions,
};
pub use document::Document;
pub use element::{FormatElement, Interned, LineMode};
pub use error::{
    ActualStart, FormatError, FormatResult, InvalidDocumentError, PrintError, PrintResult,
};
pub use format::{Format, Formatted, format, write};
pub use formatter::{Formatter, FormatterSnapshot};
pub use group::{ConditionalGroup, DebugGroupId, Group, GroupId, GroupMode, ReleaseGroupId};
pub use label::{LabelDefinition, LabelId};
pub use printer::{LineEnding, PrintOptions, Printed, PrintedSpan, Printer};
pub use sizing::{
    BestFittingMode, BestFittingVariants, BestFittingVariantsIter, FormatElements, TextLen,
    TextWidth, Width,
};
pub use source::{LINE_TERMINATORS, SourceMarker, normalize_newlines};
pub use spacing::{IndentStyle, Indentation};
pub use tag::{
    Condition, DedentMode, FitsExpanded, FormatTag, FormatTagKind, PrintMode, VerbatimKind,
};
