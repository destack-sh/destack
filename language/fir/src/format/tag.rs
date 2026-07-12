use super::group::{ConditionalGroup, Group, GroupId, GroupMode};

/// One structural delimiter written by a formatting builder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormatTag {
    /// Start one indentation scope.
    StartIndent,
    /// End one indentation scope.
    EndIndent,
    /// Start one alignment scope.
    StartAlign(u8),
    /// End one alignment scope.
    EndAlign,
    /// Start one dedentation scope.
    StartDedent(DedentMode),
    /// End one dedentation scope.
    EndDedent(DedentMode),
    /// Start one logical group.
    StartGroup(Group),
    /// End one logical group.
    EndGroup,
    /// Start one conditionally active logical group.
    StartConditionalGroup(ConditionalGroup),
    /// End one conditionally active logical group.
    EndConditionalGroup,
    /// Start content controlled by one group layout.
    StartConditionalContent(Condition),
    /// End content controlled by one group layout.
    EndConditionalContent,
    /// Start indentation controlled by one named group.
    StartIndentIfGroupBreaks(GroupId),
    /// End indentation controlled by one named group.
    EndIndentIfGroupBreaks(GroupId),
    /// Start one fill sequence.
    StartFill,
    /// End one fill sequence.
    EndFill,
    /// Start one fill or best-fitting entry.
    StartEntry,
    /// End one fill or best-fitting entry.
    EndEntry,
    /// Start content delayed until the next line break.
    StartLineSuffix,
    /// End content delayed until the next line break.
    EndLineSuffix,
    /// Start one verbatim source range.
    StartVerbatim(VerbatimKind),
    /// End one verbatim source range.
    EndVerbatim,
    /// Start one expanded-layout measurement scope.
    StartFitsExpanded(FitsExpanded),
    /// End one expanded-layout measurement scope.
    EndFitsExpanded,
    /// Start content conditionally wrapped in parentheses.
    StartBestFitParenthesize {
        /// The optional externally referenced group identifier.
        id: Option<GroupId>,
    },
    /// End content conditionally wrapped in parentheses.
    EndBestFitParenthesize,
}

impl FormatTag {
    /// Return whether this tag starts a structural scope.
    pub const fn is_start(self) -> bool {
        matches!(
            self,
            Self::StartIndent
                | Self::StartAlign(_)
                | Self::StartDedent(_)
                | Self::StartGroup(_)
                | Self::StartConditionalGroup(_)
                | Self::StartConditionalContent(_)
                | Self::StartIndentIfGroupBreaks(_)
                | Self::StartFill
                | Self::StartEntry
                | Self::StartLineSuffix
                | Self::StartVerbatim(_)
                | Self::StartFitsExpanded(_)
                | Self::StartBestFitParenthesize { .. }
        )
    }

    /// Return whether this tag ends a structural scope.
    pub const fn is_end(self) -> bool {
        !self.is_start()
    }

    /// Return this tag's structural kind.
    pub const fn kind(self) -> FormatTagKind {
        match self {
            Self::StartIndent | Self::EndIndent => FormatTagKind::Indent,
            Self::StartAlign(_) | Self::EndAlign => FormatTagKind::Align,
            Self::StartDedent(_) | Self::EndDedent(_) => FormatTagKind::Dedent,
            Self::StartGroup(_) | Self::EndGroup => FormatTagKind::Group,
            Self::StartConditionalGroup(_) | Self::EndConditionalGroup => {
                FormatTagKind::ConditionalGroup
            }
            Self::StartConditionalContent(_) | Self::EndConditionalContent => {
                FormatTagKind::ConditionalContent
            }
            Self::StartIndentIfGroupBreaks(_) | Self::EndIndentIfGroupBreaks(_) => {
                FormatTagKind::IndentIfGroupBreaks
            }
            Self::StartFill | Self::EndFill => FormatTagKind::Fill,
            Self::StartEntry | Self::EndEntry => FormatTagKind::Entry,
            Self::StartLineSuffix | Self::EndLineSuffix => FormatTagKind::LineSuffix,
            Self::StartVerbatim(_) | Self::EndVerbatim => FormatTagKind::Verbatim,
            Self::StartFitsExpanded(_) | Self::EndFitsExpanded => FormatTagKind::FitsExpanded,
            Self::StartBestFitParenthesize { .. } | Self::EndBestFitParenthesize => {
                FormatTagKind::BestFitParenthesize
            }
        }
    }
}

