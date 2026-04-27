use std::cell::Cell;

use super::group::{ConditionalGroup, Group, GroupId, GroupMode};

/// A Tag marking the start and end of some content to which some special formatting should be applied.
///
/// Tags always come in pairs of a start and an end tag and the styling defined by this tag
/// will be applied to all nodes in between the start/end tags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatTag {
    /// Indents the content one level deeper, see `crate::builders::indent` for documentation and examples.
    StartIndent,
    EndIndent,

    /// Variant of [`TagKind::Indent`] that indents content by a number of spaces.
    /// For example, `Align(2)` indents any content following a line break by an additional two spaces.
    ///
    /// Nested aligns add their space widths together.
    StartAlign(u8),
    EndAlign,

    /// Reduces the indentation of the specified content either by one level or to the root, depending on the mode.
    /// Reverse operation of `Indent` and can be used to *undo* an `Align` for nested content.
    StartDedent(DedentMode),
    EndDedent(DedentMode),

    /// Creates a logical group where its content is either consistently printed:
    /// - on a single line: Omitting `LineMode::Soft` line breaks and printing spaces for `LineMode::SoftOrSpace`
    /// - on multiple lines: Printing all line breaks
    ///
    /// See [`crate::builders::group`] for documentation and examples.
    StartGroup(Group),
    EndGroup,

    /// Creates a logical group similar to [`Tag::StartGroup`] but only if the condition is met.
    /// This is an optimized representation for (assuming the content should only be grouped if another group fits):
    ///
    /// ```text
    /// if_group_breaks(content, other_group_id),
    /// if_group_fits_on_line(group(&content), other_group_id)
    /// ```
    StartConditionalGroup(ConditionalGroup),
    EndConditionalGroup,

    /// Allows to specify content that gets printed depending on whatever the enclosing group
    /// is printed on a single line or multiple lines.
    /// See [`crate::builders::if_group_breaks`] for examples.
    StartConditionalContent(Condition),
    EndConditionalContent,

    /// Optimized version of [`Tag::StartConditionalContent`] for the case where some content
    /// should be indented if the specified group breaks.
    StartIndentIfGroupBreaks(GroupId),
    EndIndentIfGroupBreaks(GroupId),

    /// Concatenates multiple nodes together with a given separator printed in either
    /// flat or expanded mode to fill the print width.
    /// Expect that the content is a list of alternating [node, separator] See [`crate::Formatter::fill`].
    StartFill,
    EndFill,

    /// Entry inside of a [`Tag::StartFill`]
    StartEntry,
    EndEntry,

    /// Delay the printing of its content until the next line break.
    StartLineSuffix,
    EndLineSuffix,

    /// A token that tracks tokens/nodes that are printed as verbatim.
    StartVerbatim(VerbatimKind),
    EndVerbatim,

    StartFitsExpanded(FitsExpanded),
    EndFitsExpanded,

    /// Marks the start and end of a best-fitting variant.
    /// Parenthesizes the content but only if adding the parentheses and indenting the content
    /// makes the content fit in the configured line width.
    ///
    /// See [`crate::builders::best_fit_parenthesize`] for an in-depth explanation.
    StartBestFitParenthesize {
        id: Option<GroupId>,
    },
    EndBestFitParenthesize,
}

impl FormatTag {
    /// Check if this is any start tag.
    pub const fn is_start(&self) -> bool {
        matches!(
            self,
            FormatTag::StartIndent
                | FormatTag::StartAlign(_)
                | FormatTag::StartDedent(_)
                | FormatTag::StartGroup(_)
                | FormatTag::StartConditionalGroup(_)
                | FormatTag::StartConditionalContent(_)
                | FormatTag::StartIndentIfGroupBreaks(_)
                | FormatTag::StartFill
                | FormatTag::StartEntry
                | FormatTag::StartLineSuffix
                | FormatTag::StartVerbatim(_)
                | FormatTag::StartFitsExpanded(_)
                | FormatTag::StartBestFitParenthesize { .. }
        )
    }

    /// Check if this is any end tag.
    pub const fn is_end(&self) -> bool {
        !self.is_start()
    }

