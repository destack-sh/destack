/// One public query method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryMethod {
    /// Completion query.
    Completion,
    /// Completion details query.
    CompletionDetails,
    /// Hover query.
    Hover,
    /// Signature help query.
    SignatureHelp,
    /// Inlay hints query.
    InlayHints,
    /// Code lenses query.
    CodeLenses,
    /// Folding ranges query.
    FoldingRanges,
    /// Full-document semantic tokens query.
    SemanticTokens,
    /// Range semantic tokens query.
    SemanticTokensRange,
    /// Outline query.
    Outline,
    /// Symbol search query.
    SearchSymbols,
    /// Links query.
    Links,
    /// Highlight query.
    Highlight,
    /// Selection ranges query.
    SelectionRanges,
    /// Definition navigation query.
    GotoDefinition,
    /// Declaration navigation query.
    GotoDeclaration,
    /// Type-definition navigation query.
    GotoTypeDefinition,
    /// Implementation navigation query.
    GotoImplementation,
    /// Reference query.
    FindReferences,
    /// Call hierarchy item query.
    CallItem,
    /// Incoming call query.
    IncomingCalls,
    /// Outgoing call query.
    OutgoingCalls,
    /// Type hierarchy item query.
    TypeItem,
    /// Supertype query.
    Supertypes,
    /// Subtype query.
    Subtypes,
    /// Decorator query.
    Decorators,
    /// Rename target query.
    RenameTarget,
    /// Rename query.
    Rename,
    /// File rename query.
    RenameFiles,
    /// Variable extraction query.
    ExtractVariable,
    /// Inline query.
    Inline,
    /// Code action query.
    CodeActions,
}

impl QueryMethod {
    /// Every public query method in canonical order.
    pub const ALL: [Self; 32] = [
        Self::Completion,
        Self::CompletionDetails,
        Self::Hover,
        Self::SignatureHelp,
        Self::InlayHints,
        Self::CodeLenses,
        Self::FoldingRanges,
        Self::SemanticTokens,
        Self::SemanticTokensRange,
        Self::Outline,
        Self::SearchSymbols,
        Self::Links,
        Self::Highlight,
        Self::SelectionRanges,
        Self::GotoDefinition,
        Self::GotoDeclaration,
        Self::GotoTypeDefinition,
        Self::GotoImplementation,
        Self::FindReferences,
        Self::CallItem,
        Self::IncomingCalls,
        Self::OutgoingCalls,
        Self::TypeItem,
        Self::Supertypes,
        Self::Subtypes,
        Self::Decorators,
        Self::RenameTarget,
        Self::Rename,
        Self::RenameFiles,
        Self::ExtractVariable,
        Self::Inline,
        Self::CodeActions,
    ];

    /// Return the method with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|method| method.name() == name)
    }

    /// Return this method's canonical name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Completion => "completion",
            Self::CompletionDetails => "completion_details",
            Self::Hover => "hover",
            Self::SignatureHelp => "signature_help",
            Self::InlayHints => "inlay_hints",
            Self::CodeLenses => "code_lenses",
            Self::FoldingRanges => "folding_ranges",
            Self::SemanticTokens => "semantic_tokens",
            Self::SemanticTokensRange => "semantic_tokens_range",
            Self::Outline => "outline",
            Self::SearchSymbols => "search_symbols",
            Self::Links => "links",
            Self::Highlight => "highlight",
            Self::SelectionRanges => "selection_ranges",
            Self::GotoDefinition => "goto_definition",
            Self::GotoDeclaration => "goto_declaration",
            Self::GotoTypeDefinition => "goto_type_definition",
            Self::GotoImplementation => "goto_implementation",
            Self::FindReferences => "find_references",
            Self::CallItem => "call_item",
            Self::IncomingCalls => "incoming_calls",
            Self::OutgoingCalls => "outgoing_calls",
            Self::TypeItem => "type_item",
            Self::Supertypes => "supertypes",
            Self::Subtypes => "subtypes",
            Self::Decorators => "decorators",
            Self::RenameTarget => "rename_target",
            Self::Rename => "rename",
            Self::RenameFiles => "rename_files",
            Self::ExtractVariable => "extract_variable",
            Self::Inline => "inline",
            Self::CodeActions => "code_actions",
        }
    }
}
