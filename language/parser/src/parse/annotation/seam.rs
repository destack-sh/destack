/// The leading annotation attachment policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum LeadingAnnotationKind {
    /// Attach as statement or declaration style leading trivia.
    Statement,
    /// Attach as expression style leading trivia.
    Expression,
    /// Attach as type seam leading trivia.
    Type,
    /// Attach as wrapper-leading trivia that keeps line comments inline.
    Wrapper,
}

/// The trailing annotation attachment policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TrailingAnnotationKind {
    /// Use standard trailing boundary behavior.
    Default,
    /// Preserve inline line-postfix behavior at newline boundaries.
    PreserveLinePostfix,
}

/// The dot-style boundary attachment policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DotBoundaryKind {
    /// Attach around a normal member-access boundary.
    Member,
    /// Attach around an optional-call boundary.
    OptionalCall,
}

/// The blank-line attachment side.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum BlankBoundaryKind {
    /// Attach blank lines as block postfix trivia.
    Postfix,
}

/// One parser boundary attachment operation.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum AnnotationSeamKind {
    /// Attach leading trivia at a token boundary.
    Leading(LeadingAnnotationKind),
    /// Attach infix trivia at a token boundary.
    Infix,
    /// Attach synthetic stub trivia at a token boundary.
    Stub,
    /// Attach trailing trivia before a token boundary.
    Trailing(TrailingAnnotationKind),
    /// Attach trailing line comments as explicit boundary postfix trivia.
    TrailingLineBoundary,
    /// Attach dot-boundary trivia before member or optional-call edges.
    DotBoundary(DotBoundaryKind),
    /// Attach dot-prefix line comments to the following chain segment.
    DotPrefix,
    /// Attach skipped blank lines at a token boundary.
    Blank(BlankBoundaryKind),
}