    /// Get the kind of this tag.
    pub const fn kind(&self) -> FormatTagKind {
        #[allow(clippy::enum_glob_use)]
        use FormatTag::*;

        match self {
            StartIndent | EndIndent => FormatTagKind::Indent,
            StartAlign(_) | EndAlign => FormatTagKind::Align,
            StartDedent(_) | EndDedent(_) => FormatTagKind::Dedent,
            StartGroup(_) | EndGroup => FormatTagKind::Group,
            StartConditionalGroup(_) | EndConditionalGroup => FormatTagKind::ConditionalGroup,
            StartConditionalContent(_) | EndConditionalContent => FormatTagKind::ConditionalContent,
            StartIndentIfGroupBreaks(_) | EndIndentIfGroupBreaks(_) => {
                FormatTagKind::IndentIfGroupBreaks
            }
            StartFill | EndFill => FormatTagKind::Fill,
            StartEntry | EndEntry => FormatTagKind::Entry,
            StartLineSuffix | EndLineSuffix => FormatTagKind::LineSuffix,
            StartVerbatim(_) | EndVerbatim => FormatTagKind::Verbatim,
            StartFitsExpanded { .. } | EndFitsExpanded => FormatTagKind::FitsExpanded,
            StartBestFitParenthesize { .. } | EndBestFitParenthesize => {
                FormatTagKind::BestFitParenthesize
            }
        }
    }
}

/// The kind of a `Tag`.
///
/// Each start end tag pair has its own `TagKind`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FormatTagKind {
    Indent,
    Align,
    Dedent,
    Group,
    ConditionalGroup,
    ConditionalContent,
    IndentIfGroupBreaks,
    Fill,
    Entry,
    LineSuffix,
    Verbatim,
    Labelled,
    FitsExpanded,
    BestFitParenthesize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DedentMode {
    /// Reduces the indent by a level (if the current indent is > 0)
    Level,
    /// Reduces the indent to the root
    Root,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Condition {
    /// - `Flat` -> Omitted if the enclosing group is a multiline group, printed for groups fitting on a single line
    /// - `Expanded` -> Omitted if the enclosing group fits on a single line, printed if the group breaks over multiple lines.
    pub(crate) mode: PrintMode,

    /// The id of the group for which it should check if it breaks or not.
    /// The group must appear in the document before the conditional group (but doesn't have to be in the ancestor chain).
    pub(crate) group_id: Option<GroupId>,
}

impl Condition {
    /// Create a new condition with the given mode.
    pub(crate) fn new(mode: PrintMode) -> Self {
        Self {
            mode,
            group_id: None,
        }
    }

    /// Create a condition that applies if the group fits on a single line.
    pub fn if_fits_on_line() -> Self {
        Self {
            mode: PrintMode::Flat,
            group_id: None,
        }
    }

    /// Create a condition that applies if the specified group fits on a single line.
    pub fn if_group_fits_on_line(group_id: GroupId) -> Self {
        Self {
            mode: PrintMode::Flat,
            group_id: Some(group_id),
        }
    }

    /// Create a condition that applies if the group breaks across multiple lines.
    pub fn if_breaks() -> Self {
        Self {
            mode: PrintMode::Expanded,
            group_id: None,
        }
    }

    /// Create a condition that applies if the specified group breaks across multiple lines.
    pub fn if_group_breaks(group_id: GroupId) -> Self {
        Self {
            mode: PrintMode::Expanded,
            group_id: Some(group_id),
        }
    }

    /// Set the group id for this condition.
    #[must_use]
    pub fn with_group_id(mut self, id: Option<GroupId>) -> Self {
        self.group_id = id;
        self
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PrintMode {
    /// Omits any soft line breaks
    Flat,
    /// Prints soft line breaks as line breaks
    Expanded,
}

impl PrintMode {
    /// Check if this mode is flat.
    pub const fn is_flat(&self) -> bool {
        matches!(self, PrintMode::Flat)
    }

    /// Check if this mode is expanded.
    pub const fn is_expanded(&self) -> bool {
        matches!(self, PrintMode::Expanded)
    }
}

impl From<GroupMode> for PrintMode {
    fn from(value: GroupMode) -> Self {
        match value {
            GroupMode::Flat => PrintMode::Flat,
            GroupMode::Expand | GroupMode::Propagated => PrintMode::Expanded,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct FitsExpanded {
    pub(crate) condition: Option<Condition>,
    pub(crate) propagate_expand: Cell<bool>,
}

impl FitsExpanded {
    /// Create a new FitsExpanded.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the condition for this FitsExpanded.
    #[must_use]
    pub fn with_condition(mut self, condition: Option<Condition>) -> Self {
        self.condition = condition;
        self
    }

    /// Mark that expansion should be propagated.
    pub fn propagate_expand(&self) {
        self.propagate_expand.set(true);
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum VerbatimKind {
    Bogus,
    Suppressed,
    Verbatim {
        /// the length of the formatted node
        length: u32,
    },
}

impl VerbatimKind {
    /// Check if this is a bogus verbatim kind.
    pub const fn is_bogus(&self) -> bool {
        matches!(self, VerbatimKind::Bogus)
    }
}
