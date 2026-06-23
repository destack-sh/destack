use destack_lsp_types as lsp;
use destack_query as query;

/// Convert query symbol kind to LSP symbol kind.
pub(super) fn symbol_kind_to_lsp(kind: query::SymbolKind) -> lsp::SymbolKind {
    match kind {
        query::SymbolKind::File => lsp::SymbolKind::FILE,
        query::SymbolKind::Module => lsp::SymbolKind::MODULE,
        query::SymbolKind::Namespace => lsp::SymbolKind::NAMESPACE,
        query::SymbolKind::Package => lsp::SymbolKind::PACKAGE,
        query::SymbolKind::Class => lsp::SymbolKind::CLASS,
        query::SymbolKind::Method => lsp::SymbolKind::METHOD,
        query::SymbolKind::Property => lsp::SymbolKind::PROPERTY,
        query::SymbolKind::Field => lsp::SymbolKind::FIELD,
        query::SymbolKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
        query::SymbolKind::Enum => lsp::SymbolKind::ENUM,
        query::SymbolKind::Interface => lsp::SymbolKind::INTERFACE,
        query::SymbolKind::Function => lsp::SymbolKind::FUNCTION,
        query::SymbolKind::Variable => lsp::SymbolKind::VARIABLE,
        query::SymbolKind::Constant => lsp::SymbolKind::CONSTANT,
        query::SymbolKind::String => lsp::SymbolKind::STRING,
        query::SymbolKind::Number => lsp::SymbolKind::NUMBER,
        query::SymbolKind::Boolean => lsp::SymbolKind::BOOLEAN,
        query::SymbolKind::Array => lsp::SymbolKind::ARRAY,
        query::SymbolKind::Object => lsp::SymbolKind::OBJECT,
        query::SymbolKind::Key => lsp::SymbolKind::KEY,
        query::SymbolKind::Null => lsp::SymbolKind::NULL,
        query::SymbolKind::EnumMember => lsp::SymbolKind::ENUM_MEMBER,
        query::SymbolKind::Struct => lsp::SymbolKind::STRUCT,
        query::SymbolKind::Event => lsp::SymbolKind::EVENT,
        query::SymbolKind::Operator => lsp::SymbolKind::OPERATOR,
        query::SymbolKind::TypeParameter => lsp::SymbolKind::TYPE_PARAMETER,
    }
}
