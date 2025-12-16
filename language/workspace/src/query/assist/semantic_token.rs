use destack_source::{FileId, Span};

use crate::Session;

/// Semantic token type for LSP semantic highlighting.
///
/// More granular than `destack_ast::SemanticType` to support full LSP surface.
/// Maps to LSP's SemanticTokenTypes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenType {
    Namespace,
    Type,
    Class,
    Enum,
    Interface,
    Struct,
    TypeParameter,
    Parameter,
    Variable,
    Property,
    EnumMember,
    Event,
    Function,
    Method,
    Macro,
    Keyword,
    Modifier,
    Comment,
    String,
    Number,
    Regexp,
    Operator,
    Decorator,
}

/// Semantic token modifiers (can be combined).
///
/// Maps to LSP's SemanticTokenModifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SemanticTokenModifiers(u32);

impl SemanticTokenModifiers {
    pub const NONE: Self = Self(0);
    pub const DECLARATION: Self = Self(1 << 0);
    pub const DEFINITION: Self = Self(1 << 1);
    pub const READONLY: Self = Self(1 << 2);
    pub const STATIC: Self = Self(1 << 3);
    pub const DEPRECATED: Self = Self(1 << 4);
    pub const ABSTRACT: Self = Self(1 << 5);
    pub const ASYNC: Self = Self(1 << 6);
    pub const MODIFICATION: Self = Self(1 << 7);
    pub const DOCUMENTATION: Self = Self(1 << 8);
    pub const DEFAULT_LIBRARY: Self = Self(1 << 9);

    /// Combine modifiers.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check if a modifier is set.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Get the raw bits for LSP encoding.
    pub fn bits(self) -> u32 {
        self.0
    }
}

/// A single semantic token.
#[derive(Debug, Clone)]
pub struct SemanticToken {
    /// The span of the token.
    pub span: Span,
    /// The token type.
    pub token_type: SemanticTokenType,
    /// The token modifiers.
    pub modifiers: SemanticTokenModifiers,
}

impl SemanticToken {
    /// Create a new semantic token.
    pub fn new(span: Span, token_type: SemanticTokenType) -> Self {
        Self {
            span,
            token_type,
            modifiers: SemanticTokenModifiers::NONE,
        }
    }

    /// Add modifiers.
    pub fn with_modifiers(mut self, modifiers: SemanticTokenModifiers) -> Self {
        self.modifiers = self.modifiers.union(modifiers);
        self
    }
}

/// Get semantic tokens for a file.
///
/// Returns tokens suitable for LSP textDocument/semanticTokens/full.
/// The tokens are in source order (not delta-encoded - LSP layer handles that).
pub fn semantic_tokens(_session: &Session, _file: FileId) -> Vec<SemanticToken> {
    // 1. get the AST for the file
    // 2. walk the AST, collecting semantic tokens
    // 3. for identifiers, resolve to determine type (variable, parameter, function, etc.)
    // 4. add modifiers based on context (declaration, readonly, static, etc.)
    //
    // Can leverage existing SemanticTokenIndex from destack_ast as a starting point,
    // but need to add resolution for richer type information.
    todo!("#Incomplete: semantic_tokens")
}

/// Get semantic tokens for a range in a file.
///
/// Returns tokens suitable for LSP textDocument/semanticTokens/range.
pub fn semantic_tokens_range(
    _session: &Session,
    _file: FileId,
    _range: Span,
) -> Vec<SemanticToken> {
    // Same as semantic_tokens but filtered to range
    todo!("#Incomplete: semantic_tokens_range")
}
