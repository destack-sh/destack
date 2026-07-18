use destack_dir as dir;
use destack_lsp_types as lsp;

/// Convert one DIR symbol kind to an LSP symbol kind.
pub(super) fn symbol_kind_to_lsp(kind: dir::SymbolKind) -> lsp::SymbolKind {
    match kind {
        dir::SymbolKind::AssociatedConst => lsp::SymbolKind::CONSTANT,
        dir::SymbolKind::AssociatedType => lsp::SymbolKind::TYPE_PARAMETER,
        dir::SymbolKind::Class => lsp::SymbolKind::CLASS,
        dir::SymbolKind::Enum => lsp::SymbolKind::ENUM,
        dir::SymbolKind::Variant => lsp::SymbolKind::ENUM_MEMBER,
        dir::SymbolKind::Extension => lsp::SymbolKind::CLASS,
        dir::SymbolKind::Function => lsp::SymbolKind::FUNCTION,
        dir::SymbolKind::GenericTypeParameter => lsp::SymbolKind::TYPE_PARAMETER,
        dir::SymbolKind::GenericValueParameter => lsp::SymbolKind::VARIABLE,
        dir::SymbolKind::Import => lsp::SymbolKind::NAMESPACE,
        dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface => {
            lsp::SymbolKind::INTERFACE
        }
        dir::SymbolKind::Label => lsp::SymbolKind::KEY,
        dir::SymbolKind::Newtype => lsp::SymbolKind::TYPE_PARAMETER,
        dir::SymbolKind::Struct => lsp::SymbolKind::STRUCT,
        dir::SymbolKind::TypeAlias => lsp::SymbolKind::TYPE_PARAMETER,
        dir::SymbolKind::Variable => lsp::SymbolKind::VARIABLE,
    }
}
