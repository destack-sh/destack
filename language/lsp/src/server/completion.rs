use destack_lsp_types as lsp;
use destack_query as query;

/// Convert completion kind to LSP completion item kind.
pub(super) fn kind(kind: query::CompletionItemKind) -> lsp::CompletionItemKind {
    match kind {
        query::CompletionItemKind::AssociatedConst => lsp::CompletionItemKind::CONSTANT,
        query::CompletionItemKind::AssociatedType => lsp::CompletionItemKind::TYPE_PARAMETER,
        query::CompletionItemKind::Method => lsp::CompletionItemKind::METHOD,
        query::CompletionItemKind::Function => lsp::CompletionItemKind::FUNCTION,
        query::CompletionItemKind::Constructor => lsp::CompletionItemKind::CONSTRUCTOR,
        query::CompletionItemKind::Field => lsp::CompletionItemKind::FIELD,
        query::CompletionItemKind::Variable => lsp::CompletionItemKind::VARIABLE,
        query::CompletionItemKind::Class => lsp::CompletionItemKind::CLASS,
        query::CompletionItemKind::Interface => lsp::CompletionItemKind::INTERFACE,
        query::CompletionItemKind::NewtypeInterface => lsp::CompletionItemKind::INTERFACE,
        query::CompletionItemKind::Newtype => lsp::CompletionItemKind::STRUCT,
        query::CompletionItemKind::TypeAlias => lsp::CompletionItemKind::TYPE_PARAMETER,
        query::CompletionItemKind::Extension => lsp::CompletionItemKind::CLASS,
        query::CompletionItemKind::Module => lsp::CompletionItemKind::MODULE,
        query::CompletionItemKind::Property => lsp::CompletionItemKind::PROPERTY,
        query::CompletionItemKind::Value => lsp::CompletionItemKind::VALUE,
        query::CompletionItemKind::Enum => lsp::CompletionItemKind::ENUM,
        query::CompletionItemKind::Keyword => lsp::CompletionItemKind::KEYWORD,
        query::CompletionItemKind::File => lsp::CompletionItemKind::FILE,
        query::CompletionItemKind::Reference => lsp::CompletionItemKind::REFERENCE,
        query::CompletionItemKind::Label => lsp::CompletionItemKind::REFERENCE,
        query::CompletionItemKind::Folder => lsp::CompletionItemKind::FOLDER,
        query::CompletionItemKind::EnumMember => lsp::CompletionItemKind::ENUM_MEMBER,
        query::CompletionItemKind::Constant => lsp::CompletionItemKind::CONSTANT,
        query::CompletionItemKind::Struct => lsp::CompletionItemKind::STRUCT,
        query::CompletionItemKind::TypeParameter => lsp::CompletionItemKind::TYPE_PARAMETER,
        query::CompletionItemKind::ValueParameter => lsp::CompletionItemKind::VARIABLE,
        query::CompletionItemKind::BuiltinType => lsp::CompletionItemKind::KEYWORD,
    }
}
