use crate::lex::{HtmlString, Tag};

/// One split whitespace classification.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub(crate) enum SplitStatus {
    /// The token text has not been split into whitespace runs.
    NotSplit,
    /// The token text is entirely ASCII whitespace.
    Whitespace,
    /// The token text contains non-whitespace.
    NotWhitespace,
}

/// One parser-local HTML token.
#[derive(PartialEq, Eq, Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Token {
    /// One tag token.
    Tag(Tag),
    /// One comment token payload.
    Comment(HtmlString),
    /// One character token payload.
    Characters(SplitStatus, HtmlString),
    /// One null character token.
    NullCharacter,
    /// One end-of-file token.
    Eof,
}
