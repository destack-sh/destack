/// The lexer mode used for the next visible parser token.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub(crate) enum ContextualLexMode {
    /// Use regular language tokenization.
    Normal,
    /// Use tree tag tokenization.
    TreeTag,
    /// Use tree child tokenization.
    TreeChild,
    /// Use tree attribute value tokenization.
    TreeAttributeValue,
}