/// The structural kind shared by one matching tag pair.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FormatTagKind {
    /// One indentation scope.
    Indent,
    /// One alignment scope.
    Align,
    /// One dedentation scope.
    Dedent,
    /// One logical group.
    Group,
    /// One conditionally active logical group.
    ConditionalGroup,
    /// One conditional content scope.
    ConditionalContent,
    /// One conditional indentation scope.
    IndentIfGroupBreaks,
    /// One fill sequence.
    Fill,
    /// One fill or best-fitting entry.
    Entry,
    /// One delayed line suffix.
    LineSuffix,
    /// One verbatim source range.
    Verbatim,
    /// One expanded-layout measurement scope.
    FitsExpanded,
    /// One conditional parenthesized layout.
    BestFitParenthesize,
}

/// The indentation reduction applied by one dedentation scope.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DedentMode {
    /// Remove one indentation level.
    Level,
    /// Remove all indentation.
    Root,
}

/// One condition comparing a selected group layout with an expected print mode.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Condition {
    /// The print mode that enables the controlled content.
    pub(crate) mode: PrintMode,
    /// The referenced group, or the enclosing group when absent.
    pub(crate) group_id: Option<GroupId>,
}

impl Condition {
    /// Create one condition against the enclosing group.
    pub(crate) const fn new(mode: PrintMode) -> Self {
        Self {
            mode,
            group_id: None,
        }
    }

    /// Create one condition enabled by a flat enclosing group.
    pub const fn if_fits_on_line() -> Self {
        Self::new(PrintMode::Flat)
    }

    /// Create one condition enabled by a flat named group.
    pub const fn if_group_fits_on_line(group_id: GroupId) -> Self {
        Self {
            mode: PrintMode::Flat,
            group_id: Some(group_id),
        }
    }

    /// Create one condition enabled by an expanded enclosing group.
    pub const fn if_breaks() -> Self {
        Self::new(PrintMode::Expanded)
    }

    /// Create one condition enabled by an expanded named group.
    pub const fn if_group_breaks(group_id: GroupId) -> Self {
        Self {
            mode: PrintMode::Expanded,
            group_id: Some(group_id),
        }
    }

    /// Set the referenced group.
    #[must_use]
    pub const fn with_group_id(mut self, id: Option<GroupId>) -> Self {
        self.group_id = id;

        self
    }
}

/// The selected layout while printing one group.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PrintMode {
    /// Omit soft lines and print soft-or-space lines as spaces.
    Flat,
    /// Print every line instruction as a line break.
    Expanded,
}

impl PrintMode {
    /// Return whether this mode prints a flat layout.
    pub const fn is_flat(self) -> bool {
        matches!(self, Self::Flat)
    }

    /// Return whether this mode prints an expanded layout.
    pub const fn is_expanded(self) -> bool {
        matches!(self, Self::Expanded)
    }
}

impl From<GroupMode> for PrintMode {
    fn from(value: GroupMode) -> Self {
        match value {
            GroupMode::Flat => Self::Flat,
            GroupMode::Expand | GroupMode::Propagated => Self::Expanded,
        }
    }
}

/// One request to measure nested groups in expanded mode.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct FitsExpanded {
    /// The optional condition controlling expanded measurement.
    pub(crate) condition: Option<Condition>,
}

impl FitsExpanded {
    /// Create one unconditional expanded measurement.
    pub const fn new() -> Self {
        Self { condition: None }
    }

    /// Set the condition controlling expanded measurement.
    #[must_use]
    pub const fn with_condition(mut self, condition: Option<Condition>) -> Self {
        self.condition = condition;

        self
    }
}

/// The dense index of one fits-expanded row in a formatted document.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct FitsExpandedIndex(u32);

impl FitsExpandedIndex {
    /// Create one fits-expanded index.
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return this index as a vector offset.
    pub(crate) const fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Return the encoded index.
    pub(crate) const fn value(self) -> u32 {
        self.0
    }
}

/// The mutable layout row for one fits-expanded instruction.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) struct FitsExpandedState {
    /// The optional condition controlling expanded measurement.
    pub(crate) condition: Option<Condition>,
    /// Whether nested content requires expansion.
    pub(crate) is_expanded: bool,
}

impl FitsExpandedState {
    /// Create one fits-expanded row.
    pub(crate) const fn new(fits: FitsExpanded) -> Self {
        Self {
            condition: fits.condition,
            is_expanded: false,
        }
    }
}

/// The source preservation behavior for one verbatim range.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum VerbatimKind {
    /// Preserve one recovered or otherwise invalid node.
    Bogus,
    /// Preserve source skipped by a suppression directive.
    Suppressed,
    /// Preserve one valid source range of the given output length.
    Verbatim {
        /// The formatted byte length.
        length: u32,
    },
}

impl VerbatimKind {
    /// Return whether this range preserves a recovered node.
    pub const fn is_bogus(self) -> bool {
        matches!(self, Self::Bogus)
    }
}
