use destack_lsp_types as lsp;
use destack_query as query;

/// Convert one query symbol kind to an LSP symbol kind.
pub(super) fn kind(kind: query::SymbolKind) -> lsp::SymbolKind {
    match kind {
        query::SymbolKind::AssociatedConst | query::SymbolKind::Constant => {
            lsp::SymbolKind::CONSTANT
        }
        query::SymbolKind::AssociatedType => lsp::SymbolKind::TYPE_PARAMETER,
        query::SymbolKind::Class => lsp::SymbolKind::CLASS,
        query::SymbolKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
        query::SymbolKind::Enum => lsp::SymbolKind::ENUM,
        query::SymbolKind::EnumMember => lsp::SymbolKind::ENUM_MEMBER,
        query::SymbolKind::Extension => lsp::SymbolKind::CLASS,
        query::SymbolKind::Field => lsp::SymbolKind::FIELD,
        query::SymbolKind::Function => lsp::SymbolKind::FUNCTION,
        query::SymbolKind::Interface | query::SymbolKind::NewtypeInterface => {
            lsp::SymbolKind::INTERFACE
        }
        query::SymbolKind::Method => lsp::SymbolKind::METHOD,
        query::SymbolKind::Newtype => lsp::SymbolKind::STRUCT,
        query::SymbolKind::Property => lsp::SymbolKind::PROPERTY,
        query::SymbolKind::Struct => lsp::SymbolKind::STRUCT,
        query::SymbolKind::TypeAlias => lsp::SymbolKind::TYPE_PARAMETER,
        query::SymbolKind::Variable => lsp::SymbolKind::VARIABLE,
    }
}
